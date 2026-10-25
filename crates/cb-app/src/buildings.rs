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

use crate::building_state::{BuildingMaterial, BuildingStates, ElementState, slot, standing_words};
use crate::sim::{PresentSet, SimSet};
use crate::world::{Live, MeshBuilder};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use cb_core::rng::SimRng;
use cb_core::world::destruct::Cells;
use cb_core::world::dressing::{self, Anchor, DressingConfig, Mat, Shape};
use cb_core::world::interior::{self, Rest, StoreyPlan, Surface};
use cb_core::world::structure::{Frame, GROUND, Kind};
use std::collections::{BTreeMap, HashMap};

/// Materials buildings are drawn with: the facade tones (by id), the dressing and the surfaces.
#[derive(Resource)]
pub struct BuildingLooks {
    /// Facade materials, picked by building id modulo the count.
    pub tones: Vec<Handle<StandardMaterial>>,
    /// Structure and furniture surfaces, each a textured PBR set (`building-surfaces`).
    pub surfaces: BTreeMap<Surface, Handle<StandardMaterial>>,
    /// Dressing materials: brick, concrete, roof and wood textured, trim plain, glass see-through.
    pub skin: BTreeMap<Mat, Handle<StandardMaterial>>,
}

impl BuildingLooks {
    /// The base look of a building mesh of `look`.
    fn base(&self, look: Look) -> &Handle<StandardMaterial> {
        match look {
            Look::Skin(m) => &self.skin[&m],
            Look::Surface(s) | Look::Room(s) => &self.surfaces[&s],
            Look::Facade(t) => &self.tones[t as usize % self.tones.len()],
        }
    }

    /// The facade look of building `id`.
    fn facade(&self, id: u32) -> Look {
        Look::Facade((id as usize % self.tones.len()) as u8)
    }
}

/// What one of a building's meshes is made of: a dressing material, a structure or furniture
/// surface, or the plain facade panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Look {
    /// The skin, roof and props.
    Skin(Mat),
    /// Columns and slabs.
    Surface(Surface),
    /// Rooms and furniture (`building-lod`): drawn near the pilot only, casting no shadows.
    Room(Surface),
    /// The frame's facade panels, in a building tone (hidden while it is dressed).
    Facade(u8),
}

/// The building materials, one per look, shared by every building (so their draws batch), all
/// reading the one element state buffer (`BuildingStates`). Made as they are first needed.
#[derive(Resource, Default)]
struct Materials {
    /// The buffer they read.
    standing: Option<Handle<ShaderBuffer>>,
    made: BTreeMap<Look, Handle<BuildingMaterial>>,
}

impl Materials {
    /// Points every material at the state buffer's current handle (it changes when it grows).
    fn follow(&mut self, states: &BuildingStates, materials: &mut Assets<BuildingMaterial>) {
        if self.standing.as_ref() == Some(&states.handle) {
            return;
        }
        self.standing = Some(states.handle.clone());
        for m in self.made.values() {
            if let Some(mut m) = materials.get_mut(m) {
                m.extension.standing = states.handle.clone();
            }
        }
    }

    /// The material for `look`.
    fn get(
        &mut self,
        look: Look,
        looks: &BuildingLooks,
        bases: &Assets<StandardMaterial>,
        materials: &mut Assets<BuildingMaterial>,
    ) -> Handle<BuildingMaterial> {
        let standing = self.standing.clone().unwrap_or_default();
        // A room reads its surface's material, so rooms and structure batch together.
        let key = match look {
            Look::Room(s) => Look::Surface(s),
            l => l,
        };
        self.made
            .entry(key)
            .or_insert_with(|| {
                materials.add(BuildingMaterial {
                    base: bases.get(looks.base(look)).cloned().unwrap_or_default(),
                    extension: ElementState {
                        standing,
                        no_ssao: u32::from(look == Look::Skin(Mat::Glass)),
                    },
                })
            })
            .clone()
    }
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

/// The surfaces' materials: each a CC0 PBR set under `assets/textures/building/<surface>/` (colour,
/// normal, packed occlusion-roughness-metal) with a full mip chain, white so the vertex colour
/// tints it; a plain material if a set cannot be read.
pub fn surface_materials(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> BTreeMap<Surface, Handle<StandardMaterial>> {
    let root = crate::cli::repo_root().join("assets");
    Surface::ALL
        .into_iter()
        .map(|s| {
            let dir = format!("textures/building/{}", format!("{s:?}").to_lowercase());
            let mut map =
                |file: &str, srgb: bool| crate::pbr_set::load_tiling(&root, &format!("{dir}/{file}"), srgb).map(|i| images.add(i));
            let orm = map("orm.png", false);
            let material = StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: map("albedo.jpg", true),
                normal_map_texture: map("normal_gl.png", false),
                occlusion_texture: orm.clone(),
                // The ORM map's G and B scale these: rough and dielectric where it says so.
                perceptual_roughness: if orm.is_some() { 1.0 } else { 0.85 },
                metallic: if orm.is_some() { 1.0 } else { 0.0 },
                metallic_roughness_texture: orm,
                ..default()
            };
            (s, materials.add(material))
        })
        .collect()
}

/// Builds one mesh per surface.
#[derive(Default)]
struct SurfaceMeshes(BTreeMap<Surface, MeshBuilder>);

impl SurfaceMeshes {
    /// Adds a box of `surface`, tinted `color`, carrying `slot` (`building_state`).
    fn block(&mut self, surface: Surface, min: Vec3, max: Vec3, color: [f32; 3], slot: f32) {
        let t = surface_tile_m(surface);
        self.0
            .entry(surface)
            .or_insert_with(|| MeshBuilder::tiled(t, t))
            .block(min, max, [color[0], color[1], color[2], slot]);
    }

