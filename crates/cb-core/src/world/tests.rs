//! Tests for the world generator, against the shipped `data/world/world.ron`.

use super::*;
use crate::data::load_ron;
use crate::world::Ground;
use glam::Vec3;
use std::path::PathBuf;
use std::sync::OnceLock;

fn data_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn shipped() -> WorldConfig {
    load_ron(&data_path("data/world/world.ron")).expect("shipped world data loads")
}

fn seed_42() -> &'static WorldPlan {
    static PLAN: OnceLock<WorldPlan> = OnceLock::new();
    PLAN.get_or_init(|| {
        let started = std::time::Instant::now();
        let plan = generate(42, &shipped());
        eprintln!(
            "seed 42 generated in {:.0} ms (test profile)",
            started.elapsed().as_secs_f64() * 1000.0
        );
        plan
    })
}

/// The seed-42 forest grown round the start, as the game grows it before the first step.
fn woods_at_start() -> &'static Forest {
    static WOODS: OnceLock<Forest> = OnceLock::new();
    WOODS.get_or_init(|| {
        let plan = seed_42();
        let mut f = plan.forest.clone();
        let started = std::time::Instant::now();
        // Every tile within the keep radius, at once, as the game does before its first step.
        let keep = shipped().trees.keep_radius_m;
        f.grow_around(plan, plan.start.mech_xz, keep);
        f.grow_around(plan, plan.start.pilot_xz, keep);
        eprintln!(
            "grew {} trees in {} tiles round the start in {:.0} ms (test profile)",
            f.grown(),
            f.grown_tiles().count(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        f
    })
}

#[test]
fn the_shipped_world_data_loads() {
    assert_eq!(shipped(), shipped());
}

#[test]
fn the_same_seed_builds_the_same_world() {
    let again = generate(42, &shipped());
    assert!(again == *seed_42(), "seed 42 built two different worlds");
}

#[test]
fn a_different_seed_builds_a_different_world() {
    let other = generate(43, &shipped());
    assert_ne!(other.fingerprint(), seed_42().fingerprint());
}

#[test]
fn seed_42_builds_the_golden_world() {
    let plan = seed_42();
    let summary = (
        plan.cities.len(),
        plan.buildings.len(),
        plan.highways.len(),
        plan.waters.rivers.len(),
        woods_at_start().grown(),
        plan.fingerprint(),
    );
    assert_eq!(
        summary,
        (
            GOLDEN_CITIES,
            GOLDEN_BUILDINGS,
            GOLDEN_HIGHWAYS,
            GOLDEN_RIVERS,
            GOLDEN_TREES_AT_START,
            GOLDEN_FINGERPRINT
        ),
        "the seed-42 world changed; if that was intended, update the golden values"
    );
}

// Pinned 2026-10-24 (the island world, openspec change island-world) on the owner's Mac (aarch64).
// The generator uses only IEEE arithmetic plus `powf` for storeys and `sqrt`, `cos` and `sin` for
// river sources; if another platform disagrees, the difference is in those calls.
const GOLDEN_CITIES: usize = 24;
const GOLDEN_BUILDINGS: usize = 6100;
const GOLDEN_HIGHWAYS: usize = 23;
const GOLDEN_RIVERS: usize = 6;
const GOLDEN_TREES_AT_START: usize = 14044;
const GOLDEN_FINGERPRINT: u64 = 8540391024908431296;

#[test]
fn seed_42_census_for_the_design() {
    // The counts the island-world design's cost table quotes (run with --nocapture).
    let plan = seed_42();
    let cfg = shipped();
    let km: f32 = plan.highways.iter().map(|h| h.length_m()).sum::<f32>() / 1000.0;
    let (mut land, mut total, mut top) = (0, 0, f32::MIN);
    let step = 250.0;
    let n = (cfg.world_size_m / step) as i32;
    for iz in 0..n {
        for ix in 0..n {
            let p = Vec2::splat(-cfg.world_size_m * 0.5) + Vec2::new(ix as f32, iz as f32) * step;
            let h = plan.height_at(p.x, p.y);
            total += 1;
            if h > cfg.island.sea_level_m {
                land += 1;
            }
            top = top.max(h);
        }
    }
    let land_km2 = land as f32 / total as f32 * (cfg.world_size_m / 1000.0).powi(2);
    eprintln!(
        "seed 42: {} cities, {} buildings, {} highways ({km:.0} km), {} rivers, {} bridges, {} cloud puffs, \
         {land_km2:.0} km2 of land, highest ground {top:.0} m",
        plan.cities.len(),
        plan.buildings.len(),
        plan.highways.len(),
        plan.waters.rivers.len(),
        plan.waters.bridges.len(),
        plan.clouds.puffs.len(),
    );
    assert!(land_km2 > 300.0 && land_km2 < 1000.0, "{land_km2} km2 of land");
}

#[test]
fn cities_keep_their_minimum_spacing() {
    let cfg = shipped();
    let cities = &seed_42().cities;
    assert!(
        cities.len() >= 12,
        "seed 42 places at least twelve cities on the island: {}",
        cities.len()
    );
    for (i, a) in cities.iter().enumerate() {
        for b in &cities[i + 1..] {
            assert!(a.centre.distance(b.centre) >= cfg.city_min_spacing_m);
            let gap = (a.centre - b.centre).abs() - Vec2::splat(a.half_size_m + b.half_size_m);
            assert!(
                gap.max_element() >= cfg.open_ground_min_m,
                "open ground between cities {} and {}",
                a.id,
                b.id
            );
        }
    }
}

#[test]
fn every_building_stands_in_a_city() {
    let plan = seed_42();
    for b in &plan.buildings {
        let city = &plan.cities[b.city as usize];
        assert!(city.rect().inset(-0.01).unwrap().overlaps(&b.footprint));
        assert!(b.footprint.min.x >= city.rect().min.x && b.footprint.max.y <= city.rect().max.y);
    }
}

#[test]
fn streets_are_wide_enough_for_a_mech() {
    for c in &seed_42().cities {
        for s in &c.streets {
            assert!(s.width_m >= 24.0, "street {:?} is {} m", s.rect, s.width_m);
            let narrow = s.rect.size().min_element();
            assert!(narrow >= 24.0 - 1e-3, "street piece {:?} narrower than 24 m", s.rect);
        }
    }
}

#[test]
fn street_pieces_never_overlap() {
    for c in &seed_42().cities {
        for (i, a) in c.streets.iter().enumerate() {
            for b in &c.streets[i + 1..] {
                let shrunk = a.rect.inset(0.01).unwrap();
                assert!(!shrunk.overlaps(&b.rect), "{:?} overlaps {:?}", a.rect, b.rect);
            }
        }
    }
}

#[test]
fn no_building_stands_on_a_street() {
    let plan = seed_42();
    for b in &plan.buildings {
        for s in &plan.cities[b.city as usize].streets {
            assert!(!s.rect.overlaps(&b.footprint), "building {} on street {:?}", b.id, s.rect);
        }
    }
}

#[test]
fn buildings_run_from_two_storeys_to_forty_and_more() {
    let plan = seed_42();
    let lo = plan.buildings.iter().map(|b| b.storeys).min().unwrap();
    let hi = plan.buildings.iter().map(|b| b.storeys).max().unwrap();
    assert_eq!(lo, 2);
    assert!(hi >= 40, "tallest is {hi} storeys");
}

#[test]
fn building_ids_are_their_index() {
    for (i, b) in seed_42().buildings.iter().enumerate() {
        assert_eq!(b.id as usize, i);
    }
}

#[test]
fn a_city_is_flat_at_its_ground_height() {
    let plan = seed_42();
    for c in &plan.cities {
        let r = c.rect();
        for k in 0..20 {
            let t = k as f32 / 19.0;
            let p = r.min + r.size() * Vec2::new(t, 1.0 - t);
            assert!((plan.height_at(p.x, p.y) - c.ground_m).abs() < 1e-3, "city {} at {p}", c.id);
        }
    }
}

#[test]
fn the_ground_under_a_highway_is_the_road_grade() {
    let plan = seed_42();
    assert!(!plan.highways.is_empty());
    for h in &plan.highways {
        let along = (h.ends[1] - h.ends[0]).normalize();
        let across = along.perp();
        let steps = (h.length_m() / 8.0) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            for side in [-0.5, 0.0, 0.5] {
                let p = h.ends[0] + (h.ends[1] - h.ends[0]) * t + across * h.width_m * side;
                if plan.waters.bridges.iter().any(|b| b.centre.distance(p) < b.length_m * 0.5 + 2.0) {
                    continue; // On a bridge: the river runs under the deck.
                }
                let (_, tt) = h.project(p);
                let got = plan.height_at(p.x, p.y);
                // The drawn road sits `road_lift_m` (0.1 m) above the grade; the ground's straight
                // triangles may only sag or bulge a fraction of that on the road's S-curve.
                assert!(
                    (got - h.grade_at(tt)).abs() < 0.05,
                    "highway {:?} at {p}: {got} vs {}",
                    h.cities,
                    h.grade_at(tt)
                );
            }
        }
    }
}

