//! Houses and small businesses (openspec change `small-buildings`): the exported kits (`kit`), where
//! they stand (`lots`), walking into them (`walk`) and breaking them (`damage`). The kits are the
//! assets the Main Street mockup draws (`docs/mockups/small_buildings_kit.js`), so the game, the
//! mockup and the exported files cannot disagree; this module is their rules, with no engine in it.

pub mod damage;
pub mod kit;
pub mod lots;
pub mod walk;

pub use damage::{Broken, HOUSE_CHUNK_BASE, Houses};
pub use kit::{Kit, Kits};
pub use lots::{Lot, Lots, SmallBuildingsConfig, Zone};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Ground as _, WorldPlan, generate};
    use std::sync::OnceLock;

    fn island() -> &'static WorldPlan {
        static PLAN: OnceLock<WorldPlan> = OnceLock::new();
        PLAN.get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            let data = crate::GameData::load(&root, "mechs/placeholder.ron").expect("the shipped data loads");
            let t = std::time::Instant::now();
            let plan = generate(42, &data.world);
            eprintln!(
                "generate with lots: {:.2} s, {} lots",
                t.elapsed().as_secs_f32(),
                plan.small.lots.len()
            );
            plan
        })
    }

    #[test]
    fn the_island_has_suburbs_towns_and_roadside_stops() {
        let plan = island();
        let count = |z: Zone| plan.small.lots.iter().filter(|l| l.zone == z).count();
        let (s, t, r) = (count(Zone::Suburb), count(Zone::Town), count(Zone::Roadside));
        eprintln!("suburb {s}, town {t}, roadside {r}");
        assert!(s > 200, "suburbs round the cities: {s}");
        assert!(t >= 40, "towns on the highways: {t}");
        assert!(r >= 3, "roadside stops: {r}");
    }

    #[test]
    fn lots_keep_clear_of_water_roads_and_each_other() {
        let plan = island();
        let lots = &plan.small.lots;
        for (i, a) in lots.iter().enumerate() {
            for j in plan.small.candidates(a.rect.centre(), a.rect.size().max_element()) {
                if j as usize != i {
                    assert!(!a.rect.overlaps(&lots[j as usize].rect), "lots {i} and {j} overlap");
                }
            }
            assert!(plan.water_at(a.centre.x, a.centre.y).is_none(), "lot {i} stands in water");
            for h in &plan.highways {
                assert!(h.project(a.centre).0 > h.width_m * 0.5, "lot {i} on a highway");
            }
            assert!(plan.cities.iter().all(|c| !c.rect().overlaps(&a.rect)), "lot {i} in a city square");
        }
    }

    #[test]
    fn every_lot_is_flat() {
        let plan = island();
        for (i, l) in plan.small.lots.iter().enumerate() {
            let kit = plan.small.kit(i as u32);
            for (x, z) in [
                (kit.lo.x, kit.lo.z),
                (kit.hi.x, kit.lo.z),
                (kit.hi.x, kit.hi.z),
                (kit.lo.x, kit.hi.z),
                (0.0, 0.0),
            ] {
                let p = l.to_world(glam::Vec2::new(x, z));
                let h = plan.height_at(p.x, p.y);
                // Never below the foundation's foot (0.3 m down), never up to the ground floor (0.45 m up).
                let d = h - l.ground_m;
                let near = plan
                    .small
                    .candidates(p, 20.0)
                    .into_iter()
                    .filter(|&j| j as usize != i)
                    .map(|j| {
                        (
                            j,
                            plan.small.lots[j as usize].rect.distance(p),
                            plan.small.lots[j as usize].ground_m,
                        )
                    })
                    .collect::<Vec<_>>();
                let city = plan.cities.iter().map(|c| c.rect().distance(p)).fold(f32::MAX, f32::min);
                assert!(
                    d > -0.3 && d < 0.45,
                    "lot {i} ({}) at {p}: ground {h} under a corner, pad {}; near {near:?}; city {city}",
                    kit.id,
                    l.ground_m
                );
            }
        }
    }

    #[test]
    #[ignore = "a timing instrument"]
    fn time_generate_without_lots() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let cfg: crate::world::WorldConfig = crate::data::load_ron(&root.join("world/world.ron")).unwrap();
        let t = std::time::Instant::now();
        let _ = generate(42, &cfg);
        eprintln!("generate without lots: {:.2} s", t.elapsed().as_secs_f32());
    }

    #[test]
    fn the_same_seed_places_the_same_lots() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data = crate::GameData::load(&root, "mechs/placeholder.ron").unwrap();
        let again = generate(42, &data.world);
        assert_eq!(again.small.lots, island().small.lots);
    }
}
