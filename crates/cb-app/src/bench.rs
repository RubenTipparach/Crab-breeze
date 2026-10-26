//! The benchmark (`--bench`, openspec change `perf-bench`): plays the showcase take with the
//! pilot's camera rendering into an offscreen 1920 x 1080 image, so frames never wait for a display,
//! and reports frame time by beat. A measurement instrument (CLAUDE.md section 4).
//!
//! `CB_BENCH_WINDOWED=1` keeps the window instead, as a player sees it. Stills of the offscreen image only with `CB_BENCH_STILLS=1`; a measured run takes none.
//!
//! For `tools/perf/perf_suite.py` (Pale Blue Dot's suite, ported): with `CB_FRAME_LOG=<path>` it
//! writes one CSV row per frame (wall time, beat, entity count, the render passes' CPU time and,
//! where the GPU has timestamp queries, their GPU time; Metal has none), and at exit a
//! `BENCHPASS` line per render pass with its mean time, so the report can say where the time goes.

use crate::pilot::PilotCamera;
use crate::showcase::Take;
use crate::state::GameApp;
use bevy::camera::RenderTarget;
use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use bevy::render::render_resource::TextureFormat;
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

/// Measures.
pub struct BenchPlugin;

#[derive(Resource, Default)]
struct Frames {
    started_s: f32,
    by_beat: Vec<Vec<f32>>,
    target: Handle<Image>,
    shot_beats: Vec<usize>,
    /// The per-frame CSV, when `CB_FRAME_LOG` names one.
    log: Option<BufWriter<std::fs::File>>,
    /// Per render pass: (sum of CPU ms, sum of GPU ms, frames) after the warm-up.
    passes: BTreeMap<String, (f64, f64, u32)>,
}

impl Plugin for BenchPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<RenderDiagnosticsPlugin>() {
            app.add_plugins(RenderDiagnosticsPlugin);
        }
        if !app.is_plugin_added::<EntityCountDiagnosticsPlugin>() {
            app.add_plugins(EntityCountDiagnosticsPlugin::default());
        }
        let mut frames = Frames::default();
        if let Ok(path) = std::env::var("CB_FRAME_LOG") {
            match std::fs::File::create(&path) {
                Ok(f) => {
                    let mut w = BufWriter::new(f);
                    writeln!(w, "since_start_s,wall_ms,beat,entities,render_cpu_ms,gpu_total_ms").ok();
                    frames.log = Some(w);
                }
                Err(e) => warn!("bench: cannot write the frame log {path}: {e}"),
            }
        }
        app.insert_resource(frames)
            .add_after_world_spawn(offscreen)
            .add_game_systems(Last, (record, report).chain());
    }
}

fn offscreen(
    mut commands: Commands,
    camera: Single<Entity, With<PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut frames: ResMut<Frames>,
) {
    // `CB_BENCH_WINDOWED=1` plays in the window as a player sees it, held to the display's refresh:
    // what it measures is missed refreshes and freezes, not headroom (owner, 2026-10-24: a
    // benchmark with no window "doesn't tell us anything").
    if crate::cli::switched_off("CB_BENCH_WINDOWED") {
        info!("bench: rendering in the window, held to the display");
        return;
    }
    let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb));
    let handle = images.add(image);
    frames.target = handle.clone();
    commands.entity(*camera).insert(RenderTarget::Image(handle.clone().into()));
    // The cockpit pass draws into the same image (cockpit-overlay).
    commands.entity(*cockpit).insert(RenderTarget::Image(handle.into()));
    info!("bench: rendering the pilot's view offscreen at 1920 x 1080");
}

