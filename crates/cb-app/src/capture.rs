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
    enemies: Query<&GlobalTransform, With<crate::mech::Enemy>>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
    mut exit_frames: Local<u32>,
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
        // An enemy in its skin (`blockout-enemies`): the pilot stood 70 m off the nearest, looking up at it.
        // Skipped when there is none; then back behind the mech to board.
        2 if enemies.is_empty() => next = true,
        2 => {
            let a = access.0.unwrap();
            let e = enemies
                .iter()
                .map(|g| g.translation())
                .min_by(|p, q| p.distance(a.ladder_base).total_cmp(&q.distance(a.ladder_base)))
                .unwrap();
            if in_beat < 0.05 {
                let flat = Vec3::new(a.ladder_base.x - e.x, 0.0, a.ladder_base.z - e.z).normalize_or(Vec3::X);
                let feet = Vec3::new(e.x, a.ladder_base.y, e.z) + flat * 70.0;
                let to = e - feet;
                let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
                s.pitch_rad = 0.12;
                pilot.prev = s.clone();
                pilot.curr = s;
            }
            if in_beat > 2.0 {
                shoot(&mut commands, &script.dir, "enemy.png");
                let a = access.0.unwrap();
                let feet = a.ladder_base + Vec3::new(-6.0, 0.0, 10.0);
                let s = PilotState::standing(feet, 0.0);
                pilot.prev = s.clone();
                pilot.curr = s;
                next = true;
            }
        }
        // Walk to the ladder by input, then press Use.
        3 => {
            let a = access.0.unwrap();
            acc.script = Some(if pilot.curr.mode == PilotMode::OnFoot && pilot.curr.distance_to_stair(&a) > 2.0 {
                crate::showcase::walk_to(&pilot.curr, a.ladder_base, 0.0)
            } else {
                Actions {
                    use_pressed: pilot.curr.mode == PilotMode::OnFoot,
                    ..default()
                }
            });
            if matches!(pilot.curr.mode, PilotMode::Boarding(_)) {
                // The stair is down and the climb begins: the open hatch from the stair's foot.
                acc.script = Some(Actions::default());
                shoot(&mut commands, &script.dir, "hatch-open.png");
                next = true;
            }
        }
        4 => {
            // Seated; wait for the hatch to seal behind the pilot.
            if pilot.curr.mode == PilotMode::Seated && in_beat > 12.0 {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Stand up and stride: throttle forward (the mech stands first), hold, measure frame time
        // over 10 s.
        5 => {
            acc.script = Some(Actions {
                throttle_axis: if in_beat < 5.0 { 0.6 } else { 0.0 },
                ..default()
            });
            if in_beat > 7.0 {
                script.frames_ms.push(time.delta_secs() * 1000.0);
            }
            if in_beat > 17.0 {
                shoot(&mut commands, &script.dir, "cockpit-stride.png");
                next = true;
            }
        }
        // A few frames after the last shot, so its readback lands even at a software renderer's frame rate.
        6 if in_beat > 1.5 && {
            *exit_frames += 1;
            *exit_frames > 6
        } =>
        {
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
