//! How the sun reads to the pilot's eye (openspec change `sun-flare`): the bloom's tuning, the lens
//! flare's look, and the rules that place and fade the flare.
//!
//! These live here because they are plain numbers and pure functions with no engine in them: where
//! the sun is on screen (from the light's direction and the view-projection), how much of it the
//! depth samples round it leave clear, how it fades off screen, in cloud and under water, and how
//! bright it is for the exposure. `cb-app`'s `sun_flare` feeds them to the GPU, and
//! `assets/shaders/sun_flare.wesl` draws the flare. The tuning is `data/world/sun_flare.ron`, loaded
//! by the one data loader so `cb-tools validate` checks it.
//!
//! Screen positions are either NDC (x right, y up, -1..1 across the view) or "screen heights": the
//! same frame with the origin at the centre, scaled so one screen height is 1 and a pixel is square
//! (x runs -aspect/2..aspect/2, y -0.5..0.5). Every flare size is in screen heights (`_h`).

use crate::data::{Validate, finite3, positive, within};
use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::Deserialize;

/// The occlusion taps round the sun: the centre and two rings of eight (`taps` in the shader).
pub const OCCLUSION_TAPS: usize = 17;
/// The most ghosts the flare draws (`ghosts` in the shader).
pub const MAX_GHOSTS: usize = 8;
/// A depth at or below this is the sky (Bevy's depth is reverse-Z: the near plane 1, infinity 0).
/// The water's passes use the same test.
pub const SKY_DEPTH: f32 = 1.0e-7;

/// Everything about how the sun reads: the bloom on the pilot's eyes and the lens flare.
/// `data/world/sun_flare.ron`; a missing field takes the default below.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SunLook {
    /// Bevy's bloom on the pilot's eyes camera.
    pub bloom: BloomLook,
    /// The lens flare.
    pub flare: FlareLook,
}

/// Bevy's `Bloom` settings, field for field (Bevy's own names and ranges).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BloomLook {
    /// How much light scatters, 0..1 (energy-conserving).
    pub intensity: f32,
    /// How much more of it scatters widely, 0..1.
    pub low_frequency_boost: f32,
    /// How quickly the wide scatter falls to the narrow, 0..1.
    pub low_frequency_boost_curvature: f32,
    /// How wide the scatter reaches, 0..1 (1: up to 90 degrees).
    pub high_pass_frequency: f32,
    /// Only light brighter than this blooms (0: all light, physically based). Exposed linear units.
    pub threshold: f32,
    /// How softly the threshold cuts in, 0..1.
    pub threshold_softness: f32,
    /// Add the bloom on top instead of conserving energy (Bevy's advice when a threshold is set).
    pub additive: bool,
}

impl Default for BloomLook {
    fn default() -> Self {
        Self {
            intensity: 0.08,
            low_frequency_boost: 0.5,
            low_frequency_boost_curvature: 0.9,
            high_pass_frequency: 1.0,
            threshold: 0.0,
            threshold_softness: 0.0,
            additive: false,
        }
    }
}

/// A ghost's aperture shape.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum GhostShape {
    /// A round disc.
    #[default]
    Disc,
    /// The six-bladed aperture's hexagon.
    Hex,
}

/// One ghost: a reflection between the lens elements, on the line from the sun through the centre.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Ghost {
    /// Where on the line: 0 at the sun, 1 at the screen's centre, 2 the sun mirrored through it.
    pub offset: f32,
    /// Its radius (a hexagon's inner radius), screen heights.
    pub radius_h: f32,
    /// Its colour, linear, times the flare's strength.
    pub colour: [f32; 3],
    /// Disc or hexagon.
    pub shape: GhostShape,
    /// How soft its edge is, 0 (hard) to 1 (all falloff).
    pub softness: f32,
}

impl Default for Ghost {
    fn default() -> Self {
        Self {
            offset: 1.0,
            radius_h: 0.03,
            colour: [0.05, 0.08, 0.12],
            shape: GhostShape::Disc,
            softness: 0.3,
        }
    }
}

