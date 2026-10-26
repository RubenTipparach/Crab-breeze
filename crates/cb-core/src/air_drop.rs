//! The air drop (openspec change `mech-shop`, design section 6; owner, 2026-10-26: "when a player orders a mech it
//! should be air dropped close to wherever the player is on a good are on the ground (not stuck on a building)"):
//! where a bought mech lands near the pilot (`drop_site`), on ground good enough to take it (`good_spot`). Pure: the
//! world is read through `DropGround`, which the generated `WorldPlan` answers, so the rules are tested on ground made
//! up for the test as well as on the island (`PlanGround`).

use crate::data::{Validate, positive};
use crate::mech::MechConfig;
use crate::rng::SimRng;
use crate::world::WorldPlan;
use crate::world::bases::BasesConfig;
use crate::world::cover::Surface;
use glam::{Vec2, Vec3};
use serde::Deserialize;

/// How a drop picks its site and comes down, `data/career/air_drop.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DropConfig {
    /// The nearest a mech lands to the pilot, metres: far enough that its fall and dust miss them.
    pub ring_min_m: f32,
    /// The farthest of the ring searched first, metres: near enough to walk to.
    pub ring_max_m: f32,
    /// The search goes on past the ring out to this, metres; with nothing good by then the mech goes to a bay.
    pub search_max_m: f32,
    /// Rings are searched this far apart, metres.
    pub ring_step_m: f32,
    /// Spots on a ring are about this far apart, metres.
    pub spot_spacing_m: f32,
    /// Ground kept clear between the mech's footprint and any building, hangar, tower or rock, metres.
    pub building_clear_m: f32,
    /// Ground kept clear between the mech's footprint and another mech's, metres.
    pub mech_clear_m: f32,
    /// The steepest ground a mech lands on, degrees.
    pub max_slope_deg: f32,
    /// A paved spot (a road, a city street or plaza, a base's apron) counts this much nearer, metres.
    pub paved_bonus_m: f32,
    /// A spot in water (no deeper than the mech's skim depth) counts this much farther, metres.
    pub wet_penalty_m: f32,
}

impl Default for DropConfig {
    fn default() -> Self {
        Self {
            ring_min_m: 40.0,
            ring_max_m: 150.0,
            search_max_m: 600.0,
            ring_step_m: 10.0,
            spot_spacing_m: 15.0,
            building_clear_m: 6.0,
            mech_clear_m: 10.0,
            max_slope_deg: 12.0,
            paved_bonus_m: 30.0,
            wet_penalty_m: 300.0,
        }
    }
}

impl Validate for DropConfig {
    fn validate(&self) -> Result<(), String> {
        positive("ring_min_m", self.ring_min_m)?;
        positive("ring_step_m", self.ring_step_m)?;
        positive("spot_spacing_m", self.spot_spacing_m)?;
        positive("max_slope_deg", self.max_slope_deg)?;
        if self.ring_max_m < self.ring_min_m || self.search_max_m < self.ring_max_m {
            return Err("ring_min_m, ring_max_m, search_max_m: each at least the one before".into());
        }
        if self.search_max_m / self.ring_step_m > 1000.0 {
            return Err("search_max_m / ring_step_m: more than 1000 rings".into());
        }
        crate::data::within("max_slope_deg", self.max_slope_deg, 0.0, 60.0)?;
        for (field, v) in [
            ("building_clear_m", self.building_clear_m),
            ("mech_clear_m", self.mech_clear_m),
            ("paved_bonus_m", self.paved_bonus_m),
            ("wet_penalty_m", self.wet_penalty_m),
        ] {
            crate::data::finite(field, v)?;
            if v < 0.0 {
                return Err(format!("{field}: must not be negative"));
            }
        }
        Ok(())
    }
}

