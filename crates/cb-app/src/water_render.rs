//! How water looks (openspec change `water-rendering`): the one water material every river, lake
//! and the ocean is drawn with, the eye's wetness each frame, and the two fullscreen passes that
//! show the view under water and the water running off the glass after surfacing.
//!
//! Ported from Pale Blue Dot (`RubenTipparach/pale-blue-dot` at `db25fe2`,
//! `crates/pbd-app/src/planet_water.rs`): the view's water state, the compose and lens passes run in
//! Tenebris's order before tonemapping, and the uniform built per view in the render world. Bevy
//! 0.20 has no render graph: the passes are systems in the `Core3d` schedule. The cap pass becomes a
//! Bevy `Material` in the transmissive phase. What changes for a flat world is in the design.
//!
//! The rules (the tuning, under or straddling or dry, the drops' window) are `cb_core::water_look`;
//! the shaders are `assets/shaders/water_{common,surface,post}.wesl`.

use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::world::{Plan, SKY};
use bevy::camera::{Exposure, Hdr};
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::tonemapping::tonemapping;
use bevy::core_pipeline::{Core3d, Core3dSystems, FullscreenShader};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, ScreenSpaceTransmission};
use bevy::prelude::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer};
use bevy::render::render_resource::{
    AsBindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
    ColorWrites, FilterMode, FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, SpecializedMeshPipelineError,
    SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, TextureSampleType, TextureUsages, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::sync_world::RenderEntity;
use bevy::render::view::{ExtractedView, Msaa, ViewDepthStencilTexture, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::{Shader, ShaderDefVal, ShaderRef};
use cb_core::pilot::PilotMode;
use cb_core::water_look::{Lens, WaterLook, submersion};
use cb_core::world::Ground;

/// The water surface shader, loaded from `assets/`.
const SURFACE_SHADER: &str = "shaders/water_surface.wesl";
/// The fullscreen passes' shader.
const POST_SHADER: &str = "shaders/water_post.wesl";

/// Marks a camera that sees water the pilot's way: under water, and with drops on its glass. It
/// brings what the water needs: an HDR target (the water mixes linear light), a depth prepass (the
/// ground under the water) and screen-space transmission (the scene behind the water).
#[derive(Component, Default)]
#[require(Hdr, DepthPrepass, ScreenSpaceTransmission, WaterEye)]
pub struct WaterOptics;

/// How wet the eye is this frame, decided once in the main world (`track_eye`) and read by the fog,
/// the water material and the passes.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WaterEye {
    /// The water surface's height over the eye, metres (the last one seen when dry).
    pub surface_m: f32,
    /// How far the eye is below that surface, metres (negative above); `None` with no water there.
    pub depth_m: Option<f32>,
    /// 0 dry, 0.5 straddling the surface, 1 under water (`cb_core::water_look::submersion`).
    pub submersion: f32,
    /// How strongly drops run on the glass, 0..1.
    pub drips: f32,
    /// Rain on the glass from cloud, 0..1 (`cb_core::water_look::cloud_glass`).
    pub rain: f32,
    /// The sealed cabin's dry radius about the eye, metres; 0 on foot.
    pub cabin_m: f32,
    /// The glass's memory of the water.
    pub lens: Lens,
}

impl WaterEye {
    /// Whether the water, not the air, decides the view's fog.
    pub fn under_water(&self) -> bool {
        self.submersion > 0.75
    }
}

/// The water material's uniform; `WaterSurface` in `water_surface.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct WaterSurfaceUniform {
    ripple: Vec4,
    motion: Vec4,
    refraction: Vec4,
    absorption: Vec4,
    deep: Vec4,
    light: Vec4,
    sky_horizon: Vec4,
    sky_zenith: Vec4,
    foam_color: Vec4,
    foam_crest: Vec4,
    foam_slope: Vec4,
    sun_dir: Vec4,
    sun_light: Vec4,
    sun_tint: Vec4,
    cabin: Vec4,
    swell: [Vec4; 3],
}

