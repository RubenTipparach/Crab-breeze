//! Shots in flight (openspec change `weapons`): every fixed step each projectile flies through
//! `cb_core::weapons::fly` against the mechs' part capsules, standing building cells, trees, water
//! and ground; what it hits takes the hit (a part through `mech-damage`, a cell through
//! `building-destruction`, a tree snaps) and heavy shells explode. Close combat's strikes
//! (`close-combat`) are blows queued by the mechs' step and land here through the same sweep and the
//! same hit rules. Tracers, rocket motors and their smoke trails are effects (`fx`).

use crate::blast::Explosion;
use crate::fx::Fx;
use crate::mech::{MechId, MechRig, MechSim, MechSpec, apply_hit};
use crate::sim::SimSet;
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::mech::rig::BONES;
use cb_core::mech::{Hit, MechConfig, PartKind};
use cb_core::weapons::{PartCapsule, Projectile, Struck, fly, home, spent, sweep};
use cb_core::world::Scene;
use std::collections::HashMap;
use std::sync::Arc;

/// Every shot in flight, and the blows struck this step.
#[derive(Resource, Default)]
pub struct Shots(pub Vec<Projectile>, pub Vec<Blow>);

/// A close-combat strike landing this step: the arm's sweep from the shoulder to its reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    /// The mech striking (its own parts are not hit).
    pub owner: u32,
    /// The shoulder.
    pub from: Vec3,
    /// The end of the reach.
    pub to: Vec3,
    /// Armour it beats, millimetres of rolled steel equivalent.
    pub penetration_mm: f32,
    /// Structure points it takes from a part it beats.
    pub damage: f32,
}

/// What a hit carries, from a gun or a strike.
#[derive(Clone, Copy)]
struct Power {
    penetration_mm: f32,
    damage: f32,
    blast_mj: f32,
}

type Mechs<'w, 's> = Query<'w, 's, (&'static mut MechSim, &'static MechSpec, &'static MechId, Option<&'static MechRig>)>;

/// The one place a hit takes effect, whatever struck: a vehicle or a mech part takes it through the
/// damage rules, a building loses the cells round it, a tree snaps; heavy shells explode.
#[allow(clippy::too_many_arguments)]
fn land(
    struck: Struck,
    at: Vec3,
    dir: Vec3,
    power: Power,
    units: &mut crate::units::Units,
    data: &crate::sim::Data,
    live: &mut Live,
    mechs: &mut Mechs,
    fx: &mut MessageWriter<Fx>,
    explosions: &mut MessageWriter<Explosion>,
    hits: &mut MessageWriter<crate::battle_fx::MechHit>,
    fortress: &mut MessageWriter<crate::fortress::FortressHit>,
) {
    match struck {
        // The fortress's parts and hull take it by their own armour rule (crab-airship).
        Struck::Mech { mech, part } if mech == crate::fortress::FORTRESS_ID => {
            fortress.write(crate::fortress::FortressHit {
                part,
                at,
                dir,
                penetration_mm: power.penetration_mm,
                damage: power.damage,
            });
        }
        Struck::Mech { mech, .. } if mech >= crate::units::UNIT_ID_BASE => {
            if let Some(p) = units.get_mut(mech) {
                let armour = data.0.units.units.get(&p.unit.kind).map_or(0.0, |s| s.armour_mm);
                if p.unit.hit(power.penetration_mm, power.damage, armour) {
                    crate::units::unit_destroyed(p, data, fx, explosions);
                }
            }
            fx.write(Fx::new("impact_mech", at).toward(-dir));
        }
        Struck::Mech { mech, part } => {
            // The hit's mark on the part (openspec change `mech-damage-fx`).
            hits.write(crate::battle_fx::MechHit {
                mech,
                part,
                at,
                dir,
                penetration_mm: power.penetration_mm,
                blast_mj: power.blast_mj,
            });
            if let Some((mut sim, spec, _, _)) = mechs.iter_mut().find(|m| m.2.0 == mech) {
                apply_hit(
                    &mut sim,
                    &spec.0.clone(),
                    Hit {
                        part,
                        penetration_mm: power.penetration_mm,
                        damage: power.damage,
                    },
                );
            }
            fx.write(Fx::new("impact_mech", at).toward(-dir));
        }
        Struck::Building(b) => {
            let r = 3.4f32.max(1.2 * 2.2 * power.blast_mj.max(0.01).cbrt());
            let (broke, chunks) = live.structures.break_sphere(b, at, r);
            if broke > 0 {
                live.dirty_buildings.push(b);
                live.new_chunks.extend(chunks);
            }
            fx.write(Fx::new("impact_building", at).toward(-dir));
        }
        Struck::Tree(id) => {
            if live.forest.hit(id, Vec2::new(dir.x, dir.z), f32::MAX) {
                live.moved_trees.push(id);
            }
        }
        Struck::Water => {
            fx.write(Fx::new("impact_water", at));
        }
        Struck::Ground => {
            fx.write(Fx::new("impact_ground", at).toward(-dir));
        }
    }
    if power.blast_mj >= 1.0 {
        explosions.write(Explosion {
            at,
            energy_mj: power.blast_mj,
        });
    }
}

