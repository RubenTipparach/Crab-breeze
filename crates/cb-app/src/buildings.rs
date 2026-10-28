//! Building drawing and timing (`building-generation`, `structural-destruction`):
//! - a building far from the player is one plain box (no generated detail);
//! - within `dress_radius_m` of the player it is dressed instead: the mockup's facade skin per bay
//!   per storey, roof, parapet and roof props, in a trim mesh and a glass mesh, and its box hidden;
//! - a damaged one also draws its frame's floors and columns and the rooms behind them (facade
//!   panels too, where no skin hangs).
//!
//! Structure and furniture are drawn one mesh per surface (`building-surfaces`): concrete, plaster,
//! wood, fabric, metal, plastic, cardboard and leaves, each a CC0 PBR set shared by every building.
//!
//! A damaged building is meshed once (`building-gpu-state`): every vertex carries the slot of the
//! element it hangs on, and the building's element state buffer says which stand; the vertex
//! shaders drop what fell. A hit rewrites the buffer, never a mesh. Each fixed step advances the
//! frames' clock so overloaded members fail when their creak runs out and falling parts land.

use crate::building_state::{ArrayMaterial, ArrayState, BuildingMaterial, BuildingStates, ElementState, slot, standing_words};
use crate::sim::{PresentSet, SimSet};
use crate::state::GameApp;
use crate::world::{Live, MeshBuilder};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use cb_core::rng::SimRng;
use cb_core::world::destruct::Cells;
use cb_core::world::dressing::{self, Anchor, DressingConfig, Mat, Shape};
use cb_core::world::interior::{self, InteriorConfig, Rest, Site, StoreyPlan, Surface};
use cb_core::world::structure::{Frame, GROUND, Kind};
use std::collections::{BTreeMap, HashMap};

/// What buildings are drawn with: the facade tones as vertex tints,
/// the opaque looks as texture arrays (`building-texture-array`), the glass, and the concrete the
/// debris chunks use.
#[derive(Resource)]
pub struct BuildingLooks {
    /// The far boxes' facade tones (`world::BUILDING_TONES`) as linear vertex tints, for the frame's
    /// facade panels.
    pub tone_tints: Vec<[f32; 3]>,
    /// Every opaque look as a layer (`Layer`).
    pub arrays: crate::building_arrays::Arrays,
    /// The far facades, one layer per facade kind, main colour and storey band, and the roof
    /// (`building-type-lods`, `far_facades`).
    pub far: crate::building_arrays::Arrays,
    /// See-through window glass.
    pub glass: Handle<StandardMaterial>,
    /// The concrete set, for debris chunks.
    pub concrete: Handle<StandardMaterial>,
}

/// One of a building's meshes: what it holds decides its material and when it is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Look {
    /// The frame's floors and columns.
    Frame,
    /// Rooms and furniture (`building-lod`): drawn near the pilot only, casting no shadows.
    Rooms,
    /// The frame's facade panels (hidden while the building is dressed).
    Facade,
    /// The opaque skin, roof and props, and a whole building's rooms behind its glass.
    Skin,
    /// The windows.
    Glass,
}

/// A texture array layer (`building_arrays`): the eight surfaces, the skin's textured materials,
/// plain trim and the facade panels.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    /// A structure or furniture surface.
    Surface(Surface),
    /// A skin material other than glass.
    Skin(Mat),
    /// The plain facade panels of a frame.
    Facade,
}

impl Layer {
    /// Its index in the arrays (the order of `layer_sources`).
    fn index(self) -> u32 {
        let surfaces = Surface::ALL.len() as u32;
        match self {
            Layer::Surface(s) => Surface::ALL.iter().position(|x| *x == s).unwrap_or(0) as u32,
            Layer::Skin(Mat::Brick) => surfaces,
            Layer::Skin(Mat::Concrete) => surfaces + 1,
            Layer::Skin(Mat::Roof) => surfaces + 2,
            Layer::Skin(Mat::Wood) => surfaces + 3,
            Layer::Skin(Mat::Trim | Mat::Glass) => surfaces + 4,
            Layer::Facade => surfaces + 5,
        }
    }

    /// Metres per texture tile along u and v (its texture's real size).
    fn tile_m(self) -> Vec2 {
        match self {
            Layer::Surface(s) => Vec2::splat(surface_tile_m(s)),
            Layer::Skin(m) => Vec2::splat(tile_m(m)),
            Layer::Facade => Vec2::new(4.0, 3.5),
        }
    }
}

/// The arrays' sources, in `Layer::index` order: the CC0 surface sets, the generated skin maps
/// with the roughness their materials had, plain trim, the generated facade.
pub fn layer_sources(tex: &crate::textures::Textures) -> Vec<crate::building_arrays::Source> {
    use crate::building_arrays::Source;
    let b = &tex.building;
    let gen_ = |p: &crate::textures::Pbr, roughness: f32| Source::Generated {
        color: p.color.clone(),
        normal: Some(p.normal.clone()),
        roughness,
    };
    Surface::ALL
        .iter()
        .map(|s| Source::Set(format!("textures/building/{}", format!("{s:?}").to_lowercase())))
        .chain([
            gen_(&b.brick, 0.9),
            gen_(&b.concrete, 0.85),
            gen_(&b.roof, 0.95),
            gen_(&b.wood, 0.7),
            Source::Flat { roughness: 0.6 },
            Source::Generated {
                color: tex.facade.clone(),
                normal: None,
                roughness: 0.75,
            },
        ])
        .collect()
}

/// Metres per texture tile for a surface (`building-surfaces` design section 1).
fn surface_tile_m(s: Surface) -> f32 {
    match s {
        Surface::Concrete | Surface::Plaster => 2.4,
        Surface::Fabric => 0.6,
        Surface::Leaves => 0.5,
        Surface::Wood | Surface::Metal | Surface::Plastic | Surface::Cardboard => 1.0,
    }
}

/// The concrete surface set as a plain material, for the debris chunks (`physics`).
pub fn concrete_material(images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    crate::pbr_set::set_material(images, materials, "textures/building/concrete", 0.85)
}

/// A building material handle of either kind.
#[derive(Clone)]
enum Material {
    /// The opaque array material.
    Array(Handle<ArrayMaterial>),
    /// The glass.
    Glass(Handle<BuildingMaterial>),
}

/// The two building materials, shared by every building (so their draws batch), reading the one
/// element state buffer (`BuildingStates`). Made at first need.
#[derive(Resource, Default)]
struct Materials {
    /// The buffer they read.
    standing: Option<Handle<ShaderBuffer>>,
    array: Option<Handle<ArrayMaterial>>,
    glass: Option<Handle<BuildingMaterial>>,
}

