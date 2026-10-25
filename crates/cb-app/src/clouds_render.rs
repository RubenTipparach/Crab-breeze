//! Cheap, correctly lit clouds (openspec change `cloud-puffs`), replacing the sprite clouds the
//! owner turned off for their overdraw ("something is quite heavy on my gpu disable clouds for
//! now"; then "good looking clouds with correct lighting and dont cost much to render").
//!
//! Every puff of `cb_core::world::clouds` is a lumpy blob (an icosphere pushed out by
//! noise, its base flattened at the cloud base), and every puff of a 2 km cloud tile is merged into
//! one mesh: about 30 tiles in the sky within the draw radius, 30 entities, one material, so Bevy
//! batches them. The wind moves each tile by its transform. The light is the scene's own: the sun's
//! direction and radiance and the ambient light, through the water's light helper, so a cloud sits
//! in the same light as the ground. The shader (`assets/shaders/cloud.wesl`) wraps the sun round
//! the blob, darkens the base, lights a silver lining toward the sun, softens the silhouette and
//! fades a blob out close to the eye (inside, the cloud fog takes over); Bevy's fog hazes far ones. One
//! alpha-blended layer per blob, not a stack of sprites, is what keeps the overdraw low.

use crate::pilot::PilotCamera;
use crate::sim::{Data, PresentSet};
use crate::water_render::water_light;
use crate::world::Plan;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use cb_core::world::clouds::{CLOUD_TILE_M, Puff};
use std::collections::HashMap;

const SHADER: &str = "shaders/cloud.wesl";

/// The cloud material's uniform; `CloudLight` in `cloud.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct CloudUniform {
    /// xyz toward the sun; w how far the sun wraps round a blob (0 hard, 1 soft).
    pub sun_dir: Vec4,
    /// rgb the sun's light times exposure; w the silver lining's strength.
    pub sun: Vec4,
    /// rgb the ambient light times exposure; w the albedo.
    pub ambient: Vec4,
    /// x the cloud base, y the cloud top (metres), z how dark the base is (0..1), w unused.
    pub layer: Vec4,
    /// x fade-in distance from the eye (m), y edge softness, z clear-air visibility (m), w unused.
    pub fade: Vec4,
}

/// The one cloud material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CloudMaterial {
    /// The light and look, rewritten each frame.
    #[uniform(0)]
    pub light: CloudUniform,
}

impl Material for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    // Transparent: no depth prepass and no shadows (cloud shadows are a later, cheaper texture).
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

/// Clouds in the sky.
pub struct CloudsRenderPlugin;

impl Plugin for CloudsRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CloudMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(Update, (show_tiles, drift_tiles, refresh_light).chain().in_set(PresentSet::Pose));
    }
}

/// The drawn cloud tiles.
#[derive(Resource)]
struct CloudTiles {
    material: Handle<CloudMaterial>,
    /// Every tile with puffs and its centre (x, z) at time zero.
    all: Vec<((i32, i32), Vec2)>,
    drawn: HashMap<(i32, i32), Entity>,
}

/// A drawn tile (its mesh is in world space at time zero; the transform carries the wind).
#[derive(Component)]
struct CloudTile;

/// Icosphere subdivisions of a puff: 4 is 252 vertices and about 500 triangles, round enough for
/// the lumps to read and cheap enough for a few hundred puffs in view.
const PUFF_DETAIL: u32 = 4;

/// Most tiles put in the sky in one frame (a tile's mesh is a few thousand triangles).
const TILES_PER_FRAME: usize = 2;

fn setup(mut commands: Commands, plan: Res<Plan>, mut materials: ResMut<Assets<CloudMaterial>>) {
    commands.insert_resource(CloudTiles {
        material: materials.add(CloudMaterial {
            light: CloudUniform::default(),
        }),
        all: plan.clouds.tiles(),
        drawn: HashMap::new(),
    });
}

/// A small, stable hash of a point on the unit sphere and a puff, 0..1 (the lumps).
fn lump(dir: Vec3, seed: u32) -> f32 {
    let mut v = 0.0;
    let mut amp = 0.6;
    let mut f = 2.3;
    for o in 0..3u32 {
        let p = dir * f + Vec3::splat((seed.wrapping_mul(97).wrapping_add(o * 31) % 1000) as f32 * 0.37);
        v += amp * ((p.x * 1.7 + p.y * 2.3).sin() * (p.y * 1.3 - p.z * 2.9).cos() * (p.z * 2.1 + p.x * 0.7).sin() * 0.5 + 0.5);
        amp *= 0.5;
        f *= 2.1;
    }
    v / 1.05
}

/// One tile's puffs as one mesh, in world space at time zero: each an icosphere scaled to its radii,
/// pushed out by `lump`, and flattened at the base so cumulus sit on a level floor.
fn tile_mesh(puffs: &[(u32, Puff)], base_m: f32) -> Mesh {
    let unit = Sphere::new(1.0).mesh().ico(PUFF_DETAIL).expect("an icosphere");
    let (Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)), Some(Indices::U32(idx))) =
        (unit.attribute(Mesh::ATTRIBUTE_POSITION).cloned(), unit.indices().cloned())
    else {
        return Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    };
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(puffs.len() * pos.len());
    let mut indices: Vec<u32> = Vec::with_capacity(puffs.len() * idx.len());
    for (i, p) in puffs {
        let first = positions.len() as u32;
        for v in &pos {
            let d = Vec3::from_array(*v).normalize_or(Vec3::Y);
            let push = 0.78 + 0.45 * lump(d, *i);
            let mut w = p.centre + d * p.radii * push;
            w.y = w.y.max(base_m.max(p.centre.y - p.radii.y * 0.75));
            positions.push(w.to_array());
        }
        indices.extend(idx.iter().map(|k| first + k));
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}

