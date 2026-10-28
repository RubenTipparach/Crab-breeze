//! Grass, all of it on the GPU (openspec change `gpu-grass`; owner, 2026-10-24: "Add detailed
//! grass, make sure it's all gpu grass"). No blade is an entity, a transform or a row of CPU
//! instance data: the vertex shader (`assets/shaders/grass.wesl`) builds every blade from its index.
//!
//! What lives here is the adapter: square tiles round the eye (the rules are `cb_core::grass`),
//! each one entity drawing one shared blade mesh per level of detail with one material, so Bevy
//! batches every tile of a level into one instanced draw. A tile's transform is its corner and its
//! `MeshTag` is its slot in one storage buffer that holds every tile's ground heights (the
//! terrain's lattice, read through the one height query) and where grass grows; slots are baked off
//! the main thread, a few started a frame, nearest first. A second small buffer holds the bodies
//! pressing the grass flat (the pilot's and the mechs' feet), rewritten each frame. Grass casts no
//! shadow, receives them, and is seen only by the pilot's eyes (not the screens' cube cameras).
//! `CB_NO_GRASS=1` leaves the plugin out (`main.rs`), so the benchmark can compare.

use crate::mech::MechRig;
use crate::pilot::{PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, RenderLayers};
use bevy::mesh::{Indices, MeshTag, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use cb_core::grass::{BladeMesh, CrushRect, GrassConfig, GrassLayout, MAX_LODS, MAX_PUSHERS, TileBake, TileGrid, TileKey, bake_tile};
use cb_core::pilot::PilotMode;
use cb_core::world::Ground;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The grass shader, loaded from `assets/`.
const SHADER: &str = "shaders/grass.wesl";

/// The render layer grass is drawn on: the pilot's eyes see it, the screens' cube cameras (which
/// only light the cabin) do not.
pub const GRASS_LAYER: usize = 2;

/// `GrassLook` in `grass.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct GrassUniform {
    blade: Vec4,
    bend: Vec4,
    shade: Vec4,
    root: Vec4,
    tip: Vec4,
    dry: Vec4,
    wind: Vec4,
    gust: Vec4,
    clump: Vec4,
    ao: Vec4,
    hue: Vec4,
    glow: Vec4,
    gloss: Vec4,
    far: Vec4,
    gust2: Vec4,
    broad: Vec4,
    tile: Vec4,
    grid: UVec4,
    fade: Vec4,
    parched: Vec4,
    lods: [Vec4; MAX_LODS],
    next: [Vec4; MAX_LODS],
}