/// Flies shots and resolves their hits.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shots>()
            .add_game_systems(FixedUpdate, fly_shots.in_set(SimSet::Mech).after(crate::mech::step_mech));
    }
}

/// A mech's hittable parts this step, from its bones' world positions.
fn capsules(id: u32, sim: &MechSim, cfg: &MechConfig, rig: &MechRig, globals: &Query<&GlobalTransform>) -> Vec<PartCapsule> {
    let at = |name: &str| {
        BONES
            .iter()
            .position(|b| b.name == name)
            .and_then(|i| globals.get(rig.bone_entity(i)).ok())
            .map(|g| g.translation())
    };
    let b = &cfg.body;
    let mut out = Vec::new();
    for (pi, p) in cfg.parts.iter().enumerate() {
        if sim.damage.lost[pi] {
            continue;
        }
        let ends = match p.kind {
            PartKind::ArmL => at("shoulder_L").zip(at("hand_L")).map(|(a, c)| (a, c, b.arm_hit_radius_m)),
            PartKind::ArmR => at("shoulder_R").zip(at("hand_R")).map(|(a, c)| (a, c, b.arm_hit_radius_m)),
            PartKind::LegL => at("thigh_L").zip(at("foot_L")).map(|(a, c)| (a, c, b.leg_hit_radius_m)),
            PartKind::LegR => at("thigh_R").zip(at("foot_R")).map(|(a, c)| (a, c, b.leg_hit_radius_m)),
            PartKind::Head => rig.head().and_then(|e| globals.get(e).ok()).map(|g| {
                (
                    g.translation() + Vec3::Y * 0.5,
                    g.translation() + Vec3::Y * 0.6,
                    b.head_hit_radius_m,
                )
            }),
            PartKind::Torso => at("torso").map(|t| (t + Vec3::Y * b.torso_hit_m[0], t + Vec3::Y * b.torso_hit_m[1], b.torso_hit_radius_m)),
        };
        if let Some((a, b, radius_m)) = ends {
            out.push(PartCapsule {
                mech: id,
                part: pi,
                a,
                b,
                radius_m,
            });
        }
        // The hips, across from thigh to thigh: a round there hurts the torso; without it, rounds at
        // the middle passed between the thighs (owner, 2026-10-25: "enemy bullets fly through me").
        if p.kind == PartKind::Torso
            && let Some((l, r)) = at("thigh_L").zip(at("thigh_R"))
        {
            out.push(PartCapsule {
                mech: id,
                part: pi,
                a: l,
                b: r,
                radius_m: b.leg_hit_radius_m,
            });
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub fn fly_shots(
    mut shots: ResMut<Shots>,
    mut units: ResMut<crate::units::Units>,
    data: Res<crate::sim::Data>,
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    mut mechs: Mechs,
    globals: Query<&GlobalTransform>,
    time: Res<Time<Fixed>>,
    mut fx: MessageWriter<Fx>,
    mut explosions: MessageWriter<Explosion>,
    mut hits: MessageWriter<crate::battle_fx::MechHit>,
    fortress: Option<Res<crate::fortress::FortressRun>>,
    mut fortress_hits: MessageWriter<crate::fortress::FortressHit>,
) {
    let _t = crate::perf_hud::slow("fly_shots");
    if shots.0.is_empty() && shots.1.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let mut parts = Vec::new();
    let mut specs: HashMap<u32, Arc<MechConfig>> = HashMap::new();
    for (sim, spec, id, rig) in &mechs {
        specs.insert(id.0, spec.0.clone());
        if let Some(rig) = rig
            && !sim.dead
        {
            parts.extend(capsules(id.0, sim, spec, rig, &globals));
        }
    }
    // Vehicles are hit like mechs, one capsule each (invasion-forces).
    parts.extend(crate::units::unit_capsules(&units, &data));
    // The fortress's parts, then its hull (crab-airship).
    if let Some(f) = &fortress {
        parts.extend(f.capsules.iter().copied());
    }
    // Live mechs' torsos and vehicles' centres, for missiles to home on.
    let mut torsos: HashMap<u32, Vec3> = mechs
        .iter()
        .filter(|m| !m.0.dead)
        .map(|m| (m.2.0, m.0.curr.pos + Vec3::Y * m.1.aim_height_m()))
        .collect();
    torsos.extend(
        units
            .list
            .iter()
            .filter(|p| p.unit.alive() && !p.gone)
            .map(|p| (p.unit.id, p.unit.pos)),
    );
    if let Some(f) = &fortress {
        torsos.extend(f.targets.iter().copied());
    }
    // A shot's gun: its mech's, or its vehicle's one weapon.
    let unit_guns: HashMap<u32, cb_core::weapons::WeaponSpec> = units
        .list
        .iter()
        .filter_map(|p| Some((p.unit.id, data.0.units.units.get(&p.unit.kind)?.weapon.clone()?)))
        .collect();
    let weapon_of = |owner: u32, weapon: usize| -> Option<cb_core::weapons::WeaponSpec> {
        specs
            .get(&owner)
            .and_then(|c| c.weapons.get(weapon))
            .or_else(|| unit_guns.get(&owner))
            .cloned()
            .or_else(|| {
                // The fortress's rounds name the part that fired them.
                (owner == crate::fortress::FORTRESS_ID)
                    .then(|| {
                        data.0
                            .fortress
                            .shape
                            .parts
                            .get(weapon)
                            .and_then(|_| data.0.fortress.kind(weapon).gun.as_ref())
                    })
                    .flatten()
                    .map(|g| g.weapon.clone())
            })
    };
    let live = &mut *live;
    // Strikes land first: the first thing in each arm's sweep takes the blow (close-combat).
    for blow in std::mem::take(&mut shots.1) {
        let hit = {
            let scene = Scene {
                plan: &plan,
                dug: &live.dug,
                trees: None,
                structures: Some(&live.structures),
                platforms: &[],
                interiors: None,
            };
            sweep(
                blow.from,
                blow.to,
                0.5,
                Some(blow.owner),
                &scene,
                &live.structures,
                &live.forest,
                &parts,
            )
        };
        if let Some((struck, at)) = hit {
            let power = Power {
                penetration_mm: blow.penetration_mm,
                damage: blow.damage,
                blast_mj: 0.0,
            };
            let dir = (blow.to - blow.from).normalize_or(Vec3::NEG_Z);
            land(
                struck,
                at,
                dir,
                power,
                &mut units,
                &data,
                live,
                &mut mechs,
                &mut fx,
                &mut explosions,
                &mut hits,
                &mut fortress_hits,
            );
        }
    }
    let mut flying = Vec::with_capacity(shots.0.len());
    for mut shot in std::mem::take(&mut shots.0) {
        // A missile turns toward its locked target's torso first (targeting).
        if let Some(t) = shot.target {
            let seeker = weapon_of(shot.owner, shot.weapon).map_or(0.0, |w| w.seeker_turn_deg_s);
            if let Some(torso) = torsos.get(&t) {
                home(&mut shot, *torso, seeker, dt);
            }
        }
        let impact = {
            let scene = Scene {
                plan: &plan,
                dug: &live.dug,
                trees: None,
                structures: Some(&live.structures),
                platforms: &[],
                interiors: None,
            };
            let _s = crate::perf_hud::section("fly_one_shot");
            fly(&mut shot, &scene, &live.structures, &live.forest, &parts, dt)
        };
        let Some(w) = weapon_of(shot.owner, shot.weapon) else {
            continue;
        };
        let Some(hit) = impact else {
            if !spent(&shot) {
                flying.push(shot);
            }
            continue;
        };
        let dir = shot.vel.normalize_or(Vec3::Y);
        debug!(
            "impact: shot from mech {} ({}) hit {:?} at {:?}",
            shot.owner, w.name, hit.struck, hit.at
        );
        let power = Power {
            penetration_mm: w.penetration_mm,
            damage: w.damage,
            blast_mj: w.blast_mj,
        };
        land(
            hit.struck,
            hit.at,
            dir,
            power,
            &mut units,
            &data,
            live,
            &mut mechs,
            &mut fx,
            &mut explosions,
            &mut hits,
            &mut fortress_hits,
        );
    }
    shots.0 = flying;
}
