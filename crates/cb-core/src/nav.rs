//! Waypoint navigation (openspec change `waypoint-nav`; owner, 2026-10-30: "we need to add a waypoint nav system in the
//! pause menu. Add a map and let me plop waypoints down to navigate to"): the route the pilot puts down on the pause
//! menu's MAP page, and the rule that takes a waypoint off once the pilot reaches it. Here because it is a rule the
//! HUD shows and the scripted pilot could follow, with no engine in it; `cb-app::nav` draws it.

use crate::data::Validate;
use glam::Vec2;
use serde::Deserialize;

/// `data/hud/nav.ron`: the route's limits, the MAP page's zoom and pick, and the map image's colours.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct NavConfig {
    /// The most waypoints a route holds (survey WN1, provisional: a route of 9).
    pub max_waypoints: usize,
    /// A waypoint is reached, and taken off, within this distance of the pilot, metres (survey WN2, provisional).
    pub reach_m: f32,
    /// The MAP page's zoom, least and most (1 shows the whole island).
    pub zoom: [f32; 2],
    /// A right click this close to a waypoint on the map removes it, screen pixels.
    pub pick_px: f32,
    /// A left click held longer than this pans the map instead of putting down a waypoint, seconds.
    pub hold_pan_s: f32,
    /// The map image's side, pixels (one image, drawn once when the world loads).
    pub image_px: u32,
    /// The marker over a waypoint stands this far over its ground, metres.
    pub marker_height_m: f32,
    /// Land is coloured from `land_low_rgb` at sea level to `land_high_rgb` at `land_high_m` and above.
    pub land_low_rgb: [u8; 3],
    pub land_high_rgb: [u8; 3],
    pub land_high_m: f32,
    /// Water from `water_shallow_rgb` at the shore to `water_deep_rgb` at `water_deep_m` deep.
    pub water_shallow_rgb: [u8; 3],
    pub water_deep_rgb: [u8; 3],
    pub water_deep_m: f32,
    /// The highways' colour and width, pixels.
    pub highway_rgb: [u8; 3],
    pub highway_px: f32,
    /// The cities' streets.
    pub street_rgb: [u8; 3],
    /// The air bases' outline, and the airport's and the seaport's.
    pub base_rgb: [u8; 3],
    pub site_rgb: [u8; 3],
}

impl Default for NavConfig {
    fn default() -> Self {
        Self {
            max_waypoints: 9,
            reach_m: 150.0,
            zoom: [1.0, 8.0],
            pick_px: 12.0,
            hold_pan_s: 0.2,
            image_px: 1024,
            marker_height_m: 30.0,
            land_low_rgb: [28, 58, 34],
            land_high_rgb: [98, 98, 80],
            land_high_m: 900.0,
            water_shallow_rgb: [20, 78, 108],
            water_deep_rgb: [10, 38, 58],
            water_deep_m: 120.0,
            highway_rgb: [214, 214, 190],
            highway_px: 2.0,
            street_rgb: [150, 156, 142],
            base_rgb: [200, 210, 180],
            site_rgb: [255, 199, 77],
        }
    }
}

impl Validate for NavConfig {
    /// Refuses a value that cannot work: no waypoints, a non-positive or non-finite reach, a zoom out of order.
    fn validate(&self) -> Result<(), String> {
        let finite_pos = |name: &str, v: f32| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(format!("nav.{name} must be finite and above 0, is {v}"))
            }
        };
        if self.max_waypoints == 0 {
            return Err("nav.max_waypoints must be at least 1".into());
        }
        finite_pos("reach_m", self.reach_m)?;
        finite_pos("pick_px", self.pick_px)?;
        finite_pos("hold_pan_s", self.hold_pan_s)?;
        finite_pos("land_high_m", self.land_high_m)?;
        finite_pos("water_deep_m", self.water_deep_m)?;
        finite_pos("highway_px", self.highway_px)?;
        if !self.marker_height_m.is_finite() {
            return Err("nav.marker_height_m must be finite".into());
        }
        if !(self.zoom[0] > 0.0 && self.zoom[1] >= self.zoom[0] && self.zoom[1].is_finite()) {
            return Err(format!("nav.zoom must be (least > 0, most >= least), is {:?}", self.zoom));
        }
        if !(64..=2048).contains(&self.image_px) {
            return Err(format!(
                "nav.image_px must be 64..2048 (no texture over 2048 px), is {}",
                self.image_px
            ));
        }
        Ok(())
    }
}

/// The pilot's route: waypoints on the ground (x, z), the first the active one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Route {
    points: Vec<Vec2>,
}

/// What `Route::step` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Nothing reached.
    Underway,
    /// The active waypoint was reached and taken off; another is active now.
    Reached,
    /// The last waypoint was reached: the route is done.
    Arrived,
}

