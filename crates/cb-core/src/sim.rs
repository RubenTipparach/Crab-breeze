//! The simulation clock, loaded from `data/sim.ron`: the fixed step every rule advances by.

use crate::data::{Validate, within};
use serde::Deserialize;

/// Simulation timing.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SimConfig {
    /// Fixed steps per second.
    pub fixed_step_hz: f32,
    /// Most frames drawn a second (owner, 2026-10-24: "need frame limiter 120 fps max"); 0 is
    /// uncapped. The benchmark ignores it.
    pub frame_cap_hz: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            fixed_step_hz: 60.0,
            frame_cap_hz: 120.0,
        }
    }
}

impl Validate for SimConfig {
    fn validate(&self) -> Result<(), String> {
        within("fixed_step_hz", self.fixed_step_hz, 10.0, 1000.0)?;
        within("frame_cap_hz", self.frame_cap_hz, 0.0, 1000.0)
    }
}
