//! The scripted pilot (openspec change `playtest-pipeline`): `--playtest <name> [--run <i>]` plays the beats of
//! `data/playtests/<name>.ron` (`cb_core::playtest`) through the game as it is played, in first person. It writes only
//! what a player's hands would: the input actions (walk, look, throttle, steer, Use, fire, through `InputAccum`), key
//! presses (Use, Escape) and mouse clicks on the menu's buttons (pointer moves, presses and releases, the same messages
//! a real mouse sends to the picking pipeline). It reads what a player could see. It prints `PLAYTEST_READY`,
//! `PLAYTEST_START`, `PLAYTEST_BEAT`, `PLAYTEST_RESULT` and `PLAYTEST_END` for the recorder, and a beat that makes no
//! progress for `stuck_s` of game time ends the run `stuck`. A capture instrument (CLAUDE.md section 4).

use crate::career::{CareerRun, JobEnemy};
use crate::input::InputAccum;
use crate::mech::{MechAccessNow, MechSim, MechSpec, Piloted};
use crate::menu::{Menu, MenuButton, TerminalTab};
use crate::pilot::PilotSim;
use crate::showcase::{Aim, board, brain_actions, steer, throttle_to, walk_to};
use crate::world::{Live, Plan};
use bevy::camera::RenderTarget;
use bevy::ecs::system::SystemParam;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowRef};
use cb_core::input::Actions;
use cb_core::jobs::JobState;
use cb_core::pilot::{PilotMode, wrap_rad};
use cb_core::playtest::{Beat, Expect, Pick, Script};
use std::f32::consts::PI;

/// Plays one run of a playtest script.
pub struct PlaytestPlugin {
    /// The script.
    pub script: Script,
    /// Which of its runs this launch plays.
    pub run: usize,
}

/// Loads `data/playtests/<name>.ron`, refusing a bad file with its path and field.
pub fn load(name: &str) -> Result<Script, cb_core::data::DataError> {
    cb_core::data::load_ron(&crate::cli::repo_root().join("data/playtests").join(format!("{name}.ron")))
}

/// A mouse click in progress on a menu button: where, and how far it has got.
#[derive(Clone, Copy)]
struct Click {
    at: Vec2,
    stage: u8,
    since_s: f32,
}

/// Marks a house lot's id in `Pilot::shot`, apart from the buildings' ids.
const HOUSE_ID: u32 = 1 << 31;

#[derive(Resource)]
struct Pilot {
    script: Script,
    run: usize,
    beat: usize,
    started: bool,
    beat_real_s: f32,
    /// Game seconds at the beat's start and at its last progress, and the best distance to its goal so far.
    beat_game_s: f32,
    progress_game_s: f32,
    best_m: f32,
    /// A leg of a two-leg walk or drive.
    leg: usize,
    click: Option<Click>,
    clicked: bool,
    brain: Option<cb_core::ai::AiPilot>,
    service_seen: f32,
    /// When the skates were last toggled (real seconds): one press, then wait for it to take.
    toggled_s: f32,
    /// When the track was last logged.
    logged_s: f32,
    /// When the world was first seen ready (a Continue is restored on the frame after).
    ready_s: Option<f32>,
    failures: Vec<String>,
    ending: Option<f32>,
    /// A long drive blocked: how long the mech has stood still, real seconds.
    stall_s: f32,
    /// The beat's top speed since the last detour, m/s: a mech that never got going is not blocked.
    peak_m_s: f32,
    /// A detour round an obstacle: the point steered for, until when, and the side taken last.
    detour: Vec2,
    detour_until: f32,
    detour_side: f32,
    /// Jobs the pilot could not reach and gave up on (`AcceptByMail` passes them over).
    gave_up: Vec<u32>,
    /// The buildings and houses (`HOUSE_ID` set) `FireAtNearest` has fired at, and where it aims now.
    shot: Vec<u32>,
    aim: Vec3,
    /// The person, car or building a beat chose, kept while it lasts.
    goal_id: Option<u32>,
    /// A beat's goal on the ground (x, z).
    goal: Vec2,
    /// A walk's points (feet) and the one walked to now.
    route: Vec<Vec3>,
    route_i: usize,
    /// A beat's count (notches turned, presses made, shots fired) and when it last acted, real seconds.
    count: u32,
    next_s: f32,
    /// What a beat saw at its start, compared at its end (an MFD's page, the jets' heat).
    seen: Option<f32>,
    /// When `PLAYTEST_START` was said, real seconds: the take's clock, for finding a moment in the video.
    start_s: f32,
    /// A jet leg is resting the jets, on the skates (`cb_core::playtest::rest_jets`).
    jets_resting: bool,
}

/// Keys the scripted pilot presses, applied after the devices are read so the game sees them as a key's press.
#[derive(Resource, Default)]
struct PressKey(Option<(KeyCode, bool)>);

impl Plugin for PlaytestPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Pilot {
            script: self.script.clone(),
            run: self.run,
            beat: 0,
            started: false,
            beat_real_s: 0.0,
            beat_game_s: 0.0,
            progress_game_s: 0.0,
            best_m: f32::INFINITY,
            leg: 0,
            click: None,
            clicked: false,
            brain: None,
            service_seen: 0.0,
            toggled_s: 0.0,
            logged_s: 0.0,
            ready_s: None,
            failures: Vec::new(),
            ending: None,
            stall_s: 0.0,
            peak_m_s: 0.0,
            detour: Vec2::ZERO,
            detour_until: 0.0,
            detour_side: 1.0,
            gave_up: Vec::new(),
            shot: Vec::new(),
            aim: Vec3::ZERO,
            goal_id: None,
            goal: Vec2::ZERO,
            route: Vec::new(),
            route_i: 0,
            count: 0,
            next_s: 0.0,
            seen: None,
            start_s: 0.0,
            jets_resting: false,
        })
        .init_resource::<PressKey>()
        .init_resource::<Hands>()
        .add_systems(PreUpdate, turn_wheel.before(bevy::input::InputSystems))
        .add_systems(PreUpdate, (press_keys, hold_buttons).after(bevy::input::InputSystems))
        .add_systems(Update, play.in_set(crate::sim::PresentSet::Input))
        .init_resource::<FrameLog>()
        .add_systems(Last, log_frames);
    }
}

/// Every frame of a take, for the performance report (`tools/perf/free_roam_report.py`): real seconds, the frame's
/// milliseconds, the beat, where the mech is and how fast it goes, the pilot's mode, the job's enemies alive and the
/// entity count. Appended to `output/perf/<script>-frames.csv` every few seconds. A measurement instrument.
#[derive(Resource, Default)]
struct FrameLog {
    lines: String,
    path: Option<std::path::PathBuf>,
    written_s: f32,
    /// The worst frame of the last second, and when the second began, for the one-line `PERF` log.
    worst_ms: f32,
    frames: u32,
    second_s: f32,
}

fn log_frames(mut log: ResMut<FrameLog>, t: Res<Pilot>, s: Scene, entities: &bevy::ecs::entity::Entities) {
    if !t.started {
        return;
    }
    let now = s.real.elapsed_secs();
    let ms = s.real.delta_secs() * 1000.0;
    let Some((sim, _)) = s.me.iter().next() else { return };
    let m = &sim.curr;
    let alive = s.enemies.iter().filter(|(e, _)| !e.dead).count();
    let mode = match s.pilot.curr.mode {
        PilotMode::Seated => "seated",
        PilotMode::OnFoot => "foot",
        _ => "other",
    };
    if log.path.is_none() {
        let dir = crate::cli::repo_root().join("output/perf");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}-frames.csv", t.script.name));
        let _ = std::fs::write(&path, "real_s,frame_ms,beat,x,z,y,speed_m_s,mode,skates,enemies,entities\n");
        log.path = Some(path);
        log.second_s = now;
    }
    let line = format!(
        "{now:.3},{ms:.2},{},{:.0},{:.0},{:.0},{:.1},{mode},{},{alive},{}\n",
        t.beat,
        m.pos.x,
        m.pos.z,
        m.pos.y,
        m.vel.length(),
        u8::from(m.skates_on),
        entities.len()
    );
    log.lines.push_str(&line);
    log.worst_ms = log.worst_ms.max(ms);
    log.frames += 1;
    if now - log.second_s >= 1.0 {
        let fps = log.frames as f32 / (now - log.second_s);
        if log.worst_ms > 16.7 || fps < 60.0 {
            info!(
                "PERF slow: {fps:.0} fps, worst {:.1} ms, beat {} at ({:.0}, {:.0}) {:.0} m/s, {alive} enemies",
                log.worst_ms,
                t.beat,
                m.pos.x,
                m.pos.z,
                m.vel.length()
            );
        }
        log.worst_ms = 0.0;
        log.frames = 0;
        log.second_s = now;
    }
    if now - log.written_s > 5.0 || t.ending.is_some() {
        log.written_s = now;
        if let Some(path) = log.path.clone() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&path) {
                let _ = f.write_all(log.lines.as_bytes());
            }
            log.lines.clear();
        }
    }
}

/// Presses a key for one frame and lets it go the next, as a finger does.
fn press_keys(mut keys: ResMut<ButtonInput<KeyCode>>, mut press: ResMut<PressKey>) {
    match press.0.take() {
        Some((k, false)) => {
            keys.press(k);
            press.0 = Some((k, true));
        }
        Some((k, true)) => keys.release(k),
        None => {}
    }
}

/// What the pilot's hands hold besides the keys `PressKey` taps: wheel notches still to turn (positive: down, one a
/// frame, sent as the mouse's own `MouseWheel` messages so the game's routing of the wheel is what is tested), mouse
/// buttons and keys held down, and a mouse button to click once.
#[derive(Resource, Default)]
struct Hands {
    notches: Vec<i32>,
    hold_buttons: Vec<MouseButton>,
    hold_keys: Vec<KeyCode>,
    held_buttons: Vec<MouseButton>,
    held_keys: Vec<KeyCode>,
    click: Option<(MouseButton, bool)>,
}

