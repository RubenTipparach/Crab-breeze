//! Draws the generated world (`procgen-world` design section 4): terrain chunks, streets, kerbs,
//! highways and placeholder box buildings, with a sun and haze. The plan itself comes from
//! `cb_core::world::generate`; this module only presents it.

use crate::pilot::PilotCamera;
use crate::sim::Data;
use crate::sim::{PresentSet, SimSet};
use crate::water_render::WaterMaterial;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat};
use cb_core::water_look::WaterLook;
use cb_core::world::{Deformation, Forest, TreeState, WorldPlan, generate};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// The world seed.
#[derive(Resource, Clone, Copy)]
pub struct Seed(pub u64);

/// The generated world, shared with every system that walks on it.
#[derive(Resource, Clone, Deref)]
pub struct Plan(pub Arc<WorldPlan>);

/// The world as it is now: trees knocked down and ground dug since it was generated. The fixed
/// step changes it; drawing follows it.
#[derive(Resource)]
pub struct Live {
    /// Every tree and its state.
    pub forest: Forest,
    /// Craters.
    pub dug: Deformation,
    /// Crater tiles dug since they were last drawn.
    pub dirty_tiles: Vec<(i32, i32)>,
    /// Trees that started falling or landed since they were last drawn.
    pub moved_trees: Vec<u32>,
    /// Every building's cells and, once hit, its structural frame.
    pub structures: cb_core::world::Structures,
    /// Buildings damaged since they were last drawn.
    pub dirty_buildings: Vec<u32>,
    /// Pieces of buildings that fell since the physics last took them.
    pub new_chunks: Vec<cb_core::world::Chunk>,
}

/// Generates and draws the world.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let seed = app.world().resource::<Seed>().0;
        let cfg = app.world().resource::<Data>().0.world.clone();
        let started = std::time::Instant::now();
        let plan = generate(seed, &cfg);
        info!(
            "world seed {seed}: {} m island world, {} cities, {} buildings, {} highways, generated in {:.0} ms",
            cfg.world_size_m,
            plan.cities.len(),
            plan.buildings.len(),
            plan.highways.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        // Trees grow near the bodies (island-world): round the start before the first step.
        let mut forest = plan.forest.clone();
        let grown = std::time::Instant::now();
        forest.grow_around(&plan, plan.start.mech_xz, cfg.trees.keep_radius_m);
        forest.grow_around(&plan, plan.start.pilot_xz, cfg.trees.keep_radius_m);
        let live = Live {
            forest,
            dug: Deformation::new(cfg.world_size_m),
            dirty_tiles: Vec::new(),
            moved_trees: Vec::new(),
            structures: cb_core::world::Structures::with_config(&plan, cfg.structure.clone(), cfg.craters.radius_m_per_cbrt_mj),
            dirty_buildings: Vec::new(),
            new_chunks: Vec::new(),
        };
        info!(
            "world: {} trees grown round the start in {:.0} ms, {} rivers, {} bridges, sea level {:.1} m, {} cloud puffs",
            live.forest.grown(),
            grown.elapsed().as_secs_f64() * 1000.0,
            plan.waters.rivers.len(),
            plan.waters.bridges.len(),
            plan.waters.sea_level_m,
            plan.clouds.puffs.len()
        );
        app.insert_resource(live)
            .insert_resource(Plan(Arc::new(plan)))
            .insert_resource(ClearColor(SKY))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.75, 0.82, 1.0),
                brightness: 900.0,
                ..default()
            })
            .add_plugins(crate::ground::GroundPlugin)
            .add_systems(
                Startup,
                (spawn_roads, spawn_buildings, spawn_sun, spawn_water, spawn_trees, spawn_clouds),
            )
            .add_systems(FixedUpdate, keep_trees.in_set(SimSet::World))
            .add_systems(FixedUpdate, fell_trees.in_set(SimSet::Mech).after(crate::mech::step_mech))
            .add_systems(
                Update,
                ((show_trees, redraw_trees).chain(), follow_ocean, (show_clouds, face_clouds).chain()).in_set(PresentSet::Pose),
            );
    }
}

/// The sky colour, also the haze colour at the horizon.
pub const SKY: Color = Color::srgb(0.62, 0.74, 0.88);

/// Concrete and glass tones of buildings, by id modulo the count.
pub const BUILDING_TONES: [(f32, f32, f32); 6] = [
    (0.62, 0.60, 0.56),
    (0.48, 0.50, 0.53),
    (0.70, 0.66, 0.58),
    (0.36, 0.40, 0.46),
    (0.56, 0.47, 0.40),
    (0.78, 0.77, 0.74),
];

/// Accumulates triangles for one merged mesh.
#[derive(Default)]
pub struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    /// Metres per texture tile along u and v; zero means 4 m by 4 m.
    pub tile_m: Vec2,
}

impl MeshBuilder {
    /// A builder whose texture tiles every `u` by `v` metres.
    pub fn tiled(u: f32, v: f32) -> Self {
        Self {
            tile_m: Vec2::new(u, v),
            ..default()
        }
    }

