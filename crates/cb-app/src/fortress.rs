//! The Hermit, the crab air fortress, in the game (openspec change `crab-airship` section 9): the mission free
//! roam's mail starts (`free-roam-missions`). It spawns the model exported from the mockup over the city nearest
//! the pilot, steps `cb_core::fortress` every fixed step, and acts on what it says: its parts' hit capsules and
//! its hull's join the mechs' in `combat` (a hit is taken by the part's own armour rule), its turrets fire through
//! the one `Shots`, its beams burn, its catapults launch `fighter` units, its deck, shoulders and walkways are
//! platforms that carry what stands on them, its downwash pushes and burns, a lost thruster's leg breaks off and
//! falls, and when it can no longer hold its weight it lists, sinks and crashes on what is under it. The rules
//! are in `cb-core`; the cockpit's readout reads `FortressRun`.

use crate::blast::Explosion;
use crate::combat::Shots;
use crate::fx::{Fx, FxStream};
use crate::mech::{MechId, MechSim, MechSpec, Piloted, apply_hit};
use crate::missions::MissionSwitch;
use crate::pilot::PilotSim;
use crate::sim::{Data, PresentSet, SimSet};
use crate::state::GameApp;
use crate::units::{Hostile, Hostiles, Units};
use crate::world::{Live, Plan};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use cb_core::fortress::{Beam, BeamPhase, Event, Fortress, FortressData, HitOutcome, Mount, Target};
use cb_core::mech::{Hit, PartKind};
use cb_core::missions::MissionKind;
use cb_core::pilot::PilotMode;
use cb_core::units::{Unit, heading_of};
use cb_core::weapons::{PartCapsule, Projectile};
use cb_core::world::{Ground, Platform};
use std::collections::HashMap;

/// The fortress's hit capsules and its rounds' owner: one id, so its own rounds pass its own hull as they leave.
pub const FORTRESS_ID: u32 = 900_000;
/// A part as a target (boxes, locks, missiles): this plus the part's index.
pub const FORTRESS_TARGET_BASE: u32 = 900_001;
/// Hull capsules' part numbers start here (they stop rounds; they have no hit points).
const HULL_PART_BASE: usize = 1000;
const G_M_S2: f32 = 9.81;

/// A round struck one of the fortress's capsules (from `combat`).
#[derive(Message, Clone, Copy, Debug)]
pub struct FortressHit {
    /// The part's index, or `HULL_PART_BASE` and up for the hull.
    pub part: usize,
    /// Where, world.
    pub at: Vec3,
    /// The round's direction, world.
    pub dir: Vec3,
    /// Its penetration, millimetres.
    pub penetration_mm: f32,
    /// Its damage.
    pub damage: f32,
}

/// The model's nodes the game drives, by part index.
struct Nodes {
    part: Vec<Option<Entity>>,
    yaw: Vec<Option<Entity>>,
    pitch: Vec<Option<Entity>>,
    muzzle: Vec<Option<Entity>>,
    leg: Vec<Option<Entity>>,
    /// Each hull door, its rest place and which way it slides.
    doors: Vec<(Entity, Vec3, f32)>,
    /// Each thruster's plume and whether it is a hull thruster's.
    plumes: Vec<(usize, Entity, bool)>,
}

/// A leg falling after its thruster was destroyed.
struct FallingLeg {
    pivot: Entity,
    pos: Vec3,
    vel: Vec3,
    rot: Quat,
    spin: Vec3,
}

/// The fortress mission as it stands.
#[derive(Resource)]
pub struct FortressRun {
    /// The rules' state.
    pub f: Fortress,
    root: Entity,
    nodes: Option<Nodes>,
    beam_draws: Vec<Entity>,
    /// Won (`Some(true)`, it crashed) or lost (`Some(false)`, the pilot's mech went down).
    pub outcome: Option<bool>,
    status_in_s: f32,
    /// The beams this step.
    pub beams: Vec<Beam>,
    /// Hit capsules this step: every live part's, then the hull's.
    pub capsules: Vec<PartCapsule>,
    /// Every live part as a target: (id, where).
    pub targets: Vec<(u32, Vec3)>,
    legs: Vec<FallingLeg>,
    was: bevy::math::Affine3A,
}

