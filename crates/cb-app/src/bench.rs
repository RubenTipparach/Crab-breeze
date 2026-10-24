//! The benchmark (`--bench`, openspec change `perf-bench`): plays the showcase take with the
//! pilot's camera rendering into an offscreen 1920 x 1080 image, so frames never wait for a display,
//! and reports frame time by beat. A measurement instrument (CLAUDE.md section 4).

use crate::pilot::PilotCamera;
use crate::showcase::Take;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use std::io::Write;

/// The beats' names, for the report.
const BEATS: [&str; 12] = [
    "load", "look", "board", "beat3", "beat4", "beat5", "beat6", "beat7", "beat8", "beat9", "beat10", "end",
];

/// Measures.
pub struct BenchPlugin;

#[derive(Resource, Default)]
struct Frames {
    started_s: f32,
    by_beat: Vec<Vec<f32>>,
    target: Handle<Image>,
    shot_beats: Vec<usize>,
}

impl Plugin for BenchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Frames>()
            .add_systems(PostStartup, offscreen)
            .add_systems(Last, (record, report).chain());
    }
}

fn offscreen(
    mut commands: Commands,
    camera: Single<Entity, With<PilotCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut frames: ResMut<Frames>,
) {
    let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb));
    let handle = images.add(image);
    frames.target = handle.clone();
    commands.entity(*camera).insert(RenderTarget::Image(handle.into()));
    info!("bench: rendering the pilot's view offscreen at 1920 x 1080");
}

fn record(mut commands: Commands, time: Res<Time<Real>>, take: Res<Take>, mut frames: ResMut<Frames>) {
    // A still of the offscreen image every 4 s, proof that it was really drawn and what it showed.
    let slot = (time.elapsed_secs() / 4.0) as usize;
    if !frames.shot_beats.contains(&slot) && take.beat > 0 {
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
    // Throw away the first 10 s (shader compilation, asset upload).
    if now - frames.started_s < 10.0 {
        return;
    }
    let beat = take.beat.min(BEATS.len() - 1);
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

fn report(mut exit: MessageReader<AppExit>, frames: Res<Frames>) {
    if exit.read().next().is_none() {
        return;
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
            BEATS[i],
            s.len(),
            pct(&s, 0.5),
            pct(&s, 0.95),
            pct(&s, 0.99),
            s[s.len() - 1]
        );
        println!("{line}");
        json.push_str(&format!(
            "{{\"beat\":\"{}\",\"frames\":{},\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"max\":{:.3}}},",
            BEATS[i],
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