    /// Planar UV for a point on a face with normal `n`: walls run u along the wall, v up.
    fn uv(&self, p: Vec3, n: Vec3) -> [f32; 2] {
        let t = if self.tile_m == Vec2::ZERO { Vec2::splat(4.0) } else { self.tile_m };
        let a = n.abs();
        let (u, v) = if a.y >= a.x && a.y >= a.z {
            (p.x, p.z)
        } else if a.x >= a.z {
            (p.z, -p.y)
        } else {
            (p.x, -p.y)
        };
        [u / t.x, v / t.y]
    }
}

impl MeshBuilder {
    /// A flat quad, corners counter-clockwise around its outward normal `(c1 - c0) x (c3 - c0)`.
    pub fn quad(&mut self, corners: [Vec3; 4], color: [f32; 4]) {
        let base = self.positions.len() as u32;
        let n = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize_or(Vec3::Y);
        for c in corners {
            self.positions.push(c.into());
            self.normals.push(n.into());
            self.colors.push(color);
            self.uvs.push(self.uv(c, n));
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

    /// A closed box, bottom face included (for pieces seen from below, as floors through a hole).
    pub fn block(&mut self, min: Vec3, max: Vec3, color: [f32; 4]) {
        self.cuboid(min, max, color);
        let [x0, y0, z0] = min.to_array();
        let [x1, _, z1] = max.to_array();
        let v = |x, y, z| Vec3::new(x, y, z);
        self.quad([v(x0, y0, z0), v(x1, y0, z0), v(x1, y0, z1), v(x0, y0, z1)], color);
    }

    /// An upright drum or cone of `seg` sides: radius `r0` at `base`, `r1` at `base + h`, capped on top.
    pub fn drum(&mut self, base: Vec3, r0: f32, r1: f32, h: f32, seg: u32, color: [f32; 4]) {
        let at = |a: f32, r: f32, y: f32| base + Vec3::new(a.cos() * r, y, a.sin() * r);
        for i in 0..seg {
            let (a0, a1) = (
                i as f32 / seg as f32 * std::f32::consts::TAU,
                (i + 1) as f32 / seg as f32 * std::f32::consts::TAU,
            );
            self.quad([at(a0, r0, 0.0), at(a0, r1, h), at(a1, r1, h), at(a1, r0, 0.0)], color);
            if r1 > 0.0 {
                self.tri(base + Vec3::Y * h, at(a1, r1, h), at(a0, r1, h), [Vec3::Y; 3], color);
            }
        }
    }

    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, n: [Vec3; 3], color: [f32; 4]) {
        let base = self.positions.len() as u32;
        let face = (b - a).cross(c - a).normalize_or(Vec3::Y);
        for (p, n) in [(a, n[0]), (b, n[1]), (c, n[2])] {
            self.positions.push(p.into());
            self.normals.push(n.into());
            self.colors.push(color);
            self.uvs.push(self.uv(p, face));
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// An upright cylinder (sides only) from `base`, `segments` around.
    fn cylinder(&mut self, base: Vec3, r: f32, h: f32, segments: usize, color: [f32; 4]) {
        let at = |i: usize| {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            Vec3::new(a.cos(), 0.0, a.sin())
        };
        for i in 0..segments {
            let (d0, d1) = (at(i), at(i + 1));
            let (p0, p1) = (base + d0 * r, base + d1 * r);
            let up = Vec3::Y * h;
            self.tri(p0, p0 + up, p1, [d0, d0, d1], color);
            self.tri(p1, p0 + up, p1 + up, [d1, d0, d1], color);
        }
    }

    /// A cone from a circle at `base` up to a point `h` above.
    fn cone(&mut self, base: Vec3, r: f32, h: f32, segments: usize, color: [f32; 4]) {
        let at = |i: usize| {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            Vec3::new(a.cos(), 0.0, a.sin())
        };
        let tip = base + Vec3::Y * h;
        for i in 0..segments {
            let (d0, d1) = (at(i), at(i + 1));
            let n0 = (d0 * h + Vec3::Y * r).normalize();
            let n1 = (d1 * h + Vec3::Y * r).normalize();
            self.tri(base + d0 * r, tip, base + d1 * r, [n0, (n0 + n1).normalize(), n1], color);
            self.tri(base + d1 * r, base, base + d0 * r, [Vec3::NEG_Y; 3], color);
        }
    }

    /// A lumpy ellipsoid (a crown of leaves).
    fn blob(&mut self, centre: Vec3, radii: Vec3, rings: usize, color: [f32; 4]) {
        let segs = rings * 2;
        let at = |i: usize, j: usize| {
            let (th, ph) = (
                i as f32 / segs as f32 * std::f32::consts::TAU,
                j as f32 / rings as f32 * std::f32::consts::PI,
            );
            let n = Vec3::new(ph.sin() * th.cos(), ph.cos(), ph.sin() * th.sin());
            let lump = 1.0 + 0.12 * ((th * 3.0).sin() * (ph * 4.0).cos());
            (centre + n * radii * lump, n)
        };
        for j in 0..rings {
            for i in 0..segs {
                let (a, na) = at(i, j);
                let (b, nb) = at(i + 1, j);
                let (c, nc) = at(i, j + 1);
                let (d, nd) = at(i + 1, j + 1);
                self.tri(a, b, c, [na, nb, nc], color);
                self.tri(b, d, c, [nb, nd, nc], color);
            }
        }
    }

    pub fn build(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
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

fn spawn_roads(
    mut commands: Commands,
    world: Res<Plan>,
    data: Res<Data>,
    tex: Res<crate::textures::Textures>,
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
    // Lane dashes down the middle of every street and highway: 3 m painted, 6 m gap.
    let mut marks = MeshBuilder::default();
    let white = [0.92, 0.92, 0.88, 1.0];
    let mut dashes = |from: Vec2, to: Vec2, y: &dyn Fn(f32) -> f32| {
        let len = from.distance(to);
        let dir = (to - from).normalize_or(Vec2::X);
        let side = dir.perp() * 0.15;
        let mut s = 4.0;
        while s + 3.0 < len - 4.0 {
            let (a, b) = (from + dir * s, from + dir * (s + 3.0));
            let (ya, yb) = (y(s / len), y((s + 3.0) / len));
            marks.quad(
                [
                    Vec3::new(a.x + side.x, ya, a.y + side.y),
                    Vec3::new(b.x + side.x, yb, b.y + side.y),
                    Vec3::new(b.x - side.x, yb, b.y - side.y),
                    Vec3::new(a.x - side.x, ya, a.y - side.y),
                ],
                white,
            );
            s += 9.0;
        }
    };
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
            let c = s.rect.centre();
            let half = s.rect.size() * 0.5;
            let yy = y + 0.04;
            if half.x > half.y {
                dashes(Vec2::new(a.x, c.y), Vec2::new(b.x, c.y), &|_| yy);
            } else {
                dashes(Vec2::new(c.x, a.y), Vec2::new(c.x, b.y), &|_| yy);
            }
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
        let lift = cfg.road_lift_m + 0.04;
        dashes(h.ends[0], h.ends[1], &|t| h.grade_at(t) + lift);
    }
    let road_mat = materials.add(StandardMaterial {
        base_color_texture: Some(tex.asphalt.clone()),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((Name::new("roads"), Mesh3d(meshes.add(roads.build())), MeshMaterial3d(road_mat)));
    let mark_mat = vertex_material(&mut materials, 0.6);
    commands.spawn((
        Name::new("lane marks"),
        Mesh3d(meshes.add(marks.build())),
        MeshMaterial3d(mark_mat),
        NotShadowCaster,
    ));
    let kerb_mat = materials.add(StandardMaterial {
        base_color_texture: Some(tex.concrete.clone()),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((Name::new("blocks"), Mesh3d(meshes.add(kerbs.build())), MeshMaterial3d(kerb_mat)));
}

fn spawn_buildings(
    mut commands: Commands,
    world: Res<Plan>,
    tex: Res<crate::textures::Textures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ents: ResMut<crate::buildings::BuildingEntities>,
) {
    let tones = BUILDING_TONES.map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            base_color_texture: Some(tex.facade.clone()),
            perceptual_roughness: 0.75,
            ..default()
        })
    });
    commands.insert_resource(crate::buildings::BuildingLooks {
        tones: tones.to_vec(),
        structure: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        }),
        skin: {
            let b = &tex.building;
            let textured = |m: &mut Assets<StandardMaterial>, pbr: &crate::textures::Pbr, rough: f32| {
                m.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(pbr.color.clone()),
                    normal_map_texture: Some(pbr.normal.clone()),
                    perceptual_roughness: rough,
                    ..default()
                })
            };
            use cb_core::world::dressing::Mat;
            let mut m = std::collections::BTreeMap::new();
            m.insert(Mat::Brick, textured(&mut materials, &b.brick, 0.9));
            m.insert(Mat::Concrete, textured(&mut materials, &b.concrete, 0.85));
            m.insert(Mat::Roof, textured(&mut materials, &b.roof, 0.95));
            m.insert(Mat::Wood, textured(&mut materials, &b.wood, 0.7));
            m.insert(
                Mat::Trim,
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.6,
                    ..default()
                }),
            );
            // See-through glass: the rooms behind show (the mockup's windows).
            m.insert(
                Mat::Glass,
                materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 1.0, 1.0, 0.35),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.05,
                    metallic: 0.2,
                    reflectance: 0.7,
                    ..default()
                }),
            );
            m
        },
    });
    for b in &world.buildings {
        let ground = world.cities[b.city as usize].ground_m;
        // The whole box stands on the city's ground (cells and box agree; craters do not dig cities),
        // its facade tiled one window bay (4 m) by one storey (3.5 m).
        let mut mb = MeshBuilder::tiled(4.0, 3.5);
        let (a, z) = (b.footprint.min, b.footprint.max);
        mb.cuboid(Vec3::new(a.x, ground, a.y), Vec3::new(z.x, ground + b.height_m, z.y), [1.0; 4]);
        let e = commands
            .spawn((
                Name::new(format!("building {}", b.id)),
                Mesh3d(meshes.add(mb.build())),
                MeshMaterial3d(tones[b.id as usize % tones.len()].clone()),
                Transform::IDENTITY,
            ))
            .id();
        ents.0.push(e);
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Name::new("sun"),
        DirectionalLight {
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadow_maps_enabled: !crate::cli::switched_off("CB_NO_SHADOWS"),
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

/// The one water material (openspec change `water-rendering`): every river, lake and the ocean is
/// drawn with a `WaterMaterial` made here. A mesh carries its water's flow, if any, in its vertex
/// colour (rg, metres a second along x and z); without vertex colour the water is still.
pub fn water_material(materials: &mut Assets<WaterMaterial>, look: &WaterLook) -> Handle<WaterMaterial> {
    materials.add(WaterMaterial::new(look))
}

fn spawn_water(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
) {
    let water = water_material(&mut water_materials, &data.0.water_look);
    let w = &plan.waters;
    // The sea (and the lakes at its level): one sheet out to the horizon; the land hides it where
    // it rises. Its geometry and placement are `ocean_sheet` and `follow_ocean` (island-world).
    commands.spawn((
        Name::new("sea"),
        Mesh3d(meshes.add(ocean_sheet(data.0.world.view.far_m))),
        MeshMaterial3d(water.clone()),
        Transform::from_xyz(0.0, w.sea_level_m, 0.0),
        NotShadowCaster,
        OceanFollow,
    ));
    // Rivers: a strip along each course at its surface, tucked 2 m into the banks.
    for (ri, r) in w.rivers.iter().enumerate() {
        let mut positions = Vec::new();
        let mut uvs = Vec::new();
        let mut flow = Vec::new();
        let mut along = 0.0;
        for i in 0..r.path.len() {
            let p = r.path[i];
            let d = if i + 1 < r.path.len() {
                r.path[i + 1] - p
            } else {
                p - r.path[i - 1]
            }
            .normalize_or(Vec2::X);
            let side = d.perp() * (r.half_width_m[i] + 2.0);
            if i > 0 {
                along += p.distance(r.path[i - 1]);
            }
            for k in [-1.0f32, 1.0] {
                let q = p + side * k;
                positions.push([q.x, r.surface_m[i], q.y]);
                uvs.push([k * r.half_width_m[i] / 12.0, along / 24.0]);
                // The water's flow, for the water material (rg, metres a second).
                flow.push([d.x * w.flow_m_s, d.y * w.flow_m_s, 0.0, 1.0]);
            }
        }
        let mut indices = Vec::new();
        for i in 0..r.path.len() as u32 - 1 {
            let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
        let n = positions.len();
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, flow)
            .with_inserted_indices(Indices::U32(indices));
        commands.spawn((
            Name::new(format!("river {ri}")),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(water.clone()),
            NotShadowCaster,
        ));
    }
    // Bridges: a deck and two piers (placeholder boxes until a blender-csg-levels model).
    let concrete = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.57, 0.54),
        perceptual_roughness: 0.85,
        ..default()
    });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for b in &w.bridges {
        let yaw = (-b.along.x).atan2(-b.along.y);
        let rot = Quat::from_rotation_y(yaw);
        commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_xyz(b.centre.x, b.deck_m - 0.6, b.centre.y)
                .with_rotation(rot)
                .with_scale(Vec3::new(b.width_m + 2.0, 1.4, b.length_m)),
        ));
        for s in [-0.25f32, 0.25] {
            let p = b.centre + b.along * b.length_m * s;
            let floor = plan.terrain.height_at(p.x, p.y);
            let h = (b.deck_m - floor).max(1.0);
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(concrete.clone()),
                Transform::from_xyz(p.x, floor + h * 0.5 - 1.0, p.y)
                    .with_rotation(rot)
                    .with_scale(Vec3::new(b.width_m * 0.6, h, 2.5)),
            ));
        }
    }
}