impl Materials {
    /// Points both materials at the state buffer's current handle (it changes when it grows).
    fn follow(&mut self, states: &BuildingStates, arrays: &mut Assets<ArrayMaterial>, glasses: &mut Assets<BuildingMaterial>) {
        if self.standing.as_ref() == Some(&states.handle) {
            return;
        }
        self.standing = Some(states.handle.clone());
        if let Some(mut m) = self.array.as_ref().and_then(|h| arrays.get_mut(h)) {
            m.extension.standing = states.handle.clone();
        }
        if let Some(mut m) = self.glass.as_ref().and_then(|h| glasses.get_mut(h)) {
            m.extension.standing = states.handle.clone();
        }
    }

    /// The material for `look`.
    fn get(
        &mut self,
        look: Look,
        looks: &BuildingLooks,
        bases: &Assets<StandardMaterial>,
        arrays: &mut Assets<ArrayMaterial>,
        glasses: &mut Assets<BuildingMaterial>,
    ) -> Material {
        let standing = self.standing.clone().unwrap_or_default();
        match look {
            Look::Glass => Material::Glass(
                self.glass
                    .get_or_insert_with(|| {
                        glasses.add(BuildingMaterial {
                            base: StandardMaterial {
                                depth_bias: GLASS_SORT_BIAS,
                                ..bases.get(&looks.glass).cloned().unwrap_or_default()
                            },
                            extension: ElementState {
                                standing,
                                no_ssao: 1,
                                windows: Vec4::ZERO,
                            },
                        })
                    })
                    .clone(),
            ),
            _ => Material::Array(
                self.array
                    .get_or_insert_with(|| {
                        arrays.add(ArrayMaterial {
                            base: StandardMaterial::default(),
                            extension: ArrayState {
                                standing,
                                albedo: looks.arrays.albedo.clone(),
                                normal: looks.arrays.normal.clone(),
                                orm: looks.arrays.orm.clone(),
                                windows: Vec4::ZERO,
                            },
                        })
                    })
                    .clone(),
            ),
        }
    }
}

/// Adds a box from layer `layer`, tinted `tint`, carrying `slot` (`building_state`).
fn layered_block(mb: &mut MeshBuilder, layer: Layer, min: Vec3, max: Vec3, tint: [f32; 3], slot: f32) {
    mb.set_layer(layer.index(), layer.tile_m());
    mb.block(min, max, [tint[0], tint[1], tint[2], slot]);
}

/// The rooms of one storey into `mb`: its walls and furniture, each carrying the slot of what it
/// rests on.
fn rooms_into(mb: &mut MeshBuilder, plan: &StoreyPlan, slot_of: impl Fn(Rest) -> f32) {
    for w in &plan.walls {
        let s = slot_of(w.rest);
        if w.core {
            layered_block(mb, Layer::Surface(Surface::Concrete), w.min, w.max, CORE_TINT, s);
        } else {
            layered_block(mb, Layer::Surface(Surface::Plaster), w.min, w.max, PARTITION_TINT, s);
        }
    }
    for st in &plan.steps {
        layered_block(mb, Layer::Surface(Surface::Concrete), st.min, st.max, SLAB_TINT, slot_of(st.rest));
    }
    for it in &plan.items {
        let s = slot_of(it.rest);
        for (lo, hi, col, surface) in &it.boxes {
            layered_block(mb, Layer::Surface(*surface), *lo, *hi, *col, s);
        }
    }
}

/// Added to the glass's distance when transparent things are sorted (Bevy sorts by the mesh's
/// box centre plus this): the windows always draw before smoke, dust and fire. One glass mesh
/// covers a building, so sorted at its centre it drew over any smoke farther than that centre,
/// though the smoke was in front of the window (owner, 2026-10-25: "glass not playing well with
/// smoke"). Sort only: `ElementState::specialize` keeps it out of the depth test.
pub const GLASS_SORT_BIAS: f32 = -1.0e6;

/// Tints of the structure: columns and core walls, slabs, partitions.
const CORE_TINT: [f32; 3] = [0.66, 0.64, 0.6];
const SLAB_TINT: [f32; 3] = [0.56, 0.55, 0.52];
const PARTITION_TINT: [f32; 3] = [0.86, 0.84, 0.78];

/// A drawn mesh: its entity and the mesh it shows (rebuilt in place).
#[derive(Clone)]
struct Drawn {
    entity: Entity,
    mesh: Handle<Mesh>,
}

/// A dressed building's meshes, one per material, and whether they carry its frame's slots.
struct Dressing {
    parts: BTreeMap<Look, Drawn>,
    framed: bool,
}

/// Dressed buildings near the player.
#[derive(Resource, Default)]
pub(crate) struct Dressed(HashMap<u32, Dressing>);

impl Dressed {
    /// The buildings dressed now, in id order (their street doors are drawn with them: `street_doors`).
    pub(crate) fn ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.0.keys().copied().collect();
        ids.sort_unstable();
        ids
    }
}

/// What a street door leaf is drawn in (`street_doors`): the shared building materials, so a leaf
/// matches the skin it hangs in.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct LeafLooks<'w> {
    shared: ResMut<'w, Materials>,
    looks: Option<Res<'w, BuildingLooks>>,
    states: Res<'w, BuildingStates>,
    bases: Res<'w, Assets<StandardMaterial>>,
    arrays: ResMut<'w, Assets<ArrayMaterial>>,
    glasses: ResMut<'w, Assets<BuildingMaterial>>,
}

impl LeafLooks<'_> {
    /// Spawns a leaf: a box `size` (width along local +x from the hinge at the origin, height up,
    /// thickness centred on z) in `mat` tinted `tint`. `None` until the building looks are loaded.
    pub(crate) fn spawn(
        &mut self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        mat: Mat,
        tint: [f32; 3],
        size: Vec3,
        name: String,
    ) -> Option<(Entity, Handle<Mesh>)> {
        let looks = self.looks.as_deref()?;
        self.shared.follow(&self.states, &mut self.arrays, &mut self.glasses);
        let (min, max) = (Vec3::new(0.0, 0.0, -size.z * 0.5), Vec3::new(size.x, size.y, size.z * 0.5));
        let col = [tint[0], tint[1], tint[2], 0.0];
        let (look, mesh) = if mat == Mat::Glass {
            let mut mb = MeshBuilder::tiled(tile_m(Mat::Glass), tile_m(Mat::Glass));
            mb.block(min, max, col);
            (Look::Glass, mb.build())
        } else {
            let mut mb = MeshBuilder::default();
            let layer = Layer::Skin(mat);
            mb.set_layer(layer.index(), layer.tile_m());
            mb.block(min, max, col);
            (Look::Skin, mb.build_with_tangents())
        };
        let handle = meshes.add(mesh);
        let mut e = commands.spawn((Name::new(name), Mesh3d(handle.clone()), Transform::IDENTITY));
        match self.shared.get(look, looks, &self.bases, &mut self.arrays, &mut self.glasses) {
            Material::Array(h) => e.insert(MeshMaterial3d(h)),
            Material::Glass(h) => e.insert(MeshMaterial3d(h)),
        };
        Some((e.id(), handle))
    }
}

