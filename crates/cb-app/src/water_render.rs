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
//!
//! The compose pass also lays the ground fog (openspec change `ground-fog`): everything between the
//! eye and a pixel, water or air, is fogged in the one pass. The fog's rules and its mist map are
//! `cb_core::world::mist`; the map is uploaded once as a texture. `CB_NO_GROUND_FOG=1` leaves the
//! air clear, and then the pass runs only while the eye or the glass is wet, as before.

use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::GameApp;
use crate::world::{Plan, SKY};
use bevy::camera::{Exposure, Hdr};
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::tonemapping::tonemapping;
use bevy::core_pipeline::{Core3d, Core3dSystems, FullscreenShader};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, ScreenSpaceTransmission};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer};
use bevy::render::render_resource::{
    AsBindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
    ColorWrites, FilterMode, FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, SpecializedMeshPipelineError,
    SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, TextureSampleType, TextureUsages, TextureView, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::sync_world::RenderEntity;
use bevy::render::texture::{FallbackImageZero, GpuImage};
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
    shallow: Vec4,
    scatter: Vec4,
    reflect: Vec4,
    whitecap: Vec4,
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
    /// `submersion`: the eye's, 0 dry, 0.5 straddling, 1 under water (`WaterEye::submersion`).
    fn new(look: &WaterLook, seconds: f32, light: &WaterLight, eye: Vec3, cabin_m: f32, submersion: f32) -> Self {
        let sky = LinearRgba::from(SKY);
        let trains = look.swell_trains();
        let swell = std::array::from_fn(|i| {
            let t = trains[i];
            Vec4::new(t.dir.x, t.dir.y, t.k_rad_m, t.height_m)
        });
        Self {
            ripple: Vec4::new(look.ripple_scale_per_m, look.ripple_speed, look.ripple_steepness, look.slope_max),
            motion: Vec4::new(seconds * look.time_scale, look.detail_fade, look.flow_period_s, seconds),
            refraction: Vec4::new(look.refract_amount, look.refract_max_uv, look.max_path_m, look.light_falloff),
            absorption: v3(look.absorption_per_m).extend(look.sky_horizon_strength),
            // w: the eye's submersion; only a wet eye sees water from below.
            deep: v3(look.deep_color).extend(submersion),
            light: light.body.extend(look.whitecap_break_dev),
            sky_horizon: Vec4::new(sky.red, sky.green, sky.blue, 0.0),
            sky_zenith: v3(look.sky_zenith_color).extend(look.glitter_twinkle_hz),
            foam_color: v3(look.foam_color).extend(look.foam_intensity),
            foam_crest: Vec4::new(look.foam_crest[0], look.foam_crest[1], look.foam_crest_weight, look.shore_foam_m),
            foam_slope: Vec4::new(
                look.foam_slope[0],
                look.foam_slope[1],
                look.foam_slope_weight,
                look.shore_foam_weight,
            ),
            sun_dir: light.sun_dir.extend(look.specular_intensity),
            sun_light: light.sun.extend(look.glint_roughness),
            sun_tint: v3(look.sun_tint).extend(look.glint_far_roughness),
            cabin: eye.extend(cabin_m),
            shallow: v3(look.shallow_color).extend(look.shallow_depth_m),
            scatter: (v3(look.scatter_color) * look.scatter_strength).extend(look.glitter_slope),
            reflect: Vec4::new(
                look.reflect_steps as f32,
                cb_core::water_look::reflect_growth(look.reflect_steps, look.reflect_max_m),
                look.reflect_strength,
                cb_core::water_look::REFLECT_FIRST_STEP_M,
            ),
            whitecap: Vec4::new(
                look.whitecap_coverage,
                look.whitecap_scale_m,
                look.whitecap_drift_m_s[0],
                look.whitecap_drift_m_s[1],
            ),
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
            surface: WaterSurfaceUniform::new(look, 0.0, &WaterLight::default(), Vec3::ZERO, 0.0, 0.0),
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
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .insert_resource(GroundFogOn(!crate::cli::switched_off("CB_NO_GROUND_FOG")))
            .add_world_spawn(upload_mist)
            .add_game_systems(
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
                (
                    water_fog
                        .in_set(Core3dSystems::MainPass)
                        .after(bevy::pbr::main_transmissive_pass_3d)
                        .before(bevy::core_pipeline::core_3d::main_transparent_pass_3d),
                    water_post
                        .in_set(Core3dSystems::PostProcess)
                        .in_set(WaterPasses)
                        .before(tonemapping),
                ),
            )
            .edit_schedule(Core3d, |schedule| {
                schedule.add_build_pass(FogAfterSky);
            });
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
        // Flying through a cloud or under a storm's rain shaft wets the glass (the external cameras' lenses
        // when seated).
        let density = plan.clouds.wetness_at(p, time.elapsed_secs());
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
    sun: Query<(&DirectionalLight, &GlobalTransform, &crate::day_night::SkyBody)>,
    cameras: Query<(&Transform, &WaterEye, Option<&Exposure>), With<WaterOptics>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    let Some((transform, eye, exposure)) = cameras.iter().next() else {
        return;
    };
    let exposure = exposure.copied().unwrap_or_default().exposure();
    let key = crate::day_night::key_light(sun.iter());
    let light = water_light(key.as_ref().map(|(l, t)| (l, t)), &ambient, exposure);
    let surface = WaterSurfaceUniform::new(
        &data.0.water_look,
        time.elapsed_secs(),
        &light,
        transform.translation,
        eye.cabin_m,
        eye.submersion,
    );
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
    fog: FogLook,
    mist: AssetId<Image>,
    glass: PaneGlass,
}

/// Most window triangles the lens pass reads (`panes` in `water_post.wesl`): a screen wall's panes
/// are a few quads.
const MAX_PANE_TRIANGLES: usize = 32;

/// The seated pilot's window panes in world space for the lens pass: the drops are laid in each
/// pane's own plane, measured from `origin` (the interior's), so they stay on the glass as the head
/// turns. No triangles on foot.
#[derive(Clone, Copy, Debug)]
struct PaneGlass {
    origin: Vec3,
    count: usize,
    corners: [Vec4; 3 * MAX_PANE_TRIANGLES],
}

impl Default for PaneGlass {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            count: 0,
            corners: [Vec4::ZERO; 3 * MAX_PANE_TRIANGLES],
        }
    }
}

