//! Draws the houses and small businesses on their lots (openspec change `small-buildings`, design sections
//! 7.3, 7.5 and 7.6). The rules (where they stand, what stands, how they break) are `cb_core::world::small_buildings`;
//! this is the drawing.
//!
//! - Every lot gets its LOD1 shell (150-600 m) and LOD2 box (600 m-2 km) from its kit's glb, the meshes shared by
//!   every lot of that kit and the materials by every lot of the island (one per surface), so they batch.
//! - Within `DRESS_M` of the camera a lot also gets its LOD0 (within 150 m), its door leaves (swung to the angle of
//!   its doors when the pilot is on foot beside it) and its furniture.
//! - Furniture is one mesh a building in one material for all of it (owner, 2026-10-26: "I'd prefer a single
//!   material for all furniture if possible, they should all be 1 draw call"): the modelled pieces
//!   (`assets/models/furniture/`, on one atlas) within `REAL_FURNITURE_M` of a pilot on foot, the kit's low-poly
//!   blocks (on the same atlas's swatches) within `FURNITURE_M` otherwise, and in the mech only ever the blocks
//!   (owner: "real furniture should not be used when in mech tho").
//! - Every vertex of the furniture, and of a damaged building's own LOD0, carries the slot of its piece in the one
//!   building state buffer (`building_state`), so a broken piece disappears by a word written there, never by
//!   rebuilding a mesh (owner: "furniture are part of the building cells anyways, so that should save on perf when
//!   updating the state"). A damaged building stays at LOD0 for good.

use crate::building_state::{BuildingMaterial, BuildingStates, ElementState, slot};
use crate::pilot::{PilotCamera, PilotSim};
use crate::state::GameApp;
use crate::walk_in::InteriorColliders;
use crate::world::{Live, Plan};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::{Gltf, GltfMaterial, GltfMesh};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use cb_core::pilot::PilotMode;
use cb_core::world::small_buildings::kit::Node;
use cb_core::world::small_buildings::{HOUSE_CHUNK_BASE, Kits, Lot};
use std::collections::{BTreeMap, HashMap};

/// LOD0, its leaves and its furniture are spawned for lots this near the camera...
const DRESS_M: f32 = 170.0;
/// ...and freed past this (a damaged one never is).
const UNDRESS_M: f32 = 200.0;
/// Most lots dressed in one frame.
const DRESS_PER_FRAME: usize = 8;
/// The block furniture is drawn within this.
const FURNITURE_M: f32 = 100.0;
/// The modelled furniture is drawn within this of a pilot on foot (survey S5).
const REAL_FURNITURE_M: f32 = 60.0;
/// Most furniture meshes built in one frame.
const FURNISH_PER_FRAME: usize = 3;
/// The LOD switches (the kit's `within_m`), each blending over a tenth of its distance.
const LOD0_M: f32 = 150.0;
const LOD1_M: f32 = 600.0;
const LOD2_M: f32 = 2000.0;
/// Surfaces drawn see-through.
const GLASS: [&str; 4] = ["glass", "glassShop", "screen", "water"];
/// Surfaces that take no shadow (lamps, glass) or are inside (finishes and furniture) cast none.
const NO_SHADOW: [&str; 10] = [
    "glass",
    "glassShop",
    "screen",
    "water",
    "lamp",
    "plaster",
    "floorWood",
    "floorTile",
    "carpet",
    "woodInt",
];

/// The key a lot's words have in the building state buffer.
fn state_key(lot: u32) -> u32 {
    HOUSE_CHUNK_BASE + lot
}

/// The glbs as they load.
#[derive(Resource)]
struct Loading {
    kits: Vec<Handle<Gltf>>,
    furniture: Vec<(String, Handle<Gltf>)>,
}

/// One primitive of a kit's node: its mesh and its surface.
#[derive(Clone)]
struct Prim {
    mesh: Handle<Mesh>,
    surface: String,
}

/// A kit's meshes by node.
#[derive(Default)]
struct KitDraw {
    lod0: Vec<Prim>,
    lod1: Vec<Prim>,
    lod2: Vec<Prim>,
    furn: Vec<Prim>,
    leaves: Vec<Vec<Prim>>,
}

/// The kits' meshes and the island's materials, once loaded.
#[derive(Resource, Default)]
pub struct HouseKits {
    /// True once everything is in (the loading screen waits for it).
    pub ready: bool,
    kits: Vec<KitDraw>,
    /// Each modelled item's mesh (main-world copy, for merging).
    furniture: HashMap<String, Handle<Mesh>>,
    /// One plain material per surface (shells, whole LOD0s, leaves).
    plain: HashMap<String, Handle<StandardMaterial>>,
    /// One state-reading material per surface (damaged LOD0s).
    state: HashMap<String, Handle<BuildingMaterial>>,
    /// The one furniture material (the atlas).
    furniture_material: Option<Handle<BuildingMaterial>>,
    /// The state buffer the materials read.
    standing: Option<Handle<ShaderBuffer>>,
    /// The surfaces drawn with a texture set (their meshes need tangents).
    textured: Vec<String>,
}