/// Set when every building near the player is dressed (none waits for its turn), so a capture
/// can wait for the street to finish dressing on a slow renderer.
#[derive(Resource, Default)]
pub struct DressingSettled(pub bool);

/// Buildings whose dressing is being meshed on the async pool (`dress-off-thread`), by id: a
/// building's dressing took 12-15 ms of the main thread, 45 ms at worst (2026-10-25 benchmark).
#[derive(Resource, Default)]
struct DressJobs(BTreeMap<u32, Task<Vec<(Look, Mesh)>>>);

/// Most buildings being dressed at once.
const DRESS_JOBS: usize = 4;

/// A damaged building on the GPU: its frame meshes (built once) and its range in the element
/// state buffer.
struct Framed {
    parts: BTreeMap<Look, Drawn>,
    base: u32,
}

/// Damaged buildings near the player, by id.
#[derive(Resource, Default)]
struct FramedBuildings(HashMap<u32, Framed>);

/// Building entities by id.
#[derive(Resource, Default)]
pub struct BuildingEntities(pub Vec<Entity>);

/// Redraws damaged buildings.
pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingEntities>()
            .init_resource::<FramedBuildings>()
            .init_resource::<Materials>()
            .init_resource::<Dressed>()
            .init_resource::<DressJobs>()
            .init_resource::<DressingSettled>()
            .add_game_systems(FixedUpdate, step_structures.in_set(SimSet::World))
            .add_game_systems(
                Update,
                (
                    dress_near_player,
                    redraw_buildings,
                    show_rooms_near,
                    bind_far_looks,
                    show_massing,
                    light_windows,
                )
                    .chain()
                    .in_set(PresentSet::Pose),
            )
            .init_resource::<FarMaterials>()
            .init_resource::<Massing>();
    }
}

/// The mesh of a building's standing cells, in world space.
fn cells_mesh(c: &Cells) -> Mesh {
    let mut b = MeshBuilder::tiled(4.0, 3.5);
    let white = [1.0, 1.0, 1.0, 1.0];
    let solid = |i: isize, k: isize, j: isize| {
        i >= 0
            && k >= 0
            && j >= 0
            && (i as usize) < c.n[0]
            && (k as usize) < c.n[1]
            && (j as usize) < c.n[2]
            && c.at(i as usize, k as usize, j as usize)
    };
    for k in 0..c.n[1] {
        for j in 0..c.n[2] {
            for i in 0..c.n[0] {
                if !c.at(i, k, j) {
                    continue;
                }
                let lo = c.origin + c.size * Vec3::new(i as f32, k as f32, j as f32);
                let hi = lo + c.size;
                let v = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
                let (ii, kk, jj) = (i as isize, k as isize, j as isize);
                // Each face listed so its outward normal is (c1 - c0) x (c3 - c0).
                if !solid(ii, kk + 1, jj) {
                    b.quad(
                        [v(lo.x, hi.y, lo.z), v(lo.x, hi.y, hi.z), v(hi.x, hi.y, hi.z), v(hi.x, hi.y, lo.z)],
                        white,
                    );
                }
                if !solid(ii, kk - 1, jj) {
                    b.quad(
                        [v(lo.x, lo.y, lo.z), v(hi.x, lo.y, lo.z), v(hi.x, lo.y, hi.z), v(lo.x, lo.y, hi.z)],
                        white,
                    );
                }
                if !solid(ii, kk, jj + 1) {
                    b.quad(
                        [v(lo.x, lo.y, hi.z), v(hi.x, lo.y, hi.z), v(hi.x, hi.y, hi.z), v(lo.x, hi.y, hi.z)],
                        white,
                    );
                }
                if !solid(ii, kk, jj - 1) {
                    b.quad(
                        [v(hi.x, lo.y, lo.z), v(lo.x, lo.y, lo.z), v(lo.x, hi.y, lo.z), v(hi.x, hi.y, lo.z)],
                        white,
                    );
                }
                if !solid(ii + 1, kk, jj) {
                    b.quad(
                        [v(hi.x, lo.y, hi.z), v(hi.x, lo.y, lo.z), v(hi.x, hi.y, lo.z), v(hi.x, hi.y, hi.z)],
                        white,
                    );
                }
                if !solid(ii - 1, kk, jj) {
                    b.quad(
                        [v(lo.x, lo.y, lo.z), v(lo.x, lo.y, hi.z), v(lo.x, hi.y, hi.z), v(lo.x, hi.y, lo.z)],
                        white,
                    );
                }
            }
        }
    }
    b.build()
}

/// Creaking members fail on time and falling parts land; what falls goes to the debris like any
/// other chunk, and what reaches the ground is raised as a heap in the crater layer (`rubble`), so
/// the pilot, the mech and the debris stand on it.
fn step_structures(time: Res<Time<Fixed>>, mut live: ResMut<Live>, mut fx: MessageWriter<crate::fx::Fx>) {
    let _t = crate::perf_hud::slow("step_structures");
    let (changed, chunks) = live.structures.step(time.delta_secs());
    for m in live.structures.take_moments() {
        fx.write(match m {
            cb_core::world::Moment::Creak(at) => crate::fx::Fx::new("creak_dust", at),
            cb_core::world::Moment::Fail(at) => crate::fx::Fx::new("member_fail", at),
            cb_core::world::Moment::Impact { at, mass_kg } => {
                crate::fx::Fx::new("collapse_dust", at).scaled((mass_kg / 2.0e5).cbrt().clamp(0.5, 2.0))
            }
        });
    }
    if !changed.is_empty() || !chunks.is_empty() {
        crate::probe_event!("structures: {} buildings changed, {} chunks fell", changed.len(), chunks.len());
    }
    live.dirty_buildings.extend(changed);
    live.new_chunks.extend(chunks);
    let cfg = live.structures.cfg.clone();
    for heap in live.structures.take_landings() {
        let touched = live.dug.pile(
            heap.lo,
            heap.hi,
            heap.volume_m3,
            cfg.rubble_spread_m,
            cfg.repose_slope,
            cfg.max_rubble_m,
        );
        live.dirty_tiles.extend(touched);
    }
}