impl PaneGlass {
    /// The panes of `glass` (in its interior's space) carried to the world by `to_world`.
    fn new(glass: &crate::mech::CabinGlass, to_world: &GlobalTransform) -> Self {
        let mut out = Self {
            origin: to_world.translation(),
            ..default()
        };
        for tri in glass.0.iter().take(MAX_PANE_TRIANGLES) {
            for (k, v) in tri.iter().enumerate() {
                out.corners[out.count * 3 + k] = to_world.transform_point(*v).extend(1.0);
            }
            out.count += 1;
        }
        out
    }
}

/// Ground fog on (false under `CB_NO_GROUND_FOG=1`). The clouds read it too: with the fog off, no
/// cloud takes the fog's colour.
#[derive(Resource, Clone, Copy)]
pub(crate) struct GroundFogOn(pub(crate) bool);

/// The air's fog along a sight line now, the numbers the fog pass and the clouds both read (one
/// source, CLAUDE.md section 6.1): the ground fog's layer at the strength now (the clock's morning
/// mist or a mission's preset, survey F1), the aerial haze, and the rain at the eye (followed,
/// `EyeWeather`) as far as the rain reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AirFog {
    /// The ground fog's density at sea level, per metre, times the strength now.
    pub layer_per_m: f32,
    /// The ground fog's scale height, metres.
    pub layer_height_m: f32,
    /// Sea level, metres.
    pub sea_level_m: f32,
    /// The rain's extinction at the eye, per metre.
    pub rain_per_m: f32,
    /// How far along a line the rain hazes it, metres.
    pub rain_reach_m: f32,
    /// The aerial haze's density at sea level, per metre.
    pub haze_per_m: f32,
    /// The aerial haze's scale height, metres.
    pub haze_height_m: f32,
}