impl FortressRun {
    /// The model has spawned and its nodes are found.
    pub fn bound(&self) -> bool {
        self.nodes.is_some()
    }

    /// Part `i`'s node.
    pub fn part_node(&self, i: usize) -> Option<Entity> {
        self.nodes.as_ref().and_then(|n| n.part.get(i).copied().flatten())
    }
}

/// Runs the fortress mission.
pub struct FortressPlugin;

impl Plugin for FortressPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FortressHit>()
            .add_game_systems(Update, switch.after(crate::missions::MissionsSet).before(PresentSet::Pose))
            // each checks for the run itself: a switch can end it between the two
            .add_game_systems(
                Update,
                (bind, pose)
                    .chain()
                    .in_set(PresentSet::Pose)
                    .distributive_run_if(resource_exists::<FortressRun>),
            )
            .add_game_systems(
                FixedUpdate,
                hostiles
                    .in_set(SimSet::Input)
                    .after(crate::units::list_hostiles)
                    .run_if(resource_exists::<FortressRun>),
            )
            .add_game_systems(FixedUpdate, step.in_set(SimSet::World).run_if(resource_exists::<FortressRun>))
            .add_game_systems(
                FixedUpdate,
                hits.in_set(SimSet::Mech)
                    .after(crate::combat::fly_shots)
                    .run_if(resource_exists::<FortressRun>),
            );
    }
}

fn say(line: &str) {
    println!("{line}");
}

/// Begins and ends the mission on free roam's switch.
#[allow(clippy::too_many_arguments)]
fn switch(
    mut commands: Commands,
    mut switches: MessageReader<MissionSwitch>,
    run: Option<Res<FortressRun>>,
    data: Res<Data>,
    plan: Res<Plan>,
    pilot: Res<PilotSim>,
    me: Query<&MechSim, With<Piloted>>,
    mut units: ResMut<Units>,
    mut live: ResMut<Live>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut have = run
        .as_ref()
        .map(|r| (r.root, r.beam_draws.clone(), r.legs.iter().map(|l| l.pivot).collect::<Vec<_>>()));
    for s in switches.read() {
        if s.stop == Some(MissionKind::Fortress)
            && let Some((root, beams, legs)) = have.take()
        {
            for e in std::iter::once(root).chain(beams).chain(legs) {
                commands.entity(e).despawn();
            }
            units.retire(MissionKind::Fortress);
            live.platforms.retain(cb_core::world::Platform::is_fixed);
            commands.remove_resource::<FortressRun>();
            say("MISSION fortress turned off");
        }
        if s.start != Some(MissionKind::Fortress) {
            continue;
        }
        let d = &data.0.fortress;
        // Over the city nearest the pilot (or the mech they sit in).
        let at = me
            .iter()
            .next()
            .filter(|_| pilot.curr.mode == PilotMode::Seated)
            .map_or(pilot.curr.feet, |m| m.curr.pos);
        let Some(city) = plan
            .cities
            .iter()
            .min_by(|a, b| a.centre.distance(at.xz()).total_cmp(&b.centre.distance(at.xz())))
        else {
            continue;
        };
        let f = Fortress::new(d, city.centre, city.ground_m, plan.seed);
        let pose = f.pose();
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(d.shape.model.clone()));
        let root = commands
            .spawn((
                Name::new("Hermit"),
                Transform::from_matrix(Mat4::from(pose)),
                Visibility::default(),
                WorldAssetRoot(scene),
            ))
            .id();
        // The beams, one drawn cylinder each, hidden until a beam charges.
        let beam_mesh = meshes.add(Cylinder::new(1.0, 1.0));
        let beam_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.25, 0.18, 0.6),
            emissive: LinearRgba::rgb(40.0, 6.0, 4.0),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            ..default()
        });
        let beam_draws = (0..d.shape.parts.len())
            .filter(|&i| d.kind(i).gun.as_ref().is_some_and(|g| g.beam.is_some()))
            .map(|_| {
                commands
                    .spawn((
                        Name::new("Hermit beam"),
                        Mesh3d(beam_mesh.clone()),
                        MeshMaterial3d(beam_mat.clone()),
                        Transform::default(),
                        Visibility::Hidden,
                    ))
                    .id()
            })
            .collect();
        say(&format!(
            "MISSION fortress begins: the Hermit over city {} at ({:.0}, {:.0}), {:.0} m up",
            city.id + 1,
            f.pos.x,
            f.pos.z,
            d.cfg.altitude_m
        ));
        commands.insert_resource(FortressRun {
            was: pose,
            f,
            root,
            nodes: None,
            beam_draws,
            outcome: None,
            status_in_s: 0.0,
            beams: Vec::new(),
            capsules: Vec::new(),
            targets: Vec::new(),
            legs: Vec::new(),
        });
        have = None;
    }
}