/// The meshes of a frame, built once (`building-gpu-state`): its facade panels (tinted `tone`),
/// and its floors, columns, rooms and furniture in one mesh (`building-texture-array`), every
/// vertex carrying the slot of its element (rooms and furniture: the slab under them, or 0 on the
/// ground). Columns are drawn 1 cm thinner than they are and side walls stop short of the corner
/// walls, so no two faces share a plane (CLAUDE.md section 10).
fn frame_meshes(f: &Frame, site: &Site, icfg: &InteriorConfig, base: u32, tone: [f32; 3], wall_m: f32, seed: &SimRng) -> Vec<(Look, Mesh)> {
    // The stairwell is cut out of the floor bays above the ground, as in the colliders.
    let hole = interior::shell(site, icfg, seed).stair.hole();
    let mut facade = MeshBuilder::default();
    let mut frame = MeshBuilder::default();
    for (id, e) in f.elements.iter().enumerate() {
        let s = slot(base, id as u32);
        match e.kind {
            Kind::Wall => {
                let (mut lo, mut hi) = (e.min, e.max);
                if e.side >= 2 {
                    if e.at[1] == 0 {
                        lo.z += wall_m;
                    }
                    if e.at[1] as usize + 1 == f.n[2] {
                        hi.z -= wall_m;
                    }
                }
                layered_block(&mut facade, Layer::Facade, lo, hi, tone, s);
            }
            Kind::Column => layered_block(
                &mut frame,
                Layer::Surface(Surface::Concrete),
                e.min + Vec3::new(0.01, 0.0, 0.01),
                e.max - Vec3::new(0.01, 0.0, 0.01),
                CORE_TINT,
                s,
            ),
            Kind::Slab if (e.storey as usize) + 1 < f.n[1] => {
                let r = (Vec2::new(e.min.x, e.min.z), Vec2::new(e.max.x, e.max.z));
                for (lo, hi) in interior::rect_minus(r, hole) {
                    let (a, b) = (Vec3::new(lo.x, e.min.y, lo.y), Vec3::new(hi.x, e.max.y, hi.y));
                    layered_block(&mut frame, Layer::Surface(Surface::Concrete), a, b, SLAB_TINT, s);
                }
            }
            Kind::Slab => layered_block(&mut frame, Layer::Surface(Surface::Concrete), e.min, e.max, SLAB_TINT, s),
        }
    }
    let rest_slot = |r: Rest| match r {
        Rest::Ground => 0.0,
        Rest::Slab { k, i, j } => slot(base, f.slab(k as usize, i as usize, j as usize)),
    };
    let mut rooms = MeshBuilder::default();
    for k in 0..f.n[1] {
        rooms_into(&mut rooms, &interior::plan_storey(site, k, icfg, seed), rest_slot);
    }
    vec![
        (Look::Facade, facade.build_with_tangents()),
        (Look::Frame, frame.build_with_tangents()),
        (Look::Rooms, rooms.build_with_tangents()),
    ]
}

/// The slot of the element a dressing piece hangs on: 0 for the whole building or one with no
/// frame; `None` for a piece on nothing (it is never drawn).
fn anchor_slot(f: Option<&Frame>, base: u32, a: Anchor) -> Option<f32> {
    let Some(f) = f else { return Some(0.0) };
    let id = match a {
        Anchor::Whole => return Some(0.0),
        Anchor::Column { k, a, c } => f.column(k as usize, a as usize, c as usize),
        Anchor::Slab { k, i, j } => f.slab(k as usize, i as usize, j as usize),
        Anchor::Wall { k, side, idx } => f.wall(k as usize, side as usize, idx as usize),
    };
    (id != GROUND).then(|| slot(base, id))
}

/// Metres per texture tile for a material (its texture's real size).
fn tile_m(mat: Mat) -> f32 {
    match mat {
        Mat::Brick | Mat::Concrete => 2.4,
        Mat::Roof => 2.0,
        Mat::Wood => 1.0,
        Mat::Trim | Mat::Glass => 4.0,
    }
}

/// The dressing meshes of building `id`: the opaque skin in one mesh (every material a layer of
/// the arrays) and the glass in another, cut per element, every vertex carrying its element's slot,
/// once it has a frame; whole, with its floors, rooms and furniture behind the glass (in the skin
/// mesh) while it is whole (a damaged building draws its rooms with its frame).
fn dressing_meshes(c: &Cells, site: &Site, icfg: &InteriorConfig, base: u32, seed: &SimRng, cfg: &DressingConfig) -> Vec<(Look, Mesh)> {
    let f = c.frame.as_deref();
    let id = site.id;
    let shell = interior::shell(site, icfg, seed);
    let mut skin = MeshBuilder::default();
    let mut glass = MeshBuilder::tiled(tile_m(Mat::Glass), tile_m(Mat::Glass));
    for p in dressing::dress(id, c.n, c.origin, c.size, seed, f.is_some(), cfg, site.fronts, Some(&shell)) {
        let Some(s) = anchor_slot(f, base, p.anchor) else { continue };
        let mb = if p.mat == Mat::Glass {
            &mut glass
        } else {
            let layer = Layer::Skin(p.mat);
            skin.set_layer(layer.index(), layer.tile_m());
            &mut skin
        };
        let col = [p.color[0], p.color[1], p.color[2], s];
        match p.shape {
            Shape::Block { min, max } => mb.block(min, max, col),
            Shape::Drum { base, r0, r1, height_m } => mb.drum(base, r0, r1, height_m, 14, col),
        }
    }
    let mut rooms = MeshBuilder::default();
    if f.is_none() {
        for k in 0..c.n[1] {
            rooms_into(&mut rooms, &interior::plan_storey(site, k, icfg, seed), |_| 0.0);
        }
    }
    vec![
        (Look::Skin, skin.build_with_tangents()),
        (Look::Glass, glass.build()),
        (Look::Rooms, rooms.build_with_tangents()),
    ]
}

/// The stream every building's plans, dressing and colliders are drawn from: one, so what is drawn
/// is what the pilot walks into.
pub fn building_rng(world_seed: u64) -> SimRng {
    SimRng::new(world_seed).fork("dressing")
}

/// Shows `parts` in `drawn`, one entity per look, each in its material; a look already drawn has
/// its mesh replaced in place and keeps its entity, one that no longer appears is emptied.
fn show_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    drawn: &mut BTreeMap<Look, Drawn>,
    mut parts: Vec<(Look, Mesh)>,
    mut material: impl FnMut(Look) -> Material,
    name: &str,
) {
    for k in drawn.keys() {
        if !parts.iter().any(|(p, _)| p == k) {
            parts.push((*k, MeshBuilder::tiled(4.0, 4.0).build()));
        }
    }
    for (k, mesh) in parts {
        let m = material(k);
        let d = match drawn.get(&k) {
            Some(d) => {
                meshes.insert(&d.mesh, mesh).ok();
                d.clone()
            }
            None => {
                let handle = meshes.add(mesh);
                let mut e = commands.spawn((Name::new(format!("{name} {k:?}")), Mesh3d(handle.clone()), Transform::IDENTITY));
                match m {
                    Material::Array(h) => e.insert(MeshMaterial3d(h)),
                    Material::Glass(h) => e.insert(MeshMaterial3d(h)),
                };
                if k == Look::Rooms {
                    // Shown near the pilot only (`show_rooms_near`); never in the shadow maps.
                    e.insert((Visibility::Hidden, NotShadowCaster));
                }
                Drawn {
                    entity: e.id(),
                    mesh: handle,
                }
            }
        };
        drawn.insert(k, d);
    }
}

