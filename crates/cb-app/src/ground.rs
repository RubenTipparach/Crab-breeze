//! The ground as drawn (openspec change `island-world`, design section 1): the island's terrain as
//! the nodes of a quadtree `cb_core::world::lod::select` picks for the eye. A node is one mesh of
//! 64 x 64 quads (a 512 m chunk at full detail near the eye, an 8 km square at step 16 far away),
//! so the whole island is about ninety meshes with one shared material (owner, 2026-10-24: "make
//! sure to lower draw calls"). Meshes are built off the main thread as the eye moves (a few started
//! a frame, nearest first); a node stays on screen until whatever replaces it is ready, so the
//! ground never opens; skirts hide the cracks between levels. Crater tiles are drawn finer over
//! holes in full-detail chunks. Colliders (full detail, dug ground included) exist only for the
//! chunks near the eye and the loose physics bodies. The open sea floor is drawn flat past the
//! world's edge. Every height comes from the one ground query, `Heightfield`; this module only
//! presents it.

use crate::pilot::PilotCamera;
use crate::sim::{Data, PresentSet};
use crate::world::{Live, MeshBuilder, Plan};
use avian3d::prelude::{Collider, RigidBody};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use cb_core::grass::{GrassConfig, ground_cover, slope_of_normal};
use cb_core::world::lod::{self, Node};
use cb_core::world::{BIOME_COUNT, ChunkMesh, Deformation, WorldPlan};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

/// Draws the ground.
pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ground)
            .add_systems(Update, (redraw_craters, stream_ground).chain().in_set(PresentSet::Pose));
    }
}

/// Most collider builds started in one frame.
const COLLIDERS_PER_FRAME: usize = 2;
/// A chunk keeps its collider until every body is this many times the collider radius away.
const COLLIDER_RELEASE: f32 = 1.5;

/// One quadtree node's mesh.
struct NodeDraw {
    entity: Entity,
    /// Its mesh is built (and current unless `stale`).
    ready: bool,
    job: Option<Task<Mesh>>,
    /// Craters were dug in it since its mesh was started.
    stale: bool,
}

/// One chunk's collider.
#[derive(Default)]
struct ColliderSlot {
    entity: Option<Entity>,
    job: Option<Task<Option<(RigidBody, Collider)>>>,
    /// Craters were dug in it since its collider was started.
    stale: bool,
}

/// The ground's nodes, what is on screen, the colliders and the crater tiles.
#[derive(Resource)]
struct Ground {
    nodes: BTreeMap<Node, NodeDraw>,
    /// Nodes on screen now: together they cover the world once.
    drawn: BTreeSet<Node>,
    /// Nodes split at the last selection (for hysteresis).
    split: BTreeSet<Node>,
    colliders: BTreeMap<usize, ColliderSlot>,
    /// Chunks along a side.
    side: usize,
    /// Terrain cells along a chunk's side.
    cells: usize,
    tiles: HashMap<(i32, i32), Entity>,
    /// Crater tiles being meshed on a worker thread.
    crater_jobs: BTreeMap<(i32, i32), Task<Mesh>>,
    material: Handle<StandardMaterial>,
    /// The grass's tuning, for the ground's colour under it.
    grass: Arc<GrassConfig>,
}

/// How the ground takes the grass's colour (openspec change `gpu-grass`): the colour, and how far
/// toward it the ground moves where grass grows fully.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GrassTint {
    colour: Vec3,
    strength: f32,
}

impl From<&GrassConfig> for GrassTint {
    fn from(g: &GrassConfig) -> Self {
        Self {
            colour: Vec3::from_array(g.ground_tint_colour),
            strength: g.ground_tint,
        }
    }
}

/// How fully grass covers the ground at vertex `p` with normal `n`, `dug_m` changed by craters:
/// the one rule the grass grows by (`cb_core::grass::ground_cover`).
fn grass_cover(plan: &WorldPlan, grass: &GrassConfig, p: Vec3, n: Vec3, dug_m: f32) -> f32 {
    ground_cover(plan, Vec2::new(p.x, p.z), p.y, slope_of_normal(n.y), dug_m, grass)
}