#[test]
fn the_start_is_on_a_street_clear_of_buildings() {
    let plan = seed_42();
    assert!(!plan.collision.overlaps(plan.start.mech_xz, 3.0));
    assert!(!plan.collision.overlaps(plan.start.pilot_xz, 0.35));
}

#[test]
fn the_start_is_in_the_capital_near_the_islands_centre() {
    let plan = seed_42();
    let cfg = shipped();
    let capital = &plan.cities[0];
    assert_eq!(capital.half_size_m, cfg.city_half_size_m[1], "the capital is the largest city");
    assert!(capital.centre.length() <= cfg.capital_within_m);
    assert!(capital.rect().distance(plan.start.mech_xz) < 1e-3);
}

#[test]
fn every_encounter_starts_on_dry_land() {
    let plan = seed_42();
    let enc: crate::ai::Encounters = load_ron(&data_path("data/world/encounters.ron")).expect("encounters load");
    for (i, e) in enc.enemies.iter().enumerate() {
        let p = plan.start.mech_xz + Vec2::from_array(e.offset_m);
        assert!(plan.water_at(p.x, p.y).is_none(), "enemy {i} at {p} starts in water");
        assert!(!plan.collision.overlaps(p, 3.0), "enemy {i} at {p} starts in a building");
    }
}

