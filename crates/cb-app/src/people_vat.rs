//! People drawn from their vertex animation textures (openspec change `people-vat`, design section 4): every
//! civilian, and every driver running from a car, is a copy in one of a few shared meshes, posed on the GPU
//! from the baked clips (`assets/models/people/`, `tools/blender/build_people.py`). It lives in `cb-app`
//! because it is drawing; which clip a person plays and where in it are `cb_core::civilians::anim`'s.
//!
//! Each (body, level) has one material (`StandardMaterial` extended by `PeopleVat`, so people are lit, fogged
//! and shadowed like the world) and a run of chunk entities sharing one mesh of 32 copies of the body; a
//! chunk's `MeshTag` is its first slot in one storage buffer holding every person's place, clips, frames,
//! blend and colours, written whole each frame from the render world. Chunks of a level batch into one draw,
//! so the draw count is fixed whatever the crowd; nothing per person is spawned, meshed or transformed.

use crate::civilians::Crowd;
use crate::pilot::PilotCamera;
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshTag, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat};
use bevy::render::renderer::RenderQueue;
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use bevy::shader::ShaderRef;
use cb_core::civilians::anim::{Anim, Clip, ClipTable, Outfit};
use cb_core::civilians::{Activity, DrawConfig, Palette, Person};
use cb_core::people_vat::{Manifest, VatLevel};
use cb_core::world::Ground;
use std::collections::HashMap;

const SHADER: &str = "shaders/people_vat.wesl";
const PREPASS_SHADER: &str = "shaders/people_vat_prepass.wesl";

/// People a chunk entity draws (copies in its mesh).
pub const CHUNK: usize = 32;
/// Room in the crowd level for drivers on foot beyond the civilians' `max_people`.
const RUNNER_ROOM: usize = 64;
/// A runner's key, above every civilian's id.
const RUNNER_KEY: u64 = 1 << 32;

/// The people material.
pub type PeopleMaterial = ExtendedMaterial<StandardMaterial, PeopleVat>;

/// `PeopleLook` in `people_vat_common.wesl`, field for field.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct PeopleLook {
    /// The level's bounds, least corner (person space, metres).
    pub bounds_min: Vec4,
    /// The level's bounds, most corner.
    pub bounds_max: Vec4,
    /// Skin 0-7, hair 8-15, tops 16-31, bottoms 32-43, shoes 44-51, linear RGB.
    pub palette: [Vec4; 56],
}

/// The vertex animation of one body at one level.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PeopleVat {
    /// Bounds and colours.
    #[uniform(100)]
    pub look: PeopleLook,
    /// The baked texture: a row a frame, a texel a vertex.
    #[texture(101, sample_type = "u_int")]
    pub vat: Handle<Image>,
    /// Every person (`PersonGpu`).
    #[storage(102, read_only)]
    pub people: Handle<ShaderBuffer>,
}

impl MaterialExtension for PeopleVat {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        PREPASS_SHADER.into()
    }
}

/// `Person` in `people_vat_common.wesl`: 48 bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PersonGpu {
    /// Feet and heading.
    pub place: [f32; 4],
    /// Scale, the playing clip's frame, the fading clip's frame, the playing clip's weight.
    pub frames: [f32; 4],
    /// The playing clip, the fading clip (first row | frames << 16), the palette, the flags.
    pub clips: [u32; 4],
}

const FLAG_SHOWN: u32 = 1;
const FLAG_NOW_LOOPS: u32 = 2;
const FLAG_PREV: u32 = 4;
const FLAG_PREV_LOOPS: u32 = 8;

impl PersonGpu {
    fn write(&self, out: &mut Vec<u8>) {
        for f in self.place.iter().chain(&self.frames) {
            out.extend_from_slice(&f.to_le_bytes());
        }
        for u in &self.clips {
            out.extend_from_slice(&u.to_le_bytes());
        }
    }
}

/// The bake as read from disk at startup: the manifest, each body's clip table and each level's file.
struct Bake {
    manifest: Manifest,
    tables: Vec<ClipTable>,
    levels: Vec<Vec<VatLevel>>,
}