/// Whether a building's rooms are drawn (`building-lod`): shown within the rooms radius of the
/// pilot's eye, hidden beyond it plus the margin, kept as they were in between (so a building on
/// the edge does not flicker). Distance to the footprint, across the ground.
fn rooms_shown(distance_m: f32, was: bool, cfg: &DressingConfig) -> bool {
    if distance_m <= cfg.rooms_radius_m {
        true
    } else if distance_m > cfg.rooms_radius_m + cfg.rooms_margin_m {
        false
    } else {
        was
    }
}

/// Shows and hides the rooms and furniture of drawn buildings by their distance from the pilot.
fn show_rooms_near(
    mut commands: Commands,
    plan: Res<crate::world::Plan>,
    cfg: Res<crate::sim::Data>,
    camera: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    dressed: Res<Dressed>,
    framed: Res<FramedBuildings>,
    mut shown: Local<HashMap<Entity, bool>>,
) {
    let Some(here) = player_xz(&camera) else { return };
    let cfg = &cfg.0.world.dressing;
    let parts = dressed
        .0
        .iter()
        .map(|(b, d)| (*b, &d.parts))
        .chain(framed.0.iter().map(|(b, f)| (*b, &f.parts)));
    let mut seen = HashMap::new();
    for (b, parts) in parts {
        let d = plan.0.buildings[b as usize].footprint.distance(here);
        for (look, drawn) in parts {
            if *look != Look::Rooms {
                continue;
            }
            let was = shown.get(&drawn.entity).copied();
            let now = rooms_shown(d, was.unwrap_or(false), cfg);
            if was != Some(now) {
                commands
                    .entity(drawn.entity)
                    .insert(if now { Visibility::Inherited } else { Visibility::Hidden });
            }
            seen.insert(drawn.entity, now);
        }
    }
    *shown = seen;
}

/// Where the player is: the pilot's eyes, on foot or in the cockpit.
fn player_xz(camera: &Query<&GlobalTransform, With<crate::pilot::PilotCamera>>) -> Option<Vec2> {
    camera.single().ok().map(|t| Vec2::new(t.translation().x, t.translation().z))
}

/// Dresses the buildings near the player, nearest first and off the main thread (`dress-off-thread`:
/// the plain box shows until the dressing is built), and undresses those left behind (their plain
/// box shows again). Far buildings never get generated detail (owner, 2026-10-24: "far away
/// buildings would not HAVE these generated elements"). A damaged building is dressed by
/// `redraw_buildings`, with its own element state.
#[allow(clippy::too_many_arguments)]
fn dress_near_player(
    mut commands: Commands,
    mut live: ResMut<Live>,
    plan: Res<crate::world::Plan>,
    seed: Res<crate::world::Seed>,
    looks: Option<Res<BuildingLooks>>,
    cfg: Res<crate::sim::Data>,
    ents: Res<BuildingEntities>,
    camera: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    (mut dressed, mut jobs, mut settled): (ResMut<Dressed>, ResMut<DressJobs>, ResMut<DressingSettled>),
    mut shared: ResMut<Materials>,
    states: Res<BuildingStates>,
    mut meshes: ResMut<Assets<Mesh>>,
    bases: Res<Assets<StandardMaterial>>,
    mut arrays: ResMut<Assets<ArrayMaterial>>,
    mut glasses: ResMut<Assets<BuildingMaterial>>,
) {
    let _t = crate::perf_hud::slow("dress_near_player");
    settled.0 = false;
    let (Some(looks), Some(here)) = (looks, player_xz(&camera)) else {
        return;
    };
    if ents.0.is_empty() {
        return;
    }
    let icfg = &cfg.0.world.interiors;
    let cfg = &cfg.0.world.dressing;
    let rng = building_rng(seed.0);
    let mut near: Vec<u32> = plan
        .0
        .collision
        .candidates(here, cfg.dress_radius_m)
        .into_iter()
        .filter(|&b| {
            !dressed.0.contains_key(&b)
                && !jobs.0.contains_key(&b)
                && plan.0.buildings[b as usize].footprint.centre().distance(here) <= cfg.dress_radius_m
        })
        .collect();
    // Nearest first, ties by id (never hash order).
    near.sort_by(|&a, &b| {
        let (da, db) = (
            plan.0.buildings[a as usize].footprint.centre().distance(here),
            plan.0.buildings[b as usize].footprint.centre().distance(here),
        );
        da.total_cmp(&db).then(a.cmp(&b))
    });
    let keep = cfg.dress_radius_m * cfg.keep_ratio;
    shared.follow(&states, &mut arrays, &mut glasses);
    // The dressings that are built, in id order. A building still near, standing and without a frame
    // is dressed with them; any other result is dropped (a framed one is dressed by `redraw_buildings`).
    let built: Vec<(u32, Vec<(Look, Mesh)>)> = jobs.0.iter_mut().filter_map(|(b, t)| check_ready(t).map(|m| (*b, m))).collect();
    for (b, built) in built {
        jobs.0.remove(&b);
        let c = &live.structures.buildings[b as usize];
        let near = cfg.shows(
            cfg.dress_radius_m,
            plan.0.buildings[b as usize].footprint.centre().distance(here),
            true,
        );
        if c.standing == 0 || c.frame.is_some() || !near || dressed.0.contains_key(&b) {
            continue;
        }
        let mut parts = BTreeMap::new();
        show_parts(
            &mut commands,
            &mut meshes,
            &mut parts,
            built,
            |l| shared.get(l, &looks, &bases, &mut arrays, &mut glasses),
            &format!("building {b}"),
        );
        dressed.0.insert(b, Dressing { parts, framed: false });
        commands.entity(ents.0[b as usize]).insert(Visibility::Hidden);
    }
    let pool = AsyncComputeTaskPool::get();
    let (mut waiting, mut framed_now) = (0, 0);
    for b in near {
        let c = &live.structures.buildings[b as usize];
        if c.standing == 0 {
            continue;
        }
        if c.frame.is_some() {
            // One a frame: `redraw_buildings` meshes it on the main thread.
            if framed_now > 0 {
                waiting += 1;
                continue;
            }
            framed_now += 1;
            // Dressed with its frame's slots and its own materials when redrawn.
            dressed.0.insert(
                b,
                Dressing {
                    parts: BTreeMap::new(),
                    framed: false,
                },
            );
            live.dirty_buildings.push(b);
            continue;
        }
        if jobs.0.len() >= DRESS_JOBS {
            waiting += 1;
            continue;
        }
        let site = Site::of(&plan.0, b, c.n, c.origin, c.size);
        let (c, rng, cfg, icfg) = (c.clone(), rng.clone(), cfg.clone(), icfg.clone());
        jobs.0
            .insert(b, pool.spawn(async move { dressing_meshes(&c, &site, &icfg, 0, &rng, &cfg) }));
    }
    // Left behind while being dressed: the job is dropped (which cancels it).
    jobs.0
        .retain(|&b, _| plan.0.buildings[b as usize].footprint.centre().distance(here) <= keep);
    settled.0 = waiting == 0 && jobs.0.is_empty();
    let mut gone: Vec<u32> = dressed
        .0
        .keys()
        .copied()
        .filter(|&b| plan.0.buildings[b as usize].footprint.centre().distance(here) > keep)
        .collect();
    gone.sort_unstable();
    for b in gone {
        if let Some(d) = dressed.0.remove(&b) {
            for part in d.parts.values() {
                commands.entity(part.entity).despawn();
                meshes.remove(&part.mesh);
            }
        }
        let c = &live.structures.buildings[b as usize];
        if c.frame.is_some() {
            live.dirty_buildings.push(b); // its facade panels show again
        } else {
            commands.entity(ents.0[b as usize]).insert(Visibility::Visible);
        }
    }
}