// --- The sea's geometry and placement (island-world). The water's look is `water_material`
// (water-rendering); nothing here changes how water is shaded.

/// Ripple tiles are this many metres (the UV scale the water's look is drawn at).
const RIPPLE_TILE_M: f32 = 24.0;
/// The sea sheet moves in steps of this, a whole number of ripple tiles, so its UVs never slide.
const OCEAN_SNAP_M: f32 = RIPPLE_TILE_M * 100.0;

/// The sea sheet, which follows the eye across the ground so the sea always reaches the horizon.
#[derive(Component)]
struct OceanFollow;

/// The sea's surface: one flat square at height zero reaching past the far plane in every
/// direction from wherever it is placed, its UVs one per ripple tile.
fn ocean_sheet(far_m: f32) -> Mesh {
    // A whole number of double ripple tiles across, so the sheet's corner lies on the tile grid.
    let size = ((2.0 * (far_m + OCEAN_SNAP_M)) / (2.0 * RIPPLE_TILE_M)).ceil() * 2.0 * RIPPLE_TILE_M;
    let mut sheet = Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(size * 0.5)));
    if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) = sheet.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for t in uv.iter_mut() {
            *t = [t[0] * size / RIPPLE_TILE_M, t[1] * size / RIPPLE_TILE_M];
        }
    }
    sheet.clone().with_generated_tangents().unwrap_or(sheet)
}

