//! Shots in flight (openspec change `weapons`): every fixed step each projectile flies through
//! `cb_core::weapons::fly` against the mechs' part capsules, standing building cells, trees, water
//! and ground; what it hits takes the hit (a part through `mech-damage`, a cell through
//! `building-destruction`, a tree snaps) and heavy shells explode. Tracers are effects.

use crate::blast::Explosion;
use crate::fx::{Burst, FxKind};
use crate::mech::{MechId, MechRig, MechSim, MechSpec, apply_hit};
use crate::sim::SimSet;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::mech::rig::BONES;
use cb_core::mech::{Hit, MechConfig, PartKind};
use cb_core::weapons::{PartCapsule, Projectile, Struck, fly, spent};
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

pub fn fly_shots(
    mut shots: ResMut<Shots>,
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    mut mechs: Query<(&mut MechSim, &MechSpec, &MechId, Option<&MechRig>)>,
    globals: Query<&GlobalTransform>,
    time: Res<Time<Fixed>>,
    mut bursts: MessageWriter<Burst>,
    mut explosions: MessageWriter<Explosion>,
) {
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
    let live = &mut *live;
    let mut flying = Vec::with_capacity(shots.0.len());
    for mut shot in std::mem::take(&mut shots.0) {
        let impact = {
            let scene = Scene {
                plan: &plan,
                dug: &live.dug,
                trees: None,
                structures: Some(&live.structures),
            };
            fly(&mut shot, &scene, &live.structures, &live.forest, &parts, dt)
        };
        let Some(w) = specs.get(&shot.owner).and_then(|c| c.weapons.get(shot.weapon)).cloned() else {
            continue;
        };
        let Some(hit) = impact else {
            // Tracer: a bright streak every step along the path.
            bursts.write(Burst {
                kind: FxKind::Tracer,
                at: shot.pos,
                dir: shot.vel.normalize_or(Vec3::Y),
                count: 1,
                speed_m_s: 0.0,
                spread: 0.0,
                size_m: if w.blast_mj >= 1.0 { 0.9 } else { 0.5 },
                life_s: 0.06,
            });
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
                bursts.write(Burst {
                    kind: FxKind::Spark,
                    at: hit.at,
                    dir: -dir,
                    count: 25,
                    speed_m_s: 14.0,
                    spread: 0.7,
                    size_m: 0.3,
                    life_s: 0.6,
                });
            }
            Struck::Building(b) => {
                let r = 3.4f32.max(1.2 * 2.2 * w.blast_mj.max(0.01).cbrt());
                let (broke, chunks) = live.structures.break_sphere(b, hit.at, r);
                if broke > 0 {
                    live.dirty_buildings.push(b);
                    live.new_chunks.extend(chunks);
                }
                bursts.write(Burst {
                    kind: FxKind::Dust,
                    at: hit.at,
                    dir: -dir,
                    count: 18,
                    speed_m_s: 5.0,
                    spread: 0.8,
                    size_m: 2.5,
                    life_s: 3.0,
                });
            }
            Struck::Tree(id) => {
                if live.forest.hit(id, Vec2::new(dir.x, dir.z), f32::MAX) {
                    live.moved_trees.push(id);
                }
            }
            Struck::Water => {
                bursts.write(Burst {
                    kind: FxKind::Spray,
                    at: hit.at,
                    dir: Vec3::Y,
                    count: 15,
                    speed_m_s: 8.0,
                    spread: 0.3,
                    size_m: 0.8,
                    life_s: 1.2,
                });
            }
            Struck::Ground => {
                bursts.write(Burst {
                    kind: FxKind::Dirt,
                    at: hit.at,
                    dir: Vec3::Y,
                    count: 12,
                    speed_m_s: 8.0,
                    spread: 0.5,
                    size_m: 0.4,
                    life_s: 1.5,
                });
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
