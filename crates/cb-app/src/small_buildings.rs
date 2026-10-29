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
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use cb_core::pilot::PilotMode;
use cb_core::world::small_buildings::kit::Node;
use cb_core::world::small_buildings::{HOUSE_CHUNK_BASE, Kits, Lot};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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
    /// Each placed site's ground (airport-layout, seaport-layout).
    grounds: Vec<Handle<Gltf>>,
}

/// One primitive of a kit's node: its mesh and its surface, and for LOD0 and FURN its vertices copied once while the
/// loading screen is up, so a damaged shell or a building's furniture is meshed on the async pool (`Jobs`), never in a
/// frame (CLAUDE.md section 7: no frame over 16.7 ms; a terminal's furniture is 350 000 triangles).
#[derive(Clone)]
struct Prim {
    mesh: Handle<Mesh>,
    surface: String,
    src: Option<Arc<Owned>>,
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
    /// Each modelled item's vertices (a copy, for merging off the main thread).
    furniture: Arc<HashMap<String, Arc<Owned>>>,
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
    /// Each placed site's ground: its primitives and their materials (the page's stand-in looks).
    grounds: Vec<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>>,
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

/// Every lot's root, the dressed ones, and the meshes being built for them.
#[derive(Resource, Default)]
struct Houses {
    roots: Vec<Entity>,
    far: Vec<Vec<Entity>>,
    dressed: BTreeMap<u32, Dressed>,
    jobs: Jobs,
}

/// Meshes being built on the async pool: a damaged lot's own LOD0 (by surface), a dressed lot's furniture.
#[derive(Default)]
struct Jobs {
    lod0: BTreeMap<u32, Task<Vec<(String, Mesh)>>>,
    furniture: BTreeMap<u32, (Furnished, Task<Option<Mesh>>)>,
}

/// Draws the houses and small businesses.
pub struct SmallBuildingsPlugin;

impl Plugin for SmallBuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HouseKits>()
            .init_resource::<Houses>()
            .add_systems(Startup, start_loading)
            .add_systems(Update, bind.run_if(|k: Res<HouseKits>| !k.ready))
            .add_game_systems(FixedUpdate, sink_ships.in_set(crate::sim::SimSet::World))
            .add_game_systems(
                Update,
                (
                    spawn_lots,
                    follow_state,
                    redraw_damaged,
                    dress,
                    furnish,
                    finish_jobs,
                    swing_leaves,
                    pose_sinking,
                )
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
            .map(|k| server.load(format!("models/{}/{}.glb", k.folder, k.id)))
            .collect(),
        grounds: plan
            .sites
            .iter()
            .map(|s| server.load(format!("models/{}", s.template.ground.glb)))
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
    plan: Res<Plan>,
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
    let all = loading
        .kits
        .iter()
        .chain(loading.furniture.iter().map(|(_, h)| h))
        .chain(loading.grounds.iter());
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
                        src: None,
                    }
                })
                .collect();
            let copied = |prims: Vec<Prim>| {
                prims
                    .into_iter()
                    .map(|mut p| {
                        p.src = meshes.get(&p.mesh).and_then(owned).map(Arc::new);
                        p
                    })
                    .collect::<Vec<_>>()
            };
            match name.as_ref() {
                "LOD0" => d.lod0 = copied(prims),
                "FURN" => d.furn = copied(prims),
                "LOD1" => d.lod1 = prims,
                "LOD2" => d.lod2 = prims,
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
    let mut furniture = HashMap::new();
    for (name, h) in &loading.furniture {
        let Some(g) = gltfs.get(h) else { return };
        if let Some(src) = g
            .meshes
            .first()
            .and_then(|m| gltf_meshes.get(m))
            .and_then(|m| m.primitives.first())
            .and_then(|p| meshes.get(&p.mesh))
            .and_then(owned)
        {
            furniture.insert(name.clone(), Arc::new(src));
        }
    }
    kits.furniture = Arc::new(furniture);
    // Each site's ground: its one mesh's primitives in the look its table gives each surface.
    let mut grounds = Vec::with_capacity(loading.grounds.len());
    for (site, h) in plan.sites.iter().zip(&loading.grounds) {
        let Some(g) = gltfs.get(h) else { return };
        let Some(mesh) = g.meshes.first().and_then(|m| gltf_meshes.get(m)) else {
            grounds.push(Vec::new());
            continue;
        };
        let mut prims = Vec::with_capacity(mesh.primitives.len());
        for p in &mesh.primitives {
            let surface = surface_of(p);
            let Some(look) = site.template.ground.materials.get(&surface) else {
                continue;
            };
            let [r, g, b] = look.colour;
            let [er, eg, eb] = look.emissive;
            let m = plain.add(StandardMaterial {
                base_color: Color::linear_rgb(r, g, b),
                perceptual_roughness: look.roughness,
                metallic: look.metallic,
                emissive: LinearRgba::rgb(er, eg, eb) * 4.0,
                ..default()
            });
            prims.push((p.mesh.clone(), m));
        }
        grounds.push(prims);
    }
    kits.grounds = grounds;
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

/// Takes the sinking ships down a fixed step (`small_buildings::damage::Sinking`, seaport-layout section 2): a ship that
/// is gone under the seabed is redrawn with every piece down.
fn sink_ships(plan: Res<Plan>, mut live: ResMut<Live>, time: Res<Time>) {
    if live.structures.houses.states.values().all(|s| s.sinking.is_none()) {
        return;
    }
    let live = &mut *live;
    let dug = &live.dug;
    let seabed = |p: Vec2| plan.terrain.height_at(p.x, p.y) + dug.delta_at(p);
    let (_, gone) = live.structures.houses.step_sinking(&plan.small, time.delta_secs(), &seabed);
    live.dirty_houses.extend(gone);
}

/// Poses each sinking ship's root where it has gone down to.
fn pose_sinking(plan: Res<Plan>, live: Res<Live>, houses: Res<Houses>, mut roots: Query<&mut Transform>) {
    for (&lot, st) in &live.structures.houses.states {
        if st.sinking.is_none() {
            continue;
        }
        let (Some(&root), Some(m)) = (
            houses.roots.get(lot as usize),
            live.structures.houses.sink_transform(&plan.small, lot),
        ) else {
            continue;
        };
        if let Ok(mut t) = roots.get_mut(root) {
            let base = lot_transform(&plan.small.lots[lot as usize]).to_matrix();
            *t = Transform::from_matrix(base * Mat4::from(m));
        }
    }
}

fn lot_transform(l: &Lot) -> Transform {
    Transform::from_translation(Vec3::new(l.centre.x, l.ground_m, l.centre.y)).with_rotation(Quat::from_rotation_y(l.heading_rad))
}

/// How far a site's kit reaches from its origin (x, z), so its LOD distances, measured from the origin, are the
/// houses' measured from its edge; a house's is none.
fn reach_of(kit: &cb_core::world::small_buildings::Kit) -> f32 {
    if !kit.kind.is_site() {
        return 0.0;
    }
    [
        kit.lo,
        kit.hi,
        Vec3::new(kit.lo.x, 0.0, kit.hi.z),
        Vec3::new(kit.hi.x, 0.0, kit.lo.z),
    ]
    .iter()
    .map(|c| Vec2::new(c.x, c.z).length())
    .fold(0.0, f32::max)
}

/// A visibility range from `start` to `end` past `reach` (`reach_of`).
fn range_past(start: f32, end: f32, reach: f32) -> VisibilityRange {
    if reach <= 0.0 {
        return range(start, end);
    }
    let r = range(start, end);
    VisibilityRange {
        start_margin: if start > 0.0 {
            (r.start_margin.start + reach)..(r.start_margin.end + reach)
        } else {
            0.0..0.0
        },
        end_margin: (r.end_margin.start + reach)..(r.end_margin.end + reach),
        use_aabb: false,
    }
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
    // Each site's ground, in its frame (airport-layout, seaport-layout).
    for (site, prims) in plan.sites.iter().zip(&kits.grounds) {
        let at = Transform::from_translation(Vec3::new(site.origin.x, site.ground_m, site.origin.y))
            .with_rotation(Quat::from_rotation_y(site.heading_rad()));
        commands.spawn((at, Visibility::Visible)).with_children(|c| {
            for (mesh, m) in prims {
                c.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(m.clone()), Transform::IDENTITY));
            }
        });
    }
    for l in &plan.small.lots {
        let draw = &kits.kits[l.kit as usize];
        let reach = reach_of(&plan.small.kits.kits[l.kit as usize]);
        let mut far = Vec::new();
        let root = commands
            .spawn((lot_transform(l), Visibility::Visible))
            .with_children(|c| {
                for (prims, r) in [
                    (&draw.lod1, range_past(LOD0_M, LOD1_M, reach)),
                    (&draw.lod2, range_past(LOD1_M, LOD2_M, reach)),
                ] {
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

/// A source mesh's attributes and triangle list, copied out of the asset so a pool task can read it.
struct Owned {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
    tri: Vec<u32>,
}

fn owned(m: &Mesh) -> Option<Owned> {
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
    Some(Owned {
        pos: pos.clone(),
        nrm: nrm.clone(),
        uv: uv.clone(),
        col,
        tri,
    })
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
fn own_lod0(kit: &cb_core::world::small_buildings::Kit, lod0: &[Prim], base: u32, textured: &[String]) -> Vec<(String, Mesh)> {
    let mut out = Vec::new();
    for (pi, p) in lod0.iter().enumerate() {
        let Some(src) = p.src.as_deref() else {
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
    furn: &[Prim],
    models: &HashMap<String, Arc<Owned>>,
    base: u32,
    real: bool,
) -> Option<Mesh> {
    let mut c = Corners::new();
    if real {
        for f in &kit.furniture {
            let Some(src) = models.get(&f.item) else {
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
        for (pi, p) in furn.iter().enumerate() {
            let Some(src) = p.src.as_deref() else {
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
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    kits: Res<HouseKits>,
    mut houses: ResMut<Houses>,
    mut states: ResMut<BuildingStates>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
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
        let houses = &mut *houses;
        let entry = houses.dressed.entry(lot).or_insert_with(|| Dressed {
            lod0: Vec::new(),
            own: false,
            leaves: Vec::new(),
            furniture: None,
            version: 0,
        });
        // Its own LOD0 (slots per piece) is meshed on the pool; it keeps what it draws until it is in (`finish_jobs`).
        if !entry.own && !houses.jobs.lod0.contains_key(&lot) {
            let all = plan.small.kits.clone();
            let k = plan.small.lots[lot as usize].kit as usize;
            let lod0 = kits.kits[k].lod0.clone();
            let textured = kits.textured.clone();
            let task = AsyncComputeTaskPool::get().spawn(async move { own_lod0(&all.kits[k], &lod0, base, &textured) });
            houses.jobs.lod0.insert(lot, task);
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
        .filter(|(l, d)| !d.own && !houses.jobs.lod0.contains_key(l) && lots.lots[**l as usize].rect.distance(here) > UNDRESS_M)
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
        let reach = reach_of(kit);
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
                range_past(0.0, LOD0_M, reach),
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
                    range_past(0.0, LOD0_M, reach),
                    ChildOf(root),
                ))
                .with_children(|c| {
                    for p in prims {
                        if let Some(m) = kits.plain.get(&p.surface) {
                            c.spawn((
                                Mesh3d(p.mesh.clone()),
                                MeshMaterial3d(m.clone()),
                                Transform::IDENTITY,
                                range_past(0.0, LOD0_M, reach),
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
    camera: Query<&GlobalTransform, With<PilotCamera>>,
) {
    let Ok(eye) = camera.single() else { return };
    if kits.furniture_material.is_none() {
        return;
    }
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
        let pending = houses.jobs.furniture.get(&lot).map(|(f, _)| *f);
        if have == want || (want.is_some() && pending == want) {
            continue;
        }
        if want.is_none() {
            // Out of reach: none drawn, none coming.
            houses.jobs.furniture.remove(&lot);
            if let Some((_, e)) = houses.dressed.get_mut(&lot).and_then(|d| d.furniture.take()) {
                commands.entity(e).despawn();
            }
            continue;
        }
        if built >= FURNISH_PER_FRAME {
            continue;
        }
        let Some(kind) = want else { continue };
        // Meshed on the pool; the furniture it has stays until the new one is in (`finish_jobs`).
        let k = lots.lots[lot as usize].kit as usize;
        let base = states.base(state_key(lot), lots.kits.kits[k].pieces.len(), &mut buffers);
        let (all, furn, models) = (lots.kits.clone(), kits.kits[k].furn.clone(), kits.furniture.clone());
        let task = AsyncComputeTaskPool::get()
            .spawn(async move { furniture_mesh(&all.kits[k], &all, &furn, &models, base, kind == Furnished::Real) });
        houses.jobs.furniture.insert(lot, (kind, task));
        built += 1;
    }
}

/// Puts the meshes built on the pool in place: a damaged lot's own LOD0 for its shells and plain LOD0 (hidden then for
/// good), a lot's furniture for the furniture it had. A lot freed meanwhile drops its furniture.
#[allow(clippy::too_many_arguments)]
fn finish_jobs(
    mut commands: Commands,
    live: Res<Live>,
    kits: Res<HouseKits>,
    mut houses: ResMut<Houses>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut vis: Query<&mut Visibility>,
) {
    let houses = &mut *houses;
    let done: Vec<(u32, Vec<(String, Mesh)>)> = houses
        .jobs
        .lod0
        .iter_mut()
        .filter_map(|(l, t)| check_ready(t).map(|m| (*l, m)))
        .collect();
    for (lot, built) in done {
        houses.jobs.lod0.remove(&lot);
        let root = houses.roots[lot as usize];
        for &e in houses.far.get(lot as usize).into_iter().flatten() {
            if let Ok(mut v) = vis.get_mut(e) {
                *v = Visibility::Hidden;
            }
        }
        let Some(entry) = houses.dressed.get_mut(&lot) else { continue };
        for e in entry.lod0.drain(..) {
            commands.entity(e).despawn();
        }
        for (surface, mesh) in built {
            let Some(m) = kits.state.get(&surface) else { continue };
            let mut e = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(m.clone()),
                Transform::IDENTITY,
                ChildOf(root),
            ));
            if NO_SHADOW.contains(&surface.as_str()) {
                e.insert(NotShadowCaster);
            }
            entry.lod0.push(e.id());
        }
        entry.own = true;
        entry.version = live.structures.houses.version(lot);
    }
    let Some(material) = kits.furniture_material.clone() else { return };
    let done: Vec<(u32, Furnished, Option<Mesh>)> = houses
        .jobs
        .furniture
        .iter_mut()
        .filter_map(|(l, (k, t))| check_ready(t).map(|m| (*l, *k, m)))
        .collect();
    for (lot, kind, mesh) in done {
        houses.jobs.furniture.remove(&lot);
        let root = houses.roots[lot as usize];
        let Some(d) = houses.dressed.get_mut(&lot) else { continue };
        if let Some((_, e)) = d.furniture.take() {
            commands.entity(e).despawn();
        }
        let Some(mesh) = mesh else { continue };
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
