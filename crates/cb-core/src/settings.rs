//! The player's settings (openspec change `game-menus`, design section 4): mouse sensitivity and
//! invert, field of view, shadows, vsync, fullscreen and the sound effects and music volumes. One record, defaulted from the data's own
//! values, changed on the Options page and saved by the game to `settings.ron`. Pure: the steps the
//! Options page takes and the limits they keep to live here so they are tested without a window.

use crate::data::{Validate, within};
use serde::{Deserialize, Serialize};

/// Mouse sensitivity limits, degrees per pixel, and the Options page's step.
pub const MOUSE_DEG_PER_PX: (f32, f32, f32) = (0.01, 1.0, 0.01);
/// Vertical field of view limits, degrees, and the Options page's step.
pub const FOV_DEG: (f32, f32, f32) = (60.0, 110.0, 5.0);
/// Volume limits, a share of the data's own level, and the Options page's step (10 %).
pub const VOLUME: (f32, f32, f32) = (0.0, 1.0, 0.1);

fn full() -> f32 {
    1.0
}

fn on() -> bool {
    true
}

/// The player's settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Mouse look, degrees per pixel of mouse motion.
    pub mouse_deg_per_px: f32,
    /// Mouse up looks down.
    pub invert_pitch: bool,
    /// The view's vertical field of view seated in the cockpit, degrees; on foot it is narrower by the data's
    /// difference (`pilot.ron`, openspec change `seated-fov`).
    pub fov_deg: f32,
    /// The sun casts shadows.
    pub shadows: bool,
    /// Frames wait for the display's refresh.
    pub vsync: bool,
    /// Borderless fullscreen instead of a window.
    pub fullscreen: bool,
    /// Sound effects, a share of their level in the data, 0-1 (1 if the file has none).
    #[serde(default = "full")]
    pub sfx_volume: f32,
    /// Music, a share of its level in the data, 0-1 (1 if the file has none).
    #[serde(default = "full")]
    pub music_volume: f32,
    /// How much the view shakes, a share of the data's shake, 0-1 (1 if the file has none; openspec change
    /// `cockpit-shake`, CS2).
    #[serde(default = "full")]
    pub camera_shake: f32,
    /// A stolen plane's stall protection (`fly-stolen-aircraft` section 4; surveys G2 and G5: assists that can be
    /// turned off). On if the file has none.
    #[serde(default = "on")]
    pub assist_stall: bool,
    /// A stolen plane's auto-trim: it holds its pitch with the stick centred.
    #[serde(default = "on")]
    pub assist_trim: bool,
    /// A stolen helicopter's hover hold: it holds its place with the cyclic centred and its height with the collective
    /// at half.
    #[serde(default = "on")]
    pub assist_hover: bool,
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
            sfx_volume: 1.0,
            music_volume: 1.0,
            camera_shake: 1.0,
            assist_stall: true,
            assist_trim: true,
            assist_hover: true,
        }
    }

    /// The flight assists as the flight model takes them.
    pub fn flight_assists(&self) -> crate::flight_model::Assists {
        crate::flight_model::Assists {
            stall: self.assist_stall,
            trim: self.assist_trim,
            hover: self.assist_hover,
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

    /// Sound effects volume up or down by the page's step, kept within 0-1.
    pub fn nudge_sfx(&mut self, steps: i32) {
        self.sfx_volume = nudge(self.sfx_volume, steps, VOLUME);
    }

    /// Music volume up or down by the page's step, kept within 0-1.
    pub fn nudge_music(&mut self, steps: i32) {
        self.music_volume = nudge(self.music_volume, steps, VOLUME);
    }

    /// Camera shake up or down by the page's step, kept within 0-1.
    pub fn nudge_shake(&mut self, steps: i32) {
        self.camera_shake = nudge(self.camera_shake, steps, VOLUME);
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
        within("fov_deg", self.fov_deg, FOV_DEG.0, FOV_DEG.1)?;
        within("sfx_volume", self.sfx_volume, VOLUME.0, VOLUME.1)?;
        within("music_volume", self.music_volume, VOLUME.0, VOLUME.1)?;
        within("camera_shake", self.camera_shake, VOLUME.0, VOLUME.1)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn camera_shake_steps_within_zero_to_one_and_an_old_file_keeps_it_full() {
        let mut s = defaults();
        assert_eq!(s.camera_shake, 1.0);
        s.nudge_shake(-100);
        assert_eq!(s.camera_shake, 0.0, "it turns all the way off");
        s.nudge_shake(1);
        assert!(s.camera_shake > 0.0 && s.camera_shake <= 1.0);
        let old = defaults().to_ron().replace("camera_shake", "removed_key_check");
        assert!(Settings::from_ron(&old).is_err(), "an unknown key is refused");
        let without: String = defaults()
            .to_ron()
            .lines()
            .filter(|l| !l.contains("camera_shake"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            Settings::from_ron(&without).unwrap().camera_shake,
            1.0,
            "a file from before keeps the full shake"
        );
    }

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
    fn the_flight_assists_are_on_unless_turned_off_and_an_old_file_keeps_them_on() {
        let mut s = defaults();
        assert_eq!(s.flight_assists(), crate::flight_model::Assists::default());
        s.assist_trim = false;
        assert!(!s.flight_assists().trim && s.flight_assists().stall && s.flight_assists().hover);
        let old = "(mouse_deg_per_px: 0.1, invert_pitch: false, fov_deg: 90.0, shadows: true, vsync: true, fullscreen: false)";
        let read: Settings = ron::from_str(old).unwrap();
        assert!(
            read.assist_stall && read.assist_trim && read.assist_hover,
            "a settings file from before has them on"
        );
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
    fn volumes_step_by_ten_percent_and_stop_at_silent_and_full() {
        let mut s = defaults();
        assert_eq!((s.sfx_volume, s.music_volume), (1.0, 1.0));
        s.nudge_music(-2);
        assert!((s.music_volume - 0.8).abs() < 1e-6);
        s.nudge_sfx(-100);
        assert_eq!(s.sfx_volume, 0.0);
        s.nudge_sfx(100);
        assert_eq!(s.sfx_volume, 1.0);
        let back = Settings::from_ron(&s.to_ron()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_settings_file_without_volumes_loads_them_full() {
        let text = "(mouse_deg_per_px: 0.1, invert_pitch: false, fov_deg: 75.0, shadows: true, vsync: true, fullscreen: false)";
        let s = Settings::from_ron(text).unwrap();
        assert_eq!((s.sfx_volume, s.music_volume), (1.0, 1.0));
    }

    #[test]
    fn a_field_of_view_out_of_range_is_refused() {
        let mut s = defaults();
        s.fov_deg = 150.0;
        assert!(s.validate().is_err());
    }
}