impl GrassUniform {
    /// The uniform for `cfg`, its tiles laid out as `layout` on a terrain of `cell_m` cells.
    pub fn new(cfg: &GrassConfig, layout: &GrassLayout, cell_m: f32) -> Self {
        let heading = cfg.wind_heading_deg.to_radians();
        let mut lods = [Vec4::ZERO; MAX_LODS];
        let mut next = [Vec4::ZERO; MAX_LODS];
        for (i, l) in cfg.lods.iter().enumerate() {
            lods[i] = Vec4::new(l.stride as f32, l.segments as f32, l.width_scale, l.until_m);
            if let Some(n) = cfg.lods.get(i + 1) {
                next[i] = Vec4::new(n.stride as f32, n.width_scale, cfg.crossfade_m, 0.0);
            }
        }
        let c = |v: [f32; 3]| Vec3::from_array(v).extend(0.0);
        Self {
            blade: Vec4::new(cfg.height_m[0], cfg.height_m[1], cfg.width_m, cfg.taper),
            bend: Vec4::new(cfg.lean_max, cfg.stiffness, cfg.mid_height, cfg.edge_thicken),
            shade: Vec4::new(cfg.normal_round, cfg.root_light, cfg.roughness, cfg.dry_share),
            root: c(cfg.root_colour),
            tip: c(cfg.tip_colour),
            dry: c(cfg.dry_colour),
            wind: Vec4::new(heading.cos(), heading.sin(), cfg.wind_speed_m_s, cfg.wind_push),
            gust: Vec4::new(cfg.gust_per_m, cfg.gust_share, cfg.sway_hz, cfg.sway),
            clump: Vec4::new(cfg.droop, cfg.clump_tall_share, cfg.clump_tall_scale, cfg.clump_edge_shrink),
            ao: Vec4::new(cfg.ao_dense, cfg.ao_height_share, cfg.reflectance, cfg.hue_variety),
            hue: Vec3::from_array(cfg.hue_colour).extend(cfg.tip_lighten),
            glow: Vec3::from_array(cfg.translucent_colour).extend(cfg.translucency),
            gloss: Vec4::new(cfg.translucency_power, cfg.far_roughness, cfg.gloss_fade_m[0], cfg.gloss_fade_m[1]),
            far: Vec4::new(cfg.normal_up_m[0], cfg.normal_up_m[1], 0.0, 0.0),
            gust2: Vec4::new(cfg.gust_sharpness, cfg.gust_brighten, cfg.flutter_hz, cfg.flutter),
            broad: Vec4::new(cfg.broad_wavelength_m, cfg.broad_hz, cfg.broad_push, 0.0),
            tile: Vec4::new(cfg.tile_m, cfg.blade_spacing_m, cfg.density_cell_m, cell_m),
            grid: UVec4::new(
                cfg.cells_per_side(),
                layout.density_per_side,
                layout.heights_per_side,
                layout.words_per_slot,
            ),
            fade: Vec4::new(cfg.radius_m, cfg.fade_m, cfg.clump_m, cfg.clump_strength),
            parched: Vec3::from_array(cfg.parched_colour).extend(0.0),
            lods,
            next,
        }
    }
}

/// The one grass material: every tile of every level (CLAUDE.md 6.1), so they share one bind
/// group and batch.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct GrassMaterial {
    /// The look, as the shader reads it.
    #[uniform(0)]
    pub look: GrassUniform,
    /// Every tile's slot: heights, then where grass grows.
    #[storage(1, read_only)]
    pub tiles: Handle<ShaderBuffer>,
    /// Bodies pressing the grass flat, `MAX_PUSHERS` of them.
    #[storage(2, read_only)]
    pub pushers: Handle<ShaderBuffer>,
}

impl Material for GrassMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    // Not in the depth prepass (its default vertex shader would not build a blade); the main pass
    // depth-tests against the prepass as it draws.
    fn enable_prepass() -> bool {
        false
    }

    // Grass casts no shadow (Ghost of Tsushima's choice too: cheap, and barely seen).
    fn enable_shadows() -> bool {
        false
    }

    // A blade is a strip seen from both sides.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Draws the grass.
pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GrassMaterial>::default())
            .add_world_spawn(setup)
            .add_game_systems(Update, (show_to_the_pilot, (stream_tiles, press).chain().after(PresentSet::Camera)));
    }
}

/// One tile's state.
struct TileDraw {
    /// Its bake, once done (`None` while baking).
    bake: Option<TileBake>,
    job: Option<Task<TileBake>>,
    /// Its slot in the tile buffer, while it has grass.
    slot: Option<u32>,
    /// Its entity, while it has grass.
    entity: Option<Entity>,
    /// The level it is drawn at.
    lod: usize,
}