/// Which furniture a building draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Furnished {
    Blocks,
    Real,
}

/// A dressed lot's entities.
struct Dressed {
    lod0: Vec<Entity>,
    /// Its own LOD0 (damaged): slots, never freed.
    own: bool,
    leaves: Vec<(usize, Entity)>,
    furniture: Option<(Furnished, Entity)>,
    version: u32,
}

/// Every lot's root, and the dressed ones.
#[derive(Resource, Default)]
struct Houses {
    roots: Vec<Entity>,
    far: Vec<Vec<Entity>>,
    dressed: BTreeMap<u32, Dressed>,
}

/// Draws the houses and small businesses.
pub struct SmallBuildingsPlugin;

impl Plugin for SmallBuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HouseKits>()
            .init_resource::<Houses>()
            .add_systems(Startup, start_loading)
            .add_systems(Update, bind.run_if(|k: Res<HouseKits>| !k.ready))
            .add_game_systems(
                Update,
                (spawn_lots, follow_state, redraw_damaged, dress, furnish, swing_leaves)
                    .chain()
                    .in_set(crate::sim::PresentSet::Pose)
                    .run_if(|k: Res<HouseKits>| k.ready),
            );
    }
}

fn start_loading(mut commands: Commands, plan: Res<Plan>, server: Res<AssetServer>) {
    let kits = &plan.small.kits;
    commands.insert_resource(Loading {
        kits: kits
            .kits
            .iter()
            .map(|k| server.load(format!("models/small_buildings/{}.glb", k.id)))
            .collect(),
        furniture: kits
            .furniture
            .items
            .iter()
            .map(|(name, it)| (name.clone(), server.load(format!("models/{}", it.file))))
            .collect(),
    });
}

/// The surface a glb primitive is drawn with: its `extras.surface`, else its material's name.
fn surface_of(p: &bevy::gltf::GltfPrimitive) -> String {
    p.extras
        .as_ref()
        .and_then(|e| serde_json_surface(&e.value))
        .unwrap_or_else(|| p.name.clone())
}

/// `{"surface": "brick", ...}` -> `brick`, without a JSON dependency (the exporter writes it on one line).
fn serde_json_surface(extras: &str) -> Option<String> {
    extras_text(extras, "surface")
}