/// Keeps the sea sheet centred under the eye, in whole snap steps.
fn follow_ocean(camera: Single<&GlobalTransform, With<PilotCamera>>, mut sheets: Query<&mut Transform, With<OceanFollow>>) {
    let eye = camera.translation();
    let snap = |v: f32| (v / OCEAN_SNAP_M).round() * OCEAN_SNAP_M;
    for mut t in &mut sheets {
        let (x, z) = (snap(eye.x), snap(eye.z));
        if t.translation.x != x || t.translation.z != z {
            t.translation.x = x;
            t.translation.z = z;
        }
    }
}

// --- End of the sea's geometry and placement.

/// Drawn trees (island-world design section 3; batched by `gpu-instancing` step 1): the grown tiles
/// within the draw radius of the eye, each drawn as ONE merged mesh per species instead of an entity
/// per tree (the render world's CPU cost is per mesh entity per view, and trees were thousands of
/// them). A tree that is falling leaves its tile's mesh and is drawn on its own while it moves; when
/// it lands its tile is rebuilt with it lying down. Collision never reads any of this: it is
/// `cb-core`'s forest.
#[derive(Resource, Default)]
struct TreeEntities {
    tiles: BTreeMap<(u32, u32), TileDraw>,
    /// The tile each drawn tree belongs to.
    tile_of: HashMap<u32, (u32, u32)>,
    /// Falling trees, drawn one entity each until they land.
    moving: HashMap<u32, Entity>,
    species: Vec<Handle<Mesh>>,
    geometry: Vec<TreeGeometry>,
    material: Handle<StandardMaterial>,
}

