//! The building element state on the GPU (openspec change `building-gpu-state`): building meshes
//! are drawn with `BuildingMaterial`, `StandardMaterial` extended by `ElementState`, one storage
//! buffer holding every damaged building's elements, standing or not (`BuildingStates`, a range per
//! building). Every vertex carries its element's slot in its colour's alpha (`slot`); the vertex
//! shaders (`assets/shaders/building_state*.wesl`) drop the triangles of fallen elements in every
//! pass. A hit writes its building's range straight into the GPU buffer from the render world,
//! never a mesh, never a material: the materials are one per look, shared by every building, so
//! their draws batch. It lives in `cb-app` because it is drawing; which elements stand is
//! `cb-core`'s `Frame`.

use crate::state::GameApp;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::render::renderer::RenderQueue;
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use bevy::shader::ShaderDefVal;
use bevy::shader::ShaderRef;
use cb_core::world::structure::Frame;
use std::collections::HashMap;

const SHADER: &str = "shaders/building_state.wesl";
const PREPASS_SHADER: &str = "shaders/building_state_prepass.wesl";
const SURFACE_SHADER: &str = "shaders/building_surface.wesl";
const ARRAY_SHADER: &str = "shaders/building_array.wesl";

/// Words the shared buffer starts with (256 KB; a 60-storey tower is about 6,000), doubled when
/// full.
const FIRST_CAPACITY: usize = 1 << 16;

/// A building material with its own look (glass): the look, and the element state.
pub type BuildingMaterial = ExtendedMaterial<StandardMaterial, ElementState>;

/// The opaque building material (`building-texture-array`): every opaque look a layer of the
/// arrays, and the element state.
pub type ArrayMaterial = ExtendedMaterial<StandardMaterial, ArrayState>;

/// The arrays and the element state.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ArrayState {
    /// The words (`BuildingStates`).
    #[storage(100, read_only)]
    pub standing: Handle<ShaderBuffer>,
    /// Colour layers.
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    pub albedo: Handle<Image>,
    /// Normal map layers.
    #[texture(103, dimension = "2d_array")]
    pub normal: Handle<Image>,
    /// Occlusion-roughness-metal layers.
    #[texture(104, dimension = "2d_array")]
    pub orm: Handle<Image>,
    /// The windows at night on far and massing walls (`building_windows.wesl`); zero on the near
    /// walls, whose windows are glass.
    #[uniform(105)]
    pub windows: Vec4,
}

impl MaterialExtension for ArrayState {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        PREPASS_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        ARRAY_SHADER.into()
    }
}

/// Which elements stand: one `u32` per element of every damaged building, 1 standing, 0 not.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ElementState {
    /// The words (`BuildingStates`).
    #[storage(100, read_only)]
    pub standing: Handle<ShaderBuffer>,
    /// 1 for glass: it takes no screen-space ambient occlusion (`building_surface.wesl`).
    #[uniform(105)]
    pub no_ssao: u32,
    /// The windows at night (`building_surface.wesl`, `cb_core::day_night::WindowLights`): x the share
    /// lit now, y a lit window's brightness cd/m2, z a storey's height m, w a window cell's width m.
    #[uniform(106)]
    pub windows: Vec4,
}

impl MaterialExtension for ElementState {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        PREPASS_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SURFACE_SHADER.into()
    }

    // Never load the surface normal from the normal prepass (SSAO's): Bevy does for every material
    // when the view has one, and glass is not in that prepass, so a window read the normal of the
    // floor or room wall behind it and reflected the sky along it (the windows looked see-through,
    // with white wedges, 2026-10-24 stills). Bevy drops it for its own transmissive materials only.
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.retain(|d| d != &ShaderDefVal::from("LOAD_PREPASS_NORMALS"));
        }
        // The glass's `depth_bias` orders it among transparent things (`GLASS_SORT_BIAS`);
        // `StandardMaterial` also makes it a GPU depth bias, which would push windows through walls.
        // Glass writes depth (opaque surfaces already do). Drawn first among transparent things, it
        // was covered by any smoke drawn after it, including smoke behind the window or inside the
        // hollow dressed building, which then showed on the facade as if in front of it and ended
        // sharply at the building's edge (owner, 2026-10-25: "critical, massive smoke
        // sorting/blending issue"). With depth written, smoke in front of a window still covers it
        // and smoke behind it is hidden, as by glass that lets 12 % through.
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.bias.constant = 0;
            depth.depth_write_enabled = Some(true);
        }
        Ok(())
    }
}

