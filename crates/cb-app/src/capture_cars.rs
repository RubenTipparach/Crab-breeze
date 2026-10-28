//! The car stills (a measurement instrument, CLAUDE.md section 4; openspec change `car-chase-camera`, the cars'
//! height): with `CB_STILLS_CARS=1 --stills <dir>`, once the game is ready the pilot stands on foot beside the
//! nearest sedan and then the nearest bus (the parked or slowest first), at the driver's side and from a three
//! quarter view further out, and each is shot as `cars-<kind>-<view>.png`; then it exits. Each shot logs the car's
//! ground point, the bare ground and the drawn road under it, its roof and the pilot's eye, so how tall the car
//! reads against the pilot is a number as well as a picture. The pilot is placed and turned directly, as the other
//! stills do; it writes no input and plays no game.
//!
//! With `CB_STILLS_CARS=drive` it shoots the chase camera instead (openspec change `car-chase-camera`), drawn
//! offscreen so a window behind another cannot save black: the pilot is put at the driver's door of a parked car away
//! from the mech and from there plays as a player would, through input actions: Use to get in (shot getting in, in the
//! seat and once the camera is out behind), throttle (shot moving), the mouse turned 120 degrees (shot orbited), the
//! mouse still while it drives on (shot drifted back behind), the handbrake to a stop and Use to get out (shot getting
//! out and on foot). Each shot logs the camera's distance and height from the car.

use crate::capture::{shoot, stand_looking};
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::GameApp;
use crate::traffic::TrafficSim;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::car::CarSpec;
use cb_core::world::Ground;
use std::path::PathBuf;

/// Runs the car stills.
pub struct CarStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

/// A shot: the car kind, the view's name, and where the pilot stands in the car's frame (x right, z back, metres;
/// the car faces -z).
const SHOTS: [(&str, &str, [f32; 2]); 4] = [
    ("sedan", "side", [-3.2, 0.4]),
    ("sedan", "quarter", [-4.5, -7.5]),
    ("bus", "side", [-7.0, -1.0]),
    ("bus", "quarter", [-8.0, -16.0]),
];

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    /// The shot being taken (0 waits for the game to be ready).
    beat: usize,
    beat_started_s: f32,
    /// The car each kind is shot beside, picked once.
    picked: Vec<(&'static str, u32)>,
    shots: Vec<PathBuf>,
    /// The drive variant: its offscreen image, its stage and the mouse turned so far in the orbit, degrees.
    offscreen: Option<Handle<Image>>,
    stage: usize,
    stage_started_s: f32,
    orbited_deg: f32,
    shot_in_stage: u32,
}

impl Plugin for CarStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            picked: Vec::new(),
            shots: Vec::new(),
            offscreen: None,
            stage: 0,
            stage_started_s: 0.0,
            orbited_deg: 0.0,
            shot_in_stage: 0,
        })
        .add_after_world_spawn(offscreen_for_drive)
        .add_game_systems(Update, (run, drive).chain());
    }
}

/// True for the drive variant (`CB_STILLS_CARS=drive`).
fn drive_variant() -> bool {
    std::env::var("CB_STILLS_CARS").is_ok_and(|v| v == "drive")
}

/// The drive variant draws into an offscreen image.
fn offscreen_for_drive(
    mut commands: Commands,
    camera: Single<Entity, With<crate::pilot::PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut script: ResMut<Script>,
) {
    if drive_variant() {
        script.offscreen = Some(crate::capture::draw_offscreen(&mut commands, *camera, *cockpit, &mut images));
    }
}

