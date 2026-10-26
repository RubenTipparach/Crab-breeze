//! Targeting in the game (openspec change `targeting`): each fixed step, before the mechs step,
//! the live enemies within range (mechs and vehicles, `units::Hostiles`) become targets, the selected one is kept, cycled with the
//! `target_next` action or picked nearest the pilot's aim, and the missile lock builds on it while
//! it is in missile range (`cb_core::targeting`). Civil aircraft (`air_traffic::Neutrals`) are
//! targets too, but neutral: they are boxed and never selected or locked (survey W47). The cockpit draws this state and the missiles
//! fired this step home on its lock.
//!
//! Fog hides targets (survey F2): each target's fog scale comes from the fog along the line from the
//! eye to it (`cb_core::world::mist::line_depth`, at the strength `ground_fog::FogNow` sets, the
//! fog the screens draw), and only targets `seen` through it are targets at all, enemies and civil
//! aircraft alike. The scales are kept in `TargetState::fog` so the cockpit boxes exactly these.

use crate::input::StepActions;
use crate::mech::{MechSim, Piloted};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use bevy::prelude::*;
use cb_core::pilot::PilotMode;
use cb_core::targeting::{Lock, Target, TargetingConfig, fog_scale, off_aim_deg, seen, select};
use cb_core::world::mist::line_depth;

/// What the pilot has targeted.
#[derive(Resource, Default)]
pub struct TargetState {
    /// The selected target's id.
    pub selected: Option<u32>,
    /// Its distance from the eye.
    pub range_m: f32,
    /// The missile lock on it.
    pub lock: Lock,
    /// Every target seen this step and how much of the sensors' reach the fog leaves toward it
    /// (`cb_core::targeting::fog_scale`), by id.
    pub fog: Vec<(u32, f32)>,
}

impl TargetState {
    /// The locked target, if the lock is complete.
    pub fn locked(&self, cfg: &TargetingConfig) -> Option<u32> {
        if self.lock.locked(cfg) { self.lock.target } else { None }
    }

    /// The fog's scale toward target `id` if it was seen this step.
    pub fn fog_toward(&self, id: u32) -> Option<f32> {
        self.fog.binary_search_by_key(&id, |(i, _)| *i).ok().map(|k| self.fog[k].1)
    }
}

/// Keeps the selection and the lock.
pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetState>()
            .add_game_systems(FixedUpdate, update_targeting.in_set(SimSet::Pilot));
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
    neutrals: Res<crate::air_traffic::Neutrals>,
    plan: Res<crate::world::Plan>,
    fog_now: Res<crate::ground_fog::FogNow>,
    mut state: ResMut<TargetState>,
) {
    let cfg = &data.0.targeting;
    let (Some(m), true) = (me.iter().next(), pilot.curr.mode == PilotMode::Seated) else {
        *state = TargetState::default();
        return;
    };
    let eye = m.curr.pos + Vec3::Y * data.0.mech.body.eye_height_m;
    let aim = aim_dir(pilot.curr.yaw_rad, pilot.curr.pitch_rad);
    // Mechs and vehicles alike (invasion-forces), and civil aircraft as neutrals (civil-air-traffic).
    let enemies = hostiles.0.iter().map(|h| (h, false));
    let civil = neutrals.0.iter().map(|h| (h, true));
    // The fog along the line to each (survey F2): only those it lets the sensors reach are targets.
    let fog_cfg = &data.0.world.fog;
    let rain = plan.clouds.rain_at(eye, time.elapsed_secs());
    let fog_to = |p: Vec3| {
        let depth = line_depth(fog_cfg, fog_now.strength, &plan.mist, plan.waters.sea_level_m, eye, p, rain);
        fog_scale(cfg, depth)
    };
    let mut seen_now: Vec<(Target, f32)> = enemies
        .chain(civil)
        .map(|(h, neutral)| Target {
            id: h.id,
            torso: h.aim,
            neutral,
        })
        // A cheap cull before the fog is worked out: `seen` never reaches past the clear-air range.
        .filter(|t| t.torso.distance(eye) <= cfg.max_range_m)
        .map(|t| (t, fog_to(t.torso)))
        .filter(|(t, fog)| seen(cfg, t.torso.distance(eye), *fog))
        .collect();
    seen_now.sort_by_key(|(t, _)| t.id);
    let targets: Vec<Target> = seen_now.iter().map(|(t, _)| *t).collect();
    state.fog = seen_now.iter().map(|(t, f)| (t.id, *f)).collect();
    state.selected = select(state.selected, actions.0.target_next, eye, aim, &targets, cfg);
    let chosen = state.selected.and_then(|s| seen_now.iter().find(|(t, _)| t.id == s)).copied();
    let (off, range, fog) = chosen.map_or((180.0, f32::INFINITY, 1.0), |(t, f)| {
        (off_aim_deg(eye, aim, t.torso), t.torso.distance(eye), f)
    });
    state.range_m = range;
    let selected = state.selected;
    state.lock.step(selected, off, range, fog, cfg, time.delta_secs());
}
