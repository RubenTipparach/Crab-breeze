//! Input actions (`pilot-boarding` design section 1): devices are sampled every frame through the
//! bindings in `data/input/bindings.ron` and accumulated; each fixed step takes one `Actions` value
//! from them. A script (the capture instrument now, the scripted pilot later) writes the same
//! value. Nothing else in the game reads a device.

use crate::sim::{Data, PresentSet, SimSet};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use cb_core::input::{Actions, Bindings, DebugCommand, ModeRequest};
use cb_core::mech::PartKind;

/// Device input gathered since the last fixed step.
#[derive(Resource, Default)]
pub struct InputAccum {
    /// Mouse look not yet used by a fixed step, degrees (right, up).
    pub look_deg: Vec2,
    /// Use was pressed.
    pub use_pressed: bool,
    /// Throttle zero was pressed.
    pub throttle_zero: bool,
    /// A mode was asked for.
    pub mode: Option<ModeRequest>,
    /// A test command was asked for.
    pub debug: Option<DebugCommand>,
    /// The skates were toggled.
    pub skate_toggle: bool,
    /// When set, a script plays instead of the keyboard (held axes and buttons); mouse look and
    /// latches above still add.
    pub script: Option<Actions>,
}

/// The actions for the current fixed step.
#[derive(Resource, Default, Deref)]
pub struct StepActions(pub Actions);

/// Key codes resolved from the bindings data.
#[derive(Resource)]
struct Keys {
    forward: Vec<KeyCode>,
    back: Vec<KeyCode>,
    left: Vec<KeyCode>,
    right: Vec<KeyCode>,
    run: Vec<KeyCode>,
    use_key: Vec<KeyCode>,
    free_look: Vec<KeyCode>,
    throttle_zero: Vec<KeyCode>,
    mode_walk: Vec<KeyCode>,
    mode_skate: Vec<KeyCode>,
    mode_jet: Vec<KeyCode>,
    release_mouse: Vec<KeyCode>,
    lift: Vec<KeyCode>,
    skate: Vec<KeyCode>,
    fire: [(Vec<KeyCode>, Vec<MouseButton>); 2],
    debug: Vec<(Vec<KeyCode>, DebugCommand)>,
    deg_per_px: f32,
    invert_pitch: bool,
}

/// Samples devices into actions.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        let keys = match resolve(&app.world().resource::<Data>().0.bindings) {
            Ok(k) => k,
            Err(e) => panic!("data/input/bindings.ron: {e}"),
        };
        app.insert_resource(keys)
            .init_resource::<InputAccum>()
            .init_resource::<StepActions>()
            .add_systems(Update, (grab_mouse, sample_devices).chain().in_set(PresentSet::Input))
            .add_systems(FixedUpdate, take_step_actions.in_set(SimSet::Input));
    }
}

/// Key and mouse bindings from one list of names (`MouseLeft`, `MouseRight`, `MouseMiddle` or keys).
fn split(field: &str, names: &[String]) -> Result<(Vec<KeyCode>, Vec<MouseButton>), String> {
    let mut keys = Vec::new();
    let mut buttons = Vec::new();
    for n in names {
        match n.as_str() {
            "MouseLeft" => buttons.push(MouseButton::Left),
            "MouseRight" => buttons.push(MouseButton::Right),
            "MouseMiddle" => buttons.push(MouseButton::Middle),
            other => keys.push(key_code(other).ok_or_else(|| format!("`{field}`: unknown key `{other}`"))?),
        }
    }
    Ok((keys, buttons))
}

fn resolve(b: &Bindings) -> Result<Keys, String> {
    let r = |field: &str, names: &[String]| -> Result<Vec<KeyCode>, String> {
        names
            .iter()
            .map(|n| key_code(n).ok_or_else(|| format!("`{field}`: unknown key `{n}`")))
            .collect()
    };
    Ok(Keys {
        forward: r("forward", &b.forward)?,
        back: r("back", &b.back)?,
        left: r("left", &b.left)?,
        right: r("right", &b.right)?,
        run: r("run", &b.run)?,
        use_key: r("use_key", &b.use_key)?,
        free_look: r("free_look", &b.free_look)?,
        throttle_zero: r("throttle_zero", &b.throttle_zero)?,
        mode_walk: r("mode_walk", &b.mode_walk)?,
        mode_skate: r("mode_skate", &b.mode_skate)?,
        mode_jet: r("mode_jet", &b.mode_jet)?,
        release_mouse: r("release_mouse", &b.release_mouse)?,
        lift: r("lift", &b.lift)?,
        skate: r("skate", &b.skate)?,
        fire: [split("fire_primary", &b.fire_primary)?, split("fire_secondary", &b.fire_secondary)?],
        debug: vec![
            (r("debug_hit_arm_l", &b.debug_hit_arm_l)?, DebugCommand::Hit(PartKind::ArmL)),
            (r("debug_hit_arm_r", &b.debug_hit_arm_r)?, DebugCommand::Hit(PartKind::ArmR)),
            (r("debug_hit_head", &b.debug_hit_head)?, DebugCommand::Hit(PartKind::Head)),
            (r("debug_hit_leg_l", &b.debug_hit_leg_l)?, DebugCommand::Hit(PartKind::LegL)),
            (r("debug_shell", &b.debug_shell)?, DebugCommand::Shell),
        ],
        deg_per_px: b.mouse_deg_per_px,
        invert_pitch: b.invert_pitch,
    })
}