/// The ground as a drop sees it.
pub trait DropGround {
    /// The ground's height at `p` (x, z), metres.
    fn height_m(&self, p: Vec2) -> f32;
    /// How deep the water over the ground at `p` is, metres; zero when dry.
    fn water_depth_m(&self, p: Vec2) -> f32;
    /// True when a circle of `radius_m` at `p` overlaps anything built or standing on the ground: a building, a
    /// hangar, a tower, a rock.
    fn blocked(&self, p: Vec2, radius_m: f32) -> bool;
    /// True when `p` is paved: a road, a street, a plaza, an apron.
    fn paved(&self, p: Vec2) -> bool;
}

/// What the mech needs of its ground, from its own data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Needs {
    /// Its footprint's radius, metres (`locomotion.body_radius_m`).
    pub radius_m: f32,
    /// The deepest water it lands in, metres: its skim depth, the lesser of its wading depth and the depth at
    /// which its skates would ride the water.
    pub max_depth_m: f32,
}

impl Needs {
    /// A mech's needs, from its data.
    pub fn of(mech: &MechConfig) -> Self {
        let l = &mech.locomotion;
        Self {
            radius_m: l.body_radius_m,
            max_depth_m: l.max_wade_depth_m.min(l.skim_depth_share * l.body_height_m),
        }
    }
}

/// What a site keeps clear of: the pilot, and every mech already standing (feet x, z, and footprint radius).
#[derive(Clone, Copy, Debug)]
pub struct Keep<'a> {
    /// Where the pilot is: their feet on foot, their mech's feet in the seat.
    pub pilot: Vec2,
    /// The mechs standing, each with its footprint radius, metres.
    pub mechs: &'a [(Vec2, f32)],
}

/// A spot good enough to land on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Where (x, ground height, z), metres.
    pub at: Vec3,
    /// On paving.
    pub paved: bool,
    /// In water (no deeper than the mech's skim depth).
    pub wet: bool,
}

/// Is `p` good ground for a mech with `needs`, clear of everything in `keep` but the pilot: not overlapping
/// anything built with its footprint and the clearance, no deeper in water than it lands in, no steeper across its
/// footprint than the limit, and clear of every other mech. The pilot's own distance is the search's to keep.
pub fn good_spot(cfg: &DropConfig, ground: &dyn DropGround, needs: Needs, keep: &Keep, p: Vec2) -> Option<Spot> {
    if !p.is_finite() {
        return None;
    }
    let r = needs.radius_m;
    if ground.blocked(p, r + cfg.building_clear_m) {
        return None;
    }
    // The slope: the rise between opposite points on the footprint's rim, the steepest of four directions; the
    // water: the deepest of the centre and those eight points.
    let h = ground.height_m(p);
    let limit = cfg.max_slope_deg.to_radians().tan() * 2.0 * r;
    let mut depth = ground.water_depth_m(p);
    for dir in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0).normalize(), Vec2::new(1.0, -1.0).normalize()] {
        let rise = (ground.height_m(p + dir * r) - ground.height_m(p - dir * r)).abs();
        if !rise.is_finite() || rise > limit {
            return None;
        }
        depth = depth.max(ground.water_depth_m(p + dir * r)).max(ground.water_depth_m(p - dir * r));
    }
    if !depth.is_finite() || depth > needs.max_depth_m {
        return None;
    }
    if keep.mechs.iter().any(|(m, mr)| m.distance(p) < r + mr + cfg.mech_clear_m) {
        return None;
    }
    Some(Spot {
        at: Vec3::new(p.x, h, p.y),
        paved: ground.paved(p),
        wet: depth > 0.0,
    })
}

/// A good spot's score: its distance from the pilot, a paved one nearer, a wet one farther. Lowest wins.
fn score(cfg: &DropConfig, pilot: Vec2, s: &Spot) -> f32 {
    let d = Vec2::new(s.at.x, s.at.z).distance(pilot);
    d - if s.paved { cfg.paved_bonus_m } else { 0.0 } + if s.wet { cfg.wet_penalty_m } else { 0.0 }
}