/// Reads and checks the bake (`None`, with a warning, when it is missing or does not check out).
fn read_bake() -> Option<Bake> {
    let dir = crate::cli::repo_root().join("assets/models/people");
    let read = || -> Result<Bake, String> {
        let manifest = Manifest::parse(&std::fs::read_to_string(dir.join("people_vat.json")).map_err(|e| e.to_string())?)?;
        manifest.check()?;
        let mut tables = Vec::new();
        let mut levels = Vec::new();
        for b in &manifest.bodies {
            tables.push(ClipTable::from_manifest(&manifest, &b.id)?);
            let mut ls = Vec::new();
            for l in &b.levels {
                let bytes = std::fs::read(dir.join(&l.file)).map_err(|e| format!("{}: {e}", l.file))?;
                let vat = VatLevel::parse(&bytes).map_err(|e| format!("{}: {e}", l.file))?;
                manifest.check_level(l, &vat)?;
                ls.push(vat);
            }
            levels.push(ls);
        }
        Ok(Bake { manifest, tables, levels })
    };
    match read() {
        Ok(b) => Some(b),
        Err(e) => {
            warn!("people: the bake in {} cannot be drawn: {e}", dir.display());
            None
        }
    }
}

/// The level's draw mesh: `CHUNK` copies of the body, each vertex's number and copy in its UV, its shade
/// and region in its colour, and the idle's first frame as its position (the shader replaces it).
fn chunk_mesh(vat: &VatLevel) -> Mesh {
    let n = vat.vertices as usize;
    let mut pos = Vec::with_capacity(n * CHUNK);
    let mut nor = Vec::with_capacity(n * CHUNK);
    let mut uv = Vec::with_capacity(n * CHUNK);
    let mut col = Vec::with_capacity(n * CHUNK);
    let mut idx = Vec::with_capacity(vat.indices.len() * CHUNK);
    for c in 0..CHUNK {
        for v in 0..n {
            pos.push(vat.position(0, v as u32));
            nor.push([0.0, 1.0, 0.0]);
            uv.push([v as f32, c as f32]);
            col.push([f32::from(vat.shade[v]) / 255.0 * 2.0, f32::from(vat.region[v]), 0.0, 1.0]);
        }
        idx.extend(vat.indices.iter().map(|i| i + (c * n) as u32));
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx))
}

/// The level's texture: `vertices` wide, a row a frame, `Rgba16Uint`.
fn vat_image(vat: &VatLevel) -> Image {
    Image::new(
        Extent3d {
            width: vat.vertices,
            height: vat.rows,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vat.texels.clone(),
        TextureFormat::Rgba16Uint,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// The palette lists as the shader's table, linear RGB.
fn palette_table(p: &Palette) -> [Vec4; 56] {
    let mut t = [Vec4::ONE; 56];
    for (base, list) in [0usize, 8, 16, 32, 44].into_iter().zip(p.lists()) {
        for (i, c) in list.iter().enumerate() {
            let l = Color::srgb(c[0], c[1], c[2]).to_linear();
            t[base + i] = Vec4::new(l.red, l.green, l.blue, 1.0);
        }
    }
    t
}

/// One (body, level)'s run of slots.
#[derive(Clone, Copy, Debug)]
struct Section {
    /// Its first slot.
    base: usize,
    /// Its slots (a multiple of `CHUNK`).
    capacity: usize,
}

/// The chunk entity drawing slots `base..base + CHUNK` of section `section`.
#[derive(Component)]
struct Chunk {
    section: usize,
    index: usize,
}

/// Everything the people renderer keeps between frames.
#[derive(Resource)]
pub struct PeopleDraw {
    tables: Vec<ClipTable>,
    heights: Vec<f32>,
    /// `[body][level]` sections; level 0 near, 1 crowd.
    sections: Vec<[Section; 2]>,
    /// Every slot, written each frame.
    slots: Vec<PersonGpu>,
    /// The buffer the materials read.
    buffer: Handle<ShaderBuffer>,
    /// Each figure's animation, by key (a civilian's id, or a runner's car above `RUNNER_KEY`).
    anims: HashMap<u64, Anim>,
    /// The bytes of this frame's slots, for the render world.
    bytes: Vec<u8>,
}

/// One figure to draw this frame.
#[derive(Clone, Copy, Debug)]
pub struct Figure {
    /// Its key for its animation's state.
    pub key: u64,
    /// Feet.
    pub feet: Vec3,
    /// Heading, radians, 0 facing -z, positive turning left.
    pub heading_rad: f32,
    /// Standing height, metres.
    pub height_m: f32,
    /// What it looks like.
    pub look: u32,
    /// The clip it should play.
    pub clip: Clip,
    /// Its ground speed, m/s, for the cadence.
    pub pace_m_s: f32,
}

/// Draws people from their vertex animation textures.
pub struct PeopleVatPlugin;

impl Plugin for PeopleVatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PeopleMaterial>::default());
        let Some(bake) = read_bake() else { return };
        app.insert_resource(BakeOnce(bake))
            .add_world_spawn(spawn_people)
            .add_game_systems(Update, draw_people.in_set(PresentSet::Pose));
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .add_systems(ExtractSchedule, extract)
                .add_systems(Render, upload.in_set(RenderSystems::PrepareResources));
        }
    }
}

