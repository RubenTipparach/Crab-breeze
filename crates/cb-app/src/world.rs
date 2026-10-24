//! Draws the generated world (`procgen-world` design section 4): terrain chunks, streets, kerbs,
//! highways and placeholder box buildings, with a sun and haze. The plan itself comes from
//! `cb_core::world::generate`; this module only presents it.

use crate::sim::Data;
use bevy::asset::RenderAssetUsages;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use cb_core::world::{WorldPlan, generate};
use std::sync::Arc;

/// The world seed.
#[derive(Resource, Clone, Copy)]
pub struct Seed(pub u64);

/// The generated world, shared with every system that walks on it.
#[derive(Resource, Clone, Deref)]
pub struct Plan(pub Arc<WorldPlan>);

/// Generates and draws the world.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let seed = app.world().resource::<Seed>().0;
        let started = std::time::Instant::now();
        let plan = generate(seed, &app.world().resource::<Data>().0.world);
        info!(
            "world seed {seed}: {} cities, {} buildings, {} highways, generated in {:.0} ms",
            plan.cities.len(),
            plan.buildings.len(),
            plan.highways.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        app.insert_resource(Plan(Arc::new(plan)))
            .insert_resource(ClearColor(SKY))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.75, 0.82, 1.0),
                brightness: 900.0,
                ..default()
            })
            .add_systems(Startup, (spawn_ground, spawn_roads, spawn_buildings, spawn_sun));
    }
}

/// The sky colour, also the haze colour at the horizon.
pub const SKY: Color = Color::srgb(0.62, 0.74, 0.88);

/// Terrain chunk size in cells.
const CHUNK_CELLS: usize = 64;

/// Accumulates triangles for one merged mesh.
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    /// A flat quad, corners counter-clockwise around its outward normal `(c1 - c0) x (c3 - c0)`.
    fn quad(&mut self, corners: [Vec3; 4], color: [f32; 4]) {
        let base = self.positions.len() as u32;
        let n = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize_or(Vec3::Y);
        for c in corners {
            self.positions.push(c.into());
            self.normals.push(n.into());
            self.colors.push(color);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// An axis-aligned box from `min` to `max` (all six faces).
    fn cuboid(&mut self, min: Vec3, max: Vec3, color: [f32; 4]) {
        let [x0, y0, z0] = min.to_array();
        let [x1, y1, z1] = max.to_array();
        let v = |x, y, z| Vec3::new(x, y, z);
        // Each face listed so its outward normal is (b - a) x (d - a).
        self.quad([v(x0, y1, z0), v(x0, y1, z1), v(x1, y1, z1), v(x1, y1, z0)], color);
        self.quad([v(x0, y0, z1), v(x1, y0, z1), v(x1, y1, z1), v(x0, y1, z1)], color);
        self.quad([v(x1, y0, z0), v(x0, y0, z0), v(x0, y1, z0), v(x1, y1, z0)], color);
        self.quad([v(x1, y0, z1), v(x1, y0, z0), v(x1, y1, z0), v(x1, y1, z1)], color);
        self.quad([v(x0, y0, z0), v(x0, y0, z1), v(x0, y1, z1), v(x0, y1, z0)], color);
    }

    fn build(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices))
    }
}

fn vertex_material(materials: &mut Assets<StandardMaterial>, roughness: f32) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: roughness,
        ..default()
    })
}

fn spawn_ground(
    mut commands: Commands,
    world: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = &world.terrain;
    let chunks = (t.n - 1).div_ceil(CHUNK_CELLS);
    let material = vertex_material(&mut materials, 0.95);
    for cz in 0..chunks {
        for cx in 0..chunks {
            let chunk = t.chunk_mesh(cx, cz, CHUNK_CELLS);
            let colors: Vec<[f32; 4]> = chunk
                .positions
                .iter()
                .zip(&chunk.normals)
                .map(|(p, n)| {
                    // Grass on the level, earth on slopes, a little variation by height.
                    let slope = (1.0 - n.y).clamp(0.0, 1.0) * 6.0;
                    let k = (p.y * 0.05).sin() * 0.04;
                    let grass = Vec3::new(0.30 + k, 0.42 + k, 0.20);
                    let earth = Vec3::new(0.42, 0.36, 0.27);
                    grass.lerp(earth, slope.min(1.0)).extend(1.0).to_array()
                })
                .collect();
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    chunk.positions.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    chunk.normals.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
                .with_inserted_indices(Indices::U32(chunk.indices));
            commands.spawn((
                Name::new(format!("terrain {cx},{cz}")),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
            ));
        }
    }
}

