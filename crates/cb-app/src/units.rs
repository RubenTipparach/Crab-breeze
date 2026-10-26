//! The invasion's vehicles in the game (openspec change `invasion-forces`): each defence wave's
//! tanks, helicopters and fighters spawn when the wave starts, tanks flown in by cargo plane and
//! dropped by parachute; every fixed step each unit steps through `cb_core::units` and its shots
//! join the mechs' in `combat`. Placeholders are drawn from boxes and cylinders in the unit's size
//! and colour until each has its model. `Hostiles` lists every live enemy, mech or vehicle, once a
//! step for targeting, the cockpit and the scripted pilot.

use crate::combat::Shots;
use crate::fx::Fx;
use crate::mech::{Enemy, EnemyGoal, MechId, MechSim, Piloted};
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::rng::SimRng;
use cb_core::units::{Cargo, Phase, Unit, UnitKind, UnitView, heading_of};
use cb_core::weapons::Projectile;
use cb_core::world::Ground;
use std::collections::BTreeMap;

/// Ids from here up are vehicles, below are mechs.
pub const UNIT_ID_BASE: u32 = 5000;

/// A live enemy this step, mech or vehicle.
#[derive(Clone, Copy, Debug)]
pub struct Hostile {
    /// Stable id (a mech's `MechId`, or a vehicle's id from `UNIT_ID_BASE`).
    pub id: u32,
    /// The point to aim at and box.
    pub aim: Vec3,
    /// Its lowest point, for the box.
    pub feet: Vec3,
    /// How tall it is drawn, for the box.
    pub height_m: f32,
    /// Velocity.
    pub vel: Vec3,
    /// Shelling the city rather than fighting the pilot.
    pub shelling: bool,
    /// The defence wave it belongs to, if any.
    pub wave: Option<usize>,
}

/// Every live enemy this step, in id order.
#[derive(Resource, Default)]
pub struct Hostiles(pub Vec<Hostile>);

/// A vehicle and the wave it counts toward (`None`: a cargo plane, which does not).
#[derive(Clone, Debug)]
pub struct Placed {
    /// The vehicle.
    pub unit: Unit,
    /// Its wave, when it counts toward clearing one.
    pub wave: Option<usize>,
    /// A cargo plane: the wave its load counts toward.
    pub cargo_wave: Option<usize>,
    /// Gone from the world (dead and removed, or a plane that has left).
    pub gone: bool,
    /// The mission that brought it (`free-roam-missions`: it leaves when that mission is turned off).
    pub mission: Option<cb_core::missions::MissionKind>,
}

/// Every vehicle, in id order.
#[derive(Resource, Default)]
pub struct Units {
    /// The vehicles.
    pub list: Vec<Placed>,
    next_id: u32,
}

impl Units {
    fn add(&mut self, unit: impl FnOnce(u32) -> Unit, wave: Option<usize>, cargo_wave: Option<usize>) {
        // Wave vehicles are the defence's.
        let mission = (wave.is_some() || cargo_wave.is_some()).then_some(cb_core::missions::MissionKind::Defence);
        self.add_for(unit, wave, cargo_wave, mission);
    }

    /// Adds a vehicle brought by `mission`; returns its id.
    pub fn add_for(
        &mut self,
        unit: impl FnOnce(u32) -> Unit,
        wave: Option<usize>,
        cargo_wave: Option<usize>,
        mission: Option<cb_core::missions::MissionKind>,
    ) -> u32 {
        let id = UNIT_ID_BASE + self.next_id;
        self.next_id += 1;
        self.list.push(Placed {
            unit: unit(id),
            wave,
            cargo_wave,
            gone: false,
            mission,
        });
        id
    }

    /// Every vehicle `mission` brought leaves the world (the mission was turned off).
    pub fn retire(&mut self, mission: cb_core::missions::MissionKind) {
        for p in self.list.iter_mut().filter(|p| p.mission == Some(mission)) {
            p.gone = true;
        }
    }

    /// `mission`'s vehicles flying or fighting now.
    pub fn count_for(&self, mission: cb_core::missions::MissionKind) -> u32 {
        self.list
            .iter()
            .filter(|p| p.mission == Some(mission) && !p.gone && p.unit.alive())
            .count() as u32
    }