/// Finds the nodes the game drives once the model has spawned, and hangs the thrusters' plumes on them.
#[allow(clippy::too_many_arguments)]
fn bind(
    mut commands: Commands,
    mut run: ResMut<FortressRun>,
    data: Res<Data>,
    children: Query<&Children>,
    names: Query<(&Name, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if run.nodes.is_some() {
        return;
    }
    let d = &data.0.fortress;
    let mut by_name: HashMap<&str, (Entity, Vec3)> = HashMap::new();
    for e in children.iter_descendants(run.root) {
        if let Ok((n, t)) = names.get(e) {
            by_name.insert(n.as_str(), (e, t.translation));
        }
    }
    let n = d.shape.parts.len();
    let find = |prefix: &str, i: usize| by_name.get(format!("{prefix}_{}", d.shape.parts[i].id).as_str()).map(|x| x.0);
    let part: Vec<Option<Entity>> = (0..n).map(|i| find("PART", i)).collect();
    if part.iter().filter(|p| p.is_some()).count() < n {
        return; // not spawned yet
    }
    // The detail (the spikes) draws only up close: Bevy's visibility range on every mesh under a DETAIL_ node,
    // fading out over the last 100 m (design section 9, its levels of detail).
    let range = d.cfg.detail_range_m;
    let fade = bevy::camera::visibility::VisibilityRange {
        start_margin: 0.0..0.0,
        end_margin: (range - 100.0).max(0.0)..range,
        use_aabb: false,
    };
    let mut detail = 0;
    for (_, (e, _)) in by_name.iter().filter(|(k, _)| k.starts_with("DETAIL_")) {
        for m in std::iter::once(*e).chain(children.iter_descendants(*e)) {
            commands.entity(m).insert(fade.clone());
            detail += 1;
        }
    }
    let doors = by_name
        .iter()
        .filter(|(k, _)| k.starts_with("DOOR_"))
        .map(|(k, (e, t))| (*e, *t, if k.ends_with("_a") { 1.0 } else { -1.0 }))
        .collect();
    // The plumes: a glowing cone under each lift thruster's nozzle and each hull thruster's lower port.
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.6, 1.0, 0.3, 0.35),
        emissive: LinearRgba::rgb(4.0, 12.0, 1.5),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let cone = meshes.add(Cone { radius: 1.0, height: 1.0 });
    let mut plumes = Vec::new();
    for (i, r) in d.shape.parts.iter().enumerate() {
        let (hull, below, radius, length) = match r.kind.as_str() {
            "lift" => (false, 20.0, 11.0, 46.0),
            "hull" => (true, 40.0, 10.0, 44.0),
            _ => continue,
        };
        let Some(p) = part[i] else { continue };
        let e = commands
            .spawn((
                Name::new(format!("plume {}", r.id)),
                Mesh3d(cone.clone()),
                MeshMaterial3d(glow.clone()),
                // the cone's tip down, its base at the nozzle
                Transform::from_xyz(0.0, -below - length * 0.5, 0.0)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::PI))
                    .with_scale(Vec3::new(radius, length, radius)),
                if hull { Visibility::Hidden } else { Visibility::Inherited },
                ChildOf(p),
            ))
            .id();
        plumes.push((i, e, hull));
    }
    run.nodes = Some(Nodes {
        yaw: (0..n).map(|i| find("YAW", i)).collect(),
        pitch: (0..n).map(|i| find("PITCH", i)).collect(),
        muzzle: (0..n).map(|i| find("MUZZLE", i)).collect(),
        leg: (0..n).map(|i| find("LEG", i)).collect(),
        part,
        doors,
        plumes,
    });
    info!("fortress: model bound, {n} parts, {detail} detail entities drawn within {range} m");
}