/// The bake read at startup; each world's spawn builds its meshes, textures and materials from it.
#[derive(Resource)]
struct BakeOnce(Bake);

fn spawn_people(
    mut commands: Commands,
    bake: Res<BakeOnce>,
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<PeopleMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let b = &bake.0;
    let cfg = &data.0.people.cfg;
    let near_cap = cfg.draw.near_max.div_ceil(CHUNK) * CHUNK;
    let crowd_cap = (cfg.max_people + RUNNER_ROOM).div_ceil(CHUNK) * CHUNK;
    let mut sections = Vec::new();
    let mut total = 0;
    for _ in &b.manifest.bodies {
        let near = Section {
            base: total,
            capacity: near_cap,
        };
        total += near_cap;
        let crowd = Section {
            base: total,
            capacity: crowd_cap,
        };
        total += crowd_cap;
        sections.push([near, crowd]);
    }
    let buffer = buffers.add(ShaderBuffer::new(vec![[0u32; 4]; total * 3], RenderAssetUsages::RENDER_WORLD));
    let palette = palette_table(&cfg.draw.palette);
    let mut chunks = 0;
    for (bi, body) in b.manifest.bodies.iter().enumerate() {
        for (li, vat) in b.levels[bi].iter().take(2).enumerate() {
            let level = &body.levels[li];
            let mesh = meshes.add(chunk_mesh(vat));
            let material = materials.add(PeopleMaterial {
                base: StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.8,
                    ..default()
                },
                extension: PeopleVat {
                    look: PeopleLook {
                        bounds_min: Vec3::from(level.bounds_min).extend(0.0),
                        bounds_max: Vec3::from(level.bounds_max).extend(0.0),
                        palette,
                    },
                    vat: images.add(vat_image(vat)),
                    people: buffer.clone(),
                },
            });
            let s = sections[bi][li];
            for k in 0..s.capacity / CHUNK {
                let mut e = commands.spawn((
                    Name::new(format!("people {} {} chunk {k}", body.id, level.name)),
                    Chunk {
                        section: bi * 2 + li,
                        index: k,
                    },
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    MeshTag::new((s.base + k * CHUNK) as u32),
                    Transform::IDENTITY,
                    Visibility::Hidden,
                    NoFrustumCulling,
                ));
                if li > 0 {
                    e.insert(NotShadowCaster);
                }
                chunks += 1;
            }
        }
    }
    info!(
        "people: {} bodies, {} slots in {chunks} chunk entities, levels {:?}",
        b.manifest.bodies.len(),
        total,
        b.manifest
            .bodies
            .iter()
            .flat_map(|x| x.levels.iter().map(move |l| format!("{} {} {} tris", x.id, l.name, l.triangles)))
            .collect::<Vec<_>>()
    );
    commands.insert_resource(PeopleDraw {
        tables: b.tables.clone(),
        heights: b.manifest.bodies.iter().map(|x| x.height_m).collect(),
        sections,
        slots: vec![PersonGpu::default(); total],
        buffer,
        anims: HashMap::new(),
        bytes: Vec::with_capacity(total * 48),
    });
}

/// The figure for a civilian.
pub fn civilian(p: &Person, cfg: &DrawConfig, greet_hold_s: f32) -> Figure {
    Figure {
        key: u64::from(p.id),
        feet: p.feet(),
        heading_rad: p.heading_rad,
        height_m: p.height_m,
        look: p.look,
        clip: cb_core::civilians::anim::choose(p, greet_hold_s, cfg.wave_s),
        pace_m_s: if matches!(p.activity, Activity::Down { .. }) {
            0.0
        } else {
            p.pace_m_s
        },
    }
}

