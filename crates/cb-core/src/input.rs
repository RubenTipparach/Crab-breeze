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
    /// Ctrl held in the mech: dive in the air, sink on water.
    pub sink: bool,
    /// Missile trigger held (fire group 2, at the lock: `targeting`).
    pub fire_missile: bool,
    /// Select the next target this step.
    pub target_next: bool,
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
    fn every_binding_in_the_data_is_on_the_controls_page() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/input/bindings.ron");
        let text = std::fs::read_to_string(&p).unwrap();
        let b: Bindings = crate::data::load_ron(&p).unwrap();
        let fields: Vec<&str> = b.controls().iter().map(|c| c.field).collect();
        for line in text.lines() {
            let t = line.trim_start();
            if let Some((name, rest)) = t.split_once(':')
                && rest.trim_start().starts_with('[')
            {
                assert!(fields.contains(&name), "`{name}` is bound but not on the Controls page");
            }
        }
    }

    #[test]
    fn keys_read_as_a_player_names_them() {
        assert_eq!(key_label("KeyW"), "W");
        assert_eq!(key_label("ControlLeft"), "Left Ctrl");
        assert_eq!(key_label("ShiftLeft"), "Left Shift");
        assert_eq!(key_label("MouseLeft"), "Left mouse");
        assert_eq!(key_label("Digit1"), "1");
        assert_eq!(key_label("Escape"), "Esc");
        assert_eq!(key_label("Space"), "Space");
    }

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
    /// Hold to dive from the air or sink on water (in the mech).
    pub sink: Vec<String>,
    /// Fire a missile at the lock (in the mech).
    pub fire_missile: Vec<String>,
    /// Select the next target (in the mech).
    pub target_next: Vec<String>,
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
            free_look: k("ControlLeft"),
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
            sink: k("KeyC"),
            fire_missile: k("KeyF"),
            target_next: k("KeyQ"),
        }
    }
}

impl crate::data::Validate for Bindings {
    fn validate(&self) -> Result<(), String> {
        crate::data::within("mouse_deg_per_px", self.mouse_deg_per_px, 0.0, 10.0)
    }
}

/// A key's name as a player reads it: `KeyW` reads W, `ControlLeft` Left Ctrl, `MouseLeft` Left
/// mouse, `Digit1` 1.
pub fn key_label(name: &str) -> String {
    for (prefix, word) in [("Control", "Ctrl"), ("Shift", "Shift"), ("Alt", "Alt"), ("Super", "Super")] {
        if let Some(side) = name.strip_prefix(prefix) {
            return match side {
                "Left" | "Right" => format!("{side} {word}"),
                _ => word.to_string(),
            };
        }
    }
    match name {
        "MouseLeft" => "Left mouse".into(),
        "MouseRight" => "Right mouse".into(),
        "MouseMiddle" => "Middle mouse".into(),
        "Escape" => "Esc".into(),
        _ => name
            .strip_prefix("Key")
            .or_else(|| name.strip_prefix("Digit"))
            .unwrap_or(name)
            .to_string(),
    }
}

/// One row of the controls table: where it is used, what it does, its binding's field, its keys.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    /// Where it is used ("On foot", "In the mech", ...).
    pub group: &'static str,
    /// What it does, in the pilot's words.
    pub name: &'static str,
    /// The binding's field in `data/input/bindings.ron`.
    pub field: &'static str,
    /// Its keys, readable, joined with " / ".
    pub keys: String,
}

impl Bindings {
    /// Every control and its keys, grouped by where it is used: what the Controls page shows
    /// (openspec change `game-menus`). Read from the bindings, so it is never out of date.
    pub fn controls(&self) -> Vec<Control> {
        let row = |group, name, field, keys: &Vec<String>| Control {
            group,
            name,
            field,
            keys: keys.iter().map(|k| key_label(k)).collect::<Vec<_>>().join(" / "),
        };
        vec![
            row("On foot", "Walk forward", "forward", &self.forward),
            row("On foot", "Walk back", "back", &self.back),
            row("On foot", "Step left", "left", &self.left),
            row("On foot", "Step right", "right", &self.right),
            row("On foot", "Run (hold)", "run", &self.run),
            row("On foot", "Board the mech, climb out", "use_key", &self.use_key),
            row("In the mech", "Throttle up", "forward", &self.forward),
            row("In the mech", "Throttle down", "back", &self.back),
            row("In the mech", "Turn left", "left", &self.left),
            row("In the mech", "Turn right", "right", &self.right),
            row("In the mech", "Throttle to zero", "throttle_zero", &self.throttle_zero),
            row("In the mech", "Walk", "mode_walk", &self.mode_walk),
            row("In the mech", "Skate mode", "mode_skate", &self.mode_skate),
            row("In the mech", "Jet mode", "mode_jet", &self.mode_jet),
            row("In the mech", "Rockets: lift", "lift", &self.lift),
            row("In the mech", "Skates on and off", "skate", &self.skate),
            row("In the mech", "Dive, sink on water", "sink", &self.sink),
            row("Weapons", "Fire primary", "fire_primary", &self.fire_primary),
            row("Weapons", "Fire secondary", "fire_secondary", &self.fire_secondary),
            row("Weapons", "Fire missile at the lock", "fire_missile", &self.fire_missile),
            row("Weapons", "Next target", "target_next", &self.target_next),
            row(
                "Cockpit",
                "Free look (hold); click an MFD to change page",
                "free_look",
                &self.free_look,
            ),
            row("Game", "Pause menu", "release_mouse", &self.release_mouse),
            row("Test keys", "Hit left arm", "debug_hit_arm_l", &self.debug_hit_arm_l),
            row("Test keys", "Hit right arm", "debug_hit_arm_r", &self.debug_hit_arm_r),
            row("Test keys", "Hit head", "debug_hit_head", &self.debug_hit_head),
            row("Test keys", "Hit left leg", "debug_hit_leg_l", &self.debug_hit_leg_l),
            row("Test keys", "Drop a shell where you look", "debug_shell", &self.debug_shell),
        ]
    }
}
