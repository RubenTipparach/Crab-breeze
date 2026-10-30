//! A runway's approach cones (openspec change `island-sites-layout`, design section 2): a trapezoid off each end,
//! `length_m` long, the runway's width at the threshold opening to `far_width_m`, rising at `slope_deg` (a real ILS
//! glide path is 3 degrees). No other site, runway or building may rise into one. It lives here, apart from the sites
//! and the bases, because both place runways and both keep out of the other's cones: one implementation of the rule.

use super::layout::Rect;
use glam::Vec2;
use serde::Deserialize;

use crate::data::{Validate, positive};

/// The cones' shape, `approach:` under `sites:` in `data/world/world.ron`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ApproachConfig {
    /// How far out from the threshold a cone reaches, metres.
    pub length_m: f32,
    /// How wide it is at its far end, metres (at the threshold it is the runway's width).
    pub far_width_m: f32,
    /// How steeply it rises from the threshold, degrees.
    pub slope_deg: f32,
}

impl Default for ApproachConfig {
    fn default() -> Self {
        Self {
            length_m: 3000.0,
            far_width_m: 600.0,
            slope_deg: 3.0,
        }
    }
}

impl Validate for ApproachConfig {
    fn validate(&self) -> Result<(), String> {
        positive("sites.approach.length_m", self.length_m)?;
        positive("sites.approach.far_width_m", self.far_width_m)?;
        positive("sites.approach.slope_deg", self.slope_deg)
    }
}

/// One approach cone, in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Approach {
    /// The threshold's middle (x, z): the runway's end.
    pub threshold: Vec2,
    /// Out from the runway, unit (x, z).
    pub out: Vec2,
    /// The height of the threshold (the runway's surface), metres.
    pub ground_m: f32,
    /// Half its width at the threshold, metres.
    pub near_half_m: f32,
    /// The cones' shape.
    pub cfg: ApproachConfig,
}

impl Approach {
    /// Both ends' cones of the runway from `a` to `b` (x, z), `width_m` wide, its surface at `ground_m`.
    pub fn both(a: Vec2, b: Vec2, width_m: f32, ground_m: f32, cfg: ApproachConfig) -> [Approach; 2] {
        let d = (b - a).normalize_or_zero();
        let one = |threshold: Vec2, out: Vec2| Approach {
            threshold,
            out,
            ground_m,
            near_half_m: width_m * 0.5,
            cfg,
        };
        [one(a, -d), one(b, d)]
    }

    /// How far out along the cone and how far to its side `p` (x, z) stands.
    fn local(&self, p: Vec2) -> (f32, f32) {
        let q = p - self.threshold;
        (q.dot(self.out), q.dot(self.out.perp()).abs())
    }

    /// The cone's floor over `p` (x, z), metres above the sea, or `None` when `p` is not under it.
    pub fn floor_at(&self, p: Vec2) -> Option<f32> {
        let (along, side) = self.local(p);
        if !(0.0..=self.cfg.length_m).contains(&along) {
            return None;
        }
        let t = along / self.cfg.length_m;
        let half = self.near_half_m + (self.cfg.far_width_m * 0.5 - self.near_half_m) * t;
        (side <= half).then(|| self.ground_m + along * self.cfg.slope_deg.to_radians().tan())
    }

    /// True when any part of `r` lies under the cone: `r` sampled every `step_m` (edges included), and the cone's
    /// centre line and edges sampled into `r`.
    pub fn covers(&self, r: &Rect, step_m: f32) -> bool {
        let size = r.max - r.min;
        let (nx, nz) = (
            (size.x / step_m).ceil().max(1.0) as usize,
            (size.y / step_m).ceil().max(1.0) as usize,
        );
        for i in 0..=nx {
            for j in 0..=nz {
                let p = r.min + Vec2::new(size.x * i as f32 / nx as f32, size.y * j as f32 / nz as f32);
                if self.floor_at(p).is_some() {
                    return true;
                }
            }
        }
        let n = (self.cfg.length_m / step_m).ceil().max(1.0) as usize;
        (0..=n).any(|k| {
            let along = self.cfg.length_m * k as f32 / n as f32;
            let half = self.near_half_m + (self.cfg.far_width_m * 0.5 - self.near_half_m) * along / self.cfg.length_m;
            [-1.0f32, 0.0, 1.0].iter().any(|s| {
                let p = self.threshold + self.out * along + self.out.perp() * half * *s;
                r.distance(p) <= 0.0
            })
        })
    }

    /// True when a box over `r` reaching up to `top_m` rises into the cone.
    pub fn pierced_by(&self, r: &Rect, top_m: f32, step_m: f32) -> bool {
        let size = r.max - r.min;
        let (nx, nz) = (
            (size.x / step_m).ceil().max(1.0) as usize,
            (size.y / step_m).ceil().max(1.0) as usize,
        );
        for i in 0..=nx {
            for j in 0..=nz {
                let p = r.min + Vec2::new(size.x * i as f32 / nx as f32, size.y * j as f32 / nz as f32);
                if self.floor_at(p).is_some_and(|f| top_m > f) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cone() -> Approach {
        Approach::both(
            Vec2::new(-1000.0, 0.0),
            Vec2::new(1000.0, 0.0),
            45.0,
            10.0,
            ApproachConfig::default(),
        )[1]
    }

    #[test]
    fn a_cone_rises_three_degrees_from_its_threshold() {
        let c = cone();
        assert_eq!(c.floor_at(Vec2::new(1000.0, 0.0)), Some(10.0));
        let f = c.floor_at(Vec2::new(2000.0, 0.0)).expect("a kilometre out is under it");
        assert!((f - (10.0 + 1000.0 * 3f32.to_radians().tan())).abs() < 0.01, "{f}");
        assert_eq!(c.floor_at(Vec2::new(500.0, 0.0)), None, "over the runway is not the cone");
        assert_eq!(c.floor_at(Vec2::new(4100.0, 0.0)), None, "past 3 km");
    }

    #[test]
    fn a_cone_opens_from_the_runway_width_to_six_hundred_metres() {
        let c = cone();
        assert!(c.floor_at(Vec2::new(1001.0, 30.0)).is_none(), "the threshold is the runway's width");
        assert!(c.floor_at(Vec2::new(3999.0, 290.0)).is_some());
        assert!(c.floor_at(Vec2::new(3999.0, 310.0)).is_none());
    }

    #[test]
    fn a_tall_block_under_the_cone_pierces_it_and_a_low_one_does_not() {
        let c = cone();
        let r = Rect {
            min: Vec2::new(1900.0, -20.0),
            max: Vec2::new(1950.0, 20.0),
        };
        assert!(c.covers(&r, 25.0));
        assert!(c.pierced_by(&r, 10.0 + 80.0, 25.0), "80 m up, 0.9 km out");
        assert!(!c.pierced_by(&r, 10.0 + 30.0, 25.0), "30 m is under the 3 degree floor there");
    }
}
