//! The capture instrument (`pilot-boarding` design section 4; CLAUDE.md section 4 allows it as a
//! measurement instrument): `--stills <dir>` writes stills from the street and the cockpit, logs
//! frame time over a stride, looks up at the clouds and jets into one for the rain on the glass
//! (`cloud-puffs`, `cloud-rain`), then exits. It only writes input actions, as a player would,
//! except for placing the pilot at the start of the street shot. It moves into the scripted pilot
//! (`playtest-pipeline`) when that lands.

use crate::input::InputAccum;
use crate::mech::{MechAccessNow, MechSim, Piloted};
use crate::pilot::PilotSim;
use crate::showcase::{fly_to_cloud, nearest_cloud};
use crate::sim::Data;
use crate::world::Plan;
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
    cloud: Vec3,
    in_cloud_s: f32,
    shot_near: bool,
    shot_climb: bool,
    start_y_m: f32,
}

impl Plugin for StillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            frames_ms: Vec::new(),
            cloud: Vec3::ZERO,
            in_cloud_s: 0.0,
            shot_near: false,
            shot_climb: false,
            start_y_m: 0.0,
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
    mechs: Query<&MechSim, With<Piloted>>,
    plan: Res<Plan>,
    data: Res<Data>,
    ready: Res<crate::loading::GameReady>,
    time: Res<Time<Real>>,
    game_time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let in_beat = now - script.beat_started_s;
    let mut next = false;
    match script.beat {
        // Wait for the game to be ready (GAME_READY: every asset and pipeline loaded), then for the
        // rig and shaders to settle.
        0 => next = in_beat > 6.0 && ready.0 && access.0.is_some(),
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
        3 => {
            // Seated; wait for the hatch to seal behind the pilot.
            if pilot.curr.mode == PilotMode::Seated && in_beat > 12.0 {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Stand up and stride: throttle forward (the mech stands first), hold, measure frame time
        // over 10 s.
        4 => {
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
        // Stop and look up through the front screens at the clouds (cloud-puffs).
        5 => {
            let Some(m) = mechs.iter().next().map(|s| &s.curr) else { return };
            acc.script = Some(Actions {
                throttle_zero: in_beat < 0.1,
                look_delta_deg: Vec2::new(0.0, (0.3 - m.aim_pitch_rad).to_degrees() * 0.2),
                ..default()
            });
            if in_beat > 6.0 {
                shoot(&mut commands, &script.dir, "sky.png");
                script.cloud = nearest_cloud(&plan, m);
                script.start_y_m = m.pos.y;
                next = true;
            }
        }
        // Rockets up and skates across into the nearest cloud: one still 400 m up, clear of the
        // roofs, one on the way in, one after 2.5 s inside, when the glass is wet (cloud-rain). The
        // limit is in real seconds and generous: a software renderer runs the simulation slower.
        6 => {
            let Some(m) = mechs.iter().next().map(|s| &s.curr) else { return };
            acc.script = Some(fly_to_cloud(m, script.cloud));
            let eye = m.pos + Vec3::Y * data.0.mech.body.eye_height_m;
            if !script.shot_climb && m.pos.y - script.start_y_m > 400.0 {
                shoot(&mut commands, &script.dir, "climb.png");
                script.shot_climb = true;
            }
            if !script.shot_near && Vec2::new(eye.x, eye.z).distance(Vec2::new(script.cloud.x, script.cloud.z)) < 700.0 {
                shoot(&mut commands, &script.dir, "cloud-near.png");
                script.shot_near = true;
            }
            if plan.clouds.density_at(eye, now) > 0.05 {
                // Game time, as the glass wets by it.
                script.in_cloud_s += game_time.delta_secs();
            }
            if script.in_cloud_s > 2.5 || in_beat > 900.0 || m.boost.locked {
                info!("capture: {:.1} s in the cloud", script.in_cloud_s);
                shoot(&mut commands, &script.dir, "cloud-rain.png");
                next = true;
            }
        }
        // Let the last screenshot reach the disk before exiting.
        7 if in_beat > 8.0 => {
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
