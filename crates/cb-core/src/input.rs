//! What the pilot asks for this fixed step, independent of any device.
//!
//! The input plugin in `cb-app` fills this from the keyboard and mouse through the bindings in
//! `data/input/bindings.ron`; the scripted pilot (`playtest-pipeline`) will fill the same value.
//! The simulation reads nothing else (CLAUDE.md section 6.2: input is actions).

use glam::Vec2;

/// A movement mode the pilot can ask the mech for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeRequest {
    /// Walk on the legs.
    Walk,
    /// Skate on the feet's wheels (survey M4).
    Skate,
    /// Fly on the jets (survey M3).
    Jet,
}

/// A test command, until weapons exist (openspec changes `mech-damage`, `terrain-deformation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugCommand {
    /// Shoot off a part of the pilot's own mech.
    Hit(crate::mech::PartKind),
    /// Drop a shell where the pilot is looking.
    Shell,
}

/// The action state for one fixed step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Actions {
    /// On foot: strafe right (+x) and forward (+y), each in -1..1.
    pub move_xy: Vec2,
    /// Look this step: yaw right (+x) and pitch up (+y), in degrees.
    pub look_delta_deg: Vec2,
    /// In the mech: move the throttle lever up (+) or down (-), -1..1.
    pub throttle_axis: f32,
    /// In the mech: turn the legs left (+) or right (-), -1..1.
    pub turn_axis: f32,
    /// In the mech: jet lift, 0..1.
    pub lift_axis: f32,
    /// Use was pressed this step (board, climb out).
    pub use_pressed: bool,
    /// Run on foot.
    pub run: bool,
    /// Hold to look around the cockpit instead of aiming the torso.
    pub free_look: bool,
    /// Snap the throttle lever to zero this step.
    pub throttle_zero: bool,
    /// A movement mode was asked for this step.
    pub mode_request: Option<ModeRequest>,
    /// A test command this step.
    pub debug: Option<DebugCommand>,
    /// Fire groups held: [primary, secondary].
    pub fire: [bool; 2],
    /// Skates toggled this step.
    pub skate_toggle: bool,
}

impl Actions {
    /// The same actions with every finite check applied: non-finite axes become zero, axes clamp.
    pub fn sanitised(mut self) -> Self {
        let clean = |v: f32, lim: f32| if v.is_finite() { v.clamp(-lim, lim) } else { 0.0 };
        self.move_xy = Vec2::new(clean(self.move_xy.x, 1.0), clean(self.move_xy.y, 1.0));
        self.look_delta_deg = Vec2::new(clean(self.look_delta_deg.x, 360.0), clean(self.look_delta_deg.y, 360.0));
        self.throttle_axis = clean(self.throttle_axis, 1.0);
        self.turn_axis = clean(self.turn_axis, 1.0);
        self.lift_axis = clean(self.lift_axis, 1.0).max(0.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_input_becomes_zero() {
        let a = Actions {
            throttle_axis: f32::NAN,
            look_delta_deg: Vec2::new(f32::INFINITY, 3.0),
            ..Default::default()
        }
        .sanitised();
        assert_eq!(a.throttle_axis, 0.0);
        assert_eq!(a.look_delta_deg, Vec2::new(0.0, 3.0));
    }
}

/// Device bindings for the actions, loaded from `data/input/bindings.ron`. Key names are the
/// engine's key code names (`KeyW`, `ShiftLeft`); `cb-app` maps them and refuses unknown names.
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Bindings {
    /// Mouse look, degrees per pixel of mouse motion.
    pub mouse_deg_per_px: f32,
    /// Mouse up looks down.
    pub invert_pitch: bool,
    /// Walk forward; throttle up in the mech.
    pub forward: Vec<String>,
    /// Walk back; throttle down in the mech.
    pub back: Vec<String>,
    /// Strafe left; turn the legs left in the mech.
    pub left: Vec<String>,
    /// Strafe right; turn the legs right in the mech.
    pub right: Vec<String>,
    /// Run.
    pub run: Vec<String>,
    /// Use: board, climb out.
    pub use_key: Vec<String>,
    /// Hold to look around the cockpit.
    pub free_look: Vec<String>,
    /// Throttle to zero.
    pub throttle_zero: Vec<String>,
    /// Ask for walking.
    pub mode_walk: Vec<String>,
    /// Ask for skating.
    pub mode_skate: Vec<String>,
    /// Ask for jets.
    pub mode_jet: Vec<String>,
    /// Release the mouse.
    pub release_mouse: Vec<String>,
    /// Jet lift (hold).
    pub lift: Vec<String>,
    /// Test: shoot off the left arm.
    pub debug_hit_arm_l: Vec<String>,
    /// Test: shoot off the right arm.
    pub debug_hit_arm_r: Vec<String>,
    /// Test: shoot off the head.
    pub debug_hit_head: Vec<String>,
    /// Test: shoot off the left leg.
    pub debug_hit_leg_l: Vec<String>,
    /// Test: drop a shell where the pilot looks.
    pub debug_shell: Vec<String>,
    /// Fire group 1 (`MouseLeft`, `MouseRight` or key names).
    pub fire_primary: Vec<String>,
    /// Fire group 2.
    pub fire_secondary: Vec<String>,
    /// Toggle the skates (in the mech).
    pub skate: Vec<String>,
}

impl Default for Bindings {
    fn default() -> Self {
        let k = |s: &str| vec![s.to_string()];
        Self {
            mouse_deg_per_px: 0.08,
            invert_pitch: false,
            forward: k("KeyW"),
            back: k("KeyS"),
            left: k("KeyA"),
            right: k("KeyD"),
            run: k("ShiftLeft"),
            use_key: k("KeyE"),
            free_look: k("AltLeft"),
            throttle_zero: k("KeyX"),
            mode_walk: k("Digit1"),
            mode_skate: k("Digit2"),
            mode_jet: k("Digit3"),
            release_mouse: k("Escape"),
            lift: k("Space"),
            debug_hit_arm_l: k("F5"),
            debug_hit_arm_r: k("F6"),
            debug_hit_head: k("F7"),
            debug_hit_leg_l: k("F8"),
            debug_shell: k("F9"),
            fire_primary: k("MouseLeft"),
            fire_secondary: k("MouseRight"),
            skate: k("ShiftLeft"),
        }
    }
}

impl crate::data::Validate for Bindings {
    fn validate(&self) -> Result<(), String> {
        crate::data::within("mouse_deg_per_px", self.mouse_deg_per_px, 0.0, 10.0)
    }
}