/// The light the water sees this frame, all times the camera's exposure.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaterLight {
    /// Toward the sun.
    pub sun_dir: Vec3,
    /// The sun's light.
    pub sun: Vec3,
    /// The light reaching the water body from above: the sun over pi on a level surface and the
    /// ambient light (Bevy's Lambert and ambient terms, so the water sits in the same light as the
    /// ground).
    pub body: Vec3,
}

fn v3(c: [f32; 3]) -> Vec3 {
    Vec3::from_array(c)
}

impl WaterSurfaceUniform {
    /// The uniform for `look`, at `seconds`, in `light`, with the eye at `eye` and the cabin dry
    /// for `cabin_m` about it.
    fn new(look: &WaterLook, seconds: f32, light: &WaterLight, eye: Vec3, cabin_m: f32) -> Self {
        let sky = LinearRgba::from(SKY);
        let swell = std::array::from_fn(|i| {
            let heading = look.swell_heading_deg[i].to_radians();
            let k = std::f32::consts::TAU / look.swell_wavelength_m[i];
            Vec4::new(heading.cos(), heading.sin(), k, look.swell_height_m[i])
        });
        Self {
            ripple: Vec4::new(look.ripple_scale_per_m, look.ripple_speed, look.ripple_steepness, look.slope_max),
            motion: Vec4::new(seconds * look.time_scale, look.detail_fade, look.flow_period_s, seconds),
            refraction: Vec4::new(look.refract_amount, look.refract_max_uv, look.max_path_m, look.light_falloff),
            absorption: v3(look.absorption_per_m).extend(look.sky_horizon_strength),
            deep: v3(look.deep_color).extend(0.0),
            light: light.body.extend(0.0),
            sky_horizon: Vec4::new(sky.red, sky.green, sky.blue, 0.0),
            sky_zenith: v3(look.sky_zenith_color).extend(0.0),
            foam_color: v3(look.foam_color).extend(look.foam_intensity),
            foam_crest: Vec4::new(look.foam_crest[0], look.foam_crest[1], look.foam_crest_weight, look.shore_foam_m),
            foam_slope: Vec4::new(
                look.foam_slope[0],
                look.foam_slope[1],
                look.foam_slope_weight,
                look.shore_foam_weight,
            ),
            sun_dir: light.sun_dir.extend(look.specular_intensity),
            sun_light: light.sun.extend(look.specular_power),
            sun_tint: v3(look.sun_tint).extend(0.0),
            cabin: eye.extend(cabin_m),
            swell,
        }
    }
}

/// The one water material: every river, lake and the ocean (CLAUDE.md 6.1). Its uniform is
/// rewritten each frame (time, light, the eye); a river's flow comes from its mesh's vertex colour
/// (rg, metres a second), so one material serves still and running water alike.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct WaterMaterial {
    /// The look, as the shader reads it.
    #[uniform(0)]
    pub surface: WaterSurfaceUniform,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        SURFACE_SHADER.into()
    }

    // The scene behind the water is the opaque pass's copy: the water draws in the transmissive
    // phase.
    fn reads_view_transmission_texture(&self) -> bool {
        true
    }

    // Kept out of the depth prepass, so the prepass depth is the ground under the water.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    // Nothing is culled: from under water the surface is seen from below.
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

impl WaterMaterial {
    /// A water material with `look`; its per-frame values (time, light, the eye) are filled in by
    /// `refresh_materials`. Made through `world::water_material`, the one helper.
    pub fn new(look: &WaterLook) -> Self {
        Self {
            surface: WaterSurfaceUniform::new(look, 0.0, &WaterLight::default(), Vec3::ZERO, 0.0),
        }
    }
}

/// The water's look and the view under and coming out of it.
pub struct WaterRenderPlugin;

/// The water's fullscreen passes in the `Core3d` schedule, for other passes to order against (the
/// sun flare draws before them, so drops on the glass refract it).
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WaterPasses;

