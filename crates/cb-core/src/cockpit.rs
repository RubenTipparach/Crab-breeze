//! The cockpit's own light (openspec change `cockpit-lighting`): the sealed cabin is lit by its
//! lamps and its screens, never by the world outside. Data only, loaded from
//! `data/hud/cockpit_light.ron`; `cb-app` places the lights on the cockpit pass.
//!
//! The screens light the cabin as one directional light, not a point light each: Bevy 0.20's GPU
//! light clustering ignores render layers, so a clustered cockpit pass picked up every gun flash
//! and blast outside and strobed with the guns. Directional lights are kept to their layer.

use crate::data::{Validate, finite3, positive, within};
use glam::Vec3;
use serde::Deserialize;

/// `data/hud/cockpit_light.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CockpitLightConfig {
    /// The cabin lamps' ambient colour, sRGB 0..1.
    pub cabin_ambient_colour: [f32; 3],
    /// Its brightness, cd/m2.
    pub cabin_ambient_cd_m2: f32,
    /// Each screen's glow, lumens.
    pub screen_lumens: f32,
    /// How far a screen's glow reaches.
    pub screen_range_m: f32,
    /// The screens' phosphor tint, sRGB 0..1.
    pub screen_tint: [f32; 3],
    /// Where each screen's glow sits from the pilot's eye: x right, y up, z back (Bevy's camera space).
    pub screens_m: Vec<[f32; 3]>,
    /// The screens' cube map: pixels along a face's side (a power of two); 0 turns it off.
    pub screens_cube_px: u32,
    /// How strongly the screens' cube map lights the cabin, cd/m2.
    pub screens_cube_cd_m2: f32,
    /// Whole refreshes of the cube a second (six faces each), drawn one face at a time.
    pub screens_cube_hz: f32,
}

impl Default for CockpitLightConfig {
    fn default() -> Self {
        Self {
            cabin_ambient_colour: [0.9, 0.82, 0.72],
            cabin_ambient_cd_m2: 120.0,
            screen_lumens: 200.0,
            screen_range_m: 4.0,
            screen_tint: [0.75, 0.88, 1.0],
            screens_m: vec![[0.0, 0.0, -1.2], [-1.0, 0.0, -0.6], [1.0, 0.0, -0.6]],
            screens_cube_px: 64,
            screens_cube_cd_m2: 400.0,
            screens_cube_hz: 10.0,
        }
    }
}

impl CockpitLightConfig {
    /// The screens' glow as one light: the direction it travels in camera space (from the screens'
    /// mean position towards the eye) and its illuminance at the eye, lux (each screen's lumens
    /// spread over a sphere at its distance, summed). `None` with no screens.
    pub fn screen_glow(&self) -> Option<(Vec3, f32)> {
        let at: Vec<Vec3> = self.screens_m.iter().map(|p| Vec3::from_array(*p)).collect();
        if at.is_empty() {
            return None;
        }
        let mean = at.iter().copied().sum::<Vec3>() / at.len() as f32;
        let travel = (-mean).try_normalize().unwrap_or(Vec3::Z);
        let lux = at
            .iter()
            .map(|p| self.screen_lumens / (4.0 * std::f32::consts::PI * p.length_squared().max(0.25)))
            .sum();
        Some((travel, lux))
    }
}

impl Validate for CockpitLightConfig {
    fn validate(&self) -> Result<(), String> {
        finite3("cabin_ambient_colour", self.cabin_ambient_colour)?;
        within("cabin_ambient_cd_m2", self.cabin_ambient_cd_m2, 0.0, 100_000.0)?;
        positive("screen_lumens", self.screen_lumens)?;
        positive("screen_range_m", self.screen_range_m)?;
        finite3("screen_tint", self.screen_tint)?;
        if self.screens_cube_px != 0 && !self.screens_cube_px.is_power_of_two() {
            return Err(format!(
                "`screens_cube_px` must be a power of two or 0, got {}",
                self.screens_cube_px
            ));
        }
        within("screens_cube_cd_m2", self.screens_cube_cd_m2, 0.0, 100_000.0)?;
        positive("screens_cube_hz", self.screens_cube_hz)?;
        for (i, p) in self.screens_m.iter().enumerate() {
            finite3(&format!("screens_m[{i}]"), *p)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screens_glow_travels_from_the_screens_towards_the_eye() {
        let cfg = CockpitLightConfig::default();
        let (travel, lux) = cfg.screen_glow().unwrap();
        // The screens are ahead (-z), so their light travels back (+z) to the pilot.
        assert!(travel.z > 0.9, "{travel:?}");
        assert!(lux > 0.0 && lux.is_finite());
    }

    #[test]
    fn no_screens_give_no_glow() {
        let cfg = CockpitLightConfig {
            screens_m: vec![],
            ..Default::default()
        };
        assert!(cfg.screen_glow().is_none());
    }
}
