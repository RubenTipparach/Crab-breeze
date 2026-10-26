//! Explosions (openspec change `terrain-deformation`): an `Explosion` message digs a crater, snaps
//! trees, breaks building cells and hurts mechs nearby, then plays the ground, water or air blast
//! recipe (`combat-fx`) at the cube root of its energy. The crater and the trees are simulation
//! (`cb-core`); the rest is effects.

use crate::fx::Fx;
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
    /// The mech, vehicle or fortress whose shot it was, when it was one (civilians: the pilot's own
    /// blasts cost standing).
    pub by: Option<u32>,
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
    mut fx: MessageWriter<Fx>,
    mut mechs: Query<(&mut MechSim, &MechSpec)>,
) {
    let _t = crate::perf_hud::slow("explode");
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
                platforms: &[],
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
        let recipe = match (wet.is_some(), on_ground) {
            (true, true) => "explosion_water",
            (false, true) => "explosion_ground",
            (_, false) => "explosion_air",
        };
        fx.write(Fx::new(recipe, at).scaled(e.energy_mj.max(0.01).cbrt()));
        info!(
            "explosion: {:.1} MJ at ({:.0}, {:.0}), crater radius {r:.1} m",
            e.energy_mj, e.at.x, e.at.y
        );
    }
}
