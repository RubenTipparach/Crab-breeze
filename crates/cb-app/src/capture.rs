//! The capture instrument (`pilot-boarding` design section 4; CLAUDE.md section 4 allows it as a
//! measurement instrument): `--stills <dir>` writes stills from the street and the cockpit and
//! logs frame time over a stride, then exits. It only writes input actions, as a player would,
//! except for placing the pilot at the start of the street shot. It moves into the scripted pilot
//! (`playtest-pipeline`) when that lands.

use crate::input::InputAccum;
use crate::mech::MechAccessNow;
use crate::pilot::PilotSim;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use cb_core::input::Actions;
use cb_core::pilot::{PilotMode, PilotState};
use std::path::PathBuf;

/// Runs the capture script.
pub struct StillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    frames_ms: Vec<f32>,
}

impl Plugin for StillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            frames_ms: Vec::new(),
        })
        .add_systems(Update, run_script);
    }
}

fn shoot(commands: &mut Commands, dir: &std::path::Path, name: &str) {
    let path = dir.join(name);
    info!("capture: {}", path.display());
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

fn run_script(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut acc: ResMut<InputAccum>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let in_beat = now - script.beat_started_s;
    let mut next = false;
    match script.beat {
        // Wait for the world, the rig and shaders to settle.
        0 => next = in_beat > 6.0 && access.0.is_some(),
        // The street: the pilot 12 m behind the mech, looking up at it along the street.
        1 => {
            if in_beat < 0.05 {
                let a = access.0.unwrap();
                let feet = a.ladder_base + Vec3::new(-6.0, 0.0, 10.0);
                let to = a.ladder_base - feet;
                let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
                s.pitch_rad = 0.32;
                pilot.prev = s.clone();
                pilot.curr = s;
            }
            if in_beat > 1.5 {
                shoot(&mut commands, &script.dir, "street.png");
                next = true;
            }
        }
        // Walk to the ladder by input, then press Use.
        2 => {
            let a = access.0.unwrap();
            let feet = pilot.curr.feet;
            let to = a.ladder_base - feet;
            let flat = Vec2::new(to.x, to.z);
            if pilot.curr.mode == PilotMode::OnFoot && flat.length() > 2.0 {
                let want = (-to.x).atan2(-to.z);
                let turn = (want - pilot.curr.yaw_rad + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                acc.script = Some(Actions {
                    move_xy: Vec2::new(0.0, 1.0),
                    look_delta_deg: Vec2::new(-turn.to_degrees(), 0.0),
                    ..default()
                });
            } else {
                acc.script = Some(Actions {
                    use_pressed: pilot.curr.mode == PilotMode::OnFoot,
                    ..default()
                });
            }
            if matches!(pilot.curr.mode, PilotMode::Boarding(_)) {
                acc.script = Some(Actions::default());
            }
            if in_beat > 2.0 && matches!(pilot.curr.mode, PilotMode::Boarding(ref p) if p.travelled_m > 9.0) {
                shoot(&mut commands, &script.dir, "boarding.png");
                next = true;
            }
        }
        3 => {
            if pilot.curr.mode == PilotMode::Seated && in_beat > 1.0 {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Stride: throttle up to 60 %, hold, measure frame time over 10 s.
        4 => {
            acc.script = Some(Actions {
                throttle_axis: if in_beat < 1.0 { 0.6 } else { 0.0 },
                ..default()
            });
            if in_beat > 4.0 {
                script.frames_ms.push(time.delta_secs() * 1000.0);
            }
            if in_beat > 14.0 {
                shoot(&mut commands, &script.dir, "cockpit-stride.png");
                next = true;
            }
        }
        5 if in_beat > 1.5 => {
            let mut f = script.frames_ms.clone();
            f.sort_by(f32::total_cmp);
            println!(
                "FRAME_TIME_MS frames={} p50={:.2} p95={:.2} p99={:.2} max={:.2}",
                f.len(),
                percentile(&f, 0.5),
                percentile(&f, 0.95),
                percentile(&f, 0.99),
                f.last().copied().unwrap_or(0.0)
            );
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
    }
}