    /// The vehicle with this id.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Placed> {
        self.list.iter_mut().find(|p| p.unit.id == id)
    }

    /// A wave's vehicles: (alive, destroyed).
    pub fn wave_count(&self, wave: usize) -> (u32, u32) {
        let mine = self.list.iter().filter(|p| p.wave == Some(wave));
        let dead = mine.clone().filter(|p| !p.unit.alive()).count() as u32;
        (mine.count() as u32 - dead, dead)
    }
}

/// A wave's vehicles are due (sent by the defence mission when the wave starts).
#[derive(Message, Clone, Copy)]
pub struct WaveForces {
    /// The wave.
    pub wave: usize,
    /// Where its ground units land or start, from the mission's spawn points.
    pub city: u32,
}

/// Draws a vehicle; `chute` is its parachute's entity.
#[derive(Component)]
struct UnitDraw {
    id: u32,
    chute: Option<Entity>,
}

/// Steps the vehicles and keeps the hostiles list.
pub struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Units>()
            .init_resource::<Hostiles>()
            .add_message::<WaveForces>()
            .add_game_systems(FixedUpdate, list_hostiles.in_set(SimSet::Input))
            .add_game_systems(
                FixedUpdate,
                (spawn_forces, step_units)
                    .chain()
                    .in_set(SimSet::Mech)
                    .after(crate::mech::step_mech)
                    .before(crate::combat::fly_shots),
            )
            .add_game_systems(Update, draw_units);
    }
}

/// Mechs and vehicles alike, for everything that picks a target.
pub fn list_hostiles(
    mechs: Query<
        (
            &MechSim,
            &MechId,
            Option<&EnemyGoal>,
            Option<&crate::defence::Raider>,
            &crate::mech::MechSpec,
        ),
        With<Enemy>,
    >,
    units: Res<Units>,
    data: Res<Data>,
    mut out: ResMut<Hostiles>,
) {
    let mut list: Vec<Hostile> = mechs
        .iter()
        .filter(|(m, ..)| !m.dead)
        .map(|(m, id, goal, raider, spec)| Hostile {
            id: id.0,
            aim: m.curr.pos + Vec3::Y * spec.aim_height_m(),
            feet: m.curr.pos,
            height_m: spec.locomotion.body_height_m,
            vel: Vec3::new(m.curr.vel.x, m.curr.vel_y_m_s, m.curr.vel.y),
            shelling: goal.is_some_and(|g| g.point.is_some()),
            wave: raider.map(|r| r.wave),
        })
        .collect();
    for p in units.list.iter().filter(|p| p.unit.alive() && !p.gone) {
        let Some(spec) = data.0.units.units.get(&p.unit.kind) else {
            continue;
        };
        let h = spec.size_m[1];
        list.push(Hostile {
            id: p.unit.id,
            aim: p.unit.pos,
            feet: p.unit.pos - Vec3::Y * h * 0.5,
            height_m: h.max(spec.size_m[2] * 0.5),
            vel: p.unit.vel,
            shelling: false,
            wave: p.wave,
        });
    }
    list.sort_by_key(|h| h.id);
    out.0 = list;
}

