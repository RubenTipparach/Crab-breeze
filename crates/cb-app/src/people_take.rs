//! The people take (`--showcase people`, `--bench people`; openspec change `people-vat`, design section 8): a
//! measurement and capture instrument (CLAUDE.md section 4). The pilot is set down on foot on the pavement of
//! the downtown block of the city nearest the start, waits while the crowd fills (up to `max_people`), then
//! walks two laps of the block's pavement with the crowd round them: the city route the people's A/B is
//! measured on. With `CB_PEOPLE_STILLS=<dir>` it shoots stills instead: a walker, someone waiting at the
//! kerb, someone on a bench, a greeting (the wave, then the talk), the street at distance, and a shell
//! among a group (running, falling). It writes input actions as a player does, except that it places the
//! pilot and turns their view at the start of each shot. It prints `PLAYTEST_START`, `PLAYTEST_RESULT`,
//! `PLAYTEST_END` and `PEOPLE_COUNT` lines.

use crate::civilians::Crowd;
use crate::input::InputAccum;
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::world::Plan;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use cb_core::civilians::{Activity, Person};
use cb_core::input::{Actions, DebugCommand};
use cb_core::pilot::PilotState;
use cb_core::world::Rect;
use std::path::PathBuf;

/// The beats' names, for the benchmark's report.
pub const TAKE_BEATS: [&str; 5] = ["load", "fill", "lap one", "lap two", "end"];
/// The stills' beats.
const STILLS: [&str; 9] = ["walker", "kerb", "bench", "greet", "street", "far", "shell", "aftermath", "end"];
/// Game seconds the crowd is given to fill round the pilot (60 people a fixed step, re-looked every 8 m).
const FILL_S: f32 = 14.0;
/// Seconds a lap of the route may take before the take moves on.
const LAP_S: f32 = 45.0;

/// The take's state.
#[derive(Resource, Default)]
pub struct PeopleTake {
    beat_started_s: f32,
    /// The route: the downtown block's pavement loop, 2 m in from the kerb.
    route: Option<Rect>,
    /// The ground the route's pavement is at.
    route_y: f32,
    /// The waypoint walked to next.
    waypoint: u8,
    /// The stills' folder, when shooting stills.
    stills: Option<PathBuf>,
    /// The person the current still is of.
    subject: Option<u32>,
    shot: u32,
    /// The offscreen image stills are taken from.
    offscreen: Option<Handle<Image>>,
    /// The current shot's key (Use, the test shell) is pressed.
    pressed: bool,
    last_count_s: f32,
}

/// Registers the take.
pub fn add(app: &mut App) {
    app.insert_resource(PeopleTake {
        stills: std::env::var_os("CB_PEOPLE_STILLS").map(PathBuf::from),
        ..default()
    })
    .add_systems(
        Update,
        play.in_set(crate::sim::PresentSet::Input)
            .run_if(in_state(crate::state::GameState::Playing)),
    );
    if std::env::var_os("CB_PEOPLE_STILLS").is_some() {
        crate::state::GameApp::add_after_world_spawn(app, offscreen);
    }
}

/// Stills draw the pilot's eye and the cockpit pass into an offscreen 1920 x 1080 image, as the building stills
/// do: window shots come out black on macOS when the window is behind another. Never in a measured run.
fn offscreen(
    mut commands: Commands,
    camera: Single<Entity, With<crate::pilot::PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut take: ResMut<PeopleTake>,
) {
    use bevy::camera::RenderTarget;
    use bevy::render::render_resource::TextureFormat;
    let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb));
    let handle = images.add(image);
    commands.entity(*camera).insert(RenderTarget::Image(handle.clone().into()));
    commands.entity(*cockpit).insert(RenderTarget::Image(handle.clone().into()));
    take.offscreen = Some(handle);
}

/// Corner `k` (0-3) of a rectangle, then the next round it.
fn corner(r: &Rect, k: u8) -> Vec2 {
    match k % 4 {
        0 => r.min,
        1 => Vec2::new(r.max.x, r.min.y),
        2 => r.max,
        _ => Vec2::new(r.min.x, r.max.y),
    }
}

/// The view turned onto `at` (a point in the world) from feet `feet`.
fn look_at(feet: Vec3, at: Vec3) -> (f32, f32) {
    let to = at - (feet + Vec3::Y * 1.7);
    ((-to.x).atan2(-to.z), to.y.atan2(Vec2::new(to.x, to.z).length()))
}

/// Puts the pilot down at `feet` looking at `at`.
fn place(pilot: &mut PilotSim, feet: Vec3, at: Vec3) {
    let (yaw, pitch) = look_at(feet, at);
    let mut st = PilotState::standing(feet, yaw);
    st.pitch_rad = pitch;
    pilot.prev = st.clone();
    pilot.curr = st;
}