fn record(mut commands: Commands, time: Res<Time<Real>>, take: Res<Take>, diagnostics: Res<DiagnosticsStore>, mut frames: ResMut<Frames>) {
    // A still of the offscreen image every 4 s, proof that it was really drawn and what it showed,
    // only with `CB_BENCH_STILLS=1`: each costs a frame of about 11 ms (the read-back and the
    // write), so a measured run takes none (owner, 2026-10-24: "dont take screenshot on one run or
    // obs, and just do pure perf measurement"). An OBS take is the proof of what was drawn.
    let slot = (time.elapsed_secs() / 4.0) as usize;
    if crate::cli::switched_off("CB_BENCH_STILLS") && !frames.shot_beats.contains(&slot) && take.beat > 0 {
        frames.shot_beats.push(slot);
        let path = crate::cli::repo_root().join(format!("output/bench/t{:03}-beat{:02}.png", slot * 4, take.beat));
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(frames.target.clone()))
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
    let now = time.elapsed_secs();
    if frames.started_s == 0.0 {
        frames.started_s = now;
    }
    // The render passes' own times: `render/<pass>/elapsed_cpu` and `.../elapsed_gpu` (top level
    // only, so nested spans are not counted twice).
    let (mut cpu, mut gpu, mut any_gpu) = (0.0, 0.0, false);
    let warm = now - frames.started_s >= 10.0;
    for d in diagnostics.iter() {
        let path = d.path().as_str();
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 3 || parts[0] != "render" {
            continue;
        }
        let Some(v) = d.measurement().map(|m| m.value) else { continue };
        let entry = frames.passes.entry(parts[1].to_string()).or_insert((0.0, 0.0, 0));
        match parts[2] {
            "elapsed_cpu" => {
                cpu += v;
                if warm {
                    entry.0 += v;
                    entry.2 += 1;
                }
            }
            "elapsed_gpu" => {
                gpu += v;
                any_gpu = true;
                if warm {
                    entry.1 += v;
                }
            }
            _ => {}
        }
    }
    let entities = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.measurement())
        .map_or(0.0, |m| m.value);
    let since = now - frames.started_s;
    let beat_now = take.beat;
    let wall_ms = time.delta_secs() * 1000.0;
    if let Some(log) = frames.log.as_mut() {
        let gpu = if any_gpu { format!("{gpu:.3}") } else { String::new() };
        writeln!(log, "{since:.4},{wall_ms:.3},{beat_now},{entities:.0},{cpu:.3},{gpu}").ok();
    }
    // Throw away the first 10 s (shader compilation, asset upload).
    if now - frames.started_s < 10.0 {
        return;
    }
    let beat = take.beat.min(take.names.len().saturating_sub(1));
    if frames.by_beat.len() <= beat {
        frames.by_beat.resize(beat + 1, Vec::new());
    }
    frames.by_beat[beat].push(time.delta_secs() * 1000.0);
}

fn pct(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

fn report(mut exit: MessageReader<AppExit>, mut frames: ResMut<Frames>, take: Res<Take>) {
    let name = |i: usize| take.names.get(i).copied().unwrap_or("beat");
    if exit.read().next().is_none() {
        return;
    }
    if let Some(log) = frames.log.as_mut() {
        log.flush().ok();
    }
    let mut passes: Vec<(&String, &(f64, f64, u32))> = frames.passes.iter().filter(|p| p.1.2 > 0).collect();
    passes.sort_by(|a, b| (b.1.0 + b.1.1).total_cmp(&(a.1.0 + a.1.1)));
    for (name, (cpu, gpu, n)) in passes {
        let n = f64::from(*n);
        println!("BENCHPASS {name} cpu_ms={:.3} gpu_ms={:.3}", cpu / n, gpu / n);
    }
    let mut all: Vec<f32> = Vec::new();
    let mut json = String::from("{\"beats\":[");
    for (i, f) in frames.by_beat.iter().enumerate() {
        if f.is_empty() {
            continue;
        }
        let mut s = f.clone();
        s.sort_by(f32::total_cmp);
        all.extend_from_slice(&s);
        let line = format!(
            "BENCH {:<8} frames={:>5} p50={:6.2} p95={:6.2} p99={:6.2} max={:7.2}",
            name(i),
            s.len(),
            pct(&s, 0.5),
            pct(&s, 0.95),
            pct(&s, 0.99),
            s[s.len() - 1]
        );
        println!("{line}");
        json.push_str(&format!(
            "{{\"beat\":\"{}\",\"frames\":{},\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"max\":{:.3}}},",
            name(i),
            s.len(),
            pct(&s, 0.5),
            pct(&s, 0.95),
            pct(&s, 0.99),
            s[s.len() - 1]
        ));
    }
    all.sort_by(f32::total_cmp);
    println!(
        "BENCH all      frames={:>5} p50={:6.2} p95={:6.2} p99={:6.2} max={:7.2}",
        all.len(),
        pct(&all, 0.5),
        pct(&all, 0.95),
        pct(&all, 0.99),
        all.last().copied().unwrap_or(0.0)
    );
    json.pop();
    json.push_str("]}\n");
    let dir = crate::cli::repo_root().join("output/bench");
    std::fs::create_dir_all(&dir).ok();
    let name = std::env::var("CB_BENCH_NAME").unwrap_or_else(|_| "bench".into());
    if let Ok(mut f) = std::fs::File::create(dir.join(format!("{name}.json"))) {
        f.write_all(json.as_bytes()).ok();
    }
}
