//! The offshore limit (openspec change `offshore-limit`; owner, 2026-10-29 and 30: "Make sure the mech can never leave
//! too far off the coast or it will automatically mission fail and respawn the player in a city. Even in free roam",
//! "Offshore limit: 15 km", and in the plan questions "15 km, edge clamps"). The limit is 15 km past the coast or 1 km
//! inside the world's edge, whichever comes first; a warning starts `warn_m` before it, and a pilot past it for
//! `grace_s` fails once. Which city they come back in is the nearest. Pure: `cb-app` feeds it where the pilot is each
//! fixed step and acts on what it says.

use crate::data::{Validate, positive};
use crate::world::City;
use crate::world::terrain::Shape;
use glam::Vec2;
use serde::Deserialize;

/// The limit, `bounds:` in `data/world/world.ron`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BoundsConfig {
    /// How far past the coast the limit lies, metres.
    pub limit_offshore_m: f32,
    /// ...unless the world's edge comes first: the limit stays this far inside it, metres.
    pub edge_margin_m: f32,
    /// The warning starts this far before the limit, metres.
    pub warn_m: f32,
    /// Past the limit this long fails, seconds (one step of jitter at the line cannot).
    pub grace_s: f32,
    /// After the failure is shown, the pilot comes back after this, seconds.
    pub respawn_after_s: f32,
}

impl Default for BoundsConfig {
    fn default() -> Self {
        Self {
            limit_offshore_m: 15000.0,
            edge_margin_m: 1000.0,
            warn_m: 2000.0,
            grace_s: 1.0,
            respawn_after_s: 3.0,
        }
    }
}

impl Validate for BoundsConfig {
    fn validate(&self) -> Result<(), String> {
        positive("bounds.limit_offshore_m", self.limit_offshore_m)?;
        positive("bounds.edge_margin_m", self.edge_margin_m)?;
        positive("bounds.warn_m", self.warn_m)?;
        positive("bounds.grace_s", self.grace_s)?;
        positive("bounds.respawn_after_s", self.respawn_after_s)
    }
}

/// How many metres the point `p` (x, z) stands before the limit (negative past it): the smaller of what is left before
/// `limit_offshore_m` past the coast and what is left before `edge_margin_m` inside the edge of a world `world_size_m`
/// across.
pub fn left_m(shape: &Shape, world_size_m: f32, p: Vec2, cfg: &BoundsConfig) -> f32 {
    let past_coast = shape.past_land_edge_m(p).max(0.0);
    let edge = world_size_m * 0.5 - p.x.abs().max(p.y.abs());
    (cfg.limit_offshore_m - past_coast).min(edge - cfg.edge_margin_m)
}

/// What the watch says this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    /// Well inside.
    Clear,
    /// Within `warn_m` of the limit, or past it inside the grace: metres left (negative past it).
    Warn(f32),
    /// Past the limit for the whole grace: the mission fails, once.
    Fail,
    /// Failed already: waiting for the respawn.
    Failed,
}

/// The pilot's watch: how long past the limit, and whether it has failed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Watch {
    /// Seconds past the limit without a break.
    pub past_s: f32,
    /// Failed, and not yet back in a city.
    pub failed: bool,
}

impl Watch {
    /// One fixed step of `dt_s`, the pilot `left` metres before the limit.
    pub fn step(&mut self, left: f32, dt_s: f32, cfg: &BoundsConfig) -> Verdict {
        if self.failed {
            return Verdict::Failed;
        }
        if !left.is_finite() {
            return Verdict::Clear;
        }
        self.past_s = if left < 0.0 { self.past_s + dt_s } else { 0.0 };
        if self.past_s >= cfg.grace_s {
            self.failed = true;
            return Verdict::Fail;
        }
        if left <= cfg.warn_m { Verdict::Warn(left) } else { Verdict::Clear }
    }

    /// The pilot is back in a city: the watch starts again.
    pub fn respawned(&mut self) {
        *self = Self::default();
    }
}

