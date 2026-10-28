//! The chase camera while driving a car (openspec change `car-chase-camera`; owner, 2026-10-26: "also driving
//! should be in 3rd person", the one exception to THE LAW in CLAUDE.md). Pure rules: where the camera sits behind
//! and above the car for the view the player is looking along, how it pulls in to stay out of walls, how the view
//! drifts back behind the car after the mouse has been still for a while, and how it eases out of the pilot's eyes
//! on getting in and back into them on getting out. The player's mouse orbits it; nothing else turns the view
//! except that drift, which the change's design records as provisional. The app reads these and places the lens.

use crate::data::{Validate, positive, within};
use glam::{Quat, Vec3};
use serde::Deserialize;

/// The chase camera's tuning, `car_camera` in `data/pilot/pilot.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ChaseCamera {
    /// The camera's distance from the car's middle for a car of no length, metres.
    pub distance_m: f32,
    /// More distance for each metre of the car's length, so a bus is framed as a sedan is.
    pub distance_per_length: f32,
    /// The point the camera looks at and orbits sits this far over the car's roof, metres.
    pub pivot_above_roof_m: f32,
    /// The view's pitch with the mouse's pitch at zero, degrees (negative looks down at the car).
    pub rest_pitch_deg: f32,
    /// The lowest and highest the orbit's pitch goes, degrees.
    pub pitch_range_deg: [f32; 2],
    /// The orbit's point follows the car this slowly, seconds (an exponential lag; 0 for none).
    pub follow_lag_s: f32,
    /// The view turns with the car's heading this slowly, seconds (an exponential lag; 0 for none).
    pub heading_lag_s: f32,
    /// With the mouse still this long and the car moving, the view drifts back behind the car, seconds.
    pub recenter_after_s: f32,
    /// How fast it drifts back, degrees a second.
    pub recenter_deg_s: f32,
    /// The car must be faster than this for the view to drift back, m/s: parked, the view stays where it was left.
    pub recenter_min_speed_m_s: f32,
    /// Where something stands between the orbit's point and the camera, the camera stops this short of it, metres.
    pub clip_margin_m: f32,
    /// The camera is never pulled in closer to the orbit's point than this, metres.
    pub min_distance_m: f32,
    /// Getting in, the camera eases out of the driver's eyes to its place over this long, and back on getting out,
    /// seconds.
    pub blend_s: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            distance_m: 2.5,
            distance_per_length: 1.2,
            pivot_above_roof_m: 0.6,
            rest_pitch_deg: -12.0,
            pitch_range_deg: [-60.0, 30.0],
            follow_lag_s: 0.08,
            heading_lag_s: 0.25,
            recenter_after_s: 2.0,
            recenter_deg_s: 90.0,
            recenter_min_speed_m_s: 2.0,
            clip_margin_m: 0.4,
            min_distance_m: 1.5,
            blend_s: 0.6,
        }
    }
}

impl Validate for ChaseCamera {
    fn validate(&self) -> Result<(), String> {
        positive("distance_m", self.distance_m)?;
        within("distance_per_length", self.distance_per_length, 0.0, 10.0)?;
        within("pivot_above_roof_m", self.pivot_above_roof_m, 0.0, 20.0)?;
        within("rest_pitch_deg", self.rest_pitch_deg, -89.0, 89.0)?;
        let [lo, hi] = self.pitch_range_deg;
        within("pitch_range_deg[0]", lo, -89.0, 89.0)?;
        within("pitch_range_deg[1]", hi, lo, 89.0)?;
        if !(lo..=hi).contains(&self.rest_pitch_deg) {
            return Err("`rest_pitch_deg` must lie inside `pitch_range_deg`".into());
        }
        within("follow_lag_s", self.follow_lag_s, 0.0, 5.0)?;
        within("heading_lag_s", self.heading_lag_s, 0.0, 5.0)?;
        positive("recenter_after_s", self.recenter_after_s)?;
        within("recenter_deg_s", self.recenter_deg_s, 0.0, 1000.0)?;
        within("recenter_min_speed_m_s", self.recenter_min_speed_m_s, 0.0, 100.0)?;
        within("clip_margin_m", self.clip_margin_m, 0.0, 5.0)?;
        positive("min_distance_m", self.min_distance_m)?;
        positive("blend_s", self.blend_s)?;
        Ok(())
    }
}

/// A view's rotation for a yaw (0 faces -z, positive turns left) and a pitch (up positive), radians.
pub fn view_rotation(yaw_rad: f32, pitch_rad: f32) -> Quat {
    Quat::from_rotation_y(yaw_rad) * Quat::from_rotation_x(pitch_rad)
}

impl ChaseCamera {
    /// The camera's full distance for a car `length_m` long, metres.
    pub fn distance_for(&self, length_m: f32) -> f32 {
        self.distance_m + self.distance_per_length * length_m.max(0.0)
    }

    /// The point the camera orbits: over the middle of a car standing at `ground` (its middle on the ground),
    /// `height_m` tall.
    pub fn pivot(&self, ground: Vec3, height_m: f32) -> Vec3 {
        ground + Vec3::Y * (height_m + self.pivot_above_roof_m)
    }

