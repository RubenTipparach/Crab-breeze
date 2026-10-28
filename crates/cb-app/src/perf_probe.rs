//! The live performance probe (openspec change `perf-probe`, a measurement instrument, CLAUDE.md section 4): a
//! running game answers plain HTTP on `127.0.0.1:7878` (`CB_PROBE_PORT`) with what its last frames cost, which
//! systems took the time in the slow ones, what happened in the game on those frames (a building falling, debris
//! spawned, a blast) and what the world holds now (entities by name, physics bodies, meshes, lights). Built only with
//! the `perf_probe` feature; `tools/perf/probe_mcp.py` puts an MCP server in front of it, and `curl` reads it too.
//!
//! | Path | Answers |
//! | --- | --- |
//! | `/summary?s=10` | the last `s` seconds: frames, p50/p95/p99/max ms, frames over 16.7 and 11.1 ms, the ten worst with their systems, counts and events, the systems most often to blame, and the census |
//! | `/frames?since=N&limit=M` | frames after number `N`: number, time, ms, main-world ms, the eye's position and speed |
//! | `/events?since=N` | the game events after frame `N`, each with its frame's and the next frame's ms |
//! | `/census` | entities by the first word of their name, physics bodies by kind, colliders, meshes drawn and in view, lights |
//!
//! Systems' times come from `perf_hud`'s tally, so a `hitch_spans` build names every system; without it only the
//! `slow` and `section` guards appear.

#![cfg_attr(not(feature = "perf_probe"), allow(dead_code))]

use bevy::prelude::*;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write as _};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Frames kept: about a minute at 120 fps.
const KEEP: usize = 8000;
/// Seconds between censuses of the world.
const CENSUS_S: f32 = 1.0;
const SLOW_MS: f32 = 1000.0 / 60.0;
const FAST_MS: f32 = 1000.0 / 90.0;

static ON: AtomicBool = AtomicBool::new(false);
static FRAMES: Mutex<VecDeque<Frame>> = Mutex::new(VecDeque::new());
/// Events written this frame, taken by the next `record`.
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CENSUS: Mutex<String> = Mutex::new(String::new());
/// The eye's position and speed, written by `census` every frame.
static EYE: Mutex<(Vec3, f32)> = Mutex::new((Vec3::ZERO, 0.0));

