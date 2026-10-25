//! The player's settings (openspec change `game-menus`, design section 4): mouse sensitivity and
//! invert, field of view, shadows, vsync and fullscreen. One record, defaulted from the data's own
//! values, changed on the Options page and saved by the game to `settings.ron`. Pure: the steps the
//! Options page takes and the limits they keep to live here so they are tested without a window.

use crate::data::{Validate, within};
use serde::{Deserialize, Serialize};

/// Mouse sensitivity limits, degrees per pixel, and the Options page's step.
pub const MOUSE_DEG_PER_PX: (f32, f32, f32) = (0.01, 1.0, 0.01);
/// Vertical field of view limits, degrees, and the Options page's step.
pub const FOV_DEG: (f32, f32, f32) = (60.0, 110.0, 5.0);

/// The player's settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Mouse look, degrees per pixel of mouse motion.
    pub mouse_deg_per_px: f32,
    /// Mouse up looks down.
    pub invert_pitch: bool,
    /// The view's vertical field of view, degrees (on foot and seated).
    pub fov_deg: f32,
    /// The sun casts shadows.
    pub shadows: bool,
    /// Frames wait for the display's refresh.
    pub vsync: bool,
    /// Borderless fullscreen instead of a window.
    pub fullscreen: bool,
}

impl Settings {
    /// The defaults: the data's own mouse, invert and field of view, shadows and vsync on, windowed.
    pub fn from_data(bindings: &crate::input::Bindings, pilot: &crate::pilot::PilotConfig) -> Self {
        Self {
            mouse_deg_per_px: bindings.mouse_deg_per_px.clamp(MOUSE_DEG_PER_PX.0, MOUSE_DEG_PER_PX.1),
            invert_pitch: bindings.invert_pitch,
            fov_deg: pilot.view_fov_deg.clamp(FOV_DEG.0, FOV_DEG.1),
            shadows: true,
            vsync: true,
            fullscreen: false,
        }
    }

    /// Mouse sensitivity up (`steps` > 0) or down by the page's step, kept within its limits.
    pub fn nudge_mouse(&mut self, steps: i32) {
        self.mouse_deg_per_px = nudge(self.mouse_deg_per_px, steps, MOUSE_DEG_PER_PX);
    }

    /// Field of view up or down by the page's step, kept within its limits.
    pub fn nudge_fov(&mut self, steps: i32) {
        self.fov_deg = nudge(self.fov_deg, steps, FOV_DEG);
    }
}

impl Settings {
    /// The settings as the text of `settings.ron`.
    pub fn to_ron(&self) -> String {
        let body = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap_or_default();
        format!("// Your settings, written by the Options page (openspec change game-menus). Delete this file for the defaults.\n{body}\n")
    }

    /// Settings read from the text of `settings.ron`, checked; an error names what is wrong.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let s: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        s.validate()?;
        Ok(s)
    }
}

/// `v` moved by `steps` of `step`, rounded to the step (so repeated presses do not drift) and kept
/// within `lo..=hi`.
fn nudge(v: f32, steps: i32, (lo, hi, step): (f32, f32, f32)) -> f32 {
    let n = (v / step).round() + steps as f32;
    (n * step).clamp(lo, hi)
}

impl Validate for Settings {
    fn validate(&self) -> Result<(), String> {
        within("mouse_deg_per_px", self.mouse_deg_per_px, MOUSE_DEG_PER_PX.0, MOUSE_DEG_PER_PX.1)?;
        within("fov_deg", self.fov_deg, FOV_DEG.0, FOV_DEG.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Settings {
        Settings::from_data(&crate::input::Bindings::default(), &crate::pilot::PilotConfig::default())
    }

    #[test]
    fn the_defaults_are_the_datas_own_values() {
        let b = crate::input::Bindings::default();
        let s = defaults();
        assert_eq!(s.mouse_deg_per_px, b.mouse_deg_per_px);
        assert_eq!(s.invert_pitch, b.invert_pitch);
        assert!(s.shadows && s.vsync && !s.fullscreen);
        assert!(s.validate().is_ok());
    }

    #[test]
    fn nudging_steps_and_stops_at_the_limits() {
        let mut s = defaults();
        s.fov_deg = 85.0;
        s.nudge_fov(1);
        assert_eq!(s.fov_deg, 90.0);
        s.nudge_fov(100);
        assert_eq!(s.fov_deg, FOV_DEG.1);
        s.mouse_deg_per_px = 0.1;
        s.nudge_mouse(-100);
        assert_eq!(s.mouse_deg_per_px, MOUSE_DEG_PER_PX.0);
    }

    #[test]
    fn repeated_presses_do_not_drift() {
        let mut s = defaults();
        s.mouse_deg_per_px = 0.1;
        for _ in 0..7 {
            s.nudge_mouse(1);
        }
        for _ in 0..7 {
            s.nudge_mouse(-1);
        }
        assert!((s.mouse_deg_per_px - 0.1).abs() < 1e-6);
    }

    #[test]
    fn settings_survive_a_save_and_load() {
        let mut s = defaults();
        s.nudge_fov(-2);
        s.shadows = false;
        let back = Settings::from_ron(&s.to_ron()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_field_of_view_out_of_range_is_refused() {
        let mut s = defaults();
        s.fov_deg = 150.0;
        assert!(s.validate().is_err());
    }
}