/// The model follows the rules: the hull's pose, each turret's aim, the doors, the plumes, the beams, lost parts.
fn pose(run: Res<FortressRun>, data: Res<Data>, mut transforms: Query<&mut Transform>, mut vis: Query<&mut Visibility>) {
    let d = &data.0.fortress;
    if let Ok(mut t) = transforms.get_mut(run.root) {
        *t = Transform::from_matrix(Mat4::from(run.f.pose()));
    }
    for l in &run.legs {
        if let Ok(mut t) = transforms.get_mut(l.pivot) {
            *t = Transform::from_translation(l.pos).with_rotation(l.rot);
        }
    }
    let Some(n) = &run.nodes else { return };
    for (i, st) in run.f.parts.iter().enumerate() {
        if let Some(e) = n.yaw[i]
            && let Ok(mut t) = transforms.get_mut(e)
        {
            t.rotation = Quat::from_rotation_y(st.gun.yaw_rad);
        }
        if let Some(e) = n.pitch[i]
            && let Ok(mut t) = transforms.get_mut(e)
        {
            t.rotation = Quat::from_rotation_x(-st.gun.elev_rad);
        }
        // A destroyed turret, bay or lift is gone from its mount (a thruster's leg falls on its own).
        if !st.alive
            && d.shape.parts[i].kind != "lift"
            && let Some(e) = n.part[i]
            && let Ok(mut v) = vis.get_mut(e)
        {
            *v = Visibility::Hidden;
        }
    }
    let open = run.f.doors(&d.cfg);
    let ease = open * open * (3.0 - 2.0 * open);
    for (e, rest, dz) in &n.doors {
        if let Ok(mut t) = transforms.get_mut(*e) {
            t.translation = *rest + Vec3::Z * (dz * 15.0 * ease);
        }
    }
    let lift = run.f.lift(d);
    for (i, e, hull) in &n.plumes {
        let side = usize::from(d.shape.parts[*i].side < 0);
        let throttle = if *hull {
            lift.throttle_hull[side]
        } else {
            lift.throttle_outer[side]
        };
        let on = run.f.parts[*i].alive && !run.f.crashed && throttle > 0.0;
        if let Ok(mut v) = vis.get_mut(*e) {
            *v = if on { Visibility::Inherited } else { Visibility::Hidden };
        }
        if let Ok(mut t) = transforms.get_mut(*e) {
            let length = if *hull { 44.0 } else { 46.0 } * (0.6 + 0.6 * throttle);
            t.scale.y = length;
            t.translation.y = -(if *hull { 40.0 } else { 20.0 }) - length * 0.5;
        }
    }
    for (k, e) in run.beam_draws.iter().enumerate() {
        let b = run.beams.get(k);
        if let Ok(mut v) = vis.get_mut(*e) {
            *v = if b.is_some() { Visibility::Visible } else { Visibility::Hidden };
        }
        if let (Some(b), Ok(mut t)) = (b, transforms.get_mut(*e)) {
            let dir = b.to - b.from;
            let r = if b.phase == BeamPhase::On { 2.2 } else { 0.35 };
            *t = Transform::from_translation((b.from + b.to) * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, dir.normalize_or(Vec3::Y)))
                .with_scale(Vec3::new(r, dir.length().max(0.1), r));
        }
    }
}

/// Every live part is a target, boxed and lockable like a mech (`targeting`).
fn hostiles(run: Res<FortressRun>, mut out: ResMut<Hostiles>) {
    let vel = run.f.velocity();
    for (id, at) in &run.targets {
        out.0.push(Hostile {
            id: *id,
            aim: *at,
            feet: *at - Vec3::Y * 8.0,
            height_m: 16.0,
            vel,
            shelling: false,
            wave: None,
        });
    }
    out.0.sort_by_key(|h| h.id);
}

