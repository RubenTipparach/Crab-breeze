//! How the water looks and how wet the pilot's eye is (openspec change `water-rendering`).
//!
//! Two things live here because they are plain numbers and rules with no engine in them: the
//! water's look tuning (`data/world/water_look.ron`, loaded by the one data loader so `cb-tools
//! validate` checks it), and the eye's state machine (under, straddling or dry, and the drops that
//! run off the glass after surfacing), which is tested headless. `cb-app`'s `water_render` feeds
//! both to the GPU.
//!
//! Ported from Pale Blue Dot (`RubenTipparach/pale-blue-dot` at `db25fe2`,
//! `crates/pbd-app/src/planet_water.rs`: `submersion` and `emerge`; `assets/config/water.ron` and
//! `weather.ron` for the defaults), flattened from a planet to a plane.

use crate::data::{Validate, finite3, positive, within};
use serde::Deserialize;

/// The water's look. Every field is in `data/world/water_look.ron`; a missing field takes the
/// default below (Pale Blue Dot's value unless the design says otherwise).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WaterLook {
    /// How fast the ripples' noise evolves and moves, a multiplier on seconds.
    pub time_scale: f32,
    /// Ripple noise cells per metre (higher: smaller ripples).
    pub ripple_scale_per_m: f32,
    /// How fast the ripple noise evolves, a multiplier on the scaled time.
    pub ripple_speed: f32,
    /// How strongly the ripples bend the normal.
    pub ripple_steepness: f32,
    /// The largest ripple slope the normal takes (the cap that keeps the sheet from turning to a
    /// mirror).
    pub slope_max: f32,
    /// How fast ripples fade where the noise falls under a pixel (0: never).
    pub detail_fade: f32,
    /// Seconds per phase of the flow map on rivers.
    pub flow_period_s: f32,
    /// The swell's wavelengths, metres, longest first. The swell bends the normal only.
    pub swell_wavelength_m: [f32; 3],
    /// The swell's heights (crest to mean), metres.
    pub swell_height_m: [f32; 3],
    /// The swell's headings, degrees from +x toward +z.
    pub swell_heading_deg: [f32; 3],
    /// How far the ripples shift the refracted image, uv per unit of slope.
    pub refract_amount: f32,
    /// The largest refraction shift, uv.
    pub refract_max_uv: f32,
    /// The longest path through water the colour is absorbed over, metres.
    pub max_path_m: f32,
    /// How much of the light reaching an underwater surface is lost on its way down (1: all of the
    /// water above it absorbs).
    pub light_falloff: f32,
    /// Light absorbed per metre of water, red, green, blue.
    pub absorption_per_m: [f32; 3],
    /// The water body's colour, lit like an albedo by the light reaching it.
    pub deep_color: [f32; 3],
    /// The sky's colour straight up, linear, for the reflection (the horizon is the game's sky).
    pub sky_zenith_color: [f32; 3],
    /// The least reflection at a grazing angle, 0..1.
    pub sky_horizon_strength: f32,
    /// Foam's colour, an albedo.
    pub foam_color: [f32; 3],
    /// Foam's overall strength, 0..1.
    pub foam_intensity: f32,
    /// Ripple heights where crest foam starts and is full.
    pub foam_crest: [f32; 2],
    /// Crest foam's weight.
    pub foam_crest_weight: f32,
    /// Ripple slopes where slope foam starts and is full.
    pub foam_slope: [f32; 2],
    /// Slope foam's weight.
    pub foam_slope_weight: f32,
    /// Water shallower than this foams along the shore, metres.
    pub shore_foam_m: f32,
    /// Shore foam's weight.
    pub shore_foam_weight: f32,
    /// The sun's glint, a multiplier on the sun's light.
    pub specular_intensity: f32,
    /// The glint's sharpness (Blinn-Phong exponent).
    pub specular_power: f32,
    /// The glint's tint.
    pub sun_tint: [f32; 3],
    /// How far the view wobbles under water, uv.
    pub underwater_distortion: f32,
    /// How much the distance softens while the glass is wet, 0..1.
    pub wet_blur: f32,
    /// Half the height of the band about the surface where the eye straddles it, metres.
    pub partial_band_m: f32,
    /// How long the glass takes to clear after surfacing, seconds.
    pub emerge_dry_s: f32,
    /// Seated, the sealed cabin is dry this far from the eye: no murk, no surface, no drops on it,
    /// metres.
    pub cabin_dry_radius_m: f32,
    /// Drops on the glass: density (above 1 stacks layers), 0..3.
    pub lens_density: f32,
    /// How strongly a drop bends the image behind it.
    pub lens_refract: f32,
    /// How fast the drops run.
    pub lens_speed: f32,
    /// The drops' grid size.
    pub lens_size: f32,
    /// The drops this many times smaller than Tenebris's.
    pub lens_scale: f32,
    /// Rain on the glass, 0..1. Zero until weather exists (the code path is kept, gated).
    pub lens_rain: f32,
    /// Seated, metres of window pane to one unit of the drop pattern, so the drops sit on each pane
    /// at about the size they had on the screen (owner, 2026-10-25: "the rain drops on the cocpit
    /// are not projected to the panes of window correctly").
    pub lens_pane_m: f32,
    /// Rain on the glass inside a cloud at full density, 0..1 (openspec change `cloud-rain`).
    pub cloud_rain: f32,
    /// Seconds for the glass to wet through on entering a cloud.
    pub cloud_wet_s: f32,
    /// Seconds for the drops to clear after leaving one.
    pub cloud_dry_s: f32,
}