/// Turns the wheel a notch a frame, before the devices are read.
fn turn_wheel(
    mut hands: ResMut<Hands>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut wheel: MessageWriter<bevy::input::mouse::MouseWheel>,
) {
    if hands.notches.is_empty() {
        return;
    }
    let n = hands.notches.remove(0);
    if let Ok(window) = window.single() {
        wheel.write(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            // a notch down the list is the wheel rolled toward the player: negative y
            y: -(n.signum() as f32),
            window,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
    }
}

/// Holds and lets go of the buttons and keys the pilot holds, and clicks a button for one frame, as fingers do.
fn hold_buttons(mut hands: ResMut<Hands>, mut mouse: ResMut<ButtonInput<MouseButton>>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    let h = &mut *hands;
    for b in h.hold_buttons.clone() {
        if !h.held_buttons.contains(&b) {
            mouse.press(b);
            h.held_buttons.push(b);
        }
    }
    for b in std::mem::take(&mut h.held_buttons) {
        if h.hold_buttons.contains(&b) {
            h.held_buttons.push(b);
        } else {
            mouse.release(b);
        }
    }
    for k in h.hold_keys.clone() {
        if !h.held_keys.contains(&k) {
            keys.press(k);
            h.held_keys.push(k);
        }
    }
    for k in std::mem::take(&mut h.held_keys) {
        if h.hold_keys.contains(&k) {
            h.held_keys.push(k);
        } else {
            keys.release(k);
        }
    }
    match h.click.take() {
        Some((b, false)) => {
            mouse.press(b);
            h.click = Some((b, true));
        }
        Some((b, true)) => mouse.release(b),
        None => {}
    }
}

/// What else the scripted pilot can see: the people, cars, aircraft, boats, buildings and the cockpit round it.
#[derive(SystemParam)]
struct More<'w, 's> {
    crowd: Option<Res<'w, crate::civilians::Crowd>>,
    greeting: Option<Res<'w, crate::civilians::Greeting>>,
    choice: Res<'w, crate::interact::UseChoice>,
    car: Res<'w, crate::traffic::CarAccessNow>,
    traffic: Option<Res<'w, crate::traffic::TrafficSim>>,
    roads: Option<Res<'w, crate::traffic::Roads>>,
    colliders: Res<'w, crate::walk_in::InteriorColliders>,
    seed: Res<'w, crate::world::Seed>,
    air: Option<Res<'w, crate::air_traffic::AirTraffic>>,
    boats: Option<Res<'w, crate::boats::Boats>>,
    targeting: Res<'w, crate::targeting::TargetState>,
    fortress: Option<Res<'w, crate::fortress::FortressRun>>,
    hostiles: Option<Res<'w, crate::units::Hostiles>>,
    fog: Option<Res<'w, crate::ground_fog::FogNow>>,
    free: Option<Res<'w, crate::missions::FreeRoamRun>>,
    my_id: Query<'w, 's, &'static crate::mech::MechId, With<Piloted>>,
}

/// The cockpit's screens and scope, the hands, and the blasts this frame.
#[derive(SystemParam)]
struct Cockpit<'w, 's> {
    scope: Option<Res<'w, crate::scope::Scope>>,
    pointed: Res<'w, crate::mfd_apps::Pointed>,
    screens: Query<'w, 's, (&'static crate::mfd::MfdScreen, &'static GlobalTransform)>,
    eye: Query<'w, 's, &'static GlobalTransform, With<crate::pilot::PilotCamera>>,
    hands: ResMut<'w, Hands>,
    blasts: MessageReader<'w, 's, crate::blast::Explosion>,
    neutrals: Option<Res<'w, crate::air_traffic::Neutrals>>,
    falling: Query<'w, 's, (&'static crate::air_drop::Dropping, &'static GlobalTransform)>,
}

/// What the scripted pilot can see and the hands it plays with.
#[derive(SystemParam)]
struct Scene<'w, 's> {
    ready: Res<'w, crate::loading::GameReady>,
    pilot: Res<'w, PilotSim>,
    access: Res<'w, MechAccessNow>,
    me: Query<'w, 's, (&'static MechSim, &'static MechSpec), With<Piloted>>,
    enemies: Query<'w, 's, (&'static MechSim, &'static JobEnemy), Without<Piloted>>,
    plan: Res<'w, Plan>,
    live: Res<'w, Live>,
    data: Res<'w, crate::sim::Data>,
    menu: Res<'w, Menu>,
    tab: Res<'w, TerminalTab>,
    near: Res<'w, crate::air_bases::TerminalNear>,
    service: Res<'w, crate::air_bases::LastService>,
    buttons: Query<'w, 's, (&'static MenuButton, &'static ComputedNode, &'static UiGlobalTransform)>,
    window: Query<'w, 's, Entity, With<PrimaryWindow>>,
    real: Res<'w, Time<Real>>,
    game: Res<'w, Time<Virtual>>,
}

fn say(line: &str) {
    println!("{line}");
}

/// The view turned toward a point on the level, as a mouse would, eased.
fn look_toward(yaw_rad: f32, pitch_rad: f32, from: Vec3, to: Vec3, want_pitch: f32, gain: f32) -> Vec2 {
    let d = to - from;
    let turn = wrap_rad((-d.x).atan2(-d.z) - yaw_rad);
    Vec2::new(-turn.to_degrees() * gain, (want_pitch - pitch_rad).to_degrees() * gain)
}

/// Drives the mech toward `goal` (x, z): steer and throttle, the view led onto it; skates on when far. With
/// `arrive`, it comes in slow enough to stop there: no faster than it can brake in the distance left (about
/// 3 m/s/s, skates off), as a pilot eases off on the way into a hangar.
#[allow(clippy::too_many_arguments)]
fn drive(m: &cb_core::mech::MechState, yaw: f32, pitch: f32, goal: Vec2, eye: Vec3, want: f32, skate: bool, arrive: bool) -> Actions {
    let to = goal - Vec2::new(m.pos.x, m.pos.z);
    let off = wrap_rad((-to.x).atan2(-to.y) - m.heading_rad).abs();
    // Slow while turning hard, so the mech comes round instead of circling.
    let mut throttle = if off > 0.8 { want.min(0.35) } else { want };
    let too_fast = arrive && m.vel.length() > (2.0 * 3.0 * to.length()).sqrt() + 4.0;
    if too_fast {
        throttle = 0.0;
    }
    Actions {
        throttle_zero: too_fast,
        throttle_axis: throttle_to(m, throttle),
        turn_axis: steer(m, goal),
        skate_toggle: m.skates_on != (skate && !too_fast),
        look_delta_deg: look_toward(yaw, pitch, eye, Vec3::new(goal.x, eye.y, goal.y), 0.0, 0.05),
        ..default()
    }
}

/// Skates the mech toward `goal` flat out. Blocked on a long drive (a building, a cliff): after moving, standing
/// still for 2 s sends the mech toward a point 150 m off to one side (alternating sides) for 8 s, the first 3 s on the
/// rockets, then back on course, as a player getting round it would.
#[allow(clippy::too_many_arguments)]
fn travel(
    t: &mut Pilot,
    m: &cb_core::mech::MechState,
    p: &cb_core::pilot::PilotState,
    eye: Vec3,
    goal: Vec2,
    now: f32,
    dt: f32,
) -> Actions {
    let speed = m.vel.length();
    t.peak_m_s = t.peak_m_s.max(speed);
    if speed < 3.0 && t.peak_m_s > 10.0 && now > t.detour_until {
        t.stall_s += dt;
    } else {
        t.stall_s = 0.0;
    }
    if t.stall_s > 2.0 {
        t.stall_s = 0.0;
        // Stopped again after the detour: another, the other side.
        t.peak_m_s = 10.5;
        t.detour_side = -t.detour_side;
        let here = Vec2::new(m.pos.x, m.pos.z);
        let ahead = (goal - here).normalize_or_zero();
        t.detour = here + Vec2::new(-ahead.y, ahead.x) * 150.0 * t.detour_side - ahead * 40.0;
        t.detour_until = now + 8.0;
        info!("playtest: blocked at ({:.0}, {:.0}), steering round it", m.pos.x, m.pos.z);
    }
    let to = if now < t.detour_until { t.detour } else { goal };
    let mut act = drive(m, p.yaw_rad, p.pitch_rad, to, eye, 1.0, true, false);
    // The first 3 s of a detour on the rockets, W held so they swivel forward (rocket-vector): up and out of whatever
    // caught the mech.
    if now < t.detour_until && t.detour_until - now > 5.0 {
        act.lift_axis = 1.0;
        act.move_xy = Vec2::Y;
    }
    act
}

/// One step of a jet leg toward `goal`, holding about `height` metres over the ground: lift while under it, W held so
/// the rockets swivel forward (rocket-vector). The jets carry a mech about 20 s at a time, so while they rest
/// (`cb_core::playtest::rest_jets`) the pilot comes down and skates on toward the goal, as a player would, and lifts
/// again once they have recharged.
#[allow(clippy::too_many_arguments)]
fn jet_leg(
    t: &mut Pilot,
    m: &cb_core::mech::MechState,
    p: &cb_core::pilot::PilotState,
    eye: Vec3,
    goal: Vec2,
    height: f32,
    plan: &Plan,
    now: f32,
    dt: f32,
) -> Actions {
    let resting = cb_core::playtest::rest_jets(m.boost.heat, m.boost.locked, t.jets_resting);
    if resting != t.jets_resting {
        info!(
            "playtest: jets {} at heat {:.2}, {:.0} m up at ({:.0}, {:.0})",
            if resting {
                "resting: down to skate"
            } else {
                "recharged: lifting again"
            },
            m.boost.heat,
            m.pos.y - plan.terrain.height_at(m.pos.x, m.pos.z),
            m.pos.x,
            m.pos.z
        );
        t.jets_resting = resting;
    }
    if resting {
        let mut act = travel(t, m, p, eye, goal, now, dt);
        act.lift_axis = 0.0;
        return act;
    }
    let mut act = drive(m, p.yaw_rad, p.pitch_rad, goal, eye, 1.0, false, false);
    let over = m.pos.y - plan.terrain.height_at(m.pos.x, m.pos.z);
    act.lift_axis = if over < height { 1.0 } else { 0.0 };
    act.move_xy = Vec2::Y;
    act
}

#[allow(clippy::too_many_lines)]
fn play(
    mut t: ResMut<Pilot>,
    mut acc: ResMut<InputAccum>,
    mut press: ResMut<PressKey>,
    mut run: Option<ResMut<CareerRun>>,
    mut pointer: MessageWriter<PointerInput>,
    mut exit: MessageWriter<AppExit>,
    s: Scene,
    x: More,
    mut k: Cockpit,
) {
    let now = s.real.elapsed_secs();
    let game_s = s.game.elapsed_secs();
    let name = t.script.name.clone();
    let Some((sim, spec)) = s.me.iter().next() else { return };
    let p = &s.pilot.curr;
    let m = &sim.curr;
    let eye = m.pos + Vec3::Y * spec.body.eye_height_m;
    // The take begins once the world is ready and, in a career, the career is read.
    if !t.started {
        if s.ready.0 && t.ready_s.is_none() {
            t.ready_s = Some(now);
        }
        // A career's run starts once the career is read; free roam has none.
        let career_mode = t.script.runs[t.run].play.starts_with("career");
        if t.ready_s.is_some_and(|r| now - r > 1.5) && s.access.0.is_some() && (run.is_some() || !career_mode) {
            t.started = true;
            t.start_s = now;
            say(&format!("PLAYTEST_READY {name}"));
            say(&format!("PLAYTEST_START {name}"));
            t.beat_real_s = now;
            t.beat_game_s = game_s;
            t.progress_game_s = game_s;
            announce(&t);
        }
        acc.script = Some(Actions::default());
        return;
    }
    if let Some(at) = t.ending {
        acc.script = Some(Actions::default());
        if now - at > 2.0 {
            say(&format!("PLAYTEST_END {name}"));
            exit.write(AppExit::Success);
            t.ending = Some(f32::INFINITY);
        }
        return;
    }
    // Every blast this frame, noted while the pilot fires at the ground (the crater check reads the take there).
    let ground_fire = matches!(t.script.runs[t.run].beats.get(t.beat), Some(Beat::FireAtGround(..)));
    for b in k.blasts.read() {
        if ground_fire && b.at.distance(eye) < 600.0 {
            say(&format!(
                "PLAYTEST_HIT t={:.2} at ({:.0}, {:.1}, {:.0}), {:.1} MJ, {:.0} m off",
                now - t.start_s,
                b.at.x,
                b.at.y,
                b.at.z,
                b.energy_mj,
                b.at.distance(eye)
            ));
        }
    }
    let beats = t.script.runs[t.run].beats.clone();
    let Some(beat) = beats.get(t.beat).cloned() else {
        finish(&mut t, now, None);
        return;
    };
    let in_beat = now - t.beat_real_s;
    let mut act = Actions::default();
    let mut done = false;
    // Progress toward a goal, for stuck detection (distance, metres); `None` for beats with no distance.
    let mut dist: Option<f32> = None;
    let career = run.as_deref();
    let base = career.map_or(0, |c| c.career.base);
    let terminal = |bay: u32| {
        s.plan
            .bases
            .iter()
            .find(|b| b.id == base)
            .and_then(|b| b.terminals(&s.data.0.world.bases).get(bay as usize).copied())
    };
    // Clicks on a button: move the pointer onto it, press, let go.
    let mut want_click: Option<MenuButton> = None;
    match &beat {
        Beat::Wait(secs) => done = in_beat >= *secs,
        Beat::LookAround(secs) => {
            act.look_delta_deg = Vec2::new((in_beat * 1.2).sin() * 0.6, 0.0);
            done = in_beat >= *secs;
        }
        Beat::WalkToTerminal(bay) => {
            let Some(term) = terminal(*bay) else {
                fail(&mut t, now, &format!("no terminal {bay} at base {base}"));
                return;
            };
            let front = Vec3::new(term.at.x, p.feet.y, term.at.y - 0.4);
            let out = Vec3::new(term.at.x, p.feet.y, term.at.y - 25.0);
            let goal = if t.leg == 0 { out } else { front };
            let d = Vec2::new(p.feet.x - goal.x, p.feet.z - goal.z).length();
            dist = Some(d + if t.leg == 0 { 100.0 } else { 0.0 });
            // `walk_to` stops within 2 m of its goal.
            if d > 2.2 {
                act = walk_to(p, goal, 0.0);
                act.run = d > 6.0;
            } else if t.leg == 0 {
                t.leg = 1;
            } else {
                // At the terminal: face it (it stands to the +z side).
                let screen = Vec3::new(term.at.x, p.feet.y + 1.7, term.at.y);
                let eyes = p.feet + Vec3::Y * 1.7;
                act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eyes, screen, -0.15, 0.15);
                done = s.near.near == Some(term) && wrap_rad(p.yaw_rad - PI).abs() < 0.3;
            }
        }
        Beat::UseTerminal => {
            act.use_pressed = *s.menu != Menu::Terminal;
            done = *s.menu == Menu::Terminal && in_beat > 1.0;
        }
        Beat::Tab(which) => {
            let tab = match which.as_str() {
                "Shop" => TerminalTab::Shop,
                "Refit" => TerminalTab::Refit,
                "Hangar" => TerminalTab::Hangar,
                _ => TerminalTab::Jobs,
            };
            if *s.tab == tab && (t.clicked || in_beat > 0.5) {
                done = true;
            } else {
                want_click = Some(MenuButton::Tab(tab));
            }
        }
        Beat::AcceptJob(max) => match career.and_then(|c| c.career.board.current().map(|j| j.id)) {
            Some(id) => {
                info!("playtest: signed up for job {id}");
                done = t.clicked;
            }
            None => {
                let here = Vec2::new(p.feet.x, p.feet.z);
                let pick = career.and_then(|c| {
                    c.career
                        .board
                        .jobs
                        .iter()
                        .filter(|j| j.state == JobState::Offered && j.tier <= *max)
                        .min_by(|a, b| a.site.distance(here).total_cmp(&b.site.distance(here)))
                        .map(|j| j.id)
                });
                want_click = pick.map(MenuButton::Accept);
            }
        },
        Beat::AcceptByMail(rule, max) => {
            let current = career.and_then(|c| c.career.board.current().map(|j| j.id));
            if t.clicked && current.is_some() {
                // Signed up: M again closes the mail.
                if *s.menu == Menu::Mail && press.0.is_none() && now - t.toggled_s > 1.0 {
                    t.toggled_s = now;
                    press.0 = Some((KeyCode::KeyM, false));
                }
                done = !s.menu.is_open();
                if done {
                    info!("playtest: signed up by mail for job {}", current.unwrap_or_default());
                }
            } else if *s.menu != Menu::Mail {
                if press.0.is_none() && now - t.toggled_s > 1.0 {
                    t.toggled_s = now;
                    press.0 = Some((KeyCode::KeyM, false));
                }
            } else if in_beat > 240.0 {
                fail(&mut t, now, "no offer came in 240 s");
                return;
            } else {
                let here = Vec2::new(m.pos.x, m.pos.z);
                let pick = career.and_then(|c| {
                    c.career
                        .board
                        .jobs
                        .iter()
                        .filter(|j| j.state == JobState::Offered && j.tier <= *max && !t.gave_up.contains(&j.id))
                        .max_by(|a, b| {
                            let by = a.site.distance(here).total_cmp(&b.site.distance(here));
                            if *rule == Pick::Farthest { by } else { by.reverse() }
                        })
                        .map(|j| (j.id, j.site.distance(here)))
                });
                if let Some((id, d)) = pick
                    && t.click.is_none()
                    && !t.clicked
                    && in_beat > 0.4
                {
                    info!("playtest: the {rule:?} offer is job {id}, {d:.0} m away");
                }
                want_click = pick.map(|(id, _)| MenuButton::Accept(id));
            }
        }
        Beat::CloseTerminal => {
            if *s.menu == Menu::Terminal && press.0.is_none() && in_beat < 0.1 {
                press.0 = Some((KeyCode::KeyE, false));
            }
            done = !s.menu.is_open() && in_beat > 0.5;
        }
        Beat::Board => {
            act = board(p, s.access.0.as_ref());
            if let Some(a) = s.access.0.as_ref() {
                dist = Some(p.distance_to_stair(a) + if p.mode == PilotMode::OnFoot { 100.0 } else { 0.0 });
            }
            done = p.mode == PilotMode::Seated;
        }
        Beat::GoToJob => {
            let job = career.and_then(|c| c.career.board.current().cloned());
            match job {
                Some(j) if j.state == JobState::Active => done = true,
                Some(j) => {
                    dist = Some(j.site.distance(Vec2::new(m.pos.x, m.pos.z)));
                    act = travel(&mut t, m, p, eye, j.site, now, s.real.delta_secs());
                }
                None => {
                    fail(&mut t, now, "no job under way");
                    return;
                }
            }
        }
        Beat::SkateTo(x, z) => {
            let goal = Vec2::new(*x, *z);
            let d = goal.distance(Vec2::new(m.pos.x, m.pos.z));
            dist = Some(d);
            done = d < 40.0;
            act = travel(&mut t, m, p, eye, goal, now, s.real.delta_secs());
        }
        Beat::JetTo(x, z, height) => {
            let goal = Vec2::new(*x, *z);
            let d = goal.distance(Vec2::new(m.pos.x, m.pos.z));
            dist = Some(d);
            done = d < 250.0;
            act = jet_leg(&mut t, m, p, eye, goal, *height, &s.plan, now, s.real.delta_secs());
        }
        Beat::FightJob(cap) => {
            let job = career.and_then(|c| c.career.board.current().cloned());
            let alive: Vec<&MechSim> = s
                .enemies
                .iter()
                .filter(|(e, je)| !e.dead && job.as_ref().is_some_and(|j| j.id == je.0))
                .map(|(e, _)| e)
                .collect();
            let won = career.is_some_and(|c| c.career.done.last().is_some_and(|d| d.won)) && job.is_none();
            if won {
                let paid = career.and_then(|c| c.career.done.last().map(|d| d.paid)).unwrap_or(0);
                info!("playtest: the job was won and paid {paid} credits");
                done = true;
            } else if job.is_none() {
                fail(&mut t, now, "the job ended unpaid");
                return;
            } else if in_beat > *cap {
                fail(&mut t, now, &format!("the job's fight ran past {cap:.0} s"));
                return;
            } else {
                let nearest = alive
                    .iter()
                    .min_by(|a, b| a.curr.pos.distance(m.pos).total_cmp(&b.curr.pos.distance(m.pos)));
                let (target, vel) = match nearest {
                    Some(e) => (e.curr.pos, cb_core::mech::forward(e.curr.heading_rad) * e.curr.speed_m_s),
                    None => (m.pos, Vec3::ZERO),
                };
                let brain = t.brain.get_or_insert_with(|| {
                    cb_core::ai::AiPilot::new(
                        cb_core::ai::Skill {
                            aim_noise_deg: 0.3,
                            fire_error_deg: 3.5,
                            range_m: 130.0,
                            hop_chance: 0.3,
                            ..Default::default()
                        },
                        cb_core::rng::SimRng::new(s.plan.seed).fork("playtest"),
                    )
                });
                act = brain_actions(
                    brain,
                    sim,
                    &Aim {
                        eye,
                        target,
                        target_vel: vel,
                        wall: false,
                    },
                    spec,
                    &s.plan,
                    &s.live,
                    s.real.delta_secs(),
                );
                // Missiles at whatever is locked, now and then, as a player would.
                act.fire_missile = in_beat.rem_euclid(6.0) < 0.3;
                act.target_next = in_beat.rem_euclid(6.0) < 0.02;
            }
        }
        Beat::EnterBay(bay) => {
            let hangar = s
                .plan
                .bases
                .iter()
                .find(|b| b.id == base)
                .and_then(|b| b.hangars(&s.data.0.world.bases).get(*bay as usize).copied());
            let Some(h) = hangar else {
                fail(&mut t, now, &format!("no bay {bay} at base {base}"));
                return;
            };
            let apron = Vec2::new(h.centre().x, h.min.y - 120.0);
            let inside = cb_core::world::bases::hangar_at(&s.plan.bases, &s.data.0.world.bases, Vec2::new(m.pos.x, m.pos.z))
                == Some((base, *bay as usize));
            let here = Vec2::new(m.pos.x, m.pos.z);
            if t.leg == 0 && here.distance(apron) < 40.0 {
                t.leg = 1;
            }
            let goal = if t.leg == 0 { apron } else { h.centre() };
            let d = here.distance(goal);
            dist = Some(d + if t.leg == 0 { 1000.0 } else { 0.0 });
            if inside && here.distance(h.centre()) < 12.0 {
                act.throttle_zero = true;
                act.full_stop = true;
                act.skate_toggle = m.skates_on;
                // Stopping: the distance still to shed is the speed (stuck if it never comes to rest).
                dist = Some(m.vel.length() * 10.0);
                if s.service.last.as_ref().is_some_and(|l| l.0 == base && l.1 == *bay as usize) && s.service.since_s < t.service_seen + 1.0
                {
                    done = true;
                }
            } else {
                let far = d > 600.0 && t.leg == 0;
                let speed = if t.leg == 1 {
                    0.5
                } else if d < 250.0 {
                    0.6
                } else {
                    1.0
                };
                act = drive(m, p.yaw_rad, p.pitch_rad, goal, eye, speed, far, true);
                t.service_seen = s.service.since_s;
            }
        }
        Beat::Disembark => {
            act.throttle_zero = true;
            act.use_held = p.mode == PilotMode::Seated;
            done = p.mode == PilotMode::OnFoot;
        }
        Beat::Grant(credits) => {
            if let Some(r) = run.as_deref_mut() {
                let kept = r.data.career.ledger_kept;
                r.career.pay(*credits, "playtest grant", kept);
                info!("playtest: granted {credits} credits (disclosed: the ledger says playtest grant)");
            }
            done = true;
        }
        Beat::BuyMech(design) => {
            let owned = career.is_some_and(|c| c.career.mechs.iter().any(|o| &o.design == design));
            if owned && t.clicked {
                done = true;
            } else if t.clicked && in_beat > 2.0 {
                let note = career.and_then(|c| c.shop_note.clone()).unwrap_or_default();
                fail(&mut t, now, &format!("buying {design} was refused: {note}"));
                return;
            } else {
                let i = career.and_then(|c| c.data.catalogue.listings.iter().position(|l| &l.design == design));
                want_click = i.map(MenuButton::Buy);
            }
        }
        Beat::AwaitDrop(cap) => {
            let cfg = career.map(|c| c.data.drop.clone()).unwrap_or_default();
            let now_s = career.map_or(0.0, |c| c.career.time_s);
            // Progress is the time left to the last touchdown (seconds, as metres for the stuck check).
            let left = career
                .map(|c| {
                    c.career
                        .drops
                        .iter()
                        .filter(|d| !d.landed)
                        .map(|d| cb_core::air_drop::fall_s(&cfg) - d.since_release_s(&cfg, now_s))
                        .fold(0.0f32, f32::max)
                })
                .unwrap_or(0.0);
            let falling = career.is_some_and(|c| c.career.drops.iter().any(|d| !d.landed));
            if !falling {
                info!("playtest: the air drop has landed");
                done = true;
            } else if in_beat > *cap {
                fail(&mut t, now, &format!("the air drop had not landed in {cap:.0} s"));
                return;
            } else {
                dist = Some(left.max(0.0) * 10.0 + 1.0);
                // Look toward it, as a player would watch it come down: the freighter with the mech on its sling, then the
                // mech falling under its canopies, then the site.
                if let Some(d) = career.and_then(|c| c.career.drops.iter().find(|d| !d.landed)) {
                    let eyes = p.feet + Vec3::Y * 1.7;
                    let at = k
                        .falling
                        .iter()
                        .find(|(f, _)| f.0 == d.key)
                        .map(|(_, g)| g.translation() + Vec3::Y * 10.0)
                        .or_else(|| d.aircraft(&cfg, now_s, 0.0))
                        .unwrap_or(Vec3::from_array(d.site) + Vec3::Y * 60.0);
                    let v = at - eyes;
                    let pitch = v.y.atan2(Vec2::new(v.x, v.z).length()).min(1.2);
                    act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eyes, at, pitch, 0.08);
                }
            }
        }
        Beat::WalkTowardDrop(short) => match career.and_then(|c| c.career.drops.iter().find(|d| !d.landed)) {
            Some(d) => {
                let goal = Vec3::new(d.site[0], p.feet.y, d.site[2]);
                let g = Vec2::new(p.feet.x - goal.x, p.feet.z - goal.z).length();
                dist = Some(g);
                if g <= *short + 2.0 {
                    info!("playtest: out in the open, {g:.0} m from where the drop will land");
                    done = true;
                } else {
                    act = walk_close(p, goal, *short);
                    act.run = true;
                }
            }
            None => done = true,
        },
        Beat::WalkToDrop => {
            let newest = career.and_then(|c| c.career.drops.iter().rev().find(|d| d.landed).map(|d| (d.key, d.site)));
            match newest {
                Some((key, site)) => {
                    if career.is_some_and(|c| c.career.piloting == Some(key)) {
                        info!("playtest: walking up to the dropped mech {key}");
                        done = true;
                    } else {
                        let goal = Vec3::new(site[0], p.feet.y, site[2]);
                        dist = Some(Vec2::new(p.feet.x - goal.x, p.feet.z - goal.z).length());
                        act = walk_to(p, goal, 0.0);
                        act.run = dist.is_some_and(|d| d > 50.0);
                    }
                }
                None => {
                    fail(&mut t, now, "no landed air drop to walk to");
                    return;
                }
            }
        }
        Beat::ExpectOffer(tier) => {
            let best = career
                .and_then(|c| {
                    c.career
                        .board
                        .jobs
                        .iter()
                        .filter(|j| j.state == JobState::Offered)
                        .map(|j| j.tier)
                        .max()
                })
                .unwrap_or(0);
            if best >= *tier {
                info!("playtest: a tier {best} job is offered");
            } else {
                t.failures.push(format!("no tier {tier} offer (best {best})"));
            }
            done = true;
        }
        Beat::BuyPart(id) => {
            let i = career.and_then(|c| c.data.parts.weapons.iter().position(|w| &w.id == id));
            let has = career.is_some_and(|c| c.career.parts.contains(id));
            if has && t.clicked {
                done = true;
            } else if t.clicked && in_beat > 2.0 {
                fail(&mut t, now, &format!("buying {id} was refused"));
                return;
            } else {
                want_click = i.map(|i| MenuButton::Refit(crate::career::Refit::BuyPart(i)));
            }
        }
        Beat::Fit(station, id) => {
            let fitted = career
                .and_then(crate::career::piloted)
                .is_some_and(|o| o.fit.get(*station as usize).cloned().flatten().as_ref() == Some(id));
            if fitted && t.clicked {
                let name = spec.weapons.get(*station as usize).map_or("?", |w| w.name.as_str());
                info!("playtest: station {} now fires the {name}", station + 1);
                done = true;
            } else if t.clicked && in_beat > 2.0 {
                let note = career.and_then(|c| c.shop_note.clone()).unwrap_or_default();
                fail(&mut t, now, &format!("fitting {id} was refused: {note}"));
                return;
            } else {
                let i = career.and_then(|c| c.data.parts.weapons.iter().position(|w| &w.id == id));
                want_click = i.map(|i| MenuButton::Refit(crate::career::Refit::Fit(*station as usize, Some(i))));
            }
        }
        Beat::Quit => {
            // leg 0: press Escape (once); leg 1: the pause menu is up, click Quit; leg 2: the result is said.
            if *s.menu == Menu::Closed && t.leg == 0 && in_beat > 0.2 {
                write_expect(&name, career, p, m);
                press.0 = Some((KeyCode::Escape, false));
                t.leg = 1;
            } else if *s.menu == Menu::Pause {
                if t.leg == 1 && t.click.is_some_and(|c| c.stage >= 2) {
                    t.leg = 2;
                    let result = if t.failures.is_empty() {
                        "PLAYTEST_RESULT ok career-loop: every beat played".to_string()
                    } else {
                        format!("PLAYTEST_RESULT fail {}", t.failures.join("; "))
                    };
                    say(&result);
                    say(&format!("PLAYTEST_END {name}"));
                }
                want_click = Some(MenuButton::Quit);
            }
        }
        Beat::ExpectRestored(tol) => {
            if in_beat > 3.0 {
                let path = expect_path(&name);
                let saved: Option<Expect> = std::fs::read_to_string(&path).ok().and_then(|t| Expect::from_ron(&t));
                match (saved, now_expect(career, p, m)) {
                    (Some(was), Some(is)) => {
                        let diff = was.differences(&is, *tol);
                        if diff.is_empty() {
                            info!(
                                "playtest: restored {} credits, {} mechs, at ({:.0}, {:.0}), {}",
                                is.money,
                                is.mechs.len(),
                                is.at[0],
                                is.at[1],
                                if is.seated { "seated" } else { "on foot" }
                            );
                        } else {
                            t.failures.extend(diff);
                        }
                    }
                    _ => t
                        .failures
                        .push(format!("nothing to compare: {} or the career missing", path.display())),
                }
                done = true;
            }
        }
        Beat::WalkToPerson(within) => {
            let Some(crowd) = x.crowd.as_deref() else {
                fail(&mut t, now, "there are no people in this game");
                return;
            };
            let feet = Vec2::new(p.feet.x, p.feet.z);
            let calm = |q: &&cb_core::civilians::Person| {
                !matches!(
                    q.activity,
                    cb_core::civilians::Activity::Down { .. } | cb_core::civilians::Activity::Fleeing { .. }
                )
            };
            if t.goal_id.is_none_or(|id| !crowd.people.list.iter().any(|q| q.id == id && calm(&q))) {
                t.goal_id = crowd
                    .people
                    .list
                    .iter()
                    .filter(calm)
                    .filter(|q| q.pos.distance(feet) < *within)
                    .min_by(|a, b| a.pos.distance(feet).total_cmp(&b.pos.distance(feet)))
                    .map(|q| q.id);
                if let Some(id) = t.goal_id {
                    info!("playtest: walking up to person {id}");
                }
            }
            if let Some(q) = t.goal_id.and_then(|id| crowd.people.list.iter().find(|q| q.id == id)) {
                let d = q.pos.distance(feet);
                dist = Some(d);
                if crowd.people.in_reach(&s.data.0.people.cfg, p.feet, p.yaw_rad).is_some() && d < 2.4 {
                    info!("playtest: person {} is in reach, {d:.1} m away, {:?}", q.id, q.activity);
                    done = true;
                } else {
                    act = walk_close(p, q.feet(), 1.4);
                    act.run = d > 8.0;
                }
            }
        }
        Beat::Scroll(n) => {
            if t.count == 0 {
                info!("playtest: Use list before the wheel: {}", use_list(&x.choice.selector));
            }
            if (t.count as i32) < n.abs() {
                if now - t.next_s > 0.3 {
                    k.hands.notches.push(n.signum());
                    t.count += 1;
                    t.next_s = now;
                }
            } else if now - t.next_s > 0.4 {
                info!("playtest: Use list after {n} notches: {}", use_list(&x.choice.selector));
                done = true;
            }
        }
        Beat::UseOn(name) => {
            let want = cb_core::playtest::interaction(name);
            let sel = &x.choice.selector;
            let opts = sel.options();
            if t.leg == 0 {
                match want.and_then(|w| opts.iter().position(|o| *o == w)) {
                    None if in_beat > 10.0 => {
                        fail(&mut t, now, &format!("{name} never came into reach; in reach: {}", use_list(sel)));
                        return;
                    }
                    None => {}
                    Some(i) => {
                        let at = sel.selected().and_then(|s| opts.iter().position(|o| *o == s)).unwrap_or(0);
                        if at == i {
                            info!("playtest: Use on {name}: {}", use_list(sel));
                            act.use_pressed = true;
                            t.leg = 1;
                            t.next_s = now;
                        } else if k.hands.notches.is_empty() && now - t.next_s > 0.5 {
                            let n = cb_core::playtest::notches_to(at, i, opts.len());
                            info!("playtest: the wheel {n} notches to {name}: {}", use_list(sel));
                            k.hands.notches.extend(std::iter::repeat_n(n.signum(), n.unsigned_abs() as usize));
                            t.next_s = now;
                        }
                    }
                }
            } else {
                // What the press did, as the player sees it.
                let took = match name.as_str() {
                    "Greet" => x
                        .greeting
                        .as_deref()
                        .filter(|g| g.left_s > 0.0)
                        .map(|g| format!("the answer on screen: {}", g.line)),
                    "GetIn" => p.mode.in_car().then(|| format!("{:?}", p.mode)),
                    "GetOut" => matches!(p.mode, PilotMode::ExitingCar(_) | PilotMode::OnFoot).then(|| "getting out".to_string()),
                    "Terminal" => (*s.menu == Menu::Terminal).then(|| "the terminal's screen".to_string()),
                    _ => (p.mode != PilotMode::OnFoot || p.waiting.is_some()).then(|| "boarding".to_string()),
                };
                if let Some(what) = took {
                    info!("playtest: Use on {name} took: {what}");
                    done = true;
                } else if now - t.next_s > 12.0 {
                    fail(&mut t, now, &format!("Use on {name} did nothing in 12 s"));
                    return;
                }
            }
        }
        Beat::ExpectMode(mode, cap) => {
            let now_mode = match p.mode {
                PilotMode::OnFoot => "OnFoot",
                PilotMode::Driving => "Driving",
                PilotMode::Seated => "Seated",
                _ => "between",
            };
            if now_mode == mode {
                info!("playtest: the pilot is {mode}");
                done = true;
            } else if in_beat > *cap {
                let driven = x.traffic.as_deref().and_then(|tr| tr.traffic.driven()).map(|c| (c.id, c.state));
                fail(
                    &mut t,
                    now,
                    &format!(
                        "not {mode} after {cap:.0} s (now {:?}; car in reach {:?}; driven {driven:?})",
                        p.mode, x.car.0
                    ),
                );
                return;
            }
        }
        Beat::WalkToCar(within) => {
            let Some(traffic) = x.traffic.as_deref() else {
                fail(&mut t, now, "there is no traffic in this game");
                return;
            };
            let kinds = &s.data.0.traffic.kinds;
            let door_of =
                |c: &cb_core::traffic::Car| cb_core::car::CarSpec::to_world(c.pos, c.heading_rad, kinds[c.kind as usize].door_driver_m);
            let feet = p.feet;
            if t.goal_id
                .is_none_or(|id| !traffic.traffic.cars.iter().any(|c| c.id == id && !c.state.wreck()))
            {
                // A stopped car first (parked, or waiting at a red light), else the nearest.
                let near = traffic
                    .traffic
                    .cars
                    .iter()
                    .filter(|c| !c.state.wreck() && door_of(c).distance(feet) < *within);
                t.goal_id = near
                    .min_by(|a, b| {
                        // and one with road ahead of it, not boxed in a queue
                        let boxed = |c: &cb_core::traffic::Car| {
                            let f = Vec3::new(-c.heading_rad.sin(), 0.0, -c.heading_rad.cos());
                            traffic.traffic.cars.iter().any(|o| {
                                let v = o.pos - c.pos;
                                o.id != c.id && v.dot(f) > 0.0 && v.dot(f) < 14.0 && (v - f * v.dot(f)).length() < 2.5
                            })
                        };
                        let k = |c: &cb_core::traffic::Car| {
                            door_of(c).distance(feet) + if c.vel().length() > 0.5 { 60.0 } else { 0.0 } + if boxed(c) { 30.0 } else { 0.0 }
                        };
                        k(a).total_cmp(&k(b))
                    })
                    .map(|c| c.id);
                if let Some(id) = t.goal_id {
                    info!("playtest: walking to car {id}'s driver's door");
                }
            }
            if let Some(c) = t.goal_id.and_then(|id| traffic.traffic.cars.iter().find(|c| c.id == id)) {
                let door = door_of(c);
                let d = Vec2::new(door.x - feet.x, door.z - feet.z).length();
                dist = Some(d);
                let reach = x.car.0.as_ref().filter(|a| a.id == c.id);
                if reach.is_some_and(|a| p.can_take(a, &s.data.0.pilot)) {
                    info!(
                        "playtest: at car {}'s door, {d:.2} m, the car doing {:.1} m/s",
                        c.id,
                        c.vel().length()
                    );
                    done = true;
                } else if d > 0.8 {
                    act = walk_close(p, door, 0.6);
                    act.run = d > 10.0;
                } else {
                    let eyes = p.feet + Vec3::Y * 1.7;
                    act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eyes, c.pos + Vec3::Y, -0.2, 0.15);
                }
            }
        }
        Beat::DriveCar(secs, throttle) => {
            let speed = x.car.0.as_ref().map_or(0.0, |c| c.speed_m_s);
            if p.mode != PilotMode::Driving {
                fail(&mut t, now, "not in a car");
                return;
            }
            if in_beat < *secs {
                act.move_xy = Vec2::new(0.0, *throttle);
                if now - t.next_s > 2.0 {
                    t.next_s = now;
                    let driven = x.traffic.as_deref().and_then(|tr| tr.traffic.driven());
                    info!(
                        "playtest: driving at {speed:.1} m/s, throttle {throttle}, the car {:?}",
                        driven.map(|c| (c.id, c.state, c.pos))
                    );
                }
            } else {
                act.lift_axis = 1.0;
                done = speed < 0.3;
                dist = Some(speed * 10.0);
                if done {
                    info!("playtest: the car stopped after {in_beat:.1} s");
                }
            }
        }
        Beat::WalkIntoBuilding(within, storeys) => {
            if t.route.is_empty() {
                let feet = Vec2::new(p.feet.x, p.feet.z);
                let rng = crate::buildings::building_rng(x.seed.0);
                let cfg = &x.colliders.0.cfg;
                let best = x
                    .colliders
                    .0
                    .buildings
                    .keys()
                    .filter_map(|&b| {
                        let c = s.live.structures.buildings.get(b as usize)?;
                        if c.standing == 0 || c.n[1] < 2 {
                            return None;
                        }
                        let site = cb_core::world::interior::Site::of(&s.plan.0, b, c.n, c.origin, c.size);
                        let shell = cb_core::world::interior::shell(&site, cfg, &rng);
                        let d = shell.entrance.route.first()?.distance(feet);
                        (d < *within).then_some((d, b, shell))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                match best {
                    Some((d, b, shell)) => {
                        let y = p.feet.y;
                        t.route = shell.entrance.route.iter().map(|q| Vec3::new(q.x, y, q.y)).collect();
                        for storey in 0..*storeys as usize {
                            t.route.extend(shell.stair.climb(storey));
                        }
                        t.goal_id = Some(b);
                        t.route_i = 0;
                        info!(
                            "playtest: into building {b} by its {:?} door, {d:.0} m off, then {storeys} storeys up its stair ({} points)",
                            shell.entrance.kind,
                            t.route.len()
                        );
                    }
                    None if in_beat > 8.0 => {
                        fail(&mut t, now, &format!("no building with colliders within {within:.0} m"));
                        return;
                    }
                    None => {}
                }
            }
            if let Some(&goal) = t.route.get(t.route_i) {
                let flat = Vec2::new(goal.x - p.feet.x, goal.z - p.feet.z).length();
                let left: f32 = flat + t.route.windows(2).skip(t.route_i).map(|w| w[0].distance(w[1])).sum::<f32>();
                dist = Some(left);
                if flat < 0.6 {
                    t.route_i += 1;
                    if t.route_i == t.route.len() {
                        info!("playtest: up the stair, feet at {:.1} m (started at the door)", p.feet.y);
                        done = true;
                    }
                } else {
                    act = walk_close(p, goal, 0.3);
                    act.look_delta_deg.y = (0.0 - p.pitch_rad).to_degrees() * 0.1;
                }
            }
        }
        Beat::Survey(within) => {
            say(&format!(
                "PLAYTEST_SURVEY t={:.1} {}",
                now - t.start_s,
                survey(&s, &x, &k, p, m, spec, sim, *within)
            ));
            done = true;
        }
        Beat::LookAtNearest(kind, secs) => {
            let eyes = if p.mode == PilotMode::OnFoot { p.feet + Vec3::Y * 1.7 } else { eye };
            match nearest(kind, eyes, &s, &x) {
                Some((at, what)) => {
                    if t.count == 0 {
                        t.count = 1;
                        info!("playtest: looking at the nearest {kind}: {what}, {:.0} m", at.distance(eyes));
                    }
                    let d = at - eyes;
                    let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
                    act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eyes, at, pitch, 0.08);
                }
                None if t.count == 0 && in_beat > 1.0 => {
                    t.count = 1;
                    info!("playtest: no {kind} to look at");
                }
                None => {}
            }
            act.throttle_zero = p.mode == PilotMode::Seated;
            done = in_beat >= *secs;
        }
        Beat::SkateToNearest(kind, short) | Beat::JetToNearest(kind, short) => {
            let jet = matches!(beat, Beat::JetToNearest(..));
            if t.goal_id.is_none() {
                match place(kind, Vec2::new(m.pos.x, m.pos.z), &s, &x) {
                    Some((at, what)) => {
                        t.goal = at;
                        t.goal_id = Some(0);
                        info!("playtest: going to the nearest {kind}: {what} at ({:.0}, {:.0})", at.x, at.y);
                    }
                    None => {
                        fail(&mut t, now, &format!("no {kind} on the island"));
                        return;
                    }
                }
            }
            let d = t.goal.distance(Vec2::new(m.pos.x, m.pos.z));
            dist = Some(d);
            if jet {
                done = d < 250.0;
                let goal = t.goal;
                act = jet_leg(&mut t, m, p, eye, goal, *short, &s.plan, now, s.real.delta_secs());
            } else {
                done = d < short + 40.0;
                let goal = t.goal;
                act = travel(&mut t, m, p, eye, goal, now, s.real.delta_secs());
            }
        }
        Beat::SelectWeapon(name) => {
            let now_name = spec.weapons.get(sim.selected).map_or("", |w| w.name.as_str());
            if now_name.contains(name.as_str()) {
                info!("playtest: station {} selected: {now_name}", sim.selected + 1);
                done = true;
            } else if k.hands.notches.is_empty() && now - t.next_s > 0.6 {
                t.next_s = now;
                t.count += 1;
                if t.count > 12 {
                    fail(&mut t, now, &format!("no {name} on board"));
                    return;
                }
                info!("playtest: the wheel one notch on from {now_name}");
                k.hands.notches.push(1);
            }
        }
        Beat::FireFor(secs) => {
            let target = s
                .enemies
                .iter()
                .filter(|(e, _)| !e.dead)
                .map(|(e, _)| e.curr.pos + Vec3::Y * 8.0)
                .chain(x.hostiles.iter().flat_map(|h| h.0.iter().map(|h| h.aim)))
                .filter(|q| q.distance(eye) < 1500.0)
                .min_by(|a, b| a.distance(eye).total_cmp(&b.distance(eye)));
            let at = target.unwrap_or_else(|| {
                let f = cb_core::mech::forward(m.heading_rad) * 80.0 + m.pos;
                Vec3::new(f.x, s.plan.terrain.height_at(f.x, f.z), f.z)
            });
            let d = at - eye;
            let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
            act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eye, at, pitch, 0.1);
            act.throttle_zero = true;
            act.fire[0] = in_beat > 0.8 && in_beat < *secs;
            if now - t.next_s > 1.0 {
                t.next_s = now;
                let w = spec.weapons.get(sim.selected).map_or("?", |w| w.name.as_str());
                info!(
                    "playtest: firing the {w} at {}, {:.0} m; weapon heat {:.2}{}",
                    if target.is_some() { "an enemy" } else { "the ground" },
                    d.length(),
                    sim.guns.heat.heat,
                    if sim.guns.heat.locked { " (locked out: cooling)" } else { "" }
                );
            }
            done = in_beat >= *secs + 0.5;
        }
        Beat::Scope(secs, notches) => {
            let open = in_beat < *secs;
            if open {
                if !k.hands.hold_buttons.contains(&MouseButton::Middle) {
                    k.hands.hold_buttons.push(MouseButton::Middle);
                }
                act.scope = true;
                if (t.count as i32) < notches.abs() && in_beat > 1.0 + t.count as f32 * 1.5 {
                    k.hands.notches.push(-notches.signum());
                    t.count += 1;
                }
            } else {
                k.hands.hold_buttons.retain(|b| *b != MouseButton::Middle);
            }
            if now - t.next_s > 0.75 {
                t.next_s = now;
                let sc = k.scope.as_deref();
                info!(
                    "playtest: scope {} at {:.0}x, weapon {}",
                    if sc.is_some_and(|c| c.open) { "open" } else { "shut" },
                    sc.map_or(0.0, |c| c.zoom),
                    spec.weapons.get(sim.selected).map_or("?", |w| w.name.as_str())
                );
            }
            act.throttle_zero = true;
            done = in_beat >= *secs + 1.0;
        }
        Beat::MultiLock(secs) => {
            act.fire_missile = in_beat < *secs;
            act.throttle_zero = true;
            if let Some(e) = s
                .enemies
                .iter()
                .filter(|(e, _)| !e.dead)
                .map(|(e, _)| e.curr.pos + Vec3::Y * 8.0)
                .min_by(|a, b| a.distance(eye).total_cmp(&b.distance(eye)))
            {
                let d = e - eye;
                let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
                act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eye, e, pitch, 0.1);
            }
            if now - t.next_s > 0.5 {
                t.next_s = now;
                info!(
                    "playtest: missile key {}, {} locks held: {:?}",
                    if act.fire_missile { "held" } else { "let go" },
                    sim.guns.locks.held.len(),
                    sim.guns.locks.held
                );
            }
            done = in_beat >= *secs + 2.0;
        }
        Beat::CycleTargets(n) => {
            if (t.count) < *n && now - t.next_s > 1.0 {
                t.next_s = now;
                t.count += 1;
                act.target_next = true;
            } else if t.count > 0 && now - t.next_s > 0.5 && t.seen != Some(t.count as f32) {
                t.seen = Some(t.count as f32);
                let sel = x.targeting.selected;
                let civil = sel.is_some_and(|id| (crate::air_traffic::CIVIL_ID_BASE..crate::fortress::FORTRESS_ID).contains(&id));
                let boxed_civil = k.neutrals.as_deref().map_or(0, |n| n.0.len());
                info!(
                    "playtest: target {} of {n}: {:?}, {:.0} m, lock {:?}{}; {boxed_civil} civil aircraft boxed as neutral",
                    t.count,
                    sel,
                    x.targeting.range_m,
                    x.targeting.lock,
                    if civil { " A CIVIL AIRCRAFT" } else { "" }
                );
                if civil {
                    t.failures.push(format!("civil aircraft {sel:?} was selected"));
                }
            }
            done = t.count >= *n && t.seen == Some(*n as f32);
        }
        Beat::MfdFocus => {
            let pages = |k: &Cockpit| k.screens.iter().map(|(sc, _)| sc.pages.page).collect::<Vec<_>>();
            let defaults: Vec<usize> = k.screens.iter().map(|(sc, _)| sc.pages.default).collect();
            act.throttle_zero = true;
            match t.leg {
                0 => {
                    if !k.hands.hold_keys.contains(&KeyCode::ControlLeft) {
                        k.hands.hold_keys.push(KeyCode::ControlLeft);
                        info!("playtest: MFD pages before focus {:?} (defaults {defaults:?})", pages(&k));
                    }
                    act.free_look = true;
                    // the screen lowest on the left of the seat
                    let cam = k.eye.iter().next().map(|g| g.translation()).unwrap_or(eye);
                    let left = k
                        .screens
                        .iter()
                        .map(|(_, g)| g.translation())
                        .min_by(|a, b| (a.y + a.x * 0.0).total_cmp(&(b.y + b.x * 0.0)));
                    if let Some(sc) = left {
                        let d = sc - cam;
                        let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
                        act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, cam, sc, pitch, 0.15);
                    }
                    if k.pointed.screen.is_some() && in_beat > 1.5 {
                        info!("playtest: pointing at an MFD on page {:?}; pressing it", k.pointed.page);
                        k.hands.click = Some((MouseButton::Left, false));
                        t.leg = 1;
                        t.next_s = now;
                    } else if in_beat > 12.0 {
                        fail(&mut t, now, "could not point at an MFD");
                        return;
                    }
                }
                1 => {
                    act.free_look = true;
                    if now - t.next_s > 1.0 {
                        let during = pages(&k);
                        info!("playtest: MFD pages in focus after the press {during:?}");
                        if during == defaults {
                            t.failures.push("an MFD press changed no page".into());
                        }
                        k.hands.hold_keys.retain(|k| *k != KeyCode::ControlLeft);
                        t.leg = 2;
                        t.next_s = now;
                    }
                }
                _ => {
                    if now - t.next_s > 1.0 {
                        let after = pages(&k);
                        info!("playtest: MFD pages after leaving focus {after:?} (defaults {defaults:?})");
                        if after != defaults {
                            t.failures.push("the MFDs kept their pages after focus ended".into());
                        }
                        done = true;
                    }
                }
            }
        }
        Beat::FireAtGround(ahead, n) => {
            let f = cb_core::mech::forward(m.heading_rad) * *ahead + m.pos;
            let at = Vec3::new(
                f.x,
                s.plan.terrain.height_at(f.x, f.z) + s.live.dug.delta_at(Vec2::new(f.x, f.z)),
                f.z,
            );
            let d = at - eye;
            let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
            act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eye, at, pitch, 0.12);
            act.throttle_zero = true;
            if t.count < *n && now - t.next_s > 2.5 && in_beat > 1.5 {
                t.next_s = now;
                t.count += 1;
                info!(
                    "PLAYTEST_SHOT {} t={:.2} at the ground ({:.0}, {:.0}), {:.0} m off",
                    t.count,
                    now - t.start_s,
                    at.x,
                    at.z,
                    d.length()
                );
            }
            act.fire_missile = t.count > 0 && now - t.next_s < 0.25;
            done = t.count >= *n && now - t.next_s > 4.0;
        }
        Beat::FireAtNearest(kind, each, n) => {
            // The next target: the nearest building (or house) 60 to 800 m from the mech not fired at yet (nearer, the
            // mech would take its own blasts).
            if t.count == 0 || (now - t.next_s > *each && t.count < *n) {
                let here = Vec2::new(m.pos.x, m.pos.z);
                let near = |p: Vec3| p.distance(eye);
                let next = if kind == "house" {
                    s.plan
                        .small
                        .candidates(here, 800.0)
                        .into_iter()
                        .filter(|l| !t.shot.contains(&(l | HOUSE_ID)))
                        .filter_map(|l| s.plan.small.lots.get(l as usize).map(|lot| (l | HOUSE_ID, lot)))
                        .map(|(id, lot)| (id, Vec3::new(lot.centre.x, lot.ground_m + 3.0, lot.centre.y)))
                        .filter(|c| near(c.1) > 60.0)
                        .min_by(|a, b| near(a.1).total_cmp(&near(b.1)))
                } else {
                    s.plan
                        .collision
                        .candidates(here, 800.0)
                        .into_iter()
                        .filter(|b| !t.shot.contains(b))
                        .filter_map(|b| s.plan.buildings.get(b as usize).map(|bl| (b, bl)))
                        .map(|(id, bl)| {
                            let c = bl.footprint.centre();
                            (id, Vec3::new(c.x, s.plan.terrain.height_at(c.x, c.y) + bl.height_m * 0.35, c.y))
                        })
                        .filter(|c| near(c.1) > 60.0)
                        .min_by(|a, b| near(a.1).total_cmp(&near(b.1)))
                };
                match next {
                    Some((id, at)) => {
                        t.shot.push(id);
                        t.aim = at;
                        t.count += 1;
                        t.next_s = now;
                        say(&format!(
                            "PLAYTEST_TARGET {} {kind} {} at ({:.0}, {:.0}), {:.0} m",
                            t.count,
                            id & !HOUSE_ID,
                            at.x,
                            at.z,
                            at.distance(eye)
                        ));
                    }
                    None => {
                        fail(&mut t, now, &format!("no {kind} left within 800 m"));
                        return;
                    }
                }
            }
            let d = t.aim - eye;
            let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
            act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eye, t.aim, pitch, 0.12);
            act.throttle_zero = true;
            let on_target = now - t.next_s;
            act.fire[0] = on_target > 0.8;
            act.fire_missile = on_target > 1.0 && (on_target - 1.0) % 2.5 < 0.25;
            done = t.count >= *n && on_target > *each;
        }
        Beat::StartMission(which) => {
            let kind = match which.as_str() {
                "Defence" => cb_core::missions::MissionKind::Defence,
                _ => cb_core::missions::MissionKind::Fortress,
            };
            let active = x.free.as_deref().and_then(|f| f.free.active);
            if active == Some(kind) {
                if *s.menu == Menu::Mail && press.0.is_none() && now - t.toggled_s > 1.0 {
                    t.toggled_s = now;
                    press.0 = Some((KeyCode::KeyM, false));
                }
                done = !s.menu.is_open();
                if done {
                    info!("playtest: the {which} mission is running");
                }
            } else if *s.menu != Menu::Mail {
                if press.0.is_none() && now - t.toggled_s > 1.0 {
                    t.toggled_s = now;
                    press.0 = Some((KeyCode::KeyM, false));
                }
            } else {
                want_click = Some(MenuButton::StartMission(kind));
            }
        }
        Beat::LandOnFortress(cap) => {
            let Some(f) = x.fortress.as_deref() else {
                fail(&mut t, now, "no Stratostacean in the sky");
                return;
            };
            let me = x.my_id.iter().next().map_or(u32::MAX, |i| i.0);
            let deck = f.f.pose().transform_point3(Vec3::new(0.0, 49.0, 0.0));
            let here = Vec2::new(m.pos.x, m.pos.z);
            let flat = Vec2::new(deck.x, deck.z).distance(here);
            dist = Some(flat + (deck.y - m.pos.y).abs());
            if f.is_aboard(me) {
                if t.seen.is_none() {
                    t.seen = Some(m.boost.heat);
                    t.next_s = now;
                    info!(
                        "playtest: on the deck at {:.0} m up, moving {:.1} m/s with it; jets' heat {:.2}",
                        m.pos.y,
                        m.vel.length(),
                        m.boost.heat
                    );
                }
                act.throttle_zero = true;
                dist = None;
                if now - t.next_s > 6.0 {
                    info!(
                        "playtest: after 6 s on the deck the jets' heat went {:.2} to {:.2}{}",
                        t.seen.unwrap_or(0.0),
                        m.boost.heat,
                        if m.boost.heat < t.seen.unwrap_or(0.0) {
                            " (recharging)"
                        } else {
                            ""
                        }
                    );
                    done = true;
                }
            } else if in_beat > *cap {
                fail(
                    &mut t,
                    now,
                    &format!(
                        "not on the deck after {cap:.0} s, {flat:.0} m off and {:.0} m below it",
                        deck.y - m.pos.y
                    ),
                );
                return;
            } else {
                let want_y = deck.y + if flat > 80.0 { 30.0 } else { 6.0 };
                act = drive(
                    m,
                    p.yaw_rad,
                    p.pitch_rad,
                    Vec2::new(deck.x, deck.z),
                    eye,
                    if flat > 150.0 { 1.0 } else { 0.4 },
                    false,
                    flat < 150.0,
                );
                act.lift_axis = if m.pos.y < want_y { 1.0 } else { 0.0 };
                act.move_xy = if flat > 30.0 { Vec2::Y } else { Vec2::ZERO };
                if now - t.next_s > 3.0 {
                    t.next_s = now;
                    info!(
                        "playtest: closing on the deck, {flat:.0} m off, {:.0} m below it, jets' heat {:.2}",
                        deck.y - m.pos.y,
                        m.boost.heat
                    );
                }
            }
        }
        Beat::WalkToMech(within) => {
            let Some(a) = s.access.0.as_ref() else {
                fail(&mut t, now, "no mech to walk to");
                return;
            };
            let d = Vec2::new(p.feet.x - a.ladder_base.x, p.feet.z - a.ladder_base.z).length();
            dist = Some(d);
            if d <= *within {
                info!(
                    "playtest: {d:.0} m from the mech's stair; Use list {}",
                    use_list(&x.choice.selector)
                );
                done = true;
            } else {
                act = walk_close(p, a.ladder_base, *within * 0.8);
                act.run = d > 10.0;
            }
        }
        Beat::Skates(on) => {
            act.skate_toggle = m.skates_on != *on;
            done = m.skates_on == *on;
        }
    }
    // A click in progress, or a new one.
    if let Some(want) = want_click {
        match t.click {
            None if in_beat > 0.4 => {
                if let Some((_, node, at)) = s.buttons.iter().find(|(b, n, _)| **b == want && n.size().x > 0.0) {
                    let centre = bevy::math::Affine2::from(at).translation * node.inverse_scale_factor();
                    info!("playtest: clicking {want:?} at ({:.0}, {:.0})", centre.x, centre.y);
                    t.click = Some(Click {
                        at: centre,
                        stage: 0,
                        since_s: now,
                    });
                } else if in_beat > 8.0 {
                    fail(&mut t, now, &format!("no {want:?} button on the screen"));
                    return;
                }
            }
            _ => {}
        }
    }
    if let Some(mut c) = t.click {
        let target = s
            .window
            .single()
            .ok()
            .and_then(|w| RenderTarget::Window(WindowRef::Primary).normalize(Some(w)));
        if let Some(target) = target {
            let loc = Location { target, position: c.at };
            let action = match c.stage {
                0 => Some(PointerAction::Move { delta: Vec2::ZERO }),
                1 if now - c.since_s > 0.35 => Some(PointerAction::Press(PointerButton::Primary)),
                2 if now - c.since_s > 0.45 => Some(PointerAction::Release(PointerButton::Primary)),
                _ => None,
            };
            if let Some(a) = action {
                pointer.write(PointerInput::new(PointerId::Mouse, loc, a));
                c.stage += 1;
            }
        }
        t.click = (c.stage < 3).then_some(c);
        if c.stage >= 3 {
            t.clicked = true;
        }
    }
    // One press of the skate key, then time for it to take, never a press every frame.
    if act.skate_toggle {
        if now - t.toggled_s < 1.5 {
            act.skate_toggle = false;
        } else {
            t.toggled_s = now;
        }
    }
    if now - t.logged_s > 5.0 {
        t.logged_s = now;
        info!(
            "track: beat {} pilot ({:.0}, {:.0}) {:?} mech ({:.0}, {:.0}) heading {:.2} throttle {:.2} speed {:.1} skates {} dist {:?}",
            t.beat,
            p.feet.x,
            p.feet.z,
            std::mem::discriminant(&p.mode),
            m.pos.x,
            m.pos.z,
            m.heading_rad,
            m.throttle,
            m.vel.length(),
            m.skates_on,
            dist
        );
    }
    // A beat with no distance to close (a press, a click) that has not happened in 20 s is stuck too.
    let timed = !matches!(
        beat,
        Beat::Wait(_)
            | Beat::LookAround(_)
            | Beat::FightJob(_)
            | Beat::EnterBay(_)
            | Beat::AcceptByMail(..)
            | Beat::ExpectMode(..)
            | Beat::LookAtNearest(..)
            | Beat::FireFor(_)
            | Beat::Scope(..)
            | Beat::MultiLock(_)
            | Beat::FireAtGround(..)
            | Beat::FireAtNearest(..)
            | Beat::LandOnFortress(_)
            | Beat::DriveCar(..)
    ) && dist.is_none();
    if timed && in_beat > 20.0 && !done {
        say(&format!(
            "PLAYTEST_RESULT stuck in beat {} {beat:?}: nothing happened in 20 s",
            t.beat
        ));
        t.ending = Some(now);
        return;
    }
    // Stuck: no progress toward the goal for `stuck_s` of game time.
    if let Some(d) = dist {
        if d < t.best_m - 0.1 {
            t.best_m = d;
            t.progress_game_s = game_s;
        } else if game_s - t.progress_game_s > t.script.stuck_s {
            // A job that cannot be reached by mail is given up for the next one the same way (at most three): the
            // run goes on across the island, and the report says so.
            let job = run.as_deref().and_then(|c| c.career.board.current().map(|j| j.id));
            let by_mail = t.beat > 0 && matches!(beats.get(t.beat - 1), Some(Beat::AcceptByMail(..)));
            if matches!(beat, Beat::GoToJob)
                && by_mail
                && t.gave_up.len() < 3
                && let Some(id) = job
            {
                say(&format!(
                    "PLAYTEST_NOTE gave up on job {id}: no way through at ({:.0}, {:.0}), {d:.0} m short",
                    m.pos.x, m.pos.z
                ));
                t.gave_up.push(id);
                t.beat -= 1;
                t.beat_real_s = now;
                t.beat_game_s = game_s;
                t.progress_game_s = game_s;
                t.best_m = f32::INFINITY;
                t.click = None;
                t.clicked = false;
                t.peak_m_s = 0.0;
                t.detour_until = 0.0;
                announce(&t);
                acc.script = Some(Actions::default());
                return;
            }
            let msg = format!(
                "stuck in beat {} {beat:?}: {d:.0} m from its goal, pilot at ({:.0}, {:.0}), mech at ({:.0}, {:.0})",
                t.beat, p.feet.x, p.feet.z, m.pos.x, m.pos.z
            );
            say(&format!("PLAYTEST_RESULT stuck {msg}"));
            t.ending = Some(now);
            return;
        }
    } else {
        t.progress_game_s = game_s;
    }
    acc.script = Some(act);
    if done {
        info!("playtest: beat {} {beat:?} done in {in_beat:.1} s", t.beat);
        t.beat += 1;
        t.beat_real_s = now;
        t.beat_game_s = game_s;
        t.progress_game_s = game_s;
        t.best_m = f32::INFINITY;
        t.leg = 0;
        t.peak_m_s = 0.0;
        t.detour_until = 0.0;
        t.click = None;
        t.clicked = false;
        t.brain = None;
        t.goal_id = None;
        t.route.clear();
        t.route_i = 0;
        t.count = 0;
        t.next_s = 0.0;
        t.seen = None;
        k.hands.hold_buttons.clear();
        k.hands.hold_keys.clear();
        if t.beat < t.script.runs[t.run].beats.len() {
            announce(&t);
        }
    }
}