/// One fixed step of the fortress: the rules, its platforms carrying what stands on them, launches, the guns,
/// the beams, the downwash, the falling legs, the crash, and its hit capsules for this step's rounds.
#[allow(clippy::too_many_arguments)]
fn step(
    mut commands: Commands,
    mut run: ResMut<FortressRun>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Fixed>>,
    globals: Query<&GlobalTransform>,
    mut mechs: Query<(&mut MechSim, &MechSpec, &MechId, Has<Piloted>)>,
    mut pilot: ResMut<PilotSim>,
    mut live: ResMut<Live>,
    mut units: ResMut<Units>,
    mut shots: ResMut<Shots>,
    mut fx: MessageWriter<Fx>,
    mut explosions: MessageWriter<Explosion>,
) {
    let d = &data.0.fortress;
    let dt = time.delta_secs();
    let run = &mut *run;
    let ground = |x: f32, z: f32| plan.height_at(x, z);
    run.was = run.f.pose();
    for e in run.f.step(d, &ground, dt) {
        match e {
            Event::DoorsOpening => say("MISSION fortress: a side can no longer hold; the hull doors open"),
            Event::HullLit => say("MISSION fortress: the hull thrusters light"),
            Event::Falling => say("MISSION fortress: it cannot hold its weight; it is falling"),
            Event::Crashed => crash(run, d, &plan, &mut live, &mut fx, &mut explosions),
        }
    }
    // Its platforms, posed this step; what stood on them moves with them.
    let platform = Platform {
        slabs: d.shape.platforms.clone(),
        pose: run.f.pose(),
        was: run.was,
    };
    for (mut sim, ..) in &mut mechs {
        if let Some(p) = platform.carry(sim.curr.pos) {
            sim.curr.pos = p;
        }
    }
    if pilot.curr.mode != PilotMode::Seated
        && let Some(p) = platform.carry(pilot.curr.feet)
    {
        pilot.curr.feet = p;
    }
    // Its own platform replaces last step's; one built in place (a harbour's quay) stays.
    live.platforms.retain(Platform::is_fixed);
    live.platforms.push(platform);
    let at = |e: Option<Entity>| e.and_then(|e| globals.get(e).ok());
    let Some(nodes) = &run.nodes else { return };
    // Fighters off the catapults.
    if let Some(cat) = run.f.launch(d, units.count_for(MissionKind::Fortress), dt)
        && let (Some(g), Some(spec)) = (at(nodes.part[cat]), data.0.units.units.get(&d.cfg.launch.unit))
    {
        let from = g.transform_point(d.cfg.launch.off_m);
        let heading = heading_of(run.f.rotation() * Vec3::Z);
        let kind = d.cfg.launch.unit.clone();
        units.add_for(
            |id| Unit::new(id, &kind, spec, from, heading, id),
            None,
            None,
            Some(MissionKind::Fortress),
        );
        fx.write(Fx::new("vent_steam", from));
        say(&format!("MISSION fortress: a fighter launches off {}", d.shape.parts[cat].id));
    }
    // The pilot's mech is what the defences shoot at, while the pilot sits in it: an empty mech is no threat.
    let seated = pilot.curr.mode == PilotMode::Seated;
    let me = mechs.iter().find(|m| m.3 && !m.0.dead && seated).map(|(m, s, id, _)| {
        (
            m.curr.pos + Vec3::Y * s.0.aim_height_m(),
            Vec3::new(m.curr.vel.x, m.curr.vel_y_m_s, m.curr.vel.y),
            id.0,
            s.0.parts.iter().position(|p| p.kind == PartKind::Torso),
        )
    });
    let mounts: Vec<Option<Mount>> = (0..d.shape.parts.len())
        .map(|i| {
            let g = d.kind(i).gun.as_ref()?;
            if g.turns {
                Some(Mount {
                    pivot: at(nodes.yaw[i])?.translation(),
                    muzzle: at(nodes.muzzle[i])?.translation(),
                })
            } else {
                let p = at(nodes.part[i])?;
                Some(Mount {
                    pivot: p.translation(),
                    muzzle: p.transform_point(Vec3::Y * 6.0),
                })
            }
        })
        .collect();
    let target = me.map(|(pos, vel, ..)| Target { pos, vel });
    let (fires, beams) = run.f.guns(d, &mounts, target, dt);
    for f in fires {
        let Some(g) = d.kind(f.part).gun.as_ref() else { continue };
        let w = &g.weapon;
        shots.0.push(Projectile {
            owner: FORTRESS_ID,
            weapon: f.part,
            pos: f.from,
            vel: f.dir * w.speed_m_s,
            age_s: 0.0,
            rocket: w.rocket,
            target: (w.seeker_turn_deg_s > 0.0).then(|| me.map(|m| m.2)).flatten(),
        });
        let flash = if w.rocket {
            "muzzle_rocket"
        } else if w.blast_mj >= 1.0 {
            "muzzle_cannon"
        } else {
            "muzzle_autocannon"
        };
        fx.write(Fx::new(flash, f.from).toward(f.dir).scaled(2.0));
    }
    // Beams and the downwash burn the pilot's mech.
    let nozzles: Vec<(usize, Vec3)> = (0..d.shape.parts.len())
        .filter(|&i| d.shape.parts[i].kind == "lift")
        .filter_map(|i| Some((i, at(nodes.part[i])?.transform_point(Vec3::Y * -20.0))))
        .collect();
    let burn: f32 = beams.iter().map(|b| b.damage).sum();
    if let Some((pos, _, id, torso)) = me
        && let Some((mut sim, spec, ..)) = mechs.iter_mut().find(|m| m.2.0 == id)
    {
        let mut hurt = burn;
        if let Some((push, burn_per_s)) = run.f.downwash(d, &nozzles, pos) {
            sim.curr.vel_y_m_s -= push * dt;
            hurt += burn_per_s * dt;
        }
        if let Some(torso) = torso
            && hurt > 0.0
        {
            apply_hit(
                &mut sim,
                &spec.0.clone(),
                Hit {
                    part: torso,
                    penetration_mm: f32::MAX,
                    damage: hurt,
                },
            );
        }
    }
    run.beams = beams;
    // The falling legs.
    let mut landed = Vec::new();
    for (k, l) in run.legs.iter_mut().enumerate() {
        l.vel.y -= G_M_S2 * dt;
        l.pos += l.vel * dt;
        let spin = l.spin * dt;
        l.rot = Quat::from_scaled_axis(spin) * l.rot;
        if l.pos.y <= plan.height_at(l.pos.x, l.pos.z) + 15.0 {
            landed.push(k);
        }
    }
    for k in landed.into_iter().rev() {
        let l = run.legs.remove(k);
        let r = d.cfg.leg.break_radius_m;
        let hit = Vec3::new(l.pos.x, plan.height_at(l.pos.x, l.pos.z), l.pos.z);
        break_round(&plan, &mut live, hit, r);
        explosions.write(Explosion {
            at: hit,
            energy_mj: d.cfg.leg.blast_mj,
        });
        fx.write(Fx::new("explosion_ground", hit).scaled(d.cfg.leg.blast_mj.cbrt()));
        say("MISSION fortress: a leg hits the ground and explodes");
        commands.entity(l.pivot).despawn();
    }
    // The pilot's mech down: lost.
    if run.outcome.is_none() && mechs.iter().any(|m| m.3 && m.0.dead) {
        run.outcome = Some(false);
        say("MISSION fortress lost: your mech was destroyed");
    }
    // Hit capsules and targets for this step's rounds, every live part's, then the hull's.
    run.capsules.clear();
    run.targets.clear();
    if !run.f.crashed {
        for (i, st) in run.f.parts.iter().enumerate() {
            let (true, Some(g)) = (st.alive, at(nodes.part[i])) else { continue };
            let c = d.kind(i).capsule;
            let mid = g.transform_point(Vec3::Y * c.centre_y_m);
            let axis = g.rotation() * Vec3::Y * c.half_m;
            run.capsules.push(PartCapsule {
                mech: FORTRESS_ID,
                part: i,
                a: mid - axis,
                b: mid + axis,
                radius_m: c.radius_m,
            });
            run.targets.push((FORTRESS_TARGET_BASE + i as u32, mid));
        }
    }
    let pose = run.f.pose();
    for (k, h) in d.shape.hull.iter().enumerate() {
        run.capsules.push(PartCapsule {
            mech: FORTRESS_ID,
            part: HULL_PART_BASE + k,
            a: pose.transform_point3(h.a),
            b: pose.transform_point3(h.b),
            radius_m: h.radius_m,
        });
    }
    run.status_in_s -= dt;
    if run.status_in_s <= 0.0 && !run.f.crashed {
        run.status_in_s = 10.0;
        let l = run.f.lift(d);
        say(&format!(
            "MISSION fortress status: lift port {:.0} % stbd {:.0} %, alt {:.0} m, list {:.1} deg{}",
            l.port * 200.0,
            l.stbd * 200.0,
            run.f.pos.y - run.f.ground_m,
            run.f.list_rad.to_degrees(),
            if run.f.falling { ", FALLING" } else { "" }
        ));
    }
}