/// One drawn tile: its merged species meshes, whether they cast shadows, whether to rebuild.
struct TileDraw {
    meshes: Vec<Entity>,
    shadows: bool,
    dirty: bool,
}

/// A species' mesh on the CPU, one unit tall, to be merged per tile.
struct TreeGeometry {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl TreeGeometry {
    fn of(mesh: &Mesh) -> Self {
        use bevy::mesh::VertexAttributeValues as V;
        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(V::Float32x3(v)) => v.clone(),
            _ => Vec::new(),
        };
        let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(V::Float32x3(v)) => v.clone(),
            _ => vec![[0.0, 1.0, 0.0]; positions.len()],
        };
        let colors = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(V::Float32x4(v)) => v.clone(),
            _ => vec![[1.0; 4]; positions.len()],
        };
        let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(V::Float32x2(v)) => v.clone(),
            _ => vec![[0.0; 2]; positions.len()],
        };
        let indices = match mesh.indices() {
            Some(Indices::U32(i)) => i.clone(),
            Some(Indices::U16(i)) => i.iter().map(|x| *x as u32).collect(),
            None => (0..positions.len() as u32).collect(),
        };
        Self {
            positions,
            normals,
            colors,
            uvs,
            indices,
        }
    }
}

/// The trees of one species in a tile, merged into one mesh in world space.
fn merge_trees<'a>(geometry: &TreeGeometry, trees: impl Iterator<Item = (&'a cb_core::world::Tree, TreeState)>) -> Option<Mesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for (t, s) in trees {
        let m = tree_transform(t, s).to_matrix();
        let base = positions.len() as u32;
        positions.extend(
            geometry
                .positions
                .iter()
                .map(|p| m.transform_point3(Vec3::from_array(*p)).to_array()),
        );
        normals.extend(
            geometry
                .normals
                .iter()
                .map(|n| m.transform_vector3(Vec3::from_array(*n)).normalize_or(Vec3::Y).to_array()),
        );
        colors.extend_from_slice(&geometry.colors);
        uvs.extend_from_slice(&geometry.uvs);
        indices.extend(geometry.indices.iter().map(|i| i + base));
    }
    if positions.is_empty() {
        return None;
    }
    Some(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_indices(Indices::U32(indices)),
    )
}

/// Most tree tiles built (put on screen or rebuilt) in one frame.
const TREE_TILES_PER_FRAME: usize = 6;
/// Tree tiles stay on screen this far past the draw radius before they are taken off.
const TREE_KEEP_MARGIN_M: f32 = 150.0;
/// Only tree tiles within this of the eye cast shadows (the shadow pass draws far fewer trees).
const TREE_SHADOW_RADIUS_M: f32 = 300.0;

