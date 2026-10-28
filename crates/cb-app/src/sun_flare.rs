//! The sun's glow and lens flare on the pilot's eyes (openspec change `sun-flare`; owner,
//! 2026-10-24: "do we have a sun? and epic lensflare?").
//!
//! Two things: Bevy's `Bloom` on the pilot's eyes camera, tuned in data, so the sun disk and the
//! bright sky glow; and the lens flare, our own post pass. Bevy 0.20 has no render graph: the passes
//! are systems in the `Core3d` schedule, after the bloom and before the water's passes and
//! tonemapping on the eyes (so drops on the glass refract the flare too). The flare adds light only:
//! it never reads or moves the view's transform (CLAUDE.md section 8).
//!
//! The flare fades when the sun is hidden: the world's depth is sampled at 17 taps round the sun on
//! the eyes camera, and the cockpit camera (which draws the cabin after the world, on its own depth)
//! samples the same taps against its own depth into a 1x1 texture the eyes read the next frame, so
//! the frame and pillars hide the sun as buildings do. It fades off screen, in cloud and under water
//! at the eye, and scales with the sun's light and the exposure. The rules are
//! `cb_core::sun_flare`; the shader is `assets/shaders/sun_flare.wesl`; the look is
//! `data/world/sun_flare.ron`. `CB_NO_FLARE=1` leaves the flare out and `CB_NO_BLOOM=1` the bloom,
//! for the benchmark.

use crate::pilot::{CockpitCamera, PilotCamera};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use crate::water_render::{WaterEye, WaterPasses, samplable_depth};
use crate::world::Plan;
use bevy::camera::{Exposure, Hdr};
use bevy::core_pipeline::tonemapping::tonemapping;
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter, bloom};
use bevy::prelude::*;
use bevy::render::render_resource::binding_types::{texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendComponent, BlendFactor, BlendOperation, BlendState,
    CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FragmentState, LoadOp, MultisampleState, Operations, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
    SpecializedRenderPipelines, StoreOp, TextureDataOrder, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, UniformBuffer, VertexState,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::sync_world::RenderEntity;
use bevy::render::view::{ExtractedView, Msaa, ViewDepthStencilTexture, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::{Shader, ShaderDefVal};
use cb_core::sun_flare::{
    BloomLook, FlareLook, GhostShape, MAX_GHOSTS, OCCLUSION_TAPS, fade_toward, medium_fade, on_screen, screen_heights, starburst_angle,
    strength, sun_ndc, tap_uvs,
};

/// The flare's shader, loaded from `assets/`.
const SHADER: &str = "shaders/sun_flare.wesl";
/// The cockpit's share of the sun, one texel.
const SEEN_FORMAT: TextureFormat = TextureFormat::R16Float;

/// The pilot's eyes see the flare: the pass draws over this camera's image.
#[derive(Component, Default)]
#[require(Hdr, SunFlareMedium)]
pub struct SunFlareEye;

/// The cockpit camera tests the sun against its own depth (the cabin's frame and pillars) for the
/// eyes' flare.
#[derive(Component, Default)]
pub struct SunFlareCockpit;

/// How much of the flare the cloud and water at the eye let through, 0..1, following the eye
/// smoothly (`cb_core::sun_flare::fade_toward`).
#[derive(Component, Clone, Copy, Debug)]
pub struct SunFlareMedium(pub f32);

impl Default for SunFlareMedium {
    fn default() -> Self {
        Self(1.0)
    }
}

/// The bloom and the flare on the pilot's eyes.
pub struct SunFlarePlugin {
    /// Draw the lens flare (`CB_NO_FLARE=1` turns it off).
    pub flare: bool,
    /// Bloom the eyes' image (`CB_NO_BLOOM=1` turns it off).
    pub bloom: bool,
}

/// Which settings the plugin was built with, for the camera setup.
#[derive(Resource, Clone, Copy)]
struct SunFlareSwitches {
    flare: bool,
    bloom: bool,
}

impl Plugin for SunFlarePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SunFlareSwitches {
            flare: self.flare,
            bloom: self.bloom,
        })
        .add_game_systems(Update, equip_cameras);
        if !self.flare {
            return;
        }
        app.add_game_systems(
            Update,
            (
                samplable_depth::<SunFlareEye>,
                samplable_depth::<SunFlareCockpit>,
                track_medium.after(PresentSet::Camera),
            ),
        );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<FlarePipeline>>()
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(ExtractSchedule, extract_flare)
            .add_systems(Render, prepare_flare.in_set(RenderSystems::Prepare))
            .add_systems(
                Core3d,
                sun_flare
                    .in_set(Core3dSystems::PostProcess)
                    .after(bloom)
                    .before(WaterPasses)
                    .before(tonemapping),
            );
    }
}