/// The grass tiles, the buffer they are packed into and what draws them.
#[derive(Resource)]
struct Grass {
    cfg: Arc<GrassConfig>,
    grid: TileGrid,
    layout: GrassLayout,
    tiles: BTreeMap<TileKey, TileDraw>,
    /// Free slots, the highest last.
    free: Vec<u32>,
    /// The tile buffer as the CPU writes it (the GPU copy is rewritten when it changes).
    words: Vec<u32>,
    dirty: bool,
    meshes: Vec<Handle<Mesh>>,
    material: Handle<GrassMaterial>,
    tile_buffer: Handle<ShaderBuffer>,
    push_buffer: Handle<ShaderBuffer>,
    /// Blasts already masked out of the grass.
    blasts_seen: usize,
    /// Where mech prints and skate grooves crush the grass now (ground-marks), and the version of
    /// that set already packed.
    crush: Vec<CrushRect>,
    crush_version: u64,
}

/// The Bevy mesh of a level's blades: one position per vertex holding (blade, vertex, level).
fn blade_mesh(cfg: &GrassConfig, lod: usize) -> Mesh {
    let m = BladeMesh::new(cfg, lod);
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.vertices)
        .with_inserted_indices(Indices::U32(m.indices))
}

fn setup(
    mut commands: Commands,
    data: Res<Data>,
    plan: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<GrassMaterial>>,
) {
    let cfg = Arc::new(data.0.grass.clone());
    let t = &plan.terrain;
    let grid = TileGrid::new(t.origin_m, data.0.world.world_size_m, cfg.tile_m);
    let layout = GrassLayout::new(&cfg, t.cell_m);
    let slots = grid.capacity(cfg.radius_m + cfg.release_m);
    let words = vec![0u32; (slots * layout.words_per_slot) as usize];
    let tile_buffer = buffers.add(ShaderBuffer::new(words.clone(), RenderAssetUsages::RENDER_WORLD));
    let push_buffer = buffers.add(ShaderBuffer::new(vec![[0f32; 4]; MAX_PUSHERS], RenderAssetUsages::RENDER_WORLD));
    let material = materials.add(GrassMaterial {
        look: GrassUniform::new(&cfg, &layout, t.cell_m),
        tiles: tile_buffer.clone(),
        pushers: push_buffer.clone(),
    });
    let mut handles = Vec::new();
    for lod in 0..cfg.lods.len() {
        let (verts, tris) = cb_core::grass::blade_counts(cfg.lods[lod].segments);
        let blades = cfg.blades_per_tile(lod);
        info!(
            "grass level {lod}: {blades} blades a tile ({:.1} a square metre), {} vertices, {} triangles",
            cfg.blades_per_m2(lod),
            blades * verts,
            blades * tris
        );
        handles.push(meshes.add(blade_mesh(&cfg, lod)));
    }
    info!(
        "grass: {slots} tile slots of {} words ({} KiB)",
        layout.words_per_slot,
        words.len() * 4 / 1024
    );
    commands.insert_resource(Grass {
        cfg,
        grid,
        layout,
        tiles: BTreeMap::new(),
        free: (0..slots).rev().collect(),
        words,
        dirty: false,
        meshes: handles,
        material,
        tile_buffer,
        push_buffer,
        blasts_seen: 0,
        crush: Vec::new(),
        crush_version: 0,
    });
}

/// The pilot's eyes see the grass layer as well as the world's.
fn show_to_the_pilot(mut commands: Commands, cameras: Query<Entity, Added<PilotCamera>>) {
    for e in &cameras {
        commands.entity(e).insert(RenderLayers::from_layers(&[0, GRASS_LAYER]));
    }
}

/// The local bounds of a tile's blades.
fn tile_aabb(bake: &TileBake, cfg: &GrassConfig) -> Aabb {
    Aabb::from_min_max(
        Vec3::new(0.0, bake.ground_m[0] - 0.1, 0.0),
        Vec3::new(cfg.tile_m, bake.ground_m[1] + cfg.tallest_m() + 0.1, cfg.tile_m),
    )
}