/// Draws what changed in damaged buildings. On a building's first damage its frame (and, if it is
/// dressed, its skin) is meshed once with slots and given its element state buffer; after that a
/// change only rewrites the buffer (`building-gpu-state`).
#[allow(clippy::too_many_arguments)]
fn redraw_buildings(
    mut commands: Commands,
    mut live: ResMut<Live>,
    ents: Res<BuildingEntities>,
    looks: Option<Res<BuildingLooks>>,
    mut framed: ResMut<FramedBuildings>,
    mut dressed: ResMut<Dressed>,
    mut shared: ResMut<Materials>,
    mut states: ResMut<BuildingStates>,
    seed: Res<crate::world::Seed>,
    cfg: Res<crate::sim::Data>,
    plan: Res<crate::world::Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    bases: Res<Assets<StandardMaterial>>,
    mut arrays: ResMut<Assets<ArrayMaterial>>,
    mut glasses: ResMut<Assets<BuildingMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let _t = crate::perf_hud::slow("redraw_buildings");
    if live.dirty_buildings.is_empty() || ents.0.is_empty() {
        return;
    }
    let Some(looks) = looks else { return };
    let mut ids = std::mem::take(&mut live.dirty_buildings);
    ids.sort_unstable();
    ids.dedup();
    let wall_m = live.structures.cfg.wall_m;
    let rng = building_rng(seed.0);
    for id in ids {
        let c = &live.structures.buildings[id as usize];
        let e = ents.0[id as usize];
        let Some(f) = c.frame.as_ref() else {
            // No frame (an old save, a test): the cell view.
            if c.standing == 0 {
                commands.entity(e).insert(Visibility::Hidden);
            } else {
                commands.entity(e).insert((Mesh3d(meshes.add(cells_mesh(c))), Transform::IDENTITY));
            }
            continue;
        };
        let site = Site::of(&plan.0, id, c.n, c.origin, c.size);
        let base = states.base(id, f.elements.len(), &mut buffers);
        shared.follow(&states, &mut arrays, &mut glasses);
        states.write(id, &standing_words(f));
        let tone = looks.tone_tints[id as usize % looks.tone_tints.len()];
        let fb = match framed.0.entry(id) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                // First damage: the whole-building box gives way to the frame, meshed once.
                commands.entity(e).insert(Visibility::Hidden);
                let mut fb = Framed {
                    parts: BTreeMap::new(),
                    base,
                };
                show_parts(
                    &mut commands,
                    &mut meshes,
                    &mut fb.parts,
                    frame_meshes(f, &site, &cfg.0.world.interiors, base, tone, wall_m, &rng),
                    |l| shared.get(l, &looks, &bases, &mut arrays, &mut glasses),
                    &format!("building {id}"),
                );
                v.insert(fb)
            }
        };
        // A dressed building's skin is meshed once with its frame's slots, in its own materials.
        let skinned = match dressed.0.get_mut(&id) {
            Some(d) => {
                if !d.framed {
                    let parts = dressing_meshes(c, &site, &cfg.0.world.interiors, fb.base, &rng, &cfg.0.world.dressing);
                    show_parts(
                        &mut commands,
                        &mut meshes,
                        &mut d.parts,
                        parts,
                        |l| shared.get(l, &looks, &bases, &mut arrays, &mut glasses),
                        &format!("building {id}"),
                    );
                    d.framed = true;
                }
                true
            }
            None => false,
        };
        if let Some(facade) = fb.parts.get(&Look::Facade) {
            commands
                .entity(facade.entity)
                .insert(if skinned { Visibility::Hidden } else { Visibility::Inherited });
        }
    }
}

// ---------------------------------------------------------------- far looks (`building-type-lods`)

/// A material's layer in the near skin's arrays and its tile size, m: what the far facades sample.
pub fn skin_layer(mat: Mat) -> (u32, f32) {
    let l = Layer::Skin(mat);
    (l.index(), l.tile_m().x)
}

/// Marks a building's far walls (its box): they wear the far material once it exists.
#[derive(Component)]
pub struct FarWalls;

/// The far mesh of a building, relative to its cell grid's corner (place it there): walls of its
/// own facade kind and colour, the ground storey and the storeys above as their own bands, one bay
/// per cell, and a roof cap. Every vertex is untinted and on slot 0 (it always stands).
pub fn far_mesh(c: &Cells, id: u32, rng: &SimRng) -> Mesh {
    use cb_core::world::far_facade::{Band, layer_of, roof_layer};
    let v = dressing::far_variant(id, c.n, c.size, rng);
    let (ground, typical) = (layer_of(v, Band::Ground).unwrap_or(0), layer_of(v, Band::Typical).unwrap_or(1));
    let [nx, ns, nz] = c.n;
    let (w, d, s) = (c.size.x * nx as f32, c.size.z * nz as f32, c.size.y);
    let h = s * ns as f32;
    let col = [1.0, 1.0, 1.0, 0.0];
    let v3 = Vec3::new;
    let mut mb = MeshBuilder::default();
    for (layer, y0, y1) in [(ground, 0.0, s.min(h)), (typical, s, h)] {
        if y1 <= y0 {
            continue;
        }
        mb.set_layer(layer, Vec2::new(c.size.x, s));
        mb.quad([v3(w, y0, 0.0), v3(0.0, y0, 0.0), v3(0.0, y1, 0.0), v3(w, y1, 0.0)], col);
        mb.quad([v3(0.0, y0, d), v3(w, y0, d), v3(w, y1, d), v3(0.0, y1, d)], col);
        mb.set_layer(layer, Vec2::new(c.size.z, s));
        mb.quad([v3(0.0, y0, 0.0), v3(0.0, y0, d), v3(0.0, y1, d), v3(0.0, y1, 0.0)], col);
        mb.quad([v3(w, y0, d), v3(w, y0, 0.0), v3(w, y1, 0.0), v3(w, y1, d)], col);
    }
    mb.set_layer(roof_layer(), Vec2::splat(tile_m(Mat::Roof)));
    mb.quad([v3(0.0, h, 0.0), v3(0.0, h, d), v3(w, h, d), v3(w, h, 0.0)], col);
    mb.build_with_tangents()
}

