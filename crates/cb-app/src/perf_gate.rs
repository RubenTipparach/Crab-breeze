//! The 60 fps gate (CLAUDE.md section 7; owner, 2026-10-26: "ensure that when you run perf, everything is always above
//! 60 fps"). Every frame after the loading screen's `GAME_READY` is judged against 16.7 ms by `cb_core::perf`, and when
//! the game exits a `PERF_GATE` line says how many were over, which, and the worst. With `--perf-gate` the game also
//! exits non-zero when any frame was over, so the rule is checkable by anyone (`tools/perf/perf_gate.py`).
//!
//! It also watches the render world's pipeline cache: a pipeline that finishes compiling after `GAME_READY` is a
//! frame the loading screen should have paid for (on macOS Bevy compiles pipelines on the render thread, so each one
//! is a hitch), and each is logged as a `pipeline late:` line with its label and shaders, and counted on the
//! `PERF_GATE` line. A measurement instrument and the gate in one place, so the two cannot disagree.

use crate::loading::GameReady;
use bevy::prelude::*;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
use bevy::render::{Render, RenderApp, RenderSystems};
use cb_core::perf::FrameBudget;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// `GAME_READY` has been printed: read by the render world's pipeline watch.
static READY: AtomicBool = AtomicBool::new(false);
/// Pipelines that finished compiling after `GAME_READY`.
static LATE_PIPELINES: AtomicU32 = AtomicU32::new(0);

/// Whether this run exits non-zero on a frame over budget (`--perf-gate`).
#[derive(Resource, Clone, Copy)]
pub struct PerfGate {
    /// Exit with an error when any frame after `GAME_READY` was over 16.7 ms.
    pub enforce: bool,
}

/// The tally of frames after `GAME_READY`.
#[derive(Resource, Default)]
struct Tally {
    budget: FrameBudget,
    /// The frame `GAME_READY` was seen on is not counted: its length is the last loading frame's.
    armed: bool,
    reported: bool,
}

/// The gate and the late-pipeline watch.
pub struct PerfGatePlugin {
    /// See `PerfGate::enforce`.
    pub enforce: bool,
}

impl Plugin for PerfGatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PerfGate { enforce: self.enforce })
            .init_resource::<Tally>()
            .add_systems(Last, (count, report).chain());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(Render, watch_pipelines.in_set(RenderSystems::Cleanup));
        }
    }
}

/// Adds this frame's length to the tally, from the frame after `GAME_READY` on.
fn count(mut tally: ResMut<Tally>, ready: Option<Res<GameReady>>, time: Res<Time<Real>>) {
    if !ready.is_some_and(|r| r.0) {
        return;
    }
    if !tally.armed {
        tally.armed = true;
        READY.store(true, Ordering::Relaxed);
        info!("PERF_GATE armed at GAME_READY");
        return;
    }
    let ms = time.delta_secs() * 1000.0;
    tally.budget.add(ms);
    if ms > tally.budget.budget_ms {
        info!("PERF_GATE over: frame {} after GAME_READY took {ms:.1} ms", tally.budget.frames - 1);
    }
}

/// On the way out, the `PERF_GATE` line; with `--perf-gate`, an error exit when a frame was over.
fn report(
    mut tally: ResMut<Tally>,
    gate: Res<PerfGate>,
    mut exits: ResMut<Messages<AppExit>>,
    mut seen: Local<bevy::ecs::message::MessageCursor<AppExit>>,
) {
    // One parameter both reads the exit and writes the failure (a reader and a writer of one message conflict).
    if tally.reported || seen.read(&exits).next().is_none() {
        return;
    }
    tally.reported = true;
    let b = &tally.budget;
    let late = LATE_PIPELINES.load(Ordering::Relaxed);
    let verdict = if b.passed() { "pass" } else { "fail" };
    let listed: Vec<String> = b.listed.iter().take(40).map(|o| format!("#{} {:.1}", o.frame, o.ms)).collect();
    let line = format!(
        "PERF_GATE {verdict}: {} of {} frames after GAME_READY over {:.1} ms, worst {:.1} ms, {late} pipelines compiled after GAME_READY{}{}",
        b.over,
        b.frames,
        b.budget_ms,
        b.worst_ms,
        if listed.is_empty() { "" } else { " | over: " },
        listed.join(", ")
    );
    // On stdout, as GAME_READY is, so a recorder or the gate tool reads it without the log's filters.
    println!("{line}");
    if gate.enforce && !b.passed() {
        exits.write(AppExit::from_code(3));
    }
}

/// Logs every pipeline that finishes compiling after `GAME_READY` (render world).
fn watch_pipelines(cache: Res<PipelineCache>, mut done: Local<Vec<bool>>, mut first_open: Local<usize>) {
    let ready = READY.load(Ordering::Relaxed);
    let all: Vec<_> = cache.pipelines().collect();
    if done.len() < all.len() {
        done.resize(all.len(), false);
    }
    let mut first = None;
    for (i, p) in all.iter().enumerate().skip(*first_open) {
        if done[i] {
            continue;
        }
        if matches!(p.state, CachedPipelineState::Ok(_) | CachedPipelineState::Err(_)) {
            done[i] = true;
            if ready {
                LATE_PIPELINES.fetch_add(1, Ordering::Relaxed);
                info!("pipeline late: {}", describe(&p.descriptor));
            }
        } else if first.is_none() {
            first = Some(i);
        }
    }
    *first_open = first.unwrap_or(all.len());
}

/// A pipeline's label and its shaders, for the log.
fn describe(d: &bevy::render::render_resource::PipelineDescriptor) -> String {
    use bevy::render::render_resource::PipelineDescriptor as P;
    let path = |h: &Handle<Shader>| h.path().map_or_else(|| format!("{:?}", h.id()), |p| p.to_string());
    match d {
        P::RenderPipelineDescriptor(r) => format!(
            "{} | vs {} | fs {} | {} defs: {}",
            r.label.as_deref().unwrap_or("?"),
            path(&r.vertex.shader),
            r.fragment.as_ref().map_or_else(|| "-".to_string(), |f| path(&f.shader)),
            r.vertex.shader_defs.len(),
            // The variant itself (vertex layout, material features, view flags): what a warm-up behind the loading
            // screen has to draw so this one compiles there instead (city-frame section 5).
            r.vertex
                .shader_defs
                .iter()
                .map(|d| match d {
                    bevy::shader::ShaderDefVal::Bool(n, true) => n.to_string(),
                    bevy::shader::ShaderDefVal::Bool(n, false) => format!("!{n}"),
                    bevy::shader::ShaderDefVal::Int(n, v) => format!("{n}={v}"),
                    bevy::shader::ShaderDefVal::UInt(n, v) => format!("{n}={v}"),
                })
                .collect::<Vec<_>>()
                .join(" ")
        ),
        P::ComputePipelineDescriptor(c) => {
            format!("{} | cs {}", c.label.as_deref().unwrap_or("?"), path(&c.shader))
        }
        P::MeshPipelineDescriptor(m) => format!("{} | mesh pipeline", m.label.as_deref().unwrap_or("?")),
    }
}