    /// The pitch the camera looks along for the player's own pitch `pitch_rad` (zero at rest), radians: the rest
    /// pitch plus theirs, within the range.
    pub fn pitch(&self, pitch_rad: f32) -> f32 {
        let [lo, hi] = self.pitch_range_deg;
        (self.rest_pitch_deg.to_radians() + pitch_rad).clamp(lo.to_radians(), hi.to_radians())
    }

    /// The player's own pitch while driving is kept to what the range can show, so the mouse never winds up past
    /// an end: radians, zero at rest.
    pub fn clamp_player_pitch(&self, pitch_rad: f32) -> f32 {
        let [lo, hi] = self.pitch_range_deg;
        let rest = self.rest_pitch_deg;
        pitch_rad.clamp((lo - rest).to_radians(), (hi - rest).to_radians())
    }

    /// Where the camera wants to be: `distance_m` back from `pivot` against a view along `yaw_rad`, `pitch_rad`
    /// (the camera's pitch, `pitch`).
    pub fn wanted_eye(&self, pivot: Vec3, yaw_rad: f32, pitch_rad: f32, distance_m: f32) -> Vec3 {
        pivot - view_rotation(yaw_rad, pitch_rad) * Vec3::NEG_Z * distance_m
    }

    /// The camera pulled in toward `pivot` so it stays out of anything `blocked` (a point inside a building or
    /// under the ground): the boom is walked out from the pivot in `step_m` steps and the camera stops
    /// `clip_margin_m` short of the first blocked point, never closer than `min_distance_m` (or than the blocked
    /// point itself, when that is nearer still).
    pub fn clip(&self, pivot: Vec3, wanted: Vec3, step_m: f32, blocked: impl Fn(Vec3) -> bool) -> Vec3 {
        let span = wanted - pivot;
        let len = span.length();
        if !(len.is_finite() && len > 1e-4 && step_m.is_finite() && step_m > 0.0) {
            return wanted;
        }
        let dir = span / len;
        let mut s = step_m.min(len);
        loop {
            if blocked(pivot + dir * s) {
                let keep = (s - self.clip_margin_m).max(self.min_distance_m.min(s * 0.5));
                return pivot + dir * keep;
            }
            if s >= len {
                return wanted;
            }
            s = (s + step_m).min(len);
        }
    }

    /// The view's yaw from the car's nose and the player's pitch after the mouse has been still `idle_s` seconds
    /// with the car at `speed_m_s`: past `recenter_after_s`, both drift back toward zero (behind the car, at rest)
    /// at `recenter_deg_s`. Otherwise unchanged. Radians in and out.
    pub fn recenter(&self, rel_rad: f32, pitch_rad: f32, idle_s: f32, speed_m_s: f32, dt_s: f32) -> (f32, f32) {
        if idle_s < self.recenter_after_s || speed_m_s.abs() < self.recenter_min_speed_m_s || !(dt_s.is_finite() && dt_s > 0.0) {
            return (rel_rad, pitch_rad);
        }
        let step = self.recenter_deg_s.to_radians() * dt_s;
        let toward = |a: f32| a - a.clamp(-step, step);
        (toward(rel_rad), toward(pitch_rad))
    }
}

/// `from` moved toward `to` by an exponential lag of `lag_s` over `dt_s` (no lag: `to`).
pub fn ease(from: Vec3, to: Vec3, dt_s: f32, lag_s: f32) -> Vec3 {
    if lag_s <= 0.0 || !(dt_s.is_finite() && dt_s > 0.0) {
        return to;
    }
    from.lerp(to, 1.0 - (-dt_s / lag_s).exp())
}

/// The angle `from` moved toward `to` the short way round by an exponential lag of `lag_s` over `dt_s`, radians.
pub fn ease_angle(from: f32, to: f32, dt_s: f32, lag_s: f32) -> f32 {
    if lag_s <= 0.0 || !(dt_s.is_finite() && dt_s > 0.0) {
        return to;
    }
    from + super::wrap_rad(to - from) * (1.0 - (-dt_s / lag_s).exp())
}

