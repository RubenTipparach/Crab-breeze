//! Explosions (openspec change `terrain-deformation`): an `Explosion` message digs a crater, snaps
//! trees, throws dirt, smoke and sparks (or spray on water) and flashes. The crater and the trees
//! are simulation (`cb-core`); the rest is effects.

use crate::fx::{Burst, Flash, FxKind, Ripple};
use crate::mech::{MechSim, MechSpec, apply_hit};
use crate::sim::{Data, SimSet};
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::mech::{Hit, PartKind};
use cb_core::world::deform::Blast;
use cb_core::world::{Ground, Scene};

/// Something blew up.
#[derive(Message, Clone, Copy, Debug)]
pub struct Explosion {
    /// Where; a blast within its radius of the ground digs a crater.
    pub at: Vec3,
    /// Energy, megajoules (a 120 mm shell is about 4).
    pub energy_mj: f32,
}

/// Resolves explosions.
pub struct BlastPlugin;

impl Plugin for BlastPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Explosion>()
            .add_systems(FixedUpdate, explode.in_set(SimSet::Mech).after(crate::mech::step_mech));
    }
}

fn explode(
    mut explosions: MessageReader<Explosion>,
    mut live: ResMut<Live>,
    plan: Res<Plan>,
    data: Res<Data>,
    mut bursts: MessageWriter<Burst>,
    mut flashes: MessageWriter<Flash>,
    mut ripples: MessageWriter<Ripple>,
    mut mechs: Query<(&mut MechSim, &MechSpec)>,
) {
    let live = &mut *live;
    for e in explosions.read() {
        let cfg = &data.0.world.craters;
        let r = cfg.radius_m_per_cbrt_mj * e.energy_mj.cbrt();
        let xz = Vec2::new(e.at.x, e.at.z);
        let (wet, ground_y) = {
            let s = Scene {
                plan: &plan,
                dug: &live.dug,
                trees: None,
                structures: None,
            };
            (s.water_at(xz.x, xz.y), s.height_at(xz.x, xz.y))
        };
        let on_ground = e.at.y - ground_y < r;
        let at = if on_ground {
            Vec3::new(xz.x, wet.map_or(ground_y, |w| w.surface_m), xz.y)
        } else {
            e.at
        };
        if wet.is_none() && on_ground {
            let touched = live.dug.dig(
                &plan,
                Blast {
                    centre: xz,
                    energy_mj: e.energy_mj,
                },
                cfg,
            );
            live.dirty_tiles.extend(touched);
        }
        if on_ground {
            // A blast far from every body grows the trees it reaches first (island-world).
            live.forest.grow_around(&plan, xz, r * 4.0);
            let broke = live.forest.blast(xz, r * 4.0, 400_000.0 * e.energy_mj.sqrt());
            live.moved_trees.extend(broke);
        }
        // Buildings within reach lose the cells the blast reaches (building-destruction).
        for b in plan.collision.candidates(xz, r * 1.5) {
            let (broke, chunks) = live.structures.break_sphere(b, e.at, r * 1.3);
            if broke > 0 {
                live.dirty_buildings.push(b);
                live.new_chunks.extend(chunks);
            }
        }
        // Mechs nearby take splash on the part nearest the blast; armour stops most of it.
        for (mut sim, spec) in &mut mechs {
            let d = sim.curr.pos.distance(e.at - Vec3::Y * 4.0);
            if d < r * 3.0 && !sim.dead {
                let kind = if e.at.y - sim.curr.pos.y < 6.0 {
                    PartKind::LegL
                } else {
                    PartKind::Torso
                };
                if let Some(part) = spec.parts.iter().position(|p| p.kind == kind) {
                    let k = 1.0 - d / (r * 3.0);
                    let cfg = spec.0.clone();
                    apply_hit(
                        &mut sim,
                        &cfg,
                        Hit {
                            part,
                            penetration_mm: 70.0 + 60.0 * k,
                            damage: 60.0 * k * e.energy_mj.cbrt(),
                        },
                    );
                }
            }
        }
        flashes.write(Flash {
            at,
            lumens: 4.0e8 * e.energy_mj,
        });
        let burst = |kind, count, speed_m_s, spread, size_m, life_s| Burst {
            kind,
            at,
            dir: Vec3::Y,
            count,
            speed_m_s,
            spread,
            size_m,
            life_s,
        };
        if wet.is_some() {
            bursts.write(burst(FxKind::Spray, 220, 22.0, 0.35, 1.6, 2.5));
            bursts.write(burst(FxKind::Steam, 40, 4.0, 0.8, 4.0, 4.0));
            ripples.write(Ripple { at, radius_m: r * 6.0 });
        } else {
            bursts.write(burst(FxKind::Dirt, 160, 20.0, 0.6, 0.5, 3.0));
            bursts.write(burst(FxKind::Dust, 60, 6.0, 0.9, 4.0, 5.0));
            bursts.write(burst(FxKind::Smoke, 40, 4.0, 0.5, 5.0, 8.0));
        }
        bursts.write(burst(FxKind::Fire, 50, 9.0, 0.8, 2.2, 0.7));
        bursts.write(burst(FxKind::Spark, 80, 26.0, 0.9, 0.25, 1.2));
        info!(
            "explosion: {:.1} MJ at ({:.0}, {:.0}), crater radius {r:.1} m",
            e.energy_mj, e.at.x, e.at.y
        );
    }
}
