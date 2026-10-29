//! Harbours (openspec change `harbours`, design section 1): one by each coastal city, largest city first, up to
//! four, where the sea is berth-deep close to the shore and clear of the air bases. Nothing is dug or flattened:
//! the quay stands on piles over the plan's own ground, so placing harbours changes no other part of the world.
//! Pure: the same plan and data give the same harbours.

use super::{HarbourConfig, Sea};
use crate::world::WorldPlan;
use crate::world::layout::Rect;
use glam::Vec2;

/// A box on the ground turned to any heading: its centre, the unit direction of its first axis, and its half
/// sizes along that axis and across it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footprint {
    /// Centre (x, z).
    pub centre: Vec2,
    /// Unit direction of its first axis.
    pub along: Vec2,
    /// Half its size along `along`, then across it, metres.
    pub half: Vec2,
}

impl Footprint {
    /// `p` in the box's own frame (along, across).
    pub fn local(&self, p: Vec2) -> Vec2 {
        let d = p - self.centre;
        Vec2::new(d.dot(self.along), d.dot(self.along.perp()))
    }

    /// True when `p` is inside it.
    pub fn contains(&self, p: Vec2) -> bool {
        let l = self.local(p);
        l.x.abs() <= self.half.x && l.y.abs() <= self.half.y
    }

    /// Its four corners, counter-clockwise from the least.
    pub fn corners(&self) -> [Vec2; 4] {
        let (a, c) = (self.along * self.half.x, self.along.perp() * self.half.y);
        [self.centre - a - c, self.centre + a - c, self.centre + a + c, self.centre - a + c]
    }

    /// Its heading: the yaw that turns a model's +x axis onto `along` (Bevy's `Quat::from_rotation_y`).
    pub fn yaw(&self) -> f32 {
        (-self.along.y).atan2(self.along.x)
    }
}

/// A harbour by a coastal city.
#[derive(Clone, Debug, PartialEq)]
pub struct Harbour {
    /// Index, in placing order (the largest city's first).
    pub id: u32,
    /// The city it serves.
    pub city: u32,
    /// Unit direction out to sea from the shore.
    pub out: Vec2,
    /// Where the pier meets the shoreline (x, z).
    pub shore: Vec2,
    /// The pier head's seaward face, at its middle: where ships come alongside (x, z).
    pub berth: Vec2,
    /// Where ships turn in for the berth and start to slow (x, z).
    pub mouth: Vec2,
    /// The roads: where ships ride at anchor (x, z).
    pub anchorage: Vec2,
    /// The quay deck's height, metres.
    pub deck_y: f32,
    /// The apron along the shore.
    pub apron: Footprint,
    /// The pier out to the head.
    pub pier: Footprint,
    /// The pier head along the berth.
    pub head: Footprint,
    /// The cranes on the pier head (their portals), facing out to sea.
    pub cranes: Vec<Footprint>,
    /// The warehouses behind the apron, and the ground each is footed on, metres.
    pub warehouses: Vec<(Footprint, f32)>,
    /// The harbour office on the apron, its door facing the sea.
    pub office: Footprint,
    /// The office's terminal (x, z).
    pub terminal: Vec2,
    /// True for the seaport (openspec change `seaport-layout`): its quays, buildings and cranes are the site's, so the
    /// harbour draws none of its own and its decks are the site's solids.
    pub port: bool,
}

impl Harbour {
    /// Along the shore: the unit direction of the apron and the pier head.
    pub fn side(&self) -> Vec2 {
        self.out.perp()
    }

    /// Where a boat `beam_m` wide lies alongside the berth (x, z), its heading along the shore.
    pub fn moorage(&self, beam_m: f32) -> Vec2 {
        self.berth + self.out * (beam_m * 0.5 + 2.0)
    }

    /// The `k`th of `n` anchor spots in the roads, 300 m apart along the shore.
    pub fn anchor_spot(&self, k: usize, n: usize) -> Vec2 {
        self.anchorage + self.side() * ((k as f32 - (n as f32 - 1.0) * 0.5) * 300.0)
    }

    /// The walkable tops of the quay: the apron, the pier and the head, all at the deck's height (none for the
    /// seaport, whose quays are its site's solids).
    pub fn decks(&self) -> Vec<Footprint> {
        if self.port {
            return Vec::new();
        }
        vec![self.apron, self.pier, self.head]
    }