/// A tree mesh, one unit tall, base at the origin: trunk and crown in vertex colours.
fn tree_mesh(species: &str) -> Mesh {
    // The mesh is one unit tall: a foliage tile every 5 cm of it, about a metre of a real tree.
    let mut b = MeshBuilder::tiled(0.05, 0.05);
    let bark = [0.30, 0.22, 0.15, 1.0];
    let (trunk_r, trunk_top) = match species {
        "birch" => (0.018, 0.55),
        "oak" => (0.035, 0.45),
        _ => (0.022, 0.35),
    };
    b.cylinder(
        Vec3::ZERO,
        trunk_r,
        trunk_top,
        7,
        if species == "birch" { [0.85, 0.83, 0.78, 1.0] } else { bark },
    );
    match species {
        "oak" => {
            b.blob(Vec3::new(0.0, 0.68, 0.0), Vec3::new(0.34, 0.3, 0.34), 9, [0.20, 0.33, 0.13, 1.0]);
            b.blob(Vec3::new(0.12, 0.6, 0.08), Vec3::new(0.22, 0.2, 0.22), 7, [0.24, 0.36, 0.15, 1.0]);
        }
        "birch" => b.blob(Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.2, 0.3, 0.2), 8, [0.36, 0.48, 0.20, 1.0]),
        _ => {
            for (k, (y, r, h)) in [(0.2, 0.2, 0.35), (0.45, 0.16, 0.32), (0.68, 0.11, 0.32)].into_iter().enumerate() {
                let shade = 0.02 * k as f32;
                b.cone(Vec3::new(0.0, y, 0.0), r, h, 8, [0.12 + shade, 0.25 + shade, 0.12, 1.0]);
            }
        }
    }
    b.build()
}

fn tree_transform(t: &cb_core::world::Tree, s: TreeState) -> Transform {
    let (dir, angle) = s.tilt();
    // Fall about the horizontal axis across the fall direction, pivoting at the base.
    let axis = Vec3::new(dir.y, 0.0, -dir.x).normalize_or(Vec3::X);
    Transform::from_translation(t.base - Vec3::Y * 0.2)
        .with_rotation(Quat::from_axis_angle(axis, angle) * Quat::from_rotation_y(t.yaw_rad))
        .with_scale(Vec3::splat(t.height_m))
}

fn spawn_trees(
    mut commands: Commands,
    data: Res<Data>,
    tex: Res<crate::textures::Textures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(tex.foliage.clone()),
        perceptual_roughness: 0.9,
        ..default()
    });
    let built: Vec<Mesh> = data.0.world.trees.species.iter().map(|s| tree_mesh(&s.name)).collect();
    let geometry = built.iter().map(TreeGeometry::of).collect();
    let species = built.into_iter().map(|m| meshes.add(m)).collect();
    commands.insert_resource(TreeEntities {
        species,
        geometry,
        material,
        ..default()
    });
}

/// Grows the trees near the pilot and every mech before they move this step, and lets go of the
/// trees far from all of them (cb-core `Forest::keep_around`; deterministic: it reads only the
/// simulation's positions).
fn keep_trees(mut live: ResMut<Live>, plan: Res<Plan>, pilot: Res<crate::pilot::PilotSim>, mechs: Query<&crate::mech::MechSim>) {
    let feet = pilot.curr.feet;
    let mut bodies = vec![Vec2::new(feet.x, feet.z)];
    bodies.extend(mechs.iter().map(|m| Vec2::new(m.curr.pos.x, m.curr.pos.z)));
    live.forest.keep_around(&plan, &bodies);
}