fn announce(t: &Pilot) {
    if let Some(b) = t.script.runs[t.run].beats.get(t.beat) {
        say(&format!("PLAYTEST_BEAT {} {b:?}", t.beat));
    }
}

/// The run ends: its result, then a two-second hold before `PLAYTEST_END`.
fn finish(t: &mut Pilot, now: f32, failure: Option<&str>) {
    if t.ending.is_some() {
        return;
    }
    if let Some(f) = failure {
        t.failures.push(f.to_string());
    }
    let name = &t.script.name;
    if t.failures.is_empty() {
        say(&format!("PLAYTEST_RESULT ok {name} run {}: every beat played", t.run));
    } else {
        say(&format!("PLAYTEST_RESULT fail {name} run {}: {}", t.run, t.failures.join("; ")));
    }
    t.ending = Some(now);
}

fn fail(t: &mut Pilot, now: f32, why: &str) {
    let beat = t.script.runs[t.run].beats.get(t.beat).map(|b| format!("{b:?}")).unwrap_or_default();
    finish(t, now, Some(&format!("beat {} {beat}: {why}", t.beat)));
}

fn expect_path(name: &str) -> std::path::PathBuf {
    crate::cli::repo_root().join("output/playtests").join(format!("{name}-expect.ron"))
}

/// The career and where the player is, as `ExpectRestored` compares them.
fn now_expect(career: Option<&CareerRun>, p: &cb_core::pilot::PilotState, m: &cb_core::mech::MechState) -> Option<Expect> {
    let c = &career?.career;
    let seated = p.mode == PilotMode::Seated;
    let at = if seated { [m.pos.x, m.pos.z] } else { [p.feet.x, p.feet.z] };
    Some(Expect {
        money: c.money,
        mechs: c.mechs.iter().map(|o| o.design.clone()).collect(),
        fit: c.piloting.and_then(|k| c.mech(k)).map(|o| o.fit.clone()).unwrap_or_default(),
        done: c.done.len(),
        at,
        seated,
    })
}