impl AirFog {
    /// The air now, from the fog's data, its strength now, sea level and the rain at the eye (0..1).
    pub(crate) fn now(
        cfg: &cb_core::world::mist::FogConfig,
        strength: cb_core::world::mist::FogStrength,
        sea_level_m: f32,
        rain: f32,
    ) -> Self {
        Self {
            layer_per_m: cfg.density_per_m * strength.density_scale,
            layer_height_m: cfg.scale_height_m,
            sea_level_m,
            rain_per_m: cfg.rain_per_m * rain,
            rain_reach_m: cfg.rain_reach_m,
            haze_per_m: cfg.haze_density_per_m,
            haze_height_m: cfg.haze_scale_height_m,
        }
    }
}

/// The mist map's texture (`cb_core::world::mist::MistMap`), uploaded once.
#[derive(Resource)]
struct MistImage(Handle<Image>);

/// The mist map as an R8 texture, one texel a cell.
fn upload_mist(mut commands: Commands, plan: Res<Plan>, mut images: ResMut<Assets<Image>>) {
    use bevy::render::render_resource::{Extent3d, TextureDimension};
    let m = &plan.mist;
    let side = m.side as u32;
    let texels = m.mist.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).collect();
    let image = Image::new(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texels,
        TextureFormat::R8Unorm,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    commands.insert_resource(MistImage(images.add(image)));
}

