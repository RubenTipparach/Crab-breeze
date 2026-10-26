//! Platforms: surfaces a body stands on that move (openspec change `crab-airship` section 9, "Standing on
//! it"): the Hermit's deck, shoulders and pylon walkways. Each is a footprint with a plane over it, in its
//! owner's frame; the owner's pose places them in the world. `Scene::support_at` stands bodies on them as it
//! does on roofs, and `carry` moves a body standing on one by the platform's own motion this step, so the
//! deck takes what stands on it along as it flies, sinks and lists. Pure.

use glam::{Affine3A, Vec2, Vec3};
use serde::Deserialize;

/// A body this far above a platform's top, or less, stands on it (and is carried with it).
pub const STAND_M: f32 = 0.6;

/// A walkable top in its owner's frame: a footprint and the plane over it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Slab {
    /// What it is (`deck`, `shoulder`, `walkway_L1`).
    pub name: String,
    /// The footprint's least corner (x, z), metres.
    pub min: Vec2,
    /// The footprint's greatest corner (x, z), metres.
    pub max: Vec2,
    /// Where `top_m` is measured (x, z), metres.
    pub centre: Vec2,
    /// The top's height at `centre`, metres.
    pub top_m: f32,
    /// How the top rises along x and z, metres a metre.
    pub slope: Vec2,
}

impl Slab {
    /// The top's height over the owner-frame point `(x, z)`, if it is inside the footprint.
    pub fn top_at(&self, p: Vec2) -> Option<f32> {
        let inside = p.cmpge(self.min).all() && p.cmple(self.max).all();
        inside.then(|| self.top_m + self.slope.dot(p - self.centre))
    }
}

/// Slabs in a moving frame: where its owner is this step and where it was the step before.
#[derive(Clone, Debug, PartialEq)]
pub struct Platform {
    /// Its tops, in the owner's frame.
    pub slabs: Vec<Slab>,
    /// Owner frame to world, this step.
    pub pose: Affine3A,
    /// Owner frame to world, the step before (for `carry`).
    pub was: Affine3A,
}

impl Platform {
    /// The highest top under world point `(x, z)` no higher than `feet_y + 1 m` (a step up, as a roof).
    pub fn top_under(&self, x: f32, z: f32, feet_y: f32) -> Option<f32> {
        let inv = self.pose.inverse();
        let local = inv.transform_point3(Vec3::new(x, feet_y, z));
        self.slabs
            .iter()
            .filter_map(|s| {
                let top = s.top_at(Vec2::new(local.x, local.z))?;
                Some(self.pose.transform_point3(Vec3::new(local.x, top, local.z)).y)
            })
            .filter(|y| *y <= feet_y + 1.0)
            .reduce(f32::max)
    }

    /// Where a body with its feet at `feet` is after this step's motion, if it stood on a top the step before
    /// (within `STAND_M` above it); `None` if it was not standing on this platform.
    pub fn carry(&self, feet: Vec3) -> Option<Vec3> {
        let local = self.was.inverse().transform_point3(feet);
        let stood = self.slabs.iter().any(|s| {
            s.top_at(Vec2::new(local.x, local.z))
                .is_some_and(|top| (local.y - top) >= -0.5 && (local.y - top) <= STAND_M)
        });
        stood.then(|| self.pose.transform_point3(local))
    }
}

/// The highest platform top under `(x, z)` within a step of `feet_y`.
pub fn top_under(platforms: &[Platform], x: f32, z: f32, feet_y: f32) -> Option<f32> {
    platforms.iter().filter_map(|p| p.top_under(x, z, feet_y)).reduce(f32::max)
}

/// Where a body with its feet at `feet` is carried to by the platform it stands on, if any.
pub fn carry(platforms: &[Platform], feet: Vec3) -> Option<Vec3> {
    platforms.iter().find_map(|p| p.carry(feet))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    fn deck() -> Slab {
        Slab {
            name: "deck".into(),
            min: Vec2::new(-40.0, -180.0),
            max: Vec2::new(40.0, 180.0),
            centre: Vec2::ZERO,
            top_m: 49.0,
            slope: Vec2::ZERO,
        }
    }

    fn at(pos: Vec3) -> Affine3A {
        Affine3A::from_translation(pos)
    }

    #[test]
    fn a_body_over_the_deck_stands_on_its_top() {
        let p = Platform {
            slabs: vec![deck()],
            pose: at(Vec3::new(100.0, 600.0, 0.0)),
            was: at(Vec3::new(100.0, 600.0, 0.0)),
        };
        assert_eq!(p.top_under(110.0, 20.0, 700.0), Some(649.0));
        assert_eq!(p.top_under(110.0, 20.0, 600.0), None, "below the deck: not on it");
        assert_eq!(p.top_under(200.0, 20.0, 700.0), None, "off its edge");
    }

    #[test]
    fn a_sloped_walkway_rises_along_its_slope() {
        let w = Slab {
            name: "walkway".into(),
            min: Vec2::new(100.0, -7.0),
            max: Vec2::new(144.0, 7.0),
            centre: Vec2::new(122.0, 0.0),
            top_m: 7.5,
            slope: Vec2::new(0.136, 0.0),
        };
        assert!((w.top_at(Vec2::new(144.0, 0.0)).unwrap() - (7.5 + 22.0 * 0.136)).abs() < 1e-4);
    }

    #[test]
    fn a_mech_standing_on_the_drifting_deck_stays_where_it_stood() {
        // The deck flies 15 m/s for a second in sixty steps, sinking and starting to list: a mech standing still
        // on it ends where it stood on the deck.
        let mut p = Platform {
            slabs: vec![deck()],
            pose: at(Vec3::new(0.0, 600.0, 0.0)),
            was: at(Vec3::new(0.0, 600.0, 0.0)),
        };
        let mut feet = Vec3::new(10.0, 649.0, 30.0);
        let start_local = p.pose.inverse().transform_point3(feet);
        for k in 1..=60 {
            let t = k as f32 / 60.0;
            p.was = p.pose;
            p.pose = Affine3A::from_rotation_translation(Quat::from_rotation_z(-0.05 * t), Vec3::new(15.0 * t, 600.0 - 2.0 * t, 0.0));
            feet = p.carry(feet).expect("still standing on it");
        }
        let end_local = p.pose.inverse().transform_point3(feet);
        assert!(end_local.distance(start_local) < 1e-3, "{start_local} -> {end_local}");
    }

    #[test]
    fn a_body_in_the_air_above_it_is_not_carried() {
        let p = Platform {
            slabs: vec![deck()],
            pose: at(Vec3::new(5.0, 600.0, 0.0)),
            was: at(Vec3::new(0.0, 600.0, 0.0)),
        };
        assert_eq!(p.carry(Vec3::new(0.0, 660.0, 0.0)), None);
        assert!(p.carry(Vec3::new(0.0, 649.2, 0.0)).is_some());
    }
}
