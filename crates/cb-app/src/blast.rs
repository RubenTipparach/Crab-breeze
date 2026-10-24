//! Explosions (openspec change `terrain-deformation`): an `Explosion` message digs a crater, snaps
//! trees, throws dirt, smoke and sparks (or spray on water) and flashes. The crater and the trees
//! are simulation (`cb-core`); the rest is effects.

use crate::fx::{Burst, Flash, FxKind, Ripple};
use crate::sim::{Data, SimSet};
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::world::deform::Blast;
use cb_core::world::{Ground, Scene};

/// Something blew up.
#[derive(Message, Clone, Copy, Debug)]
pub struct Explosion {
    /// Where (x, z); it goes off on the ground or water there.
    pub at: Vec2,
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
) {
    let live = &mut *live;
    for e in explosions.read() {
        let cfg = &data.0.world.craters;
        let r = cfg.radius_m_per_cbrt_mj * e.energy_mj.cbrt();
        let wet = Scene {
            plan: &plan,
            dug: &live.dug,
            trees: None,
        }
        .water_at(e.at.x, e.at.y);
        let ground_y = Scene {
            plan: &plan,
            dug: &live.dug,
            trees: None,
        }
        .height_at(e.at.x, e.at.y);
        let at = Vec3::new(e.at.x, wet.map_or(ground_y, |w| w.surface_m), e.at.y);
        if wet.is_none() {
            let touched = live.dug.dig(
                &plan,
                Blast {
                    centre: e.at,
                    energy_mj: e.energy_mj,
                },
                cfg,
            );
            live.dirty_tiles.extend(touched);
        }
        let broke = live.forest.blast(e.at, r * 4.0, 400_000.0 * e.energy_mj.sqrt());
        live.moved_trees.extend(broke);
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