impl Grass {
    /// Puts a baked tile on screen (or takes it off, when nothing grows in it): its slot written,
    /// its entity spawned or updated.
    fn place(&mut self, commands: &mut Commands, key: TileKey) {
        let Some(tile) = self.tiles.get_mut(&key) else { return };
        let Some(bake) = &tile.bake else { return };
        if bake.bare() {
            if let Some(e) = tile.entity.take() {
                commands.entity(e).despawn();
            }
            if let Some(s) = tile.slot.take() {
                self.free.push(s);
            }
            return;
        }
        if tile.slot.is_none() {
            // Out of slots: the tile waits (the capacity covers every tile in reach).
            let Some(s) = self.free.pop() else { return };
            tile.slot = Some(s);
        }
        let Some(slot) = tile.slot else { return };
        let w = self.layout.words_per_slot as usize;
        let rect = self.grid.rect(key);
        let crush: Vec<CrushRect> = self.crush.iter().filter(|c| c.touches(&rect)).copied().collect();
        bake.pack_crushed(
            &mut self.words[slot as usize * w..(slot as usize + 1) * w],
            &crush,
            rect.min,
            &self.cfg,
        );
        self.dirty = true;
        let aabb = tile_aabb(bake, &self.cfg);
        match tile.entity {
            Some(e) => {
                commands.entity(e).insert(aabb);
            }
            None => {
                let corner = self.grid.rect(key).min;
                tile.entity = Some(
                    commands
                        .spawn((
                            Name::new(format!("grass {}:{}", key.0, key.1)),
                            Mesh3d(self.meshes[tile.lod].clone()),
                            MeshMaterial3d(self.material.clone()),
                            Transform::from_xyz(corner.x, 0.0, corner.y),
                            MeshTag::new(slot),
                            aabb,
                            NoAutoAabb,
                            RenderLayers::layer(GRASS_LAYER),
                        ))
                        .id(),
                );
            }
        }
    }

    /// Lets go of a tile: its entity, its slot and any bake in flight.
    fn release(&mut self, commands: &mut Commands, key: TileKey) {
        if let Some(t) = self.tiles.remove(&key) {
            if let Some(e) = t.entity {
                commands.entity(e).despawn();
            }
            if let Some(s) = t.slot {
                self.free.push(s);
            }
        }
    }
}

