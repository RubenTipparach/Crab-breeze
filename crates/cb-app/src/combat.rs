//! Shots in flight (openspec change `weapons`): every fixed step each projectile flies through
//! `cb_core::weapons::fly` against the mechs' part capsules, standing building cells, trees, water
//! and ground; what it hits takes the hit (a part through `mech-damage`, a cell through
//! `building-destruction`, a tree snaps) and heavy shells explode. Close combat's strikes
//! (`close-combat`) are blows queued by the mechs' step and land here through the same sweep and the
//! same hit rules. A laser's beam (`weapon-kinds`) is traced through `cb_core::weapons::trace` each step it
//! is held and lands here by the same rules; a plasma ball that has flown its distance bursts in the air.
//! Tracers, rocket motors and their smoke trails are effects (`fx`).

use crate::blast::Explosion;
use crate::fx::Fx;
use crate::mech::{MechId, MechRig, MechSim, MechSpec, apply_hit};
use crate::sim::SimSet;
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::mech::rig::BONES;
use cb_core::mech::{Hit, MechConfig, PartKind};
use cb_core::weapons::{PartCapsule, Projectile, Struck, burst_due, fly, home, spent, sweep, trace};
use cb_core::world::Scene;
use std::collections::HashMap;
use std::sync::Arc;

/// Every shot in flight, the blows struck this step and the beams burning this step.
#[derive(Resource, Default)]
pub struct Shots(pub Vec<Projectile>, pub Vec<Blow>, pub Vec<Beam>);

/// A laser held this step (`weapon-kinds`): it burns this step's share of its damage into the first
/// thing on its line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    /// The mech firing (its own parts are not hit).
    pub owner: u32,
    /// The muzzle.
    pub from: Vec3,
    /// The aim, a unit vector.
    pub dir: Vec3,
    /// How far it reaches, metres.
    pub range_m: f32,
    /// Armour it beats, millimetres of rolled steel equivalent.
    pub penetration_mm: f32,
    /// Structure points it takes this step (its damage a second times the step).
    pub damage: f32,
}

/// Where each beam burned last step, for the effects (`fx`): from the muzzle to the hit or its reach.
#[derive(Resource, Default)]
pub struct BeamsDrawn(pub Vec<(Vec3, Vec3)>);

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

/// What a hit carries, from a gun or a strike, and who fired it.
#[derive(Clone, Copy)]
struct Power {
    penetration_mm: f32,
    damage: f32,
    blast_mj: f32,
    /// The player's own mech fired or struck it (a civilian harmed by it costs standing: survey W34).
    by_player: bool,
    /// Whose shot or blow it was.
    owner: u32,
}

type Mechs<'w, 's> = Query<'w, 's, (&'static mut MechSim, &'static MechSpec, &'static MechId, Option<&'static MechRig>)>;

/// The one place a hit takes effect, whatever struck: a vehicle, a civil aircraft or a mech part takes
/// it through the damage rules, a building loses the cells round it, a tree snaps; heavy shells explode.
/// `power.owner` fired it (a civil aircraft hit by the pilot counts against their standing).
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
    cars: &mut crate::traffic::TrafficSim,
    civil: &mut crate::air_traffic::CivilHits,
    boats: &mut MessageWriter<crate::boats::BoatHit>,
) {
    match struck {
        // A car: a round that beats its thin skin sets it alight (car-traffic).
        Struck::Mech { mech, .. } if mech >= crate::traffic::CAR_ID_BASE => {
            cars.traffic.hit(
                &data.0.traffic,
                mech - crate::traffic::CAR_ID_BASE,
                power.penetration_mm,
                power.by_player,
            );
            fx.write(Fx::new("impact_mech", at).toward(-dir));
        }
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
        // A civil aircraft (civil-air-traffic), by the same armour rule as a vehicle.
        Struck::Mech { mech, .. } if crate::air_traffic::is_civil(mech) => {
            civil.hit(mech, power.owner, at, power.penetration_mm, power.damage, data, fx);
            fx.write(Fx::new("impact_mech", at).toward(-dir));
        }
        // A boat takes it by the vehicles' rule, in `boats` (harbours); its ids sit below the civil aircraft's.
        Struck::Mech { mech, .. } if (crate::boats::BOAT_ID_BASE..crate::air_traffic::CIVIL_ID_BASE).contains(&mech) => {
            boats.write(crate::boats::BoatHit {
                boat: Some(mech - crate::boats::BOAT_ID_BASE),
                at,
                penetration_mm: power.penetration_mm,
                damage: power.damage,
                blast_mj: 0.0,
                owner: power.owner,
            });
            fx.write(Fx::new("impact_mech", at).toward(-dir));
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
            by: Some(power.owner),
        });
    }
    // A round's blast may splash a boat, and the boats need to know whose it was (harbours).
    if power.blast_mj > 0.0 {
        boats.write(crate::boats::BoatHit {
            boat: None,
            at,
            penetration_mm: 0.0,
            damage: 0.0,
            blast_mj: power.blast_mj,
            owner: power.owner,
        });
    }
}

