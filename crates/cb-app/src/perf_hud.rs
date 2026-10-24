//! The performance readout (owner, 2026-10-24: "print out fps in the corner", "measure drawcalls,
//! and figure out where the cpu bottleneck is"). A measurement instrument (CLAUDE.md section 4):
//! the frame rate and frame time, the meshes drawn this frame (draw calls before Bevy batches
//! instances), the entity count, and the GPU's own time per frame summed over every render pass
//! (Bevy's render diagnostics: timestamp queries, Vulkan and DX12). A frame much longer than the GPU's
//! time is waiting on the CPU. Every 5 s the slowest passes go to the log.

use bevy::camera::visibility::ViewVisibility;
use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;

#[derive(Component)]
struct PerfText;

/// The corner readout and the pass log.
pub struct PerfHudPlugin;

impl Plugin for PerfHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            RenderDiagnosticsPlugin,
        ))
        .add_systems(Startup, spawn)
        .add_systems(Update, update);
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        PerfText,
        Text::new(""),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(1.0, 0.85, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8.0),
            right: px(10.0),
            ..default()
        },
        GlobalZIndex(900),
    ));
}

/// The GPU time of every render pass this frame, ms, slowest first.
fn gpu_passes(store: &DiagnosticsStore) -> Vec<(String, f64)> {
    let mut passes: Vec<(String, f64)> = store
        .iter()
        .filter(|d| d.path().as_str().ends_with("/elapsed_gpu"))
        .filter_map(|d| {
            d.smoothed()
                .map(|v| (d.path().as_str().trim_end_matches("/elapsed_gpu").to_string(), v))
        })
        .collect();
    passes.sort_by(|a, b| b.1.total_cmp(&a.1));
    passes
}

fn update(
    store: Res<DiagnosticsStore>,
    time: Res<Time<Real>>,
    mut log_in_s: Local<f32>,
    meshes: Query<&ViewVisibility, With<Mesh3d>>,
    mut text: Single<&mut Text, With<PerfText>>,
) {
    let fps = store
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let ms = store
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let entities = store
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0);
    let drawn = meshes.iter().filter(|v| v.get()).count();
    let passes = gpu_passes(&store);
    // The top-level passes nest the others; the largest is the frame's GPU time.
    let gpu = passes.first().map_or(0.0, |p| p.1);
    let bound = if gpu <= 0.0 {
        "GPU time n/a"
    } else if ms > gpu * 1.3 {
        "CPU-bound"
    } else {
        "GPU-bound"
    };
    text.0 = format!("{fps:.0} fps  {ms:.1} ms\nGPU {gpu:.1} ms  {bound}\nmeshes drawn {drawn}\nentities {entities:.0}");
    *log_in_s -= time.delta_secs();
    if *log_in_s <= 0.0 {
        *log_in_s = 5.0;
        let top: Vec<String> = passes.iter().take(8).map(|(p, v)| format!("{p} {v:.2}")).collect();
        info!(
            "perf: {fps:.0} fps, {ms:.1} ms, GPU {gpu:.1} ms ({bound}), {drawn} meshes drawn, {entities:.0} entities | {}",
            top.join(", ")
        );
    }
}