fn spawn_roads(
    mut commands: Commands,
    world: Res<Plan>,
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cfg = &data.0.world;
    let asphalt = [0.16, 0.16, 0.17, 1.0];
    let arterial = [0.13, 0.13, 0.14, 1.0];
    let pavement = [0.55, 0.54, 0.51, 1.0];
    let park = [0.26, 0.40, 0.19, 1.0];
    let mut roads = MeshBuilder::default();
    let mut kerbs = MeshBuilder::default();
    for city in &world.cities {
        let y = city.ground_m + cfg.road_lift_m;
        for s in &city.streets {
            let (a, b) = (s.rect.min, s.rect.max);
            let colour = if s.kind == cb_core::world::StreetKind::Arterial {
                arterial
            } else {
                asphalt
            };
            roads.quad(
                [
                    Vec3::new(a.x, y, a.y),
                    Vec3::new(a.x, y, b.y),
                    Vec3::new(b.x, y, b.y),
                    Vec3::new(b.x, y, a.y),
                ],
                colour,
            );
        }
        for block in &city.blocks {
            let (a, b) = (block.rect.min, block.rect.max);
            kerbs.cuboid(
                Vec3::new(a.x, city.ground_m - 1.0, a.y),
                Vec3::new(b.x, city.ground_m + cfg.kerb_height_m, b.y),
                if block.park { park } else { pavement },
            );
        }
    }
    for h in &world.highways {
        let along = h.ends[1] - h.ends[0];
        let across = along.normalize().perp() * h.width_m * 0.5;
        let steps = (h.length_m() / 4.0).ceil() as usize;
        for i in 0..steps {
            let (t0, t1) = (i as f32 / steps as f32, (i + 1) as f32 / steps as f32);
            let p = |t: f32, side: f32| {
                let xz = h.ends[0] + along * t + across * side;
                Vec3::new(xz.x, h.grade_at(t) + cfg.road_lift_m, xz.y)
            };
            roads.quad([p(t0, 1.0), p(t1, 1.0), p(t1, -1.0), p(t0, -1.0)], asphalt);
        }
    }
    let road_mat = vertex_material(&mut materials, 0.9);
    commands.spawn((Name::new("roads"), Mesh3d(meshes.add(roads.build())), MeshMaterial3d(road_mat)));
    let kerb_mat = vertex_material(&mut materials, 0.85);
    commands.spawn((Name::new("blocks"), Mesh3d(meshes.add(kerbs.build())), MeshMaterial3d(kerb_mat)));
}

fn spawn_buildings(
    mut commands: Commands,
    world: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let tones = [
        (0.62, 0.60, 0.56),
        (0.48, 0.50, 0.53),
        (0.70, 0.66, 0.58),
        (0.36, 0.40, 0.46),
        (0.56, 0.47, 0.40),
        (0.78, 0.77, 0.74),
    ]
    .map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: 0.75,
            ..default()
        })
    });
    for b in &world.buildings {
        let ground = world.cities[b.city as usize].ground_m;
        let size = b.footprint.size();
        let centre = b.footprint.centre();
        // From a metre below the ground to the roof, so no sliver of sky shows under a wall.
        let bottom = ground - 1.0;
        let height = b.height_m + 1.0;
        commands.spawn((
            Name::new(format!("building {}", b.id)),
            Mesh3d(cube.clone()),
            MeshMaterial3d(tones[b.id as usize % tones.len()].clone()),
            Transform::from_xyz(centre.x, bottom + height * 0.5, centre.y).with_scale(Vec3::new(size.x, height, size.y)),
        ));
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Name::new("sun"),
        DirectionalLight {
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.8, -0.75, 0.0)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            first_cascade_far_bound: 40.0,
            maximum_distance: 900.0,
            ..default()
        }
        .build(),
    ));
}
