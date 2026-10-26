//! The car stills (a measurement instrument, CLAUDE.md section 4; openspec change `car-chase-camera`, the cars'
//! height): with `CB_STILLS_CARS=1 --stills <dir>`, once the game is ready the pilot stands on foot beside the
//! nearest sedan and then the nearest bus (the parked or slowest first), at the driver's side and from a three
//! quarter view further out, and each is shot as `cars-<kind>-<view>.png`; then it exits. Each shot logs the car's
//! ground point, the bare ground and the drawn road under it, its roof and the pilot's eye, so how tall the car
//! reads against the pilot is a number as well as a picture. The pilot is placed and turned directly, as the other
//! stills do; it writes no input and plays no game.

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
        })
        .add_game_systems(Update, run);
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
        let feet = Vec3::new(stand.x, plan.height_at(stand.x, stand.z), stand.z);
        let look = c.pos + Vec3::Y * spec.size_m[1] * 0.55;
        stand_looking(&mut pilot, feet, look);
        let wait = 5.0;
        if t > wait {
            let name = format!("cars-{kind}-{view}.png");
            shoot(&mut commands, &script.dir.clone(), &name);
            let path = script.dir.join(&name);
            script.shots.push(path);
            let lift = data.0.world.road_lift_m;
            let road = plan.road_top_at(Vec2::new(c.pos.x, c.pos.z), lift);
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