impl Default for WaterLook {
    fn default() -> Self {
        Self {
            time_scale: 0.75,
            ripple_scale_per_m: 0.6,
            ripple_speed: 1.0,
            ripple_steepness: 0.45,
            slope_max: 0.7,
            detail_fade: 4.0,
            flow_period_s: 4.0,
            swell_wavelength_m: [60.0, 23.0, 9.0],
            swell_height_m: [0.35, 0.12, 0.05],
            swell_heading_deg: [20.0, 75.0, -40.0],
            refract_amount: 0.04,
            refract_max_uv: 0.03,
            max_path_m: 200.0,
            light_falloff: 1.0,
            absorption_per_m: [0.45, 0.12, 0.06],
            deep_color: [0.0, 0.12, 0.28],
            sky_zenith_color: [0.03, 0.18, 0.55],
            sky_horizon_strength: 0.22,
            foam_color: [0.95, 0.97, 1.0],
            foam_intensity: 0.10,
            foam_crest: [0.35, 0.60],
            foam_crest_weight: 0.55,
            foam_slope: [0.50, 1.20],
            foam_slope_weight: 0.26,
            shore_foam_m: 1.5,
            shore_foam_weight: 0.6,
            specular_intensity: 0.06,
            specular_power: 140.0,
            sun_tint: [1.10, 1.15, 1.20],
            underwater_distortion: 0.0015,
            wet_blur: 0.02,
            partial_band_m: 0.8,
            emerge_dry_s: 2.6,
            cabin_dry_radius_m: 2.5,
            lens_density: 1.4,
            lens_refract: 1.7,
            lens_speed: 1.0,
            lens_size: 0.7,
            lens_scale: 2.0,
            lens_rain: 0.0,
            lens_pane_m: 1.8,
            cloud_rain: 0.85,
            cloud_wet_s: 1.2,
            cloud_dry_s: 4.0,
        }
    }
}

/// Requires `[lo, hi]` with `lo < hi`, both finite.
fn rising(field: &str, r: [f32; 2]) -> Result<(), String> {
    within(field, r[0], -1.0e6, 1.0e6)?;
    within(field, r[1], -1.0e6, 1.0e6)?;
    if r[0] < r[1] {
        Ok(())
    } else {
        Err(format!("`{field}` must be [low, high] with low < high, got {r:?}"))
    }
}