fn spawn_forces(mut due: MessageReader<WaveForces>, mut units: ResMut<Units>, data: Res<Data>, plan: Res<Plan>) {
    for f in due.read() {
        let cfg = &data.0.defence;
        let Some(forces) = cfg.forces.get(f.wave) else { continue };
        let city = &plan.cities[f.city as usize];
        let points = cb_core::mission::spawn_points(&plan, cfg, f.wave, &SimRng::new(plan.seed).fork("forces"));
        let mut k = 0usize;
        for force in forces {
            let Some(spec) = data.0.units.units.get(&force.unit) else {
                continue;
            };
            let place = |i: u32| {
                let (xz, _) = points[(k + i as usize) % points.len().max(1)];
                xz
            };
            if force.by_air {
                let Some(plane) = data.0.units.units.get("cargo_plane") else {
                    continue;
                };
                let planes = force.count.div_ceil(force.per_plane);
                for p in 0..planes {
                    let drop_at = place(p);
                    // In from outside the city, over the drop point, toward the city.
                    let out = (drop_at - city.centre).normalize_or(Vec2::X);
                    let start = drop_at + out * 3000.0;
                    let y = plan.height_at(start.x, start.y) + plane.altitude_m[0];
                    let heading = heading_of(Vec3::new(-out.x, 0.0, -out.y));
                    let n = force.per_plane.min(force.count - p * force.per_plane);
                    let load = vec![force.unit.clone(); n as usize];
                    units.add(
                        |id| {
                            let mut u = Unit::new(id, "cargo_plane", plane, Vec3::new(start.x, y, start.y), heading, id);
                            u.cargo = Some(Cargo {
                                load,
                                drop_at,
                                released: 0,
                            });
                            u
                        },
                        None,
                        Some(f.wave),
                    );
                }
            } else {
                for i in 0..force.count {
                    let at = place(i);
                    let out = (at - city.centre).normalize_or(Vec2::X);
                    let (xz, y) = match spec.kind {
                        UnitKind::Tank => (at, plan.height_at(at.x, at.y) + spec.size_m[1] * 0.5),
                        // Aircraft come in from further out, at the top of their band.
                        _ => {
                            let s = at + out * 1500.0;
                            (s, plan.height_at(s.x, s.y) + spec.altitude_m[1])
                        }
                    };
                    let heading = heading_of(Vec3::new(-out.x, 0.0, -out.y));
                    let kind = force.unit.clone();
                    units.add(
                        |id| Unit::new(id, &kind, spec, Vec3::new(xz.x, y, xz.y), heading, id),
                        Some(f.wave),
                        None,
                    );
                }
            }
            k += force.count as usize;
        }
        info!("units: wave {} brings {} vehicles", f.wave + 1, units.list.len());
    }
}

#[allow(clippy::too_many_arguments)]
fn step_units(
    mut units: ResMut<Units>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Fixed>>,
    me: Query<(&MechSim, &crate::mech::MechSpec), With<Piloted>>,
    mut shots: ResMut<Shots>,
    mut fx: MessageWriter<Fx>,
    neutrals: Res<crate::air_traffic::Neutrals>,
) {
    let _t = crate::perf_hud::slow("step_units");
    if units.list.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let cfg = &data.0.units;
    let target = me.iter().find(|(m, _)| !m.dead).map(|(m, spec)| {
        (
            m.curr.pos + Vec3::Y * spec.aim_height_m(),
            Vec3::new(m.curr.vel.x, m.curr.vel_y_m_s, m.curr.vel.y),
        )
    });
    let ground = |x: f32, z: f32| plan.height_at(x, z);
    let mut dropped = Vec::new();
    for p in units.list.iter_mut().filter(|p| !p.gone) {
        let Some(spec) = cfg.units.get(&p.unit.kind) else { continue };
        // A civil aircraft within its guns' reach is a shot of opportunity (civil-air-traffic): the
        // nearest, ties by id.
        let opportunity = neutrals
            .0
            .iter()
            .map(|n| (n.aim.distance(p.unit.pos), n))
            .filter(|(d, _)| *d <= spec.fire_range_m)
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)))
            .map(|(_, n)| (n.aim, n.vel));
        let view = UnitView {
            spec,
            target: target.map(|t| t.0),
            target_vel: target.map_or(Vec3::ZERO, |t| t.1),
            opportunity,
            ground: &ground,
            drop: &cfg.drop,
        };
        let step = p.unit.step(&view, dt);
        if let (Some((from, dir)), Some(w)) = (step.fire, spec.weapon.as_ref()) {
            shots.0.push(Projectile {
                owner: p.unit.id,
                weapon: 0,
                pos: from,
                vel: dir * w.speed_m_s,
                age_s: 0.0,
                rocket: w.rocket,
                target: None,
            });
            let flash = if w.rocket {
                "muzzle_rocket"
            } else if w.blast_mj >= 1.0 {
                "muzzle_cannon"
            } else {
                "muzzle_autocannon"
            };
            fx.write(Fx::new(flash, from).toward(dir));
        }
        for (kind, at, vel) in step.released {
            dropped.push((kind, at, vel, p.cargo_wave));
        }
        if step.landed {
            fx.write(Fx::new("foot_dust", p.unit.pos - Vec3::Y * spec.size_m[1] * 0.5));
        }
        // A plane that has dropped its load leaves; far enough out it is gone.
        if p.unit.phase == Phase::Leaving
            && let Some(c) = &p.unit.cargo
            && Vec2::new(p.unit.pos.x, p.unit.pos.z).distance(c.drop_at) > 3500.0
        {
            p.gone = true;
        }
    }
    // A plane's load counts toward the wave it was sent with.
    for (kind, at, vel, wave) in dropped {
        let Some(spec) = cfg.units.get(&kind) else { continue };
        units.add(|id| Unit::dropped(id, &kind, spec, at, vel), wave, None);
    }
}

