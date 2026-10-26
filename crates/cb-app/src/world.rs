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
use bevy::render::render_resource::PrimitiveTopology;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use cb_core::water_look::WaterLook;
use cb_core::world::{Deformation, Forest, TreeState, WorldPlan, generate};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// The world seed.
#[derive(Resource, Clone, Copy)]
pub struct Seed(pub u64);

/// The sun's light: the one directional light that lights the world (the cockpit's screen glow is
/// another, kept to the cockpit layer). The Options page turns its shadows on and off (`settings`).
#[derive(Component)]
pub struct Sun;

/// True unless `CB_NO_SKY=1`: the pilot's view has Bevy's scattering sky (sky-atmosphere).
pub fn scattering_sky() -> bool {
    !crate::cli::switched_off("CB_NO_SKY")
}

/// How much brighter a light set at Bevy's default exposure (EV100 9.7) must be to look the same
/// under the scattering sky's exposure: `2^(EV100 - 9.7)`, 1 without the scattering sky. The one
/// place the flat ambient and the sky light (`sky_light`) are scaled from.
pub fn exposure_scale(view: &cb_core::world::config::ViewConfig) -> f32 {
    if scattering_sky() {
        2f32.powf(view.exposure_ev100 - 9.7)
    } else {
        1.0
    }
}

/// Start on a grassy rise overlooking a city, facing away from it with the sun beyond it
/// (`--start overlook`, for poster stills: `cb_core::grass::overlook_start`).
#[derive(Resource)]
pub struct StartOnOverlook;

/// Start on the first air base's apron, behind the mech, which faces the hangar bays (`--start base`,
/// air-bases).
#[derive(Resource)]
pub struct StartAtBase;

/// Start in the nearest meadow instead of the city (`--start meadow`, gpu-grass).
#[derive(Resource)]
pub struct StartInMeadow;

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
        let mut plan = generate(seed, &cfg);
        if app.world().contains_resource::<StartOnOverlook>() {
            let grass = &app.world().resource::<Data>().0.grass;
            match cb_core::grass::overlook_start(&plan, grass, cb_core::world::sun_shade::to_sun()) {
                Some((s, rise)) => {
                    info!(
                        "world: starting on an overlook at ({:.0}, {:.0}), {rise:.0} m over the city's ground",
                        s.mech_xz.x, s.mech_xz.y
                    );
                    let mut s = s;
                    // Poster stills turn the mech off the camera for a silhouette (`capture`).
                    if crate::cli::switched_off("CB_STILLS_POSTER") {
                        s.mech_heading_rad += crate::capture::POSTER_MECH_TURN_DEG.to_radians();
                    }
                    plan.start = s;
                }
                None => warn!("world: no grassy overlook found; starting in the city"),
            }
        }
        if app.world().contains_resource::<StartInMeadow>() {
            let grass = &app.world().resource::<Data>().0.grass;
            // A wide meadow near the start, else (the island's biomes leave few) a smaller one farther out.
            let found = cb_core::grass::meadow_start(&plan, grass, 40.0, 6000.0)
                .or_else(|| cb_core::grass::meadow_start(&plan, grass, 15.0, 15000.0));
            match found {
                Some(s) => {
                    info!("world: starting in a meadow at ({:.0}, {:.0})", s.mech_xz.x, s.mech_xz.y);
                    plan.start = s;
                }
                None => warn!("world: no meadow within 6 km; starting in the city"),
            }
        }
        if app.world().contains_resource::<StartAtBase>() {
            match plan.bases.first() {
                Some(b) => {
                    // 150 m in front of the middle hangar bay's mouth, facing it (heading pi faces +z);
                    // the pilot stands 25 m behind.
                    let bays = b.hangars(&cfg.bases);
                    let mouth = bays.get(bays.len() / 2).map_or(b.centre, |h| Vec2::new(h.centre().x, h.min.y));
                    let mech_xz = mouth - Vec2::new(0.0, 150.0);
                    plan.start = cb_core::world::Start {
                        mech_xz,
                        mech_heading_rad: std::f32::consts::PI,
                        pilot_xz: mech_xz - Vec2::new(0.0, cfg.start_pilot_distance_m),
                    };
                    info!("world: starting on air base {} at ({:.0}, {:.0})", b.id, mech_xz.x, mech_xz.y);
                }
                None => warn!("world: no air base; starting in the city"),
            }
        }
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
        let sky = app.world().resource::<Data>().0.sky_light.clone();
        let ambient_cd_m2 = crate::sky_light::ambient_cd_m2(&sky);
        let [ar, ag, ab] = sky.ambient_colour;
        app.insert_resource(live)
            .insert_resource(Plan(Arc::new(plan)))
            .insert_resource(ClearColor(SKY))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(ar, ag, ab),
                // Under the scattering sky the exposure is darker by the step from Bevy's default
                // EV100 9.7 to the view's, so the ambient rises by the same factor.
                brightness: ambient_cd_m2 * exposure_scale(&cfg.view),
                ..default()
            })
            .add_plugins((crate::ground::GroundPlugin, crate::rocks::RocksPlugin))
            .add_systems(Startup, (spawn_roads, spawn_buildings, spawn_sun, spawn_water, spawn_trees))
            .add_systems(FixedUpdate, keep_trees.in_set(SimSet::World))
            .add_systems(FixedUpdate, fell_trees.in_set(SimSet::Mech).after(crate::mech::step_mech))
            .add_systems(Update, ((show_trees, redraw_trees).chain(), follow_ocean).in_set(PresentSet::Pose));
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
    /// Second UV: the texture array layer in x (`building-texture-array`), when `layered`.
    uv1s: Vec<[f32; 2]>,
    indices: Vec<u32>,
    /// Metres per texture tile along u and v; zero means 4 m by 4 m.
    pub tile_m: Vec2,
    /// The layer the next faces are drawn from.
    layer: f32,
    /// True once a layer was set: the mesh carries the second UV.
    layered: bool,
}