/// Ground colours of the biomes, in `BIOMES` order (snow, rock, montane forest floor, temperate
/// grass, savanna, desert, beach, wetland). Stand-ins until the PBR layers of `terrain-materials`.
const BIOME_COLOURS: [[f32; 3]; BIOME_COUNT] = [
    [0.88, 0.9, 0.93],
    [0.45, 0.44, 0.42],
    [0.24, 0.32, 0.19],
    [0.30, 0.42, 0.20],
    [0.52, 0.48, 0.28],
    [0.70, 0.58, 0.40],
    [0.62, 0.57, 0.44],
    [0.27, 0.33, 0.21],
];

/// The biome weights at a drawn vertex (the one biome rule, `Biomes::weights`).
fn biome_at(plan: &WorldPlan, p: Vec3, n: Vec3) -> [f32; BIOME_COUNT] {
    plan.biomes.weights(Vec2::new(p.x, p.z), p.y, slope_of_normal(n.y))
}

/// Ground colour: the biomes' colours blended by their weights (`biome`), earth on slopes, sand
/// on the beaches, silt under the sea, mud by rivers, scorch in craters, and the grass's own colour
/// where grass covers it (`cover`, 0..1, gpu-grass: no pale soil between the blades). Vertex
/// colours stand in until the PBR layers of `terrain-materials` arrive.
#[allow(clippy::too_many_arguments)]
fn ground_colour(
    p: Vec3,
    n: Vec3,
    sea_level_m: f32,
    wet: bool,
    scorch: f32,
    cover: f32,
    tint: GrassTint,
    biome: [f32; BIOME_COUNT],
) -> [f32; 4] {
    let slope = ((1.0 - n.y).clamp(0.0, 1.0) * 6.0).min(1.0);
    let k = (p.y * 0.05).sin() * 0.04 + ((p.x * 0.013).sin() * (p.z * 0.011).cos()) * 0.03;
    let land: Vec3 = biome.iter().zip(BIOME_COLOURS).map(|(w, c)| Vec3::from(c) * *w).sum::<Vec3>() + Vec3::new(k, k, 0.0);
    let earth = Vec3::new(0.42, 0.36, 0.27);
    let mud = Vec3::new(0.27, 0.24, 0.19);
    let rock = Vec3::new(0.45, 0.44, 0.42);
    let sand = Vec3::new(0.62 + k, 0.57 + k, 0.44);
    let silt = Vec3::new(0.36, 0.34, 0.28);
    // Snow and bare rock keep their own colour on slopes; everything else shows earth.
    let soil = 1.0 - biome[0] - biome[1];
    let mut c = land.lerp(earth, slope * soil);
    if slope > 0.9 {
        c = c.lerp(rock, 0.6 * soil);
    }
    if p.y < sea_level_m - 1.5 {
        c = c.lerp(silt, 0.85);
    } else if p.y < sea_level_m + 2.5 && !wet {
        c = c.lerp(sand, 0.8 * (1.0 - slope));
    }
    if wet {
        c = c.lerp(mud, 0.8);
    }
    c = c.lerp(tint.colour, (cover * tint.strength).clamp(0.0, 1.0));
    c = c.lerp(Vec3::new(0.10, 0.08, 0.07), scorch.clamp(0.0, 1.0) * 0.85);
    c.extend(1.0).to_array()
}

fn chunk_to_mesh(chunk: ChunkMesh, colors: Vec<[f32; 4]>) -> Mesh {
    let uvs: Vec<[f32; 2]> = chunk.positions.iter().map(|p| [p.x / 4.0, p.z / 4.0]).collect();
    // The UVs are x and z, so the tangent is +x laid onto the ground; v runs along +z, which is
    // minus n x t, hence the handedness -1 (for the grass set's normal map).
    let tangents: Vec<[f32; 4]> = chunk
        .normals
        .iter()
        .map(|n| {
            let t = (Vec3::X - *n * n.x).normalize_or(Vec3::X);
            [t.x, t.y, t.z, -1.0]
        })
        .collect();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, tangents)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            chunk.positions.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            chunk.normals.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(chunk.indices))
}