    /// The rooms of one storey: its walls and furniture, each carrying the slot of what it rests on.
    fn storey(&mut self, plan: &StoreyPlan, slot_of: impl Fn(Rest) -> f32) {
        for w in &plan.walls {
            let s = slot_of(w.rest);
            if w.core {
                self.block(Surface::Concrete, w.min, w.max, CORE_TINT, s);
            } else {
                self.block(Surface::Plaster, w.min, w.max, PARTITION_TINT, s);
            }
        }
        for it in &plan.items {
            let s = slot_of(it.rest);
            for (lo, hi, col, surface) in &it.boxes {
                self.block(*surface, *lo, *hi, *col, s);
            }
        }
    }

    /// The meshes, with tangents for the normal maps.
    fn build(self) -> Vec<(Surface, Mesh)> {
        self.0.into_iter().map(|(s, mb)| (s, mb.build_with_tangents())).collect()
    }
}

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
struct Dressed(HashMap<u32, Dressing>);

/// Set when every building near the player is dressed (none waits for its turn), so a capture
/// can wait for the street to finish dressing on a slow renderer.
#[derive(Resource, Default)]
pub struct DressingSettled(pub bool);

/// Most buildings dressed in one frame (a 60-storey tower's skin and rooms are a few milliseconds).
const DRESS_PER_FRAME: usize = 1;

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
            .init_resource::<DressingSettled>()
            .add_systems(FixedUpdate, step_structures.in_set(SimSet::World))
            .add_systems(
                Update,
                (dress_near_player, redraw_buildings, show_rooms_near)
                    .chain()
                    .in_set(PresentSet::Pose),
            );
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

/// The meshes of a frame, built once (`building-gpu-state`): its facade panels, and its floors,
/// columns and the rooms and furniture behind them per surface, every vertex carrying the slot of
/// its element (rooms and furniture: the slab under them, or 0 on the ground). Columns are drawn
/// 1 cm thinner than they are and side walls stop short of the corner walls, so no two faces share
/// a plane (CLAUDE.md section 10).
fn frame_meshes(f: &Frame, base: u32, facade_look: Look, wall_m: f32, seed: &SimRng) -> Vec<(Look, Mesh)> {
    let mut facade = MeshBuilder::tiled(4.0, 3.5);
    let mut bare = SurfaceMeshes::default();
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
                facade.block(lo, hi, [1.0, 1.0, 1.0, s]);
            }
            Kind::Column => bare.block(
                Surface::Concrete,
                e.min + Vec3::new(0.01, 0.0, 0.01),
                e.max - Vec3::new(0.01, 0.0, 0.01),
                CORE_TINT,
                s,
            ),
            Kind::Slab => bare.block(Surface::Concrete, e.min, e.max, SLAB_TINT, s),
        }
    }
    let rest_slot = |r: Rest| match r {
        Rest::Ground => 0.0,
        Rest::Slab { k, i, j } => slot(base, f.slab(k as usize, i as usize, j as usize)),
    };
    let mut rooms = SurfaceMeshes::default();
    for k in 0..f.n[1] {
        rooms.storey(&interior::plan_storey(f.building, k, f.n, f.origin, f.size, seed), rest_slot);
    }
    std::iter::once((facade_look, facade.build()))
        .chain(bare.build().into_iter().map(|(s, m)| (Look::Surface(s), m)))
        .chain(rooms.build().into_iter().map(|(s, m)| (Look::Room(s), m)))
        .collect()
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

