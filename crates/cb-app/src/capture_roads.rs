//! The road stills (a measurement instrument, CLAUDE.md section 4; openspec change `road-surfaces`): with
//! `CB_STILLS_ROADS=1 --stills <dir>`, once the game is ready the pilot is stood on the roads nearest the start and
//! each view is shot as `roads-<view>.png`: a city street and its pavement on foot, the same street from the
//! cockpit's eye height, an arterial, a highway on foot and from the cockpit's height, and a shell's crater on the
//! street. The pilot is placed and turned directly, as the other stills do; the crater is the game's own
//! `Explosion`, the message a shell sends. Drawn offscreen, so a window behind another (macOS) still shoots.

use crate::capture::{render_offscreen, shoot_offscreen, stand_looking};
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::world::StreetKind;
use std::path::PathBuf;

/// Runs the road stills.
pub struct RoadStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

/// The views, in order.
const VIEWS: [&str; 7] = [
    "street-foot",
    "pavement-foot",
    "street-cockpit",
    "arterial-foot",
    "highway-foot",
    "highway-cockpit",
    "crater-street",
];

/// Seconds each view is held before it is shot (the street dresses itself a building a frame).
const HOLD_S: f32 = 5.0;

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    /// 0 waits for the game; then the view `beat - 1`.
    beat: usize,
    beat_started_s: f32,
    /// Where the pilot started, the place the nearest roads are found from.
    start: Option<Vec2>,
    offscreen: Option<Handle<Image>>,
    shots: Vec<PathBuf>,
    blasted: bool,
}

impl Plugin for RoadStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            start: None,
            offscreen: None,
            shots: Vec::new(),
            blasted: false,
        })
        .add_after_world_spawn(offscreen)
        .add_game_systems(Update, run);
    }
}

fn offscreen(
    mut commands: Commands,
    camera: Single<Entity, With<crate::pilot::PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut script: ResMut<Script>,
) {
    script.offscreen = Some(render_offscreen(&mut commands, *camera, *cockpit, &mut images));
}

/// A view: where the feet stand and the point looked at.
fn view(name: &str, plan: &Plan, data: &Data, here: Vec2) -> Option<(Vec3, Vec3)> {
    let cfg = &data.0.world;
    let lift = cfg.road_lift_m;
    // feet raised so the pilot's eye sits at the mech's cockpit eye
    let cockpit_up = (data.0.mech.body.eye_height_m - data.0.pilot.eye_height_m).max(0.0);
    let street = |kind: StreetKind| {
        plan.0
            .cities
            .iter()
            .flat_map(|c| c.streets.iter().map(move |s| (c.ground_m, *s)))
            .filter(|(_, s)| s.kind == kind && s.rect.size().max_element() > 80.0)
            .min_by(|a, b| a.1.rect.distance(here).total_cmp(&b.1.rect.distance(here)))
    };
    let along = |s: &cb_core::world::StreetPiece, side_m: f32, at: f32, ahead_m: f32| {
        let (c, half) = (s.rect.centre(), s.rect.size() * 0.5);
        let (dir, across) = if half.x > half.y { (Vec2::X, Vec2::Y) } else { (Vec2::Y, Vec2::X) };
        let len = half.max_element();
        let foot = c + dir * len * (at - 0.5) + across * side_m;
        (foot, foot + dir * ahead_m - across * side_m * 0.5)
    };
    match name {
        "street-foot" | "street-cockpit" | "crater-street" | "arterial-foot" | "pavement-foot" => {
            let kind = if name == "arterial-foot" {
                StreetKind::Arterial
            } else {
                StreetKind::Street
            };
            let (ground, s) = street(kind)?;
            let y = ground + lift;
            let near_kerb = s.width_m * 0.5 - 1.5;
            let (foot, at, up, ahead_y) = match name {
                "street-cockpit" => {
                    let (f, a) = along(&s, 0.0, 0.25, 70.0);
                    (f, a, cockpit_up, 0.0)
                }
                "pavement-foot" => {
                    // on the pavement 3 m back from the kerb, looking along it and down at the kerb's edge
                    let (f, a) = along(&s, s.width_m * 0.5 + 3.0, 0.3, 12.0);
                    (f, a, cfg.kerb_height_m - lift, -1.0)
                }
                _ => {
                    let (f, a) = along(&s, near_kerb, 0.25, 35.0);
                    (f, a, 0.0, 0.0)
                }
            };
            Some((Vec3::new(foot.x, y + up, foot.y), Vec3::new(at.x, y + ahead_y, at.y)))
        }
        "highway-foot" | "highway-cockpit" => {
            let h = plan
                .0
                .highways
                .iter()
                .min_by(|a, b| a.point_at(0.5).distance(here).total_cmp(&b.point_at(0.5).distance(here)))?;
            let t = 0.5;
            let (side, ahead, up) = if name == "highway-foot" {
                (h.width_m * 0.5 - 1.5, 35.0, 0.0)
            } else {
                (0.0, 90.0, cockpit_up)
            };
            let f = h.point_at(t) + h.dir_at(t).perp() * side;
            let ta = (t + ahead / h.length_m()).min(1.0);
            let a = h.point_at(ta) + h.dir_at(ta).perp() * side * 0.5;
            Some((
                Vec3::new(f.x, h.grade_at(t) + lift + up, f.y),
                Vec3::new(a.x, h.grade_at(ta) + lift, a.y),
            ))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut pilot: ResMut<PilotSim>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Real>>,
    ready: Option<Res<crate::loading::GameReady>>,
    mut blasts: MessageWriter<crate::blast::Explosion>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    let next = if script.beat == 0 {
        let go = ready.is_some_and(|r| r.0) && t > 6.0;
        if go {
            let f = pilot.curr.feet;
            script.start = Some(Vec2::new(f.x, f.z));
        }
        go
    } else if let Some(&name) = VIEWS.get(script.beat - 1) {
        let here = script.start.unwrap_or_default();
        let Some((feet, look)) = view(name, &plan, &data, here) else {
            eprintln!("road stills: no place for {name}");
            exit.write(AppExit::error());
            return;
        };
        stand_looking(&mut pilot, feet, look);
        if name == "crater-street" && !script.blasted {
            // a 120 mm shell's 4 MJ on the street 18 m ahead
            let at = feet + (look - feet).with_y(0.0).normalize_or_zero() * 18.0;
            blasts.write(crate::blast::Explosion {
                at: at.with_y(feet.y),
                energy_mj: 4.0,
                by: None,
            });
            script.blasted = true;
        }
        if t > HOLD_S {
            let file = format!("roads-{name}.png");
            if let Some(img) = script.offscreen.clone() {
                shoot_offscreen(&mut commands, img, &script.dir.clone(), &file);
            }
            let path = script.dir.join(&file);
            script.shots.push(path);
            println!("STILLS_ROADS {name} feet ({:.1}, {:.1}, {:.1})", feet.x, feet.y, feet.z);
        }
        t > HOLD_S
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