/// The drawn mesh of a node, leaving out the cells `skip` holds (crater tiles; only a chunk has
/// any). Runs off the main thread.
fn node_mesh(plan: &WorldPlan, grass: &GrassConfig, node: Node, cells: usize, skip: &HashSet<(usize, usize)>) -> Mesh {
    let t = &plan.terrain;
    let step = node.step();
    let chunk = t.chunk_mesh_lod(
        node.x as usize,
        node.z as usize,
        node.cells(cells),
        step,
        lod::skirt_m(step, t.cell_m),
        |ix, iz| skip.contains(&(ix, iz)),
    );
    let sea = plan.waters.sea_level_m;
    let colors = chunk
        .positions
        .iter()
        .zip(&chunk.normals)
        .map(|(p, n)| {
            // A river's water 3 m over this ground: its bank or bed.
            let river = plan
                .waters
                .water_at(Vec2::new(p.x, p.z), p.y - 3.0)
                .is_some_and(|w| w.surface_m > sea);
            let cover = grass_cover(plan, grass, *p, *n, 0.0);
            ground_colour(*p, *n, sea, river, 0.0, cover, grass.into(), biome_at(plan, *p, *n))
        })
        .collect();
    chunk_to_mesh(chunk, colors)
}

/// The full-detail collider of chunk `(cx, cz)`, with the dug ground's `deltas` (one per vertex,
/// row by row) added when craters were dug in it. Runs off the main thread.
fn chunk_collider(plan: &WorldPlan, cx: usize, cz: usize, cells: usize, deltas: Option<Vec<f32>>) -> Option<(RigidBody, Collider)> {
    let mut chunk = plan.terrain.chunk_mesh_lod(cx, cz, cells, 1, 0.0, |_, _| false);
    if let Some(d) = deltas {
        for (p, dy) in chunk.positions.iter_mut().zip(d) {
            p.y += dy;
        }
    }
    crate::physics::ground_collider(&chunk.positions, &chunk.indices)
}

/// The terrain cells of a chunk under crater tiles (drawn by the finer tile instead).
fn crater_cells(plan: &WorldPlan, dug: &Deformation, cx: usize, cz: usize, cells: usize) -> HashSet<(usize, usize)> {
    let t = &plan.terrain;
    let mut out = HashSet::new();
    for iz in cz * cells..(cz + 1) * cells {
        for ix in cx * cells..(cx + 1) * cells {
            let c = t.sample_xz(ix, iz) + Vec2::splat(t.cell_m * 0.5);
            if dug.has_tile(dug.tile_of(c)) {
                out.insert((ix, iz));
            }
        }
    }
    out
}

/// The dug ground's change at each full-detail vertex of a chunk, row by row.
fn crater_deltas(plan: &WorldPlan, dug: &Deformation, cx: usize, cz: usize, cells: usize) -> Vec<f32> {
    let t = &plan.terrain;
    let mut out = Vec::with_capacity((cells + 1) * (cells + 1));
    for iz in cz * cells..=(cz + 1) * cells {
        for ix in cx * cells..=(cx + 1) * cells {
            out.push(dug.delta_at(t.sample_xz(ix, iz)));
        }
    }
    out
}

/// The chunk `(cx, cz)` holding a world point, if inside the world.
fn chunk_of(plan: &WorldPlan, cells: usize, side: usize, p: Vec2) -> Option<(usize, usize)> {
    let t = &plan.terrain;
    let chunk_m = cells as f32 * t.cell_m;
    let (fx, fz) = ((p.x - t.origin_m) / chunk_m, (p.y - t.origin_m) / chunk_m);
    (fx >= 0.0 && fz >= 0.0 && (fx as usize) < side && (fz as usize) < side).then_some((fx as usize, fz as usize))
}

/// Chunks with crater tiles in them.
fn crater_chunks(plan: &WorldPlan, dug: &Deformation, cells: usize, side: usize) -> BTreeSet<(usize, usize)> {
    dug.tile_keys()
        .into_iter()
        .filter_map(|key| chunk_of(plan, cells, side, dug.sample_xz(key, 0, 0) + Vec2::splat(1.0)))
        .collect()
}

/// Chunk indices whose ground lies within `r` of `p`.
fn chunks_near(plan: &WorldPlan, cells: usize, side: usize, p: Vec2, r: f32, out: &mut BTreeSet<usize>) {
    let t = &plan.terrain;
    let chunk_m = cells as f32 * t.cell_m;
    let at = |v: f32| (((v - t.origin_m) / chunk_m).floor().max(0.0) as usize).min(side - 1);
    if !p.is_finite() {
        return;
    }
    for cz in at(p.y - r)..=at(p.y + r) {
        for cx in at(p.x - r)..=at(p.x + r) {
            if lod::chunk_rect(t, cells, cx, cz).distance(p) <= r {
                out.insert(cz * side + cx);
            }
        }
    }
}