/// The nearest person to `from` that `want` accepts.
fn nearest(crowd: &Crowd, from: Vec3, want: impl Fn(&Person) -> bool) -> Option<&Person> {
    crowd
        .people
        .list
        .iter()
        .filter(|p| want(p))
        .min_by(|a, b| a.feet().distance_squared(from).total_cmp(&b.feet().distance_squared(from)))
}

fn shoot(dir: &std::path::Path, name: &str, commands: &mut Commands, image: Option<Handle<Image>>) {
    let path = dir.join(format!("{name}.png"));
    println!("PEOPLE_STILL {}", path.display());
    let shot = match image {
        Some(img) => Screenshot::image(img),
        None => Screenshot::primary_window(),
    };
    commands.spawn(shot).observe(save_to_disk(path));
}

#[allow(clippy::too_many_arguments)]
fn play(
    mut commands: Commands,
    mut take: ResMut<PeopleTake>,
    mut beat: ResMut<crate::showcase::Take>,
    mut acc: ResMut<InputAccum>,
    mut pilot: ResMut<PilotSim>,
    crowd: Option<Res<Crowd>>,
    plan: Res<Plan>,
    data: Res<Data>,
    ready: Res<crate::loading::GameReady>,
    time: Res<Time<Virtual>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let in_beat = now - take.beat_started_s;
    let count = crowd.as_ref().map_or(0, |c| c.people.list.len());
    if take.route.is_some() && now - take.last_count_s >= 5.0 {
        take.last_count_s = now;
        println!("PEOPLE_COUNT beat={} people={count}", TAKE_BEATS[beat.beat.min(4)]);
    }
    let mut next = false;
    if beat.beat == 0 {
        if !ready.0 {
            return;
        }
        // the downtown block: the city nearest the start, its building block nearest its centre
        let start = plan.0.start.pilot_xz;
        let Some(city) = plan
            .0
            .cities
            .iter()
            .min_by(|a, b| a.centre.distance(start).total_cmp(&b.centre.distance(start)))
        else {
            println!("PLAYTEST_RESULT fail people: no city in the world");
            exit.write(AppExit::error());
            return;
        };
        let Some(block) = city.blocks.iter().filter(|b| !b.park).min_by(|a, b| {
            a.rect
                .centre()
                .distance(city.centre)
                .total_cmp(&b.rect.centre().distance(city.centre))
        }) else {
            return;
        };
        let Some(route) = block.rect.inset(2.0) else { return };
        take.route = Some(route);
        take.route_y = city.ground_m + data.0.world.kerb_height_m;
        let feet = corner(&route, 0).extend(take.route_y).xzy();
        place(&mut pilot, feet, corner(&route, 1).extend(take.route_y + 1.7).xzy());
        println!("PLAYTEST_START people");
        next = true;
    } else if take.stills.is_some() {
        next = stills(
            &mut commands,
            &mut take,
            beat.beat - 1,
            in_beat,
            &mut pilot,
            &mut acc,
            crowd.as_deref(),
            &data,
        );
        if beat.beat >= STILLS.len() {
            println!("PLAYTEST_RESULT ok people stills: {count} people");
            println!("PLAYTEST_END people");
            exit.write(AppExit::Success);
        }
    } else {
        match beat.beat {
            1 => next = in_beat >= FILL_S,
            2 | 3 => {
                let route = take.route.expect("set in the load beat");
                let target = corner(&route, take.waypoint).extend(take.route_y).xzy();
                acc.script = Some(crate::showcase::walk_to(&pilot.curr, target, 0.0));
                if Vec2::new(pilot.curr.feet.x - target.x, pilot.curr.feet.z - target.z).length() < 2.5 {
                    take.waypoint += 1;
                }
                next = take.waypoint >= 4 * (beat.beat as u8 - 1) || in_beat >= LAP_S;
            }
            _ => {
                acc.script = Some(Actions::default());
                let verdict = if count >= 1000 { "ok" } else { "fail" };
                println!("PLAYTEST_RESULT {verdict} people: {count} people round the route");
                println!("PLAYTEST_END people");
                exit.write(AppExit::Success);
            }
        }
    }
    if next {
        beat.beat += 1;
        take.beat_started_s = now;
        take.subject = None;
        take.shot = 0;
        take.pressed = false;
    }
}