/// The fog's numbers for the compose pass, this frame.
#[derive(Clone, Copy, Default)]
struct FogLook {
    /// x density at sea level per m, y scale height m, z sea level m, w 1 on (0 off).
    layer: Vec4,
    /// x mist strength, y mist taps' reach m, z a sky ray's length m, w extra extinction per m (rain).
    mist: Vec4,
    /// rgb the sky's light the fog scatters back; w the phase asymmetry.
    ambient: Vec4,
    /// rgb the sun's light the fog scatters (storm shade in it); w the haze's density at sea level per m.
    sun: Vec4,
    /// xyz toward the sun; w the haze's scale height m.
    sun_dir: Vec4,
    /// x the map's least corner (x and z) m, y its cell m, z cells a side, w the rain's reach m.
    map: Vec4,
    /// x the view's exposure (the scattering sky's colour is drawn times it), y how much darker the
    /// fog is under the clouds' shade now (`storm_dim` times the shade), zw unused.
    sky: Vec4,
    /// The night sky (`day_night`): x the stars' brightness on screen now, y the moon's lit face on
    /// screen now, z the sky's brightness now (1 by day, `night_sky_dim` once the stars are out), w the share of the
    /// sky's cells holding a star.
    night: Vec4,
    /// xyz toward the moon; w its disc's angular radius, radians.
    moon: Vec4,
    /// xyz toward the sun itself (`sun_dir` is the key light's, the moon's at night); w unused.
    sun_body: Vec4,
    /// xy the distance (m) over which the glow toward the sun comes into the fog's colour (`FogTint::glow`); zw unused.
    glow: Vec4,
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
    lens_pane_m: f32,
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
            lens_pane_m: look.lens_pane_m,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_water_eye(
    mut commands: Commands,
    data: Extract<Res<Data>>,
    time: Extract<Res<Time>>,
    plan: Extract<Res<Plan>>,
    fog_on: Extract<Res<GroundFogOn>>,
    fog_now: Extract<Res<crate::ground_fog::FogNow>>,
    weather: Extract<Res<crate::ground_fog::EyeWeather>>,
    day_light: Extract<Res<crate::day_night::Light>>,
    mist: Extract<Option<Res<MistImage>>>,
    ambient: Extract<Res<GlobalAmbientLight>>,
    sun: Extract<Query<(&DirectionalLight, &GlobalTransform, &crate::day_night::SkyBody)>>,
    cameras: Extract<Query<(RenderEntity, &WaterEye, &GlobalTransform, Option<&Exposure>), With<WaterOptics>>>,
    cabins: Extract<Query<(&crate::mech::CabinGlass, &GlobalTransform)>>,
) {
    let look = PostLook::from(&data.0.water_look);
    let cfg = &data.0.world.fog;
    let dn = &data.0.day_night;
    let hour = day_light.0;
    let m = &plan.mist;
    for (entity, eye, transform, exposure) in &cameras {
        let exposure = exposure.copied().unwrap_or_default().exposure();
        let key = crate::day_night::key_light(sun.iter());
        let light = water_light(key.as_ref().map(|(l, t)| (l, t)), &ambient, exposure);
        let at = transform.translation();
        let t = time.elapsed_secs();
        let a = LinearRgba::from(ambient.color);
        let sky = Vec3::new(a.red, a.green, a.blue) * ambient.brightness * exposure;
        // The weather at the eye as followed over a second and a half, never a raw sample (`EyeWeather`).
        let (shade, rain) = (weather.sun, weather.rain);
        let air = AirFog::now(cfg, fog_now.strength, plan.waters.sea_level_m, rain);
        let tint = FogTint::now(cfg, &light, sky, shade, exposure);
        let fog = FogLook {
            // Scaled by the strength now: the clock's morning mist or a mission's preset (survey F1).
            layer: Vec4::new(
                air.layer_per_m,
                air.layer_height_m,
                air.sea_level_m,
                if fog_on.0 { 1.0 } else { 0.0 },
            ),
            mist: Vec4::new(
                cfg.mist_strength * fog_now.strength.mist_scale,
                cfg.mist_taps_m,
                cfg.sky_length_m,
                air.rain_per_m,
            ),
            ambient: tint.ambient,
            sun: tint.sun.extend(air.haze_per_m),
            sun_dir: tint.sun_dir.extend(air.haze_height_m),
            map: Vec4::new(m.origin_m, m.cell_m, m.side as f32, air.rain_reach_m),
            sky: Vec4::new(tint.exposure, tint.storm, 0.0, 0.0),
            night: Vec4::new(
                dn.star_brightness * hour.stars,
                // Pale by day, as the moon is in a blue sky.
                dn.moon_disc_brightness * (0.25 + 0.75 * (1.0 - hour.day)),
                // The sky's brightness now: its own by day, `night_sky_dim` of it once the stars are out.
                1.0 - (1.0 - dn.night_sky_dim) * hour.stars,
                dn.star_share,
            ),
            moon: hour.moon_dir.extend((dn.moon_disc_deg * 0.5).to_radians()),
            sun_body: hour.sun_dir.extend(0.0),
            glow: tint.glow.extend(0.0).extend(0.0),
        };
        // Seated: the panes of the interior the eye sits in (the nearest, within a cabin's reach).
        let glass = if eye.cabin_m > 0.0 {
            cabins
                .iter()
                .map(|(g, t)| (t.translation().distance(at), g, t))
                .filter(|(d, ..)| *d < 30.0)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, g, t)| PaneGlass::new(g, t))
                .unwrap_or_default()
        } else {
            PaneGlass::default()
        };
        if let Ok(mut e) = commands.get_entity(entity) {
            e.insert(WaterPostSettings {
                eye: *eye,
                seconds: t,
                look,
                light: light.body,
                fog,
                mist: mist.as_ref().map_or(AssetId::default(), |m| m.0.id()),
                glass,
            });
        }
    }
}

/// `WaterPost` in `water_post.wesl`, field for field.
#[derive(Clone, Copy, Debug, ShaderType)]
struct WaterPostUniform {
    world_from_clip: Mat4,
    camera: Vec4,
    surface: Vec4,
    absorption: Vec4,
    deep: Vec4,
    light: Vec4,
    lens: Vec4,
    lens_fx: Vec4,
    fog_layer: Vec4,
    fog_mist: Vec4,
    fog_ambient: Vec4,
    fog_sun: Vec4,
    fog_sun_dir: Vec4,
    fog_map: Vec4,
    fog_sky_x: Vec4,
    fog_sky_y: Vec4,
    fog_sky_z: Vec4,
    fog_sky_lut: Vec4,
    fog_night: Vec4,
    fog_moon: Vec4,
    fog_sun_body: Vec4,
    fog_glow: Vec4,
    glass: Vec4,
    glass_look: Vec4,
    panes: [Vec4; 3 * MAX_PANE_TRIANGLES],
}