/// Where a bought mech lands (design section 6.1): rings outward from the pilot, `ring_step_m` apart from
/// `ring_min_m`, spots about `spot_spacing_m` apart on each, the first spot on each ring at an angle drawn from
/// `rng`. Every good spot inside the ring (to `ring_max_m`) is scored, and the best wins, ties to the spot tried
/// first; only when the ring holds none does the search go on, a ring at a time, and the first ring with a good
/// spot gives the site. `None` when nothing out to `search_max_m` is good: the mech goes to a bay instead.
pub fn drop_site(cfg: &DropConfig, ground: &dyn DropGround, needs: Needs, keep: &Keep, rng: &SimRng) -> Option<Spot> {
    if !keep.pilot.is_finite() {
        return None;
    }
    let rings = ((cfg.search_max_m - cfg.ring_min_m) / cfg.ring_step_m).floor().max(0.0) as usize;
    let mut best: Option<(f32, Spot)> = None;
    for i in 0..=rings {
        let r = cfg.ring_min_m + i as f32 * cfg.ring_step_m;
        let mut ring_rng = rng.fork(&format!("ring/{i}"));
        let phase = ring_rng.range_f32(0.0, std::f32::consts::TAU);
        let n = ((std::f32::consts::TAU * r / cfg.spot_spacing_m).ceil() as usize).clamp(8, 4096);
        for k in 0..n {
            let a = phase + std::f32::consts::TAU * k as f32 / n as f32;
            let p = keep.pilot + Vec2::new(a.cos(), a.sin()) * r;
            let Some(s) = good_spot(cfg, ground, needs, keep, p) else {
                continue;
            };
            let sc = score(cfg, keep.pilot, &s);
            if best.as_ref().is_none_or(|(b, _)| sc < *b) {
                best = Some((sc, s));
            }
        }
        // Inside the ring every spot competes; past it, the first ring with a good spot ends the search.
        if best.is_some() && r + cfg.ring_step_m > cfg.ring_max_m {
            break;
        }
    }
    best.map(|(_, s)| s)
}

/// The island as a drop sees it: the generated world and its bases' layout (the hangars' size is data the plan
/// does not keep).
#[derive(Clone, Copy)]
pub struct PlanGround<'a> {
    /// The generated world.
    pub plan: &'a WorldPlan,
    /// `bases:` in `data/world/world.ron`.
    pub bases: &'a BasesConfig,
}