/// A string field of the exporter's one-line extras.
fn extras_text(extras: &str, key: &str) -> Option<String> {
    let at = extras.find(&format!("\"{key}\""))?;
    let rest = &extras[at + key.len() + 2..];
    let rest = &rest[rest.find(':')? + 1..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')?;
    Some(rest[start..start + end].to_string())
}

/// A number field of the exporter's one-line extras.
fn extras_number(extras: &str, key: &str) -> Option<f32> {
    let at = extras.find(&format!("\"{key}\""))?;
    let rest = &extras[at + key.len() + 2..];
    let rest = rest[rest.find(':')? + 1..].trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// A surface's texture set (`assets/textures/houses/<generator>/`, Material Maker: design section 3) and how many
/// metres one tile of it covers.
struct SurfaceTextures {
    albedo: Handle<Image>,
    normal: Handle<Image>,
    orm: Handle<Image>,
    tile_m: f32,
    /// The ORM map's mean roughness (G) and metal (B), 0..1.
    rough: f32,
    metal: f32,
    /// The albedo's mean colour, sRGB 0..1.
    mean: Vec3,
}

/// An albedo's mean colour (sRGB, 0..1) over its top level.
fn mean_colour(img: &Image) -> Vec3 {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let Some(data) = img.data.as_ref().filter(|d| d.len() >= w * h * 4 && w * h > 0) else {
        return Vec3::ONE;
    };
    let mut sum = [0u64; 3];
    for px in data[..w * h * 4].as_chunks::<4>().0 {
        for (k, total) in sum.iter_mut().enumerate() {
            *total += u64::from(px[k]);
        }
    }
    let n = (w * h) as f32 * 255.0;
    Vec3::new(sum[0] as f32 / n, sum[1] as f32 / n, sum[2] as f32 / n)
}

/// A textured surface's colour factor. A set that carries its own colour (brick, asphalt: a saturated mean) draws
/// white, the vertex colour giving the paint; a near-neutral set (planks shared by the deck and the floor, plain
/// paint) takes the surface's own stand-in hue over it, so a wood floor stays brown, scaled so the set's mean comes
/// out at the stand-in.
fn tint_over(mean: Vec3, standin: Color) -> Color {
    let saturation = mean.max_element() - mean.min_element();
    if saturation > 0.08 {
        return Color::WHITE;
    }
    let want = standin.to_srgba();
    let luma = (0.2126 * mean.x + 0.7152 * mean.y + 0.0722 * mean.z).max(0.05);
    let k = |c: f32| (c / luma).clamp(0.0, 1.0);
    Color::srgb(k(want.red), k(want.green), k(want.blue))
}

/// The mean of an ORM image's roughness (G) and metal (B) channels over its top level.
fn mean_roughness_metal(orm: &Image) -> (f32, f32) {
    let (w, h) = (orm.width() as usize, orm.height() as usize);
    let Some(data) = orm.data.as_ref().filter(|d| d.len() >= w * h * 4 && w * h > 0) else {
        return (1.0, 0.0);
    };
    let (mut g, mut b) = (0u64, 0u64);
    for px in data[..w * h * 4].as_chunks::<4>().0 {
        g += u64::from(px[1]);
        b += u64::from(px[2]);
    }
    let n = (w * h) as f32 * 255.0;
    (g as f32 / n, b as f32 / n)
}

/// Once every glb is in: each kit's meshes by node, a plain and a state material per surface (the glb's stand-in
/// colours), and the furniture material on the atlas.
#[allow(clippy::too_many_arguments)]
fn bind(
    loading: Option<Res<Loading>>,
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    nodes: Res<Assets<bevy::gltf::GltfNode>>,
    mut kits: ResMut<HouseKits>,
    mut plain: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<Assets<BuildingMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    states: Res<BuildingStates>,
) {
    let Some(loading) = loading else { return };
    let all = loading.kits.iter().chain(loading.furniture.iter().map(|(_, h)| h));
    if !all.clone().all(|h| server.is_loaded_with_dependencies(h)) {
        return;
    }
    let mut draws = Vec::with_capacity(loading.kits.len());
    let mut materials_of: HashMap<String, GltfMaterial> = HashMap::new();
    // Each surface's generator and tile size, from its material's extras.
    let mut look_of: HashMap<String, (String, f32)> = HashMap::new();
    for h in &loading.kits {
        let Some(g) = gltfs.get(h) else { return };
        let mut d = KitDraw::default();
        for (name, nh) in &g.named_nodes {
            let Some(node) = nodes.get(nh) else { continue };
            let Some(mesh) = node.mesh.as_ref().and_then(|m| gltf_meshes.get(m)) else {
                continue;
            };
            let prims: Vec<Prim> = mesh
                .primitives
                .iter()
                .map(|p| {
                    let surface = surface_of(p);
                    if let Some(m) = p.material.as_ref().and_then(|m| gltf_materials.get(m)) {
                        materials_of.entry(surface.clone()).or_insert_with(|| m.clone());
                    }
                    if let Some(x) = p.material_extras.as_ref()
                        && let (Some(g), Some(t)) = (extras_text(&x.value, "generator"), extras_number(&x.value, "tile_m"))
                    {
                        look_of.entry(surface.clone()).or_insert((g, t));
                    }
                    Prim {
                        mesh: p.mesh.clone(),
                        surface,
                    }
                })
                .collect();
            match name.as_ref() {
                "LOD0" => d.lod0 = prims,
                "LOD1" => d.lod1 = prims,
                "LOD2" => d.lod2 = prims,
                "FURN" => d.furn = prims,
                n if n.starts_with("LEAF") => {
                    let i: usize = n[4..].parse().unwrap_or(0);
                    if d.leaves.len() <= i {
                        d.leaves.resize(i + 1, Vec::new());
                    }
                    d.leaves[i] = prims;
                }
                _ => {}
            }
        }
        draws.push(d);
    }
    // Each generator's texture set, where it has been made (one set serves every surface of that generator).
    let root = crate::cli::repo_root().join("assets");
    type Set = (Handle<Image>, Handle<Image>, Handle<Image>, f32, f32, Vec3);
    let mut sets: HashMap<String, Option<Set>> = HashMap::new();
    let mut textures_of = |generator: &str, images: &mut Assets<Image>| {
        sets.entry(generator.to_string())
            .or_insert_with(|| {
                let dir = format!("textures/houses/{generator}");
                if !root.join(&dir).join("albedo.png").exists() {
                    return None;
                }
                let read = |name: &str, srgb: bool| crate::pbr_set::load_tiling(&root, &format!("{dir}/{name}"), srgb);
                let (albedo, normal, orm) = (read("albedo.png", true)?, read("normal.png", false)?, read("orm.png", false)?);
                let (rough, metal) = mean_roughness_metal(&orm);
                let mean = mean_colour(&albedo);
                Some((images.add(albedo), images.add(normal), images.add(orm), rough, metal, mean))
            })
            .clone()
    };
    let mut textured: Vec<String> = Vec::new();
    for (surface, m) in &materials_of {
        let glass = GLASS.contains(&surface.as_str());
        let set = look_of.get(surface).and_then(|(g, t)| {
            textures_of(g, &mut images).map(|(albedo, normal, orm, rough, metal, mean)| SurfaceTextures {
                albedo,
                normal,
                orm,
                tile_m: t.max(0.05),
                rough,
                metal,
                mean,
            })
        });
        if set.is_some() {
            textured.push(surface.clone());
        }
        // A textured surface: white, so the vertex colour (the paint) shows over the albedo; the maps repeat every
        // `tile_m` metres (the kit's UVs are metres). Several surfaces share a set (paint, plaster and metal are all
        // `smooth`), so the ORM map's roughness and metal are scaled to each surface's own (the set's mean times the
        // factor is the surface's value); a metal surface on a set with no metal in it keeps its plain numbers.
        let metal_from_map = set.as_ref().is_some_and(|t| t.metal > 0.05 || m.metallic < 0.05);
        let base = StandardMaterial {
            base_color: set.as_ref().map_or(m.base_color, |t| tint_over(t.mean, m.base_color)),
            base_color_texture: set.as_ref().map(|t| t.albedo.clone()),
            normal_map_texture: set.as_ref().map(|t| t.normal.clone()),
            metallic_roughness_texture: set.as_ref().filter(|_| metal_from_map).map(|t| t.orm.clone()),
            occlusion_texture: set.as_ref().map(|t| t.orm.clone()),
            uv_transform: set.as_ref().map_or(bevy::math::Affine2::IDENTITY, |t| {
                bevy::math::Affine2::from_scale(Vec2::splat(1.0 / t.tile_m))
            }),
            perceptual_roughness: match &set {
                Some(t) if metal_from_map => (m.perceptual_roughness / t.rough.max(0.05)).min(4.0),
                _ => m.perceptual_roughness,
            },
            metallic: match &set {
                Some(t) if metal_from_map && t.metal > 0.05 => (m.metallic / t.metal).min(4.0),
                _ => m.metallic,
            },
            emissive: m.emissive,
            alpha_mode: if glass { AlphaMode::Blend } else { AlphaMode::Opaque },
            double_sided: m.double_sided,
            cull_mode: if m.double_sided {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            ..default()
        };
        kits.plain.insert(surface.clone(), plain.add(base.clone()));
        kits.state.insert(
            surface.clone(),
            state.add(BuildingMaterial {
                base,
                extension: ElementState {
                    standing: states.handle.clone(),
                    no_ssao: u32::from(glass),
                    // Houses' windows are not lit yet (day-night-cycle task 4.5).
                    windows: Vec4::ZERO,
                },
            }),
        );
    }
    // A normal map needs tangents, which the kits do not store: made once per shared mesh of a textured surface.
    let mut tangents = 0;
    for d in &draws {
        for p in d.lod0.iter().chain(&d.lod1).chain(&d.lod2).chain(d.leaves.iter().flatten()) {
            if textured.contains(&p.surface)
                && let Some(mut m) = meshes.get_mut(&p.mesh)
                && m.attribute(Mesh::ATTRIBUTE_TANGENT).is_none()
                && m.generate_tangents().is_ok()
            {
                tangents += 1;
            }
        }
    }
    info!(
        "small buildings: {} of {} surfaces textured, tangents for {tangents} meshes",
        textured.len(),
        materials_of.len()
    );
    kits.textured = textured;
    let mut atlas = |path: &str, srgb: bool| crate::pbr_set::load_tiling(&root, path, srgb).map(|i| images.add(i));
    let albedo = atlas("textures/furniture/albedo.png", true);
    let normal = atlas("textures/furniture/normal.png", false);
    let orm = atlas("textures/furniture/orm.png", false);
    kits.furniture_material = Some(state.add(BuildingMaterial {
        base: StandardMaterial {
            base_color_texture: albedo,
            normal_map_texture: normal,
            metallic_roughness_texture: orm.clone(),
            occlusion_texture: orm,
            perceptual_roughness: 1.0,
            metallic: 1.0,
            ..default()
        },
        extension: ElementState {
            standing: states.handle.clone(),
            no_ssao: 0,
            windows: Vec4::ZERO,
        },
    }));
    for (name, h) in &loading.furniture {
        let Some(g) = gltfs.get(h) else { return };
        if let Some(mesh) = g.meshes.first().and_then(|m| gltf_meshes.get(m)).and_then(|m| m.primitives.first()) {
            kits.furniture.insert(name.clone(), mesh.mesh.clone());
        }
    }
    kits.kits = draws;
    kits.standing = Some(states.handle.clone());
    kits.ready = true;
    info!(
        "small buildings: {} kits, {} furniture items, {} surfaces",
        kits.kits.len(),
        kits.furniture.len(),
        kits.plain.len()
    );
}

/// Points the state materials at the building state buffer's handle (it changes when the buffer grows).
fn follow_state(mut kits: ResMut<HouseKits>, states: Res<BuildingStates>, mut materials: ResMut<Assets<BuildingMaterial>>) {
    if kits.standing.as_ref() == Some(&states.handle) {
        return;
    }
    kits.standing = Some(states.handle.clone());
    for h in kits.state.values().chain(kits.furniture_material.iter()) {
        if let Some(mut m) = materials.get_mut(h) {
            m.extension.standing = states.handle.clone();
        }
    }
}

fn lot_transform(l: &Lot) -> Transform {
    Transform::from_translation(Vec3::new(l.centre.x, l.ground_m, l.centre.y)).with_rotation(Quat::from_rotation_y(l.heading_rad))
}

fn range(start: f32, end: f32) -> VisibilityRange {
    let (a, b) = (start * 0.1, end * 0.1);
    VisibilityRange {
        start_margin: if start > 0.0 { (start - a)..(start + a) } else { 0.0..0.0 },
        end_margin: (end - b)..(end + b),
        use_aabb: false,
    }
}

/// Spawns every lot's root with its LOD1 and LOD2, once.
fn spawn_lots(mut commands: Commands, plan: Res<Plan>, kits: Res<HouseKits>, mut houses: ResMut<Houses>) {
    if !houses.roots.is_empty() || plan.small.lots.is_empty() {
        return;
    }
    let _t = crate::perf_hud::slow("small buildings: spawn");
    for l in &plan.small.lots {
        let draw = &kits.kits[l.kit as usize];
        let mut far = Vec::new();
        let root = commands
            .spawn((lot_transform(l), Visibility::Visible))
            .with_children(|c| {
                for (prims, r) in [(&draw.lod1, range(LOD0_M, LOD1_M)), (&draw.lod2, range(LOD1_M, LOD2_M))] {
                    for p in prims {
                        let Some(m) = kits.plain.get(&p.surface) else { continue };
                        let mut e = c.spawn((Mesh3d(p.mesh.clone()), MeshMaterial3d(m.clone()), Transform::IDENTITY, r.clone()));
                        if NO_SHADOW.contains(&p.surface.as_str()) || std::ptr::eq(prims, &draw.lod2) {
                            e.insert(NotShadowCaster);
                        }
                        far.push(e.id());
                    }
                }
            })
            .id();
        houses.roots.push(root);
        houses.far.push(far);
    }
    info!("small buildings: {} lots spawned", houses.roots.len());
}

/// A mesh's positions, normals, UVs and colours, one vertex per triangle corner (no sharing, so every corner can
/// carry its own piece's slot).
struct Corners {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
}

impl Corners {
    fn new() -> Self {
        Self {
            pos: Vec::new(),
            nrm: Vec::new(),
            uv: Vec::new(),
            col: Vec::new(),
        }
    }

    fn mesh(self) -> Mesh {
        let n = self.pos.len() as u32;
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.col);
        m.insert_indices(Indices::U32((0..n).collect()));
        m
    }
}

/// A source mesh's attributes and triangle list.
struct Source<'a> {
    pos: &'a [[f32; 3]],
    nrm: &'a [[f32; 3]],
    uv: &'a [[f32; 2]],
    col: Vec<[f32; 4]>,
    tri: Vec<u32>,
}

fn source(m: &Mesh) -> Option<Source<'_>> {
    let VertexAttributeValues::Float32x3(pos) = m.attribute(Mesh::ATTRIBUTE_POSITION)? else {
        return None;
    };
    let VertexAttributeValues::Float32x3(nrm) = m.attribute(Mesh::ATTRIBUTE_NORMAL)? else {
        return None;
    };
    let VertexAttributeValues::Float32x2(uv) = m.attribute(Mesh::ATTRIBUTE_UV_0)? else {
        return None;
    };
    let col = match m.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(c)) => c.clone(),
        Some(VertexAttributeValues::Float32x3(c)) => c.iter().map(|c| [c[0], c[1], c[2], 1.0]).collect(),
        _ => vec![[1.0; 4]; pos.len()],
    };
    let tri = match m.indices()? {
        Indices::U16(i) => i.iter().map(|&v| v as u32).collect(),
        Indices::U32(i) => i.clone(),
    };
    Some(Source { pos, nrm, uv, col, tri })
}