impl Validate for WaterLook {
    fn validate(&self) -> Result<(), String> {
        positive("time_scale", self.time_scale)?;
        positive("ripple_scale_per_m", self.ripple_scale_per_m)?;
        within("ripple_speed", self.ripple_speed, 0.0, 100.0)?;
        within("ripple_steepness", self.ripple_steepness, 0.0, 10.0)?;
        positive("slope_max", self.slope_max)?;
        within("detail_fade", self.detail_fade, 0.0, 1000.0)?;
        positive("flow_period_s", self.flow_period_s)?;
        for i in 0..3 {
            positive("swell_wavelength_m", self.swell_wavelength_m[i])?;
            within("swell_height_m", self.swell_height_m[i], 0.0, 50.0)?;
            within("swell_heading_deg", self.swell_heading_deg[i], -360.0, 360.0)?;
        }
        within("refract_amount", self.refract_amount, 0.0, 1.0)?;
        within("refract_max_uv", self.refract_max_uv, 0.0, 0.5)?;
        positive("max_path_m", self.max_path_m)?;
        within("light_falloff", self.light_falloff, 0.0, 1.0)?;
        finite3("absorption_per_m", self.absorption_per_m)?;
        for a in self.absorption_per_m {
            within("absorption_per_m", a, 0.0, 100.0)?;
        }
        for (field, c) in [
            ("deep_color", self.deep_color),
            ("sky_zenith_color", self.sky_zenith_color),
            ("foam_color", self.foam_color),
            ("sun_tint", self.sun_tint),
        ] {
            for x in c {
                within(field, x, 0.0, 10.0)?;
            }
        }
        within("sky_horizon_strength", self.sky_horizon_strength, 0.0, 1.0)?;
        within("foam_intensity", self.foam_intensity, 0.0, 1.0)?;
        rising("foam_crest", self.foam_crest)?;
        within("foam_crest_weight", self.foam_crest_weight, 0.0, 1.0)?;
        rising("foam_slope", self.foam_slope)?;
        within("foam_slope_weight", self.foam_slope_weight, 0.0, 1.0)?;
        within("shore_foam_m", self.shore_foam_m, 0.0, 100.0)?;
        within("shore_foam_weight", self.shore_foam_weight, 0.0, 1.0)?;
        within("specular_intensity", self.specular_intensity, 0.0, 100.0)?;
        within("specular_power", self.specular_power, 1.0, 10000.0)?;
        within("underwater_distortion", self.underwater_distortion, 0.0, 0.1)?;
        within("wet_blur", self.wet_blur, 0.0, 1.0)?;
        positive("partial_band_m", self.partial_band_m)?;
        positive("emerge_dry_s", self.emerge_dry_s)?;
        within("cabin_dry_radius_m", self.cabin_dry_radius_m, 0.0, 50.0)?;
        within("lens_density", self.lens_density, 0.0, 3.0)?;
        within("lens_refract", self.lens_refract, 0.0, 10.0)?;
        within("lens_speed", self.lens_speed, 0.0, 10.0)?;
        positive("lens_size", self.lens_size)?;
        positive("lens_scale", self.lens_scale)?;
        within("lens_rain", self.lens_rain, 0.0, 1.0)?;
        positive("lens_pane_m", self.lens_pane_m)?;
        within("cloud_rain", self.cloud_rain, 0.0, 1.0)?;
        positive("cloud_wet_s", self.cloud_wet_s)?;
        positive("cloud_dry_s", self.cloud_dry_s)
    }
}

/// The eye fully under water.
pub const UNDER: f32 = 1.0;
/// The eye within the band about the surface.
pub const STRADDLING: f32 = 0.5;
/// The eye in air.
pub const DRY: f32 = 0.0;

/// Which side of the surface the eye is on, from how far it is below the water surface over it
/// (`None`: no water there, dry): [`UNDER`], [`STRADDLING`] within `band_m` of the surface, or
/// [`DRY`]. Pale Blue Dot's tri-state; the band is wide enough that the swell, which is drawn in
/// the normals only, never needs asking.
pub fn submersion(eye_depth_m: Option<f32>, band_m: f32) -> f32 {
    match eye_depth_m {
        Some(d) if d.is_finite() && d > band_m => UNDER,
        Some(d) if d.is_finite() && d >= -band_m => STRADDLING,
        _ => DRY,
    }
}

/// The glass's memory of the water: the emerge window Pale Blue Dot keeps per view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lens {
    /// Whether the eye was under water at the last step.
    pub was_under: bool,
    /// When the glass will be dry, seconds on the caller's clock.
    pub dry_at_s: f32,
}

impl Default for Lens {
    fn default() -> Self {
        Self {
            was_under: false,
            dry_at_s: f32::NEG_INFINITY,
        }
    }
}

impl Lens {
    /// Steps the glass to `now_s` with the eye at `submersion`, and returns how strongly drops run
    /// on it, 1 down to 0. Surfacing arms the window `dry_s` ahead and straddling keeps it topped
    /// up, so the glass stays wet until it clears; under water there are no drops.
    pub fn step(&mut self, now_s: f32, submersion: f32, dry_s: f32) -> f32 {
        let under = submersion > 0.75;
        let straddling = submersion > 0.25 && !under;
        if (self.was_under && !under) || straddling {
            self.dry_at_s = now_s + dry_s;
        }
        self.was_under = under;
        if under {
            0.0
        } else {
            ((self.dry_at_s - now_s) / dry_s.max(1e-6)).clamp(0.0, 1.0)
        }
    }
}

