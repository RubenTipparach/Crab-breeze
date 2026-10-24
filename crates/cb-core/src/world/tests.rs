//! Tests for the world generator, against the shipped `data/world/world.ron`.

use super::*;
use crate::data::load_ron;
use crate::world::Ground;
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
    PLAN.get_or_init(|| generate(42, &shipped()))
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
        plan.forest.trees.len(),
        plan.fingerprint(),
    );
    assert_eq!(
        summary,
        (GOLDEN_CITIES, GOLDEN_BUILDINGS, GOLDEN_HIGHWAYS, GOLDEN_TREES, GOLDEN_FINGERPRINT),
        "the seed-42 world changed; if that was intended, update the golden values"
    );
}

// Pinned 2026-10-24 (with water and trees) on the owner's Mac (aarch64). The generator uses only IEEE arithmetic plus
// `powf` for storeys; if another platform disagrees, the difference is in that one call.
const GOLDEN_CITIES: usize = 4;
const GOLDEN_BUILDINGS: usize = 1226;
const GOLDEN_HIGHWAYS: usize = 3;
const GOLDEN_TREES: usize = 41571;
const GOLDEN_FINGERPRINT: u64 = 13446694087906155705;

#[test]
fn cities_keep_their_minimum_spacing() {
    let cfg = shipped();
    let cities = &seed_42().cities;
    assert!(cities.len() >= 3, "seed 42 places at least three cities");
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
        let steps = (h.length_m() / 2.0) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            for side in [-0.5, -0.25, 0.0, 0.25, 0.5] {
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
fn seed_42_has_rivers_and_lakes() {
    let w = &seed_42().waters;
    assert!(!w.rivers.is_empty(), "rivers");
    let wet = seed_42().terrain.heights_m.iter().filter(|h| **h < w.sea_level_m).count();
    assert!(wet > 0, "some ground lies under the lakes");
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
    for r in &plan.waters.rivers {
        for p in &r.path {
            for c in &plan.cities {
                assert!(
                    c.rect().distance(*p) >= plan_cfg().water.river_city_clearance_m - plan_cfg().water.river_grid_m,
                    "river at {p} near city {}",
                    c.id
                );
            }
        }
    }
}

fn plan_cfg() -> WorldConfig {
    shipped()
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
fn seed_42_grows_woods_clear_of_roads_water_and_buildings() {
    let plan = seed_42();
    let f = &plan.forest;
    assert!(f.trees.len() > 5000, "woods: {} trees", f.trees.len());
    for t in f.trees.iter().step_by(7) {
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