/// Puts the grown tiles within the draw radius of the eye on screen as merged meshes, a few tiles a
/// frame nearest first (new ones, then ones a fallen tree changed), and takes off the ones beyond
/// it or let go.
fn show_trees(
    mut commands: Commands,
    live: Res<Live>,
    data: Res<Data>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    trees: Option<ResMut<TreeEntities>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(mut trees) = trees else { return };
    let trees = &mut *trees;
    let eye = camera.translation();
    let eye = Vec2::new(eye.x, eye.z);
    let radius = data.0.world.trees.draw_radius_m;
    let grown: Vec<(f32, (u32, u32))> = live
        .forest
        .grown_tiles()
        .map(|(k, _)| (live.forest.tile_rect(k).distance(eye), k))
        .collect();
    // On screen within the radius; taken off only past it by a margin, so a tile on the edge does
    // not come and go frame by frame.
    let keep: BTreeSet<(u32, u32)> = grown.iter().filter(|g| g.0 <= radius + TREE_KEEP_MARGIN_M).map(|g| g.1).collect();
    let gone: Vec<(u32, u32)> = trees.tiles.keys().filter(|k| !keep.contains(k)).copied().collect();
    for key in gone {
        if let Some(tile) = trees.tiles.remove(&key) {
            for e in tile.meshes {
                commands.entity(e).despawn();
            }
        }
        trees.tile_of.retain(|_, k| *k != key);
    }
    // New tiles and changed ones, nearest first, a few a frame.
    let mut todo: Vec<(f32, (u32, u32))> = grown
        .into_iter()
        .filter(|g| g.0 <= radius && trees.tiles.get(&g.1).is_none_or(|t| t.dirty))
        .collect();
    todo.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (d, key) in todo.into_iter().take(TREE_TILES_PER_FRAME) {
        let Some((_, grown)) = live.forest.grown_tiles().find(|(k, _)| *k == key) else {
            continue;
        };
        if let Some(old) = trees.tiles.remove(&key) {
            for e in old.meshes {
                commands.entity(e).despawn();
            }
        }
        let shadows = d <= TREE_SHADOW_RADIUS_M;
        let mut entities = Vec::new();
        for (sp, geometry) in trees.geometry.iter().enumerate() {
            // Falling trees are drawn on their own (`redraw_trees`), not in the tile.
            let these = grown
                .iter()
                .filter(|t| t.species as usize == sp && !trees.moving.contains_key(&t.id))
                .map(|t| (t, live.forest.state(t.id)));
            let Some(mesh) = merge_trees(geometry, these) else { continue };
            let mut e = commands.spawn((
                Name::new("tree tile"),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(trees.material.clone()),
                Transform::IDENTITY,
                VisibilityRange::abrupt(0.0, radius + TREE_KEEP_MARGIN_M),
            ));
            if !shadows {
                e.insert(NotShadowCaster);
            }
            entities.push(e.id());
        }
        for t in grown {
            trees.tile_of.insert(t.id, key);
        }
        trees.tiles.insert(
            key,
            TileDraw {
                meshes: entities,
                shadows,
                dirty: false,
            },
        );
    }
    // Shadows from the near tiles only; a tile changes over as a whole, rarely.
    for (key, tile) in trees.tiles.iter_mut() {
        let near = live.forest.tile_rect(*key).distance(eye) <= TREE_SHADOW_RADIUS_M;
        if near != tile.shadows {
            tile.shadows = near;
            for e in &tile.meshes {
                if near {
                    commands.entity(*e).remove::<NotShadowCaster>();
                } else {
                    commands.entity(*e).insert(NotShadowCaster);
                }
            }
        }
    }
}

fn fell_trees(mut live: ResMut<Live>, time: Res<Time<Fixed>>, mut fx: MessageWriter<crate::fx::Fx>) {
    let live = &mut *live;
    let landed = live.forest.step(time.delta_secs());
    for id in &landed {
        let Some(t) = live.forest.tree(*id).copied() else { continue };
        let (dir, _) = live.forest.state(*id).tilt();
        let tip = t.base + Vec3::new(dir.x, 0.0, dir.y) * t.height_m * 0.7;
        fx.write(crate::fx::Fx::new("tree_fall", tip));
    }
    let falling = live.forest.falling();
    live.moved_trees.extend(falling);
    live.moved_trees.extend(landed);
}

/// A tree that starts falling leaves its tile's mesh and is drawn on its own as it falls; when it
/// lands it goes back into its tile, which is rebuilt with it lying down.
fn redraw_trees(mut commands: Commands, mut live: ResMut<Live>, trees: Option<ResMut<TreeEntities>>, mut q: Query<&mut Transform>) {
    let Some(mut trees) = trees else { return };
    let trees = &mut *trees;
    let moved = std::mem::take(&mut live.moved_trees);
    for id in moved {
        let Some(tree) = live.forest.tree(id) else { continue };
        let state = live.forest.state(id);
        let Some(key) = trees.tile_of.get(&id).copied() else { continue };
        match state {
            TreeState::Falling { .. } => match trees.moving.get(&id) {
                Some(e) => {
                    if let Ok(mut t) = q.get_mut(*e) {
                        *t = tree_transform(tree, state);
                    }
                }
                None => {
                    let e = commands
                        .spawn((
                            Mesh3d(trees.species[tree.species as usize].clone()),
                            MeshMaterial3d(trees.material.clone()),
                            tree_transform(tree, state),
                        ))
                        .id();
                    trees.moving.insert(id, e);
                    if let Some(tile) = trees.tiles.get_mut(&key) {
                        tile.dirty = true;
                    }
                }
            },
            _ => {
                if let Some(e) = trees.moving.remove(&id) {
                    commands.entity(e).despawn();
                }
                if let Some(tile) = trees.tiles.get_mut(&key) {
                    tile.dirty = true;
                }
            }
        }
    }
}

/// A cloud billboard.
#[derive(Component)]
struct CloudSprite {
    anchor: Vec3,
}