/// The lens flare's look. Colours are linear and multiply the flare's strength (the sun's exposed
/// illuminance times `gain`); sizes are screen heights.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FlareLook {
    /// The flare's strength per unit of the sun's exposed illuminance (lux times the camera's
    /// exposure).
    pub gain: f32,
    /// The bright core at the sun.
    pub core_colour: [f32; 3],
    /// The core's radius, screen heights.
    pub core_radius_h: f32,
    /// The wide glow round the sun.
    pub glow_colour: [f32; 3],
    /// The glow's half-strength radius, screen heights.
    pub glow_radius_h: f32,
    /// How fast the glow falls past its radius (higher: tighter).
    pub glow_power: f32,
    /// The starburst's spikes (the aperture blades' diffraction).
    pub spike_colour: [f32; 3],
    /// How many spikes.
    pub spike_count: u32,
    /// How thin the spikes are (an exponent on the angular cosine).
    pub spike_sharpness: f32,
    /// How far the spikes reach before they fall to 1/e, screen heights.
    pub spike_length_h: f32,
    /// How many fine rays between the spikes.
    pub ray_count: u32,
    /// The fine rays' strength against the spikes.
    pub ray_strength: f32,
    /// How far the fine rays reach, screen heights.
    pub ray_length_h: f32,
    /// How far the starburst turns as the sun crosses the screen, radians per screen height of the
    /// sun's x plus y.
    pub spike_turn_rad_per_h: f32,
    /// How much longer red reaches than blue along the spikes, a share of the length.
    pub spike_fringe: f32,
    /// The anamorphic streak across the sun.
    pub streak_colour: [f32; 3],
    /// How far the streak reaches to each side before it falls to 1/e, screen heights.
    pub streak_length_h: f32,
    /// The streak's half thickness, screen heights.
    pub streak_thickness_h: f32,
    /// The halo ring round the sun.
    pub halo_colour: [f32; 3],
    /// The ring's radius, screen heights.
    pub halo_radius_h: f32,
    /// The ring's half width, screen heights.
    pub halo_width_h: f32,
    /// How far red and blue ring apart, a share of the radius.
    pub halo_fringe: f32,
    /// The ghosts, at most `MAX_GHOSTS`.
    pub ghosts: Vec<Ghost>,
    /// The ghosts' chromatic fringe: red drawn this share larger and blue this share smaller.
    pub ghost_fringe: f32,
    /// How bright a ghost's rim is against its body.
    pub ghost_rim: f32,
    /// The hexagonal ghosts' rotation, degrees.
    pub aperture_deg: f32,
    /// The radius of the depth taps round the sun, screen heights: a hiding edge fades the flare
    /// across twice this.
    pub occlusion_radius_h: f32,
    /// How far past the edge of the view (NDC) the sun can be before the flare is gone.
    pub offscreen_margin_ndc: f32,
    /// The cloud density at the eye where the flare is gone (0..1, the cloud query's density).
    pub cloud_full_fade: f32,
    /// How fast the flare follows the eye into and out of cloud and water, seconds (a time constant).
    pub fade_s: f32,
}

impl Default for FlareLook {
    fn default() -> Self {
        Self {
            gain: 0.1,
            core_colour: [6.0, 5.0, 3.8],
            core_radius_h: 0.012,
            glow_colour: [1.0, 0.72, 0.45],
            glow_radius_h: 0.05,
            glow_power: 1.4,
            spike_colour: [1.4, 1.2, 0.95],
            spike_count: 6,
            spike_sharpness: 90.0,
            spike_length_h: 0.16,
            ray_count: 24,
            ray_strength: 0.35,
            ray_length_h: 0.07,
            spike_turn_rad_per_h: 0.6,
            spike_fringe: 0.08,
            streak_colour: [0.30, 0.50, 1.0],
            streak_length_h: 0.45,
            streak_thickness_h: 0.0025,
            halo_colour: [0.10, 0.12, 0.14],
            halo_radius_h: 0.22,
            halo_width_h: 0.012,
            halo_fringe: 0.035,
            ghosts: Vec::new(),
            ghost_fringe: 0.04,
            ghost_rim: 0.6,
            aperture_deg: 15.0,
            occlusion_radius_h: 0.012,
            offscreen_margin_ndc: 0.3,
            cloud_full_fade: 0.6,
            fade_s: 0.15,
        }
    }
}