fn write_expect(name: &str, career: Option<&CareerRun>, p: &cb_core::pilot::PilotState, m: &cb_core::mech::MechState) {
    let Some(e) = now_expect(career, p, m) else { return };
    let path = expect_path(name);
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
    let text = e.to_ron();
    if let Err(err) = std::fs::write(&path, text) {
        warn!("playtest: could not write {}: {err}", path.display());
    } else {
        info!("playtest: quitting with {} credits at ({:.0}, {:.0})", e.money, e.at[0], e.at[1]);
    }
}

/// Walks toward `target` until within `stop_m` of it (`walk_to` stops at 2 m, too far for a car's door or a stair).
fn walk_close(p: &cb_core::pilot::PilotState, target: Vec3, stop_m: f32) -> Actions {
    let to = target - p.feet;
    let flat = Vec2::new(to.x, to.z).length();
    let mut a = walk_to(p, target, 0.0);
    a.move_xy = if flat < stop_m { Vec2::ZERO } else { Vec2::new(0.0, 1.0) };
    // Turn first, then go: a walk round a tight corner (a door, a stair) does not cut it.
    let want = (-to.x).atan2(-to.z);
    if wrap_rad(want - p.yaw_rad).abs() > 0.6 {
        a.move_xy = Vec2::ZERO;
    }
    a
}

