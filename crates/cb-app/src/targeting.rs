//! Targeting in the game (openspec change `targeting`): each fixed step, before the mechs step,
//! the live enemies within range (mechs and vehicles, `units::Hostiles`) become targets, the selected one is kept, cycled with the
//! `target_next` action or picked nearest the pilot's aim, and the missile lock builds on it while
//! it is in missile range (`cb_core::targeting`). The cockpit draws this state and the missiles
//! fired this step home on its lock.

use crate::input::StepActions;
use crate::mech::{MechSim, Piloted};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use bevy::prelude::*;
use cb_core::pilot::PilotMode;
use cb_core::targeting::{Lock, Target, TargetingConfig, nearest_to_aim, next, off_aim_deg};

/// What the pilot has targeted.
#[derive(Resource, Default)]
pub struct TargetState {
    /// The selected target's id.
    pub selected: Option<u32>,
    /// Its distance from the eye.
    pub range_m: f32,
    /// The missile lock on it.
    pub lock: Lock,
}

impl TargetState {
    /// The locked target, if the lock is complete.
    pub fn locked(&self, cfg: &TargetingConfig) -> Option<u32> {
        if self.lock.locked(cfg) { self.lock.target } else { None }
    }
}

/// Keeps the selection and the lock.
pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetState>()
            .add_systems(FixedUpdate, update_targeting.in_set(SimSet::Pilot));
    }
}

/// The pilot's aim: where the view looks, from yaw and pitch.
pub fn aim_dir(yaw_rad: f32, pitch_rad: f32) -> Vec3 {
    Vec3::new(-yaw_rad.sin() * pitch_rad.cos(), pitch_rad.sin(), -yaw_rad.cos() * pitch_rad.cos())
}

fn update_targeting(
    data: Res<Data>,
    pilot: Res<PilotSim>,
    actions: Res<StepActions>,
    time: Res<Time<Fixed>>,
    me: Query<&MechSim, With<Piloted>>,
    hostiles: Res<crate::units::Hostiles>,
    mut state: ResMut<TargetState>,
) {
    let cfg = &data.0.targeting;
    let (Some(m), true) = (me.iter().next(), pilot.curr.mode == PilotMode::Seated) else {
        *state = TargetState::default();
        return;
    };
    let eye = m.curr.pos + Vec3::Y * data.0.mech.body.eye_height_m;
    let aim = aim_dir(pilot.curr.yaw_rad, pilot.curr.pitch_rad);
    // Mechs and vehicles alike (invasion-forces).
    let mut targets: Vec<Target> = hostiles
        .0
        .iter()
        .map(|h| Target { id: h.id, torso: h.aim })
        .filter(|t| t.torso.distance(eye) <= cfg.max_range_m)
        .collect();
    targets.sort_by_key(|t| t.id);
    let still = state.selected.filter(|s| targets.iter().any(|t| t.id == *s));
    state.selected = if actions.0.target_next {
        next(still, &targets, eye)
    } else {
        still.or_else(|| nearest_to_aim(eye, aim, &targets, cfg.pick_cone_deg))
    };
    let chosen = state.selected.and_then(|s| targets.iter().find(|t| t.id == s)).copied();
    let (off, range) = chosen.map_or((180.0, f32::INFINITY), |t| (off_aim_deg(eye, aim, t.torso), t.torso.distance(eye)));
    state.range_m = range;
    let selected = state.selected;
    state.lock.step(selected, off, range, cfg, time.delta_secs());
}