/// Bevy's `Bloom` from the data.
pub fn bloom_from(look: &BloomLook) -> Bloom {
    Bloom {
        intensity: look.intensity,
        low_frequency_boost: look.low_frequency_boost,
        low_frequency_boost_curvature: look.low_frequency_boost_curvature,
        high_pass_frequency: look.high_pass_frequency,
        prefilter: BloomPrefilter {
            threshold: look.threshold,
            threshold_softness: look.threshold_softness,
        },
        composite_mode: if look.additive {
            BloomCompositeMode::Additive
        } else {
            BloomCompositeMode::EnergyConserving
        },
        ..Bloom::NATURAL
    }
}

/// Gives the pilot's eyes their bloom and flare, and the cockpit camera its share of the flare's
/// occlusion. The cockpit camera never blooms: it draws the cabin over the eyes' finished image,
/// after their bloom, so the cabin and its lit panels stay crisp.
fn equip_cameras(
    mut commands: Commands,
    data: Res<Data>,
    switches: Res<SunFlareSwitches>,
    eyes: Query<Entity, Added<PilotCamera>>,
    cockpits: Query<Entity, Added<CockpitCamera>>,
) {
    for e in &eyes {
        let mut eye = commands.entity(e);
        if switches.bloom {
            eye.insert(bloom_from(&data.0.sun.bloom));
        }
        if switches.flare {
            eye.insert(SunFlareEye);
        }
    }
    if switches.flare {
        for e in &cockpits {
            commands.entity(e).insert(SunFlareCockpit);
        }
    }
}

/// Follows the cloud and water at the eye: the flare goes out in cloud and under water.
fn track_medium(
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time>,
    mut eyes: Query<(&Transform, Option<&WaterEye>, &mut SunFlareMedium), With<SunFlareEye>>,
) {
    let look = &data.0.sun.flare;
    for (transform, water, mut medium) in &mut eyes {
        let submersion = water.map_or(0.0, |w| w.submersion);
        let cloud = plan.clouds.density_at(transform.translation, time.elapsed_secs());
        let target = medium_fade(submersion, cloud, look.cloud_full_fade);
        medium.0 = fade_toward(medium.0, target, time.delta_secs(), look.fade_s);
    }
}

// ---- The passes (render world) -----------------------------------------------------------------

/// Which camera a view is to the flare.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Role {
    /// The pilot's eyes: the flare is drawn here.
    Eye,
    /// The cockpit camera: its depth decides how much of the sun the cabin leaves clear.
    Cockpit,
}

/// What the passes need from the main world, per view.
#[derive(Component, Clone, Copy)]
struct FlareView {
    role: Role,
    /// Cloud and water at the eye, 0..1 (1 for the cockpit).
    medium: f32,
    /// The flare's strength: the sun's exposed light, linear RGB, times the gain.
    light: Vec3,
    /// Toward the sun.
    toward_sun: Vec3,
}

/// The look, as the render world holds it (data is fixed for the run).
#[derive(Resource)]
struct RenderFlareLook(FlareLook);