/// The Use list as the prompt shows it, the selected one in brackets.
fn use_list(sel: &cb_core::interact::Selector) -> String {
    let list: Vec<String> = sel
        .options()
        .iter()
        .map(|o| {
            if Some(*o) == sel.selected() {
                format!("[{o:?}]")
            } else {
                format!("{o:?}")
            }
        })
        .collect();
    if list.is_empty() {
        "nothing in reach".into()
    } else {
        list.join(" ")
    }
}

/// The nearest thing of a kind to look at, from `eyes`: where, and what it is.
fn nearest(kind: &str, eyes: Vec3, s: &Scene, x: &More) -> Option<(Vec3, String)> {
    let flat = |v: Vec3| Vec2::new(v.x, v.z);
    let here = flat(eyes);
    let pick = |it: Vec<(Vec3, String)>| it.into_iter().min_by(|a, b| a.0.distance(eyes).total_cmp(&b.0.distance(eyes)));
    let ground = |q: Vec2| s.plan.terrain.height_at(q.x, q.y);
    match kind {
        "person" | "bench" => {
            let crowd = x.crowd.as_deref()?;
            pick(
                crowd
                    .people
                    .list
                    .iter()
                    .filter(|q| q.standing())
                    .filter(|q| kind != "bench" || matches!(q.activity, cb_core::civilians::Activity::Sitting { .. }))
                    .map(|q| (q.feet() + Vec3::Y * 1.2, format!("person {} {:?}", q.id, q.activity)))
                    .collect(),
            )
        }
        "car" => pick(
            x.traffic
                .as_deref()?
                .traffic
                .cars
                .iter()
                .map(|c| {
                    (
                        c.pos + Vec3::Y,
                        format!("car {} at {:.1} m/s {:?}", c.id, c.vel().length(), c.state),
                    )
                })
                .collect(),
        ),
        "light" => {
            let roads = x.roads.as_deref()?;
            let tr = x.traffic.as_deref()?;
            let times = &s.data.0.traffic.cfg.lights;
            pick(
                roads
                    .0
                    .nodes
                    .iter()
                    .filter(|n| n.signal && n.at.distance(here) > 15.0)
                    .map(|n| {
                        let t_s = tr.traffic.time_s();
                        (
                            Vec3::new(n.at.x, ground(n.at) + 4.0, n.at.y),
                            format!(
                                "junction {} lights {:?} / {:?}",
                                n.id,
                                cb_core::traffic::light(times, n.id, 0, t_s),
                                cb_core::traffic::light(times, n.id, 1, t_s)
                            ),
                        )
                    })
                    .collect(),
            )
        }
        "aircraft" => pick(
            x.air
                .as_deref()?
                .0
                .list
                .iter()
                .map(|a| {
                    (
                        a.pos,
                        format!("{} {} {:?} at {:.0} m up", a.kind, a.id, a.phase, a.pos.y - ground(flat(a.pos))),
                    )
                })
                .collect(),
        ),
        "boat" => pick(
            x.boats
                .as_deref()?
                .traffic
                .boats
                .iter()
                .map(|b| (Vec3::new(b.pos.x, 3.0, b.pos.y), format!("{} {} {:?}", b.kind, b.id, b.phase)))
                .collect(),
        ),
        "harbour" => pick(
            x.boats
                .as_deref()?
                .traffic
                .harbours
                .iter()
                .map(|h| {
                    (
                        Vec3::new(h.berth.x, h.deck_y + 6.0, h.berth.y),
                        format!("harbour {} of city {}", h.id, h.city),
                    )
                })
                .collect(),
        ),
        "door" => {
            let rng = crate::buildings::building_rng(x.seed.0);
            pick(
                x.colliders
                    .0
                    .buildings
                    .keys()
                    .filter_map(|&b| {
                        let c = s.live.structures.buildings.get(b as usize)?;
                        let site = cb_core::world::interior::Site::of(&s.plan.0, b, c.n, c.origin, c.size);
                        let e = cb_core::world::interior::shell(&site, &x.colliders.0.cfg, &rng).entrance;
                        Some((
                            Vec3::new(e.centre.x, eyes.y, e.centre.y),
                            format!("building {b}'s {:?} door", e.kind),
                        ))
                    })
                    .collect(),
            )
        }
        "enemy" => pick(
            s.enemies
                .iter()
                .filter(|(e, _)| !e.dead)
                .map(|(e, _)| (e.curr.pos + Vec3::Y * 8.0, "an enemy mech".to_string()))
                .chain(
                    x.hostiles
                        .iter()
                        .flat_map(|h| h.0.iter().map(|h| (h.aim, format!("hostile {}", h.id)))),
                )
                .collect(),
        ),
        "fortress" => x.fortress.as_deref().map(|f| {
            (
                f.f.pose().transform_point3(Vec3::new(0.0, 49.0, 0.0)),
                "the Stratostacean's deck".to_string(),
            )
        }),
        _ => None,
    }
}