/// The slot a vertex carries for element `id` of a building whose range starts at `base` (0 is
/// "always shown").
pub fn slot(base: u32, id: u32) -> f32 {
    (base + id + 1) as f32
}

/// The standing bit of an element's word (`building_slot.wesl`): set while it stands.
pub const STANDING: u32 = 1;
/// The dark bit of an element's word: set on every word of a building whose windows have gone out.
pub const DARK: u32 = 2;

/// The words of a frame, as the shaders read them: each element's standing bit, and the dark bit on every word once
/// the building's windows are out (`Frame::lit`).
pub fn standing_words(f: &Frame, lights_out_share: f32) -> Vec<u32> {
    let dark = if f.lit(lights_out_share) { 0 } else { DARK };
    f.elements.iter().map(|e| if e.alive { STANDING } else { 0 } | dark).collect()
}

/// Every damaged building's element words in one buffer, a range each, with the writes of this
/// frame for the render world.
#[derive(Resource)]
pub struct BuildingStates {
    /// The CPU copy, its length the buffer's capacity.
    words: Vec<u32>,
    /// Words handed out.
    used: usize,
    /// The buffer the materials read.
    pub handle: Handle<ShaderBuffer>,
    /// Each building's range start.
    bases: HashMap<u32, u32>,
    /// This frame's writes: (first word, words).
    writes: Vec<(u32, Vec<u32>)>,
}

impl BuildingStates {
    fn new(buffers: &mut Assets<ShaderBuffer>) -> Self {
        let words = vec![1u32; FIRST_CAPACITY];
        let handle = buffers.add(ShaderBuffer::new(words.clone(), RenderAssetUsages::RENDER_WORLD));
        Self {
            words,
            used: 0,
            handle,
            bases: HashMap::new(),
            writes: Vec::new(),
        }
    }

    /// Building `b`'s range start, handing out `len` words the first time. When the buffer is full
    /// it is doubled into a new buffer (the handle changes; the caller points the materials at it).
    pub fn base(&mut self, b: u32, len: usize, buffers: &mut Assets<ShaderBuffer>) -> u32 {
        if let Some(&base) = self.bases.get(&b) {
            return base;
        }
        if self.used + len > self.words.len() {
            let mut cap = self.words.len();
            while self.used + len > cap {
                cap *= 2;
            }
            self.words.resize(cap, 1);
            self.handle = buffers.add(ShaderBuffer::new(self.words.clone(), RenderAssetUsages::RENDER_WORLD));
        }
        let base = self.used as u32;
        self.used += len;
        self.bases.insert(b, base);
        base
    }

    /// Writes building `b`'s words (its range must exist).
    pub fn write(&mut self, b: u32, words: &[u32]) {
        let Some(&base) = self.bases.get(&b) else { return };
        let start = base as usize;
        let range = start..start + words.len();
        if self.words[range.clone()] != *words {
            self.words[range].copy_from_slice(words);
            self.writes.push((base, words.to_vec()));
        }
    }
}

/// This frame's writes as the render world receives them.
#[derive(Resource, Clone)]
struct StateUploads {
    handle: Handle<ShaderBuffer>,
    writes: Vec<(u32, Vec<u32>)>,
}

/// Registers the building material and the shared element state.
pub struct BuildingStatePlugin;

impl Plugin for BuildingStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<BuildingMaterial>::default(),
            MaterialPlugin::<ArrayMaterial>::default(),
        ))
        .add_systems(PreStartup, setup)
        .add_game_systems(First, |mut s: ResMut<BuildingStates>| s.writes.clear());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .add_systems(ExtractSchedule, extract)
                .add_systems(Render, upload.in_set(RenderSystems::PrepareResources));
        }
    }
}

fn setup(mut commands: Commands, mut buffers: ResMut<Assets<ShaderBuffer>>) {
    commands.insert_resource(BuildingStates::new(&mut buffers));
}

fn extract(mut commands: Commands, states: Extract<Option<Res<BuildingStates>>>) {
    if let Some(s) = states.as_deref() {
        commands.insert_resource(StateUploads {
            handle: s.handle.clone(),
            writes: s.writes.clone(),
        });
    }
}