fn extract_flare(
    mut commands: Commands,
    data: Extract<Res<Data>>,
    look: Option<Res<RenderFlareLook>>,
    sun: Extract<Query<(&DirectionalLight, &GlobalTransform, &crate::day_night::SkyBody), With<crate::world::Sun>>>,
    eyes: Extract<Query<(RenderEntity, &SunFlareMedium, Option<&Exposure>), With<SunFlareEye>>>,
    cockpits: Extract<Query<RenderEntity, With<SunFlareCockpit>>>,
) {
    let flare = &data.0.sun.flare;
    if look.is_none() {
        commands.insert_resource(RenderFlareLook(flare.clone()));
    }
    // The sun (`world::Sun`), as the water's light has it.
    let Some((light, transform, body)) = sun.iter().next() else {
        return;
    };
    let toward_sun = transform.back().as_vec3();
    let c = LinearRgba::from(light.color);
    let colour = Vec3::new(c.red, c.green, c.blue);
    for (entity, medium, exposure) in &eyes {
        let exposure = exposure.copied().unwrap_or_default().exposure();
        if let Ok(mut e) = commands.get_entity(entity) {
            e.insert(FlareView {
                role: Role::Eye,
                medium: medium.0,
                // No flare from a sun below the horizon (`day_night`).
                light: colour * strength(light.illuminance * body.visible, exposure, flare.gain),
                toward_sun,
            });
        }
    }
    for entity in &cockpits {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.insert(FlareView {
                role: Role::Cockpit,
                medium: 1.0,
                light: Vec3::ZERO,
                toward_sun,
            });
        }
    }
}

/// `SunFlare` in `sun_flare.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
struct SunFlareUniform {
    sun: Vec4,
    light: Vec4,
    core: Vec4,
    glow: Vec4,
    spikes: Vec4,
    spike_shape: Vec4,
    rays: Vec4,
    streak: Vec4,
    halo: Vec4,
    shape: Vec4,
    ghost_look: Vec4,
    ghosts: [Vec4; MAX_GHOSTS],
    ghost_colours: [Vec4; MAX_GHOSTS],
    taps: [Vec4; OCCLUSION_TAPS],
}

fn v3(c: [f32; 3]) -> Vec3 {
    Vec3::from_array(c)
}