impl MeshBuilder {
    /// A builder whose texture tiles every `u` by `v` metres.
    pub fn tiled(u: f32, v: f32) -> Self {
        Self {
            tile_m: Vec2::new(u, v),
            ..default()
        }
    }

    /// The texture array layer, and its tile size, of the faces added next.
    pub fn set_layer(&mut self, layer: u32, tile_m: Vec2) {
        self.layer = layer as f32;
        self.tile_m = tile_m;
        self.layered = true;
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
            self.uv1s.push([self.layer, 0.0]);
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
            self.uv1s.push([self.layer, 0.0]);
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

    /// The mesh with tangents for a normal map, written straight from the planar UV mapping (`uv`):
    /// u runs along +x on floors and on z-facing walls, along +z on x-facing walls; v runs +z on
    /// floors and down on walls. MikkTSpace (`Mesh::generate_tangents`) gives the same on these
    /// faces and cost most of a combat frame's building redraw (`building-surfaces`, 2026-10-24).
    pub fn build_with_tangents(self) -> Mesh {
        let tangents: Vec<[f32; 4]> = self.normals.iter().map(|n| planar_tangent(Vec3::from(*n))).collect();
        self.build().with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, tangents)
    }

    pub fn build(self) -> Mesh {
        let layered = self.layered;
        let uv1s = self.uv1s;
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_indices(Indices::U32(self.indices));
        if layered {
            mesh.with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, uv1s)
        } else {
            mesh
        }
    }
}