impl Plugin for WaterRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default()).add_systems(
            Update,
            (
                samplable_depth::<WaterOptics>,
                refresh_materials.after(crate::sim::PresentSet::Camera),
            ),
        );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<WaterPostPipeline>>()
            .add_systems(RenderStartup, init_post_pipeline)
            .add_systems(ExtractSchedule, extract_water_eye)
            .add_systems(Render, prepare_water_post.in_set(RenderSystems::Prepare))
            .add_systems(
                Core3d,
                water_post
                    .in_set(Core3dSystems::PostProcess)
                    .in_set(WaterPasses)
                    .before(tonemapping),
            );
    }
}

/// A pass that reads the main depth needs it samplable: makes the depth buffer of every camera
/// that gains `M` samplable (`WaterOptics` for the compose pass; the sun flare's cameras too).
pub fn samplable_depth<M: Component>(mut cameras: Query<&mut Camera3d, Added<M>>) {
    for mut camera in &mut cameras {
        let usages = TextureUsages::from(camera.depth_texture_usages) | TextureUsages::TEXTURE_BINDING;
        camera.depth_texture_usages = usages.into();
    }
}

/// Decides how wet each `WaterOptics` camera's eye is, from the one water query. Runs after the
/// camera is placed and before the fog is decided (`pilot::PilotPlugin` chains it).
pub fn track_eye(
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time>,
    pilot: Res<PilotSim>,
    mut cameras: Query<(&Transform, &mut WaterEye), With<WaterOptics>>,
) {
    let look = &data.0.water_look;
    let seated = pilot.curr.mode == PilotMode::Seated;
    for (transform, mut eye) in &mut cameras {
        let p = transform.translation;
        let water = plan.water_at(p.x, p.z);
        if let Some(w) = water {
            eye.surface_m = w.surface_m;
        }
        eye.depth_m = water.map(|w| w.surface_m - p.y);
        eye.submersion = submersion(eye.depth_m, look.partial_band_m);
        let state = eye.submersion;
        eye.drips = eye.lens.step(time.elapsed_secs(), state, look.emerge_dry_s);
        // Flying through a cloud wets the glass (the external cameras' lenses when seated).
        let density = plan.clouds.density_at(p, time.elapsed_secs());
        eye.rain = cb_core::water_look::cloud_glass(eye.rain, density, time.delta_secs(), look);
        eye.cabin_m = if seated { look.cabin_dry_radius_m } else { 0.0 };
    }
}

/// The light the water sees: the first directional light (the sun), the ambient light and the
/// camera's exposure, as Bevy's lighting would apply them.
pub(crate) fn water_light(sun: Option<(&DirectionalLight, &GlobalTransform)>, ambient: &GlobalAmbientLight, exposure: f32) -> WaterLight {
    let (sun_dir, sun) = sun.map_or((Vec3::Y, Vec3::ZERO), |(light, transform)| {
        let c = LinearRgba::from(light.color);
        (
            transform.back().as_vec3(),
            Vec3::new(c.red, c.green, c.blue) * light.illuminance * exposure,
        )
    });
    let a = LinearRgba::from(ambient.color);
    let ambient = Vec3::new(a.red, a.green, a.blue) * ambient.brightness * exposure;
    WaterLight {
        sun_dir,
        sun,
        body: sun * sun_dir.y.max(0.0) / std::f32::consts::PI + ambient,
    }
}

/// Rewrites every water material's uniform for this frame: time, light and the eye.
fn refresh_materials(
    data: Res<Data>,
    time: Res<Time>,
    ambient: Res<GlobalAmbientLight>,
    sun: Query<(&DirectionalLight, &GlobalTransform)>,
    cameras: Query<(&Transform, &WaterEye, Option<&Exposure>), With<WaterOptics>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    let Some((transform, eye, exposure)) = cameras.iter().next() else {
        return;
    };
    let exposure = exposure.copied().unwrap_or_default().exposure();
    let light = water_light(sun.iter().next(), &ambient, exposure);
    let surface = WaterSurfaceUniform::new(&data.0.water_look, time.elapsed_secs(), &light, transform.translation, eye.cabin_m);
    for (_, material) in materials.iter_mut() {
        material.surface = surface;
    }
}

// ---- The fullscreen passes (render world) ------------------------------------------------------

/// What the passes need from the main world, per view.
#[derive(Component, Clone, Copy)]
struct WaterPostSettings {
    eye: WaterEye,
    seconds: f32,
    look: PostLook,
    light: Vec3,
}