/// The dressing meshes of building `id`, one per material: cut per element, every vertex carrying
/// its element's slot, once it has a frame; whole, with its floors, rooms and furniture behind the
/// glass while it is whole (a damaged building draws its rooms with its frame).
fn dressing_meshes(c: &Cells, id: u32, base: u32, seed: &SimRng, cfg: &DressingConfig) -> Vec<(Look, Mesh)> {
    let f = c.frame.as_deref();
    let mut builders: BTreeMap<Mat, MeshBuilder> = BTreeMap::new();
    for p in dressing::dress(id, c.n, c.origin, c.size, seed, f.is_some(), cfg) {
        let Some(s) = anchor_slot(f, base, p.anchor) else { continue };
        let mb = builders
            .entry(p.mat)
            .or_insert_with(|| MeshBuilder::tiled(tile_m(p.mat), tile_m(p.mat)));
        let col = [p.color[0], p.color[1], p.color[2], s];
        match p.shape {
            Shape::Block { min, max } => mb.block(min, max, col),
            Shape::Drum { base, r0, r1, height_m } => mb.drum(base, r0, r1, height_m, 14, col),
        }
    }
    let mut rooms = SurfaceMeshes::default();
    if f.is_none() {
        for k in 0..c.n[1] {
            rooms.storey(&interior::plan_storey(id, k, c.n, c.origin, c.size, seed), |_| 0.0);
        }
    }
    builders
        .into_iter()
        .map(|(mat, mb)| {
            let mesh = if matches!(mat, Mat::Brick | Mat::Concrete | Mat::Roof | Mat::Wood) {
                mb.build_with_tangents()
            } else {
                mb.build()
            };
            (Look::Skin(mat), mesh)
        })
        .chain(rooms.build().into_iter().map(|(s, mesh)| (Look::Room(s), mesh)))
        .collect()
}