/// Breaks the building cells within `radius_m` of `at`.
fn break_round(plan: &Plan, live: &mut Live, at: Vec3, radius_m: f32) {
    for b in plan.collision.candidates(at.xz(), radius_m) {
        let (broke, chunks) = live.structures.break_sphere(b, at, radius_m);
        if broke > 0 {
            live.dirty_buildings.push(b);
            live.new_chunks.extend(chunks);
        }
    }
}

/// It hit the ground: the blast, and every building under it and round it brought down (its ground floor
/// broken, so the structure falls: `structural-destruction`).
fn crash(
    run: &mut FortressRun,
    d: &FortressData,
    plan: &Plan,
    live: &mut Live,
    fx: &mut MessageWriter<Fx>,
    explosions: &mut MessageWriter<Explosion>,
) {
    let at = run.f.pos;
    let reach = 220.0 + d.cfg.crash.break_margin_m;
    let inv = run.f.pose().inverse();
    let mut n = 0;
    for b in plan.collision.candidates(at.xz(), reach) {
        let c = plan.buildings[b as usize].footprint.centre();
        let local = inv.transform_point3(Vec3::new(c.x, at.y, c.y));
        let (ax, az) = (140.0 + d.cfg.crash.break_margin_m, 210.0 + d.cfg.crash.break_margin_m);
        if (local.x / ax).powi(2) + (local.z / az).powi(2) > 1.0 {
            continue;
        }
        let st = &live.structures.buildings[b as usize];
        let base = Vec3::new(c.x, st.origin.y + 2.0, c.y);
        let half = (st.size.x * st.n[0] as f32).max(st.size.z * st.n[2] as f32) * 0.75;
        let (broke, chunks) = live.structures.break_sphere(b, base, half.max(6.0));
        if broke > 0 {
            live.dirty_buildings.push(b);
            live.new_chunks.extend(chunks);
            n += 1;
        }
    }
    let ground = Vec3::new(at.x, plan.height_at(at.x, at.z), at.z);
    explosions.write(Explosion {
        at: ground,
        energy_mj: d.cfg.crash.blast_mj,
    });
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU;
        let p = ground + Vec3::new(a.cos() * 120.0, 0.0, a.sin() * 160.0);
        fx.write(Fx::new("explosion_ground", p).scaled(d.cfg.crash.blast_mj.cbrt() * 0.6));
    }
    fx.write(Fx::new("explosion_ground", ground).scaled(d.cfg.crash.blast_mj.cbrt()));
    run.outcome.get_or_insert(true);
    say(&format!(
        "MISSION fortress won: the Hermit is down, {n} buildings under it brought down"
    ));
}