/// The nearest place of a kind to go to, from `here`: its point on the ground and what it is.
fn place(kind: &str, here: Vec2, s: &Scene, x: &More) -> Option<(Vec2, String)> {
    let pick = |it: Vec<(Vec2, String)>| it.into_iter().min_by(|a, b| a.0.distance(here).total_cmp(&b.0.distance(here)));
    match kind {
        "harbour" => pick(
            x.boats
                .as_deref()?
                .traffic
                .harbours
                .iter()
                .map(|h| (h.apron.centre, format!("harbour {} of city {}", h.id, h.city)))
                .collect(),
        ),
        // the middle of the nearest runway that is not the one underfoot
        "runway" => pick(
            s.plan
                .bases
                .iter()
                .filter(|b| b.centre.distance(here) > 800.0)
                .map(|b| ((b.runway.a + b.runway.b) * 0.5, format!("air base {}'s runway", b.id + 1)))
                .collect(),
        ),
        "city" => pick(
            s.plan
                .cities
                .iter()
                .filter(|c| c.centre.distance(here) > c.half_size_m)
                .map(|c| (c.centre, format!("city {}", c.id)))
                .collect(),
        ),
        // the nearest house lot more than 300 m off (a suburb to bring down)
        "house" => pick(
            s.plan
                .small
                .lots
                .iter()
                .enumerate()
                .filter(|(_, l)| l.centre.distance(here) > 300.0)
                .map(|(i, l)| (l.centre, format!("house lot {i}")))
                .collect(),
        ),
        // the city farthest from here (the fast-flight runs)
        "farthest-city" => s
            .plan
            .cities
            .iter()
            .max_by(|a, b| a.centre.distance(here).total_cmp(&b.centre.distance(here)))
            .map(|c| (c.centre, format!("city {}, {:.0} m off", c.id, c.centre.distance(here)))),
        _ => None,
    }
}

