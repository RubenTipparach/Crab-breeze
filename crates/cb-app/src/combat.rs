//! Shots in flight (openspec change `weapons`): every fixed step each projectile flies through
//! `cb_core::weapons::fly` against the mechs' part capsules, standing building cells, trees, water
//! and ground; what it hits takes the hit (a part through `mech-damage`, a cell through
//! `building-destruction`, a tree snaps) and heavy shells explode. Tracers are effects (`fx`);
//! a rocket lays its smoke trail here, every `trail_spacing_m` of the path it flew this step.

use crate::blast::Explosion;
use crate::fx::Fx;
use crate::mech::{MechId, MechRig, MechSim, MechSpec, apply_hit};
use crate::sim::{Data, SimSet};
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
    data: Res<Data>,
    mut fx: MessageWriter<Fx>,
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
    let spacing = data.0.fx.trail_spacing_m;
    for mut shot in std::mem::take(&mut shots.0) {
        let (from, flown_before) = (shot.pos, shot.age_s * shot.vel.length());
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
        if shot.rocket {
            // The trail: a puff at every multiple of the spacing passed this step.
            let flown = shot.age_s * shot.vel.length();
            let (first, last) = ((flown_before / spacing).floor() as i64 + 1, (flown / spacing).floor() as i64);
            for k in first..=last {
                let t = ((k as f32 * spacing - flown_before) / (flown - flown_before).max(1e-3)).clamp(0.0, 1.0);
                fx.write(Fx::new("rocket_trail", from.lerp(shot.pos, t)).toward(shot.vel));
            }
        }
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