/// Which piece each triangle of primitive `prim` of `node` belongs to (from the kit's runs).
fn piece_of_triangles(kit: &cb_core::world::small_buildings::Kit, node: Node, prim: usize, triangles: usize) -> Vec<u32> {
    let mut out = vec![u32::MAX; triangles];
    for (i, p) in kit.pieces.iter().enumerate() {
        if p.node != node {
            continue;
        }
        for r in &p.runs {
            if r[0] as usize == prim {
                for t in r[1]..(r[1] + r[2]).min(triangles as u32) {
                    out[t as usize] = i as u32;
                }
            }
        }
    }
    out
}

/// A damaged building's own LOD0: one mesh per surface, each corner carrying its piece's slot.
fn own_lod0(
    kit: &cb_core::world::small_buildings::Kit,
    draw: &KitDraw,
    meshes: &Assets<Mesh>,
    base: u32,
    textured: &[String],
) -> Vec<(String, Mesh)> {
    let mut out = Vec::new();
    for (pi, p) in draw.lod0.iter().enumerate() {
        let Some(src) = meshes.get(&p.mesh).and_then(source) else {
            continue;
        };
        let pieces = piece_of_triangles(kit, Node::Lod0, pi, src.tri.len() / 3);
        let mut c = Corners::new();
        for (t, tri) in src.tri.as_chunks::<3>().0.iter().enumerate() {
            let s = pieces[t];
            let a = if s == u32::MAX { 0.0 } else { slot(base, s) };
            for &v in tri {
                let v = v as usize;
                c.pos.push(src.pos[v]);
                c.nrm.push(src.nrm[v]);
                c.uv.push(src.uv[v]);
                let k = src.col[v];
                c.col.push([k[0], k[1], k[2], a]);
            }
        }
        let mut mesh = c.mesh();
        if textured.contains(&p.surface) {
            let _ = mesh.generate_tangents();
        }
        out.push((p.surface.clone(), mesh));
    }
    out
}