/// The tangent (xyz, handedness w) of `MeshBuilder::uv`'s planar mapping on a face of normal `n`.
fn planar_tangent(n: Vec3) -> [f32; 4] {
    let a = n.abs();
    let (t, b) = if a.y >= a.x && a.y >= a.z {
        (Vec3::X, Vec3::Z)
    } else if a.x >= a.z {
        (Vec3::Z, Vec3::NEG_Y)
    } else {
        (Vec3::X, Vec3::NEG_Y)
    };
    let t = (t - n * n.dot(t)).normalize_or(Vec3::X);
    // Bevy's tangent space has v running against the bitangent (glTF's flipped v), so the
    // handedness is the opposite of cross(n, t) against the direction v grows.
    let w = if n.cross(t).dot(b) < 0.0 { 1.0 } else { -1.0 };
    [t.x, t.y, t.z, w]
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
    // Highways: a strip along the routed centre line, every 4 m, each edge square to the road
    // where it is, with the centre dashes along each 16 m piece.
    for h in &world.highways {
        let steps = (h.length_m() / 4.0).ceil() as usize;
        let p = |t: f32, side: f32| {
            let xz = h.point_at(t) + h.dir_at(t).perp() * h.width_m * 0.5 * side;
            Vec3::new(xz.x, h.grade_at(t) + cfg.road_lift_m, xz.y)
        };
        for i in 0..steps {
            let (t0, t1) = (i as f32 / steps as f32, (i + 1) as f32 / steps as f32);
            roads.quad([p(t0, 1.0), p(t1, 1.0), p(t1, -1.0), p(t0, -1.0)], asphalt);
        }
        let lift = cfg.road_lift_m + 0.04;
        let n = h.path.len();
        for k in 0..n - 1 {
            let (g0, g1) = (h.grade_m[k], h.grade_m[k + 1]);
            dashes(h.path[k], h.path[k + 1], &|t| g0 + (g1 - g0) * t + lift);
        }
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
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut ents: ResMut<crate::buildings::BuildingEntities>,
) {
    // `CB_NO_BUILDINGS=1`: no buildings drawn (perf-bench switch, to measure what they cost).
    if crate::cli::switched_off("CB_NO_BUILDINGS") {
        return;
    }
    let tones = BUILDING_TONES.map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            base_color_texture: Some(tex.facade.clone()),
            perceptual_roughness: 0.75,
            ..default()
        })
    });
    let arrays = crate::building_arrays::build(&mut images, &crate::buildings::layer_sources(&tex));
    commands.insert_resource(crate::buildings::BuildingLooks {
        tone_tints: BUILDING_TONES
            .iter()
            .map(|&(r, g, b)| Color::srgb(r, g, b).to_linear().to_f32_array_no_alpha())
            .collect(),
        arrays,
        // See-through glass: the rooms behind show (the mockup's windows), and it reflects the sky
        // at full strength (glass-reflections).
        glass: materials.add(crate::sky_light::glass_material(&data.0.sky_light.glass)),
        concrete: crate::buildings::concrete_material(&mut images, &mut materials),
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

fn spawn_sun(mut commands: Commands, data: Res<Data>, mut media: ResMut<Assets<bevy::light::atmosphere::ScatteringMedium>>) {
    if scattering_sky() {
        // Earth's air (sky-atmosphere): Rayleigh and Mie scattering through Bevy's lookup tables.
        let air = media.add(bevy::light::atmosphere::ScatteringMedium::earth(256, 256));
        commands.spawn((Name::new("atmosphere"), bevy::light::Atmosphere::earth(air)));
    }
    let mut sun = commands.spawn((
        Name::new("sun"),
        Sun,
        DirectionalLight {
            // Raw sunlight under the scattering sky, which filters it; the old daylight value otherwise.
            illuminance: if scattering_sky() {
                light_consts::lux::RAW_SUNLIGHT
            } else {
                light_consts::lux::AMBIENT_DAYLIGHT
            },
            shadow_maps_enabled: !crate::cli::switched_off("CB_NO_SHADOWS"),
            ..default()
        },
        // The one sun, shared with the terrain's baked shadow (distant-shadows).
        Transform::from_rotation(cb_core::world::sun_shade::sun_rotation()),
        // The live shadows near the eye, from data (`sun_shade:` in `data/world/world.ron`).
        CascadeShadowConfigBuilder {
            num_cascades: data.0.world.sun_shade.live_cascades as usize,
            first_cascade_far_bound: data.0.world.sun_shade.live_first_cascade_m,
            maximum_distance: data.0.world.sun_shade.live_far_m,
            ..default()
        }
        .build(),
    ));
    if scattering_sky() {
        let view = &data.0.world.view;
        sun.insert(bevy::light::SunDisk {
            angular_size: view.sun_disk_deg.to_radians(),
            intensity: view.sun_disk_intensity,
        });
    }
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
    // Rivers: a strip along each course at its surface, tucked 2 m into the banks, gathered into
    // one mesh per `RIVER_TILE_M` tile so the island's hundreds of streams are a few draw calls. A
    // segment inside a lake is not drawn (the lake's surface is the water there), nor one down a
    // waterfall (the falling sheet is).
    let mut tiles: BTreeMap<(i32, i32), Ribbons> = BTreeMap::new();
    for (ri, r) in w.rivers.iter().enumerate() {
        let falling = |i: usize| w.falls.iter().any(|f| f.river == ri && i >= f.lip && i < f.foot);
        let in_lake: Vec<bool> = r.path.iter().map(|p| w.lakes.at(*p).is_some()).collect();
        let mut along = 0.0;
        let mut edges = Vec::with_capacity(r.path.len());
        for i in 0..r.path.len() {
            let p = r.path[i];
            let d = if i + 1 < r.path.len() {
                r.path[i + 1] - p
            } else {
                p - r.path[i - 1]
            }
            .normalize_or(Vec2::X);
            if i > 0 {
                along += p.distance(r.path[i - 1]);
            }
            let side = d.perp() * (r.half_width_m[i] + 2.0);
            edges.push((p - side, p + side, d, along));
        }
        for i in 0..r.path.len() - 1 {
            if (in_lake[i] && in_lake[i + 1]) || falling(i) {
                continue;
            }
            let mid = (r.path[i] + r.path[i + 1]) * 0.5;
            let key = ((mid.x / RIVER_TILE_M).floor() as i32, (mid.y / RIVER_TILE_M).floor() as i32);
            let rb = tiles.entry(key).or_default();
            let base = rb.positions.len() as u32;
            for k in [i, i + 1] {
                let (l, rt, d, at) = edges[k];
                for (q, u) in [(l, -1.0f32), (rt, 1.0)] {
                    rb.positions.push([q.x, r.surface_m[k], q.y]);
                    rb.uvs.push([u * r.half_width_m[k] / 12.0, at / 24.0]);
                    // The water's flow, for the water material (rg, metres a second).
                    rb.flow.push([d.x * w.flow_m_s, d.y * w.flow_m_s, 0.0, 1.0]);
                }
            }
            rb.indices
                .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }
    }
    for ((tx, tz), rb) in tiles {
        let n = rb.positions.len();
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, rb.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, rb.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, rb.flow)
            .with_inserted_indices(Indices::U32(rb.indices));
        commands.spawn((
            Name::new(format!("rivers {tx},{tz}")),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(water.clone()),
            NotShadowCaster,
        ));
    }
    // Lakes: a still surface at the lake's level out to 3 m past its shore, where the raised bank
    // hides its edge (the shore's outline is `Lakes::outline_m`, the same rule the carve uses).
    for (li, l) in w.lakes.list.iter().enumerate() {
        let rays = 96;
        let mut positions = vec![[l.centre.x, l.level_m, l.centre.y]];
        for k in 0..rays {
            let a = k as f32 / rays as f32 * std::f32::consts::TAU;
            let q = l.centre + Vec2::new(a.cos(), a.sin()) * w.lakes.outline_m(li, a, 3.0);
            positions.push([q.x, l.level_m, q.y]);
        }
        let uvs: Vec<[f32; 2]> = positions.iter().map(|p| [p[0] / RIPPLE_TILE_M, p[2] / RIPPLE_TILE_M]).collect();
        let mut indices = Vec::with_capacity(rays * 3);
        for k in 0..rays as u32 {
            // Wound to face up: centre, next, this (angles run from +x toward +z).
            indices.extend_from_slice(&[0, 1 + (k + 1) % rays as u32, 1 + k]);
        }
        let n = positions.len();
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_indices(Indices::U32(indices));
        commands.spawn((
            Name::new(format!("lake {li}")),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(water.clone()),
            NotShadowCaster,
        ));
    }
    spawn_falls(&mut commands, &plan, &mut meshes, &mut materials);
    // Bridges: a deck and two piers (placeholder boxes until a blender-csg-levels model).
    let concrete = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.57, 0.54),
        perceptual_roughness: 0.85,
        ..default()
    });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    // The deck's top lies 5 cm under the highway's surface, which runs across it at `road_lift_m` over
    // the grade: level with it, the two fought (owner, 2026-10-25: "this bridge has zfighting").
    let deck_top = data.0.world.road_lift_m - 0.05;
    for b in &w.bridges {
        let yaw = (-b.along.x).atan2(-b.along.y);
        let rot = Quat::from_rotation_y(yaw);
        commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_xyz(b.centre.x, b.deck_m + deck_top - 0.7, b.centre.y)
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

/// River strips are gathered into one mesh per tile this many metres square.
const RIVER_TILE_M: f32 = 4096.0;

/// One tile's river strips, being built.
#[derive(Default)]
struct Ribbons {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    flow: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

/// Gravity for the falling water's arc, metres a second squared.
const GRAVITY_M_S2: f32 = 9.81;

/// Waterfalls (openspec change `hydrology`): a sheet of water leaving each lip at the river's flow
/// speed and falling on gravity's arc, never through the cliff face behind it, to the plunge
/// pool's surface, with foam where it lands.
fn spawn_falls(commands: &mut Commands, plan: &Plan, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let w = &plan.waters;
    if w.falls.is_empty() {
        return;
    }
    let sheet = materials.add(StandardMaterial {
        base_color: Color::srgba(0.86, 0.91, 0.95, 0.8),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.35,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    let foam = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.97, 0.98, 0.7),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        ..default()
    });
    let disc = meshes.add(Circle::new(1.0));
    for (fi, f) in w.falls.iter().enumerate() {
        let lip = w.rivers[f.river].path[f.lip];
        let side = f.along.perp();
        let height = (f.top_m - f.bottom_m).max(0.5);
        let fall_s = (2.0 * height / GRAVITY_M_S2).sqrt();
        let v0 = w.flow_m_s.max(0.5);
        let rows = 24;
        let (mut positions, mut uvs, mut indices) = (Vec::new(), Vec::new(), Vec::new());
        let mut landing = lip;
        for k in 0..=rows {
            let t = fall_s * k as f32 / rows as f32;
            let c = lip + f.along * v0 * t;
            let ground = plan.terrain.height_at(c.x, c.y);
            let y = (f.top_m - 0.5 * GRAVITY_M_S2 * t * t)
                .max(ground + 0.4)
                .min(f.top_m)
                .max(f.bottom_m);
            let hw = f.half_width_m * (1.0 + 0.3 * k as f32 / rows as f32);
            for s in [-1.0f32, 1.0] {
                let q = c + side * hw * s;
                positions.push([q.x, y, q.y]);
                uvs.push([s * 0.5 + 0.5, (f.top_m - y) / 8.0]);
            }
            landing = c;
        }
        for k in 0..rows as u32 {
            let a = 2 * k;
            indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
        let n = positions.len();
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[-f.along.x, 0.0, -f.along.y]; n])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_indices(Indices::U32(indices));
        commands.spawn((
            Name::new(format!("waterfall {fi}")),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(sheet.clone()),
            NotShadowCaster,
        ));
        commands.spawn((
            Name::new(format!("waterfall {fi} foam")),
            Mesh3d(disc.clone()),
            MeshMaterial3d(foam.clone()),
            Transform::from_xyz(landing.x, f.bottom_m + 0.25, landing.y)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(f.half_width_m * 1.8)),
            NotShadowCaster,
        ));
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
    /// Felled trees, falling or lying, drawn one entity each (left out of their tile's mesh).
    moving: HashMap<u32, Entity>,
    species: Vec<Handle<Mesh>>,
    geometry: Arc<Vec<TreeGeometry>>,
    /// The far version of each species (`tree_mesh(.., true)`), for tiles past `TREE_FAR_M`.
    geometry_far: Arc<Vec<TreeGeometry>>,
    material: Handle<StandardMaterial>,
    /// Tiles whose meshes are being merged on a worker thread, and whether in the far version.
    building: BTreeMap<(u32, u32), (Task<Vec<(Mesh, Vec3)>>, bool)>,
}