impl Validate for SunLook {
    fn validate(&self) -> Result<(), String> {
        let b = &self.bloom;
        within("bloom.intensity", b.intensity, 0.0, 1.0)?;
        within("bloom.low_frequency_boost", b.low_frequency_boost, 0.0, 1.0)?;
        within("bloom.low_frequency_boost_curvature", b.low_frequency_boost_curvature, 0.0, 1.0)?;
        within("bloom.high_pass_frequency", b.high_pass_frequency, 0.0, 1.0)?;
        within("bloom.threshold", b.threshold, 0.0, 1000.0)?;
        within("bloom.threshold_softness", b.threshold_softness, 0.0, 1.0)?;
        let f = &self.flare;
        within("flare.gain", f.gain, 0.0, 100.0)?;
        for (field, c) in [
            ("flare.core_colour", f.core_colour),
            ("flare.glow_colour", f.glow_colour),
            ("flare.spike_colour", f.spike_colour),
            ("flare.streak_colour", f.streak_colour),
            ("flare.halo_colour", f.halo_colour),
        ] {
            colour(field, c)?;
        }
        for (field, v) in [
            ("flare.core_radius_h", f.core_radius_h),
            ("flare.glow_radius_h", f.glow_radius_h),
            ("flare.spike_length_h", f.spike_length_h),
            ("flare.ray_length_h", f.ray_length_h),
            ("flare.streak_length_h", f.streak_length_h),
            ("flare.streak_thickness_h", f.streak_thickness_h),
            ("flare.halo_radius_h", f.halo_radius_h),
            ("flare.halo_width_h", f.halo_width_h),
            ("flare.occlusion_radius_h", f.occlusion_radius_h),
        ] {
            positive(field, v)?;
            within(field, v, 0.0, 4.0)?;
        }
        within("flare.glow_power", f.glow_power, 0.1, 10.0)?;
        if !(1..=32).contains(&f.spike_count) {
            return Err(format!("`flare.spike_count` must be within 1 and 32, got {}", f.spike_count));
        }
        if f.ray_count > 128 {
            return Err(format!("`flare.ray_count` must be 128 or fewer, got {}", f.ray_count));
        }
        within("flare.spike_sharpness", f.spike_sharpness, 1.0, 4096.0)?;
        within("flare.ray_strength", f.ray_strength, 0.0, 10.0)?;
        within("flare.spike_turn_rad_per_h", f.spike_turn_rad_per_h, -20.0, 20.0)?;
        within("flare.spike_fringe", f.spike_fringe, 0.0, 0.5)?;
        within("flare.halo_fringe", f.halo_fringe, 0.0, 0.5)?;
        within("flare.ghost_fringe", f.ghost_fringe, 0.0, 0.5)?;
        within("flare.ghost_rim", f.ghost_rim, 0.0, 10.0)?;
        within("flare.aperture_deg", f.aperture_deg, -360.0, 360.0)?;
        within("flare.offscreen_margin_ndc", f.offscreen_margin_ndc, 0.0, 4.0)?;
        within("flare.cloud_full_fade", f.cloud_full_fade, 0.01, 1.0)?;
        within("flare.fade_s", f.fade_s, 0.0, 10.0)?;
        if f.ghosts.len() > MAX_GHOSTS {
            return Err(format!("`flare.ghosts` holds at most {MAX_GHOSTS}, got {}", f.ghosts.len()));
        }
        for g in &f.ghosts {
            within("flare.ghosts.offset", g.offset, -4.0, 4.0)?;
            positive("flare.ghosts.radius_h", g.radius_h)?;
            within("flare.ghosts.radius_h", g.radius_h, 0.0, 2.0)?;
            colour("flare.ghosts.colour", g.colour)?;
            within("flare.ghosts.softness", g.softness, 0.0, 1.0)?;
        }
        Ok(())
    }
}

/// A linear colour, finite and within 0..100 per channel.
fn colour(field: &str, c: [f32; 3]) -> Result<(), String> {
    finite3(field, c)?;
    for x in c {
        within(field, x, 0.0, 100.0)?;
    }
    Ok(())
}