/// One stills beat; true when it is done.
#[allow(clippy::too_many_arguments)]
fn stills(
    commands: &mut Commands,
    take: &mut PeopleTake,
    i: usize,
    in_beat: f32,
    pilot: &mut PilotSim,
    acc: &mut InputAccum,
    crowd: Option<&Crowd>,
    data: &Data,
) -> bool {
    let Some(dir) = take.stills.clone() else { return true };
    let Some(crowd) = crowd else { return true };
    if i == 0 && in_beat < FILL_S {
        return false;
    }
    acc.script = Some(Actions::default());
    let route = take.route.expect("set in the load beat");
    let here = pilot.curr.feet;
    let find = |id: u32| crowd.people.list.iter().find(|p| p.id == id);
    let name = STILLS[i.min(STILLS.len() - 1)];
    let wait = if i == 0 { FILL_S } else { 0.0 };
    let t = in_beat - wait;
    // choose the subject and put the pilot down by them once, at the start of the shot
    if take.subject.is_none() && t >= 0.0 {
        let pick: Option<&Person> = match name {
            "walker" => nearest(crowd, here, |p| matches!(p.activity, Activity::Walking) && p.pace_m_s > 0.8),
            "kerb" => nearest(crowd, here, |p| matches!(p.activity, Activity::Waiting { .. })),
            "bench" => nearest(crowd, here, |p| matches!(p.activity, Activity::Sitting { left_s } if left_s > 8.0)),
            "greet" | "shell" => nearest(crowd, here, |p| matches!(p.activity, Activity::Walking | Activity::Idle { .. })),
            _ => None,
        };
        match (name, pick) {
            ("street" | "far", _) => {
                let back = if name == "far" { 120.0 } else { 0.0 };
                let a = corner(&route, 0);
                let b = corner(&route, 1);
                let along = (b - a).normalize_or(Vec2::X);
                let feet = (a - along * back).extend(take.route_y).xzy();
                let at = (b + along * 80.0).extend(take.route_y + 1.2).xzy();
                place(pilot, feet, at);
                take.subject = Some(u32::MAX);
            }
            (_, Some(p)) => {
                let fwd = Vec2::new(-p.heading_rad.sin(), -p.heading_rad.cos());
                let side = Vec2::new(-fwd.y, fwd.x);
                let (off, chest) = match name {
                    "walker" => (fwd * 5.0 + side * 4.0, 1.1),
                    "kerb" => (side * 4.0 + fwd * 3.0, 1.1),
                    "bench" => (fwd * 3.2 + side * 0.8, 0.8),
                    "greet" => (fwd * 1.7, 1.5),
                    _ => (fwd * 16.0, 0.0),
                };
                let at = p.pos + off;
                let feet = Vec3::new(at.x, p.feet_y, at.y);
                place(pilot, feet, p.feet() + Vec3::Y * chest);
                take.subject = Some(p.id);
            }
            ("aftermath", None) => take.subject = Some(u32::MAX),
            _ => {
                println!("PEOPLE_STILL none for {name}");
                return in_beat > wait + 5.0;
            }
        }
    }
    let Some(id) = take.subject else { return false };
    // keep the view on a moving subject, as a player would follow them with the mouse
    if let Some(p) = find(id)
        && matches!(name, "walker" | "greet")
    {
        let (yaw, pitch) = look_at(pilot.curr.feet, p.feet() + Vec3::Y * if name == "greet" { 1.5 } else { 1.1 });
        pilot.curr.yaw_rad = yaw;
        pilot.curr.pitch_rad = pitch;
    }
    let mut snap = |at: f32, label: &str, take: &mut PeopleTake| {
        if t >= at && take.shot == shot_index(name, label) {
            shoot(&dir, &format!("{:02}-{label}", i + 1), commands, take.offscreen.clone());
            take.shot += 1;
        }
    };
    match name {
        "greet" => {
            if t >= 0.2 && !take.pressed {
                take.pressed = true;
                acc.use_pressed = true;
            }
            snap(1.0, "greet-wave", take);
            snap(4.2, "greet-talk", take);
            t > 4.5
        }
        "shell" => {
            if t >= 0.2 && !take.pressed {
                take.pressed = true;
                acc.debug = Some(DebugCommand::Shell);
            }
            snap(1.2, "shell-running", take);
            t > 1.5
        }
        "aftermath" => {
            snap(2.5, "shell-fallen", take);
            t > 3.0
        }
        "end" => true,
        _ => {
            snap(if name == "walker" { 0.6 } else { 1.0 }, name, take);
            t > 1.5 && data.0.people.cfg.max_people > 0
        }
    }
}

/// The index of a shot within its beat (the greeting takes two).
fn shot_index(beat: &str, label: &str) -> u32 {
    u32::from(beat == "greet" && label == "greet-talk")
}
