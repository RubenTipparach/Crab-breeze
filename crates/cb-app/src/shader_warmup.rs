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
        app.add_systems(Update, (show_zoo, clear_zoo));
    }
}

fn show_zoo(
    mut commands: Commands,
    ready: Res<crate::loading::GameReady>,
    eye: Query<Entity, With<PilotCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut leaves: crate::buildings::LeafLooks,
    mut done: Local<bool>,
) {
    if *done || ready.0 {
        return;
    }
    let Ok(eye) = eye.single() else { return };
    // A building piece needs the buildings' looks loaded: wait for them, then lay the whole zoo out at once.
    let Some(skin) = leaves.spawn(
        &mut commands,
        &mut meshes,
        Mat::Concrete,
        [0.8, 0.8, 0.8],
        Vec3::splat(1.0),
        "zoo skin".into(),
    ) else {
        return;
    };
    *done = true;
    let mut pieces = vec![skin.0];
    if let Some(glass) = leaves.spawn(
        &mut commands,
        &mut meshes,
        Mat::Glass,
        [1.0, 1.0, 1.0],
        Vec3::splat(1.0),
        "zoo glass".into(),
    ) {
        pieces.push(glass.0);
    }
    for alpha in [AlphaMode::Opaque, AlphaMode::Blend, AlphaMode::Premultiplied, AlphaMode::Mask(0.5)] {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.5, 0.5, 0.9),
            alpha_mode: alpha,
            ..default()
        });
        for (uv, tangents, colours) in LAYOUTS {
            let e = commands
                .spawn((
                    Name::new("zoo box"),
                    Mesh3d(meshes.add(zoo_box(uv, tangents, colours))),
                    MeshMaterial3d(material.clone()),
                    Transform::default(),
                ))
                .id();
            pieces.push(e);
        }
    }
    // Two metres ahead of the eye, a few centimetres across, side by side: in every camera's view and under the
    // loading panel.
    let n = pieces.len() as f32;
    for (i, e) in pieces.into_iter().enumerate() {
        let x = (i as f32 - n / 2.0) * 0.06;
        commands
            .entity(e)
            .insert((Zoo, ChildOf(eye), Transform::from_xyz(x, 0.0, -2.0).with_scale(Vec3::splat(0.05))));
    }
    info!("shader warm-up: {n} pieces behind the loading screen");
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