/// A building's massing mesh in world space: the pieces its own dressing puts on and above the
/// roof line, in the near skin's layers (glass left out: there is none up there).
fn massing_mesh(pieces: &[dressing::Piece]) -> Mesh {
    let mut mb = MeshBuilder::default();
    for p in pieces.iter().filter(|p| p.mat != Mat::Glass) {
        let layer = Layer::Skin(p.mat);
        mb.set_layer(layer.index(), layer.tile_m());
        let col = [p.color[0], p.color[1], p.color[2], 0.0];
        match p.shape {
            Shape::Block { min, max } => mb.block(min, max, col),
            Shape::Drum { base, r0, r1, height_m } => mb.drum(base, r0, r1, height_m, 10, col),
        }
    }
    mb.build_with_tangents()
}

/// The far walls' material and the massing's (the near skin's arrays), both reading the element
/// state buffer and following it when it grows.
#[derive(Resource, Default)]
struct FarMaterials {
    standing: Option<Handle<ShaderBuffer>>,
    far: Option<Handle<ArrayMaterial>>,
    massing: Option<Handle<ArrayMaterial>>,
}

/// Makes the far materials once the buffer and the looks exist, gives every building's far walls
/// theirs, and follows the buffer when it grows.
fn bind_far_looks(
    mut commands: Commands,
    mut mats: ResMut<FarMaterials>,
    looks: Option<Res<BuildingLooks>>,
    states: Option<Res<BuildingStates>>,
    mut arrays: ResMut<Assets<ArrayMaterial>>,
    walls: Query<Entity, (With<FarWalls>, Without<MeshMaterial3d<ArrayMaterial>>)>,
) {
    let (Some(looks), Some(states)) = (looks, states) else { return };
    let make = |a: &crate::building_arrays::Arrays| ArrayMaterial {
        base: StandardMaterial::default(),
        extension: ArrayState {
            standing: states.handle.clone(),
            albedo: a.albedo.clone(),
            normal: a.normal.clone(),
            orm: a.orm.clone(),
            windows: Vec4::ZERO,
        },
    };
    if mats.far.is_none() {
        mats.far = Some(arrays.add(make(&looks.far)));
        mats.massing = Some(arrays.add(make(&looks.arrays)));
        mats.standing = Some(states.handle.clone());
    } else if mats.standing.as_ref() != Some(&states.handle) {
        mats.standing = Some(states.handle.clone());
        for h in [&mats.far, &mats.massing].into_iter().flatten() {
            if let Some(mut m) = arrays.get_mut(h) {
                m.extension.standing = states.handle.clone();
            }
        }
    }
    if let Some(far) = &mats.far {
        for e in &walls {
            commands.entity(e).insert(MeshMaterial3d(far.clone()));
        }
    }
}

/// The massing shown, by building id.
#[derive(Resource, Default)]
struct Massing(HashMap<u32, Entity>);

/// Shows each building's massing within `massing_radius_m` of the player and drops it past
/// 1.15 times that; it shows and hides with its far walls, so a dressed or damaged building's
/// massing is hidden with its box.
#[allow(clippy::too_many_arguments)]
fn show_massing(
    mut commands: Commands,
    mut shown: ResMut<Massing>,
    mats: Res<FarMaterials>,
    plan: Res<crate::world::Plan>,
    live: Res<Live>,
    seed: Res<crate::world::Seed>,
    cfg: Res<crate::sim::Data>,
    ents: Res<BuildingEntities>,
    camera: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    vis: Query<&Visibility>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(material), Some(here)) = (mats.massing.clone(), player_xz(&camera)) else {
        return;
    };
    let cfg = &cfg.0.world.dressing;
    let rng = SimRng::new(seed.0).fork("dressing");
    let mut near: Vec<u32> = plan
        .0
        .collision
        .candidates(here, cfg.massing_radius_m)
        .into_iter()
        .filter(|&b| !shown.0.contains_key(&b) && plan.0.buildings[b as usize].footprint.centre().distance(here) <= cfg.massing_radius_m)
        .collect();
    // Nearest first, ties by id (never hash order), a few a frame: each is a whole dressing pass.
    near.sort_by(|&a, &b| {
        let d = |i: u32| plan.0.buildings[i as usize].footprint.centre().distance(here);
        d(a).total_cmp(&d(b)).then(a.cmp(&b))
    });
    for b in near.into_iter().take(cfg.massing_per_frame as usize) {
        let Some(&box_e) = ents.0.get(b as usize) else { continue };
        let c = &live.structures.buildings[b as usize];
        let fronts = plan.0.fronts.get(b as usize).copied().unwrap_or([cb_core::world::FRONT_LIMIT_M; 4]);
        let pieces = dressing::massing(b, c.n, c.origin, c.size, &rng, cfg, fronts, cfg.massing_pieces as usize);
        let e = commands
            .spawn((
                Name::new(format!("building {b} massing")),
                Mesh3d(meshes.add(massing_mesh(&pieces))),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                vis.get(box_e).copied().unwrap_or_default(),
            ))
            .id();
        shown.0.insert(b, e);
    }
    shown.0.retain(|&b, e| {
        let far = !cfg.shows(
            cfg.massing_radius_m,
            plan.0.buildings[b as usize].footprint.centre().distance(here),
            true,
        );
        if far {
            commands.entity(*e).despawn();
            return false;
        }
        if let (Some(&box_e), Ok(now)) = (ents.0.get(b as usize), vis.get(*e))
            && let Ok(want) = vis.get(box_e)
            && want != now
        {
            commands.entity(*e).insert(*want);
        }
        true
    });
}