/// Rounds that struck the fortress take its armour rule; flak bursts near the pilot's mech.
#[allow(clippy::too_many_arguments)]
fn hits(
    mut commands: Commands,
    mut run: ResMut<FortressRun>,
    data: Res<Data>,
    mut incoming: MessageReader<FortressHit>,
    globals: Query<&GlobalTransform>,
    me: Query<(&MechSim, &MechSpec), With<Piloted>>,
    mut shots: ResMut<Shots>,
    mut fx: MessageWriter<Fx>,
    mut explosions: MessageWriter<Explosion>,
) {
    let d = &data.0.fortress;
    let run = &mut *run;
    for h in incoming.read() {
        fx.write(Fx::new("impact_mech", h.at).toward(-h.dir).scaled(1.5));
        if h.part >= HULL_PART_BASE {
            continue; // the hull: 300 mm, no hit points
        }
        let Some(node) = run.nodes.as_ref().and_then(|n| n.part.get(h.part).copied().flatten()) else {
            continue;
        };
        let Ok(g) = globals.get(node) else { continue };
        let inv = g.affine().inverse();
        let local_at = inv.transform_point3(h.at);
        let local_dir = inv.transform_vector3(h.dir);
        let out = run.f.hit(d, h.part, local_at, local_dir, h.penetration_mm, h.damage);
        let name = &d.kind(h.part).name;
        let id = &d.shape.parts[h.part].id;
        match out {
            HitOutcome::Destroyed { .. } => {
                say(&format!("MISSION fortress: {name} {id} destroyed"));
                let at = g.translation();
                explosions.write(Explosion { at, energy_mj: 8.0 });
                fx.write(Fx::new("explosion_air", at).scaled(3.0));
                if d.shape.parts[h.part].kind == "lift" {
                    break_off(&mut commands, run, d, h.part, &globals);
                } else {
                    // it burns where it stood, riding the hull
                    let local = run.f.pose().inverse().transform_point3(at);
                    commands.spawn((
                        FxStream {
                            recipe: "burning_hull",
                            dir: Vec3::Y,
                            on: true,
                            density: 1.0,
                        },
                        Transform::from_translation(local).with_scale(Vec3::splat(3.0)),
                        ChildOf(run.root),
                    ));
                }
            }
            HitOutcome::NoPen { armour_mm } => debug!("fortress: {id} no pen ({armour_mm} mm)"),
            HitOutcome::Hurt { damage, weak } => debug!("fortress: {id} -{damage}{}", if weak { " weak point" } else { "" }),
            HitOutcome::Wreck => {}
        }
    }
    // Flak bursts within its fuse of the pilot's mech.
    // the chest, where every shooter aims (`MechConfig::aim_height_m`)
    let Some(torso) = me
        .iter()
        .find(|(m, _)| !m.dead)
        .map(|(m, s)| m.curr.pos + Vec3::Y * s.0.aim_height_m())
    else {
        return;
    };
    shots.0.retain(|s| {
        if s.owner != FORTRESS_ID {
            return true;
        }
        let Some(g) = d.kind(s.weapon).gun.as_ref().filter(|g| g.proximity_m > 0.0) else {
            return true;
        };
        if s.pos.distance(torso) > g.proximity_m {
            return true;
        }
        explosions.write(Explosion {
            at: s.pos,
            energy_mj: g.weapon.blast_mj,
        });
        fx.write(Fx::new("explosion_air", s.pos));
        false
    });
}