/// Keeps grass on the tiles near the eye at the level each calls for: takes finished bakes, masks
/// new craters out, starts a few bakes nearest first, lets far tiles go, and rewrites the tile
/// buffer when anything in it changed.
#[allow(clippy::too_many_arguments)]
fn stream_tiles(
    mut commands: Commands,
    grass: Option<ResMut<Grass>>,
    crush: Option<Res<crate::ground_marks::PrintCrush>>,
    plan: Res<Plan>,
    live: Res<Live>,
    data: Res<Data>,
    camera: Single<&Transform, With<PilotCamera>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(mut grass) = grass else { return };
    let g = &mut *grass;
    let eye = camera.translation;
    let ground = plan.height_at(eye.x, eye.z);
    let cfg = Arc::clone(&g.cfg);

    // Finished bakes, with today's craters masked out.
    let mut done = Vec::new();
    for (key, t) in &mut g.tiles {
        if let Some(mut bake) = t.job.as_mut().and_then(check_ready) {
            bake.mask_craters(&live.dug, g.grid.rect(*key).min, &cfg);
            t.bake = Some(bake);
            t.job = None;
            done.push(*key);
        }
    }
    for key in done {
        g.place(&mut commands, key);
    }

    // New craters clear the grass they reach.
    let blasts = &live.dug.blasts;
    for b in blasts.iter().skip(g.blasts_seen) {
        let reach = data.0.world.craters.reach_m(b.energy_mj);
        let hit: Vec<TileKey> = g
            .tiles
            .keys()
            .filter(|k| g.grid.rect(**k).distance(b.centre) <= reach)
            .copied()
            .collect();
        for key in hit {
            let corner = g.grid.rect(key).min;
            if let Some(bake) = g.tiles.get_mut(&key).and_then(|t| t.bake.as_mut()) {
                bake.mask_craters(&live.dug, corner, &cfg);
                g.place(&mut commands, key);
            }
        }
    }
    g.blasts_seen = blasts.len();

    // Prints and grooves laid or faded since: repack the tiles the old or the new set touches.
    if let Some(c) = crush
        && c.version != g.crush_version
    {
        let touched = |rects: &[CrushRect], g: &Grass| -> std::collections::BTreeSet<TileKey> {
            g.tiles
                .keys()
                .filter(|k| {
                    let r = g.grid.rect(**k);
                    rects.iter().any(|c| c.touches(&r))
                })
                .copied()
                .collect()
        };
        let mut redo = touched(&g.crush, g);
        redo.extend(touched(&c.rects, g));
        g.crush.clone_from(&c.rects);
        g.crush_version = c.version;
        for key in redo {
            g.place(&mut commands, key);
        }
    }

    // What the eye wants, its level, and the bakes to start.
    let wanted = g.grid.within(eye, ground, cfg.radius_m);
    let pool = AsyncComputeTaskPool::get();
    let mut started = 0;
    for (key, distance) in &wanted {
        let t = g.tiles.entry(*key).or_insert_with(|| TileDraw {
            bake: None,
            job: None,
            slot: None,
            entity: None,
            lod: cfg.lod_for(*distance, None),
        });
        let lod = cfg.lod_for(*distance, Some(t.lod));
        if lod != t.lod {
            t.lod = lod;
            if let Some(e) = t.entity {
                commands.entity(e).insert(Mesh3d(g.meshes[lod].clone()));
            }
        }
        if t.bake.is_none() && t.job.is_none() && started < cfg.bakes_per_frame {
            let (p, c, grid, key) = (Arc::clone(&plan.0), Arc::clone(&cfg), g.grid, *key);
            t.job = Some(pool.spawn(async move { bake_tile(&p, &grid, key, &c) }));
            started += 1;
        }
    }

    // Let go of tiles well past the radius.
    let keep: std::collections::BTreeSet<TileKey> = g
        .grid
        .within(eye, ground, cfg.radius_m + cfg.release_m)
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    let gone: Vec<TileKey> = g.tiles.keys().filter(|k| !keep.contains(k)).copied().collect();
    for key in gone {
        g.release(&mut commands, key);
    }

    if g.dirty
        && let Some(mut b) = buffers.get_mut(&g.tile_buffer)
    {
        // The same size every time, so the GPU buffer is written in place and the material's bind
        // group stays valid.
        *b = ShaderBuffer::new(g.words.clone(), RenderAssetUsages::RENDER_WORLD);
        g.dirty = false;
    }
}

/// The bodies pressing the grass flat this frame, nearest the eye first: the pilot's feet on
/// foot, and every mech's feet. Radius 0 marks an empty entry.
fn pushers(eye: Vec3, bodies: impl Iterator<Item = (Vec3, f32)>) -> [[f32; 4]; MAX_PUSHERS] {
    let mut all: Vec<(Vec3, f32)> = bodies.filter(|(p, r)| p.is_finite() && *r > 0.0).collect();
    all.sort_by(|a, b| a.0.distance_squared(eye).total_cmp(&b.0.distance_squared(eye)));
    let mut out = [[0.0; 4]; MAX_PUSHERS];
    for (o, (p, r)) in out.iter_mut().zip(all) {
        *o = [p.x, p.y, p.z, r];
    }
    out
}