/// A building's furniture as one mesh on the atlas: the modelled pieces (`real`) or the kit's blocks on the atlas's
/// swatches, each corner carrying its piece's slot.
fn furniture_mesh(
    kit: &cb_core::world::small_buildings::Kit,
    kits: &Kits,
    draw: &KitDraw,
    models: &HashMap<String, Handle<Mesh>>,
    meshes: &Assets<Mesh>,
    base: u32,
    real: bool,
) -> Option<Mesh> {
    let mut c = Corners::new();
    if real {
        for f in &kit.furniture {
            let Some(src) = models.get(&f.item).and_then(|h| meshes.get(h)).and_then(source) else {
                continue;
            };
            // Local (x, z) lies at (-z, x) for each quarter turn (the kit's `placeItems`): a turn of -90 degrees.
            let turn = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2 * f.quarter_turns as f32);
            let a = slot(base, f.piece);
            for &v in &src.tri {
                let v = v as usize;
                c.pos.push((f.at + turn * Vec3::from(src.pos[v])).to_array());
                c.nrm.push((turn * Vec3::from(src.nrm[v])).to_array());
                c.uv.push(src.uv[v]);
                let k = src.col[v];
                let tint = Vec3::ONE.lerp(Vec3::from(f.tint), k[3]);
                c.col.push([tint.x, tint.y, tint.z, a]);
            }
        }
    } else {
        for (pi, p) in draw.furn.iter().enumerate() {
            let Some(src) = meshes.get(&p.mesh).and_then(source) else {
                continue;
            };
            let Some(sw) = kits.furniture.swatches.get(&p.surface) else {
                continue;
            };
            let pieces = piece_of_triangles(kit, Node::Furn, pi, src.tri.len() / 3);
            for (t, tri) in src.tri.as_chunks::<3>().0.iter().enumerate() {
                let s = pieces[t];
                let a = if s == u32::MAX { 0.0 } else { slot(base, s) };
                // Each triangle's metre UVs from its own corner, two metres across the swatch.
                let lo = tri
                    .iter()
                    .fold(Vec2::splat(f32::MAX), |m, &v| m.min(Vec2::from(src.uv[v as usize])));
                for &v in tri {
                    let v = v as usize;
                    let q = ((Vec2::from(src.uv[v]) - lo) / 2.0).clamp(Vec2::ZERO, Vec2::ONE);
                    c.pos.push(src.pos[v]);
                    c.nrm.push(src.nrm[v]);
                    c.uv.push([
                        sw[0] + (sw[2] - sw[0]) * (0.1 + 0.8 * q.x),
                        sw[1] + (sw[3] - sw[1]) * (0.1 + 0.8 * q.y),
                    ]);
                    let k = src.col[v];
                    c.col.push([k[0], k[1], k[2], a]);
                }
            }
        }
    }
    if c.pos.is_empty() {
        return None;
    }
    let mut m = c.mesh();
    if let Err(e) = m.generate_tangents() {
        warn!("small buildings: furniture tangents: {e}");
    }
    Some(m)
}