/// The windows for the hour (day-night-cycle): the share lit now into the glass material and the far
/// and massing walls' materials, so every intact building's windows follow the clock (a damaged
/// building's glass never lights; its far walls are hidden once it is framed).
fn light_windows(
    data: Res<crate::sim::Data>,
    light: Res<crate::day_night::Light>,
    shared: Res<Materials>,
    far: Res<FarMaterials>,
    mut glasses: ResMut<Assets<BuildingMaterial>>,
    mut arrays: ResMut<Assets<ArrayMaterial>>,
) {
    let w = &data.0.day_night.windows;
    let want = Vec4::new(w.lit_share(light.0.hour), w.cd_m2, data.0.world.storey_height_m, w.column_m);
    if let Some(handle) = shared.glass.as_ref()
        && glasses.get(handle).is_some_and(|m| m.extension.windows != want)
        && let Some(mut m) = glasses.get_mut(handle)
    {
        m.extension.windows = want;
    }
    for handle in [&far.far, &far.massing].into_iter().flatten() {
        if arrays.get(handle).is_some_and(|m| m.extension.windows != want)
            && let Some(mut m) = arrays.get_mut(handle)
        {
            m.extension.windows = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    use cb_core::world::StructureConfig;

    fn frame() -> Frame {
        Frame::new(3, [4, 9, 3], Vec3::ZERO, Vec3::new(6.0, 3.5, 6.0), &StructureConfig::default())
    }

    fn site(f: &Frame) -> Site {
        Site {
            id: f.building,
            n: f.n,
            origin: f.origin,
            size: f.size,
            street: 0,
            fronts: [cb_core::world::FRONT_LIMIT_M; 4],
        }
    }

    fn meshes_of(f: &Frame) -> Vec<(Look, Mesh)> {
        frame_meshes(f, &site(f), &InteriorConfig::default(), 0, [1.0; 3], 0.3, &SimRng::new(1))
    }

    /// The slots a mesh's vertices carry.
    fn slots(mesh: &Mesh) -> Vec<u32> {
        match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(VertexAttributeValues::Float32x4(c)) => c.iter().map(|c| c[3] as u32).collect(),
            _ => panic!("no colours"),
        }
    }

    #[test]
    fn every_vertex_carries_the_slot_of_its_element() {
        let f = frame();
        let meshes = meshes_of(&f);
        let all: Vec<u32> = meshes.iter().flat_map(|(_, m)| slots(m)).collect();
        assert!(
            all.iter().all(|&s| s as usize <= f.elements.len()),
            "every slot names an element or 0"
        );
        for id in 0..f.elements.len() as u32 {
            assert!(all.contains(&(id + 1)), "element {id} is drawn with its slot");
        }
        // Rooms and furniture on a slab carry that slab's slot: in the rooms mesh, every vertex of
        // the wood layer (furniture only) carries a slab's slot or 0 (the ground floor).
        let frame_mesh = &meshes.iter().find(|(l, _)| *l == Look::Rooms).unwrap().1;
        let layers: Vec<u32> = match frame_mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
            Some(VertexAttributeValues::Float32x2(u)) => u.iter().map(|u| u[0] as u32).collect(),
            _ => panic!("no layers"),
        };
        let wood = Layer::Surface(Surface::Wood).index();
        let on_wood: Vec<u32> = slots(frame_mesh)
            .into_iter()
            .zip(&layers)
            .filter(|(_, l)| **l == wood)
            .map(|(s, _)| s)
            .collect();
        assert!(!on_wood.is_empty(), "an office has wooden furniture");
        for s in on_wood {
            assert!(
                s == 0 || f.elements[s as usize - 1].kind == Kind::Slab,
                "furniture rests on a slab or the ground"
            );
        }
    }

    #[test]
    fn rooms_are_their_own_mesh() {
        let f = frame();
        let meshes = meshes_of(&f);
        let frame_mesh = &meshes.iter().find(|(l, _)| *l == Look::Frame).unwrap().1;
        for s in slots(frame_mesh) {
            assert!(
                matches!(f.elements[s as usize - 1].kind, Kind::Column | Kind::Slab),
                "the frame mesh is columns and slabs"
            );
        }
        assert!(
            meshes.iter().any(|(l, m)| *l == Look::Rooms && m.count_vertices() > 0),
            "the rooms are their own mesh"
        );
        let cfg = DressingConfig::default();
        assert!(rooms_shown(249.0, false, &cfg), "rooms show within 250 m");
        assert!(!rooms_shown(271.0, true, &cfg), "and hide past 270 m");
        assert!(
            rooms_shown(260.0, true, &cfg) && !rooms_shown(260.0, false, &cfg),
            "in between they keep their state"
        );
    }

    #[test]
    fn the_facade_mesh_is_its_walls_only() {
        let f = frame();
        let meshes = meshes_of(&f);
        let facade = &meshes.iter().find(|(l, _)| *l == Look::Facade).unwrap().1;
        for s in slots(facade) {
            assert_eq!(f.elements[s as usize - 1].kind, Kind::Wall);
        }
    }

    #[test]
    fn every_look_has_a_layer() {
        let mut seen: Vec<u32> = Surface::ALL.iter().map(|s| Layer::Surface(*s).index()).collect();
        seen.extend([Mat::Brick, Mat::Concrete, Mat::Roof, Mat::Wood, Mat::Trim].map(|m| Layer::Skin(m).index()));
        seen.push(Layer::Facade.index());
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), seen.len(), "no two looks share a layer");
        let sources = Surface::ALL.len() + 6;
        assert_eq!(
            sorted,
            (0..sources as u32).collect::<Vec<_>>(),
            "the layers are the sources, in order"
        );
    }

    #[test]
    fn dressing_hangs_each_piece_on_its_element() {
        let f = frame();
        let cfg = DressingConfig::default();
        let rng = SimRng::new(1);
        let all = dressing::dress(3, f.n, f.origin, f.size, &rng, true, &cfg, [cb_core::world::FRONT_LIMIT_M; 4], None);
        let hung: Vec<f32> = all.iter().filter_map(|p| anchor_slot(Some(&f), 0, p.anchor)).collect();
        assert!(!hung.is_empty());
        assert!(hung.iter().all(|&s| s as usize <= f.elements.len()));
        assert!(hung.iter().any(|&s| s > 0.0), "pieces hang on elements once there is a frame");
        assert_eq!(anchor_slot(None, 0, Anchor::Whole), Some(0.0), "no frame: always shown");
        assert_eq!(anchor_slot(Some(&f), 0, Anchor::Whole), Some(0.0));
        let shifted: Vec<f32> = all.iter().filter_map(|p| anchor_slot(Some(&f), 500, p.anchor)).collect();
        assert!(
            shifted.iter().zip(&hung).all(|(s, h)| *s == 0.0 || *s == h + 500.0),
            "a building's slots start at its range"
        );
    }
}
