//! The showcase take (`--showcase`): the game played start to finish for a recording, the way the
//! scripted pilot (`playtest-pipeline`) will play it. The pilot starts where the game starts, looks
//! around, walks to the mech, presses Use once, climbs in, stands the mech up and strides down the
//! street aiming the torso, then stops. It writes only input actions, as a player's keyboard and
//! mouse do, and prints `PLAYTEST_START`, `PLAYTEST_RESULT` and `PLAYTEST_END` for the recorder.

use crate::input::InputAccum;
use crate::mech::MechAccessNow;
use crate::pilot::PilotSim;
use bevy::prelude::*;
use cb_core::input::Actions;
use cb_core::pilot::{PilotMode, PilotState};
use std::f32::consts::{PI, TAU};

/// Plays the showcase.
pub struct ShowcasePlugin;

#[derive(Resource, Default)]
struct Take {
    beat: usize,
    beat_started_s: f32,
}

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Take>().add_systems(Update, play);
    }
}

/// Actions that walk the pilot toward `target` (feet), turning the view onto it as a mouse would,
/// with the view pitched to `pitch_rad`. Shared with the capture instrument.
pub fn walk_to(pilot: &PilotState, target: Vec3, pitch_rad: f32) -> Actions {
    let to = target - pilot.feet;
    let want = (-to.x).atan2(-to.z);
    let turn = (want - pilot.yaw_rad + PI).rem_euclid(TAU) - PI;
    let near = Vec2::new(to.x, to.z).length() < 2.0;
    Actions {
        move_xy: if near { Vec2::ZERO } else { Vec2::new(0.0, 1.0) },
        look_delta_deg: Vec2::new(-turn.to_degrees() * 0.15, (pitch_rad - pilot.pitch_rad).to_degrees() * 0.1),
        ..default()
    }
}

fn say(line: &str) {
    println!("{line}");
}

fn play(
    mut take: ResMut<Take>,
    mut acc: ResMut<InputAccum>,
    pilot: Res<PilotSim>,
    access: Res<MechAccessNow>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let t = now - take.beat_started_s;
    let mut next = false;
    let p = &pilot.curr;
    match take.beat {
        // The world and rig are up and shaders have settled.
        0 => {
            if t > 5.0 && access.0.is_some() {
                say("PLAYTEST_READY showcase");
                say("PLAYTEST_START showcase");
                next = true;
            }
        }
        // Look around the street: left, right, then back to the mech.
        1 => {
            let sweep = if t < 1.5 {
                -1.0
            } else if t < 4.5 {
                1.0
            } else if t < 6.0 {
                -1.0
            } else {
                0.0
            };
            acc.script = Some(Actions {
                look_delta_deg: Vec2::new(sweep * 0.45, 0.0),
                ..default()
            });
            next = t > 6.5;
        }
        // Walk to the stair's foot and press Use once; the mech sits, opens, and the pilot climbs.
        2 => {
            let a = access.0.unwrap();
            acc.script = Some(match p.mode {
                PilotMode::OnFoot if p.waiting.is_none() && p.distance_to_stair(&a) < 2.5 => Actions {
                    use_pressed: true,
                    ..default()
                },
                PilotMode::OnFoot if p.waiting.is_none() => walk_to(p, a.ladder_base, 0.2),
                // Waiting: look up at the mech as it sits and opens.
                PilotMode::OnFoot => walk_to(p, a.ladder_base + Vec3::Y * 1.0, 0.45),
                _ => Actions::default(),
            });
            next = p.mode == PilotMode::Seated;
        }
        // In the seat: a glance round the cockpit with free look.
        3 => {
            let sweep = if t < 1.2 {
                -1.0
            } else if t < 3.6 {
                1.0
            } else if t < 4.8 {
                -1.0
            } else {
                0.0
            };
            acc.script = Some(Actions {
                free_look: t < 5.5,
                look_delta_deg: Vec2::new(sweep * 0.9, 0.0),
                ..default()
            });
            next = t > 6.0;
        }
        // Throttle up (the mech stands first), stride, swing the torso left and right.
        4 => {
            let aim = if (6.0..9.0).contains(&t) {
                -0.5
            } else if (9.0..15.0).contains(&t) {
                0.5
            } else if (15.0..18.0).contains(&t) {
                -0.5
            } else {
                0.0
            };
            acc.script = Some(Actions {
                throttle_axis: if t < 6.0 { 0.8 } else { 0.0 },
                look_delta_deg: Vec2::new(aim, 0.0),
                ..default()
            });
            next = t > 24.0;
        }
        // Stop.
        5 => {
            acc.script = Some(Actions {
                throttle_zero: t < 0.1,
                ..default()
            });
            if t > 4.0 {
                say("PLAYTEST_RESULT ok showcase: walked, boarded, strode");
                next = true;
            }
        }
        6 => {
            if t > 2.0 {
                say("PLAYTEST_END showcase");
                exit.write(AppExit::Success);
                next = true;
            }
        }
        _ => {}
    }
    if next {
        take.beat += 1;
        take.beat_started_s = now;
    }
}
