//! The emitter gallery (`./run.sh --start fx`, openspec change `particle-depth`): every effect recipe the
//! game has, set out in a grid on open meadow in front of the pilot and replayed on a loop, so each one can
//! be walked round and looked at, near and far, against the ground and the sky (owner, 2026-10-25: "just set
//! uop a scene with different emitters", "and test all of them via a gif"). Streams (jet flame and trail,
//! burning, steam) run all the time; a jet trail's emitter sweeps to and fro so its smoke is laid along a
//! path; bursts replay every few seconds. A far row repeats the smoke-heavy blasts 450 m out.
//!
//! With `CB_FX_GALLERY_SHOTS=<dir>` it is also a capture instrument: the pilot's view pans slowly across the
//! grid and a still is written every 0.3 s of game time (`gallery-NNN.png`), for a GIF. Only then does it turn
//! the view; in play the view is the player's.

use crate::fx::{Fx, FxStream};
use crate::loading::GameReady;
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::world::Plan;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use cb_core::fx::RECIPES;
use cb_core::world::Ground;
use std::path::PathBuf;

/// Recipes a row holds.
const PER_ROW: usize = 8;
/// Metres between recipes along a row, and between rows.
const SPACING_M: f32 = 30.0;
/// Metres from the pilot to the first row.
const FIRST_ROW_M: f32 = 60.0;
/// Metres to the far row.
const FAR_ROW_M: f32 = 450.0;
/// The far row: the blasts whose smoke has to hold up at range.
const FAR: [&str; 4] = ["explosion_ground", "explosion_air", "collapse_dust", "mech_death"];

/// The gallery, turned on by `--start fx`.
pub struct FxGalleryPlugin;

impl Plugin for FxGalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gallery>()
            .add_systems(Update, (lay_out, replay, sweep, pan).chain());
    }
}

/// One spot in the gallery.
struct Spot {
    recipe: &'static str,
    at: Vec3,
    /// Seconds between replays of a burst; 0 for a stream.
    every_s: f32,
    /// Seconds to its next replay.
    next_s: f32,
}

#[derive(Resource, Default)]
struct Gallery {
    spots: Vec<Spot>,
    /// A jet trail's emitter, its row centre and the row's direction.
    sweeper: Option<(Entity, Vec3, Vec3)>,
    /// Where the pilot stood and looked when it was laid out.
    eye: Option<(Vec3, Vec3, Vec3)>,
    started_s: f32,
    shots: u32,
}

/// Lays the gallery out in front of the pilot once the game is ready.
fn lay_out(
    mut commands: Commands,
    mut gallery: ResMut<Gallery>,
    ready: Option<Res<GameReady>>,
    pilot: Res<PilotSim>,
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time>,
) {
    if gallery.eye.is_some() || !ready.is_some_and(|r| r.0) {
        return;
    }
    let feet = pilot.curr.feet;
    let yaw = pilot.curr.yaw_rad;
    let fwd = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let side = Vec3::new(-fwd.z, 0.0, fwd.x);
    let ground = |p: Vec3| Vec3::new(p.x, plan.height_at(p.x, p.z), p.z);
    let streams = &data.0.fx.stream_per_s;
    let mut spots = Vec::new();
    let mut sweeper = None;
    for (i, recipe) in RECIPES.iter().copied().enumerate() {
        let (row, col) = (i / PER_ROW, i % PER_ROW);
        let along = (col as f32 - (PER_ROW as f32 - 1.0) / 2.0) * SPACING_M;
        let at = ground(feet + fwd * (FIRST_ROW_M + row as f32 * SPACING_M) + side * along) + Vec3::Y * 1.0;
        info!("fx gallery: {recipe} at row {row}, column {col}");
        if streams.contains_key(recipe) {
            // Jets point down; everything else streams up.
            let jet = recipe.starts_with("jet_");
            let at = if jet { at + Vec3::Y * 12.0 } else { at };
            let e = commands
                .spawn((
                    Name::new(format!("fx gallery {recipe}")),
                    Transform::from_translation(at),
                    FxStream {
                        recipe,
                        dir: if jet { Vec3::NEG_Y } else { Vec3::Y },
                        on: true,
                        density: 1.0,
                    },
                ))
                .id();
            if recipe == "jet_trail" {
                sweeper = Some((e, at, side));
            }
            continue;
        }
        let big = recipe.starts_with("explosion") || recipe.contains("collapse") || recipe.contains("death");
        spots.push(Spot {
            recipe,
            at,
            every_s: if big { 8.0 } else { 3.0 },
            next_s: (i % 5) as f32 * 0.4,
        });
    }
    for (i, recipe) in FAR.iter().copied().enumerate() {
        if !RECIPES.contains(&recipe) {
            continue;
        }
        let along = (i as f32 - 1.5) * 60.0;
        spots.push(Spot {
            recipe,
            at: ground(feet + fwd * FAR_ROW_M + side * along) + Vec3::Y,
            every_s: 8.0,
            next_s: i as f32,
        });
    }
    gallery.spots = spots;
    gallery.sweeper = sweeper;
    gallery.eye = Some((feet, fwd, side));
    gallery.started_s = time.elapsed_secs();
}

