//! Shots in flight (openspec change `weapons`): every fixed step each projectile flies through
//! `cb_core::weapons::fly` against the mechs' part capsules, standing building cells, trees, water
//! and ground; what it hits takes the hit (a part through `mech-damage`, a cell through
//! `building-destruction`, a tree snaps) and heavy shells explode. Tracers, rocket motors and
//! their smoke trails are effects (`fx`).

use crate::blast::Explosion;
use crate::fx::Fx;
use crate::mech::{MechId, MechRig, MechSim, MechSpec, apply_hit};
use crate::sim::SimSet;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::mech::rig::BONES;
use cb_core::mech::{Hit, MechConfig, PartKind};
use cb_core::weapons::{PartCapsule, Projectile, Struck, fly, home, spent};
use cb_core::world::Scene;
use std::collections::HashMap;
use std::sync::Arc;

/// Every shot in flight.
#[derive(Resource, Default)]
pub struct Shots(pub Vec<Projectile>);

/// Flies shots and resolves their hits.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shots>()
            .add_systems(FixedUpdate, fly_shots.in_set(SimSet::Mech).after(crate::mech::step_mech));
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
    let mut out = Vec::new();
    for (pi, p) in cfg.parts.iter().enumerate() {
        if sim.damage.lost[pi] {
            continue;
        }
        let ends = match p.kind {
            PartKind::ArmL => at("shoulder_L").zip(at("hand_L")).map(|(a, b)| (a, b, 1.0)),
            PartKind::ArmR => at("shoulder_R").zip(at("hand_R")).map(|(a, b)| (a, b, 1.0)),
            PartKind::LegL => at("thigh_L").zip(at("foot_L")).map(|(a, b)| (a, b, 1.1)),
            PartKind::LegR => at("thigh_R").zip(at("foot_R")).map(|(a, b)| (a, b, 1.1)),
            PartKind::Head => rig
                .head()
                .and_then(|e| globals.get(e).ok())
                .map(|g| (g.translation() + Vec3::Y * 0.5, g.translation() + Vec3::Y * 0.6, 1.1)),
            PartKind::Torso => at("torso").map(|t| (t + Vec3::Y * 0.5, t + Vec3::Y * 3.9, 2.4)),
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
    mut mechs: Query<(&mut MechSim, &MechSpec, &MechId, Option<&MechRig>)>,
    globals: Query<&GlobalTransform>,
    time: Res<Time<Fixed>>,
    mut fx: MessageWriter<Fx>,
    mut explosions: MessageWriter<Explosion>,
) {
    let _t = crate::perf_hud::slow("fly_shots");
    if shots.0.is_empty() {
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
    // Live mechs' torsos and vehicles' centres, for missiles to home on.
    let mut torsos: HashMap<u32, Vec3> = mechs
        .iter()
        .filter(|m| !m.0.dead)
        .map(|m| (m.2.0, m.0.curr.pos + Vec3::Y * cb_core::ai::TORSO_AIM_M))
        .collect();
    torsos.extend(
        units
            .list
            .iter()
            .filter(|p| p.unit.alive() && !p.gone)
            .map(|p| (p.unit.id, p.unit.pos)),
    );
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
    };
    let live = &mut *live;
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
            };
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
        match hit.struck {
            Struck::Mech { mech, .. } if mech >= crate::units::UNIT_ID_BASE => {
                if let Some(p) = units.get_mut(mech) {
                    let armour = data.0.units.units.get(&p.unit.kind).map_or(0.0, |s| s.armour_mm);
                    if p.unit.hit(w.penetration_mm, w.damage, armour) {
                        crate::units::unit_destroyed(p, &data, &mut fx, &mut explosions);
                    }
                }
                fx.write(Fx::new("impact_mech", hit.at).toward(-dir));
            }
            Struck::Mech { mech, part } => {
                if let Some((mut sim, spec, _, _)) = mechs.iter_mut().find(|m| m.2.0 == mech) {
                    apply_hit(
                        &mut sim,
                        &spec.0.clone(),
                        Hit {
                            part,
                            penetration_mm: w.penetration_mm,
                            damage: w.damage,
                        },
                    );
                }
                fx.write(Fx::new("impact_mech", hit.at).toward(-dir));
            }
            Struck::Building(b) => {
                let r = 3.4f32.max(1.2 * 2.2 * w.blast_mj.max(0.01).cbrt());
                let (broke, chunks) = live.structures.break_sphere(b, hit.at, r);
                if broke > 0 {
                    live.dirty_buildings.push(b);
                    live.new_chunks.extend(chunks);
                }
                fx.write(Fx::new("impact_building", hit.at).toward(-dir));
            }
            Struck::Tree(id) => {
                if live.forest.hit(id, Vec2::new(dir.x, dir.z), f32::MAX) {
                    live.moved_trees.push(id);
                }
            }
            Struck::Water => {
                fx.write(Fx::new("impact_water", hit.at));
            }
            Struck::Ground => {
                fx.write(Fx::new("impact_ground", hit.at).toward(-dir));
            }
        }
        if w.blast_mj >= 1.0 {
            explosions.write(Explosion {
                at: hit.at,
                energy_mj: w.blast_mj,
            });
        }
    }
    shots.0 = flying;
}