/// The drive variant's stages (see the module comment).
#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut pilot: ResMut<PilotSim>,
    mut acc: ResMut<crate::input::InputAccum>,
    car: Res<crate::traffic::CarAccessNow>,
    access: Res<crate::mech::MechAccessNow>,
    traffic: Res<TrafficSim>,
    roads: Res<crate::traffic::Roads>,
    data: Res<Data>,
    camera: Single<&Transform, With<crate::pilot::PilotCamera>>,
    time: Res<Time<Real>>,
    ready: Option<Res<crate::loading::GameReady>>,
    mut exit: MessageWriter<AppExit>,
) {
    use cb_core::input::Actions;
    use cb_core::pilot::PilotMode;
    if !drive_variant() {
        return;
    }
    let now = time.elapsed_secs();
    let t = now - script.stage_started_s;
    let mode = pilot.curr.mode.clone();
    let speed = car.0.as_ref().map_or(0.0, |c| c.speed_m_s);
    let mut shot = |script: &mut Script, name: &str| {
        let Some(img) = script.offscreen.clone() else { return };
        let path = script.dir.join(format!("drive-{name}.png"));
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(img))
            .observe(bevy::render::view::screenshot::save_to_disk(path.clone()));
        script.shots.push(path);
        let cam = camera.translation;
        let line = match car.0.as_ref() {
            Some(c) => format!(
                "camera {:.1} m from the car's middle, {:.1} m over its ground, car at {:.1} m/s",
                Vec2::new(cam.x - c.centre.x, cam.z - c.centre.z).length(),
                cam.y - c.centre.y,
                c.speed_m_s
            ),
            None => "no car".into(),
        };
        println!("STILLS_DRIVE {name} mode {:?} {line}", std::mem::discriminant(&pilot.curr.mode));
    };
    let mut next = false;
    match script.stage {
        // ready, then a parked car well away from the mech: stand at its door facing it
        0 => next = ready.is_some_and(|r| r.0) && t > 6.0,
        1 => {
            let mech = access.0.map(|a| a.ladder_base).unwrap_or(Vec3::ZERO);
            let feet0 = pilot.curr.feet;
            let id = match script.picked.iter().find(|(n, _)| *n == "drive") {
                Some(&(_, id)) => id,
                None => {
                    let Some(c) = traffic
                        .traffic
                        .cars
                        .iter()
                        // parked away from the mech, with 150 m of its street ahead (a run long enough to drive,
                        // orbit and let the view drift back before the junction)
                        .filter(|c| {
                            c.state == cb_core::traffic::CarState::Parked
                                && c.pos.distance(mech) > 120.0
                                && roads.0.lanes[c.lane as usize].length_m() - c.s_m > 150.0
                        })
                        .min_by(|a, b| a.pos.distance(feet0).total_cmp(&b.pos.distance(feet0)))
                    else {
                        eprintln!("car stills: no parked car away from the mech");
                        exit.write(AppExit::error());
                        return;
                    };
                    script.picked.push(("drive", c.id));
                    c.id
                }
            };
            let Some(c) = traffic.traffic.get(id) else { return };
            let spec = &data.0.traffic.kinds[c.kind as usize];
            let door = CarSpec::to_world(c.pos, c.heading_rad, spec.door_driver_m);
            let out = CarSpec::to_world(c.pos, c.heading_rad, [spec.door_driver_m[0] - 0.4, 0.0, spec.door_driver_m[2]]);
            let feet = Vec3::new(out.x, c.pos.y, out.z);
            stand_looking(&mut pilot, feet, c.pos + Vec3::Y * 1.0);
            let _ = door;
            if t > 4.0 {
                acc.use_pressed = true;
                next = true;
            }
        }
        // getting in: from the eyes, then the camera eases out behind
        2 => {
            if script.shot_in_stage == 0 && matches!(mode, PilotMode::EnteringCar(_)) && t > 0.35 {
                shot(&mut script, "0-getting-in");
                script.shot_in_stage = 1;
            }
            if mode == PilotMode::Driving && script.shot_in_stage == 1 {
                script.shot_in_stage = 2;
                script.orbited_deg = now;
            }
            if script.shot_in_stage == 2 && now - script.orbited_deg > 0.25 {
                shot(&mut script, "1-easing-out");
                script.shot_in_stage = 3;
            }
            if script.shot_in_stage == 3 && now - script.orbited_deg > 1.5 {
                shot(&mut script, "2-chase-parked");
                next = true;
            }
            if t > 12.0 && !next {
                eprintln!("car stills: never got in ({mode:?})");
                exit.write(AppExit::error());
            }
        }
        // drive away
        3 => {
            // out from the kerb into the lane (left, then straight), then along it
            let steer = if t < 1.0 {
                0.6
            } else if t < 2.0 {
                -0.6
            } else {
                0.0
            };
            acc.script = Some(Actions {
                move_xy: Vec2::new(steer, 0.2),
                ..Default::default()
            });
            if t > 4.0 {
                shot(&mut script, "3-driving");
                script.orbited_deg = 0.0;
                next = true;
            }
        }
        // the mouse turns the camera 120 degrees round the car
        4 => {
            acc.script = Some(Actions {
                move_xy: Vec2::new(0.0, 0.2),
                ..Default::default()
            });
            if script.orbited_deg < 120.0 {
                acc.look_deg.x += 3.0;
                script.orbited_deg += 3.0;
                script.stage_started_s = now;
            } else if t > 0.3 {
                shot(&mut script, "4-orbited");
                next = true;
            }
        }
        // the mouse still: the view drifts back behind the moving car
        5 => {
            acc.script = Some(Actions {
                move_xy: Vec2::new(0.0, 0.2),
                ..Default::default()
            });
            if t > data.0.pilot.car_camera.recenter_after_s + 2.5 {
                shot(&mut script, "5-mouse-still");
                next = true;
            }
        }
        // the handbrake to a stop, then Use to get out
        6 => {
            acc.script = Some(Actions {
                lift_axis: 1.0,
                ..Default::default()
            });
            if speed < 0.3 && t > 1.0 {
                acc.script = Some(Actions::default());
                acc.use_pressed = true;
                next = true;
            }
        }
        7 => {
            if script.shot_in_stage != 7 && matches!(mode, PilotMode::ExitingCar(_)) && t > 0.3 {
                shot(&mut script, "6-getting-out");
                script.shot_in_stage = 7;
            }
            if mode == PilotMode::OnFoot && t > 1.5 {
                shot(&mut script, "7-on-foot");
                next = true;
            }
            if t > 12.0 && !next {
                eprintln!("car stills: never got out ({mode:?})");
                exit.write(AppExit::error());
            }
        }
        _ => {
            if script.shots.iter().all(|p| p.exists()) || t > 30.0 {
                exit.write(AppExit::Success);
            }
        }
    }
    if next {
        script.stage += 1;
        script.stage_started_s = now;
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut pilot: ResMut<PilotSim>,
    traffic: Res<TrafficSim>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Real>>,
    ready: Option<Res<crate::loading::GameReady>>,
    mut exit: MessageWriter<AppExit>,
) {
    if drive_variant() {
        return;
    }
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    let d = &data.0.traffic;
    let next = if script.beat == 0 {
        ready.is_some_and(|r| r.0) && t > 6.0
    } else if let Some(&(kind, view, at)) = SHOTS.get(script.beat - 1) {
        let Some(k) = d.kind(kind) else {
            exit.write(AppExit::error());
            return;
        };
        let feet0 = pilot.curr.feet;
        let id = match script.picked.iter().find(|(n, _)| *n == kind) {
            Some(&(_, id)) => id,
            None => {
                // the parked or slowest car of the kind nearest the pilot
                let Some(c) = traffic
                    .traffic
                    .cars
                    .iter()
                    .filter(|c| c.kind as usize == k && !c.state.wreck())
                    .min_by(|a, b| {
                        let key = |c: &cb_core::traffic::Car| c.pos.distance(feet0) + c.vel().length() * 200.0;
                        key(a).total_cmp(&key(b))
                    })
                else {
                    eprintln!("car stills: no {kind} in traffic");
                    exit.write(AppExit::error());
                    return;
                };
                script.picked.push((kind, c.id));
                c.id
            }
        };
        let Some(c) = traffic.traffic.get(id) else {
            exit.write(AppExit::error());
            return;
        };
        let spec = &d.kinds[k];
        // held beside the car every frame (it may be driving)
        let stand = CarSpec::to_world(c.pos, c.heading_rad, [at[0], 0.0, at[1]]);
        let feet = Vec3::new(
            stand.x,
            plan.walk_top_at(Vec2::new(stand.x, stand.z))
                .unwrap_or_else(|| plan.height_at(stand.x, stand.z)),
            stand.z,
        );
        let look = c.pos + Vec3::Y * spec.size_m[1] * 0.55;
        stand_looking(&mut pilot, feet, look);
        let wait = 5.0;
        if t > wait {
            let name = format!("cars-{kind}-{view}.png");
            shoot(&mut commands, &script.dir.clone(), &name);
            let path = script.dir.join(&name);
            script.shots.push(path);
            let road = plan.road_top_at(Vec2::new(c.pos.x, c.pos.z));
            println!(
                "STILLS_CARS {kind} {view} car_y {:.2} ground {:.2} road_top {} roof {:.2} eye {:.2} (pilot feet {:.2}) height {:.2} m",
                c.pos.y,
                plan.height_at(c.pos.x, c.pos.z),
                road.map_or("none".to_string(), |r| format!("{r:.2}")),
                c.pos.y + spec.size_m[1],
                feet.y + data.0.pilot.eye_height_m,
                feet.y,
                spec.size_m[1],
            );
        }
        t > wait
    } else {
        if script.shots.iter().all(|p| p.exists()) || t > 60.0 {
            exit.write(AppExit::Success);
        }
        false
    };
    if next {
        script.beat += 1;
        script.beat_started_s = now;
    }
}
