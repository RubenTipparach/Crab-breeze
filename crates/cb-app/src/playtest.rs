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
        })
        .init_resource::<PressKey>()
        .add_systems(PreUpdate, press_keys.after(bevy::input::InputSystems))
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

#[allow(clippy::too_many_lines)]
fn play(
    mut t: ResMut<Pilot>,
    mut acc: ResMut<InputAccum>,
    mut press: ResMut<PressKey>,
    mut run: Option<ResMut<CareerRun>>,
    mut pointer: MessageWriter<PointerInput>,
    mut exit: MessageWriter<AppExit>,
    s: Scene,
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
        if t.ready_s.is_some_and(|r| now - r > 1.5) && s.access.0.is_some() && run.is_some() {
            t.started = true;
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
                        .filter(|j| j.state == JobState::Offered && j.tier <= *max)
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
                    let d = j.site.distance(Vec2::new(m.pos.x, m.pos.z));
                    dist = Some(d);
                    // Blocked on a long drive (a building, a cliff): after moving, standing still for 2 s sends the
                    // mech toward a point 150 m off to one side (alternating sides) for 8 s, the first 3 s on the
                    // rockets, then back on course, as a player getting round it would.
                    let speed = m.vel.length();
                    t.peak_m_s = t.peak_m_s.max(speed);
                    if speed < 3.0 && t.peak_m_s > 10.0 && now > t.detour_until {
                        t.stall_s += s.real.delta_secs();
                    } else {
                        t.stall_s = 0.0;
                    }
                    if t.stall_s > 2.0 {
                        t.stall_s = 0.0;
                        t.peak_m_s = 0.0;
                        t.detour_side = -t.detour_side;
                        let here = Vec2::new(m.pos.x, m.pos.z);
                        let ahead = (j.site - here).normalize_or_zero();
                        t.detour = here + Vec2::new(-ahead.y, ahead.x) * 150.0 * t.detour_side - ahead * 40.0;
                        t.detour_until = now + 8.0;
                        info!("playtest: blocked at ({:.0}, {:.0}), steering round it", m.pos.x, m.pos.z);
                    }
                    let goal = if now < t.detour_until { t.detour } else { j.site };
                    act = drive(m, p.yaw_rad, p.pitch_rad, goal, eye, 1.0, true, false);
                    // The first 3 s of a detour on the rockets, W held so they swivel forward (rocket-vector): up and
                    // out of whatever caught the mech.
                    if now < t.detour_until && t.detour_until - now > 5.0 {
                        act.lift_axis = 1.0;
                        act.move_xy = Vec2::Y;
                    }
                }
                None => {
                    fail(&mut t, now, "no job under way");
                    return;
                }
            }
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
                // Look toward it, as a player would watch it come down.
                if let Some(d) = career.and_then(|c| c.career.drops.iter().find(|d| !d.landed)) {
                    let eyes = p.feet + Vec3::Y * 1.7;
                    let site = Vec3::from_array(d.site) + Vec3::Y * 60.0;
                    act.look_delta_deg = look_toward(p.yaw_rad, p.pitch_rad, eyes, site, 0.25, 0.05);
                }
            }
        }
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
    }
    // A click in progress, or a new one.
    if let Some(want) = want_click {
        match t.click {
            None if in_beat > 0.4 => {
                if let Some((_, node, at)) = s.buttons.iter().find(|(b, _, _)| **b == want) {
                    let centre = bevy::math::Affine2::from(at).translation * node.inverse_scale_factor();
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
        Beat::Wait(_) | Beat::LookAround(_) | Beat::FightJob(_) | Beat::EnterBay(_) | Beat::AcceptByMail(..)
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