/// A vehicle was destroyed: the blast and the wreck's fire, and it leaves the world.
pub fn unit_destroyed(p: &mut Placed, data: &Data, fx: &mut MessageWriter<Fx>, explosions: &mut MessageWriter<crate::blast::Explosion>) {
    let air = data.0.units.units.get(&p.unit.kind).is_some_and(|s| s.kind != UnitKind::Tank);
    fx.write(Fx::new(if air { "explosion_air" } else { "explosion_ground" }, p.unit.pos));
    explosions.write(crate::blast::Explosion {
        at: p.unit.pos,
        energy_mj: if air { 1.5 } else { 4.0 },
        by: None,
    });
    p.gone = true;
}

/// The hit shapes of every live vehicle: one capsule along its body.
pub fn unit_capsules(units: &Units, data: &Data) -> Vec<cb_core::weapons::PartCapsule> {
    units
        .list
        .iter()
        .filter(|p| p.unit.alive() && !p.gone)
        .filter_map(|p| {
            let s = data.0.units.units.get(&p.unit.kind)?;
            let fwd = cb_core::units::forward(p.unit.heading_rad);
            let half = (s.size_m[2] * 0.5 - s.size_m[1] * 0.5).max(0.0);
            Some(cb_core::weapons::PartCapsule {
                mech: p.unit.id,
                part: 0,
                a: p.unit.pos - fwd * half,
                b: p.unit.pos + fwd * half,
                radius_m: s.size_m[0].max(s.size_m[1]) * 0.5,
            })
        })
        .collect()
}