/// The city a failed pilot comes back in: the nearest to `p` by its square's edge (ties to the first).
pub fn respawn_city(cities: &[City], p: Vec2) -> Option<&City> {
    cities.iter().min_by(|a, b| a.rect().distance(p).total_cmp(&b.rect().distance(p)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;
    use crate::world::WorldConfig;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    fn world() -> &'static (WorldConfig, Shape) {
        static W: OnceLock<(WorldConfig, Shape)> = OnceLock::new();
        W.get_or_init(|| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
            let cfg: WorldConfig = load_ron(&p).unwrap();
            let shape = Shape::new(42, &cfg);
            (cfg, shape)
        })
    }

    /// Walks out from the island's middle along `dir` to the first point whose `left_m` is below zero.
    fn limit_along(dir: Vec2) -> (f32, f32) {
        let (cfg, shape) = world();
        let b = cfg.bounds;
        let mut coast = None;
        let mut r = 0.0;
        while r < cfg.world_size_m {
            let p = dir * r;
            if coast.is_none() && shape.past_land_edge_m(p) > 0.0 && r > 4000.0 {
                coast = Some(r);
            }
            if left_m(shape, cfg.world_size_m, p, &b) < 0.0 {
                return (coast.unwrap_or(r), r);
            }
            r += 50.0;
        }
        panic!("no limit along {dir}");
    }

    #[test]
    fn heading_east_the_worlds_edge_less_a_kilometre_stops_the_pilot() {
        let (cfg, _) = world();
        let (coast, limit) = limit_along(Vec2::X);
        assert!(
            limit - coast < 15000.0,
            "less than 15 km of sea before the edge: coast {coast}, limit {limit}"
        );
        assert!(
            (limit - (cfg.world_size_m * 0.5 - 1000.0)).abs() <= 50.0,
            "1 km inside the edge: {limit}"
        );
    }

    #[test]
    fn toward_a_corner_fifteen_kilometres_off_the_coast_stops_the_pilot() {
        let (_, shape) = world();
        let (coast, limit) = limit_along(Vec2::new(1.0, -1.0).normalize());
        let at = Vec2::new(1.0, -1.0).normalize() * limit;
        assert!(coast < limit);
        assert!(
            (shape.past_land_edge_m(at) - 15000.0).abs() <= 100.0,
            "15 km past the coast at the limit"
        );
    }

    #[test]
    fn on_land_the_pilot_is_well_inside() {
        let (cfg, shape) = world();
        assert!(left_m(shape, cfg.world_size_m, Vec2::ZERO, &cfg.bounds) > 10000.0);
    }

    #[test]
    fn the_watch_warns_before_the_line_and_fails_once_after_the_grace() {
        let cfg = BoundsConfig::default();
        let mut w = Watch::default();
        assert_eq!(w.step(5000.0, 0.1, &cfg), Verdict::Clear);
        assert_eq!(w.step(1500.0, 0.1, &cfg), Verdict::Warn(1500.0));
        // a step over the line and back is no failure
        assert_eq!(w.step(-1.0, 0.5, &cfg), Verdict::Warn(-1.0));
        assert_eq!(w.step(10.0, 0.1, &cfg), Verdict::Warn(10.0));
        assert_eq!(w.step(-1.0, 0.5, &cfg), Verdict::Warn(-1.0));
        assert_eq!(w.step(-2.0, 0.5, &cfg), Verdict::Fail);
        assert_eq!(w.step(-3.0, 0.5, &cfg), Verdict::Failed, "it fails once");
        w.respawned();
        assert_eq!(w.step(5000.0, 0.1, &cfg), Verdict::Clear);
    }

    #[test]
    fn a_failed_pilot_comes_back_in_the_nearest_city() {
        let (cfg, _) = world();
        let plan = crate::world::generate(42, cfg);
        let east = Vec2::new(cfg.world_size_m * 0.45, 0.0);
        let c = respawn_city(&plan.cities, east).expect("a city");
        let d = c.rect().distance(east);
        assert!(plan.cities.iter().all(|o| o.rect().distance(east) >= d - 1e-3));
    }
}