/// Which fullscreen pass.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum PostPass {
    Fog,
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
        "water post: uniform, scene, depth, sampler, mist map, sky-view LUT",
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
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
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
            PostPass::Fog => ("water fog: the ground fog, before the transparent pass", "fog"),
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
    fog: CachedRenderPipelineId,
    compose: CachedRenderPipelineId,
    lens: CachedRenderPipelineId,
    multisampled: bool,
    run: Passes,
    /// The mist map's texture.
    mist: AssetId<Image>,
    /// The scattering sky's sky-view LUT this frame, if the view has the sky.
    sky_lut: Option<TextureView>,
}

/// Which passes a frame needs: the fog while the ground fog is on and the eye is not under water
/// (there the compose pass's murk is the fog), compose while the eye is wet or the glass is (the wet
/// blur), the lens while drops run. Dry with no fog, none runs and the water costs nothing here.
fn passes_needed(eye: &WaterEye, rain: f32, fog: bool) -> Passes {
    let glass = eye.drips > 0.001 || rain > 0.001;
    Passes {
        fog: fog && !eye.under_water(),
        compose: eye.submersion > 0.25 || glass,
        lens: glass && !eye.under_water(),
    }
}

/// The water passes a frame runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Passes {
    fog: bool,
    compose: bool,
    lens: bool,
}

fn prepare_water_post(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<WaterPostPipeline>,
    mut specialized: ResMut<SpecializedRenderPipelines<WaterPostPipeline>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &Msaa,
        &WaterPostSettings,
        Option<&mut WaterPostGpu>,
        Option<SkyAtmosphere>,
    )>,
) {
    for (entity, view, msaa, settings, gpu, atmosphere) in &mut views {
        let clip_from_world = view
            .clip_from_world
            .unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
        let eye = settings.eye;
        let look = settings.look;
        let sky = SkyFrame::of(atmosphere, view.world_from_view.translation());
        // `CB_FOG_TRACE=1`: every frame, every input to the fog's colour, for the one-frame flicker
        // (docs/playtests/2026-10-27-sky-flicker.md).
        if crate::cli::switched_off("CB_FOG_TRACE") {
            let at = view.world_from_view.translation();
            info!(
                "FOG_TRACE view {entity:?} eye ({:.1}, {:.1}, {:.1}) exposure {:.5} storm {:.4} r {:.3} inner {:.3} up ({:.5}, {:.5}, {:.5}) back ({:.5}, {:.5}, {:.5}) lut {}x{} view {:?} fog_on {}",
                at.x,
                at.y,
                at.z,
                settings.fog.sky.x,
                settings.fog.sky.y,
                sky.x.w,
                sky.y.w,
                sky.y.x,
                sky.y.y,
                sky.y.z,
                sky.z.x,
                sky.z.y,
                sky.z.z,
                sky.lut.x,
                sky.lut.y,
                sky.lut_view.as_ref().map(|v| v.id()),
                settings.fog.layer.w
            );
        }
        let uniform = WaterPostUniform {
            world_from_clip: clip_from_world.inverse(),
            camera: view.world_from_view.translation().extend(settings.seconds),
            surface: Vec4::new(eye.surface_m, eye.depth_m.unwrap_or(-1.0e3), look.band_m, eye.submersion),
            absorption: look.absorption.extend(look.light_falloff),
            deep: look.deep.extend(look.distortion),
            light: settings.light.extend(look.wet_blur),
            lens: look.lens,
            lens_fx: Vec4::new(eye.drips, look.lens_rain.max(eye.rain), look.lens_scale, eye.cabin_m),
            fog_layer: settings.fog.layer,
            fog_mist: settings.fog.mist,
            fog_ambient: settings.fog.ambient,
            fog_sun: settings.fog.sun,
            fog_sun_dir: settings.fog.sun_dir,
            fog_map: settings.fog.map,
            fog_sky_x: sky.x,
            fog_sky_y: sky.y,
            fog_sky_z: sky.z,
            fog_sky_lut: sky.lut.extend(settings.fog.sky.x).extend(settings.fog.sky.y),
            fog_night: settings.fog.night,
            fog_moon: settings.fog.moon,
            fog_sun_body: settings.fog.sun_body,
            fog_glow: settings.fog.glow,
            glass: Vec4::new(
                settings.glass.origin.x,
                settings.glass.origin.y,
                settings.glass.origin.z,
                settings.glass.count as f32,
            ),
            glass_look: Vec4::new(look.lens_pane_m, 0.0, 0.0, 0.0),
            panes: settings.glass.corners,
        };
        let run = passes_needed(&eye, look.lens_rain.max(eye.rain), settings.fog.layer.w > 0.5);
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
        let (fog, compose, lens) = (id(PostPass::Fog), id(PostPass::Compose), id(PostPass::Lens));
        if let Some(mut gpu) = gpu {
            gpu.uniform.set(uniform);
            gpu.uniform.write_buffer(&device, &queue);
            gpu.fog = fog;
            gpu.compose = compose;
            gpu.lens = lens;
            gpu.multisampled = multisampled;
            gpu.run = run;
            gpu.mist = settings.mist;
            gpu.sky_lut = sky.lut_view;
            continue;
        }
        let mut buffer = UniformBuffer::from(uniform);
        buffer.write_buffer(&device, &queue);
        commands.entity(entity).insert(WaterPostGpu {
            uniform: buffer,
            fog,
            compose,
            lens,
            multisampled,
            run,
            mist: settings.mist,
            sky_lut: sky.lut_view,
        });
    }
}