/// Writes the bodies pressing the grass flat.
fn press(
    grass: Option<Res<Grass>>,
    pilot: Res<PilotSim>,
    camera: Single<&Transform, With<PilotCamera>>,
    rigs: Query<&MechRig>,
    transforms: Query<&GlobalTransform>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(grass) = grass else { return };
    let cfg = &grass.cfg;
    let feet = [foot_bone("foot_L"), foot_bone("foot_R")];
    let pilot_feet = (pilot.curr.mode == PilotMode::OnFoot).then_some((pilot.curr.feet, cfg.pilot_press_m));
    let mech_feet = rigs.iter().flat_map(|rig| {
        feet.iter()
            .flatten()
            .filter_map(|i| transforms.get(rig.bone_entity(*i)).ok())
            .map(|t| (t.translation(), cfg.mech_foot_press_m))
            .collect::<Vec<_>>()
    });
    let list = pushers(camera.translation, pilot_feet.into_iter().chain(mech_feet));
    if let Some(mut b) = buffers.get_mut(&grass.push_buffer) {
        *b = ShaderBuffer::new(list.to_vec(), RenderAssetUsages::RENDER_WORLD);
    }
}

/// The index of a contract bone by name.
fn foot_bone(name: &str) -> Option<usize> {
    cb_core::mech::rig::BONES.iter().position(|b| b.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_check::{compile, struct_size, with_bevy};

    #[test]
    fn the_grass_shader_compiles_against_bevy_pbr_and_its_uniform_matches() {
        let Some(resolver) = with_bevy() else {
            eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
            return;
        };
        // The main opaque pass as the pilot's eyes have it (HDR, no MSAA, a depth prepass,
        // distance fog), with and without the water's prepass, and the plainest fallback.
        let base = [("VERTEX_POSITIONS", true), ("CLUSTERED_FORWARD_STORAGE_BUFFER", true)];
        for (msaa, prepass, fog) in [(false, true, true), (true, false, true), (false, false, false)] {
            let mut flags = base.to_vec();
            flags.extend([("MULTISAMPLED", msaa), ("DEPTH_PREPASS", prepass), ("DISTANCE_FOG", fog)]);
            let module = compile(&resolver, "grass", &flags);
            assert_eq!(
                u64::from(struct_size(&module, "GrassLook")),
                GrassUniform::min_size().get(),
                "the Rust uniform must be the WESL struct's size"
            );
        }
    }

    #[test]
    fn the_shipped_grass_fills_every_level_of_the_uniform_it_uses() {
        let cfg = GrassConfig::default();
        let layout = GrassLayout::new(&cfg, 8.0);
        let u = GrassUniform::new(&cfg, &layout, 8.0);
        assert_eq!(u.lods[0], Vec4::new(1.0, 5.0, 1.0, 25.0));
        assert_eq!(u.next[0].x, 2.0, "level 0 crossfades into level 1's stride");
        assert_eq!(u.next[2], Vec4::ZERO, "the last level has no next");
        assert_eq!(
            u.grid,
            UVec4::new(160, 32, 5, 25 + 256 + 256),
            "heights, density, then dryness (#89)"
        );
    }

    #[test]
    fn the_blade_mesh_is_only_indices() {
        let cfg = GrassConfig::default();
        let m = blade_mesh(&cfg, 2);
        assert_eq!(m.count_vertices(), 8_000);
        assert_eq!(m.indices().map(Indices::len), Some(3 * 4_800));
        assert!(m.attribute(Mesh::ATTRIBUTE_NORMAL).is_none(), "the shader builds the normals");
    }

    #[test]
    fn the_nearest_bodies_press_the_grass_and_the_rest_wait() {
        let eye = Vec3::ZERO;
        let bodies = (0..MAX_PUSHERS + 4).map(|i| (Vec3::new(i as f32 * 10.0, 0.0, 0.0), 1.0));
        let list = pushers(eye, bodies.rev());
        assert_eq!(list[0], [0.0, 0.0, 0.0, 1.0], "the nearest first");
        assert_eq!(list[MAX_PUSHERS - 1][0], (MAX_PUSHERS - 1) as f32 * 10.0, "the nearest, no more");
        let one = pushers(eye, std::iter::once((Vec3::X, 2.0)));
        assert_eq!(one[1][3], 0.0, "an empty entry has no radius");
    }
}
