//! The performance readout (owner, 2026-10-24: "print out fps in the corner", "measure drawcalls,
//! and figure out where the cpu bottleneck is"). A measurement instrument (CLAUDE.md section 4):
//! the frame rate and frame time, the meshes drawn this frame (draw calls before Bevy batches
//! instances), the entity count, and the GPU's own time per frame summed over every render pass
//! (Bevy's render diagnostics: timestamp queries, Vulkan and DX12). A frame much longer than the GPU's
//! time is waiting on the CPU. Every 5 s the slowest passes go to the log. Under the readout, a
//! frame graph (owner, 2026-10-24: "I need a frame graph on screen"): the last 120 frames as bars,
//! green within 8.3 ms, amber within 16.7 ms, red past it, scaled to 50 ms; every frame over 50 ms is
//! logged as a `hitch:` line, with the frame number, so a hitch can be matched to what happened.

use bevy::camera::visibility::ViewVisibility;
use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;

#[derive(Component)]
struct PerfText;

/// Times a system: logs a `slow:` line when it runs past 4 ms (a measurement instrument; put
/// `let _t = slow("name");` first in a system under suspicion).
pub fn slow(name: &'static str) -> SlowGuard {
    SlowGuard(name, std::time::Instant::now())
}

/// See `slow`.
pub struct SlowGuard(&'static str, std::time::Instant);

impl Drop for SlowGuard {
    fn drop(&mut self) {
        let ms = self.1.elapsed().as_secs_f32() * 1000.0;
        if ms > 4.0 {
            info!("slow: {} took {ms:.1} ms", self.0);
        }
    }
}

/// Bars in the frame graph, one a frame.
const BARS: usize = 120;
/// The graph's height stands for this frame time.
const GRAPH_MS: f32 = 50.0;
const GRAPH_PX: f32 = 60.0;

/// One bar of the frame graph, by slot.
#[derive(Component)]
struct Bar(usize);

/// The corner readout and the pass log.
pub struct PerfHudPlugin;

impl Plugin for PerfHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            RenderDiagnosticsPlugin,
        ));
        // The packaged game (itch.bat builds with `shipping`) keeps the diagnostics but shows no
        // readout or frame graph on screen (owner, 2026-10-25: "itch.bat should disable on csreen
        // debug graph").
        if cfg!(feature = "shipping") {
            return;
        }
        app.add_systems(Startup, spawn).add_systems(Update, (update, graph));
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
    // The frame graph: under the readout, right-aligned, bars growing up from its base.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(84.0),
                right: px(10.0),
                width: px(BARS as f32 * 2.0),
                height: px(GRAPH_PX),
                align_items: AlignItems::FlexEnd,
                border: UiRect::bottom(px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            GlobalZIndex(900),
        ))
        .with_children(|g| {
            for i in 0..BARS {
                g.spawn((
                    Bar(i),
                    Node {
                        width: px(2.0),
                        height: px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.9, 0.3)),
                ));
            }
        });
}

/// Each frame's time as a bar, the newest on the right; frames over 50 ms logged.
fn graph(
    time: Res<Time<Real>>,
    mut frames: Local<std::collections::VecDeque<f32>>,
    mut frame_no: Local<u64>,
    mut bars: Query<(&Bar, &mut Node, &mut BackgroundColor)>,
) {
    let _t = crate::perf_hud::slow("graph");
    let ms = time.delta_secs() * 1000.0;
    *frame_no += 1;
    if ms > GRAPH_MS && *frame_no > 10 {
        info!("hitch: frame {} took {ms:.0} ms", *frame_no);
    }
    frames.push_back(ms);
    while frames.len() > BARS {
        frames.pop_front();
    }
    let pad = BARS - frames.len();
    for (b, mut node, mut colour) in &mut bars {
        let Some(ms) = b.0.checked_sub(pad).and_then(|i| frames.get(i)) else {
            node.height = px(0.0);
            continue;
        };
        node.height = px((ms / GRAPH_MS).min(1.0) * GRAPH_PX);
        colour.0 = if *ms <= 8.4 {
            Color::srgb(0.3, 0.9, 0.3)
        } else if *ms <= 16.8 {
            Color::srgb(1.0, 0.75, 0.2)
        } else {
            Color::srgb(1.0, 0.25, 0.2)
        };
    }
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
    let _t = crate::perf_hud::slow("update");
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