/// The scattering sky a view has, if any: its planet, its LUT sizes and its LUTs (Bevy's atmosphere).
type SkyAtmosphere = (
    &'static bevy::pbr::resources::GpuAtmosphere,
    &'static bevy::pbr::GpuAtmosphereSettings,
    &'static bevy::pbr::resources::AtmosphereTextures,
);

/// How the fog pass reads the scattering sky's sky-view LUT for a view: the atmosphere's frame
/// round the eye (`SkyAxes`) and the LUT itself.
struct SkyFrame {
    x: Vec4,
    y: Vec4,
    z: Vec4,
    lut: Vec2,
    lut_view: Option<TextureView>,
}

impl SkyFrame {
    fn of(
        atmosphere: Option<(
            &bevy::pbr::resources::GpuAtmosphere,
            &bevy::pbr::GpuAtmosphereSettings,
            &bevy::pbr::resources::AtmosphereTextures,
        )>,
        eye: Vec3,
    ) -> Self {
        let Some((planet, settings, textures)) = atmosphere else {
            let a = SkyAxes::NONE;
            return Self {
                x: a.x,
                y: a.y,
                z: a.z,
                lut: a.lut,
                lut_view: None,
            };
        };
        let a = SkyAxes::at(
            planet.world_to_atmosphere,
            planet.inner_radius,
            settings.sky_view_lut_size.as_vec2(),
            eye,
        );
        Self {
            x: a.x,
            y: a.y,
            z: a.z,
            lut: a.lut,
            lut_view: Some(textures.sky_view_lut.default_view.clone()),
        }
    }
}

/// The scattering sky's frame round the eye, as the fog's colour reads the sky-view LUT (`FogColour` in
/// `fog_colour.wesl`): the atmosphere's axes x, y and z (Bevy's `prepare_atmosphere_transforms`), the eye's distance
/// from the planet's centre and the planet's radius in their w, `z.w` 1 with the sky (0 without, `CB_NO_SKY=1`, when
/// the fog keeps its own colour), and the LUT's size. The one copy: the fog pass (render world) and the clouds (main
/// world) both build theirs here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkyAxes {
    pub x: Vec4,
    pub y: Vec4,
    pub z: Vec4,
    pub lut: Vec2,
}

impl SkyAxes {
    /// No scattering sky: the fog's own colour.
    pub(crate) const NONE: Self = Self {
        x: Vec4::X,
        y: Vec4::Y,
        z: Vec4::Z,
        lut: Vec2::ONE,
    };