/// Key code for a binding name: Bevy's `KeyCode` variant names.
fn key_code(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    const LETTERS: [KeyCode; 26] = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV,
        KeyW, KeyX, KeyY, KeyZ,
    ];
    const DIGITS: [KeyCode; 10] = [Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9];
    if let Some(c) = name.strip_prefix("Key").filter(|c| c.len() == 1) {
        let i = c.as_bytes()[0].checked_sub(b'A')?;
        return LETTERS.get(i as usize).copied();
    }
    if let Some(c) = name.strip_prefix("Digit").filter(|c| c.len() == 1) {
        let i = c.as_bytes()[0].checked_sub(b'0')?;
        return DIGITS.get(i as usize).copied();
    }
    Some(match name {
        "ShiftLeft" => ShiftLeft,
        "ShiftRight" => ShiftRight,
        "ControlLeft" => ControlLeft,
        "ControlRight" => ControlRight,
        "AltLeft" => AltLeft,
        "AltRight" => AltRight,
        "Space" => Space,
        "Tab" => Tab,
        "Enter" => Enter,
        "Escape" => Escape,
        "ArrowUp" => ArrowUp,
        "ArrowDown" => ArrowDown,
        "ArrowLeft" => ArrowLeft,
        "ArrowRight" => ArrowRight,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        _ => return None,
    })
}

fn any(input: &ButtonInput<KeyCode>, keys: &[KeyCode]) -> bool {
    input.any_pressed(keys.iter().copied())
}

fn any_just(input: &ButtonInput<KeyCode>, keys: &[KeyCode]) -> bool {
    input.any_just_pressed(keys.iter().copied())
}

fn grab_mouse(
    mut cursor: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    input: Res<ButtonInput<KeyCode>>,
    keys: Res<Keys>,
    mut first: Local<bool>,
) {
    if !*first || mouse.just_pressed(MouseButton::Left) {
        *first = true;
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    }
    if any_just(&input, &keys.release_mouse) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

fn sample_devices(
    input: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions>,
    keys: Res<Keys>,
    mut acc: ResMut<InputAccum>,
) {
    if cursor.grab_mode != CursorGrabMode::None {
        let sign = if keys.invert_pitch { 1.0 } else { -1.0 };
        acc.look_deg += Vec2::new(motion.delta.x, sign * motion.delta.y) * keys.deg_per_px;
    }
    acc.use_pressed |= any_just(&input, &keys.use_key);
    acc.throttle_zero |= any_just(&input, &keys.throttle_zero);
    for (k, m) in [
        (&keys.mode_walk, ModeRequest::Walk),
        (&keys.mode_skate, ModeRequest::Skate),
        (&keys.mode_jet, ModeRequest::Jet),
    ] {
        if any_just(&input, k) {
            acc.mode = Some(m);
        }
    }
    acc.skate_toggle |= any_just(&input, &keys.skate);
    for (k, cmd) in &keys.debug {
        if any_just(&input, k) {
            acc.debug = Some(*cmd);
        }
    }
}

fn axis(input: &ButtonInput<KeyCode>, pos: &[KeyCode], neg: &[KeyCode]) -> f32 {
    f32::from(u8::from(any(input, pos))) - f32::from(u8::from(any(input, neg)))
}

fn take_step_actions(
    input: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<Keys>,
    mut acc: ResMut<InputAccum>,
    mut out: ResMut<StepActions>,
) {
    let held = match acc.script {
        Some(s) => s,
        None => {
            let fwd = axis(&input, &keys.forward, &keys.back);
            let side = axis(&input, &keys.right, &keys.left);
            Actions {
                move_xy: Vec2::new(side, fwd),
                throttle_axis: fwd,
                turn_axis: -side,
                run: any(&input, &keys.run),
                lift_axis: f32::from(u8::from(any(&input, &keys.lift))),
                fire: keys
                    .fire
                    .clone()
                    .map(|(k, m)| any(&input, &k) || mouse.any_pressed(m.iter().copied())),
                free_look: any(&input, &keys.free_look),
                dash: any(&input, &keys.skate),
                ..default()
            }
        }
    };
    out.0 = Actions {
        look_delta_deg: held.look_delta_deg + acc.look_deg,
        use_pressed: held.use_pressed || acc.use_pressed,
        throttle_zero: held.throttle_zero || acc.throttle_zero,
        mode_request: held.mode_request.or(acc.mode),
        debug: held.debug.or(acc.debug),
        skate_toggle: held.skate_toggle || acc.skate_toggle,
        ..held
    }
    .sanitised();
    acc.look_deg = Vec2::ZERO;
    acc.use_pressed = false;
    acc.throttle_zero = false;
    acc.mode = None;
    acc.debug = None;
    acc.skate_toggle = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_binding_names_a_key() {
        resolve(&Bindings::default()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(key_code("KeyW"), Some(KeyCode::KeyW));
        assert_eq!(key_code("Digit3"), Some(KeyCode::Digit3));
        assert_eq!(key_code("KeyÆ"), None);
        assert_eq!(key_code("Hyper"), None);
    }
}
