//! Houses and small businesses (openspec change `small-buildings`): the exported kits (`kit`), where
//! they stand (`lots`), walking into them (`walk`) and breaking them (`damage`). The kits are the
//! assets the Main Street mockup draws (`docs/mockups/small_buildings_kit.js`), so the game, the
//! mockup and the exported files cannot disagree; this module is their rules, with no engine in it.

pub mod damage;
pub mod kit;
pub mod lots;
pub mod paving;
pub mod walk;

pub use damage::{Breakable, Broken, HOUSE_CHUNK_BASE, HouseState, Houses, Sinking, lot_pose};
pub use kit::{Kit, Kits};
pub use lots::{Lot, Lots, Placed, SmallBuildingsConfig, Zone};
pub use paving::Paving;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Ground as _, WorldPlan, generate};
    use glam::Vec2;
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
        // A site's buildings stand on its own ground, beside each other (a jet bridge at its terminal) and over its
        // water (a jetty, a ship): `the_island_has_an_airport_and_a_seaport` checks them.
        for (i, a) in lots.iter().enumerate().filter(|(_, l)| !l.zone.is_site()) {
            for j in plan.small.candidates(a.rect.centre(), a.rect.size().max_element()) {
                if j as usize != i && !lots[j as usize].zone.is_site() {
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
        for (i, l) in plan.small.lots.iter().enumerate().filter(|(_, l)| !l.zone.is_site()) {
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
                    .filter(|&j| j as usize != i && !plan.small.lots[j as usize].zone.is_site())
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
    fn the_island_has_an_airport_and_a_seaport() {
        let plan = island();
        let kinds: Vec<_> = plan.sites.iter().map(|s| s.template.kind).collect();
        eprintln!(
            "sites: {:?}",
            plan.sites
                .iter()
                .map(|s| (s.template.kind, s.origin, s.turn, s.ground_m, s.city))
                .collect::<Vec<_>>()
        );
        assert!(kinds.contains(&crate::world::sites::SiteKind::Airport), "an airport: {kinds:?}");
        assert!(kinds.contains(&crate::world::sites::SiteKind::Seaport), "a seaport: {kinds:?}");
        for s in &plan.sites {
            // Every building of the site is a lot of its zone.
            let zone = match s.template.kind {
                crate::world::sites::SiteKind::Airport => Zone::Airport,
                crate::world::sites::SiteKind::Seaport => Zone::Seaport,
                crate::world::sites::SiteKind::MilitaryBase => Zone::MilitaryBase,
            };
            // every site of its kind (two airports, two seaports: island-sites-layout), each building but a port's
            // ships (they sail: ship-traffic)
            let n = plan.small.lots.iter().filter(|l| l.zone == zone).count();
            let built: usize = plan
                .sites
                .iter()
                .filter(|o| o.template.kind == s.template.kind)
                .map(|o| o.template.placements.iter().filter(|p| !p.ship).count())
                .sum();
            assert_eq!(n, built, "{:?}'s lots", s.template.kind);
            // The ground is shaped to every pad: flat at its height a cell inside its edges.
            for (r, y) in s.pads() {
                let m = Vec2::splat(30.0);
                let inner = crate::world::layout::Rect {
                    min: r.min + m,
                    max: r.max - m,
                };
                for p in [inner.min, inner.max, inner.centre(), Vec2::new(inner.min.x, inner.max.y)] {
                    let h = plan.height_at(p.x, p.y);
                    assert!((h - y).abs() < 0.3, "{:?} pad at {p}: ground {h}, pad {y}", s.template.kind);
                }
            }
            // A quay meets the land with no water between: just inside its landward edge, and just on the land
            // beside it, the ground stands above the sea (it fell to the basin's depth across that terrain cell).
            for (lo, hi) in s
                .template
                .ground
                .solids
                .iter()
                .filter(|(lo, hi)| lo.y < 0.0 && hi.y > 1.0 && lo.z.abs() < 1.0 && (hi.x - lo.x) * (hi.z - lo.z) >= 100.0)
            {
                // A terrain cell (8 m) in from its side walls: at the corners the ground slopes into the basin beside it.
                for x in [lo.x + 8.0, (lo.x + hi.x) * 0.5, hi.x - 8.0] {
                    for z in [lo.z - 2.0, lo.z + 2.0, lo.z + 6.0] {
                        let p = s.to_world(Vec2::new(x, z));
                        let h = plan.height_at(p.x, p.y);
                        assert!(h > s.ground_m, "the ground at a quay's landward edge, {p}: {h} under the sea");
                    }
                }
            }
            // No house stands on a site.
            let foot = s.footprint();
            for l in plan.small.lots.iter().filter(|l| !l.zone.is_site()) {
                assert!(!l.rect.overlaps(&foot), "a house on the {:?}", s.template.kind);
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

    /// The paving a house's drive or a business's curb cut ends on: a suburb street, a connector or a highway.
    fn on_a_road(plan: &WorldPlan, p: Vec2) -> bool {
        plan.small
            .paving
            .streets
            .iter()
            .any(|s| paving::path_distance(&s.path, p) <= s.width_m * 0.5)
            || plan.highways.iter().any(|h| h.project(p).0 <= h.width_m * 0.5)
    }

    #[test]
    fn every_house_has_a_drive_to_a_street_or_a_highway() {
        let plan = island();
        let cfg = &crate::world::WorldConfig::default().small_buildings;
        let mut houses = 0;
        for (i, l) in plan.small.lots.iter().enumerate() {
            let k = plan.small.kit(i as u32);
            let kit::Access::Drive { x_m, .. } = k.access else { continue };
            houses += 1;
            let start = l.to_world(Vec2::new(x_m, k.hi.z));
            let dir = (l.to_world(Vec2::new(x_m, k.hi.z + 1.0)) - start).normalize();
            // Paved all the way from the kit's slab until it is on a road.
            let mut t = 0.1;
            while !on_a_road(plan, start + dir * t) {
                assert!(
                    plan.small.paving.paved(start + dir * t, 0.0),
                    "lot {i} ({}): no drive {t} m out",
                    k.id
                );
                t += 0.25;
                assert!(t <= cfg.drive_reach_m + 0.5, "lot {i} ({}): the drive never reaches a road", k.id);
            }
        }
        eprintln!("{houses} houses, each with a drive to a road");
        assert!(houses > 200);
    }

    #[test]
    fn every_suburb_street_joins_a_city_or_a_highway() {
        let plan = island();
        let streets = &plan.small.paving.streets;
        let touches = |a: &paving::Street, b: &paving::Street| {
            a.path
                .iter()
                .any(|&p| paving::path_distance(&b.path, p) <= (a.width_m + b.width_m) * 0.5)
                || b.path
                    .iter()
                    .any(|&p| paving::path_distance(&a.path, p) <= (a.width_m + b.width_m) * 0.5)
        };
        let rooted = |s: &paving::Street| {
            s.path.iter().any(|&p| {
                plan.cities.iter().any(|c| c.rect().distance(p) <= 0.0) || plan.highways.iter().any(|h| h.project(p).0 <= h.width_m * 0.5)
            })
        };
        let mut joined: Vec<bool> = streets.iter().map(rooted).collect();
        loop {
            let mut grew = false;
            for i in 0..streets.len() {
                if !joined[i] && (0..streets.len()).any(|j| joined[j] && touches(&streets[i], &streets[j])) {
                    joined[i] = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        let alone: Vec<usize> = (0..streets.len()).filter(|&i| !joined[i]).collect();
        eprintln!(
            "{} streets, {} km",
            streets.len(),
            streets
                .iter()
                .map(|s| s.path.windows(2).map(|w| w[0].distance(w[1])).sum::<f32>())
                .sum::<f32>()
                / 1000.0
        );
        assert!(streets.len() > 20);
        assert!(alone.is_empty(), "streets {alone:?} join nothing");
    }

    #[test]
    fn every_business_has_a_car_park_with_a_curb_cut_onto_its_highway() {
        let plan = island();
        let cfg = &crate::world::WorldConfig::default().small_buildings;
        let mut businesses = 0;
        for (i, l) in plan.small.lots.iter().enumerate() {
            let k = plan.small.kit(i as u32);
            let kit::Access::Lot { .. } = k.access else { continue };
            businesses += 1;
            let mid = (k.lo.x + k.hi.x) * 0.5;
            assert!(
                plan.small
                    .paving
                    .paved(l.to_world(Vec2::new(mid, k.hi.z + cfg.parking_depth_m * 0.5)), 0.0),
                "lot {i} ({}) has no car park",
                k.id
            );
            let cut = plan.small.paving.slabs.iter().any(|s| {
                s.surface == paving::Surface::Asphalt
                    && s.corners.iter().any(|&c| l.to_local(c).y > k.hi.z + cfg.parking_depth_m - 0.1)
                    && on_a_road(plan, (s.corners[2] + s.corners[3]) * 0.5)
                    && l.rect.distance(s.corners[0]) < cfg.parking_depth_m + 2.0
            });
            assert!(cut, "lot {i} ({}): no curb cut reaches the road", k.id);
        }
        assert!(businesses > 20, "{businesses} businesses");
    }

    #[test]
    fn the_same_seed_places_the_same_lots() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data = crate::GameData::load(&root, "mechs/placeholder.ron").unwrap();
        let again = generate(42, &data.world);
        assert_eq!(again.small.lots, island().small.lots);
        assert_eq!(again.small.paving, island().small.paving);
    }
}