/// One drawn tile: its merged species meshes, whether they cast shadows, whether to rebuild.
struct TileDraw {
    meshes: Vec<Entity>,
    shadows: bool,
    dirty: bool,
    /// Built from the far version of the trees.
    far: bool,
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
fn merge_trees<'a>(geometry: &TreeGeometry, trees: impl Iterator<Item = (&'a cb_core::world::Tree, TreeState)>) -> Option<(Mesh, Vec3)> {
    let mut trees = trees.peekable();
    // The mesh sits at its first tree, its vertices relative to it: the entity's position is
    // what the draw-distance cutoff measures from. At the world origin, every tile more than the
    // draw radius from the origin was hidden, wherever its trees stood (the start is 2.3 km out).
    let origin = trees.peek().map(|(t, _)| t.base)?;
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
                .map(|p| (m.transform_point3(Vec3::from_array(*p)) - origin).to_array()),
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
    Some((
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_indices(Indices::U32(indices)),
        origin,
    ))
}

/// Most tree tiles started building (new or rebuilt) in one frame.
const TREE_TILES_PER_FRAME: usize = 4;
/// Most tree tiles being merged on worker threads at once.
const TREE_TILES_BUILDING: usize = 8;
/// Most built tree tiles put on screen in one frame (each is an upload to the GPU).
const TREE_TILES_SHOWN_PER_FRAME: usize = 2;
/// Tree tiles stay on screen this far past the draw radius before they are taken off.
const TREE_KEEP_MARGIN_M: f32 = 150.0;
/// Only tree tiles within this of the eye cast shadows (the shadow pass draws far fewer trees).
const TREE_SHADOW_RADIUS_M: f32 = 300.0;