/// The camera between the pilot's eyes (`first`, its rotation `first_rot`) and its chase place (`chase`,
/// `chase_rot`), at a 0..1 blend eased in and out (a smoothstep): 0 is the eyes, 1 the chase camera.
pub fn blend_view(first: Vec3, first_rot: Quat, chase: Vec3, chase_rot: Quat, blend: f32) -> (Vec3, Quat) {
    let b = blend.clamp(0.0, 1.0);
    let s = b * b * (3.0 - 2.0 * b);
    (first.lerp(chase, s), first_rot.slerp(chase_rot, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn the_camera_sits_behind_and_above_a_car_looking_ahead_along_its_nose() {
        let c = ChaseCamera::default();
        let pivot = c.pivot(Vec3::ZERO, 1.65);
        let eye = c.wanted_eye(pivot, 0.0, c.pitch(0.0), c.distance_for(4.7));
        // the car faces -z: behind it is +z, and the rest pitch looks down, so the camera is above the pivot
        assert!(eye.z > 5.0, "behind the car: {eye}");
        assert!(eye.y > pivot.y, "above the orbit's point: {eye}");
        assert!(eye.x.abs() < 1e-4);
        assert!((eye.distance(pivot) - c.distance_for(4.7)).abs() < 1e-4);
    }

    #[test]
    fn a_longer_car_is_framed_from_further_back() {
        let c = ChaseCamera::default();
        assert!(c.distance_for(12.0) > c.distance_for(4.7) + 5.0);
    }

    #[test]
    fn the_mouse_orbits_the_camera_round_the_car() {
        let c = ChaseCamera::default();
        let pivot = c.pivot(Vec3::ZERO, 1.65);
        // looking along the car's right side (yaw -90 degrees faces +x): the camera is off its left
        let eye = c.wanted_eye(pivot, -std::f32::consts::FRAC_PI_2, c.pitch(0.0), 8.0);
        assert!(eye.x < -7.0 && eye.z.abs() < 1e-3, "{eye}");
    }

    #[test]
    fn pitch_stays_within_the_range_and_the_mouse_cannot_wind_past_it() {
        let c = ChaseCamera::default();
        let [lo, hi] = c.pitch_range_deg;
        assert!((c.pitch(10.0) - hi.to_radians()).abs() < 1e-5);
        assert!((c.pitch(-10.0) - lo.to_radians()).abs() < 1e-5);
        let p = c.clamp_player_pitch(10.0);
        assert!((c.pitch(p) - hi.to_radians()).abs() < 1e-5);
        assert!((c.pitch(p - 0.1) - (hi.to_radians() - 0.1)).abs() < 1e-5, "it turns back at once");
    }

    #[test]
    fn a_wall_behind_the_car_pulls_the_camera_in_short_of_it() {
        let c = ChaseCamera::default();
        let pivot = Vec3::new(0.0, 2.0, 0.0);
        let wanted = Vec3::new(0.0, 2.0, 8.0);
        // a wall across z = 5
        let eye = c.clip(pivot, wanted, 0.25, |p| p.z >= 5.0);
        assert!(eye.z < 5.0 - c.clip_margin_m + 0.26 && eye.z > 4.0, "{eye}");
        // nothing in the way: the wanted place
        assert_eq!(c.clip(pivot, wanted, 0.25, |_| false), wanted);
        // the ground in the way of a camera looking up from under it: never through it
        let low = c.clip(pivot, Vec3::new(0.0, -3.0, 8.0), 0.25, |p| p.y < 0.0);
        assert!(low.y >= 0.0, "{low}");
    }

    #[test]
    fn the_view_drifts_back_behind_a_moving_car_only_after_the_mouse_has_been_still() {
        let c = ChaseCamera::default();
        let rel = 1.0;
        assert_eq!(
            c.recenter(rel, 0.2, 0.5, 15.0, DT),
            (rel, 0.2),
            "the mouse moved lately: the view stays"
        );
        assert_eq!(
            c.recenter(rel, 0.2, 10.0, 0.0, DT),
            (rel, 0.2),
            "parked: the view stays where it was left"
        );
        let (mut r, mut p) = (rel, 0.2);
        for _ in 0..(3.0 / DT) as usize {
            (r, p) = c.recenter(r, p, 10.0, 15.0, DT);
        }
        assert!(r.abs() < 1e-5 && p.abs() < 1e-5, "back behind the car at rest: {r} {p}");
        let (r1, _) = c.recenter(rel, 0.0, 10.0, 15.0, DT);
        assert!((rel - r1 - c.recenter_deg_s.to_radians() * DT).abs() < 1e-5, "at the data's rate");
    }

    #[test]
    fn getting_in_eases_out_of_the_eyes_and_getting_out_eases_back_in_with_no_cut() {
        let (eye, chase) = (Vec3::new(0.0, 1.3, 0.0), Vec3::new(0.0, 3.5, 8.0));
        let (r0, r1) = (view_rotation(0.0, 0.0), view_rotation(0.0, -0.2));
        assert_eq!(blend_view(eye, r0, chase, r1, 0.0).0, eye);
        assert_eq!(blend_view(eye, r0, chase, r1, 1.0).0, chase);
        let mut last = eye;
        for i in 1..=60 {
            let (p, _) = blend_view(eye, r0, chase, r1, i as f32 / 60.0);
            assert!(p.distance(last) < 0.5, "no jump at {i}: {last} to {p}");
            last = p;
        }
    }

    #[test]
    fn the_lags_settle_on_their_target_and_turn_the_short_way() {
        let mut p = Vec3::ZERO;
        for _ in 0..120 {
            p = ease(p, Vec3::X, DT, 0.1);
        }
        assert!(p.distance(Vec3::X) < 1e-3);
        let a = ease_angle(3.0, -3.0, DT, 0.2);
        assert!(a > 3.0, "from 3 to -3 radians goes on round past pi, not back through zero: {a}");
        assert_eq!(ease_angle(0.3, 0.5, DT, 0.0), 0.5);
    }

    #[test]
    fn the_default_tuning_is_valid() {
        assert_eq!(ChaseCamera::default().validate(), Ok(()));
    }
}
