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

/// The shipped data with rivers on (the stream threshold the hydrology was built and tuned at): the
/// game ships with none (owner, 2026-10-25: "delete all rivers"), but the river code stays tested
/// for when they come back.
fn shipped_with_rivers() -> WorldConfig {
    let mut cfg = shipped();
    cfg.water.rivers = true;
    cfg.water.stream_min_flow = 12.0;
    cfg
}

#[test]
fn the_shipped_world_has_lakes_and_the_sea_and_no_rivers() {
    let plan = generate(42, &shipped());
    let w = &plan.waters;
    assert!(
        w.rivers.is_empty() && w.falls.is_empty() && w.bridges.is_empty(),
        "no rivers, falls or bridges"
    );
    assert!(!w.lakes.list.is_empty(), "the basin lakes stay");
    assert!(plan.water_at(19_000.0, 0.0).is_some(), "the sea");
    // Nothing is carved for a river: the ground is the ground of the same world without rivers'
    // channels, away from the lakes.
    let rivers = seed_42();
    let r = &rivers.waters.rivers[0];
    let p = r.path[r.path.len() / 2];
    if w.lakes.at(p).is_none() {
        assert!(
            plan.terrain.height_at(p.x, p.y) > rivers.terrain.height_at(p.x, p.y),
            "the river's bed is not dug"
        );
    }
}