/// A tree mesh, one unit tall, base at the origin: trunk and crown in vertex colours.
/// Past this, a tree tile is built from the far version of its trees (a fraction of the vertices):
/// with trees drawn to 2 km most tiles are far, and their full-detail meshes were most of the
/// upload behind the frame spikes (2026-10-24). A tile changes version 60 m past the line.
const TREE_FAR_M: f32 = 600.0;
const TREE_FAR_SLACK_M: f32 = 60.0;

/// Whether a tile at `d` metres from the eye should be the far version, given what it is now.
fn tile_far(d: f32, now_far: Option<bool>) -> bool {
    // `CB_NO_TREE_LOD=1`: every tile at full detail (perf-bench switch).
    if crate::cli::switched_off("CB_NO_TREE_LOD") {
        return false;
    }
    match now_far {
        Some(true) => d > TREE_FAR_M - TREE_FAR_SLACK_M,
        Some(false) => d > TREE_FAR_M + TREE_FAR_SLACK_M,
        None => d > TREE_FAR_M,
    }
}

fn tree_mesh(species: &str, far: bool) -> Mesh {
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
        if far { 4 } else { 7 },
        if species == "birch" { [0.85, 0.83, 0.78, 1.0] } else { bark },
    );
    // The far version: one coarse crown in the same place and size, a few dozen vertices.
    if far {
        match species {
            "oak" => b.blob(Vec3::new(0.04, 0.66, 0.03), Vec3::new(0.36, 0.32, 0.36), 3, [0.21, 0.34, 0.14, 1.0]),
            "birch" => b.blob(Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.2, 0.3, 0.2), 3, [0.36, 0.48, 0.20, 1.0]),
            _ => b.cone(Vec3::new(0.0, 0.2, 0.0), 0.2, 0.8, 5, [0.13, 0.26, 0.12, 1.0]),
        }
        return b.build();
    }
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
    let built: Vec<Mesh> = data.0.world.trees.species.iter().map(|s| tree_mesh(&s.name, false)).collect();
    let geometry = built.iter().map(TreeGeometry::of).collect();
    let geometry_far: Vec<TreeGeometry> = data
        .0
        .world
        .trees
        .species
        .iter()
        .map(|s| TreeGeometry::of(&tree_mesh(&s.name, true)))
        .collect();
    let counts: Vec<String> = data
        .0
        .world
        .trees
        .species
        .iter()
        .zip(built.iter().zip(geometry_far.iter()))
        .map(|(s, (n, f)): (_, (&Mesh, &TreeGeometry))| format!("{} {}/{}", s.name, n.count_vertices(), f.positions.len()))
        .collect();
    info!("trees: vertices a tree, full/far: {}", counts.join(", "));
    let species = built.into_iter().map(|m| meshes.add(m)).collect();
    commands.insert_resource(TreeEntities {
        species,
        geometry: Arc::new(geometry),
        geometry_far: Arc::new(geometry_far),
        material,
        ..default()
    });
}