/// Makes damaged lots draw their state: their own LOD0 meshes (once), their words in the state buffer, their shells
/// hidden for good.
#[allow(clippy::too_many_arguments)]
fn redraw_damaged(
    mut commands: Commands,
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    kits: Res<HouseKits>,
    mut houses: ResMut<Houses>,
    mut states: ResMut<BuildingStates>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut vis: Query<&mut Visibility>,
) {
    if live.dirty_houses.is_empty() {
        return;
    }
    let mut dirty = std::mem::take(&mut live.dirty_houses);
    dirty.sort_unstable();
    dirty.dedup();
    for lot in dirty {
        let Some(alive) = live.structures.houses.alive(lot) else { continue };
        let kit = plan.small.kit(lot);
        let key = state_key(lot);
        let base = states.base(key, kit.pieces.len(), &mut buffers);
        let words: Vec<u32> = alive.iter().map(|&a| u32::from(a)).collect();
        states.write(key, &words);
        for &e in houses.far.get(lot as usize).into_iter().flatten() {
            if let Ok(mut v) = vis.get_mut(e) {
                *v = Visibility::Hidden;
            }
        }
        let root = houses.roots[lot as usize];
        let entry = houses.dressed.entry(lot).or_insert_with(|| Dressed {
            lod0: Vec::new(),
            own: false,
            leaves: Vec::new(),
            furniture: None,
            version: 0,
        });
        if !entry.own {
            for e in entry.lod0.drain(..) {
                commands.entity(e).despawn();
            }
            let draw = &kits.kits[plan.small.lots[lot as usize].kit as usize];
            for (surface, mesh) in own_lod0(kit, draw, &meshes, base, &kits.textured) {
                let Some(m) = kits.state.get(&surface) else { continue };
                let h = meshes.add(mesh);
                let mut e = commands.spawn((Mesh3d(h), MeshMaterial3d(m.clone()), Transform::IDENTITY, ChildOf(root)));
                if NO_SHADOW.contains(&surface.as_str()) {
                    e.insert(NotShadowCaster);
                }
                entry.lod0.push(e.id());
            }
            entry.own = true;
        }
        // Leaves of broken pieces go.
        for (i, e) in &entry.leaves {
            let piece = kit.leaves[*i].piece as usize;
            if !alive[piece]
                && let Ok(mut v) = vis.get_mut(*e)
            {
                *v = Visibility::Hidden;
            }
        }
        entry.version = live.structures.houses.version(lot);
    }
}