fn seed_42() -> &'static WorldPlan {
    static PLAN: OnceLock<WorldPlan> = OnceLock::new();
    PLAN.get_or_init(|| {
        let started = std::time::Instant::now();
        let plan = generate(42, &shipped_with_rivers());
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
    let again = generate(42, &shipped_with_rivers());
    assert!(again == *seed_42(), "seed 42 built two different worlds");
}

#[test]
fn a_different_seed_builds_a_different_world() {
    let other = generate(43, &shipped_with_rivers());
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

// Pinned 2026-10-25 in a Linux cloud session (x86_64), when the island mockup's land and its ten
// biomes replaced the crater (openspec change `island-biomes`), under the drainage, the lakes, the
// waterfalls (`hydrology`) and the routed highways (`curved-roads`). The generator uses IEEE
// arithmetic plus `powf`, `sqrt`, `atan2`, `hypot`, `exp`, `sin` and `cos` (the island's gradient
// noise); if another platform disagrees, the difference is in those calls.
const GOLDEN_CITIES: usize = 13;
const GOLDEN_BUILDINGS: usize = 3609;
const GOLDEN_HIGHWAYS: usize = 12;
const GOLDEN_RIVERS: usize = 64;
const GOLDEN_TREES_AT_START: usize = 38407;
// 2026-10-25: the five air bases (air-bases) flatten their sites; counts unchanged.
const GOLDEN_FINGERPRINT: u64 = 2360908105245372907;

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
    let river_km: f32 = plan
        .waters
        .rivers
        .iter()
        .map(|r| r.path.windows(2).map(|w| w[0].distance(w[1])).sum::<f32>())
        .sum::<f32>()
        / 1000.0;
    let fall_heights = plan
        .waters
        .falls
        .iter()
        .map(|f| format!("{:.0}", f.top_m - f.bottom_m))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!(
        "seed 42: {} cities, {} buildings, {} highways ({km:.0} km), {} rivers ({river_km:.0} km), {} lakes \
         ({} plunge pools), {} waterfalls ({fall_heights}), {} bridges, {} cloud puffs, \
         {land_km2:.0} km2 of land, highest ground {top:.0} m",
        plan.cities.len(),
        plan.buildings.len(),
        plan.highways.len(),
        plan.waters.rivers.len(),
        plan.waters.lakes.list.len(),
        plan.waters.lakes.list.iter().filter(|l| l.plunge).count(),
        plan.waters.falls.len(),
        plan.waters.bridges.len(),
        plan.clouds.puffs.len(),
    );
    assert!(land_km2 > 300.0 && land_km2 < 1000.0, "{land_km2} km2 of land");
    let mut kinds = std::collections::BTreeMap::new();
    for p in &plan.clouds.puffs {
        *kinds.entry(p.kind).or_insert(0usize) += 1;
    }
    eprintln!(
        "seed 42 sky: {} puffs ({kinds:?}), {} rain shafts, {} darkened cells, {} cirrus",
        plan.clouds.puffs.len(),
        plan.clouds.shafts.len(),
        plan.clouds.shades.len(),
        plan.clouds.cirrus.len()
    );
    let shares = biome_shares(plan, &cfg);
    eprintln!(
        "seed 42 biomes (share of land): {}",
        BIOMES
            .iter()
            .zip(shares)
            .map(|(b, s)| format!("{b:?} {:.1}%", s * 100.0))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The share of the land where each biome is strongest, sampled every 160 m.
fn biome_shares(plan: &WorldPlan, cfg: &WorldConfig) -> [f32; BIOME_COUNT] {
    let step = 160.0;
    let n = (cfg.world_size_m / step) as i32;
    let mut count = [0u32; BIOME_COUNT];
    let mut land = 0u32;
    for iz in 0..n {
        for ix in 0..n {
            let p = Vec2::splat(-cfg.world_size_m * 0.5) + Vec2::new(ix as f32, iz as f32) * step;
            let h = plan.height_at(p.x, p.y);
            if h <= cfg.island.sea_level_m || plan.water_at(p.x, p.y).is_some() {
                continue;
            }
            let slope = Vec2::new(plan.height_at(p.x + 4.0, p.y) - h, plan.height_at(p.x, p.y + 4.0) - h).length() / 4.0;
            let b = plan.biomes.dominant(p, h, slope);
            count[BIOMES.iter().position(|x| *x == b).unwrap_or(0)] += 1;
            land += 1;
        }
    }
    count.map(|c| c as f32 / land.max(1) as f32)
}

#[test]
fn a_quarter_of_the_land_is_desert() {
    // Owner, 2026-10-25: "Desert should be 1/4 the island".
    let shares = biome_shares(seed_42(), &shipped());
    let desert = shares[6] + shares[7];
    assert!(
        (0.2..=0.3).contains(&desert),
        "desert and canyon are {:.1}% of the land",
        desert * 100.0
    );
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
        let steps = (h.length_m() / 8.0) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            for side in [-0.5, 0.0, 0.5] {
                let p = h.point_at(t) + h.dir_at(t).perp() * h.width_m * side;
                if plan.waters.bridges.iter().any(|b| b.centre.distance(p) < b.length_m * 0.5 + 2.0) {
                    continue; // On a bridge: the river runs under the deck.
                }
                let (_, tt) = h.project(p);
                let got = plan.height_at(p.x, p.y);
                // The drawn road sits `road_lift_m` (0.15 m) above the grade; the ground's straight
                // 8 m triangles may only sag or bulge less than that under a climbing bend.
                assert!(
                    (got - h.grade_at(tt)).abs() < 0.15,
                    "highway {:?} at {p}: {got} vs {}",
                    h.cities,
                    h.grade_at(tt)
                );
            }
        }
    }
}

/// The island's lane graph (car-traffic): every lane leads on, so no car ever dead-ends, and every
/// highway's lanes join two cities.
#[test]
fn every_lane_on_the_island_has_a_way_out() {
    let plan = seed_42();
    let cfg = shipped();
    let g = lanes::LaneGraph::build(&plan.cities, &plan.highways, &lanes::LaneLayout::default(), cfg.arterial_width_m);
    assert!(g.lanes.len() > 1000, "{} lanes", g.lanes.len());
    for l in &g.lanes {
        assert!(!l.out.is_empty(), "lane {} ({:?}) dead-ends", l.id, l.kind);
    }
    let highway_lanes = g.road_lanes().filter(|l| l.kind == lanes::LaneKind::Highway).count();
    assert_eq!(highway_lanes, plan.highways.len() * 4, "two lanes each way on every highway");
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
fn the_sea_floor_meets_the_flat_floor_at_the_worlds_edge() {
    // The island mockup's sea falls to 230 m; over the last `edge_fade_m` it rises or falls to the
    // flat floor drawn past the world's edge, so the two meet exactly.
    let plan = seed_42();
    let cfg = shipped();
    let i = &cfg.island;
    let half = cfg.world_size_m * 0.5;
    for k in 0..40 {
        let along = -half + k as f32 * cfg.world_size_m / 40.0;
        for p in [Vec2::new(half, along), Vec2::new(along, -half)] {
            assert!((plan.height_at(p.x, p.y) - (i.sea_level_m - i.sea_depth_m)).abs() < 1e-3, "at {p}");
            assert!(plan.water_at(p.x, p.y).is_some());
        }
    }
}

#[test]
fn the_shore_shelves_gently_enough_to_walk_out_of_the_sea() {
    // Along rays from the centre, the sea floor in the shallows (2-30 m) never climbs faster than
    // 1 in 4 (a mech walks up 1 in 2): a mech that sank near the shore can walk out, except under
    // the north-east's sea cliffs and round the archipelago's islets.
    let plan = seed_42();
    let cfg = shipped();
    let i = &cfg.island;
    let half = cfg.world_size_m * 0.5 - i.edge_fade_m;
    for k in 0..64 {
        let a = k as f32 / 64.0 * std::f32::consts::TAU;
        let dir = Vec2::new(a.cos(), a.sin());
        let mut last: Option<f32> = None;
        let mut r = 8000.0;
        while r < half {
            let p = dir * r;
            let h = plan.height_at(p.x, p.y);
            let near_city = plan.cities.iter().any(|c| c.rect().distance(p) < 300.0);
            // The north-east's sea cliffs and the cliffed islets drop straight into the water: no
            // walking ashore there.
            let f = plan.terrain.shape().height_and_fields(p).1;
            let cliff = f.cliff_coast > 0.05 || f.islet_q < 1.6;
            // The shallows: past the shelf the floor falls to 230 m (the mockup's sea), deeper than
            // any mech stands.
            let shallow = |y: f32| (i.sea_level_m - 30.0..i.sea_level_m - 2.0).contains(&y);
            if let Some(l) = last
                && shallow(h)
                && shallow(l)
                && !near_city
                && !cliff
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
                    c.rect().distance(*p) >= cfg.water.river_city_clearance_m - cfg.water.drainage_cell_m * 0.5,
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
        // A stream's channel is as deep as it is half wide (at least 5 m wide), a river's 7 m.
        assert!(w.depth_m > 1.0, "a river is deep at its centre: {}", w.depth_m);
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
    // step grows at most its must and urgent tiles plus `tiles_per_step` more.
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

#[test]
fn a_body_new_to_a_place_grows_its_near_trees_at_once_and_the_rest_a_few_a_step() {
    let plan = seed_42();
    let cfg = shipped().trees;
    let mut f = plan.forest.clone();
    let here = plan.start.mech_xz;
    for _ in 0..400 {
        f.keep_around_seen(plan, &[here], 1);
    }
    // An enemy brought in 6 km off: its whole 400 m ring used to grow in one step.
    let new = here + Vec2::new(6000.0, 0.0);
    let first = f.keep_around_seen(plan, &[here, new], 1);
    let grown = |f: &Forest| f.grown_tiles().map(|(k, _)| k).collect::<std::collections::BTreeSet<_>>();
    let now = grown(&f);
    assert!(
        f.tiles_within(new, cfg.must_radius_m).iter().all(|k| now.contains(k)),
        "its near trees stand at once"
    );
    let tile_m = cfg.tile_sites as f32 * cfg.spacing_m;
    let must_most = std::f32::consts::PI * ((cfg.must_radius_m + tile_m * 1.5) / tile_m).powi(2);
    assert!(
        first.grown.len() as f32 <= must_most + cfg.urgent_tiles_per_step as f32,
        "{} tiles grown in one step",
        first.grown.len()
    );
    let ring = f.tiles_within(new, cfg.urgent_radius_m);
    assert!(ring.iter().any(|k| !now.contains(k)), "the rest of the ring waits");
    for _ in 0..ring.len() {
        f.keep_around_seen(plan, &[here, new], 1);
    }
    let later = grown(&f);
    assert!(ring.iter().all(|k| later.contains(k)), "and is grown within a few steps");
}

#[test]
fn only_the_seen_body_grows_trees_out_to_the_keep_radius() {
    let plan = seed_42();
    let cfg = shipped().trees;
    let mut f = plan.forest.clone();
    let here = plan.start.mech_xz;
    let other = here + Vec2::new(cfg.keep_radius_m * 3.0, 0.0);
    for _ in 0..2000 {
        f.keep_around_seen(plan, &[here, other], 1);
    }
    let far_of = |p: Vec2| {
        f.grown_tiles()
            .map(|(k, _)| f.tile_rect(k).distance(p))
            .filter(|d| *d < cfg.keep_radius_m * 1.5)
            .fold(0.0f32, f32::max)
    };
    assert!(
        far_of(here) > cfg.urgent_radius_m * 2.0,
        "the seen body's woods stop at {}",
        far_of(here)
    );
    assert!(
        far_of(other) <= cfg.urgent_radius_m + 1.0,
        "the other body's woods reach {}",
        far_of(other)
    );
}

#[test]
fn the_crater_wall_has_waterfalls_at_least_the_least_height() {
    let plan = seed_42();
    let cfg = shipped();
    assert!(!plan.waters.falls.is_empty(), "no waterfalls");
    for f in &plan.waters.falls {
        assert!(
            f.top_m - f.bottom_m >= cfg.water.fall_min_height_m,
            "a fall of {} m",
            f.top_m - f.bottom_m
        );
    }
}

#[test]
fn a_lakes_water_stands_at_its_level() {
    let plan = seed_42();
    let cfg = shipped();
    let basins = plan.waters.lakes.list.iter().filter(|l| !l.plunge).count();
    assert!(basins as u32 <= cfg.water.lake_max_count, "{basins} basin lakes");
    for l in &plan.waters.lakes.list {
        let w = plan.water_at(l.centre.x, l.centre.y).expect("water at a lake's centre");
        assert!(
            w.surface_m >= l.level_m - 1e-3,
            "lake at {} has its surface at {}",
            l.level_m,
            w.surface_m
        );
    }
}

#[test]
fn the_capital_stands_on_low_ground_near_the_centre() {
    // The range fills the middle: the capital is the first site within `capital_within_m` on low,
    // level land (the mockup's capital is on the northern plain about 7.5 km out).
    let plan = seed_42();
    let cfg = shipped();
    let c = &plan.cities[0];
    assert!(c.centre.length() < cfg.capital_within_m, "capital at {}", c.centre);
    assert!(c.ground_m < cfg.island.sea_level_m + cfg.city_max_ground_m);
}

/// Instrument (river water vs the carved ground): how much of every river's centre line has its water
/// surface below the ground there, the water then hidden in a dry channel. Run with --ignored --nocapture.
#[test]
#[ignore]
fn measure_river_water_against_the_ground() {
    let plan = seed_42();
    let (mut points, mut buried, mut worst) = (0usize, 0usize, 0.0f32);
    let mut per_river = Vec::new();
    for (ri, r) in plan.waters.rivers.iter().enumerate() {
        let mut b = 0;
        for (i, p) in r.path.iter().enumerate() {
            let ground = plan.terrain.height_at(p.x, p.y);
            let under = ground - r.surface_m[i];
            points += 1;
            if under > 0.05 {
                buried += 1;
                b += 1;
                worst = worst.max(under);
            }
        }
        per_river.push((ri, r.path.len(), b));
    }
    eprintln!(
        "rivers {}, points {points}, buried {buried} ({:.1} %), worst {worst:.1} m under",
        plan.waters.rivers.len(),
        100.0 * buried as f32 / points.max(1) as f32
    );
    per_river.sort_by_key(|r| std::cmp::Reverse(r.2));
    for (ri, n, b) in per_river.iter().take(8) {
        eprintln!("  river {ri}: {b} of {n} points buried");
    }
}

#[test]
fn far_terrain_never_hides_a_rivers_water() {
    // At every coarse step, the four corners of the far cell holding a river's water lie under its
    // surface, so the water is drawn over the far ground (owner, 2026-10-25: "rivers misssing").
    let plan = seed_42();
    let t = &plan.terrain;
    let road_flat = shipped_with_rivers().highway_width_m * 0.5 + shipped_with_rivers().highway_flat_margin_m;
    let (mut checked, mut over, mut worst) = (0, 0, 0.0f32);
    let mut before = 0;
    for (ri, r) in plan.waters.rivers.iter().enumerate() {
        for (i, p) in r.path.iter().enumerate().step_by(7) {
            // A waterfall's water is its falling sheet, not the river's ribbon.
            if plan.waters.falls.iter().any(|f| f.river == ri && i + 2 >= f.lip && i <= f.foot + 2) {
                continue;
            }
            if plan.highways.iter().any(|h| h.project(*p).0 <= road_flat + 200.0)
                || plan.cities.iter().any(|c| c.rect().distance(*p) < 300.0)
            {
                continue;
            }
            for step in [2usize, 4, 8, 16] {
                let cell = t.cell_m * step as f32;
                let ix = (((p.x - t.origin_m) / cell).floor() as i64) * step as i64;
                let iz = (((p.y - t.origin_m) / cell).floor() as i64) * step as i64;
                // The far ground at the river point, on the two triangles the far mesh draws.
                let s = step as i64;
                let corners = [
                    t.lattice_at_step(ix, iz, step),
                    t.lattice_at_step(ix + s, iz, step),
                    t.lattice_at_step(ix, iz + s, step),
                    t.lattice_at_step(ix + s, iz + s, step),
                ];
                let u = (p.x - (t.origin_m + ix as f32 * t.cell_m)) / cell;
                let v = (p.y - (t.origin_m + iz as f32 * t.cell_m)) / cell;
                let ground = super::terrain::cell_height(corners, u, v);
                let full = [(0, 0), (s, 0), (0, s), (s, s)].map(|(dx, dz)| t.lattice_at_step(ix + dx, iz + dz, 1));
                if super::terrain::cell_height(full, u, v) > r.surface_m[i] + 0.1 {
                    before += 1;
                }
                if ground > r.surface_m[i] + 0.1 {
                    over += 1;
                    worst = worst.max(ground - r.surface_m[i]);
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 1000, "{checked}");
    eprintln!("far river points hidden: {over} of {checked} (worst {worst:.1} m); with the full carve: {before}");
    // Measured 2026-10-25 on seed 42: 4 710 of 12 648 far river points hidden with the full carve,
    // 148 (1.2 %) with the far carve, mostly where two rivers meet. Keep it under 2 %.
    assert!(
        over * 50 <= checked,
        "{over} of {checked} far river points hidden, worst {worst:.1} m"
    );
}

#[test]
fn the_baked_sun_shade_agrees_with_a_ray_toward_the_sun() {
    // Far ground is shaded by the bake (distant-shadows): where it says shade, a ray from the ground
    // toward the sun meets the terrain, and where it says sunlit, the ray is clear.
    let plan = seed_42();
    let t = &plan.terrain;
    let to_sun = super::sun_shade::to_sun();
    let started = std::time::Instant::now();
    let again = super::sun_shade::SunShade::bake(t, shipped().world_size_m, to_sun, &shipped().sun_shade);
    eprintln!(
        "sun shade baked in {:.0} ms (test profile)",
        started.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(&again, &plan.sun_shade, "the bake is deterministic");
    let half = shipped().world_size_m * 0.4;
    let mut rng = crate::rng::SimRng::new(7);
    let (mut shaded, mut sure, mut agree) = (0, 0, 0);
    for _ in 0..3000 {
        let xz = Vec2::new(rng.range_f32(-half, half), rng.range_f32(-half, half));
        let lit = plan.sun_shade.at(xz);
        if lit < 0.5 {
            shaded += 1;
        }
        if (0.1..=0.9).contains(&lit) {
            continue;
        }
        let ground = Vec3::new(xz.x, t.height_at(xz.x, xz.y), xz.y);
        let blocked = (1..400).any(|i| {
            let q = ground + to_sun * (i as f32 * 20.0);
            t.height_at(q.x, q.z) > q.y + 2.0
        });
        sure += 1;
        if blocked == (lit < 0.5) {
            agree += 1;
        }
    }
    eprintln!("{shaded} of 3000 in shade; {agree} of {sure} agree with the ray");
    assert!(shaded > 10, "the island's ridges cast shade: {shaded}");
    assert!(agree as f32 >= sure as f32 * 0.93, "{agree} of {sure}");
}

#[test]
fn a_pilot_standing_still_on_a_deck_over_the_city_stays_where_they_stand() {
    // A cloud run's regression (crab-airship section 9): standing still on the Stratostacean's deck, 600 m over the city,
    // the pilot slid 26 m across it, pushed out of the footprints of the buildings and trees far below.
    let plan = seed_42();
    let b = &plan.buildings[0];
    let over = b.footprint.centre();
    let top = plan.terrain.height_at(over.x, over.y) + 600.0;
    let deck = Platform {
        slabs: vec![Slab {
            name: "deck".into(),
            min: Vec2::splat(-200.0),
            max: Vec2::splat(200.0),
            centre: Vec2::ZERO,
            top_m: 0.0,
            slope: Vec2::ZERO,
        }],
        pose: glam::Affine3A::from_translation(Vec3::new(over.x, top, over.y)),
        was: glam::Affine3A::from_translation(Vec3::new(over.x, top, over.y)),
    };
    let dug = Deformation::new(4096.0);
    let structures = Structures::new(plan);
    let forest = Forest::default();
    let platforms = [deck];
    let scene = Scene {
        plan,
        dug: &dug,
        trees: Some(&forest),
        structures: Some(&structures),
        platforms: &platforms,
        interiors: None,
    };
    let start = Vec3::new(over.x, top, over.y);
    let mut p = crate::pilot::PilotState::standing(start, 0.0);
    let cfg = crate::pilot::PilotConfig::default();
    for _ in 0..120 {
        p.step(&crate::input::Actions::default(), &cfg, &scene, None, 1.0 / 60.0);
    }
    assert!(p.feet.distance(start) < 1e-3, "{start} -> {}", p.feet);
}

#[test]
fn no_rivers_water_lies_under_the_ground() {
    // Owner, 2026-10-25: "I see rivers still have holes in them". Every river's water is over the
    // ground along its whole course, joins included, except where a road's flat band or a bridge
    // crosses it (the road stands over the water by design) and on a waterfall's sheet.
    let plan = seed_42();
    let cfg = shipped();
    let road_flat = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
    let mut buried = Vec::new();
    for (ri, r) in plan.waters.rivers.iter().enumerate() {
        for (i, p) in r.path.iter().enumerate() {
            if plan.waters.falls.iter().any(|f| f.river == ri && i + 3 >= f.lip && i <= f.foot + 3)
                || plan.highways.iter().any(|h| h.project(*p).0 <= road_flat + 10.0)
                || plan.waters.bridges.iter().any(|b| b.centre.distance(*p) < b.length_m)
            {
                continue;
            }
            let under = plan.terrain.height_at(p.x, p.y) - r.surface_m[i];
            if under > 0.05 {
                buried.push((ri, i, under));
            }
        }
    }
    assert!(buried.is_empty(), "river points under the ground: {buried:?}");
}

#[test]
fn the_island_has_air_bases_on_flat_ground_clear_of_the_cities() {
    // Owner, 2026-10-30: four air bases (island-sites-layout; survey J5 had five). Each stands on its flattened site, clear of every
    // city, the bases spread apart, the first the nearest the largest city.
    let plan = seed_42();
    let cfg = shipped().bases;
    eprintln!(
        "{} air bases: {:?}",
        plan.bases.len(),
        plan.bases
            .iter()
            .map(|b| (b.centre.x.round(), b.centre.y.round(), b.ground_m.round()))
            .collect::<Vec<_>>()
    );
    assert_eq!(plan.bases.len(), cfg.count, "every base the data asks for stands on seed 42");
    for b in &plan.bases {
        for c in &plan.cities {
            assert!(c.rect().distance(b.centre) > cfg.city_clear_m, "base {} is by a city", b.id);
        }
        for p in [b.rect().min, b.rect().max, b.centre] {
            let h = plan.terrain.height_at(p.x, p.y);
            assert!(
                (h - b.ground_m).abs() < 1.0,
                "base {}'s site is flat: {h} against {}",
                b.id,
                b.ground_m
            );
        }
    }
    for (i, a) in plan.bases.iter().enumerate() {
        for b in &plan.bases[i + 1..] {
            assert!(
                a.centre.distance(b.centre) >= cfg.spacing_m,
                "bases {} and {} too close",
                a.id,
                b.id
            );
        }
    }
    let largest = plan.cities[0].centre;
    let nearest = plan
        .bases
        .iter()
        .min_by(|a, b| a.centre.distance(largest).total_cmp(&b.centre.distance(largest)))
        .unwrap();
    assert_eq!(nearest.id, 0, "the first base is the one nearest the start");
}

#[test]
fn nothing_grows_on_an_air_base() {
    // A base's apron is built ground: no tree is grown on it and no grass (the shared cover rule).
    let plan = seed_42();
    let b = plan.bases[0];
    for (u, v) in [(0.0, 0.0), (0.4, 0.3), (-0.4, -0.3), (0.45, -0.45)] {
        let p = b.centre + b.half_m * Vec2::new(u, v) * 2.0;
        let clear = crate::world::cover::Clearance {
            city_m: 0.0,
            park_inset_m: 0.0,
            road_m: 0.0,
            bridge_share: 0.0,
            building_m: 0.0,
        };
        assert_eq!(plan.cover_at(p, &clear), crate::world::cover::Cover::Bare, "{p}");
    }
}

#[test]
fn every_air_base_has_a_runway_of_1_5_to_2_km_flat_dry_and_clear() {
    // Survey W45, owner 2026-10-26: "Add a real runway, about 1.5 to 2 km, to each base".
    for plan in [seed_42(), &generate(42, &shipped())] {
        let cfg = shipped().bases;
        for b in &plan.bases {
            let r = b.runway;
            eprintln!("base {}: runway {:?}, {:.0} m, from {} to {}", b.id, r.side, r.length_m(), r.a, r.b);
            assert!(
                (1500.0 - 1.0..=2000.0 + 1.0).contains(&r.length_m()),
                "base {}'s runway is {} m",
                b.id,
                r.length_m()
            );
            assert!(!r.rect().overlaps(&b.rect()), "base {}'s runway is off its site", b.id);
            for s in b.structures(&cfg) {
                assert!(!r.rect().overlaps(&s), "base {}'s runway meets a hangar or the tower", b.id);
            }
            for p in r.samples(25.0) {
                let h = plan.terrain.height_at(p.x, p.y);
                assert!(
                    (h - b.ground_m).abs() < 1.0,
                    "base {}'s runway is flat at {p}: {h} against {}",
                    b.id,
                    b.ground_m
                );
                assert!(plan.water_at(p.x, p.y).is_none(), "base {}'s runway is dry at {p}", b.id);
                for hw in &plan.highways {
                    assert!(
                        hw.project(p).0 > hw.width_m * 0.5,
                        "base {}'s runway is clear of the highways at {p}",
                        b.id
                    );
                }
                for c in &plan.cities {
                    assert!(
                        c.rect().distance(p) > cfg.runway_city_clear_m,
                        "base {}'s runway is clear of the cities",
                        b.id
                    );
                }
            }
            for bld in &plan.buildings {
                assert!(!bld.footprint.overlaps(&r.rect()), "a building stands on base {}'s runway", b.id);
            }
        }
    }
}

/// Cars ride on the drawn road (car-traffic): on a city street the road's top is the city's ground plus the lift
/// the game draws it at, on a highway its grade plus the lift, and off the roads there is none.
#[test]
fn the_road_top_is_the_drawn_road_and_cars_ride_on_it() {
    let plan = seed_42();
    let lift = shipped().road_lift_m;
    let city = &plan.cities[0];
    let street = city.streets[0].rect.centre();
    assert_eq!(plan.road_top_at(street), Some(city.ground_m + lift));
    let h = &plan.highways[0];
    let mid = h.point_at(0.5);
    let top = plan.road_top_at(mid).expect("a highway's middle is road");
    assert!((top - (h.grade_at(0.5) + lift)).abs() < 0.05, "{top} vs {}", h.grade_at(0.5) + lift);
    let block = city.blocks[0].rect.centre();
    assert_eq!(plan.road_top_at(block), None, "a block is no road");
    // A car's wheels meet the road's top on the street and the bare ground off it.
    let on = OnRoads { ground: plan, plan };
    assert!((on.height_at(street.x, street.y) - (city.ground_m + lift)).abs() < 1e-4);
    assert_eq!(on.height_at(block.x, block.y), plan.height_at(block.x, block.y));
}

/// A body on foot stands on the drawn surface (owner wanted every item verified, 2026-10-26: the pilot stood 0.15 m
/// under the road and 0.2 m under the pavement): on a street the road's top, on a block's pavement the kerb's top,
/// inside a building's footprint its own floors (not the pavement), off the city the bare ground.
#[test]
fn a_body_on_foot_stands_on_the_drawn_road_and_pavement() {
    let plan = seed_42();
    let cfg = shipped();
    let city = &plan.cities[0];
    let street = city.streets[0].rect.centre();
    let dug = Deformation::default();
    let scene = Scene {
        plan,
        dug: &dug,
        trees: None,
        structures: None,
        platforms: &[],
        interiors: None,
    };
    let feet = city.ground_m + 1.0;
    assert!((scene.support_at(street.x, street.y, feet) - (city.ground_m + cfg.road_lift_m)).abs() < 1e-4);
    // a pavement point: a block's corner, inside its rect and clear of any building
    let pave = city
        .blocks
        .iter()
        .flat_map(|b| [b.rect.min + Vec2::splat(0.5), b.rect.max - Vec2::splat(0.5)])
        .find(|p| plan.buildings.iter().all(|b| b.footprint.distance(*p) > 0.5))
        .expect("a pavement corner");
    assert_eq!(plan.walk_top_at(pave), Some(city.ground_m + cfg.kerb_height_m));
    assert!((scene.support_at(pave.x, pave.y, feet) - (city.ground_m + cfg.kerb_height_m)).abs() < 1e-4);
    // inside a building's footprint the pavement does not lift the feet
    let inside = plan
        .buildings
        .iter()
        .find(|b| b.city == city.id)
        .map(|b| b.footprint.centre())
        .unwrap();
    assert_eq!(plan.walk_top_at(inside), None);
    // far from any city or road, the bare ground
    let wild = Vec2::new(19_000.0, 19_000.0);
    if plan.road_top_at(wild).is_none() {
        assert_eq!(plan.walk_top_at(wild), None);
    }
    // (roads and cities are never cratered: `Deformation::dig` protects them)
}

/// The pilot walking from a street onto a pavement stands on the road's top, then steps up onto the kerb's.
#[test]
fn the_pilot_walks_on_the_road_and_steps_up_onto_the_kerb() {
    use crate::input::Actions;
    use crate::pilot::{PilotConfig, PilotState};
    let plan = seed_42();
    let cfg = shipped();
    let city = &plan.cities[0];
    let dug = Deformation::default();
    let scene = Scene {
        plan,
        dug: &dug,
        trees: None,
        structures: None,
        platforms: &[],
        interiors: None,
    };
    let pc = PilotConfig::default();
    let street = city.streets[0].rect.centre();
    let mut p = PilotState::standing(Vec3::new(street.x, city.ground_m, street.y), 0.0);
    for _ in 0..30 {
        p.step(&Actions::default(), &pc, &scene, None, 1.0 / 60.0);
    }
    assert!(
        (p.feet.y - (city.ground_m + cfg.road_lift_m)).abs() < 1e-3,
        "on the road: {}",
        p.feet.y
    );
    let pave = city
        .blocks
        .iter()
        .flat_map(|b| [b.rect.min + Vec2::splat(0.5), b.rect.max - Vec2::splat(0.5)])
        .find(|q| plan.buildings.iter().all(|b| b.footprint.distance(*q) > 0.5))
        .expect("a pavement corner");
    let mut p = PilotState::standing(Vec3::new(pave.x, city.ground_m + cfg.road_lift_m, pave.y), 0.0);
    for _ in 0..30 {
        p.step(&Actions::default(), &pc, &scene, None, 1.0 / 60.0);
    }
    assert!(
        (p.feet.y - (city.ground_m + cfg.kerb_height_m)).abs() < 1e-3,
        "on the pavement: {}",
        p.feet.y
    );
}

/// Seed 42 as the game builds it, with the site templates `GameData::load` reads from `assets/models/sites/`.
fn with_sites() -> &'static (WorldPlan, WorldConfig) {
    static PLAN: OnceLock<(WorldPlan, WorldConfig)> = OnceLock::new();
    PLAN.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data = crate::GameData::load(&root, "mechs/placeholder.ron").expect("the shipped data loads");
        (generate(42, &data.world), data.world)
    })
}

#[test]
fn the_island_holds_two_airports_two_seaports_and_four_bases_none_overlapping() {
    // Owner, 2026-10-30 (openspec change `island-sites-layout`): "We need to add 2 civilian airports", "Add ship ports.
    // I want at least 2", three bases, then "lets add military base 2 back in".
    let (plan, cfg) = with_sites();
    let airports: Vec<_> = plan.sites.iter().filter(|s| s.template.kind == sites::SiteKind::Airport).collect();
    let ports: Vec<_> = plan.sites.iter().filter(|s| s.template.kind == sites::SiteKind::Seaport).collect();
    assert_eq!((airports.len(), ports.len(), plan.bases.len()), (2, 2, 4));
    let mut rects: Vec<(String, Rect)> = plan
        .sites
        .iter()
        .map(|s| (format!("{:?} {:?}", s.template.kind, s.origin), s.footprint()))
        .collect();
    rects.extend(plan.bases.iter().map(|b| (format!("base {}", b.id), b.rect())));
    rects.extend(plan.bases.iter().map(|b| (format!("base {} runway", b.id), b.runway.rect())));
    for (i, (a, ra)) in rects.iter().enumerate() {
        for (b, rb) in &rects[i + 1..] {
            let same_base = a.split(' ').take(2).eq(b.split(' ').take(2)) && a.starts_with("base");
            assert!(same_base || !ra.overlaps(rb), "{a} overlaps {b}");
        }
    }
    let d = airports[0].footprint().centre().distance(airports[1].footprint().centre());
    assert!(d >= cfg.sites.airport_spacing_m, "the airports are {d} m apart");
    let apart = cfg.sites.port_coast_apart_deg.to_radians().cos();
    assert!(
        ports[0].out().dot(ports[1].out()) <= apart + 1e-4,
        "the seaports face different coasts"
    );
}

#[test]
fn nothing_stands_in_a_civil_runways_approach_cones() {
    let (plan, cfg) = with_sites();
    let cfg = cfg.sites.approach;
    let airports: Vec<_> = plan.sites.iter().filter(|s| s.template.kind == sites::SiteKind::Airport).collect();
    for (i, a) in airports.iter().enumerate() {
        for cone in a.approaches(cfg) {
            for s in plan.sites.iter().filter(|s| !std::ptr::eq(*s, *a)) {
                assert!(
                    !cone.covers(&s.footprint(), 100.0),
                    "airport {i}'s cone covers a site at {:?}",
                    s.origin
                );
            }
            for b in &plan.bases {
                assert!(
                    !cone.covers(&b.rect(), 100.0) && !cone.covers(&b.runway.rect(), 100.0),
                    "airport {i}'s cone covers base {}",
                    b.id
                );
            }
            for b in &plan.buildings {
                let top = plan.cities[b.city as usize].ground_m + b.height_m;
                assert!(
                    !cone.pierced_by(&b.footprint, top, 10.0),
                    "building {} rises into airport {i}'s cone",
                    b.id
                );
            }
        }
    }
}

#[test]
fn a_road_runs_from_a_city_to_every_site_and_the_lane_graph_reaches_its_gate() {
    let (plan, cfg) = with_sites();
    let access: Vec<_> = plan.highways.iter().filter(|h| h.cities[1] == roads::SITE_END).collect();
    assert_eq!(access.len(), plan.sites.len(), "one access road a site");
    for s in &plan.sites {
        let f = s.footprint();
        assert!(
            access.iter().any(|h| f.distance(h.ends[1]) < 1.0),
            "a road ends at {:?}'s gate",
            s.origin
        );
    }
    let graph = lanes::LaneGraph::build(&plan.cities, &plan.highways, &lanes::LaneLayout::default(), cfg.arterial_width_m);
    for h in &access {
        let gate = graph.nodes.iter().find(|n| n.at.distance(h.ends[1]) < 1.0).expect("a gate node");
        assert!(
            graph.lanes.iter().any(|l| l.to_node == gate.id),
            "lanes run into the gate at {:?}",
            gate.at
        );
        assert!(graph.lanes.iter().any(|l| l.from_node == gate.id), "and out of it");
    }
}