/// Grows the trees near the pilot and every mech before they move this step, and lets go of the
/// trees far from all of them (cb-core `Forest::keep_around`; deterministic: it reads only the
/// simulation's positions).
fn keep_trees(mut live: ResMut<Live>, plan: Res<Plan>, pilot: Res<crate::pilot::PilotSim>, mechs: Query<&crate::mech::MechSim>) {
    let _t = crate::perf_hud::slow("keep_trees");
    let feet = pilot.curr.feet;
    let mut bodies = vec![Vec2::new(feet.x, feet.z)];
    bodies.extend(mechs.iter().map(|m| Vec2::new(m.curr.pos.x, m.curr.pos.z)));
    // The pilot first: only their eye sees the woods out to the draw radius (`keep_around_seen`).
    live.forest.keep_around_seen(&plan, &bodies, 1);
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
    // `CB_NO_TREES=1`: no trees drawn (perf-bench switch, to measure what they cost).
    if crate::cli::switched_off("CB_NO_TREES") {
        return;
    }
    let _t = crate::perf_hud::slow("show_trees");
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
        trees.building.remove(&key);
        // Fallen and falling trees drawn on their own go with their tile.
        let loose: Vec<u32> = trees
            .moving
            .keys()
            .copied()
            .filter(|id| trees.tile_of.get(id) == Some(&key))
            .collect();
        for id in loose {
            if let Some(e) = trees.moving.remove(&id) {
                commands.entity(e).despawn();
            }
        }
        trees.tile_of.retain(|_, k| *k != key);
    }
    // Tiles built off the main thread go on screen a couple a frame (each is a GPU upload).
    let mut done = Vec::new();
    for (key, (task, far)) in trees.building.iter_mut() {
        if done.len() >= TREE_TILES_SHOWN_PER_FRAME {
            break;
        }
        if let Some(built) = check_ready(task) {
            done.push((*key, built, *far));
        }
    }
    for (key, built, far) in done {
        trees.building.remove(&key);
        if !keep.contains(&key) {
            continue;
        }
        let shadows = live.forest.tile_rect(key).distance(eye) <= TREE_SHADOW_RADIUS_M;
        // A tree that fell while the tile was being built leaves it dirty: built again next.
        let dirty = match trees.tiles.remove(&key) {
            Some(old) => {
                for e in old.meshes {
                    commands.entity(e).despawn();
                }
                old.dirty
            }
            None => false,
        };
        let mut entities = Vec::new();
        for (mesh, origin) in built {
            let mut e = commands.spawn((
                Name::new("tree tile"),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(trees.material.clone()),
                Transform::from_translation(origin),
                VisibilityRange::abrupt(0.0, radius + TREE_KEEP_MARGIN_M),
            ));
            if !shadows {
                e.insert(NotShadowCaster);
            }
            entities.push(e.id());
        }
        trees.tiles.insert(
            key,
            TileDraw {
                meshes: entities,
                shadows,
                dirty,
                far,
            },
        );
    }
    // New tiles and changed ones, nearest first: their meshes are merged on a worker thread (the
    // merge of a tile's few hundred trees took 40-75 ms a frame on the main thread, 2026-10-24).
    let mut todo: Vec<(f32, (u32, u32))> = grown
        .into_iter()
        .filter(|g| {
            g.0 <= radius
                && !trees.building.contains_key(&g.1)
                && trees.tiles.get(&g.1).is_none_or(|t| t.dirty || t.far != tile_far(g.0, Some(t.far)))
        })
        .collect();
    todo.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let room = TREE_TILES_BUILDING.saturating_sub(trees.building.len());
    let pool = AsyncComputeTaskPool::get();
    for (d, key) in todo.into_iter().take(TREE_TILES_PER_FRAME.min(room)) {
        let Some((_, grown)) = live.forest.grown_tiles().find(|(k, _)| *k == key) else {
            continue;
        };
        if let Some(t) = trees.tiles.get_mut(&key) {
            t.dirty = false;
        }
        let species = trees.geometry.len();
        // Falling trees are drawn on their own (`redraw_trees`), not in the tile.
        let mut by_species: Vec<Vec<(cb_core::world::Tree, TreeState)>> = vec![Vec::new(); species];
        for t in grown {
            trees.tile_of.insert(t.id, key);
            if !trees.moving.contains_key(&t.id) && (t.species as usize) < species {
                by_species[t.species as usize].push((*t, live.forest.state(t.id)));
            }
        }
        let far = tile_far(d, trees.tiles.get(&key).map(|t| t.far));
        let geometry = if far { trees.geometry_far.clone() } else { trees.geometry.clone() };
        let task = pool.spawn(async move {
            geometry
                .iter()
                .zip(by_species.iter())
                .filter_map(|(g, list)| merge_trees(g, list.iter().map(|(t, st)| (t, *st))))
                .collect::<Vec<(Mesh, Vec3)>>()
        });
        trees.building.insert(key, (task, far));
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
    let _t = crate::perf_hud::slow("fell_trees");
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

/// A tree that starts falling leaves its tile's mesh (one rebuild) and is drawn on its own as it
/// falls and where it lies; it joins a tile's mesh again only when the tile is grown afresh.
fn redraw_trees(mut commands: Commands, mut live: ResMut<Live>, trees: Option<ResMut<TreeEntities>>, mut q: Query<&mut Transform>) {
    let _t = crate::perf_hud::slow("redraw_trees");
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
            // Landed: it stays drawn on its own where it lies, and its tile is not rebuilt again.
            // In a fight mechs fell about 200 trees in two minutes, and rebuilding and uploading a
            // whole tile's mesh for each landing made frames over 50 ms (2026-10-24).
            _ => match trees.moving.get(&id) {
                Some(e) => {
                    if let Ok(mut t) = q.get_mut(*e) {
                        *t = tree_transform(tree, state);
                    }
                }
                None => {
                    if let Some(tile) = trees.tiles.get_mut(&key) {
                        tile.dirty = true;
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    fn tangents(mesh: &Mesh) -> Vec<[f32; 4]> {
        match mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
            Some(VertexAttributeValues::Float32x4(t)) => t.clone(),
            _ => panic!("no tangents"),
        }
    }

    #[test]
    fn planar_tangents_match_mikktspace_on_a_box() {
        let mut a = MeshBuilder::tiled(2.4, 2.4);
        a.block(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 3.5, 7.0), [1.0; 4]);
        let mut b = MeshBuilder::tiled(2.4, 2.4);
        b.block(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 3.5, 7.0), [1.0; 4]);
        let ours = tangents(&a.build_with_tangents());
        let mut mikk = b.build();
        mikk.generate_tangents().unwrap();
        for (i, (o, m)) in ours.iter().zip(tangents(&mikk)).enumerate() {
            let (o3, m3) = (Vec3::new(o[0], o[1], o[2]), Vec3::new(m[0], m[1], m[2]));
            assert!(o3.dot(m3) > 0.99, "vertex {i}: tangent {o3} against {m3}");
            assert_eq!(o[3], m[3], "vertex {i}: handedness");
        }
    }
}