    /// The harbour the seaport is (openspec change `seaport-layout` section 3b): its berth, mouth and terminal the
    /// site's marks, its office the site's harbour office, its deck the site's quay deck; it draws nothing of its own.
    pub fn of_port(
        site: &crate::world::sites::PlacedSite,
        kits: &crate::world::small_buildings::Kits,
        cfg: &HarbourConfig,
    ) -> Option<Self> {
        let out = site.out();
        let side = out.perp();
        let (berth, mouth, terminal) = (site.mark("berth")?, site.mark("mouth")?, site.mark("terminal")?);
        let office = site
            .template
            .placements
            .iter()
            .find(|p| kits.kits[p.kit].id == "harbour_office")
            .map(|p| {
                let k = &kits.kits[p.kit];
                let c = Vec2::new((k.lo.x + k.hi.x) * 0.5, (k.lo.z + k.hi.z) * 0.5);
                Footprint {
                    centre: site.to_world(p.at + c),
                    along: side,
                    half: Vec2::new((k.hi.x - k.lo.x) * 0.5, (k.hi.z - k.lo.z) * 0.5),
                }
            })?;
        let none = Footprint {
            centre: site.origin,
            along: side,
            half: Vec2::ZERO,
        };
        Some(Self {
            id: 0,
            city: site.city,
            out,
            shore: site.origin,
            berth,
            mouth,
            anchorage: mouth + out * cfg.anchorage_m,
            deck_y: site.ground_m + cfg.deck_m,
            apron: none,
            pier: none,
            head: none,
            cranes: Vec::new(),
            warehouses: Vec::new(),
            office,
            terminal,
            port: true,
        })
    }
}

/// What harbours keep clear of and where they may go.
pub struct Sites<'a> {
    /// Each city's id and square, largest first.
    pub cities: Vec<(u32, Rect)>,
    /// Sites a harbour's shore keeps `base_clear_m` from (the air bases).
    pub bases: Vec<Rect>,
    /// The distance from a point to the nearest highway's centre line, metres.
    pub road_dist: &'a dyn Fn(Vec2) -> f32,
    /// What is built keeps this far from a highway's centre line, metres.
    pub road_m: f32,
    /// The seaports' harbours (`Harbour::of_port`): placed first, and their cities get no other.
    pub ports: Vec<Harbour>,
}

/// The seaports of `plan` as harbours.
pub fn ports_of(plan: &WorldPlan, cfg: &HarbourConfig) -> Vec<Harbour> {
    plan.sites
        .iter()
        .filter(|s| s.template.kind == crate::world::sites::SiteKind::Seaport)
        .filter_map(|s| Harbour::of_port(s, &plan.small.kits, cfg))
        .collect()
}

/// Bearings tried round a city.
const BEARINGS: usize = 48;
/// The step walked out along a bearing, metres.
const STEP_M: f32 = 20.0;
/// What is built keeps this far from a city's square, metres.
const CITY_CLEAR_M: f32 = 20.0;

/// The harbours of `plan`: `road_m` is how far from a highway's centre line its graded band reaches.
pub fn place_in(plan: &WorldPlan, cfg: &HarbourConfig, road_m: f32) -> Vec<Harbour> {
    let road_dist = |p: Vec2| plan.highways.iter().map(|h| h.project(p).0).fold(f32::INFINITY, f32::min);
    let sites = Sites {
        cities: plan.cities.iter().map(|c| (c.id, c.rect())).collect(),
        bases: plan.bases.iter().map(|b| b.rect()).collect(),
        road_dist: &road_dist,
        road_m,
        ports: ports_of(plan, cfg),
    };
    let sea = Sea {
        ground: plan,
        sea_level_m: plan.waters.sea_level_m,
    };
    place(&sites, &sea, cfg)
}

/// Places the harbours: each city in order, the best berth bearing round it, kept `spacing_m` from the harbours
/// already placed, until `most` stand.
pub fn place(sites: &Sites, sea: &Sea, cfg: &HarbourConfig) -> Vec<Harbour> {
    let mut out: Vec<Harbour> = Vec::new();
    for p in &sites.ports {
        if out.len() < cfg.most {
            out.push(Harbour {
                id: out.len() as u32,
                ..p.clone()
            });
        }
    }
    for &(city, rect) in &sites.cities {
        if out.len() >= cfg.most {
            break;
        }
        if sites.ports.iter().any(|p| p.city == city) {
            continue;
        }
        let Some(h) = best_bearing(city, &rect, sites, sea, cfg) else {
            continue;
        };
        if out.iter().any(|o| o.berth.distance(h.berth) < cfg.spacing_m) {
            continue;
        }
        out.push(Harbour { id: out.len() as u32, ..h });
    }
    out
}