/// Placeholder bodies: built when a vehicle first appears, moved every frame, removed when gone.
fn draw_units(
    mut commands: Commands,
    units: Res<Units>,
    data: Res<Data>,
    mut drawn: Query<(Entity, &UnitDraw, &mut Transform)>,
    mut vis: Query<&mut Visibility>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut known: Local<BTreeMap<u32, Entity>>,
) {
    let _t = crate::perf_hud::slow("draw_units");
    let mut seen = std::collections::BTreeSet::new();
    for (e, d, mut t) in &mut drawn {
        let Some(p) = units.list.iter().find(|p| p.unit.id == d.id) else {
            continue;
        };
        if p.gone || !p.unit.alive() {
            commands.entity(e).despawn();
            continue;
        }
        seen.insert(d.id);
        *t = Transform::from_translation(p.unit.pos).with_rotation(Quat::from_rotation_y(p.unit.heading_rad));
        if let Some(c) = d.chute
            && let Ok(mut v) = vis.get_mut(c)
        {
            let open = matches!(p.unit.phase, Phase::Dropping { t_s } if t_s >= data.0.units.drop.chute_open_s);
            *v = if open { Visibility::Inherited } else { Visibility::Hidden };
        }
    }
    known.retain(|id, _| seen.contains(id));
    for p in units
        .list
        .iter()
        .filter(|p| !p.gone && p.unit.alive() && !seen.contains(&p.unit.id))
    {
        if known.contains_key(&p.unit.id) {
            continue;
        }
        let Some(s) = data.0.units.units.get(&p.unit.kind) else { continue };
        let paint = materials.add(StandardMaterial {
            base_color: Color::srgb(s.tint[0], s.tint[1], s.tint[2]),
            perceptual_roughness: 0.7,
            metallic: 0.3,
            ..default()
        });
        let dark = materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.12, 0.12),
            perceptual_roughness: 0.6,
            ..default()
        });
        let [w, h, l] = s.size_m;
        let mut parts: Vec<(Mesh, Transform, bool)> = Vec::new();
        match s.kind {
            UnitKind::Tank => {
                parts.push((Cuboid::new(w, h * 0.55, l).into(), Transform::from_xyz(0.0, -h * 0.2, 0.0), false));
                parts.push((
                    Cuboid::new(w * 0.65, h * 0.4, l * 0.45).into(),
                    Transform::from_xyz(0.0, h * 0.27, 0.3),
                    false,
                ));
                parts.push((
                    Cylinder::new(0.15, l * 0.7).into(),
                    Transform::from_xyz(0.0, h * 0.3, -l * 0.45).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    true,
                ));
            }
            UnitKind::Helicopter => {
                parts.push((
                    Cuboid::new(w, h * 0.6, l * 0.4).into(),
                    Transform::from_xyz(0.0, -h * 0.1, -l * 0.15),
                    false,
                ));
                parts.push((
                    Cuboid::new(w * 0.25, w * 0.25, l * 0.6).into(),
                    Transform::from_xyz(0.0, 0.0, l * 0.3),
                    false,
                ));
                parts.push((
                    Cylinder::new(l * 0.5, 0.08).into(),
                    Transform::from_xyz(0.0, h * 0.35, -l * 0.15),
                    true,
                ));
            }
            UnitKind::Fighter => {
                parts.push((Cuboid::new(1.6, h * 0.6, l).into(), Transform::IDENTITY, false));
                parts.push((Cuboid::new(w, 0.25, l * 0.3).into(), Transform::from_xyz(0.0, 0.0, l * 0.05), false));
                parts.push((
                    Cuboid::new(0.2, h * 0.8, l * 0.15).into(),
                    Transform::from_xyz(0.0, h * 0.4, l * 0.4),
                    false,
                ));
            }
            UnitKind::CargoPlane => {
                parts.push((Cuboid::new(w * 0.15, h, l).into(), Transform::IDENTITY, false));
                parts.push((
                    Cuboid::new(w, 0.6, l * 0.15).into(),
                    Transform::from_xyz(0.0, h * 0.4, -l * 0.05),
                    false,
                ));
                parts.push((
                    Cuboid::new(w * 0.35, 0.4, l * 0.1).into(),
                    Transform::from_xyz(0.0, h * 0.4, l * 0.45),
                    false,
                ));
            }
        }
        let root = commands
            .spawn((
                Name::new(format!("unit {} {}", p.unit.kind, p.unit.id)),
                Transform::from_translation(p.unit.pos).with_rotation(Quat::from_rotation_y(p.unit.heading_rad)),
                Visibility::default(),
            ))
            .id();
        for (mesh, t, is_dark) in parts {
            let m = meshes.add(mesh);
            let mat = if is_dark { dark.clone() } else { paint.clone() };
            commands.spawn((Mesh3d(m), MeshMaterial3d(mat), t, ChildOf(root)));
        }
        // A tank comes down under a chute: a dome over it on lines, shown while it is open.
        let chute = (s.kind == UnitKind::Tank).then(|| {
            let cloth = materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.82, 0.7),
                perceptual_roughness: 0.9,
                cull_mode: None,
                ..default()
            });
            let r = l.max(w) * 1.2;
            commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(r).mesh().uv(24, 12))),
                    MeshMaterial3d(cloth),
                    Transform::from_xyz(0.0, h + r * 1.4, 0.0).with_scale(Vec3::new(1.0, 0.45, 1.0)),
                    Visibility::Hidden,
                    ChildOf(root),
                ))
                .id()
        });
        commands.entity(root).insert(UnitDraw { id: p.unit.id, chute });
        known.insert(p.unit.id, root);
    }
}
