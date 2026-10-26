//! The loading screen and the ready signal (CLAUDE.md section 9; owner, 2026-10-24: "we need a
//! loading screen and a signal to obs when all game stuff is loaded to record..otherwise we get
//! blank screens"). A full-screen panel covers the view until the game is ready: the pilot's mech
//! is bound to its model (so its glb and the world have loaded and spawned) and the boarding access
//! is published, and the sky light is on the eye camera, then the frames hold steady for `STEADY_S` (shader pipelines compile on their first
//! frames and show as long hitches). Then it prints `GAME_READY` once and lifts the panel. The
//! scripted takes begin only after it, so a recording never opens on a blank or half-built window.

use crate::mech::{MechAccessNow, MechRig, Piloted};
use crate::state::GameApp;
use bevy::prelude::*;

/// Seconds of steady frames needed after everything is in.
const STEADY_S: f32 = 1.0;
/// A frame longer than this is a hitch (a pipeline compiling), and the steady clock starts over.
const HITCH_S: f32 = 0.05;
/// Past this long without steady frames the game goes on anyway, saying how slow its frames are,
/// so a slow build cannot hang a take.
const GIVE_UP_S: f32 = 30.0;

/// Set once the game is ready to be played and recorded.
#[derive(Resource, Default)]
pub struct GameReady(pub bool);

#[derive(Component)]
struct LoadingScreen;

/// The loading screen and `GAME_READY`.
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameReady>()
            .add_world_spawn(show)
            .add_game_systems(Update, watch);
    }
}

fn show(mut commands: Commands) {
    commands
        .spawn((
            LoadingScreen,
            Node {
                width: percent(100.0),
                height: percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.06, 0.07)),
            GlobalZIndex(1000),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("CRAB BREEZE\nloading"),
                TextFont::from_font_size(36.0),
                TextColor(Color::srgb(0.55, 0.95, 0.6)),
                TextLayout::justify(Justify::Center),
            ));
        });
}

fn watch(
    mut commands: Commands,
    mut ready: ResMut<GameReady>,
    mut steady_s: Local<f32>,
    mut log_in_s: Local<f32>,
    mut frames: Local<Vec<f32>>,
    time: Res<Time<Real>>,
    rig: Query<(), (With<Piloted>, With<MechRig>)>,
    access: Res<MechAccessNow>,
    sky: Res<crate::sky_light::SkyLight>,
    screen: Query<Entity, With<LoadingScreen>>,
) {
    let dt = time.delta_secs();
    if ready.0 {
        // After loading, the frame time every 5 s (a measurement instrument: p50, p95, p99, worst).
        frames.push(dt * 1000.0);
        *log_in_s -= dt;
        if *log_in_s <= 0.0 {
            *log_in_s = 5.0;
            frames.sort_by(f32::total_cmp);
            let p50 = frames[frames.len() / 2];
            let p95 = frames[(frames.len() * 95 / 100).min(frames.len() - 1)];
            let p99 = frames[(frames.len() * 99 / 100).min(frames.len() - 1)];
            info!(
                "frames: p50 {p50:.1} ms, p95 {p95:.1} ms, p99 {p99:.1} ms, worst {:.1} ms over {}",
                frames[frames.len() - 1],
                frames.len()
            );
            frames.clear();
        }
        return;
    }
    let loaded = !rig.is_empty() && access.0.is_some() && sky.ready;
    *steady_s = if loaded && dt < HITCH_S { *steady_s + dt } else { 0.0 };
    *log_in_s -= dt;
    if *log_in_s <= 0.0 {
        *log_in_s = 1.0;
        info!("loading: frame {:.1} ms, loaded {loaded}, steady {:.2} s", dt * 1000.0, *steady_s);
    }
    let gave_up = loaded && time.elapsed_secs() > GIVE_UP_S;
    if *steady_s >= STEADY_S || gave_up {
        ready.0 = true;
        if gave_up && *steady_s < STEADY_S {
            println!("GAME_READY slow: frames at {:.1} ms", dt * 1000.0);
        } else {
            println!("GAME_READY");
        }
        for e in &screen {
            commands.entity(e).despawn();
        }
    }
}