#[test]
fn the_island_has_land_in_the_middle_and_sea_at_the_edges() {
    let plan = seed_42();
    let cfg = shipped();
    let sea = cfg.island.sea_level_m;
    // Most of the ground within 8 km of the centre is dry land.
    let mut dry = 0;
    let mut total = 0;
    for iz in -8..=8 {
        for ix in -8..=8 {
            let p = Vec2::new(ix as f32, iz as f32) * 1000.0;
            if p.length() > 8000.0 {
                continue;
            }
            total += 1;
            if plan.height_at(p.x, p.y) > sea {
                dry += 1;
            }
        }
    }
    assert!(dry * 10 >= total * 9, "{dry} of {total} points within 8 km are dry");
    // Every point along the world's edge, and past it, is under the sea.
    let half = cfg.world_size_m * 0.5;
    for k in 0..400 {
        let t = -half + cfg.world_size_m * k as f32 / 400.0;
        for p in [
            Vec2::new(t, -half),
            Vec2::new(t, half),
            Vec2::new(-half, t),
            Vec2::new(half, t),
            Vec2::new(t * 1.7, half * 1.4),
        ] {
            let w = plan.water_at(p.x, p.y).unwrap_or_else(|| panic!("no sea at {p}"));
            assert_eq!(w.surface_m, sea);
            assert_eq!(w.flow_m_s, Vec2::ZERO);
        }
    }
}