/// Where the sun is on screen, in NDC: the direction toward the sun as a point at infinity through
/// `clip_from_world`. `None` when the sun is behind the eye (or exactly beside it).
pub fn sun_ndc(clip_from_world: Mat4, toward_sun: Vec3) -> Option<Vec2> {
    if !toward_sun.is_finite() || toward_sun.length_squared() < 1.0e-12 {
        return None;
    }
    let clip = clip_from_world * toward_sun.normalize().extend(0.0);
    if clip.w.is_nan() || clip.w <= 1.0e-6 {
        return None;
    }
    let ndc = Vec2::new(clip.x, clip.y) / clip.w;
    ndc.is_finite().then_some(ndc)
}

/// An NDC position in screen heights (origin at the centre, y up, one screen height is 1) for a
/// view `aspect` (width over height) wide.
pub fn screen_heights(ndc: Vec2, aspect: f32) -> Vec2 {
    Vec2::new(ndc.x * aspect * 0.5, ndc.y * 0.5)
}

/// How much of the flare is left as the sun leaves the view: 1 on screen, falling smoothly to 0
/// `margin` NDC past the edge (so the ghosts linger a little after the sun goes).
pub fn on_screen(ndc: Vec2, margin: f32) -> f32 {
    let past = (ndc.x.abs() - 1.0).max(ndc.y.abs() - 1.0).max(0.0);
    if margin <= 0.0 {
        return if past > 0.0 { 0.0 } else { 1.0 };
    }
    1.0 - smoothstep(0.0, margin, past)
}

/// The occlusion taps' offsets from the sun (screen heights) and weights: 17 points spread evenly
/// over a disc of `radius_h` on a golden-angle (Vogel) spiral, every tap weighed the same. The
/// spiral has no mirror symmetry, so a straight hiding edge crossing the disc meets the taps one
/// at a time and takes the flare down in small steps.
pub fn occlusion_taps(radius_h: f32) -> [(Vec2, f32); OCCLUSION_TAPS] {
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    std::array::from_fn(|i| {
        let r = radius_h * ((i as f32 + 0.5) / OCCLUSION_TAPS as f32).sqrt();
        (Vec2::from_angle(i as f32 * golden) * r, 1.0)
    })
}

/// The taps as the shader reads them, for the sun at `ndc` in a view `aspect` wide: xy the tap's uv
/// (0..1, y down), z its weight, w 1 when it falls on screen (an off-screen tap counts as clear).
pub fn tap_uvs(ndc: Vec2, aspect: f32, radius_h: f32) -> [Vec4; OCCLUSION_TAPS] {
    occlusion_taps(radius_h).map(|(offset, weight)| {
        let x = ndc.x + offset.x * 2.0 / aspect.max(1.0e-3);
        let y = ndc.y + offset.y * 2.0;
        let uv = Vec2::new(x * 0.5 + 0.5, 0.5 - y * 0.5);
        let inside = (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y);
        Vec4::new(uv.x, uv.y, weight, if inside { 1.0 } else { 0.0 })
    })
}

/// The share of the sun the depth samples leave clear, 0..1: each sample is its depth (`None` off
/// screen, which counts as clear) and its weight; a sample is clear where the depth is the sky's.
/// `tap_visibility` in `sun_flare.wesl` is this rule on the GPU.
pub fn depth_visibility(samples: &[(Option<f32>, f32)]) -> f32 {
    let total: f32 = samples.iter().map(|(_, w)| w.max(0.0)).sum();
    if total <= 0.0 {
        return 1.0;
    }
    let clear: f32 = samples
        .iter()
        .filter(|(depth, _)| depth.is_none_or(|d| d <= SKY_DEPTH))
        .map(|(_, w)| w.max(0.0))
        .sum();
    (clear / total).clamp(0.0, 1.0)
}

/// How much of the flare the air and water at the eye let through: none under water (`submersion`
/// 1), half while straddling the surface, and none in cloud as thick as `cloud_full_fade`.
pub fn medium_fade(submersion: f32, cloud_density: f32, cloud_full_fade: f32) -> f32 {
    let water = 1.0 - submersion.clamp(0.0, 1.0);
    let cloud = 1.0 - smoothstep(0.0, cloud_full_fade.max(1.0e-3), cloud_density.max(0.0));
    if water.is_finite() && cloud.is_finite() {
        water * cloud
    } else {
        0.0
    }
}