impl DropGround for PlanGround<'_> {
    fn height_m(&self, p: Vec2) -> f32 {
        self.plan.terrain.height_at(p.x, p.y)
    }

    fn water_depth_m(&self, p: Vec2) -> f32 {
        self.plan
            .waters
            .water_at(p, self.plan.terrain.height_at(p.x, p.y))
            .map_or(0.0, |w| w.depth_m)
    }

    /// Every building's footprint (the one collision grid), every base's hangars with their walls and its tower,
    /// and every rock's feet.
    fn blocked(&self, p: Vec2, radius_m: f32) -> bool {
        if self.plan.collision.overlaps(p, radius_m) {
            return true;
        }
        if self
            .plan
            .bases
            .iter()
            .filter(|b| b.rect().distance(p) < radius_m)
            .flat_map(|b| b.structures(self.bases))
            .any(|s| s.distance(p) < radius_m)
        {
            return true;
        }
        self.plan
            .rocks
            .iter()
            .flat_map(|r| r.feet())
            .any(|(c, rr)| c.distance(p) < radius_m + rr)
    }

    /// The world's one cover rule (`WorldPlan::surface_at`): a highway, a city outside its parks, a base's site.
    fn paved(&self, p: Vec2) -> bool {
        self.plan.surface_at(p) == Surface::Concrete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::Rect;

    /// Ground made up for a test: flat at zero unless a slope or water says otherwise.
    #[derive(Default)]
    struct Made {
        buildings: Vec<Rect>,
        /// Water this deep over each rectangle.
        water: Vec<(Rect, f32)>,
        /// Ground rising at this gradient (rise over run) along +x inside each rectangle.
        slopes: Vec<(Rect, f32)>,
        paved: Vec<Rect>,
    }

    impl DropGround for Made {
        fn height_m(&self, p: Vec2) -> f32 {
            self.slopes
                .iter()
                .find(|(r, _)| r.distance(p) == 0.0)
                .map_or(0.0, |(r, g)| (p.x - r.min.x) * g)
        }
        fn water_depth_m(&self, p: Vec2) -> f32 {
            self.water
                .iter()
                .filter(|(r, _)| r.distance(p) == 0.0)
                .map(|(_, d)| *d)
                .fold(0.0, f32::max)
        }
        fn blocked(&self, p: Vec2, radius_m: f32) -> bool {
            self.buildings.iter().any(|b| b.distance(p) < radius_m)
        }
        fn paved(&self, p: Vec2) -> bool {
            self.paved.iter().any(|r| r.distance(p) == 0.0)
        }
    }

    /// A Foreman's needs (`data/mechs/foreman.ron`: 7.56 m footprint, 12 m skim depth).
    const NEEDS: Needs = Needs {
        radius_m: 7.56,
        max_depth_m: 12.0,
    };

    fn cfg() -> DropConfig {
        crate::data::load_ron(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/career/air_drop.ron")).unwrap()
    }

    fn keep(pilot: Vec2) -> Keep<'static> {
        Keep { pilot, mechs: &[] }
    }

    /// A dense block: 40 m towers on a 52 m pitch (12 m streets, too narrow for a mech) for 600 m round the
    /// pilot, with one 120 m plaza 260 m east of them.
    fn dense_city() -> Made {
        let mut g = Made::default();
        let plaza = Rect::centred(Vec2::new(260.0, 0.0), Vec2::splat(60.0));
        for i in -12..=12 {
            for j in -12..=12 {
                let c = Vec2::new(i as f32 * 52.0, j as f32 * 52.0);
                let b = Rect::centred(c, Vec2::splat(20.0));
                if !b.overlaps(&plaza) {
                    g.buildings.push(b);
                }
            }
        }
        g.paved.push(Rect::centred(Vec2::ZERO, Vec2::splat(700.0)));
        g
    }

    #[test]
    fn in_a_dense_city_block_the_site_is_never_in_a_building() {
        let c = cfg();
        let g = dense_city();
        let pilot = Vec2::new(26.0, 26.0); // a street crossing among the towers
        for seed in 0..16 {
            let s = drop_site(&c, &g, NEEDS, &keep(pilot), &SimRng::new(seed)).expect("the plaza takes it");
            let at = Vec2::new(s.at.x, s.at.z);
            assert!(
                !g.blocked(at, NEEDS.radius_m + c.building_clear_m),
                "seed {seed}: {at} is clear of every tower by the footprint and the clearance"
            );
            assert!(g.buildings.iter().all(|b| b.distance(at) >= NEEDS.radius_m));
        }
    }

    #[test]
    fn the_site_is_never_in_deep_water() {
        let c = cfg();
        // A lake 30 m deep everywhere but a dry strip 200 m east, 60 m wide.
        let strip = Rect::centred(Vec2::new(200.0, 0.0), Vec2::new(30.0, 800.0));
        let made = Made {
            water: vec![
                (Rect::centred(Vec2::new(-315.0, 0.0), Vec2::new(485.0, 800.0)), 30.0),
                (Rect::centred(Vec2::new(615.0, 0.0), Vec2::new(385.0, 800.0)), 30.0),
            ],
            ..Default::default()
        };
        for seed in 0..8 {
            let s = drop_site(&c, &made, NEEDS, &keep(Vec2::ZERO), &SimRng::new(seed)).expect("the strip takes it");
            let at = Vec2::new(s.at.x, s.at.z);
            assert!(
                made.water_depth_m(at) <= NEEDS.max_depth_m,
                "seed {seed}: {at} is not in deep water"
            );
            assert!(strip.distance(at) == 0.0, "seed {seed}: {at} is on the dry strip");
        }
    }

    #[test]
    fn dry_ground_wins_over_shallow_water_in_the_ring() {
        let c = cfg();
        // Shallow water (2 m) everywhere: it lands in it, as the last resort.
        let shallow = Made {
            water: vec![(Rect::centred(Vec2::ZERO, Vec2::splat(800.0)), 2.0)],
            ..Default::default()
        };
        let s = drop_site(&c, &shallow, NEEDS, &keep(Vec2::ZERO), &SimRng::new(3)).unwrap();
        assert!(s.wet, "with no dry ground the shallow water takes it");
        // The same water with a dry island 140 m north, 50 m across: the dry ground wins though it is farther.
        let island = Rect::centred(Vec2::new(0.0, 140.0), Vec2::splat(25.0));
        let dry = Made {
            water: vec![
                (
                    Rect {
                        min: Vec2::splat(-800.0),
                        max: Vec2::new(800.0, 115.0),
                    },
                    2.0,
                ),
                (
                    Rect {
                        min: Vec2::new(-800.0, 165.0),
                        max: Vec2::splat(800.0),
                    },
                    2.0,
                ),
                (
                    Rect {
                        min: Vec2::new(-800.0, 115.0),
                        max: Vec2::new(-25.0, 165.0),
                    },
                    2.0,
                ),
                (
                    Rect {
                        min: Vec2::new(25.0, 115.0),
                        max: Vec2::new(800.0, 165.0),
                    },
                    2.0,
                ),
            ],
            ..Default::default()
        };
        let s = drop_site(&c, &dry, NEEDS, &keep(Vec2::ZERO), &SimRng::new(3)).unwrap();
        assert!(!s.wet, "{:?} is on the island", s.at);
        assert!(island.distance(Vec2::new(s.at.x, s.at.z)) == 0.0);
    }

    #[test]
    fn the_site_is_never_on_a_steep_slope() {
        let c = cfg();
        // A hillside rising at 1 in 2 (27 degrees) everywhere west of x = 120 m; level beyond.
        let made = Made {
            slopes: vec![(
                Rect {
                    min: Vec2::new(-800.0, -800.0),
                    max: Vec2::new(120.0, 800.0),
                },
                0.5,
            )],
            ..Default::default()
        };
        for seed in 0..8 {
            let s = drop_site(&c, &made, NEEDS, &keep(Vec2::ZERO), &SimRng::new(seed)).expect("the level ground takes it");
            let at = Vec2::new(s.at.x, s.at.z);
            assert!(at.x >= 120.0 + NEEDS.radius_m - 0.01, "seed {seed}: {at} is off the hillside");
        }
    }

    #[test]
    fn on_open_ground_the_site_is_always_inside_the_ring() {
        let c = cfg();
        let g = Made::default();
        for seed in 0..32 {
            let pilot = Vec2::new(seed as f32 * 37.0, -(seed as f32) * 11.0);
            let s = drop_site(&c, &g, NEEDS, &keep(pilot), &SimRng::new(seed)).unwrap();
            let d = Vec2::new(s.at.x, s.at.z).distance(pilot);
            assert!(
                (c.ring_min_m - 0.01..=c.ring_max_m + 0.01).contains(&d),
                "seed {seed}: {d:.1} m from the pilot"
            );
        }
    }

    #[test]
    fn the_ring_holds_good_ground_so_it_lands_in_the_ring_even_when_far_off_is_paved() {
        let c = cfg();
        // One good spot inside the ring, wide open paved ground just past it.
        let mut g = Made::default();
        g.paved.push(Rect::centred(Vec2::new(400.0, 0.0), Vec2::splat(200.0)));
        let s = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(1)).unwrap();
        assert!(Vec2::new(s.at.x, s.at.z).length() <= c.ring_max_m + 0.01);
    }

    #[test]
    fn a_road_in_the_ring_is_preferred_to_open_ground_nearer() {
        let c = cfg();
        // A road 20 m wide, 45-65 m north of the pilot: paved, 30 m of bonus, beats open ground at 40 m.
        let mut g = Made::default();
        g.paved.push(Rect::centred(Vec2::new(0.0, 55.0), Vec2::new(800.0, 10.0)));
        let s = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(9)).unwrap();
        assert!(s.paved, "{:?} is on the road", s.at);
    }

    #[test]
    fn it_keeps_clear_of_other_mechs() {
        let c = cfg();
        let g = Made::default();
        let mechs: Vec<(Vec2, f32)> = (0..64)
            .map(|k| {
                let a = k as f32 / 64.0 * std::f32::consts::TAU;
                (Vec2::new(a.cos(), a.sin()) * 45.0, 7.56)
            })
            .collect();
        let k = Keep {
            pilot: Vec2::ZERO,
            mechs: &mechs,
        };
        let s = drop_site(&c, &g, NEEDS, &k, &SimRng::new(2)).unwrap();
        let at = Vec2::new(s.at.x, s.at.z);
        for (m, r) in &mechs {
            assert!(m.distance(at) >= NEEDS.radius_m + r + c.mech_clear_m - 0.01);
        }
    }

    #[test]
    fn with_no_good_ground_in_the_search_there_is_no_site() {
        let c = cfg();
        let made = Made {
            buildings: vec![Rect::centred(Vec2::ZERO, Vec2::splat(2000.0))],
            ..Default::default()
        };
        assert_eq!(drop_site(&c, &made, NEEDS, &keep(Vec2::ZERO), &SimRng::new(4)), None);
        let deep = Made {
            water: vec![(Rect::centred(Vec2::ZERO, Vec2::splat(2000.0)), 40.0)],
            ..Default::default()
        };
        assert_eq!(drop_site(&c, &deep, NEEDS, &keep(Vec2::ZERO), &SimRng::new(4)), None);
    }

    #[test]
    fn the_same_seed_gives_the_same_site_and_another_seed_another() {
        let c = cfg();
        let g = Made::default();
        let a = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(77)).unwrap();
        let b = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(77)).unwrap();
        assert_eq!(a, b);
        let other = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(78)).unwrap();
        assert_ne!(a.at, other.at, "the seed turns the ring");
    }

    #[test]
    fn on_the_island_a_drop_in_the_biggest_city_misses_every_building() {
        let world = crate::world::WorldConfig::default();
        let plan = crate::world::generate(42, &world);
        let g = PlanGround {
            plan: &plan,
            bases: &world.bases,
        };
        let c = cfg();
        let city = &plan.cities[0];
        let pilot = city.streets[0].rect.centre();
        let s = drop_site(&c, &g, NEEDS, &keep(pilot), &SimRng::new(42)).expect("the city has a street wide enough");
        let at = Vec2::new(s.at.x, s.at.z);
        assert!(!plan.collision.overlaps(at, NEEDS.radius_m + c.building_clear_m), "{at}");
        assert!(g.water_depth_m(at) <= NEEDS.max_depth_m);
        // And at a base: never in a hangar or the tower.
        let base = plan.bases[0];
        let inside = base.hangars(&world.bases)[1].centre();
        let s = drop_site(&c, &g, NEEDS, &keep(inside), &SimRng::new(42)).expect("the apron takes it");
        let at = Vec2::new(s.at.x, s.at.z);
        assert!(
            base.structures(&world.bases).iter().all(|r| r.distance(at) >= NEEDS.radius_m),
            "{at}"
        );
        assert!(s.paved, "the apron is paved");
    }
}