#[test]
fn the_open_sea_floor_is_between_twenty_and_sixty_metres_down() {
    let plan = seed_42();
    let cfg = shipped();
    let i = &cfg.island;
    let shelf_ends = i.land_radius_m + i.coast_noise_m + i.shelf_m;
    let mut checked = 0;
    for k in 0..2000 {
        let a = k as f32 * 0.618_034 * std::f32::consts::TAU;
        let r = shelf_ends + (k % 13) as f32 * 900.0;
        let p = Vec2::new(a.cos(), a.sin()) * r;
        let depth = i.sea_level_m - plan.height_at(p.x, p.y);
        assert!((20.0..=60.0).contains(&depth), "the sea is {depth} m deep at {p}");
        assert_eq!(plan.water_at(p.x, p.y).map(|w| w.depth_m), Some(depth));
        checked += 1;
    }
    assert_eq!(checked, 2000);
    // At the world's edge the floor is exactly the open sea floor (it meets the flat floor beyond).
    let half = cfg.world_size_m * 0.5;
    assert_eq!(plan.height_at(half, 1234.0), i.sea_level_m - i.sea_depth_m);
}

#[test]
fn the_shore_shelves_gently_enough_to_walk_out_of_the_sea() {
    // Along rays from the centre, the ground between the open floor and the beach never climbs
    // faster than 1 in 4 (a mech walks up 1 in 2): a mech that sank can walk ashore.
    let plan = seed_42();
    let cfg = shipped();
    let i = &cfg.island;
    for k in 0..64 {
        let a = k as f32 / 64.0 * std::f32::consts::TAU;
        let dir = Vec2::new(a.cos(), a.sin());
        let mut last: Option<f32> = None;
        let start = i.land_radius_m - i.coast_noise_m;
        let mut r = start;
        while r < i.land_radius_m + i.coast_noise_m + i.shelf_m {
            let p = dir * r;
            let h = plan.height_at(p.x, p.y);
            let near_city = plan.cities.iter().any(|c| c.rect().distance(p) < 300.0);
            if let Some(l) = last
                && h < i.sea_level_m
                && !near_city
            {
                assert!((l - h) / 32.0 < 0.25, "the sea floor climbs {} m in 32 m at {p}", l - h);
            }
            last = Some(h);
            r += 32.0;
        }
    }
}

#[test]
fn seed_42_has_rivers_and_the_sea() {
    let w = &seed_42().waters;
    assert!(w.rivers.len() >= 3, "rivers: {}", w.rivers.len());
    assert!(seed_42().water_at(19_000.0, 0.0).is_some(), "the sea");
}

#[test]
fn a_river_always_runs_downhill() {
    for r in &seed_42().waters.rivers {
        for w in r.surface_m.windows(2) {
            assert!(w[1] <= w[0], "the surface rose from {} to {}", w[0], w[1]);
        }
    }
}

#[test]
fn no_city_square_is_under_water() {
    let plan = seed_42();
    for c in &plan.cities {
        let r = c.rect();
        for k in 0..400 {
            let t = Vec2::new((k % 20) as f32 / 19.0, (k / 20) as f32 / 19.0);
            let p = r.min + r.size() * t;
            assert!(plan.water_at(p.x, p.y).is_none(), "city {} is wet at {p}", c.id);
        }
    }
}

#[test]
fn rivers_keep_clear_of_cities() {
    let plan = seed_42();
    let cfg = shipped();
    for r in &plan.waters.rivers {
        for p in &r.path {
            for c in &plan.cities {
                assert!(
                    c.rect().distance(*p) >= cfg.water.river_city_clearance_m - cfg.water.river_grid_m,
                    "river at {p} near city {}",
                    c.id
                );
            }
        }
    }
}