/// Shows `parts` in `drawn`, one entity per look, each in its material; a look already drawn has
/// its mesh replaced in place and keeps its entity, one that no longer appears is emptied.
fn show_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    drawn: &mut BTreeMap<Look, Drawn>,
    mut parts: Vec<(Look, Mesh)>,
    mut material: impl FnMut(Look) -> Handle<BuildingMaterial>,
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
                commands.entity(d.entity).insert(MeshMaterial3d(m));
                d.clone()
            }
            None => {
                let handle = meshes.add(mesh);
                let mut e = commands.spawn((
                    Name::new(format!("{name} {k:?}")),
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(m),
                    Transform::IDENTITY,
                ));
                if matches!(k, Look::Room(_)) {
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
            if !matches!(look, Look::Room(_)) {
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

/// Dresses the buildings near the player, a few a frame, and undresses those left behind (their
/// plain box shows again). Far buildings never get generated detail (owner, 2026-10-24: "far away
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
    mut dressed: ResMut<Dressed>,
    mut settled: ResMut<DressingSettled>,
    mut shared: ResMut<Materials>,
    states: Res<BuildingStates>,
    mut meshes: ResMut<Assets<Mesh>>,
    bases: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<BuildingMaterial>>,
) {
    let _t = crate::perf_hud::slow("dress_near_player");
    settled.0 = false;
    let (Some(looks), Some(here)) = (looks, player_xz(&camera)) else {
        return;
    };
    if ents.0.is_empty() {
        return;
    }
    let cfg = &cfg.0.world.dressing;
    let rng = SimRng::new(seed.0).fork("dressing");
    let mut near: Vec<u32> = plan
        .0
        .collision
        .candidates(here, cfg.dress_radius_m)
        .into_iter()
        .filter(|&b| !dressed.0.contains_key(&b) && plan.0.buildings[b as usize].footprint.centre().distance(here) <= cfg.dress_radius_m)
        .collect();
    // Nearest first, ties by id (never hash order).
    near.sort_by(|&a, &b| {
        let (da, db) = (
            plan.0.buildings[a as usize].footprint.centre().distance(here),
            plan.0.buildings[b as usize].footprint.centre().distance(here),
        );
        da.total_cmp(&db).then(a.cmp(&b))
    });
    settled.0 = near.len() <= DRESS_PER_FRAME;
    shared.follow(&states, &mut materials);
    for b in near.into_iter().take(DRESS_PER_FRAME) {
        let c = &live.structures.buildings[b as usize];
        if c.standing == 0 {
            continue;
        }
        if c.frame.is_some() {
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
        let mut parts = BTreeMap::new();
        show_parts(
            &mut commands,
            &mut meshes,
            &mut parts,
            dressing_meshes(c, b, 0, &rng, cfg),
            |l| shared.get(l, &looks, &bases, &mut materials),
            &format!("building {b}"),
        );
        dressed.0.insert(b, Dressing { parts, framed: false });
        commands.entity(ents.0[b as usize]).insert(Visibility::Hidden);
    }
    let keep = cfg.dress_radius_m * 1.15;
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
    mut meshes: ResMut<Assets<Mesh>>,
    bases: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<BuildingMaterial>>,
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
    let rng = SimRng::new(seed.0).fork("dressing");
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
        let base = states.base(id, f.elements.len(), &mut buffers);
        shared.follow(&states, &mut materials);
        states.write(id, &standing_words(f));
        let facade_look = looks.facade(id);
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
                    frame_meshes(f, base, facade_look, wall_m, &rng),
                    |l| shared.get(l, &looks, &bases, &mut materials),
                    &format!("building {id}"),
                );
                v.insert(fb)
            }
        };
        // A dressed building's skin is meshed once with its frame's slots, in its own materials.
        let skinned = match dressed.0.get_mut(&id) {
            Some(d) => {
                if !d.framed {
                    let parts = dressing_meshes(c, id, fb.base, &rng, &cfg.0.world.dressing);
                    show_parts(
                        &mut commands,
                        &mut meshes,
                        &mut d.parts,
                        parts,
                        |l| shared.get(l, &looks, &bases, &mut materials),
                        &format!("building {id}"),
                    );
                    d.framed = true;
                }
                true
            }
            None => false,
        };
        if let Some(facade) = fb.parts.get(&facade_look) {
            commands
                .entity(facade.entity)
                .insert(if skinned { Visibility::Hidden } else { Visibility::Inherited });
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
        let meshes = frame_meshes(&f, 0, Look::Facade(0), 0.3, &SimRng::new(1));
        let all: Vec<u32> = meshes.iter().flat_map(|(_, m)| slots(m)).collect();
        assert!(
            all.iter().all(|&s| s as usize <= f.elements.len()),
            "every slot names an element or 0"
        );
        for id in 0..f.elements.len() as u32 {
            assert!(all.contains(&(id + 1)), "element {id} is drawn with its slot");
        }
        // Rooms and furniture on a slab carry that slab's slot: the furniture surfaces carry only
        // slab slots and 0 (the ground floor).
        let wood = &meshes
            .iter()
            .find(|(l, _)| *l == Look::Room(Surface::Wood))
            .expect("an office has wood")
            .1;
        for s in slots(wood) {
            assert!(
                s == 0 || f.elements[s as usize - 1].kind == Kind::Slab,
                "furniture rests on a slab or the ground"
            );
        }
    }

    #[test]
    fn rooms_are_their_own_mesh() {
        let f = frame();
        let meshes = frame_meshes(&f, 0, Look::Facade(0), 0.3, &SimRng::new(1));
        for (look, mesh) in &meshes {
            match look {
                // The structure: columns and slabs only, so concrete only.
                Look::Surface(s) => {
                    assert_eq!(*s, Surface::Concrete);
                    for s in slots(mesh) {
                        assert!(matches!(f.elements[s as usize - 1].kind, Kind::Column | Kind::Slab));
                    }
                }
                Look::Room(_) | Look::Facade(_) => {}
                Look::Skin(_) => panic!("a frame has no skin"),
            }
        }
        assert!(
            meshes.iter().any(|(l, _)| *l == Look::Room(Surface::Wood)),
            "furniture is in the rooms"
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
        let meshes = frame_meshes(&f, 0, Look::Facade(0), 0.3, &SimRng::new(1));
        let facade = &meshes.iter().find(|(l, _)| *l == Look::Facade(0)).unwrap().1;
        for s in slots(facade) {
            assert_eq!(f.elements[s as usize - 1].kind, Kind::Wall);
        }
    }

    #[test]
    fn dressing_hangs_each_piece_on_its_element() {
        let f = frame();
        let cfg = DressingConfig::default();
        let rng = SimRng::new(1);
        let all = dressing::dress(3, f.n, f.origin, f.size, &rng, true, &cfg);
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