struct Frame {
    n: u64,
    t_s: f64,
    ms: f32,
    main_ms: f32,
    top: Vec<(&'static str, f32)>,
    counts: Vec<(&'static str, u32)>,
    events: Vec<String>,
    eye: Vec3,
    speed: f32,
}

/// Whether the probe is running (a `perf_probe` build): call sites test it before formatting an event.
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// A game event on this frame (`probe_event!` formats it only when the probe is on).
pub fn event(what: String) {
    if let Ok(mut p) = PENDING.lock() {
        p.push(what);
    }
}

/// Writes a game event to the probe, formatted only in a `perf_probe` build that is running.
#[macro_export]
macro_rules! probe_event {
    ($($arg:tt)*) => {
        if $crate::perf_probe::on() {
            $crate::perf_probe::event(format!($($arg)*));
        }
    };
}

/// One finished frame, from `perf_hud::frame_start`: its number, length, main-world share, timed systems and counts.
pub(crate) fn record(n: u64, ms: f32, main_ms: f32, spans: &[(&'static str, f32)], counts: &[(&'static str, u32)]) {
    if !on() {
        return;
    }
    let mut top: Vec<_> = spans.iter().copied().filter(|s| s.1 >= 0.2).collect();
    top.sort_by(|a, b| b.1.total_cmp(&a.1));
    top.truncate(8);
    let mut counts = counts.to_vec();
    counts.sort_by_key(|c| std::cmp::Reverse(c.1));
    counts.truncate(8);
    let events = PENDING.lock().map(|mut p| std::mem::take(&mut *p)).unwrap_or_default();
    let (eye, speed) = EYE.lock().map(|e| *e).unwrap_or_default();
    let t_s = START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64();
    if let Ok(mut f) = FRAMES.lock() {
        f.push_back(Frame {
            n,
            t_s,
            ms,
            main_ms,
            top,
            counts,
            events,
            eye,
            speed,
        });
        while f.len() > KEEP {
            f.pop_front();
        }
    }
}

static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// The probe: the census each second and the HTTP server.
pub struct PerfProbePlugin;

impl Plugin for PerfProbePlugin {
    fn build(&self, app: &mut App) {
        let port: u16 = std::env::var("CB_PROBE_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(7878);
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => {
                ON.store(true, Ordering::Relaxed);
                info!("perf probe: http://127.0.0.1:{port}/summary");
                std::thread::spawn(move || {
                    for stream in listener.incoming().flatten() {
                        answer(stream);
                    }
                });
            }
            Err(e) => warn!("perf probe: port {port} not bound, probe off: {e}"),
        }
        app.add_systems(Last, census);
    }
}

#[allow(clippy::too_many_arguments)]
fn census(
    time: Res<Time<Real>>,
    mut since: Local<f32>,
    mut last: Local<Option<Vec3>>,
    eye: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    named: Query<Option<&Name>>,
    bodies: Query<&avian3d::prelude::RigidBody>,
    colliders: Query<(), With<avian3d::prelude::Collider>>,
    drawn: Query<&ViewVisibility, With<Mesh3d>>,
    lights: Query<(), Or<(With<PointLight>, With<SpotLight>)>>,
    meshes: Res<Assets<Mesh>>,
) {
    let dt = time.delta_secs();
    if let Ok(g) = eye.single() {
        let p = g.translation();
        let speed = last.map_or(0.0, |l| if dt > 0.0 { l.distance(p) / dt } else { 0.0 });
        *last = Some(p);
        if let Ok(mut e) = EYE.lock() {
            *e = (p, speed);
        }
    }
    *since += dt;
    if *since < CENSUS_S {
        return;
    }
    *since = 0.0;
    let mut by_name: std::collections::BTreeMap<&str, u32> = Default::default();
    let mut total = 0u32;
    for n in &named {
        total += 1;
        let word = n.map_or("unnamed", |n| n.as_str().split([' ', ':']).next().unwrap_or("unnamed"));
        *by_name.entry(word).or_default() += 1;
    }
    let mut names: Vec<_> = by_name.into_iter().collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.1));
    let (mut dynamic, mut kinematic, mut fixed) = (0u32, 0u32, 0u32);
    for b in &bodies {
        match b {
            avian3d::prelude::RigidBody::Dynamic => dynamic += 1,
            avian3d::prelude::RigidBody::Kinematic => kinematic += 1,
            avian3d::prelude::RigidBody::Static => fixed += 1,
        }
    }
    let (mut meshes_drawn, mut in_view) = (0u32, 0u32);
    for v in &drawn {
        meshes_drawn += 1;
        in_view += u32::from(v.get());
    }
    let mut s = String::new();
    let _ = write!(
        s,
        "{{\"entities\":{total},\"bodies\":{{\"dynamic\":{dynamic},\"kinematic\":{kinematic},\"static\":{fixed}}},\
         \"colliders\":{},\"meshes_drawn\":{meshes_drawn},\"meshes_in_view\":{in_view},\"mesh_assets\":{},\
         \"lights\":{},\"by_name\":{{",
        colliders.iter().count(),
        meshes.len(),
        lights.iter().count()
    );
    for (i, (n, c)) in names.iter().take(30).enumerate() {
        let _ = write!(s, "{}\"{}\":{c}", if i > 0 { "," } else { "" }, esc(n));
    }
    s.push_str("}}");
    if let Ok(mut c) = CENSUS.lock() {
        *c = s;
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn answer(stream: std::net::TcpStream) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let target = line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let arg = |k: &str| {
        query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .find(|(a, _)| *a == k)
            .and_then(|(_, v)| v.parse::<f64>().ok())
    };
    let body = match path {
        "/summary" => summary(arg("s").unwrap_or(10.0)),
        "/frames" => frames(arg("since").unwrap_or(0.0) as u64, arg("limit").unwrap_or(2000.0) as usize),
        "/events" => events(arg("since").unwrap_or(0.0) as u64),
        "/census" => CENSUS.lock().map(|c| c.clone()).unwrap_or_default(),
        _ => "{\"paths\":[\"/summary?s=10\",\"/frames?since=0&limit=2000\",\"/events?since=0\",\"/census\"]}".into(),
    };
    let mut stream = stream;
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn frame_json(f: &Frame, next_ms: Option<f32>) -> String {
    let mut s = format!(
        "{{\"n\":{},\"t_s\":{:.3},\"ms\":{:.2},\"main_ms\":{:.2},\"next_ms\":{},\"eye\":[{:.0},{:.0},{:.0}],\"speed_m_s\":{:.1},\"systems\":{{",
        f.n,
        f.t_s,
        f.ms,
        f.main_ms,
        next_ms.map_or("null".into(), |m| format!("{m:.2}")),
        f.eye.x,
        f.eye.y,
        f.eye.z,
        f.speed
    );
    for (i, (n, v)) in f.top.iter().enumerate() {
        let _ = write!(s, "{}\"{}\":{v:.2}", if i > 0 { "," } else { "" }, esc(n));
    }
    s.push_str("},\"counts\":{");
    for (i, (n, c)) in f.counts.iter().enumerate() {
        let _ = write!(s, "{}\"{}\":{c}", if i > 0 { "," } else { "" }, esc(n));
    }
    s.push_str("},\"events\":[");
    for (i, e) in f.events.iter().enumerate() {
        let _ = write!(s, "{}\"{}\"", if i > 0 { "," } else { "" }, esc(e));
    }
    s.push_str("]}");
    s
}

fn pct(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

fn summary(seconds: f64) -> String {
    let Ok(f) = FRAMES.lock() else { return "{}".into() };
    let Some(end) = f.back().map(|b| b.t_s) else {
        return "{\"frames\":0}".into();
    };
    let from = f.iter().position(|x| x.t_s >= end - seconds).unwrap_or(0);
    let win: Vec<&Frame> = f.range(from..).collect();
    let mut ms: Vec<f32> = win.iter().map(|x| x.ms).collect();
    ms.sort_by(f32::total_cmp);
    let over = |l: f32| ms.iter().filter(|m| **m > l).count();
    // A main-world spike shows in the next frame's length (pipelined rendering): a frame is slow when it or the one
    // after it is over the line, and both lengths are reported.
    let mut idx: Vec<usize> = (0..win.len()).collect();
    let worst_of = |i: usize| win[i].ms.max(win[i].main_ms);
    idx.sort_by(|a, b| worst_of(*b).total_cmp(&worst_of(*a)));
    let mut blame: std::collections::BTreeMap<&str, (u32, f32)> = Default::default();
    for x in win.iter().filter(|x| x.ms > SLOW_MS || x.main_ms > SLOW_MS) {
        if let Some((n, v)) = x
            .top
            .iter()
            .find(|(n, _)| !n.ends_with("run_render_schedule") && !n.ends_with("run_fixed_main_schedule"))
        {
            let e = blame.entry(n).or_default();
            e.0 += 1;
            e.1 = e.1.max(*v);
        }
    }
    let mut blame: Vec<_> = blame.into_iter().collect();
    blame.sort_by_key(|b| std::cmp::Reverse(b.1.0));
    let mut s = format!(
        "{{\"seconds\":{seconds},\"frames\":{},\"fps\":{:.1},\"p50_ms\":{:.2},\"p95_ms\":{:.2},\"p99_ms\":{:.2},\"max_ms\":{:.2},\
         \"over_16_7\":{},\"over_11_1\":{},\"last_frame\":{},\"worst\":[",
        win.len(),
        win.len() as f64 / seconds.min(end).max(1e-3),
        pct(&ms, 0.5),
        pct(&ms, 0.95),
        pct(&ms, 0.99),
        ms.last().copied().unwrap_or(0.0),
        over(SLOW_MS),
        over(FAST_MS),
        f.back().map_or(0, |b| b.n)
    );
    for (k, &i) in idx.iter().take(10).enumerate() {
        let next = win.get(i + 1).map(|x| x.ms);
        let _ = write!(s, "{}{}", if k > 0 { "," } else { "" }, frame_json(win[i], next));
    }
    s.push_str("],\"blamed\":[");
    for (k, (n, (c, w))) in blame.iter().take(12).enumerate() {
        let _ = write!(
            s,
            "{}{{\"system\":\"{}\",\"slow_frames\":{c},\"worst_ms\":{w:.2}}}",
            if k > 0 { "," } else { "" },
            esc(n)
        );
    }
    let census = CENSUS.lock().map(|c| c.clone()).unwrap_or_default();
    let _ = write!(s, "],\"census\":{}}}", if census.is_empty() { "null" } else { &census });
    s
}

fn frames(since: u64, limit: usize) -> String {
    let Ok(f) = FRAMES.lock() else { return "[]".into() };
    let mut s = String::from("[");
    for (k, x) in f.iter().filter(|x| x.n > since).take(limit).enumerate() {
        let _ = write!(
            s,
            "{}[{},{:.3},{:.2},{:.2},{:.0},{:.0},{:.0},{:.1}]",
            if k > 0 { "," } else { "" },
            x.n,
            x.t_s,
            x.ms,
            x.main_ms,
            x.eye.x,
            x.eye.y,
            x.eye.z,
            x.speed
        );
    }
    s.push(']');
    s
}

fn events(since: u64) -> String {
    let Ok(f) = FRAMES.lock() else { return "[]".into() };
    let mut s = String::from("[");
    let mut k = 0;
    let all: Vec<&Frame> = f.iter().collect();
    for (i, x) in all.iter().enumerate().filter(|(_, x)| x.n > since && !x.events.is_empty()) {
        let next = all.get(i + 1).map_or("null".to_string(), |y| format!("{:.2}", y.ms));
        for e in &x.events {
            let _ = write!(
                s,
                "{}{{\"n\":{},\"t_s\":{:.3},\"ms\":{:.2},\"next_ms\":{next},\"event\":\"{}\"}}",
                if k > 0 { "," } else { "" },
                x.n,
                x.t_s,
                x.ms,
                esc(e)
            );
            k += 1;
        }
    }
    s.push(']');
    s
}