/// A destroyed lift thruster's leg breaks off at the knee and falls (owner, 2026-10-25: "when a pylon is destroyed
/// it should fall off and fall onto the ground and explode").
fn break_off(commands: &mut Commands, run: &mut FortressRun, d: &FortressData, part: usize, globals: &Query<&GlobalTransform>) {
    let Some(nodes) = &run.nodes else { return };
    let (Some(leg), Some(nacelle)) = (nodes.leg[part], nodes.part[part]) else {
        return;
    };
    let (Ok(lg), Ok(ng)) = (globals.get(leg), globals.get(nacelle)) else {
        return;
    };
    // Turn about the nacelle, not the fortress's middle: a pivot there carries the leg.
    let pos = ng.translation();
    let rot = run.f.rotation();
    let pivot = commands
        .spawn((
            Name::new("falling leg"),
            Transform::from_translation(pos).with_rotation(rot),
            Visibility::default(),
        ))
        .id();
    let pivot_inv = bevy::math::Affine3A::from_rotation_translation(rot, pos).inverse();
    let rel = Transform::from_matrix(Mat4::from(pivot_inv * lg.affine()));
    commands.entity(leg).insert((ChildOf(pivot), rel));
    // It falls away from the hull, tumbling outboard about the fortress's length.
    let side = f32::from(d.shape.parts[part].side);
    let out = rot * Vec3::X * side;
    run.legs.push(FallingLeg {
        pivot,
        pos,
        vel: run.f.velocity() + out * 4.0,
        rot,
        spin: rot * Vec3::Z * (-side * d.cfg.leg.tumble_deg_s.to_radians()),
    });
}