/// Spawns LOD0 and the leaves for lots near the camera, frees them past `UNDRESS_M`.
#[allow(clippy::too_many_arguments)]
fn dress(
    mut commands: Commands,
    plan: Res<Plan>,
    live: Res<Live>,
    kits: Res<HouseKits>,
    mut houses: ResMut<Houses>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
) {
    let Ok(eye) = camera.single() else { return };
    let here = Vec2::new(eye.translation().x, eye.translation().z);
    let lots = &plan.small;
    // Free the far ones (a damaged one stays).
    let far: Vec<u32> = houses
        .dressed
        .iter()
        .filter(|(l, d)| !d.own && lots.lots[**l as usize].rect.distance(here) > UNDRESS_M)
        .map(|(l, _)| *l)
        .collect();
    for l in far {
        if let Some(d) = houses.dressed.remove(&l) {
            for e in d
                .lod0
                .into_iter()
                .chain(d.leaves.into_iter().map(|(_, e)| e))
                .chain(d.furniture.map(|(_, e)| e))
            {
                commands.entity(e).despawn();
            }
        }
    }
    let mut near: Vec<(u32, f32)> = lots
        .candidates(here, DRESS_M)
        .into_iter()
        .filter(|l| !houses.dressed.contains_key(l))
        .map(|l| (l, lots.lots[l as usize].rect.distance(here)))
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    for (lot, _) in near.into_iter().take(DRESS_PER_FRAME) {
        let l = &lots.lots[lot as usize];
        let kit = lots.kit(lot);
        let draw = &kits.kits[l.kit as usize];
        let root = houses.roots[lot as usize];
        let mut d = Dressed {
            lod0: Vec::new(),
            own: false,
            leaves: Vec::new(),
            furniture: None,
            version: live.structures.houses.version(lot),
        };
        for p in &draw.lod0 {
            let Some(m) = kits.plain.get(&p.surface) else { continue };
            let mut e = commands.spawn((
                Mesh3d(p.mesh.clone()),
                MeshMaterial3d(m.clone()),
                Transform::IDENTITY,
                range(0.0, LOD0_M),
                ChildOf(root),
            ));
            if NO_SHADOW.contains(&p.surface.as_str()) {
                e.insert(NotShadowCaster);
            }
            d.lod0.push(e.id());
        }
        let alive = live.structures.houses.alive(lot);
        for (i, leaf) in kit.leaves.iter().enumerate() {
            let Some(prims) = draw.leaves.get(i) else { continue };
            let shown = alive.is_none_or(|a| a[leaf.piece as usize]);
            let e = commands
                .spawn((
                    Transform::from_translation(leaf.hinge),
                    if shown { Visibility::Inherited } else { Visibility::Hidden },
                    range(0.0, LOD0_M),
                    ChildOf(root),
                ))
                .with_children(|c| {
                    for p in prims {
                        if let Some(m) = kits.plain.get(&p.surface) {
                            c.spawn((
                                Mesh3d(p.mesh.clone()),
                                MeshMaterial3d(m.clone()),
                                Transform::IDENTITY,
                                range(0.0, LOD0_M),
                            ));
                        }
                    }
                })
                .id();
            d.leaves.push((i, e));
        }
        houses.dressed.insert(lot, d);
    }
}