/// Writes this frame's ranges into the GPU buffer in place.
fn upload(uploads: Option<Res<StateUploads>>, gpu: Res<RenderAssets<GpuShaderBuffer>>, queue: Res<RenderQueue>) {
    let Some(u) = uploads else { return };
    let Some(buffer) = gpu.get(&u.handle) else { return };
    for (base, words) in &u.writes {
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        queue.write_buffer(&buffer.buffer, u64::from(*base) * 4, &bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_check::{compile, with_bevy};

    #[test]
    fn the_building_state_shaders_compile_against_bevy_pbr() {
        let Some(resolver) = with_bevy() else {
            eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
            return;
        };
        // The main pass of a textured surface (normals, UVs, tangents, colours), and of a plain one.
        // Bevy's mesh pipeline always sets MESH_PIPELINE and VERTEX_OUTPUT_INSTANCE_INDEX.
        let base = [
            ("MESH_PIPELINE", true),
            ("VERTEX_OUTPUT_INSTANCE_INDEX", true),
            ("VERTEX_POSITIONS", true),
            ("VERTEX_NORMALS", true),
            ("VERTEX_UVS_A", true),
            ("VERTEX_COLORS", true),
        ];
        for tangents in [true, false] {
            let mut flags = base.to_vec();
            flags.push(("VERTEX_TANGENTS", tangents));
            compile(&resolver, "building_state", &flags);
        }
        // The prepass: depth only (the shadow maps, with the directional lights' depth clamp), and
        // the depth and normal prepass SSAO reads.
        for (normals, clamp) in [(false, true), (true, false)] {
            let mut flags = base.to_vec();
            flags.extend([
                ("NORMAL_PREPASS_OR_DEFERRED_PREPASS", normals),
                ("VERTEX_TANGENTS", normals),
                ("UNCLIPPED_DEPTH_ORTHO_EMULATION", clamp),
            ]);
            compile(&resolver, "building_state_prepass", &flags);
        }
        // The building fragment as the pilot's eyes draw it (SSAO, fog) and the plainest.
        for (ssao, fog) in [(true, true), (false, false)] {
            let mut flags = base.to_vec();
            flags.extend([
                ("VERTEX_TANGENTS", true),
                ("CLUSTERED_FORWARD_STORAGE_BUFFER", true),
                ("DEPTH_PREPASS", true),
                ("SCREEN_SPACE_AMBIENT_OCCLUSION", ssao),
                ("DISTANCE_FOG", fog),
            ]);
            compile(&resolver, "building_surface", &flags);
        }
        // The array material's fragment, as the pilot's eyes draw it (HDR, no MSAA, a depth
        // prepass, fog, SSAO) and the plainest.
        let lit = [
            ("VERTEX_UVS_B", true),
            ("VERTEX_TANGENTS", true),
            ("CLUSTERED_FORWARD_STORAGE_BUFFER", true),
        ];
        for (prepass, fog, ssao) in [(true, true, true), (false, false, false)] {
            let mut flags = base.to_vec();
            flags.extend(lit);
            flags.extend([
                ("DEPTH_PREPASS", prepass),
                ("DISTANCE_FOG", fog),
                ("SCREEN_SPACE_AMBIENT_OCCLUSION", ssao),
            ]);
            compile(&resolver, "building_array", &flags);
        }
    }

    #[test]
    fn slot_zero_is_always_shown_and_elements_start_at_one() {
        assert_eq!(slot(0, 0), 1.0);
        assert_eq!(slot(1000, 4999), 6000.0);
        assert_eq!(
            slot(1_000_000, 4999) as u32 - 1,
            1_004_999,
            "a slot is exact in an f32 colour channel"
        );
    }

    #[test]
    fn each_building_gets_its_own_range_and_the_buffer_grows() {
        let mut buffers = Assets::<ShaderBuffer>::default();
        let mut s = BuildingStates::new(&mut buffers);
        let first = s.handle.clone();
        assert_eq!(s.base(7, 100, &mut buffers), 0);
        assert_eq!(s.base(3, 50, &mut buffers), 100);
        assert_eq!(s.base(7, 100, &mut buffers), 0, "a building keeps its range");
        s.write(3, &[0; 50]);
        assert_eq!(s.writes, vec![(100, vec![0; 50])]);
        s.write(3, &[0; 50]);
        assert_eq!(s.writes.len(), 1, "an unchanged range is not written again");
        let big = s.base(9, FIRST_CAPACITY, &mut buffers);
        assert_eq!(big, 150);
        assert_ne!(s.handle, first, "a full buffer is doubled into a new one");
    }
}