fn cloud_texture(images: &mut Assets<Image>) -> Handle<Image> {
    const N: usize = 128;
    let mut data = Vec::with_capacity(N * N * 4);
    for j in 0..N {
        for i in 0..N {
            let (u, v) = (i as f32 / (N - 1) as f32 * 2.0 - 1.0, j as f32 / (N - 1) as f32 * 2.0 - 1.0);
            let r = (u * u + v * v).sqrt();
            let n = ((u * 9.0).sin() * (v * 7.0).cos() * 0.08 + (u * 17.0 + v * 13.0).sin() * 0.05).abs();
            let a = ((1.0 - r).clamp(0.0, 1.0).powf(1.4) - n).clamp(0.0, 1.0);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

/// Drawn clouds (island-world): the cloud tiles near the eye, a few billboards per puff.
#[derive(Resource)]
struct CloudSprites {
    quad: Handle<Mesh>,
    shades: Vec<Handle<StandardMaterial>>,
    /// Every tile with puffs and its centre (x, z) at time zero.
    all: Vec<((i32, i32), Vec2)>,
    drawn: HashMap<(i32, i32), Vec<Entity>>,
}

/// Most cloud tiles put in the sky in one frame.
const CLOUD_TILES_PER_FRAME: usize = 2;

fn spawn_clouds(
    mut commands: Commands,
    plan: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let tex = cloud_texture(&mut images);
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let shades: Vec<Handle<StandardMaterial>> = [0.98, 0.9, 0.8]
        .iter()
        .map(|g| {
            materials.add(StandardMaterial {
                base_color: Color::srgba(*g, *g, *g * 1.02, 0.85),
                base_color_texture: Some(tex.clone()),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                fog_enabled: true,
                ..default()
            })
        })
        .collect();
    commands.insert_resource(CloudSprites {
        quad,
        shades,
        all: plan.clouds.tiles(),
        drawn: HashMap::new(),
    });
}

/// The billboards of one puff: seven, scattered through it, darker underneath. A pure function of
/// the seed and the puff, so a tile drawn again looks the same.
fn puff_sprites(seed: u64, index: u32, p: &cb_core::world::clouds::Puff) -> [(Vec3, f32, usize); 7] {
    let mut s: u64 = (seed ^ 0xC10D).wrapping_add(u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut f = move || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 40) as f32) / (1u64 << 24) as f32
    };
    std::array::from_fn(|_| {
        let off = Vec3::new(f() - 0.5, f() - 0.5, f() - 0.5) * p.radii * 1.1;
        let size = p.radii.x.max(p.radii.y) * (0.9 + 0.6 * f());
        let shade = if off.y < -p.radii.y * 0.2 {
            2
        } else if off.y < p.radii.y * 0.2 {
            1
        } else {
            0
        };
        (p.centre + off, size, shade)
    })
}

/// Puts the cloud tiles within the draw radius of the eye in the sky (as the wind has carried them),
/// a couple a frame nearest first, and takes the ones beyond it down. Beyond the radius the haze
/// has hidden them anyway.
fn show_clouds(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    sprites: Option<ResMut<CloudSprites>>,
) {
    let Some(mut sprites) = sprites else { return };
    let eye = camera.translation();
    let drift = plan.clouds.wind_m_s * time.elapsed_secs();
    let radius = data.0.world.clouds.draw_radius_m;
    let half_diag = cb_core::world::clouds::CLOUD_TILE_M * std::f32::consts::FRAC_1_SQRT_2;
    let dist = |c: Vec2| ((c + drift).distance(Vec2::new(eye.x, eye.z)) - half_diag).max(0.0);
    let s = &mut *sprites;
    let gone: Vec<(i32, i32)> = s
        .drawn
        .keys()
        .copied()
        .filter(|k| s.all.iter().find(|a| a.0 == *k).is_none_or(|a| dist(a.1) > radius + half_diag))
        .collect();
    for key in gone {
        for e in s.drawn.remove(&key).unwrap_or_default() {
            commands.entity(e).despawn();
        }
    }
    let mut wanted: Vec<(f32, (i32, i32))> = s
        .all
        .iter()
        .filter(|a| !s.drawn.contains_key(&a.0))
        .map(|a| (dist(a.1), a.0))
        .filter(|w| w.0 <= radius)
        .collect();
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, key) in wanted.into_iter().take(CLOUD_TILES_PER_FRAME) {
        let mut ents = Vec::new();
        for i in plan.clouds.puffs_in(key) {
            for (at, size, shade) in puff_sprites(plan.seed, i, &plan.clouds.puffs[i as usize]) {
                ents.push(
                    commands
                        .spawn((
                            Mesh3d(s.quad.clone()),
                            MeshMaterial3d(s.shades[shade].clone()),
                            Transform::from_translation(at).with_scale(Vec3::splat(size)),
                            NotShadowCaster,
                            CloudSprite { anchor: at },
                        ))
                        .id(),
                );
            }
        }
        s.drawn.insert(key, ents);
    }
}

fn face_clouds(
    time: Res<Time>,
    plan: Res<Plan>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut q: Query<(&mut Transform, &CloudSprite)>,
) {
    let drift = Vec3::new(plan.clouds.wind_m_s.x, 0.0, plan.clouds.wind_m_s.y) * time.elapsed_secs();
    let rot = camera.rotation();
    for (mut t, c) in &mut q {
        t.translation = c.anchor + drift;
        t.rotation = rot;
    }
}