/// Gives each dressed lot the furniture it should draw now: modelled within `REAL_FURNITURE_M` of a pilot on foot,
/// blocks within `FURNITURE_M` otherwise, none beyond.
#[allow(clippy::too_many_arguments)]
fn furnish(
    mut commands: Commands,
    plan: Res<Plan>,
    kits: Res<HouseKits>,
    mut houses: ResMut<Houses>,
    pilot: Res<PilotSim>,
    mut states: ResMut<BuildingStates>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
) {
    let Ok(eye) = camera.single() else { return };
    let Some(material) = kits.furniture_material.clone() else { return };
    let here = Vec2::new(eye.translation().x, eye.translation().z);
    let on_foot = pilot.curr.mode == PilotMode::OnFoot;
    let lots = &plan.small;
    let mut built = 0;
    let ids: Vec<u32> = houses.dressed.keys().copied().collect();
    let mut wants: Vec<(u32, f32, Option<Furnished>)> = ids
        .into_iter()
        .map(|l| {
            let d = lots.lots[l as usize].rect.distance(here);
            let want = if d > FURNITURE_M {
                None
            } else if on_foot && d <= REAL_FURNITURE_M {
                Some(Furnished::Real)
            } else {
                Some(Furnished::Blocks)
            };
            (l, d, want)
        })
        .collect();
    wants.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    for (lot, _, want) in wants {
        let have = houses.dressed[&lot].furniture.map(|(f, _)| f);
        if have == want {
            continue;
        }
        if want.is_some() && built >= FURNISH_PER_FRAME {
            continue;
        }
        let root = houses.roots[lot as usize];
        let d = houses.dressed.get_mut(&lot).expect("dressed");
        if let Some((_, e)) = d.furniture.take() {
            commands.entity(e).despawn();
        }
        let Some(kind) = want else { continue };
        let kit = lots.kit(lot);
        let base = states.base(state_key(lot), kit.pieces.len(), &mut buffers);
        let draw = &kits.kits[lots.lots[lot as usize].kit as usize];
        let Some(mesh) = furniture_mesh(kit, &lots.kits, draw, &kits.furniture, &meshes, base, kind == Furnished::Real) else {
            continue;
        };
        let e = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                NotShadowCaster,
                ChildOf(root),
            ))
            .id();
        d.furniture = Some((kind, e));
        built += 1;
    }
}

/// Turns each drawn leaf to its door's angle: the pilot's colliders hold the doors while on foot beside it (one
/// for each standing leaf, in the kit's order: `walk::walkable`); otherwise every door is shut.
fn swing_leaves(
    plan: Res<Plan>,
    live: Res<Live>,
    houses: Res<Houses>,
    colliders: Res<InteriorColliders>,
    fixed: Res<Time<Fixed>>,
    mut leaves: Query<&mut Transform>,
) {
    let alpha = fixed.overstep_fraction();
    for (lot, d) in &houses.dressed {
        let kit = plan.small.kit(*lot);
        let doors = colliders.0.houses.get(lot).map(|(w, _)| &w.doors);
        let alive = live.structures.houses.alive(*lot);
        let stands = |i: usize| alive.is_none_or(|a| a[kit.leaves[i].piece as usize]);
        for (i, e) in &d.leaves {
            let Ok(mut t) = leaves.get_mut(*e) else { continue };
            let leaf = &kit.leaves[*i];
            let door = (0..*i).filter(|&k| stands(k)).count();
            let angle = doors
                .filter(|_| stands(*i))
                .and_then(|ds| ds.get(door))
                .map_or(0.0, |s| s.prev_angle_deg + (s.angle_deg - s.prev_angle_deg) * alpha);
            let (sn, cs) = angle.to_radians().sin_cos();
            let dir = leaf.shut * cs + leaf.inward * sn;
            let want = Quat::from_rotation_arc(
                Vec3::new(leaf.shut.x, 0.0, leaf.shut.y),
                Vec3::new(dir.x, 0.0, dir.y).normalize_or(Vec3::X),
            );
            if t.rotation != want {
                t.rotation = want;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_primitives_surface_is_read_from_its_extras() {
        assert_eq!(
            serde_json_surface(r#"{"surface":"brick","triangles":12}"#).as_deref(),
            Some("brick")
        );
        assert_eq!(
            serde_json_surface(r#"{"surface": "sign:EAT|red|white|1"}"#).as_deref(),
            Some("sign:EAT|red|white|1")
        );
        assert_eq!(serde_json_surface("{}"), None);
        let x = r#"{"surface":"siding","generator":"siding","tile_m":1.0,"standin":true}"#;
        assert_eq!(extras_text(x, "generator").as_deref(), Some("siding"));
        assert_eq!(extras_number(x, "tile_m"), Some(1.0));
        assert_eq!(extras_number(r#"{"tile_m": 2.5}"#, "tile_m"), Some(2.5));
    }
}