/// Flies shots and resolves their hits.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shots>()
            .init_resource::<BeamsDrawn>()
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
    (mut cars, piloted): (ResMut<crate::traffic::TrafficSim>, Query<&MechId, With<crate::mech::Piloted>>),
    mut civil: crate::air_traffic::CivilHits,
    (boats, mut boat_hits, mut drawn): (
        Option<Res<crate::boats::Boats>>,
        MessageWriter<crate::boats::BoatHit>,
        ResMut<BeamsDrawn>,
    ),
) {
    let _t = crate::perf_hud::slow("fly_shots");
    drawn.0.clear();
    if shots.0.is_empty() && shots.1.is_empty() && shots.2.is_empty() {
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
    // Vehicles are hit like mechs, one capsule each (invasion-forces); civil aircraft by their
    // fuselage and wing (civil-air-traffic).
    parts.extend(crate::units::unit_capsules(&units, &data));
    parts.extend(civil.capsules(&data));
    // Boats afloat, one capsule each (harbours).
    if let Some(b) = &boats {
        parts.extend(crate::boats::boat_capsules(b, &data));
    }
    // The fortress's parts, then its hull (crab-airship).
    if let Some(f) = &fortress {
        parts.extend(f.capsules.iter().copied());
    }
    // Cars near a shot or a strike this step (car-traffic).
    let near: Vec<Vec3> = shots
        .0
        .iter()
        .map(|s| s.pos)
        .chain(shots.1.iter().map(|b| b.from))
        .chain(shots.2.iter().map(|b| b.from + b.dir * b.range_m * 0.5))
        .collect();
    let reach = shots
        .0
        .iter()
        .map(|s| s.vel.length() * dt)
        .chain(shots.2.iter().map(|b| b.range_m * 0.5))
        .fold(30.0f32, f32::max)
        + 10.0;
    parts.extend(crate::traffic::car_capsules(&cars, &data, &near, reach));
    let me = piloted.iter().next().map(|m| m.0);
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
    // A civil aircraft only ever has a missile on it when the pilot selected it on purpose.
    torsos.extend(civil.centres());
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
                by_player: Some(blow.owner) == me,
                owner: blow.owner,
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
                &mut cars,
                &mut civil,
                &mut boat_hits,
            );
        }
    }
    // Beams burn the first thing on their line (weapon-kinds): the same hit rules as a round.
    for beam in std::mem::take(&mut shots.2) {
        let hit = {
            let scene = Scene {
                plan: &plan,
                dug: &live.dug,
                trees: None,
                structures: Some(&live.structures),
                platforms: &[],
                interiors: None,
            };
            trace(
                beam.from,
                beam.dir,
                beam.range_m,
                Some(beam.owner),
                &scene,
                &live.structures,
                &live.forest,
                &parts,
            )
        };
        drawn.0.push((beam.from, hit.map_or(beam.from + beam.dir * beam.range_m, |h| h.1)));
        if let Some((struck, at)) = hit {
            let power = Power {
                penetration_mm: beam.penetration_mm,
                damage: beam.damage,
                blast_mj: 0.0,
                by_player: Some(beam.owner) == me,
                owner: beam.owner,
            };
            land(
                struck,
                at,
                beam.dir,
                power,
                &mut units,
                &data,
                live,
                &mut mechs,
                &mut fx,
                &mut explosions,
                &mut hits,
                &mut fortress_hits,
                &mut cars,
                &mut civil,
                &mut boat_hits,
            );
        }
    }
    let mut flying = Vec::with_capacity(shots.0.len());
    for mut shot in std::mem::take(&mut shots.0) {
        // A missile turns toward its locked target's torso first (targeting).
        if let Some(t) = shot.target {
            let seeker = weapon_of(shot.owner, shot.weapon).map_or(0.0, |w| w.turn_deg_s());
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
            fly(&mut shot, &scene, &live.structures, &live.forest, &parts, dt)
        };
        let Some(w) = weapon_of(shot.owner, shot.weapon) else {
            continue;
        };
        let Some(hit) = impact else {
            // A plasma ball that met nothing bursts in the air at its distance (weapon-kinds).
            if burst_due(w.shot(), &shot) {
                explosions.write(Explosion {
                    at: shot.pos,
                    energy_mj: w.blast_mj,
                    by: Some(shot.owner),
                });
                boat_hits.write(crate::boats::BoatHit {
                    boat: None,
                    at: shot.pos,
                    penetration_mm: 0.0,
                    damage: 0.0,
                    blast_mj: w.blast_mj,
                    owner: shot.owner,
                });
            } else if !spent(&shot) {
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
            by_player: Some(shot.owner) == me,
            owner: shot.owner,
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
            &mut cars,
            &mut civil,
            &mut boat_hits,
        );
    }
    shots.0 = flying;
}
