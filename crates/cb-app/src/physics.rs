//! Physics debris (openspec change `debris-physics`): Avian 3D for what falls and tumbles. The
//! ground gets triangle-mesh colliders built from the very meshes drawn; falling building chunks
//! and shot-off parts are dynamic bodies. Debris is presentation: the simulation (cells, trees,
//! ground, mechs) is `cb-core`'s and deterministic; Avian only decides where pieces come to rest.
//!
//! Only near the pilot (`world.ron` `debris`): a chunk breaking off farther than the physics radius
//! is a dust puff, not a body, and a moving body left far behind is made static where it is. The
//! cell simulation breaks and drops buildings at every distance.

use crate::fx::Fx;
use crate::pilot::PilotCamera;
use crate::sim::PresentSet;
use crate::state::GameApp;
use crate::world::Live;
use avian3d::prelude::{AngularVelocity, Collider, LinearVelocity, PhysicsPlugins, RigidBody};
use bevy::prelude::*;
use std::collections::VecDeque;

/// Seconds between checks for moving bodies left beyond the physics radius.
const FREEZE_CHECK_S: f32 = 0.5;

/// Debris in spawn order.
#[derive(Resource, Default)]
struct DebrisQueue {
    dynamic: VecDeque<Entity>,
    resting: VecDeque<Entity>,
}

/// Shared chunk drawing: one cube, and the concrete surface tinted by each building tone (made
/// from the buildings' concrete material once it exists).
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
            .add_world_spawn(setup)
            .add_game_systems(Update, (spawn_chunks, freeze_far_debris).in_set(PresentSet::Pose));
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

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(ChunkAssets {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        tones: Vec::new(),
    });
}

/// The chunk materials: the buildings' concrete surface, darkened to each building tone.
fn chunk_tones(looks: &crate::buildings::BuildingLooks, materials: &mut Assets<StandardMaterial>) -> Vec<Handle<StandardMaterial>> {
    let concrete = materials.get(&looks.concrete).cloned().unwrap_or_default();
    crate::world::BUILDING_TONES
        .iter()
        .map(|(r, g, b)| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(r * 0.85, g * 0.85, b * 0.85),
                ..concrete.clone()
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn spawn_chunks(
    mut commands: Commands,
    mut live: ResMut<Live>,
    mut assets: ResMut<ChunkAssets>,
    mut queue: ResMut<DebrisQueue>,
    data: Res<crate::sim::Data>,
    looks: Option<Res<crate::buildings::BuildingLooks>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
    mut fx: MessageWriter<Fx>,
    mut n: Local<u32>,
) {
    let _t = crate::perf_hud::slow("spawn_chunks");
    if live.new_chunks.is_empty() {
        return;
    }
    if assets.tones.is_empty() {
        let Some(looks) = looks else { return };
        assets.tones = chunk_tones(&looks, &mut materials);
    }
    let cfg = &data.0.world.debris;
    let eye = camera.single().ok().map(GlobalTransform::translation);
    let chunks = std::mem::take(&mut live.new_chunks);
    let (offered, bodies_before) = (chunks.len(), queue.dynamic.len());
    // One puff per far building per frame, in building order.
    let mut puffed: Vec<u32> = Vec::new();
    for c in chunks {
        if !cfg.is_physical(eye.map_or(f32::INFINITY, |e| e.distance(c.centre))) {
            if !puffed.contains(&c.building) {
                puffed.push(c.building);
                fx.write(Fx::new("member_fail", c.centre));
            }
            continue;
        }
        *n = n.wrapping_mul(1664525).wrapping_add(1013904223);
        let r = |k: u32| ((*n >> k) & 255) as f32 / 255.0 - 0.5;
        // Leave a little gap so neighbouring chunks do not start interpenetrating.
        let size = (c.size - Vec3::splat(0.15)).max(Vec3::splat(0.5));
        let mass_scale = (size.x * size.y * size.z).cbrt();
        let tone = assets.tones[c.building as usize % assets.tones.len()].clone();
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
    crate::probe_event!(
        "debris: {offered} chunks, {} new bodies, {} far puffs; {} moving, {} resting",
        queue.dynamic.len() - bodies_before,
        puffed.len(),
        queue.dynamic.len(),
        queue.resting.len()
    );
    while queue.dynamic.len() > cfg.max_dynamic as usize {
        if let Some(e) = queue.dynamic.pop_front() {
            commands.entity(e).insert(RigidBody::Static);
            queue.resting.push_back(e);
        }
    }
    while queue.resting.len() > cfg.max_kept as usize {
        if let Some(e) = queue.resting.pop_front() {
            commands.entity(e).despawn();
        }
    }
}

/// Every `FREEZE_CHECK_S`, makes static where they are the moving bodies the pilot has left beyond
/// the physics radius (times the freeze factor, so one on the edge does not flicker).
fn freeze_far_debris(
    mut commands: Commands,
    time: Res<Time>,
    mut since: Local<f32>,
    mut queue: ResMut<DebrisQueue>,
    data: Res<crate::sim::Data>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
    bodies: Query<&Transform>,
) {
    *since += time.delta_secs();
    if *since < FREEZE_CHECK_S || queue.dynamic.is_empty() {
        return;
    }
    *since = 0.0;
    let Ok(eye) = camera.single().map(GlobalTransform::translation) else {
        return;
    };
    let cfg = &data.0.world.debris;
    let dynamic = std::mem::take(&mut queue.dynamic);
    for e in dynamic {
        let far = bodies.get(e).is_ok_and(|t| cfg.freezes(t.translation.distance(eye)));
        if far {
            commands.entity(e).insert(RigidBody::Static);
            queue.resting.push_back(e);
        } else {
            queue.dynamic.push_back(e);
        }
    }
}