fn spawn_ground(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    tex: Res<crate::textures::Textures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let cfg = &data.0.world;
    let grass = Arc::new(data.0.grass.clone());
    let t = &plan.terrain;
    let cells = cfg.terrain_lod.chunk_cells as usize;
    let side = (t.n - 1).div_ceil(cells);
    // One material for every node, so they all share a pipeline and batch.
    // The ground's grass: the CC0 set (ambientCG Grass001, assets/textures/ground/grass001) with
    // its normal and packed occlusion-roughness map; the vertex colours tint it per surface. The
    // stand-in grass texture if the set cannot be read.
    let root = crate::cli::repo_root().join("assets");
    let mut tiling = |path: &str, srgb: bool| crate::pbr_set::load_tiling(&root, path, srgb).map(|i| images.add(i));
    let albedo = tiling("textures/ground/grass001/albedo.jpg", true);
    let normal = tiling("textures/ground/grass001/normal_gl.png", false);
    let orm = tiling("textures/ground/grass001/orm.png", false);
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(albedo.unwrap_or_else(|| tex.grass.clone())),
        normal_map_texture: normal,
        occlusion_texture: orm.clone(),
        metallic_roughness_texture: orm,
        metallic: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    // The first frame's eye: the pilot's, at the start.
    let s = plan.start.pilot_xz;
    let eye = Vec3::new(s.x, t.height_at(s.x, s.y) + 1.7, s.y);
    let sea = plan.waters.sea_level_m;
    let started = std::time::Instant::now();
    let (leaves, split) = lod::select(t, cells, side, eye, sea, &cfg.terrain_lod, &BTreeSet::new(), &BTreeSet::new());
    let none = HashSet::new();
    let mut nodes = BTreeMap::new();
    let mut triangles = 0;
    for n in &leaves {
        triangles += lod::chunk_triangles(n.cells(cells), n.step());
        let e = commands
            .spawn((
                Name::new(format!("terrain {}:{},{}", n.level, n.x, n.z)),
                Mesh3d(meshes.add(node_mesh(&plan, &grass, *n, cells, &none))),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        nodes.insert(
            *n,
            NodeDraw {
                entity: e,
                ready: true,
                job: None,
                stale: false,
            },
        );
    }
    let mut near = BTreeSet::new();
    chunks_near(&plan, cells, side, s, cfg.terrain_lod.collider_radius_m, &mut near);
    let mut colliders = BTreeMap::new();
    for &i in &near {
        let entity = chunk_collider(&plan, i % side, i / side, cells, None)
            .map(|c| commands.spawn((Name::new(format!("ground collider {i}")), c)).id());
        colliders.insert(i, ColliderSlot { entity, ..default() });
    }
    info!(
        "ground: {} meshes, {triangles} triangles, {} chunk colliders, built in {:.0} ms",
        leaves.len(),
        near.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    spawn_far_floor(&mut commands, &plan, cfg, &mut meshes, material.clone());
    commands.insert_resource(Ground {
        drawn: leaves.into_iter().collect(),
        nodes,
        split,
        colliders,
        side,
        cells,
        tiles: HashMap::new(),
        crater_jobs: BTreeMap::new(),
        material,
        grass,
    });
}

/// The open sea floor past the world's edge, out to the far plane: four flat strips round the
/// world's square at the floor's depth, which the terrain meets exactly at the edge (the floor's
/// noise fades out before it).
fn spawn_far_floor(
    commands: &mut Commands,
    plan: &WorldPlan,
    cfg: &cb_core::world::WorldConfig,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    let half = cfg.world_size_m * 0.5;
    let out = half + cfg.view.far_m;
    let y = cfg.island.sea_level_m - cfg.island.sea_depth_m;
    let colour = ground_colour(
        Vec3::new(0.0, y, 0.0),
        Vec3::Y,
        plan.waters.sea_level_m,
        false,
        0.0,
        0.0,
        GrassTint::default(),
        biome_at(plan, Vec3::new(0.0, y, 0.0), Vec3::Y),
    );
    let mut b = MeshBuilder::tiled(64.0, 64.0);
    // Each strip (x0, z0)-(x1, z1), wound to face up.
    for (x0, z0, x1, z1) in [
        (-out, -out, out, -half),
        (-out, half, out, out),
        (-out, -half, -half, half),
        (half, -half, out, half),
    ] {
        b.quad(
            [
                Vec3::new(x0, y, z0),
                Vec3::new(x0, y, z1),
                Vec3::new(x1, y, z1),
                Vec3::new(x1, y, z0),
            ],
            colour,
        );
    }
    commands.spawn((
        Name::new("sea floor past the edge"),
        Mesh3d(meshes.add(b.build())),
        MeshMaterial3d(material),
    ));
}

/// Rebuilds the ground where craters were dug: the finer tile on a worker thread (meshing a tile
/// on the main thread took 56-260 ms a crater, 2026-10-24) and on screen when it is ready; its
/// chunk (mesh and collider) through the stream.
fn redraw_craters(
    mut commands: Commands,
    mut live: ResMut<Live>,
    plan: Res<Plan>,
    ground: Option<ResMut<Ground>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let _t = crate::perf_hud::slow("redraw_craters");
    let Some(mut ground) = ground else { return };
    let ground = &mut *ground;
    // Finished tiles on screen.
    let ready: Vec<((i32, i32), Mesh)> = ground
        .crater_jobs
        .iter_mut()
        .filter_map(|(k, task)| check_ready(task).map(|m| (*k, m)))
        .collect();
    for (key, mesh) in ready {
        ground.crater_jobs.remove(&key);
        let mesh = meshes.add(mesh);
        match ground.tiles.get(&key) {
            Some(e) => {
                commands.entity(*e).insert(Mesh3d(mesh));
            }
            None => {
                let e = commands
                    .spawn((
                        Name::new(format!("crater tile {key:?}")),
                        Mesh3d(mesh),
                        MeshMaterial3d(ground.material.clone()),
                    ))
                    .id();
                ground.tiles.insert(key, e);
            }
        }
    }
    if live.dirty_tiles.is_empty() {
        return;
    }
    let mut keys = std::mem::take(&mut live.dirty_tiles);
    keys.sort_unstable();
    keys.dedup();
    let (cells, side) = (ground.cells, ground.side);
    let pool = AsyncComputeTaskPool::get();
    for key in keys {
        if let Some((cx, cz)) = chunk_of(&plan, cells, side, live.dug.sample_xz(key, 0, 0) + Vec2::splat(1.0)) {
            let chunk = Node {
                level: 0,
                x: cx as u32,
                z: cz as u32,
            };
            if let Some(n) = ground.nodes.get_mut(&chunk) {
                n.stale = true;
            }
            if let Some(c) = ground.colliders.get_mut(&(cz * side + cx)) {
                c.stale = true;
            }
        }
        // A tile dug again while it is being meshed starts over from the newer ground (the old
        // job is dropped, which cancels it).
        let dug = live.dug.around(key);
        let world = plan.0.clone();
        let grass = ground.grass.clone();
        let task = pool.spawn(async move {
            let tile = dug.tile_mesh(&world, key);
            let sea = world.waters.sea_level_m;
            let colors = tile
                .positions
                .iter()
                .zip(&tile.normals)
                .enumerate()
                .map(|(k, (p, n))| {
                    let (i, j) = (k % cb_core::world::deform::TILE_N, k / cb_core::world::deform::TILE_N);
                    let cover = grass_cover(&world, &grass, *p, *n, dug.delta_at(Vec2::new(p.x, p.z)));
                    ground_colour(
                        *p,
                        *n,
                        sea,
                        false,
                        dug.scorch_at(key, i, j),
                        cover,
                        (&*grass).into(),
                        biome_at(&world, *p, *n),
                    )
                })
                .collect();
            chunk_to_mesh(tile, colors)
        });
        ground.crater_jobs.insert(key, task);
    }
}

/// The nodes to keep on screen: the wanted leaves where they are ready, and otherwise whatever is
/// on screen there now, so the ground never opens and no two drawn nodes overlap.
fn next_drawn(drawn: &BTreeSet<Node>, leaves: &[Node], ready: impl Fn(&Node) -> bool) -> BTreeSet<Node> {
    let mut out = BTreeSet::new();
    let mut kept_ancestors: Vec<Node> = Vec::new();
    for d in drawn {
        if leaves.contains(d) {
            out.insert(*d);
        } else if let Some(l) = leaves.iter().find(|l| l.contains(d)) {
            // A coarser node is wanted over it: keep it until that one is ready.
            if !ready(l) {
                out.insert(*d);
            }
        } else {
            // Finer nodes are wanted inside it: keep it until all of them are ready.
            let inside: Vec<&Node> = leaves.iter().filter(|l| d.contains(l)).collect();
            if !inside.iter().all(|l| ready(l)) {
                out.insert(*d);
                kept_ancestors.push(*d);
            }
        }
    }
    for l in leaves {
        let under_kept = kept_ancestors.iter().any(|a| a.contains(l));
        let over_kept = out.iter().any(|o| o != l && l.contains(o));
        if ready(l) && !under_kept && !over_kept {
            out.insert(*l);
        }
    }
    out
}

/// Keeps the ground drawn at the detail the eye calls for, and colliders on the chunks near the eye
/// and the loose physics bodies. Builds run off the main thread, a few started a frame, nearest
/// first, so moving fast never stalls a frame.
fn stream_ground(
    mut commands: Commands,
    mut ground: ResMut<Ground>,
    plan: Res<Plan>,
    live: Res<Live>,
    data: Res<Data>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    bodies: Query<(&GlobalTransform, &RigidBody)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let _t = crate::perf_hud::slow("stream_ground");
    let cfg = &data.0.world.terrain_lod;
    let sea = plan.waters.sea_level_m;
    let eye = camera.translation();
    let g = &mut *ground;
    let (side, cells) = (g.side, g.cells);
    let pool = AsyncComputeTaskPool::get();

    // Take what has finished.
    for n in g.nodes.values_mut() {
        if let Some(mesh) = n.job.as_mut().and_then(check_ready) {
            commands.entity(n.entity).insert(Mesh3d(meshes.add(mesh)));
            n.ready = true;
            n.job = None;
        }
    }
    for c in g.colliders.values_mut() {
        if let Some(built) = c.job.as_mut().and_then(check_ready) {
            if let Some(e) = c.entity.take() {
                commands.entity(e).despawn();
            }
            c.entity = built.map(|b| commands.spawn((Name::new("ground collider"), b)).id());
            c.job = None;
        }
    }

    // What the eye wants drawn.
    let craters = crater_chunks(&plan, &live.dug, cells, side);
    let (leaves, split) = lod::select(&plan.terrain, cells, side, eye, sea, cfg, &g.split, &craters);
    g.split = split;

    // Build the wanted nodes that are missing or dug, nearest first.
    let mut builds: Vec<(f32, Node)> = Vec::new();
    for l in &leaves {
        let n = g.nodes.entry(*l).or_insert_with(|| NodeDraw {
            entity: commands
                .spawn((
                    Name::new(format!("terrain {}:{},{}", l.level, l.x, l.z)),
                    MeshMaterial3d(g.material.clone()),
                    Visibility::Hidden,
                ))
                .id(),
            ready: false,
            job: None,
            stale: false,
        });
        if n.job.is_none() && (!n.ready || n.stale) {
            builds.push((lod::eye_distance(&l.rect(&plan.terrain, cells), eye, sea), *l));
        }
    }
    builds.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, node) in builds.into_iter().take(cfg.builds_per_frame as usize) {
        let skip = if node.level == 0 && craters.contains(&(node.x as usize, node.z as usize)) {
            crater_cells(&plan, &live.dug, node.x as usize, node.z as usize, cells)
        } else {
            HashSet::new()
        };
        let p = Arc::clone(&plan.0);
        if let Some(n) = g.nodes.get_mut(&node) {
            let grass = Arc::clone(&g.grass);
            n.job = Some(pool.spawn(async move { node_mesh(&p, &grass, node, cells, &skip) }));
            n.stale = false;
        }
    }

    // Swap what is on screen, and let go of nodes neither drawn nor wanted.
    let nodes = &g.nodes;
    let drawn = next_drawn(&g.drawn, &leaves, |n| nodes.get(n).is_some_and(|d| d.ready));
    for n in drawn.difference(&g.drawn) {
        commands.entity(g.nodes[n].entity).insert(Visibility::Inherited);
    }
    for n in g.drawn.difference(&drawn) {
        commands.entity(g.nodes[n].entity).insert(Visibility::Hidden);
    }
    let wanted: BTreeSet<Node> = leaves.iter().copied().collect();
    let gone: Vec<Node> = g
        .nodes
        .keys()
        .filter(|n| !drawn.contains(n) && !wanted.contains(n))
        .copied()
        .collect();
    for n in gone {
        if let Some(d) = g.nodes.remove(&n) {
            commands.entity(d.entity).despawn();
        }
    }
    g.drawn = drawn;

    // Colliders: near the eye and every loose body; let go well beyond.
    let mut need = BTreeSet::new();
    let mut keep = BTreeSet::new();
    let r = cfg.collider_radius_m;
    let centres = std::iter::once(eye).chain(
        bodies
            .iter()
            .filter(|(_, rb)| **rb == RigidBody::Dynamic)
            .map(|(t, _)| t.translation()),
    );
    for c in centres {
        let xz = Vec2::new(c.x, c.z);
        chunks_near(&plan, cells, side, xz, r, &mut need);
        chunks_near(&plan, cells, side, xz, r * COLLIDER_RELEASE, &mut keep);
    }
    let mut started = 0;
    for &i in &need {
        let slot = g.colliders.entry(i).or_default();
        let missing = slot.entity.is_none() && slot.job.is_none();
        if started >= COLLIDERS_PER_FRAME || slot.job.is_some() || !(missing || slot.stale) {
            continue;
        }
        let (cx, cz) = (i % side, i / side);
        let deltas = craters.contains(&(cx, cz)).then(|| crater_deltas(&plan, &live.dug, cx, cz, cells));
        let p = Arc::clone(&plan.0);
        slot.job = Some(pool.spawn(async move { chunk_collider(&p, cx, cz, cells, deltas) }));
        slot.stale = false;
        started += 1;
    }
    let gone: Vec<usize> = g.colliders.keys().filter(|i| !keep.contains(i)).copied().collect();
    for i in gone {
        if let Some(slot) = g.colliders.remove(&i)
            && let Some(e) = slot.entity
        {
            commands.entity(e).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Temperate ground alone.
    const TEMPERATE: [f32; BIOME_COUNT] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];

    fn n(level: u32, x: u32, z: u32) -> Node {
        Node { level, x, z }
    }

    #[test]
    fn ground_under_full_grass_takes_the_grass_tint_and_bare_ground_keeps_its_own() {
        let grass = GrassConfig::default();
        let tint = GrassTint::from(&grass);
        let p = Vec3::new(0.0, 20.0, 0.0);
        let bare = ground_colour(p, Vec3::Y, 0.0, false, 0.0, 0.0, tint, TEMPERATE);
        let full = ground_colour(p, Vec3::Y, 0.0, false, 0.0, 1.0, tint, TEMPERATE);
        let want = Vec3::new(bare[0], bare[1], bare[2]).lerp(tint.colour, grass.ground_tint);
        assert!((Vec3::new(full[0], full[1], full[2]) - want).length() < 1e-5);
        assert_ne!(bare, full);
        let none = ground_colour(p, Vec3::Y, 0.0, false, 0.0, 1.0, GrassTint::default(), TEMPERATE);
        assert_eq!(none, bare, "no tint strength, no change");
    }

    #[test]
    fn a_split_waits_until_every_child_is_ready() {
        let drawn = BTreeSet::from([n(1, 0, 0)]);
        let leaves = n(1, 0, 0).children();
        let some = |m: &Node| *m == leaves[0];
        assert_eq!(
            next_drawn(&drawn, &leaves, some),
            drawn,
            "the parent stays until all four are built"
        );
        let all = |_: &Node| true;
        assert_eq!(next_drawn(&drawn, &leaves, all), leaves.iter().copied().collect());
    }

    #[test]
    fn a_merge_waits_until_the_parent_is_ready() {
        let children: BTreeSet<Node> = n(1, 0, 0).children().into_iter().collect();
        let leaves = vec![n(1, 0, 0)];
        assert_eq!(next_drawn(&children, &leaves, |_| false), children);
        assert_eq!(next_drawn(&children, &leaves, |_| true), BTreeSet::from([n(1, 0, 0)]));
    }
}