/// One line of what is about the pilot within `r` metres, for the report (`Beat::Survey`).
#[allow(clippy::too_many_arguments)]
fn survey(
    s: &Scene,
    x: &More,
    k: &Cockpit,
    p: &cb_core::pilot::PilotState,
    m: &cb_core::mech::MechState,
    spec: &MechSpec,
    sim: &MechSim,
    r: f32,
) -> String {
    use cb_core::civilians::Activity as A;
    let here = if p.mode == PilotMode::Seated {
        Vec2::new(m.pos.x, m.pos.z)
    } else {
        Vec2::new(p.feet.x, p.feet.z)
    };
    let mode = match p.mode {
        PilotMode::OnFoot => "on foot",
        PilotMode::Seated => "seated",
        PilotMode::Driving => "driving",
        _ => "between",
    };
    let mut out = vec![format!("pilot {mode} at ({:.0}, {:.0}, {:.1} up)", here.x, here.y, p.feet.y)];
    if let Some(c) = x.crowd.as_deref() {
        let near: Vec<_> = c.people.list.iter().filter(|q| q.pos.distance(here) < r).collect();
        let n = |f: &dyn Fn(&A) -> bool| near.iter().filter(|q| f(&q.activity)).count();
        out.push(format!(
            "people {}: walking {}, waiting at a kerb {}, idle {}, on a bench {}, greeting {}, fleeing {} ({} for a door), down {}",
            near.len(),
            n(&|a| matches!(a, A::Walking)),
            n(&|a| matches!(a, A::Waiting { .. })),
            n(&|a| matches!(a, A::Idle { .. })),
            n(&|a| matches!(a, A::Sitting { .. })),
            n(&|a| matches!(a, A::Greeting { .. })),
            n(&|a| matches!(a, A::Fleeing { .. })),
            n(&|a| matches!(a, A::Fleeing { door: Some(_), .. })),
            n(&|a| matches!(a, A::Down { .. })),
        ));
    }
    if let Some(tr) = x.traffic.as_deref() {
        let near: Vec<_> = tr
            .traffic
            .cars
            .iter()
            .filter(|c| Vec2::new(c.pos.x, c.pos.z).distance(here) < r)
            .collect();
        let moving = near.iter().filter(|c| c.vel().length() > 0.5).count();
        let wrecks = near.iter().filter(|c| c.state.wreck()).count();
        out.push(format!(
            "cars {}: moving {moving}, standing {}, wrecks {wrecks}",
            near.len(),
            near.len() - moving - wrecks
        ));
    }
    if let (Some(roads), Some(tr)) = (x.roads.as_deref(), x.traffic.as_deref()) {
        let lit: Vec<_> = roads.0.nodes.iter().filter(|n| n.signal && n.at.distance(here) < r).collect();
        let times = &s.data.0.traffic.cfg.lights;
        let shows: Vec<String> = lit
            .iter()
            .take(4)
            .map(|n| {
                format!(
                    "{}: {:?}/{:?}",
                    n.id,
                    cb_core::traffic::light(times, n.id, 0, tr.traffic.time_s()),
                    cb_core::traffic::light(times, n.id, 1, tr.traffic.time_s())
                )
            })
            .collect();
        out.push(format!("lit junctions {} [{}]", lit.len(), shows.join(", ")));
    }
    if let Some(a) = x.air.as_deref() {
        let ground = |v: Vec3| s.plan.terrain.height_at(v.x, v.z);
        let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
        for f in &a.0.list {
            *kinds.entry(f.kind.clone()).or_default() += 1;
        }
        let near = a.0.list.iter().min_by(|p1, p2| {
            Vec2::new(p1.pos.x, p1.pos.z)
                .distance(here)
                .total_cmp(&Vec2::new(p2.pos.x, p2.pos.z).distance(here))
        });
        out.push(format!(
            "aircraft {} {:?}{}",
            a.0.list.len(),
            kinds,
            near.map_or(String::new(), |f| format!(
                ", nearest {} {:?} {:.0} m off at {:.0} m up",
                f.kind,
                f.phase,
                Vec2::new(f.pos.x, f.pos.z).distance(here),
                f.pos.y - ground(f.pos)
            ))
        ));
    }
    if let Some(b) = x.boats.as_deref() {
        let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
        for boat in &b.traffic.boats {
            *kinds.entry(boat.kind.clone()).or_default() += 1;
        }
        let harbour = b
            .traffic
            .harbours
            .iter()
            .map(|h| h.apron.centre.distance(here))
            .fold(f32::INFINITY, f32::min);
        out.push(format!(
            "boats {} {:?}, nearest harbour {harbour:.0} m",
            b.traffic.boats.len(),
            kinds
        ));
    }
    if let Some(f) = x.fog.as_deref() {
        out.push(format!("fog {:?} at {:.1} h", f.strength, f.hour_h));
    }
    out.push(format!(
        "targets seen {} (fog scales {:?}), selected {:?}, neutrals boxed {}",
        x.targeting.fog.len(),
        x.targeting
            .fog
            .iter()
            .take(6)
            .map(|(id, f)| (*id, (f * 100.0).round() / 100.0))
            .collect::<Vec<_>>(),
        x.targeting.selected,
        k.neutrals.as_deref().map_or(0, |n| n.0.len())
    ));
    out.push(format!("walk-in colliders built {}", x.colliders.0.buildings.len()));
    out.push(format!(
        "mech {}: jets burn {:.0} s (heat {:.2}), skates burn {:.0} s (heat {:.2}), weapon heat {:.2}, station {} {}",
        spec.name,
        spec.jet.burn_s,
        m.boost.heat,
        spec.skate.burn_s,
        m.skate.heat,
        sim.guns.heat.heat,
        sim.selected + 1,
        spec.weapons.get(sim.selected).map_or("?", |w| w.name.as_str())
    ));
    out.join("; ")
}