impl SunFlareUniform {
    /// The uniform for the sun at `ndc` in a view `aspect` wide, with the
    /// flare's `light` and the CPU's `visibility` (on screen, cloud, water).
    fn new(look: &FlareLook, ndc: Vec2, aspect: f32, light: Vec3, visibility: f32) -> Self {
        let mut ghosts = [Vec4::ZERO; MAX_GHOSTS];
        let mut ghost_colours = [Vec4::ZERO; MAX_GHOSTS];
        for (i, g) in look.ghosts.iter().take(MAX_GHOSTS).enumerate() {
            let hexagon = if g.shape == GhostShape::Hex { 1.0 } else { 0.0 };
            ghosts[i] = Vec4::new(g.offset, g.radius_h, g.softness, hexagon);
            ghost_colours[i] = v3(g.colour).extend(0.0);
        }
        let count = look.ghosts.len().min(MAX_GHOSTS) as f32;
        Self {
            sun: screen_heights(ndc, aspect).extend(aspect).extend(visibility),
            light: light.extend(starburst_angle(screen_heights(ndc, aspect), look.spike_turn_rad_per_h)),
            core: v3(look.core_colour).extend(look.core_radius_h),
            glow: v3(look.glow_colour).extend(look.glow_radius_h),
            spikes: v3(look.spike_colour).extend(look.spike_length_h),
            spike_shape: Vec4::new(
                look.spike_count as f32,
                look.spike_sharpness,
                look.spike_fringe,
                look.ray_count as f32,
            ),
            rays: Vec4::new(look.ray_strength, look.ray_length_h, look.glow_power, count),
            streak: v3(look.streak_colour).extend(look.streak_length_h),
            halo: v3(look.halo_colour).extend(look.halo_radius_h),
            shape: Vec4::new(look.streak_thickness_h, look.halo_width_h, look.halo_fringe, 0.0),
            ghost_look: Vec4::new(look.ghost_fringe, look.ghost_rim, look.aperture_deg.to_radians(), 0.0),
            ghosts,
            ghost_colours,
            taps: tap_uvs(ndc, aspect, look.occlusion_radius_h),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct FlareKey {
    role: Role,
    format: TextureFormat,
    samples: u32,
}

#[derive(Resource)]
struct FlarePipeline {
    shader: Handle<Shader>,
    eye: BindGroupLayoutDescriptor,
    eye_multisampled: BindGroupLayoutDescriptor,
    cockpit: BindGroupLayoutDescriptor,
    cockpit_multisampled: BindGroupLayoutDescriptor,
    /// The share of the sun the cockpit's frame left clear, one texel, written by the cockpit
    /// camera and read by the eyes the next frame.
    seen: TextureView,
}

fn layout(role: Role, multisampled: bool) -> BindGroupLayoutDescriptor {
    let depth = if multisampled {
        texture_depth_2d_multisampled()
    } else {
        texture_depth_2d()
    };
    let uniform = uniform_buffer::<SunFlareUniform>(false);
    match role {
        Role::Eye => BindGroupLayoutDescriptor::new(
            "sun flare: uniform, depth, cockpit's share",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (uniform, depth, texture_2d(TextureSampleType::Float { filterable: false })),
            ),
        ),
        Role::Cockpit => BindGroupLayoutDescriptor::new(
            "sun flare taps: uniform, depth",
            &BindGroupLayoutEntries::sequential(ShaderStages::VERTEX_FRAGMENT, (uniform, depth)),
        ),
    }
}

fn init_pipeline(mut commands: Commands, assets: Res<AssetServer>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    // Starts at 1 (all clear), so the flare is right before the cockpit camera's first test.
    let one = half_one();
    let seen = device.create_texture_with_data(
        &queue,
        &TextureDescriptor {
            label: Some("sun flare: cockpit's share"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: SEEN_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        TextureDataOrder::LayerMajor,
        &one,
    );
    commands.insert_resource(FlarePipeline {
        shader: assets.load(SHADER),
        eye: layout(Role::Eye, false),
        eye_multisampled: layout(Role::Eye, true),
        cockpit: layout(Role::Cockpit, false),
        cockpit_multisampled: layout(Role::Cockpit, true),
        seen: seen.create_view(&TextureViewDescriptor::default()),
    });
}

/// 1.0 as an IEEE half float, little-endian.
fn half_one() -> [u8; 2] {
    0x3C00u16.to_le_bytes()
}

impl FlarePipeline {
    fn layout(&self, role: Role, multisampled: bool) -> &BindGroupLayoutDescriptor {
        match (role, multisampled) {
            (Role::Eye, false) => &self.eye,
            (Role::Eye, true) => &self.eye_multisampled,
            (Role::Cockpit, false) => &self.cockpit,
            (Role::Cockpit, true) => &self.cockpit_multisampled,
        }
    }
}

impl SpecializedRenderPipeline for FlarePipeline {
    type Key = FlareKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.samples > 1 {
            shader_defs.push(ShaderDefVal::from("MULTISAMPLED"));
        }
        let (label, vertex, fragment, format, blend, samples) = match key.role {
            // Added onto the image: light only, the image's alpha kept.
            Role::Eye => (
                "sun flare",
                "flare_vertex",
                "flare_fragment",
                key.format,
                Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                }),
                key.samples,
            ),
            Role::Cockpit => (
                "sun flare taps: the cockpit's share",
                "taps_vertex",
                "taps_fragment",
                SEEN_FORMAT,
                None,
                1,
            ),
        };
        let multisampled = key.samples > 1;
        RenderPipelineDescriptor {
            label: Some(label.into()),
            layout: vec![self.layout(key.role, multisampled).clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some(vertex.into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(fragment.into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            multisample: MultisampleState {
                count: samples,
                ..default()
            },
            ..default()
        }
    }
}

/// A view's flare on the GPU: its uniform, its pipeline and whether it runs this frame.
#[derive(Component)]
struct FlareGpu {
    uniform: UniformBuffer<SunFlareUniform>,
    pipeline: CachedRenderPipelineId,
    role: Role,
    multisampled: bool,
    run: bool,
}

/// What the CPU decides for a view: where the sun is (NDC) and how much of the flare the view
/// shows before the depth taps (on screen, cloud, water). `None`: nothing to draw or test.
fn placed(view: &FlareView, clip_from_world: Mat4, margin: f32) -> Option<(Vec2, f32)> {
    let ndc = sun_ndc(clip_from_world, view.toward_sun)?;
    let visibility = on_screen(ndc, margin) * view.medium.clamp(0.0, 1.0);
    let lit = view.role == Role::Cockpit || view.light.max_element() > 0.0;
    (visibility > 0.0 && lit).then_some((ndc, visibility))
}

fn prepare_flare(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<FlarePipeline>,
    look: Option<Res<RenderFlareLook>>,
    mut specialized: ResMut<SpecializedRenderPipelines<FlarePipeline>>,
    mut views: Query<(Entity, &ExtractedView, &Msaa, &FlareView, Option<&mut FlareGpu>)>,
) {
    let Some(look) = look else {
        return;
    };
    let look = &look.0;
    for (entity, view, msaa, flare, gpu) in &mut views {
        let clip_from_world = view
            .clip_from_world
            .unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
        let size = view.viewport.zw().as_vec2();
        let aspect = if size.y > 0.0 { size.x / size.y } else { 1.0 };
        let placed = placed(flare, clip_from_world, look.offscreen_margin_ndc);
        let (ndc, visibility) = placed.unwrap_or((Vec2::splat(4.0), 0.0));
        let uniform = SunFlareUniform::new(look, ndc, aspect, flare.light, visibility);
        let samples = msaa.samples();
        let id = specialized.specialize(
            &cache,
            &pipeline,
            FlareKey {
                role: flare.role,
                format: view.target_format,
                samples,
            },
        );
        let run = placed.is_some();
        if let Some(mut gpu) = gpu {
            gpu.uniform.set(uniform);
            gpu.uniform.write_buffer(&device, &queue);
            gpu.pipeline = id;
            gpu.role = flare.role;
            gpu.multisampled = samples > 1;
            gpu.run = run;
            continue;
        }
        let mut buffer = UniformBuffer::from(uniform);
        buffer.write_buffer(&device, &queue);
        commands.entity(entity).insert(FlareGpu {
            uniform: buffer,
            pipeline: id,
            role: flare.role,
            multisampled: samples > 1,
            run,
        });
    }
}

/// Draws the flare over the eyes' image, or tests the sun against the cockpit's depth into the
/// one-texel share, for the current view.
fn sun_flare(
    view: ViewQuery<(&ViewTarget, &ViewDepthStencilTexture, &FlareGpu)>,
    pipeline: Res<FlarePipeline>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, depth, gpu) = view.into_inner();
    if !gpu.run {
        return;
    }
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        return;
    };
    let Some(uniform) = gpu.uniform.binding() else {
        return;
    };
    let Some(render_pipeline) = cache.get_render_pipeline(gpu.pipeline) else {
        return;
    };
    let layout = cache.get_bind_group_layout(pipeline.layout(gpu.role, gpu.multisampled));
    let (bind_group, attachment) = match gpu.role {
        Role::Eye => (
            ctx.render_device().create_bind_group(
                Some("sun_flare"),
                &layout,
                &BindGroupEntries::sequential((uniform, depth_view, &pipeline.seen)),
            ),
            target.get_color_attachment(),
        ),
        Role::Cockpit => (
            ctx.render_device().create_bind_group(
                Some("sun_flare_taps"),
                &layout,
                &BindGroupEntries::sequential((uniform, depth_view)),
            ),
            RenderPassColorAttachment {
                view: &pipeline.seen,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Default::default()),
                    store: StoreOp::Store,
                },
            },
        ),
    };
    let label = match gpu.role {
        Role::Eye => "sun_flare",
        Role::Cockpit => "sun_flare_taps",
    };
    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(attachment)],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(render_pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests;
