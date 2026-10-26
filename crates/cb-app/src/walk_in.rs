//! The detailed building colliders a pilot on foot walks into (openspec change `building-interiors`,
//! design section 4). Owner, 2026-10-26: "when on foot I should be able to wlak into the buildings, I
//! think you would generate detailed colliders for buildings only when the player is on foot".
//!
//! Each fixed step, before the pilot moves: while the pilot is on foot, every standing building within
//! `collision_radius_m` gets its `Walkable` (the facade with its street door, floors with their
//! stairwells, partitions, stair and furniture: `cb_core::world::interior`), those within
//! `collision_sync_m` at once and the rest on the async pool; past the radius plus the margin they are
//! freed. The moment the pilot is not on foot (boarding, seated, climbing out) all of them are freed and
//! jobs dropped: in the mech, buildings are their coarse cells. Climbing out ends on foot in one step,
//! and this runs before the next, so the first step on foot already has the buildings beside it.

use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::world::{Live, Plan, Seed};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use cb_core::pilot::PilotMode;
use cb_core::world::interior::{Interiors, Site, Walkable};
use std::collections::BTreeMap;

/// The colliders built now, read by the pilot's step (`Scene::interiors`).
#[derive(Resource, Default)]
pub struct InteriorColliders(pub Interiors);

/// Colliders being built on the async pool, by building id.
#[derive(Resource, Default)]
struct ColliderJobs(BTreeMap<u32, Task<Walkable>>);

/// Most colliders built on the pool at once.
const COLLIDER_JOBS: usize = 4;

/// Builds and frees the colliders round a pilot on foot.
pub struct WalkInPlugin;

impl Plugin for WalkInPlugin {
    fn build(&self, app: &mut App) {
        let cfg = app.world().resource::<Data>().0.world.interiors.clone();
        app.insert_resource(InteriorColliders(Interiors { cfg, ..Default::default() }))
            .init_resource::<ColliderJobs>()
            .add_systems(FixedUpdate, stream_colliders.in_set(SimSet::Pilot).before(crate::pilot::step_pilot));
    }
}

/// Keeps the colliders to the buildings round a pilot on foot (module docs).
fn stream_colliders(
    pilot: Res<PilotSim>,
    plan: Res<Plan>,
    live: Res<Live>,
    seed: Res<Seed>,
    mut colliders: ResMut<InteriorColliders>,
    mut jobs: ResMut<ColliderJobs>,
) {
    let _t = crate::perf_hud::slow("stream_colliders");
    let p = &pilot.curr;
    let on_foot = p.mode == PilotMode::OnFoot;
    let here = Vec2::new(p.feet.x, p.feet.z);
    let inside = &mut colliders.0;
    inside.prune(on_foot, here, &plan.0);
    if !on_foot {
        jobs.0.clear();
        return;
    }
    let built: Vec<(u32, Walkable)> = jobs.0.iter_mut().filter_map(|(b, t)| check_ready(t).map(|w| (*b, w))).collect();
    for (b, w) in built {
        jobs.0.remove(&b);
        inside.buildings.insert(b, w);
    }
    let rng = crate::buildings::building_rng(seed.0);
    let column_m = live.structures.cfg.column_m;
    let pool = AsyncComputeTaskPool::get();
    for (b, d) in inside.wanted(true, here, &plan.0) {
        let c = &live.structures.buildings[b as usize];
        if inside.buildings.contains_key(&b) || c.standing == 0 {
            continue;
        }
        let site = Site::of(&plan.0, b, c.n, c.origin, c.size);
        if d <= inside.cfg.collision_sync_m {
            jobs.0.remove(&b);
            let w = Walkable::build(&site, &inside.cfg, column_m, &rng);
            inside.buildings.insert(b, w);
        } else if !jobs.0.contains_key(&b) && jobs.0.len() < COLLIDER_JOBS {
            let (cfg, rng) = (inside.cfg.clone(), rng.clone());
            jobs.0
                .insert(b, pool.spawn(async move { Walkable::build(&site, &cfg, column_m, &rng) }));
        }
    }
    let keep = inside.cfg.collision_radius_m + inside.cfg.collision_margin_m;
    jobs.0.retain(|&b, _| plan.0.buildings[b as usize].footprint.distance(here) <= keep);
}
