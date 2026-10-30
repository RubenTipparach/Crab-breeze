//! The fortress stills (a measurement instrument, CLAUDE.md section 4; `crab-airship` section 9): with
//! `CB_STILLS_FORTRESS=1 --stills <dir> --mission fortress`, free roam's mail is shot open as M opens it, then the
//! pilot stands where the Stratostacean is seen whole from the street, then under a lift thruster, then on its deck (the
//! platform carrying them, logged), and the thrusters are broken through the game's own hit path (`FortressHit`,
//! as a shell from under the nozzle would): three on the port side open the hull, two hull thrusters there bring it
//! down; it is shot opening, falling and crashed, then the run exits. The pilot is placed and turned directly, as
//! the other stills do.

use crate::capture::{shoot, stand_looking};
use crate::fortress::{FortressHit, FortressRun};
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::world::Ground;
use std::path::PathBuf;

/// Runs the fortress stills.
pub struct FortressStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    done: Vec<&'static str>,
    /// When it was first seen down.
    crashed_at: Option<f32>,
}

impl Script {
    fn once(&mut self, step: &'static str) -> bool {
        if self.done.contains(&step) {
            return false;
        }
        self.done.push(step);
        true
    }
}

impl Plugin for FortressStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            done: Vec::new(),
            crashed_at: None,
        })
        .add_game_systems(Update, run);
    }
}