/// Replays each burst when its time comes round.
fn replay(mut gallery: ResMut<Gallery>, time: Res<Time>, mut fx: MessageWriter<Fx>) {
    let dt = time.delta_secs();
    for s in &mut gallery.spots {
        s.next_s -= dt;
        if s.next_s <= 0.0 {
            s.next_s += s.every_s;
            fx.write(Fx::new(s.recipe, s.at));
        }
    }
}

/// Sweeps the jet trail's emitter 40 m each way along its row at 30 m/s.
fn sweep(gallery: Res<Gallery>, time: Res<Time>, mut q: Query<&mut Transform, With<FxStream>>) {
    let Some((e, centre, side)) = gallery.sweeper else { return };
    let t = time.elapsed_secs() - gallery.started_s;
    // A triangle wave: out and back over 160 m.
    let phase = (t * 30.0 / 160.0).fract();
    let x = if phase < 0.5 { phase * 4.0 - 1.0 } else { 3.0 - phase * 4.0 };
    if let Ok(mut tf) = q.get_mut(e) {
        tf.translation = centre + side * x * 40.0;
    }
}

/// The capture instrument (`CB_FX_GALLERY_SHOTS=<dir>`): after the effects have had 4 s to fill, the view pans
/// from the left of the grid to the right and back, down at the near rows then out to the far row, a still every
/// 0.3 s; then it exits.
fn pan(
    mut commands: Commands,
    mut gallery: ResMut<Gallery>,
    mut pilot: ResMut<PilotSim>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
    mut dir: Local<Option<Option<PathBuf>>>,
) {
    let dir = dir.get_or_insert_with(|| std::env::var("CB_FX_GALLERY_SHOTS").ok().map(PathBuf::from));
    let Some(dir) = dir.as_ref() else { return };
    let Some((feet, fwd, side)) = gallery.eye else { return };
    let t = time.elapsed_secs() - gallery.started_s - 4.0;
    if t < 0.0 {
        return;
    }
    const PAN_S: f32 = 24.0;
    let u = (t / PAN_S).min(1.0);
    // Left to right across the near grid, then right to left across the far row.
    let (across, ahead) = if u < 0.6 {
        let k = u / 0.6;
        ((k * 2.0 - 1.0) * 110.0, FIRST_ROW_M + 40.0)
    } else {
        let k = (u - 0.6) / 0.4;
        ((1.0 - k * 2.0) * 120.0, FAR_ROW_M)
    };
    let look = feet + fwd * ahead + side * across;
    let to = look - feet;
    pilot.curr.yaw_rad = (-to.x).atan2(-to.z);
    pilot.curr.pitch_rad = (-1.7f32).atan2(to.with_y(0.0).length()) + 0.08;
    pilot.prev = pilot.curr.clone();
    let due = (t / 0.3) as u32;
    if due >= gallery.shots {
        std::fs::create_dir_all(dir).ok();
        let path = dir.join(format!("gallery-{:03}.png", gallery.shots));
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
        gallery.shots += 1;
    }
    if u >= 1.0 {
        exit.write(AppExit::Success);
    }
}
