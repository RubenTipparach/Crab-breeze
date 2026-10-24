//! Physics debris (openspec change `debris-physics`): Avian 3D for what falls and tumbles. The
//! ground gets triangle-mesh colliders built from the very meshes drawn; falling building chunks
//! and shot-off parts are dynamic bodies. Debris is presentation: the simulation (cells, trees,
//! ground, mechs) is `cb-core`'s and deterministic; Avian only decides where pieces come to rest.

use crate::sim::PresentSet;
use crate::world::{Live, Plan};
use avian3d::prelude::{AngularVelocity, Collider, LinearVelocity, PhysicsPlugins, RigidBody};
use bevy::prelude::*;
use std::collections::VecDeque;

/// Most moving debris bodies; past it the oldest settle where they are (become static).
const MAX_DYNAMIC: usize = 600;
/// Most debris kept at all; past it the oldest are removed.
const MAX_KEPT: usize = 2500;

/// Debris in spawn order.
#[derive(Resource, Default)]
struct DebrisQueue {
    dynamic: VecDeque<Entity>,
    resting: VecDeque<Entity>,
}

/// Shared chunk drawing.
#[derive(Resource)]
struct ChunkAssets {
    cube: Handle<Mesh>,
    tones: Vec<Handle<StandardMaterial>>,
}

/// Avian and the debris it moves.
pub struct DebrisPhysicsPlugin;

impl Plugin for DebrisPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .init_resource::<DebrisQueue>()
            .add_systems(Startup, setup)
            .add_systems(Update, spawn_chunks.in_set(PresentSet::Pose));
    }
}

/// A static triangle-mesh collider for a ground mesh.
pub fn ground_collider(positions: &[Vec3], indices: &[u32]) -> Option<(RigidBody, Collider)> {
    if indices.len() < 3 {
        return None;
    }
    let tris = indices.chunks(3).map(|t| [t[0], t[1], t[2]]).collect();
    Collider::try_trimesh(positions.to_vec(), tris).ok().map(|c| (RigidBody::Static, c))
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let tones = crate::world::BUILDING_TONES
        .iter()
        .map(|(r, g, b)| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(r * 0.85, g * 0.85, b * 0.85),
                perceptual_roughness: 0.85,
                ..default()
            })
        })
        .collect();
    commands.insert_resource(ChunkAssets {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        tones,
    });
}

fn spawn_chunks(
    mut commands: Commands,
    mut live: ResMut<Live>,
    assets: Res<ChunkAssets>,
    mut queue: ResMut<DebrisQueue>,
    plan: Res<Plan>,
    mut n: Local<u32>,
) {
    let _t = crate::perf_hud::slow("spawn_chunks");
    let chunks = std::mem::take(&mut live.new_chunks);
    for c in chunks {
        *n = n.wrapping_mul(1664525).wrapping_add(1013904223);
        let r = |k: u32| ((*n >> k) & 255) as f32 / 255.0 - 0.5;
        // Leave a little gap so neighbouring chunks do not start interpenetrating.
        let size = (c.size - Vec3::splat(0.15)).max(Vec3::splat(0.5));
        let mass_scale = (size.x * size.y * size.z).cbrt();
        let tone = assets.tones[c.building as usize % assets.tones.len()].clone();
        let _ = plan.seed;
        let e = commands
            .spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(tone),
                Transform::from_translation(c.centre).with_scale(size),
                RigidBody::Dynamic,
                Collider::cuboid(1.0, 1.0, 1.0),
                LinearVelocity(Vec3::new(r(0), -0.5, r(8)) * 2.0),
                AngularVelocity(Vec3::new(r(16), r(4), r(12)) * (1.5 / mass_scale.max(1.0))),
            ))
            .id();
        queue.dynamic.push_back(e);
    }
    while queue.dynamic.len() > MAX_DYNAMIC {
        if let Some(e) = queue.dynamic.pop_front() {
            commands.entity(e).insert(RigidBody::Static);
            queue.resting.push_back(e);
        }
    }
    while queue.resting.len() > MAX_KEPT {
        if let Some(e) = queue.resting.pop_front() {
            commands.entity(e).despawn();
        }
    }
}