/// A cannon shell's worth of hits on part `id` from straight under it (its nozzle or port).
fn shell_from_below(run: &FortressRun, data: &Data, globals: &Query<&GlobalTransform>, id: &str, out: &mut MessageWriter<FortressHit>) {
    let d = &data.0.fortress;
    let Some(i) = d.part(id) else { return };
    let Some(g) = run.part_node(i).and_then(|e| globals.get(e).ok()) else {
        return;
    };
    let below = if d.shape.parts[i].kind == "hull" { -38.0 } else { -21.0 };
    out.write(FortressHit {
        part: i,
        at: g.transform_point(Vec3::Y * below),
        dir: g.rotation() * Vec3::Y,
        penetration_mm: 260.0,
        damage: 90.0,
    });
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut pilot: ResMut<PilotSim>,
    fortress: Option<Res<FortressRun>>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time>,
    globals: Query<&GlobalTransform>,
    mut hits: MessageWriter<FortressHit>,
    mut exit: MessageWriter<AppExit>,
    ready: Option<Res<crate::loading::GameReady>>,
    mut menu: ResMut<crate::menu::Menu>,
    free: Option<ResMut<crate::missions::FreeRoamRun>>,
    defending: Option<Res<crate::defence::Defending>>,
) {
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    // Last: the mail starts the defence, which must turn the Stratostacean off (free-roam-missions).
    if script.beat == 7 {
        if script.once("switch") {
            if let Some(mut f) = free {
                f.start(cb_core::missions::MissionKind::Defence);
            }
            script.beat_started_s = now;
            return;
        }
        if t > 3.0 {
            println!(
                "STILLS fortress switch: the defence started from the mail; the Stratostacean {}, the defence {}",
                if fortress.is_some() { "STILL RUNNING" } else { "turned off" },
                if defending.is_some() { "running" } else { "NOT RUNNING" }
            );
            exit.write(AppExit::Success);
        }
        return;
    }
    let Some(run) = fortress else { return };
    let f = &run.f;
    let pose = f.pose();
    let mid = f.pos;
    let ground = |x: f32, z: f32| plan.height_at(x, z);
    let mut next = false;
    match script.beat {
        0 => {
            next = ready.is_some_and(|r| r.0) && run.bound() && t > 3.0;
            // `CB_STILLS_FORTRESS=switch`: straight to the switch, to check it alone
            if next && std::env::var("CB_STILLS_FORTRESS").is_ok_and(|v| v == "switch") {
                script.beat = 7;
                script.beat_started_s = now;
                return;
            }
        }
        // Free roam's mail, as M opens it: the Stratostacean running (Stop), the defence offered (Start).
        1 => {
            if script.once("mail") {
                *menu = crate::menu::Menu::Mail;
            }
            if t > 2.0 && script.once("mail shot") {
                shoot(&mut commands, &script.dir, "fortress-mail.png");
            }
            if t > 4.0 {
                *menu = crate::menu::Menu::Closed;
                next = true;
            }
        }
        // Whole, from the street 900 m off, as the pilot first sees it.
        2 => {
            let off = Vec3::new(1.0, 0.0, 0.6).normalize() * 900.0;
            let feet = Vec3::new(mid.x + off.x, ground(mid.x + off.x, mid.z + off.z), mid.z + off.z);
            if script.once("street") {
                stand_looking(&mut pilot, feet, mid);
            }
            if t > 3.0 && script.once("street shot") {
                shoot(&mut commands, &script.dir, "fortress-street.png");
            }
            next = t > 5.0;
        }
        // Under a port lift thruster, inboard of it, looking up at its nozzle and plate.
        3 => {
            let nozzle = pose.transform_point3(Vec3::new(150.0, -40.0, 45.0));
            let feet = Vec3::new(nozzle.x - 60.0, ground(nozzle.x - 60.0, nozzle.z + 40.0), nozzle.z + 40.0);
            if script.once("under") {
                stand_looking(&mut pilot, feet, nozzle);
            }
            if t > 3.0 && script.once("under shot") {
                shoot(&mut commands, &script.dir, "fortress-under.png");
            }
            next = t > 5.0;
        }
        // On its deck, looking along it to the bridge; the deck carries the pilot as it flies.
        4 => {
            let feet = pose.transform_point3(Vec3::new(10.0, 49.05, -90.0));
            let ahead = pose.transform_point3(Vec3::new(0.0, 60.0, 150.0));
            if script.once("deck") {
                stand_looking(&mut pilot, feet, ahead);
            }
            if t > 4.0 && script.once("deck shot") {
                let local = pose.inverse().transform_point3(pilot.curr.feet);
                println!(
                    "STILLS fortress deck: the pilot's feet at ({:.1}, {:.2}, {:.1}) in the fortress's frame after {t:.1} s (stood at (10.0, 49.05, -90.0))",
                    local.x, local.y, local.z
                );
                shoot(&mut commands, &script.dir, "fortress-deck.png");
            }
            next = t > 6.0;
        }
        // Three port lift thrusters broken from under them: the hull opens.
        5 => {
            // from the open field the first still stands in, 900 m off, clear of the woods
            let off = Vec3::new(1.0, 0.0, 0.6).normalize() * 900.0;
            let feet = Vec3::new(mid.x + off.x, ground(mid.x + off.x, mid.z + off.z), mid.z + off.z);
            if script.once("side") {
                stand_looking(&mut pilot, feet, mid);
            }
            if t < 2.0 {
                for id in ["L1", "L2", "L3"] {
                    shell_from_below(&run, &data, &globals, id, &mut hits);
                }
            }
            if t > 4.0 && script.once("legs shot") {
                shoot(&mut commands, &script.dir, "fortress-legs-falling.png");
            }
            if t > 9.0 && script.once("open shot") {
                shoot(&mut commands, &script.dir, "fortress-hull-open.png");
            }
            next = t > 10.0;
        }
        // Two port hull thrusters broken from under them: it falls, lists and crashes.
        6 => {
            if t < 2.0 {
                for id in ["HL1", "HL2"] {
                    shell_from_below(&run, &data, &globals, id, &mut hits);
                }
            }
            let feet = pilot.curr.feet;
            if t > 8.0 && script.once("falling shot") {
                stand_looking(&mut pilot, feet, mid);
                shoot(&mut commands, &script.dir, "fortress-falling.png");
            }
            let nuke = data.0.fortress.cfg.crash.nuke;
            if f.crashed && script.crashed_at.is_none() {
                // the wreck from the field side, 400 m off; a nuclear one from 1.5 km, the cloud's whole height in view
                script.crashed_at = Some(now);
                let off = Vec3::new(1.0, 0.0, 0.6).normalize() * if nuke.is_some() { 1500.0 } else { 400.0 };
                let near = Vec3::new(mid.x + off.x, ground(mid.x + off.x, mid.z + off.z), mid.z + off.z);
                let look = if nuke.is_some() { mid + Vec3::Y * 350.0 } else { mid };
                stand_looking(&mut pilot, near, look);
            }
            let down_s = script.crashed_at.map_or(0.0, |c| now - c);
            // The nuclear crash (crab-airship section 11): the fireball swelling and the mushroom rising, then the crater
            // from its rim.
            if let Some(n) = nuke {
                for (s, name) in [
                    (1.0, "fortress-nuke-01s.png"),
                    (3.0, "fortress-nuke-03s.png"),
                    (6.0, "fortress-nuke-06s.png"),
                    (12.0, "fortress-nuke-12s.png"),
                    (25.0, "fortress-nuke-25s.png"),
                    (45.0, "fortress-nuke-45s.png"),
                ] {
                    if down_s > s && script.once(name) {
                        shoot(&mut commands, &script.dir, name);
                    }
                }
                if down_s > 47.0 && script.once("rim") {
                    let off = Vec3::new(1.0, 0.0, 0.6).normalize() * (n.crater_radius_m + 60.0);
                    let rim = Vec3::new(mid.x + off.x, ground(mid.x + off.x, mid.z + off.z) + 20.0, mid.z + off.z);
                    let floor = Vec3::new(mid.x, ground(mid.x, mid.z), mid.z);
                    stand_looking(&mut pilot, rim, floor);
                }
                if down_s > 50.0 && script.once("crater shot") {
                    shoot(&mut commands, &script.dir, "fortress-crater.png");
                }
                next = down_s > 52.0 || t > 300.0;
                if next {
                    println!("STILLS fortress done: nuclear crash, outcome {:?}", run.outcome);
                }
                if next {
                    script.beat += 1;
                    script.beat_started_s = now;
                }
                return;
            }
            if down_s > 4.0 && script.once("crashed shot") {
                shoot(&mut commands, &script.dir, "fortress-crashed.png");
            }
            // and from the open field, 900 m off
            if down_s > 6.0 && script.once("crashed far") {
                let off = Vec3::new(1.0, 0.0, 0.6).normalize() * 900.0;
                let far = Vec3::new(mid.x + off.x, ground(mid.x + off.x, mid.z + off.z), mid.z + off.z);
                stand_looking(&mut pilot, far, mid);
            }
            if down_s > 9.0 && script.once("crashed far shot") {
                shoot(&mut commands, &script.dir, "fortress-crashed-far.png");
            }
            next = down_s > 11.0 || t > 240.0;
            if next {
                println!(
                    "STILLS fortress done: crashed {}, outcome {:?}, alt {:.0} m",
                    f.crashed,
                    run.outcome,
                    f.pos.y - f.ground_m
                );
            }
        }
        _ => {}
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
    }
}