/// Rain on the glass from cloud (openspec change `cloud-rain`): the glass wets toward `cloud_rain`
/// x the cloud's density at the eye over `cloud_wet_s`, and clears over `cloud_dry_s` once the
/// eye is out. `prev` is last step's value; returns this step's, 0..1. Pure, so the lens pass and
/// any test read the same rule.
pub fn cloud_glass(prev: f32, density: f32, dt_s: f32, look: &WaterLook) -> f32 {
    let target = (look.cloud_rain * density.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let prev = if prev.is_finite() { prev.clamp(0.0, 1.0) } else { 0.0 };
    let dt = if dt_s.is_finite() { dt_s.max(0.0) } else { 0.0 };
    if target > prev {
        (prev + dt / look.cloud_wet_s.max(1e-3)).min(target)
    } else {
        (prev - dt / look.cloud_dry_s.max(1e-3)).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse_ron;
    use std::path::Path;

    #[test]
    fn the_shipped_water_look_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/water_look.ron");
        let look: WaterLook = crate::data::load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        look.validate().unwrap();
    }

    #[test]
    fn a_negative_dry_time_is_rejected_naming_the_field() {
        let path = Path::new("data/world/water_look.ron");
        let e = parse_ron::<WaterLook>("(emerge_dry_s: -1.0)", path).unwrap_err().to_string();
        assert!(e.contains("data/world/water_look.ron"), "{e}");
        assert!(e.contains("emerge_dry_s"), "{e}");
    }

    #[test]
    fn flying_into_a_cloud_wets_the_glass_and_leaving_it_dries_it() {
        let look = WaterLook::default();
        let mut wet = 0.0;
        for _ in 0..120 {
            wet = cloud_glass(wet, 1.0, 1.0 / 60.0, &look);
        }
        assert!(
            (wet - look.cloud_rain).abs() < 1e-4,
            "two seconds in a thick cloud wets the glass through ({wet})"
        );
        let mut t = 0.0;
        while wet > 0.0 {
            wet = cloud_glass(wet, 0.0, 1.0 / 60.0, &look);
            t += 1.0 / 60.0;
        }
        assert!(
            (t - look.cloud_rain * look.cloud_dry_s).abs() < 0.05,
            "it clears in the drying time ({t} s)"
        );
    }

    #[test]
    fn clear_air_leaves_dry_glass_dry_and_bad_numbers_change_nothing() {
        let look = WaterLook::default();
        assert_eq!(cloud_glass(0.0, 0.0, 1.0, &look), 0.0);
        assert_eq!(cloud_glass(f32::NAN, f32::NAN, f32::NAN, &look), 0.0);
        assert!(
            cloud_glass(0.0, 0.3, 10.0, &look) <= look.cloud_rain * 0.3 + 1e-6,
            "a thin cloud wets it less"
        );
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        assert!(parse_ron::<WaterLook>("(foam_colour: (1.0, 1.0, 1.0))", Path::new("x.ron")).is_err());
    }

    #[test]
    fn the_eye_is_under_straddling_or_dry() {
        let band = 0.8;
        assert_eq!(submersion(Some(3.0), band), UNDER);
        assert_eq!(submersion(Some(0.5), band), STRADDLING);
        assert_eq!(submersion(Some(-0.5), band), STRADDLING);
        assert_eq!(submersion(Some(-2.0), band), DRY);
        assert_eq!(submersion(None, band), DRY, "no water under the eye is dry");
        assert_eq!(submersion(Some(f32::NAN), band), DRY, "a bad number is dry, never drowned");
    }

    #[test]
    fn the_glass_dries_after_surfacing() {
        let dry = 2.6;
        let mut lens = Lens::default();
        assert_eq!(lens.step(0.0, UNDER, dry), 0.0, "no drops under water");
        assert_eq!(lens.step(1.0, DRY, dry), 1.0, "full drops on surfacing");
        let half = lens.step(1.0 + dry / 2.0, DRY, dry);
        assert!((half - 0.5).abs() < 1e-5, "half way: {half}");
        assert_eq!(lens.step(1.0 + dry + 0.1, DRY, dry), 0.0, "dry after the window");
    }

    #[test]
    fn straddling_keeps_the_glass_wet() {
        let dry = 2.6;
        let mut lens = Lens::default();
        lens.step(0.0, UNDER, dry);
        assert_eq!(lens.step(1.0, STRADDLING, dry), 1.0);
        assert_eq!(lens.step(10.0, STRADDLING, dry), 1.0, "still wet while straddling");
        assert_eq!(lens.step(10.0 + dry, DRY, dry), 0.0);
    }

    #[test]
    fn diving_again_clears_the_drops_until_the_next_surfacing() {
        let dry = 2.6;
        let mut lens = Lens::default();
        lens.step(0.0, UNDER, dry);
        assert_eq!(lens.step(1.0, DRY, dry), 1.0);
        assert_eq!(lens.step(1.5, UNDER, dry), 0.0, "under again: no drops");
        assert_eq!(lens.step(20.0, DRY, dry), 1.0, "a fresh window on surfacing");
    }

    #[test]
    fn a_dry_eye_that_never_dived_has_no_drops() {
        let mut lens = Lens::default();
        assert_eq!(lens.step(5.0, DRY, 2.6), 0.0);
    }
}