/// The harbour round one city with the shortest pier, if any bearing has berth-deep water close enough.
fn best_bearing(city: u32, rect: &Rect, sites: &Sites, sea: &Sea, cfg: &HarbourConfig) -> Option<Harbour> {
    let centre = rect.centre();
    let reach = rect.size().max_element() * 0.75 + cfg.coast_reach_m;
    let mut best: Option<(f32, Harbour)> = None;
    for i in 0..BEARINGS {
        let a = i as f32 / BEARINGS as f32 * std::f32::consts::TAU;
        let out = Vec2::new(a.cos(), a.sin());
        let Some((shore, pier_m)) = walk(centre, out, reach, sea, cfg) else {
            continue;
        };
        if rect.distance(shore) > cfg.coast_reach_m {
            continue;
        }
        let h = lay_out(city, shore, out, pier_m, sea, cfg);
        if !fits(&h, rect, sites, sea, cfg) {
            continue;
        }
        if best.as_ref().is_none_or(|b| pier_m < b.0) {
            best = Some((pier_m, h));
        }
    }
    best.map(|b| b.1)
}

/// Walks out from `from` along `out`: the shoreline, and how far past it the water is berth-deep (within
/// `pier_max_m`), if both are within `reach`.
fn walk(from: Vec2, out: Vec2, reach: f32, sea: &Sea, cfg: &HarbourConfig) -> Option<(Vec2, f32)> {
    let mut d = 0.0;
    let mut shore = None;
    while d <= reach {
        let p = from + out * d;
        let depth = sea.sea_depth(p);
        match shore {
            None if depth > 0.0 => {
                // The crossing, to a metre: bisect the last step.
                let (mut lo, mut hi) = ((d - STEP_M).max(0.0), d);
                for _ in 0..5 {
                    let mid = (lo + hi) * 0.5;
                    if sea.sea_depth(from + out * mid) > 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                shore = Some(hi);
            }
            Some(s) if depth >= cfg.berth_depth_m => return Some((from + out * s, d - s)),
            Some(s) if d - s > cfg.pier_max_m => return None,
            // Back on land past the shore (a spit or an islet): not a harbour's water.
            Some(_) if depth <= 0.0 => return None,
            _ => {}
        }
        d += STEP_M;
    }
    None
}

/// Lays a harbour out from its shore point and seaward direction (design section 1).
fn lay_out(city: u32, shore: Vec2, out: Vec2, pier_m: f32, sea: &Sea, cfg: &HarbourConfig) -> Harbour {
    let side = out.perp();
    let at = |a: f32, o: f32| shore + side * a + out * o;
    let half_len = cfg.quay_len_m * 0.5;
    let pier_m = pier_m.max(cfg.head_w_m);
    let apron = Footprint {
        centre: at(0.0, (10.0 - cfg.apron_m) * 0.5),
        along: side,
        half: Vec2::new(half_len, (cfg.apron_m + 10.0) * 0.5),
    };
    let pier = Footprint {
        centre: at(0.0, pier_m * 0.5),
        along: side,
        half: Vec2::new(cfg.pier_w_m * 0.5, pier_m * 0.5),
    };
    let head = Footprint {
        centre: at(0.0, pier_m - cfg.head_w_m * 0.5),
        along: side,
        half: Vec2::new(half_len, cfg.head_w_m * 0.5),
    };
    let berth = at(0.0, pier_m);
    let n = cfg.cranes.max(1) as f32;
    let cranes = (0..cfg.cranes)
        .map(|i| Footprint {
            centre: at(-half_len + (i as f32 + 0.5) * cfg.quay_len_m / n, pier_m - cfg.head_w_m * 0.5),
            along: side,
            half: Vec2::splat(cfg.crane_m[0] * 0.5),
        })
        .collect();
    let [ww, _, wd] = cfg.warehouse_m;
    let warehouses = (0..cfg.warehouses)
        .map(|i| {
            let f = Footprint {
                centre: at(
                    (i as f32 - (cfg.warehouses as f32 - 1.0) * 0.5) * (ww + 12.0),
                    -cfg.apron_m - 8.0 - wd * 0.5,
                ),
                along: side,
                half: Vec2::new(ww * 0.5, wd * 0.5),
            };
            let foot = f
                .corners()
                .iter()
                .chain(std::iter::once(&f.centre))
                .map(|p| ground(sea, *p))
                .fold(f32::INFINITY, f32::min);
            (f, foot)
        })
        .collect();
    let [ow, _, od] = cfg.office_m;
    let office = Footprint {
        centre: at(-half_len + ow * 0.5 + 4.0, -cfg.apron_m + od * 0.5 + 2.0),
        along: side,
        half: Vec2::new(ow * 0.5, od * 0.5),
    };
    let terminal = office.centre + out * (od * 0.5 + cfg.terminal_out_m);
    // The deck stands its least height over the sea, or clear of the highest ground under the apron.
    let highest = apron
        .corners()
        .iter()
        .chain(std::iter::once(&apron.centre))
        .map(|p| ground(sea, *p))
        .fold(f32::MIN, f32::max);
    let deck_y = (sea.sea_level_m + cfg.deck_m).max(highest + 0.3);
    Harbour {
        id: 0,
        city,
        out,
        shore,
        berth,
        mouth: berth + out * cfg.approach_m,
        anchorage: berth + out * cfg.anchorage_m,
        deck_y,
        apron,
        pier,
        head,
        cranes,
        warehouses,
        office,
        terminal,
        port: false,
    }
}

fn ground(sea: &Sea, p: Vec2) -> f32 {
    sea.ground.height_at(p.x, p.y)
}

/// True when the harbour's buildings keep clear of its city's square, the bases and the highways, the pier
/// head's ends have water under them, and its mouth and roads are at sea.
fn fits(h: &Harbour, city: &Rect, sites: &Sites, sea: &Sea, cfg: &HarbourConfig) -> bool {
    if h.deck_y - sea.sea_level_m > cfg.max_deck_m {
        return false;
    }
    let built: Vec<Footprint> = std::iter::once(h.apron)
        .chain(std::iter::once(h.office))
        .chain(h.warehouses.iter().map(|w| w.0))
        .collect();
    let points: Vec<Vec2> = built
        .iter()
        .flat_map(|f| f.corners().into_iter().chain(std::iter::once(f.centre)))
        .collect();
    if points.iter().any(|p| city.distance(*p) < CITY_CLEAR_M) {
        return false;
    }
    if sites.bases.iter().any(|b| b.distance(h.shore) < cfg.base_clear_m) {
        return false;
    }
    if points.iter().any(|p| (sites.road_dist)(*p) < sites.road_m) {
        return false;
    }
    let side = h.side() * (cfg.quay_len_m * 0.5);
    let deep_ends = [h.berth + side, h.berth - side]
        .iter()
        .all(|p| sea.sea_depth(*p) >= cfg.berth_depth_m * 0.7);
    let at_sea = [h.mouth, h.anchorage].iter().all(|p| sea.sea_depth(*p) >= cfg.berth_depth_m);
    deep_ends && at_sea
}

/// The harbour whose office terminal a pilot on foot at `feet`, looking along `yaw_rad`, can use: within
/// `reach_m` and within `look_deg` of straight ahead (the air bases' terminal rule, `world::bases`).
pub fn office_in_reach(harbours: &[Harbour], feet: Vec2, yaw_rad: f32, reach_m: f32, look_deg: f32) -> Option<u32> {
    let look = super::ahead(yaw_rad);
    let cone = look_deg.to_radians().cos();
    harbours
        .iter()
        .filter(|h| {
            let to = h.terminal - feet;
            let d = to.length();
            d <= reach_m && (d < 0.3 || to.dot(look) / d >= cone)
        })
        .min_by(|a, b| a.terminal.distance(feet).total_cmp(&b.terminal.distance(feet)))
        .map(|h| h.id)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::test_world;
    use super::*;
    use std::sync::OnceLock;

    fn island_sites() -> Sites<'static> {
        // A city 500 m across whose square's edge is 2.5 km from the centre, toward +x; and one inland.
        Sites {
            cities: vec![
                (0, Rect::centred(Vec2::new(2700.0, 0.0), Vec2::splat(250.0))),
                (1, Rect::centred(Vec2::new(-200.0, 0.0), Vec2::splat(250.0))),
            ],
            bases: vec![],
            road_dist: &|_| f32::INFINITY,
            road_m: 0.0,
            ports: Vec::new(),
        }
    }

    #[test]
    fn a_coastal_city_gets_a_harbour_and_an_inland_one_does_not() {
        let cfg = test_world::shipped().harbours;
        let hs = place(&island_sites(), &test_world::sea(), &cfg);
        assert_eq!(hs.len(), 1, "{hs:?}");
        let h = &hs[0];
        assert_eq!(h.city, 0);
        assert!(h.out.x > 0.9, "it faces out to sea: {:?}", h.out);
        let sea = test_world::sea();
        assert!(sea.sea_depth(h.berth) >= cfg.berth_depth_m - 0.01, "the berth is berth-deep");
        assert!(
            sea.sea_depth(h.shore) > 0.0 && sea.sea_depth(h.shore) < 1.0,
            "the shore is the shoreline"
        );
        assert!(h.berth.distance(h.shore) <= cfg.pier_max_m);
        assert!(h.deck_y >= cfg.deck_m, "the deck stands over the sea");
    }

    #[test]
    fn a_harbour_keeps_clear_of_an_air_base() {
        let cfg = test_world::shipped().harbours;
        let mut sites = island_sites();
        // A base on the coast straight out from the city: the harbour goes round it or not at all.
        let base = Rect::centred(Vec2::new(3400.0, 0.0), Vec2::new(350.0, 250.0));
        sites.bases.push(base);
        for h in place(&sites, &test_world::sea(), &cfg) {
            assert!(base.distance(h.shore) >= cfg.base_clear_m, "{:?}", h.shore);
        }
    }

    #[test]
    fn its_buildings_stand_clear_of_the_city_square() {
        let cfg = test_world::shipped().harbours;
        let sites = island_sites();
        let h = &place(&sites, &test_world::sea(), &cfg)[0];
        let city = sites.cities[0].1;
        for f in std::iter::once(h.office).chain(h.warehouses.iter().map(|w| w.0)) {
            for c in f.corners() {
                assert!(city.distance(c) >= CITY_CLEAR_M, "{c:?}");
            }
        }
    }

    #[test]
    fn the_terminal_is_found_by_the_air_bases_rule() {
        let cfg = test_world::shipped().harbours;
        let hs = place(&island_sites(), &test_world::sea(), &cfg);
        let h = &hs[0];
        // Standing 2 m short of it, facing it.
        let feet = h.terminal - h.out * 2.0;
        let yaw = super::super::heading_of(h.out);
        assert_eq!(office_in_reach(&hs, feet, yaw, 3.0, 45.0), Some(h.id));
        assert_eq!(
            office_in_reach(&hs, feet, yaw + std::f32::consts::PI, 3.0, 45.0),
            None,
            "facing away"
        );
        assert_eq!(office_in_reach(&hs, feet - h.out * 5.0, yaw, 3.0, 45.0), None, "too far");
    }

    #[test]
    fn a_footprint_knows_its_inside_and_its_yaw() {
        let f = Footprint {
            centre: Vec2::new(10.0, 0.0),
            along: Vec2::new(0.0, 1.0),
            half: Vec2::new(5.0, 1.0),
        };
        assert!(f.contains(Vec2::new(10.5, 4.0)));
        assert!(!f.contains(Vec2::new(12.0, 0.0)));
        // A model's +x turned onto +z.
        let q = glam::Quat::from_rotation_y(f.yaw());
        let x = q * glam::Vec3::X;
        assert!((x - glam::Vec3::Z).length() < 1e-5, "{x:?}");
    }

    pub(crate) fn seed_42() -> &'static (WorldPlan, Vec<Harbour>) {
        static P: OnceLock<(WorldPlan, Vec<Harbour>)> = OnceLock::new();
        P.get_or_init(|| {
            let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
            let world: crate::world::WorldConfig = crate::data::load_ron(&root.join("world/world.ron")).unwrap();
            let plan = crate::world::generate(42, &world);
            let road_m = world.highway_width_m * 0.5 + world.highway_flat_margin_m;
            let hs = place_in(&plan, &test_world::shipped().harbours, road_m);
            (plan, hs)
        })
    }

    #[test]
    fn seed_42_has_harbours_by_its_coastal_cities_clear_of_the_bases() {
        let (plan, hs) = seed_42();
        let cfg = test_world::shipped().harbours;
        assert!(!hs.is_empty() && hs.len() <= cfg.most, "{} harbours", hs.len());
        for h in hs {
            let city = &plan.cities[h.city as usize];
            assert!(
                city.rect().distance(h.shore) <= cfg.coast_reach_m,
                "harbour {} by city {}",
                h.id,
                h.city
            );
            for b in &plan.bases {
                assert!(b.rect().distance(h.shore) >= cfg.base_clear_m, "harbour {} by base {}", h.id, b.id);
            }
            assert!(plan.waters.sea_level_m - plan.terrain.height_at(h.berth.x, h.berth.y) >= cfg.berth_depth_m - 0.01);
        }
        for (i, a) in hs.iter().enumerate() {
            for b in &hs[i + 1..] {
                assert!(a.berth.distance(b.berth) >= cfg.spacing_m);
            }
        }
    }
}