/// `current` moved toward `target` over `dt_s` seconds with time constant `tau_s` (0: at once).
pub fn fade_toward(current: f32, target: f32, dt_s: f32, tau_s: f32) -> f32 {
    if !current.is_finite() || tau_s <= 0.0 {
        return target;
    }
    let k = 1.0 - (-dt_s.max(0.0) / tau_s).exp();
    current + (target - current) * k
}

/// The flare's strength: the sun's illuminance (lux) times the camera's exposure (Bevy's
/// `Exposure::exposure`, the multiplier from lux to the view's linear units) times `gain`.
pub fn strength(illuminance_lux: f32, exposure: f32, gain: f32) -> f32 {
    let s = illuminance_lux * exposure * gain;
    if s.is_finite() { s.max(0.0) } else { 0.0 }
}

/// How far the starburst is turned, radians, with the sun at `sun_h` (screen heights): `turn_per_h`
/// times the sun's x plus y, so the spikes wheel slowly as the view swings the sun across the
/// screen, and never jump (the angle follows the sun's position, which moves smoothly).
pub fn starburst_angle(sun_h: Vec2, turn_per_h: f32) -> f32 {
    let a = turn_per_h * (sun_h.x + sun_h.y);
    if a.is_finite() { a } else { 0.0 }
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse_ron;
    use std::path::Path;

    /// Bevy's infinite reverse-Z perspective (`ExtractedView`'s doc) times a view at the origin
    /// looking down -z.
    fn camera(fov_y: f32, aspect: f32) -> Mat4 {
        let f = 1.0 / (fov_y / 2.0).tan();
        Mat4::from_cols(
            Vec4::new(f / aspect, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, -1.0),
            Vec4::new(0.0, 0.0, 0.05, 0.0),
        )
    }

    #[test]
    fn a_sun_behind_the_eye_has_no_place_on_screen() {
        let m = camera(75f32.to_radians(), 16.0 / 9.0);
        assert_eq!(sun_ndc(m, Vec3::Z), None);
        assert_eq!(sun_ndc(m, Vec3::new(0.3, 0.2, 1.0)), None);
        // Exactly beside the eye: nowhere on the image plane.
        assert_eq!(sun_ndc(m, Vec3::X), None);
        assert_eq!(sun_ndc(m, Vec3::ZERO), None);
        assert_eq!(sun_ndc(m, Vec3::NAN), None);
    }

    #[test]
    fn a_sun_straight_ahead_is_at_the_centre() {
        let m = camera(75f32.to_radians(), 16.0 / 9.0);
        let ndc = sun_ndc(m, Vec3::NEG_Z).unwrap();
        assert!(ndc.length() < 1.0e-6, "{ndc}");
    }

    #[test]
    fn a_sun_at_the_edge_of_the_view_is_at_the_edge_of_ndc() {
        let fov = 75f32.to_radians();
        let aspect = 16.0 / 9.0;
        let m = camera(fov, aspect);
        // Half the vertical field up: the top edge.
        let up = Vec3::new(0.0, (fov / 2.0).sin(), -(fov / 2.0).cos());
        let ndc = sun_ndc(m, up).unwrap();
        assert!((ndc.y - 1.0).abs() < 1.0e-5 && ndc.x.abs() < 1.0e-6, "{ndc}");
        // Right by the horizontal half field: the right edge.
        let half_w = ((fov / 2.0).tan() * aspect).atan();
        let right = Vec3::new(half_w.sin(), 0.0, -half_w.cos());
        let ndc = sun_ndc(m, right).unwrap();
        assert!((ndc.x - 1.0).abs() < 1.0e-5 && ndc.y.abs() < 1.0e-6, "{ndc}");
    }

    #[test]
    fn the_sun_follows_the_view_not_the_world() {
        // A view turned 90 degrees left (looking down -x) sees a sun in -x straight ahead.
        let view = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let m = camera(1.0, 1.5) * view.inverse();
        let ndc = sun_ndc(m, Vec3::NEG_X).unwrap();
        assert!(ndc.length() < 1.0e-5, "{ndc}");
        // The eye's position does not move a point at infinity.
        let moved = camera(1.0, 1.5) * (Mat4::from_translation(Vec3::new(500.0, 30.0, -900.0)) * view).inverse();
        assert!((sun_ndc(moved, Vec3::NEG_X).unwrap() - ndc).length() < 1.0e-4);
    }

    #[test]
    fn screen_heights_keep_pixels_square() {
        let p = screen_heights(Vec2::new(1.0, 1.0), 16.0 / 9.0);
        assert!((p.x - 8.0 / 9.0).abs() < 1.0e-6 && (p.y - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn the_flare_fades_smoothly_off_the_edge() {
        assert_eq!(on_screen(Vec2::new(0.99, -0.5), 0.3), 1.0);
        let half = on_screen(Vec2::new(1.15, 0.0), 0.3);
        assert!((half - 0.5).abs() < 1.0e-5, "{half}");
        assert_eq!(on_screen(Vec2::new(0.0, -1.3), 0.3), 0.0);
        assert_eq!(on_screen(Vec2::new(2.0, 0.0), 0.0), 0.0);
        // Never a step: a small move gives a small change.
        let mut last = 1.0;
        for i in 0..=100 {
            let v = on_screen(Vec2::new(1.0 + i as f32 * 0.004, 0.0), 0.3);
            assert!(last - v < 0.03 && v <= last);
            last = v;
        }
    }

    #[test]
    fn every_tap_sky_is_full_and_every_tap_hidden_is_none() {
        let taps = occlusion_taps(0.01);
        let sky: Vec<_> = taps.iter().map(|(_, w)| (Some(0.0), *w)).collect();
        let wall: Vec<_> = taps.iter().map(|(_, w)| (Some(0.3), *w)).collect();
        assert_eq!(depth_visibility(&sky), 1.0);
        assert_eq!(depth_visibility(&wall), 0.0);
    }

    #[test]
    fn an_edge_sliding_over_the_sun_fades_the_flare_in_small_steps() {
        // A wall's edge sweeps left to right across the taps: everything left of it is hidden.
        let taps = occlusion_taps(0.01);
        let mut last = 1.0;
        let mut seen = Vec::new();
        for i in 0..=40 {
            let edge = -0.0105 + i as f32 * 0.000525;
            let samples: Vec<_> = taps.iter().map(|(o, w)| (Some(if o.x < edge { 0.5 } else { 0.0 }), *w)).collect();
            let v = depth_visibility(&samples);
            assert!(v <= last + 1.0e-6, "visibility rose as more was hidden");
            assert!(last - v <= 2.0 / 17.0 + 1.0e-6, "a jump of {} at step {i}", last - v);
            last = v;
            seen.push(v);
        }
        assert_eq!(seen[0], 1.0);
        assert_eq!(*seen.last().unwrap(), 0.0);
        // It passes through the middle, not just its ends.
        assert!(seen.iter().any(|v| (0.3..0.7).contains(v)));
    }

    #[test]
    fn taps_off_screen_count_as_clear() {
        let samples = [(None, 1.0), (Some(0.2), 1.0)];
        assert_eq!(depth_visibility(&samples), 0.5);
        assert_eq!(depth_visibility(&[]), 1.0);
    }

    #[test]
    fn tap_uvs_centre_on_the_sun_and_mark_the_ones_off_screen() {
        let taps = tap_uvs(Vec2::ZERO, 16.0 / 9.0, 0.01);
        assert!(taps.iter().all(|t| t.w == 1.0 && t.z == 1.0));
        // The taps sit within 0.01 screen heights of the centre: 0.01 of the height in uv.
        for t in taps {
            let h = Vec2::new((t.x - 0.5) * 16.0 / 9.0, t.y - 0.5);
            assert!(h.length() <= 0.01 + 1.0e-5, "{h}");
        }
        assert!(taps.iter().any(|t| Vec2::new((t.x - 0.5) * 16.0 / 9.0, t.y - 0.5).length() > 0.009));
        let edge = tap_uvs(Vec2::new(1.0, 0.0), 1.5, 0.01);
        assert!(edge.iter().any(|t| t.w == 0.0) && edge.iter().any(|t| t.w == 1.0));
    }

    #[test]
    fn water_and_cloud_at_the_eye_put_the_flare_out() {
        assert_eq!(medium_fade(0.0, 0.0, 0.6), 1.0);
        assert_eq!(medium_fade(1.0, 0.0, 0.6), 0.0);
        assert_eq!(medium_fade(0.5, 0.0, 0.6), 0.5);
        assert_eq!(medium_fade(0.0, 0.6, 0.6), 0.0);
        let thin = medium_fade(0.0, 0.1, 0.6);
        assert!(thin > 0.8 && thin < 1.0);
        assert_eq!(medium_fade(f32::NAN, 0.0, 0.6), 0.0);
    }

    #[test]
    fn the_fade_follows_without_a_jump() {
        let mut v = 1.0;
        v = fade_toward(v, 0.0, 1.0 / 60.0, 0.15);
        assert!(v < 1.0 && v > 0.85, "{v}");
        for _ in 0..120 {
            v = fade_toward(v, 0.0, 1.0 / 60.0, 0.15);
        }
        assert!(v < 1.0e-3);
        assert_eq!(fade_toward(0.3, 1.0, 0.016, 0.0), 1.0);
        assert_eq!(fade_toward(f32::NAN, 0.4, 0.016, 0.1), 0.4);
    }

    #[test]
    fn strength_scales_with_the_sun_and_the_exposure() {
        // Raw sunlight at EV100 13.4: about 9.25 exposed lux.
        let exposure = (-13.4f32).exp2() / 1.2;
        let s = strength(120_000.0, exposure, 1.0);
        assert!((s - 9.25).abs() < 0.05, "{s}");
        assert!((strength(60_000.0, exposure, 1.0) - s / 2.0).abs() < 1.0e-3);
        assert!((strength(120_000.0, exposure * 2.0, 1.0) - s * 2.0).abs() < 1.0e-3);
        assert_eq!(strength(f32::INFINITY, exposure, 1.0), 0.0);
    }

    #[test]
    fn the_starburst_turns_with_the_view() {
        assert_eq!(starburst_angle(Vec2::ZERO, 0.6), 0.0);
        assert!((starburst_angle(Vec2::new(0.5, 0.0), 0.6) - 0.3).abs() < 1.0e-6);
        assert!((starburst_angle(Vec2::new(0.25, 0.25), 0.6) - 0.3).abs() < 1.0e-6);
        // A small swing of the view turns it a little, never a jump.
        let a = starburst_angle(Vec2::new(0.300, -0.1), 0.6);
        let b = starburst_angle(Vec2::new(0.301, -0.1), 0.6);
        assert!((a - b).abs() < 1.0e-3);
        assert_eq!(starburst_angle(Vec2::new(f32::NAN, 0.0), 0.6), 0.0);
    }

    #[test]
    fn the_data_file_loads_and_validates() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/sun_flare.ron");
        let look: SunLook = crate::data::load_ron(&path).unwrap();
        assert!(!look.flare.ghosts.is_empty());
        assert!(look.flare.ghosts.iter().any(|g| g.shape == GhostShape::Hex));
        assert!(look.flare.ghosts.iter().any(|g| g.shape == GhostShape::Disc));
    }

    #[test]
    fn bad_flare_data_is_refused_by_field() {
        let origin = Path::new("sun_flare.ron");
        let err = |text: &str| parse_ron::<SunLook>(text, origin).unwrap_err().reason;
        assert!(err("(flare: (gain: -1.0))").contains("flare.gain"));
        assert!(err("(flare: (spike_count: 0))").contains("flare.spike_count"));
        assert!(err("(bloom: (intensity: 2.0))").contains("bloom.intensity"));
        assert!(err("(flare: (ghosts: [(radius_h: 0.0)]))").contains("flare.ghosts.radius_h"));
        assert!(err("(flare: (sparkle: 1.0))").contains("sparkle"));
        let nine = format!("(flare: (ghosts: [{}]))", ["()"; 9].join(", "));
        assert!(err(&nine).contains("flare.ghosts"));
        // Empty takes every default.
        assert_eq!(parse_ron::<SunLook>("()", origin).unwrap(), SunLook::default());
    }
}