impl Route {
    /// The waypoints in order, the active one first.
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    /// The waypoint to make for, if any.
    pub fn active(&self) -> Option<Vec2> {
        self.points.first().copied()
    }

    /// Puts down a waypoint at the end of the route. A full route, or a point that is not finite, changes nothing
    /// and says so.
    pub fn add(&mut self, at: Vec2, max: usize) -> bool {
        if self.points.len() >= max || !at.is_finite() {
            return false;
        }
        self.points.push(at);
        true
    }

    /// Takes off the waypoint nearest `at` within `within_m`, if there is one; says which it was.
    pub fn remove_near(&mut self, at: Vec2, within_m: f32) -> Option<usize> {
        let (i, d) = self
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.distance(at)))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))?;
        (d <= within_m).then(|| {
            self.points.remove(i);
            i
        })
    }

    /// Takes off every waypoint.
    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// Takes off the active waypoint once `pilot` is within `reach_m` of it.
    pub fn step(&mut self, pilot: Vec2, reach_m: f32) -> Step {
        match self.active() {
            Some(p) if p.distance(pilot) <= reach_m => {
                self.points.remove(0);
                if self.points.is_empty() { Step::Arrived } else { Step::Reached }
            }
            _ => Step::Underway,
        }
    }

    /// The route's length from `pilot` through every waypoint in turn, metres.
    pub fn length_from(&self, pilot: Vec2) -> f32 {
        let mut at = pilot;
        let mut sum = 0.0;
        for &p in &self.points {
            sum += at.distance(p);
            at = p;
        }
        sum
    }
}

/// A distance as the HUD writes it: `850 M` under a kilometre, `1.4 KM` from one.
pub fn distance_label(m: f32) -> String {
    if m >= 1000.0 {
        format!("{:.1} KM", m / 1000.0)
    } else {
        format!("{:.0} M", m.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_config_is_valid_and_its_defaults_are_the_surveys_recommendations() {
        let cfg = NavConfig::default();
        cfg.validate().unwrap();
        assert_eq!(cfg.max_waypoints, 9);
        assert_eq!(cfg.reach_m, 150.0);
    }

    #[test]
    fn a_route_takes_waypoints_up_to_its_limit_and_refuses_a_point_that_is_not_finite() {
        let mut r = Route::default();
        for i in 0..3 {
            assert!(r.add(Vec2::splat(i as f32), 3));
        }
        assert!(!r.add(Vec2::ZERO, 3), "a full route takes no more");
        let mut r = Route::default();
        assert!(!r.add(Vec2::new(f32::NAN, 0.0), 9));
        assert!(r.points().is_empty());
    }

    #[test]
    fn reaching_the_active_waypoint_takes_it_off_and_the_last_one_arrives() {
        let mut r = Route::default();
        r.add(Vec2::new(0.0, 1000.0), 9);
        r.add(Vec2::new(0.0, 2000.0), 9);
        assert_eq!(r.step(Vec2::ZERO, 150.0), Step::Underway);
        assert_eq!(r.step(Vec2::new(0.0, 900.0), 150.0), Step::Reached);
        assert_eq!(r.active(), Some(Vec2::new(0.0, 2000.0)));
        assert_eq!(r.step(Vec2::new(100.0, 2000.0), 150.0), Step::Arrived);
        assert_eq!(r.active(), None);
        assert_eq!(r.step(Vec2::ZERO, 150.0), Step::Underway, "an empty route stays empty");
    }

    #[test]
    fn a_right_click_takes_off_only_the_nearest_waypoint_within_reach() {
        let mut r = Route::default();
        r.add(Vec2::new(0.0, 0.0), 9);
        r.add(Vec2::new(100.0, 0.0), 9);
        assert_eq!(r.remove_near(Vec2::new(500.0, 0.0), 50.0), None);
        assert_eq!(r.remove_near(Vec2::new(90.0, 0.0), 50.0), Some(1));
        assert_eq!(r.points(), &[Vec2::ZERO]);
    }

    #[test]
    fn the_route_length_runs_from_the_pilot_through_every_waypoint() {
        let mut r = Route::default();
        r.add(Vec2::new(0.0, 300.0), 9);
        r.add(Vec2::new(400.0, 300.0), 9);
        assert_eq!(r.length_from(Vec2::ZERO), 700.0);
    }

    #[test]
    fn distances_read_in_metres_under_a_kilometre_and_kilometres_from_one() {
        assert_eq!(distance_label(850.4), "850 M");
        assert_eq!(distance_label(1449.0), "1.4 KM");
    }

    #[test]
    fn a_config_that_cannot_work_is_refused() {
        let bad = NavConfig {
            zoom: [8.0, 1.0],
            ..NavConfig::default()
        };
        assert!(bad.validate().is_err());
        let bad = NavConfig {
            image_px: 4096,
            ..NavConfig::default()
        };
        assert!(bad.validate().is_err(), "no texture over 2048 px");
    }
}