    /// The frame for an eye at `eye` under the atmosphere `world_to_atmosphere` of radius `inner_radius`, its sky-view LUT
    /// `lut_size` texels.
    pub(crate) fn at(world_to_atmosphere: Mat4, inner_radius: f32, lut_size: Vec2, eye: Vec3) -> Self {
        // As Bevy builds it: up is the planet's normal under the eye, back the world's -z made level.
        let at = world_to_atmosphere.transform_point3(eye);
        let up = at.try_normalize().unwrap_or(Vec3::Y);
        let back = (Vec3::NEG_Z - up * up.dot(Vec3::NEG_Z)).try_normalize().unwrap_or(Vec3::NEG_Y);
        let right = up.cross(back).normalize();
        // Bevy clamps the eye to the ground before it reads the LUT.
        let r = at.length().max(inner_radius + 1.0e-3);
        Self {
            x: right.extend(r),
            y: up.extend(inner_radius),
            z: back.extend(1.0),
            lut: lut_size,
        }
    }

    /// An atmosphere entity's world-to-atmosphere transform in the main world, as Bevy's `extract_atmosphere` builds it
    /// for the render world (its scale and translation; atmospheres ignore their orientation).
    pub(crate) fn world_to_atmosphere(transform: &GlobalTransform) -> Mat4 {
        let (scale, _, translation) = transform.to_scale_rotation_translation();
        Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, translation).inverse()
    }
}

/// The light the fog's colour is made of, for the fog pass and the clouds alike (one source, CLAUDE.md section 6.1):
/// its own colour's ambient and forward-scattered sun (used without the scattering sky), and with the sky the view's
/// exposure and how much darker it is under the clouds' shade (`storm_dim` times the shade).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FogTint {
    /// rgb the sky's light the fog scatters back; w the phase asymmetry g.
    pub ambient: Vec4,
    /// The sun's light the fog scatters, the storm's shade in it.
    pub sun: Vec3,
    /// Toward the sun.
    pub sun_dir: Vec3,
    /// The view's exposure (the sky's LUT is drawn times it).
    pub exposure: f32,
    /// How much darker the fog is under the clouds' shade now.
    pub storm: f32,
    /// The distance (m) over which the glow toward the sun comes into the fog's colour (`FogConfig::sun_glow_m`).
    pub glow: Vec2,
}

impl FogTint {
    /// The tint now: `light` the scene's (`water_light`), `sky` the ambient light times exposure, `shade` the sun's share
    /// through the clouds over the eye (followed, `EyeWeather`).
    pub(crate) fn now(cfg: &cb_core::world::mist::FogConfig, light: &WaterLight, sky: Vec3, shade: f32, exposure: f32) -> Self {
        Self {
            ambient: (sky * cfg.ambient_share).extend(cfg.phase_g),
            sun: light.sun * shade * cfg.sun_share,
            sun_dir: light.sun_dir,
            exposure,
            storm: cfg.storm_dim * (1.0 - shade.clamp(0.0, 1.0)),
            glow: Vec2::from(cfg.sun_glow_m),
        }
    }
}

/// Runs the compose and lens passes for the current view, each reading the main texture and
/// writing the other (Bevy's post-process swap).
fn water_post(
    view: ViewQuery<(&ViewTarget, &ViewDepthStencilTexture, &WaterPostGpu)>,
    pipeline: Res<WaterPostPipeline>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    no_mist: Res<FallbackImageZero>,
    ctx: RenderContext,
) {
    let (target, depth, gpu) = view.into_inner();
    let passes = [
        (gpu.run.compose, gpu.compose, "water_compose"),
        (gpu.run.lens, gpu.lens, "water_lens"),
    ];
    draw_passes(target, depth, gpu, &passes, &pipeline, &cache, &images, &no_mist, ctx);
}

/// Orders the ground fog pass after Bevy's sky pass. Bevy runs `render_sky` after the opaque pass and before the
/// transparent one, and the fog after the transmissive pass and before the transparent one, with nothing between the
/// two: the scheduler could run the fog first on any frame, and that frame's fog came out darker and bluer over the
/// whole sky (owner, 2026-10-27: the sky flickering for one frame "when flying from one location to another"; measured
/// in docs/playtests/2026-10-27-sky-flicker.md: 50 such frames in 4.4 min with only the fog on, none with the fog
/// forced after the sky). `render_sky` is private to Bevy, so the edge is added by name while the schedule builds.
#[derive(Debug)]
struct FogAfterSky;

