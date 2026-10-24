//! The cockpit's own light (openspec change `cockpit-lighting`): the sealed cabin is lit by its
//! lamps and its screens, never by the world outside. Data only, loaded from
//! `data/hud/cockpit_light.ron`; `cb-app` places the lights on the cockpit pass.

use crate::data::{Validate, finite3, positive, within};
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
        }
    }
}

impl Validate for CockpitLightConfig {
    fn validate(&self) -> Result<(), String> {
        finite3("cabin_ambient_colour", self.cabin_ambient_colour)?;
        within("cabin_ambient_cd_m2", self.cabin_ambient_cd_m2, 0.0, 100_000.0)?;
        positive("screen_lumens", self.screen_lumens)?;
        positive("screen_range_m", self.screen_range_m)?;
        finite3("screen_tint", self.screen_tint)?;
        for (i, p) in self.screens_m.iter().enumerate() {
            finite3(&format!("screens_m[{i}]"), *p)?;
        }
        Ok(())
    }
}