/// Every figure this frame: the civilians and the drivers running from their cars.
fn figures(crowd: Option<&Crowd>, traffic: Option<&crate::traffic::TrafficSim>, plan: &crate::world::Plan, data: &Data) -> Vec<Figure> {
    let cfg = &data.0.people.cfg;
    let mut out: Vec<Figure> = crowd
        .map(|c| c.people.list.iter().map(|p| civilian(p, &cfg.draw, cfg.greet_hold_s)).collect())
        .unwrap_or_default();
    if let Some(t) = traffic {
        for r in &t.traffic.runners {
            let dir = (r.to - r.at).normalize_or(Vec2::NEG_Y);
            out.push(Figure {
                key: RUNNER_KEY + u64::from(r.car),
                feet: Vec3::new(r.at.x, plan.height_at(r.at.x, r.at.y), r.at.y),
                heading_rad: (-dir.x).atan2(-dir.y),
                height_m: 1.78,
                look: r.car.wrapping_mul(2_654_435_761),
                clip: Clip::Run,
                pace_m_s: f32::NAN,
            });
        }
    }
    out
}

/// Steps every figure's animation, packs them into their sections nearest first, shows the chunks in use.
fn draw_people(
    draw: Option<ResMut<PeopleDraw>>,
    crowd: Option<Res<Crowd>>,
    traffic: Option<Res<crate::traffic::TrafficSim>>,
    plan: Res<crate::world::Plan>,
    data: Res<Data>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut chunks: Query<(&Chunk, &mut Visibility)>,
) {
    let _t = crate::perf_hud::slow("draw_people");
    let Some(mut draw) = draw else { return };
    let draw = &mut *draw;
    let cfg = &data.0.people.cfg;
    let dc = &cfg.draw;
    let eye = camera.translation();
    let dt = time.delta_secs();
    let figs = figures(crowd.as_deref(), traffic.as_deref(), &plan, &data);
    let lists = cfg.draw.palette.lists().map(<[[f32; 3]]>::len);
    let bodies = draw.tables.len();
    let mut used = vec![[0usize; 2]; bodies];
    let mut near_left = dc.near_max;
    let near2 = dc.near_m * dc.near_m;
    let mut seen = Vec::with_capacity(figs.len());
    for f in &figs {
        let outfit = Outfit::of(f.look, bodies, lists);
        let table = &draw.tables[outfit.body];
        let pace = if f.pace_m_s.is_finite() {
            f.pace_m_s
        } else {
            table.get(f.clip).speed_m_s
        };
        let rate = table.rate(f.clip, pace, dc.cadence);
        let anim = draw.anims.entry(f.key).or_insert_with(|| Anim::start(f.clip, f.key as u32, table));
        anim.step(f.clip, rate, dt, dc.crossfade_s, table);
        seen.push(f.key);
        let level = if near_left > 0 && f.feet.distance_squared(eye) <= near2 {
            near_left -= 1;
            0
        } else {
            1
        };
        let s = draw.sections[outfit.body][level];
        let k = used[outfit.body][level];
        if k >= s.capacity {
            continue;
        }
        used[outfit.body][level] += 1;
        let clip_word = |c: Clip| {
            let t = table.get(c);
            t.row | t.frames << 16
        };
        let now = anim.now;
        let mut flags = FLAG_SHOWN;
        if table.get(now.clip).looped {
            flags |= FLAG_NOW_LOOPS;
        }
        let (prev_word, prev_frame) = match anim.prev {
            Some(p) => {
                flags |= FLAG_PREV;
                if table.get(p.clip).looped {
                    flags |= FLAG_PREV_LOOPS;
                }
                (clip_word(p.clip), p.frame(table))
            }
            None => (0, 0.0),
        };
        draw.slots[s.base + k] = PersonGpu {
            place: [f.feet.x, f.feet.y, f.feet.z, f.heading_rad],
            frames: [
                f.height_m / draw.heights[outfit.body],
                now.frame(table),
                prev_frame,
                anim.weight(dc.crossfade_s),
            ],
            clips: [clip_word(now.clip), prev_word, outfit.packed(), flags],
        };
    }
    if draw.anims.len() > seen.len() {
        seen.sort_unstable();
        draw.anims.retain(|k, _| seen.binary_search(k).is_ok());
    }
    // clear the rest of every chunk in use, and show exactly the chunks in use
    for (bi, secs) in draw.sections.clone().iter().enumerate() {
        for (li, s) in secs.iter().enumerate() {
            let n = used[bi][li];
            let end = n.div_ceil(CHUNK) * CHUNK;
            for slot in &mut draw.slots[s.base + n..s.base + end.min(s.capacity)] {
                slot.clips[3] = 0;
            }
        }
    }
    for (c, mut vis) in &mut chunks {
        let (bi, li) = (c.section / 2, c.section % 2);
        let want = if c.index * CHUNK < used[bi][li] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    draw.bytes.clear();
    let PeopleDraw { slots, bytes, .. } = draw;
    for s in slots.iter() {
        s.write(bytes);
    }
}

/// This frame's slots for the render world.
#[derive(Resource)]
struct PeopleUpload {
    handle: Handle<ShaderBuffer>,
    bytes: Vec<u8>,
}

fn extract(mut commands: Commands, draw: Extract<Option<Res<PeopleDraw>>>) {
    if let Some(d) = draw.as_deref() {
        commands.insert_resource(PeopleUpload {
            handle: d.buffer.clone(),
            bytes: d.bytes.clone(),
        });
    }
}

/// Writes every slot into the GPU buffer in place.
fn upload(u: Option<Res<PeopleUpload>>, gpu: Res<RenderAssets<GpuShaderBuffer>>, queue: Res<RenderQueue>) {
    let Some(u) = u else { return };
    let Some(buffer) = gpu.get(&u.handle) else { return };
    if !u.bytes.is_empty() {
        queue.write_buffer(&buffer.buffer, 0, &u.bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_check::{compile, compile_with, struct_size, with_bevy};

    #[test]
    fn the_people_shaders_compile_against_bevy_pbr_and_their_structs_match() {
        let Some(resolver) = with_bevy() else {
            eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
            return;
        };
        let base = [
            ("MESH_PIPELINE", true),
            ("VERTEX_OUTPUT_INSTANCE_INDEX", true),
            ("VERTEX_POSITIONS", true),
            ("VERTEX_NORMALS", true),
            ("VERTEX_UVS_A", true),
            ("VERTEX_COLORS", true),
        ];
        compile(&resolver, "people_vat", &base);
        // the shared module on its own, every declaration kept, for its structs' sizes
        let m = compile_with(&resolver, "people_vat_common", &base, false);
        assert_eq!(struct_size(&m, "Person"), 48);
        assert_eq!(struct_size(&m, "PeopleLook") as u64, u64::from(PeopleLook::min_size()));
        for (normals, clamp, motion) in [(false, true, false), (true, false, true)] {
            let mut flags = base.to_vec();
            flags.extend([
                ("NORMAL_PREPASS_OR_DEFERRED_PREPASS", normals),
                ("UNCLIPPED_DEPTH_ORTHO_EMULATION", clamp),
                ("MOTION_VECTOR_PREPASS", motion),
            ]);
            compile(&resolver, "people_vat_prepass", &flags);
        }
    }

    #[test]
    fn a_person_packs_into_the_shaders_48_bytes() {
        let mut out = Vec::new();
        PersonGpu::default().write(&mut out);
        assert_eq!(out.len(), 48);
    }

    #[test]
    fn the_chunk_mesh_numbers_every_copy_and_vertex() {
        let dir = crate::cli::repo_root().join("assets/models/people");
        let m = Manifest::parse(&std::fs::read_to_string(dir.join("people_vat.json")).unwrap()).unwrap();
        let l = &m.bodies[0].levels[1];
        let vat = VatLevel::parse(&std::fs::read(dir.join(&l.file)).unwrap()).unwrap();
        let mesh = chunk_mesh(&vat);
        assert_eq!(mesh.count_vertices(), vat.vertices as usize * CHUNK);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
            panic!("uv")
        };
        let last = uv.last().unwrap();
        assert_eq!(*last, [(vat.vertices - 1) as f32, (CHUNK - 1) as f32]);
        assert_eq!(mesh.indices().unwrap().len(), vat.indices.len() * CHUNK);
    }
}