/// The look's numbers the passes read.
#[derive(Clone, Copy)]
struct PostLook {
    absorption: Vec3,
    light_falloff: f32,
    deep: Vec3,
    distortion: f32,
    wet_blur: f32,
    band_m: f32,
    lens: Vec4,
    lens_scale: f32,
    lens_rain: f32,
}

impl From<&WaterLook> for PostLook {
    fn from(look: &WaterLook) -> Self {
        Self {
            absorption: v3(look.absorption_per_m),
            light_falloff: look.light_falloff,
            deep: v3(look.deep_color),
            distortion: look.underwater_distortion,
            wet_blur: look.wet_blur,
            band_m: look.partial_band_m,
            lens: Vec4::new(look.lens_density, look.lens_refract, look.lens_speed, look.lens_size),
            lens_scale: look.lens_scale,
            lens_rain: look.lens_rain,
        }
    }
}

fn extract_water_eye(
    mut commands: Commands,
    data: Extract<Res<Data>>,
    time: Extract<Res<Time>>,
    ambient: Extract<Res<GlobalAmbientLight>>,
    sun: Extract<Query<(&DirectionalLight, &GlobalTransform)>>,
    cameras: Extract<Query<(RenderEntity, &WaterEye, Option<&Exposure>), With<WaterOptics>>>,
) {
    let look = PostLook::from(&data.0.water_look);
    for (entity, eye, exposure) in &cameras {
        let exposure = exposure.copied().unwrap_or_default().exposure();
        let light = water_light(sun.iter().next(), &ambient, exposure);
        if let Ok(mut e) = commands.get_entity(entity) {
            e.insert(WaterPostSettings {
                eye: *eye,
                seconds: time.elapsed_secs(),
                look,
                light: light.body,
            });
        }
    }
}

/// `WaterPost` in `water_post.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
struct WaterPostUniform {
    world_from_clip: Mat4,
    camera: Vec4,
    surface: Vec4,
    absorption: Vec4,
    deep: Vec4,
    light: Vec4,
    lens: Vec4,
    lens_fx: Vec4,
}

/// Which fullscreen pass.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum PostPass {
    Compose,
    Lens,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct PostKey {
    pass: PostPass,
    format: TextureFormat,
    multisampled: bool,
}

#[derive(Resource)]
struct WaterPostPipeline {
    shader: Handle<Shader>,
    fullscreen: FullscreenShader,
    layout: BindGroupLayoutDescriptor,
    layout_multisampled: BindGroupLayoutDescriptor,
    sampler: Sampler,
}

fn post_layout(multisampled: bool) -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "water post: uniform, scene, depth, sampler",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<WaterPostUniform>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                if multisampled {
                    texture_depth_2d_multisampled()
                } else {
                    texture_depth_2d()
                },
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    )
}