#[test]
fn a_river_centre_is_under_water_and_its_depth_is_surface_minus_ground() {
    let plan = seed_42();
    for r in &plan.waters.rivers {
        let mid = r.path.len() / 2;
        let p = r.path[mid];
        let w = plan.water_at(p.x, p.y).expect("water at a river's centre");
        assert!((w.surface_m - w.depth_m - plan.height_at(p.x, p.y)).abs() < 1e-3);
        assert!(w.depth_m > 2.0, "a river is deep at its centre: {}", w.depth_m);
    }
}

#[test]
fn every_highway_crossing_has_a_bridge_at_road_grade() {
    let plan = seed_42();
    for b in &plan.waters.bridges {
        let h = plan
            .highways
            .iter()
            .find(|h| h.project(b.centre).0 < 1.0)
            .expect("a bridge is on a highway");
        let (_, t) = h.project(b.centre);
        assert!((b.deck_m - h.grade_at(t)).abs() < 1e-3);
    }
}

#[test]
fn woods_grow_round_the_start_clear_of_roads_water_and_buildings() {
    let plan = seed_42();
    let f = woods_at_start();
    assert!(f.grown() > 2000, "woods: {} trees", f.grown());
    for (_, trees) in f.grown_tiles() {
        for t in trees.iter().step_by(7) {
            let p = Vec2::new(t.base.x, t.base.z);
            assert!(plan.water_at(p.x, p.y).is_none(), "tree {} in water", t.id);
            assert!(
                plan.highways.iter().all(|h| h.project(p).0 > h.width_m * 0.5),
                "tree {} on a highway",
                t.id
            );
            assert!(!plan.collision.overlaps(p, 1.0), "tree {} in a building", t.id);
            for c in &plan.cities {
                for st in &c.streets {
                    assert!(st.rect.distance(p) > 0.0, "tree {} on a street", t.id);
                }
            }
        }
    }
}

#[test]
fn trees_grown_in_any_order_are_the_same_trees() {
    let plan = seed_42();
    let p = plan.start.mech_xz + Vec2::new(600.0, -300.0);
    let mut a = plan.forest.clone();
    a.grow_around(plan, p, 500.0);
    let mut b = plan.forest.clone();
    b.grow_around(plan, p + Vec2::new(400.0, 0.0), 300.0);
    b.grow_around(plan, p, 500.0);
    for id in a.near(p, 400.0) {
        assert_eq!(a.tree(id), b.tree(id), "tree {id}");
    }
    assert_eq!(a.near(p, 400.0), b.near(p, 400.0));
}

#[test]
fn a_felled_tree_stays_felled_when_its_tile_is_let_go_and_grown_again() {
    let plan = seed_42();
    let cfg = shipped().trees;
    let here = plan.start.mech_xz + Vec2::new(500.0, 500.0);
    let mut f = plan.forest.clone();
    f.keep_around(plan, &[here]);
    let id = f.near(here, 300.0).into_iter().next().expect("a tree near the start");
    assert!(f.hit(id, Vec2::X, f32::MAX));
    let away = here + Vec2::new(0.0, cfg.release_radius_m + 2000.0);
    let changes = f.keep_around(plan, &[away]);
    assert!(!changes.released.is_empty());
    assert!(f.tree(id).is_none(), "its tile was let go");
    assert!(matches!(f.state(id), TreeState::Fallen { .. }), "remembered as fallen");
    f.keep_around(plan, &[here]);
    assert!(f.tree(id).is_some(), "its tile grew again");
    assert!(matches!(f.state(id), TreeState::Fallen { .. }), "and it still lies felled");
}