/// Puts the tiles within the draw radius of the eye in the sky (as the wind has carried them), a
/// couple a frame nearest first, and takes the ones beyond it down.
fn show_tiles(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    tiles: Option<ResMut<CloudTiles>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(mut tiles) = tiles else { return };
    let eye = camera.translation();
    let drift = plan.clouds.wind_m_s * time.elapsed_secs();
    let radius = data.0.world.clouds.draw_radius_m;
    let half_diag = CLOUD_TILE_M * std::f32::consts::FRAC_1_SQRT_2;
    let dist = |c: Vec2| ((c + drift).distance(Vec2::new(eye.x, eye.z)) - half_diag).max(0.0);
    let t = &mut *tiles;
    let mut gone: Vec<(i32, i32)> = t
        .drawn
        .keys()
        .copied()
        .filter(|k| t.all.iter().find(|a| a.0 == *k).is_none_or(|a| dist(a.1) > radius + half_diag))
        .collect();
    gone.sort_unstable();
    for key in gone {
        if let Some(e) = t.drawn.remove(&key) {
            commands.entity(e).despawn();
        }
    }
    let mut wanted: Vec<(f32, (i32, i32))> = t
        .all
        .iter()
        .filter(|a| !t.drawn.contains_key(&a.0))
        .map(|a| (dist(a.1), a.0))
        .filter(|w| w.0 <= radius)
        .collect();
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let base = data.0.world.clouds.base_m;
    for (_, key) in wanted.into_iter().take(TILES_PER_FRAME) {
        let puffs: Vec<(u32, Puff)> = plan
            .clouds
            .puffs_in(key)
            .into_iter()
            .map(|i| (i, plan.clouds.puffs[i as usize]))
            .collect();
        let e = commands
            .spawn((
                Name::new(format!("cloud tile {key:?}")),
                CloudTile,
                Mesh3d(meshes.add(tile_mesh(&puffs, base))),
                MeshMaterial3d(t.material.clone()),
                Transform::from_translation(Vec3::new(drift.x, 0.0, drift.y)),
                NotShadowCaster,
            ))
            .id();
        t.drawn.insert(key, e);
    }
}

/// The wind carries every drawn tile.
fn drift_tiles(plan: Res<Plan>, time: Res<Time>, mut q: Query<&mut Transform, With<CloudTile>>) {
    let d = plan.clouds.wind_m_s * time.elapsed_secs();
    for mut t in &mut q {
        t.translation = Vec3::new(d.x, 0.0, d.y);
    }
}

/// The scene's light into the cloud material each frame.
fn refresh_light(
    data: Res<Data>,
    ambient: Res<GlobalAmbientLight>,
    sun: Query<(&DirectionalLight, &GlobalTransform)>,
    camera: Query<Option<&Exposure>, With<PilotCamera>>,
    tiles: Option<Res<CloudTiles>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
) {
    let Some(tiles) = tiles else { return };
    let exposure = camera.iter().next().flatten().copied().unwrap_or_default().exposure();
    let light = water_light(sun.iter().next(), &ambient, exposure);
    let a = LinearRgba::from(ambient.color);
    let amb = Vec3::new(a.red, a.green, a.blue) * ambient.brightness * exposure;
    let c = &data.0.world.clouds;
    let Some(mut m) = materials.get_mut(&tiles.material) else { return };
    m.light = CloudUniform {
        sun_dir: light.sun_dir.extend(c.look_wrap),
        sun: light.sun.extend(c.look_silver),
        ambient: amb.extend(c.look_albedo),
        layer: Vec4::new(c.base_m, c.top_m, c.look_base_shade, 0.0),
        fade: Vec4::new(c.look_near_fade_m, c.look_edge_softness, data.0.world.view.clear_visibility_m, 0.0),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn puff() -> (u32, Puff) {
        (
            7,
            Puff {
                centre: Vec3::new(100.0, 700.0, -50.0),
                radii: Vec3::new(150.0, 90.0, 120.0),
            },
        )
    }

    #[test]
    fn a_tile_mesh_holds_every_puff_and_sits_on_the_cloud_base() {
        let p = puff();
        let m = tile_mesh(
            &[
                p,
                (
                    8,
                    Puff {
                        centre: p.1.centre + Vec3::X * 300.0,
                        ..p.1
                    },
                ),
            ],
            610.0,
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("positions")
        };
        let one = Sphere::new(1.0).mesh().ico(PUFF_DETAIL).unwrap().count_vertices();
        assert_eq!(pos.len(), 2 * one, "two puffs, one icosphere each");
        assert!(pos.iter().all(|v| v[1] >= 610.0 - 1e-3), "nothing hangs below the cloud base");
        assert!(m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(), "lit with smooth normals");
    }

    #[test]
    fn lumps_are_stable_and_bounded() {
        for i in 0..50u32 {
            let d = Vec3::new((i as f32).sin(), (i as f32 * 0.7).cos(), 0.3).normalize();
            let a = lump(d, i);
            assert_eq!(a, lump(d, i), "the same puff is the same shape every time");
            assert!((0.0..=1.0).contains(&a), "{a}");
        }
    }
}