fn init_post_pipeline(mut commands: Commands, assets: Res<AssetServer>, device: Res<RenderDevice>, fullscreen: Res<FullscreenShader>) {
    commands.insert_resource(WaterPostPipeline {
        shader: assets.load(POST_SHADER),
        fullscreen: fullscreen.clone(),
        layout: post_layout(false),
        layout_multisampled: post_layout(true),
        sampler: device.create_sampler(&SamplerDescriptor {
            label: Some("water post sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
    });
}

impl SpecializedRenderPipeline for WaterPostPipeline {
    type Key = PostKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.multisampled {
            shader_defs.push(ShaderDefVal::from("MULTISAMPLED"));
        }
        let (label, entry) = match key.pass {
            PostPass::Compose => ("water compose: under water, waterline, wet blur", "compose"),
            PostPass::Lens => ("water lens: drops on the glass", "lens"),
        };
        RenderPipelineDescriptor {
            label: Some(label.into()),
            layout: vec![if key.multisampled {
                self.layout_multisampled.clone()
            } else {
                self.layout.clone()
            }],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(entry.into()),
                targets: vec![Some(ColorTargetState {
                    format: key.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

/// A view's water passes on the GPU: its uniform, its pipelines and which passes run this frame.
#[derive(Component)]
struct WaterPostGpu {
    uniform: UniformBuffer<WaterPostUniform>,
    compose: CachedRenderPipelineId,
    lens: CachedRenderPipelineId,
    multisampled: bool,
    run_compose: bool,
    run_lens: bool,
}

/// Which passes a frame needs: compose while the eye is wet or the glass is (the wet blur), the
/// lens while drops run. Dry, neither runs and the water costs nothing here.
fn passes_needed(eye: &WaterEye, rain: f32) -> (bool, bool) {
    let glass = eye.drips > 0.001 || rain > 0.001;
    (eye.submersion > 0.25 || glass, glass && !eye.under_water())
}

fn prepare_water_post(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<WaterPostPipeline>,
    mut specialized: ResMut<SpecializedRenderPipelines<WaterPostPipeline>>,
    mut views: Query<(Entity, &ExtractedView, &Msaa, &WaterPostSettings, Option<&mut WaterPostGpu>)>,
) {
    for (entity, view, msaa, settings, gpu) in &mut views {
        let clip_from_world = view
            .clip_from_world
            .unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
        let eye = settings.eye;
        let look = settings.look;
        let uniform = WaterPostUniform {
            world_from_clip: clip_from_world.inverse(),
            camera: view.world_from_view.translation().extend(settings.seconds),
            surface: Vec4::new(eye.surface_m, eye.depth_m.unwrap_or(-1.0e3), look.band_m, eye.submersion),
            absorption: look.absorption.extend(look.light_falloff),
            deep: look.deep.extend(look.distortion),
            light: settings.light.extend(look.wet_blur),
            lens: look.lens,
            lens_fx: Vec4::new(eye.drips, look.lens_rain.max(eye.rain), look.lens_scale, eye.cabin_m),
        };
        let (run_compose, run_lens) = passes_needed(&eye, look.lens_rain.max(eye.rain));
        let multisampled = msaa.samples() > 1;
        // Cached by key: a lookup each frame, a new pipeline only if the format or MSAA changed.
        let mut id = |pass| {
            specialized.specialize(
                &cache,
                &pipeline,
                PostKey {
                    pass,
                    format: view.target_format,
                    multisampled,
                },
            )
        };
        let (compose, lens) = (id(PostPass::Compose), id(PostPass::Lens));
        if let Some(mut gpu) = gpu {
            gpu.uniform.set(uniform);
            gpu.uniform.write_buffer(&device, &queue);
            gpu.compose = compose;
            gpu.lens = lens;
            gpu.multisampled = multisampled;
            gpu.run_compose = run_compose;
            gpu.run_lens = run_lens;
            continue;
        }
        let mut buffer = UniformBuffer::from(uniform);
        buffer.write_buffer(&device, &queue);
        commands.entity(entity).insert(WaterPostGpu {
            uniform: buffer,
            compose,
            lens,
            multisampled,
            run_compose,
            run_lens,
        });
    }
}

/// Runs the compose and lens passes for the current view, each reading the main texture and
/// writing the other (Bevy's post-process swap).
fn water_post(
    view: ViewQuery<(&ViewTarget, &ViewDepthStencilTexture, &WaterPostGpu)>,
    pipeline: Res<WaterPostPipeline>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, depth, gpu) = view.into_inner();
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        return;
    };
    let Some(uniform) = gpu.uniform.binding() else {
        return;
    };
    let layout = cache.get_bind_group_layout(if gpu.multisampled {
        &pipeline.layout_multisampled
    } else {
        &pipeline.layout
    });
    for (run, id, label) in [
        (gpu.run_compose, gpu.compose, "water_compose"),
        (gpu.run_lens, gpu.lens, "water_lens"),
    ] {
        if !run {
            continue;
        }
        let Some(render_pipeline) = cache.get_render_pipeline(id) else {
            continue;
        };
        let post = target.post_process_write();
        let bind_group = ctx.render_device().create_bind_group(
            Some(label),
            &layout,
            &BindGroupEntries::sequential((uniform.clone(), post.source, depth_view, &pipeline.sampler)),
        );
        let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(render_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests;
