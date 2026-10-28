//! Shaders compiled behind the loading screen (openspec change `city-frame` section 5): the first frame a new kind of
//! mesh is seen, every camera that sees it (the eye, the sun's cascades, the cockpit cube's faces) builds a pipeline
//! for it, and on the 3070 those frames ran 100 to 600 ms when the first building fell, the first debris flew or a
//! new prop came into view (107 pipelines compiled after `GAME_READY` in one capital run, `perf_gate`'s log). So while
//! the loading screen is up a small zoo stands just in front of the eye, under the panel: a box in every vertex layout
//! the world's meshes use (the layouts the late pipelines named) in each blend the world uses, a building piece in a
//! skin and in glass (the frame and window shaders a hit building swaps in), so every camera builds them there. The
//! loading screen waits for steady frames, so the compiles lengthen the load rather than a played frame. The zoo goes
//! the frame the game is ready.

use crate::pilot::PilotCamera;
use bevy::prelude::*;
use cb_core::world::dressing::Mat;

/// The zoo's pieces, removed when the game is ready.
#[derive(Component)]
struct Zoo;

/// The vertex layouts of the world's meshes: (UVs, tangents, colours) beside positions and normals.
const LAYOUTS: [(bool, bool, bool); 6] = [
    (false, false, false),
    (true, false, false),
    (true, true, false),
    (false, false, true),
    (true, false, true),
    (true, true, true),
];

/// The zoo behind the loading screen.
pub struct ShaderWarmupPlugin;

impl Plugin for ShaderWarmupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (show_boxes, show_pieces, clear_zoo));
    }
}

/// The boxes: every vertex layout in every blend, in a row just under the eye line.
fn show_boxes(
    mut commands: Commands,
    ready: Res<crate::loading::GameReady>,
    eye: Query<Entity, With<PilotCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut done: Local<bool>,
) {
    if *done || ready.0 {
        return;
    }
    let Ok(eye) = eye.single() else { return };
    *done = true;
    let mut pieces = Vec::new();
    for alpha in [AlphaMode::Opaque, AlphaMode::Blend, AlphaMode::Premultiplied, AlphaMode::Mask(0.5)] {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.5, 0.5, 0.9),
            alpha_mode: alpha,
            ..default()
        });
        for (uv, tangents, colours) in LAYOUTS {
            pieces.push(
                commands
                    .spawn((
                        Name::new("zoo box"),
                        Mesh3d(meshes.add(zoo_box(uv, tangents, colours))),
                        MeshMaterial3d(material.clone()),
                    ))
                    .id(),
            );
        }
    }
    info!("shader warm-up: {} boxes behind the loading screen", pieces.len());
    place(&mut commands, eye, pieces, -0.04);
}

/// The building pieces, in a skin and in glass, once the buildings' looks are loaded: a system of their own, since the
/// buildings' material helper reads the same material store the boxes write.
fn show_pieces(
    mut commands: Commands,
    ready: Res<crate::loading::GameReady>,
    eye: Query<Entity, With<PilotCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut leaves: crate::buildings::LeafLooks,
    mut done: Local<bool>,
) {
    if *done || ready.0 {
        return;
    }
    let Ok(eye) = eye.single() else { return };
    let mut pieces = Vec::new();
    for (mat, name) in [(Mat::Concrete, "zoo skin"), (Mat::Glass, "zoo glass")] {
        match leaves.spawn(&mut commands, &mut meshes, mat, [0.8, 0.8, 0.8], Vec3::splat(1.0), name.into()) {
            Some((e, _)) => pieces.push(e),
            // The looks are not in yet: try again next frame.
            None => return,
        }
    }
    *done = true;
    place(&mut commands, eye, pieces, 0.04);
}

/// Two metres ahead of the eye, a few centimetres across, side by side in a row at `y` m: in every camera's view and
/// under the loading panel.
fn place(commands: &mut Commands, eye: Entity, pieces: Vec<Entity>, y: f32) {
    let n = pieces.len() as f32;
    for (i, e) in pieces.into_iter().enumerate() {
        let x = (i as f32 - n / 2.0) * 0.06;
        commands
            .entity(e)
            .insert((Zoo, ChildOf(eye), Transform::from_xyz(x, y, -2.0).with_scale(Vec3::splat(0.05))));
    }
}

fn clear_zoo(mut commands: Commands, ready: Res<crate::loading::GameReady>, zoo: Query<Entity, With<Zoo>>) {
    if ready.0 {
        for e in &zoo {
            commands.entity(e).despawn();
        }
    }
}

/// A unit box with positions and normals, and UVs, tangents and vertex colours as asked.
fn zoo_box(uv: bool, tangents: bool, colours: bool) -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(1.0, 1.0, 1.0));
    if !uv {
        mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0);
    }
    if tangents {
        let _ = mesh.generate_tangents();
    }
    if colours {
        let n = mesh.count_vertices();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32, 1.0, 1.0, 1.0]; n]);
    }
    mesh
}