impl bevy::ecs::schedule::ScheduleBuildPass for FogAfterSky {
    type EdgeOptions = ();

    fn add_dependency(&mut self, _from: bevy::ecs::schedule::NodeId, _to: bevy::ecs::schedule::NodeId, _options: Option<&()>) {}

    fn collapse_set(
        &mut self,
        _set: bevy::ecs::schedule::SystemSetKey,
        _systems: &indexmap::IndexSet<bevy::ecs::schedule::SystemKey, bevy::platform::hash::FixedHasher>,
        _dependency_flattening: &bevy::ecs::schedule::graph::DiGraph<bevy::ecs::schedule::NodeId>,
    ) -> impl Iterator<Item = (bevy::ecs::schedule::NodeId, bevy::ecs::schedule::NodeId)> {
        std::iter::empty()
    }

    fn build(
        &mut self,
        _world: &mut World,
        graph: &mut bevy::ecs::schedule::ScheduleGraph,
        mut dependencies: bevy::ecs::schedule::FlattenedDependencies<'_>,
    ) -> Result<(), bevy::ecs::schedule::ScheduleBuildError> {
        let find = |suffix: &str| {
            graph
                .systems
                .iter()
                .find(|(_, system, _)| system.name().to_string().ends_with(suffix))
                .map(|(key, ..)| key)
        };
        match (find("::render_sky"), find("::water_fog")) {
            (Some(sky), Some(fog)) => {
                dependencies.add_edge(sky, fog);
                info!("ground fog: ordered after Bevy's sky pass");
            }
            (sky, fog) => warn!(
                "ground fog: not ordered after the sky pass (sky pass found: {}, fog pass found: {}); the sky can flicker",
                sky.is_some(),
                fog.is_some()
            ),
        }
        Ok(())
    }
}

/// Runs the ground fog pass for the current view, after the opaque and transmissive passes and Bevy's sky
/// (`FogAfterSky`), and before the transparent one (`fog` in `water_post.wesl`).
fn water_fog(
    view: ViewQuery<(&ViewTarget, &ViewDepthStencilTexture, &WaterPostGpu)>,
    pipeline: Res<WaterPostPipeline>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    no_mist: Res<FallbackImageZero>,
    ctx: RenderContext,
) {
    let (target, depth, gpu) = view.into_inner();
    draw_passes(
        target,
        depth,
        gpu,
        &[(gpu.run.fog, gpu.fog, "water_fog")],
        &pipeline,
        &cache,
        &images,
        &no_mist,
        ctx,
    );
}

/// Draws each pass that runs, in order, each a fullscreen triangle reading the main texture (and the
/// depth and the mist map) and writing the other one.
#[allow(clippy::too_many_arguments)]
fn draw_passes(
    target: &ViewTarget,
    depth: &ViewDepthStencilTexture,
    gpu: &WaterPostGpu,
    passes: &[(bool, CachedRenderPipelineId, &str)],
    pipeline: &WaterPostPipeline,
    cache: &PipelineCache,
    images: &RenderAssets<GpuImage>,
    no_mist: &FallbackImageZero,
    mut ctx: RenderContext,
) {
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        return;
    };
    let Some(uniform) = gpu.uniform.binding() else {
        return;
    };
    // The mist map, or no mist until it is on the GPU.
    let mist = images.get(gpu.mist).map_or(&no_mist.texture_view, |i| &i.texture_view);
    // The sky-view LUT, or a stand-in the shader never reads (no sky: `fog_sky_z.w` is 0).
    let sky_lut = gpu.sky_lut.as_ref().unwrap_or(&no_mist.texture_view);
    let layout = cache.get_bind_group_layout(if gpu.multisampled {
        &pipeline.layout_multisampled
    } else {
        &pipeline.layout
    });
    for &(run, id, label) in passes {
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
            &BindGroupEntries::sequential((uniform.clone(), post.source, depth_view, &pipeline.sampler, mist, sky_lut)),
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