#[test]
fn growing_trees_is_bounded_by_the_bodies_not_the_island() {
    // However big the island, the trees kept are those within the keep radius of each body, and a
    // step grows at most its urgent tiles plus `tiles_per_step` more.
    let plan = seed_42();
    let cfg = shipped().trees;
    let mut f = plan.forest.clone();
    let first = f.keep_around(plan, &[plan.start.mech_xz]);
    let tile_m = cfg.tile_sites as f32 * cfg.spacing_m;
    let urgent = std::f32::consts::PI * ((cfg.urgent_radius_m + tile_m * 1.5) / tile_m).powi(2);
    assert!(
        first.grown.len() as f32 <= urgent + cfg.tiles_per_step as f32,
        "{} tiles in one step",
        first.grown.len()
    );
    for _ in 0..400 {
        f.keep_around(plan, &[plan.start.mech_xz]);
    }
    let keep = std::f32::consts::PI * ((cfg.keep_radius_m + tile_m * 1.5) / tile_m).powi(2);
    let tiles = f.grown_tiles().count();
    assert!(tiles as f32 <= keep, "{tiles} tiles kept for one body");
    let most = (keep * (cfg.tile_sites * cfg.tile_sites) as f32) as usize;
    assert!(f.grown() < most, "{} trees", f.grown());
}

#[test]
fn ground_chunks_cost_less_the_coarser_they_are_drawn() {
    // A timing of the one ground mesh builder at every step, for the design's cost table. Not a
    // pass mark (cloud machines and the test profile differ); the triangle counts are.
    let plan = seed_42();
    let cfg = shipped();
    let cells = cfg.terrain_lod.chunk_cells as usize;
    let t = &plan.terrain;
    let chunk_m = cells as f32 * t.cell_m;
    let cx = ((plan.start.mech_xz.x - t.origin_m) / chunk_m) as usize;
    let cz = ((plan.start.mech_xz.y - t.origin_m) / chunk_m) as usize;
    let mut step = 1;
    while step <= cfg.terrain_lod.max_step() {
        let started = std::time::Instant::now();
        let m = t.chunk_mesh_lod(cx, cz, cells, step, lod::skirt_m(step, t.cell_m), |_, _| false);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(m.indices.len() / 3, lod::chunk_triangles(cells, step));
        eprintln!(
            "chunk at step {step:>2}: {:>5} triangles, {:>5} vertices, built in {ms:.2} ms (test profile)",
            m.indices.len() / 3,
            m.positions.len()
        );
        step *= 2;
    }
}

#[test]
fn the_lod_rings_keep_the_ground_to_a_few_hundred_thousand_triangles() {
    let plan = seed_42();
    let cfg = shipped();
    let cells = cfg.terrain_lod.chunk_cells as usize;
    let chunks = ((plan.terrain.n - 1) / cells).max(1);
    for (what, eye) in [
        (
            "on foot at the start",
            Vec3::new(plan.start.pilot_xz.x, plan.cities[0].ground_m + 1.7, plan.start.pilot_xz.y),
        ),
        ("flying 1500 m up over the centre", Vec3::new(0.0, 1500.0, 0.0)),
        ("on the beach", Vec3::new(14_500.0, 3.0, 0.0)),
    ] {
        let rows = lod::census(&plan.terrain, cells, chunks, eye, cfg.island.sea_level_m, &cfg.terrain_lod);
        let total: usize = rows.iter().map(|r| r.2).sum();
        let count: usize = rows.iter().map(|r| r.1).sum();
        let (meshes, tris) = lod::draws(&plan.terrain, cells, chunks, eye, cfg.island.sea_level_m, &cfg.terrain_lod);
        eprintln!(
            "{what}: {count} chunks, {total} triangles: {rows:?} (step, chunks, triangles); \
             drawn as a quadtree: {meshes} meshes, {tris} triangles"
        );
        assert_eq!(count, chunks * chunks);
        assert!(total < 700_000, "{what}: {total} triangles");
        assert!(tris < 900_000, "{what}: {tris} triangles");
        assert!(meshes < 200, "{what}: {meshes} ground meshes before culling");
    }
}
