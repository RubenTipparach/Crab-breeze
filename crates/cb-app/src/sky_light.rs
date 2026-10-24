//! The sky's light on the world (openspec change `glass-reflections`; owner, 2026-10-24: "that
//! glass needs better reflections/materials to be reflective, cache some cube maps?" and
//! https://polyhaven.com/hdris). A cube map baked once from a Poly Haven sky
//! (`tools/textures/gen_sky_cube.py`, `assets/textures/sky/sky_cube.hdr`) is loaded at startup and
//! filtered on the GPU by Bevy's `GeneratedEnvironmentMapLight` on the pilot's eye camera: every
//! PBR surface in the world, the buildings' glass above all, reflects that sky, and it fills the
//! shade with sky light.
//!
//! It is filtered once and cached. Bevy's generator re-filters its cube every frame it is present
//! (measured on lavapipe: 214 ms radiance and 50 ms irradiance a frame), so once the render world
//! has seen `FILTERED_FRAMES` frames with the generator's pipelines compiled and the map queued,
//! the generator is taken off and the filtered `EnvironmentMapLight` it made stays. Counting plain
//! frames is not enough: on a slow renderer the pipelines are still compiling after dozens of
//! frames, and the cache was black. The loading screen waits for it.
//!
//! The tuning is `cb_core::sky_light`, from `data/world/sky_light.ron`; the glass material is made
//! here so every pane in the game is the one material.

use crate::pilot::PilotCamera;
use crate::sim::Data;
use bevy::light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy::pbr::generate::{GeneratorPipelines, RenderEnvironmentMap};
use bevy::prelude::*;
use bevy::render::render_resource::{PipelineCache, TextureViewDescriptor, TextureViewDimension};
use bevy::render::{Render, RenderApp, RenderSystems};
use cb_core::sky_light::{GlassLook, SkyLightConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Frames filtered with every pipeline ready before the result is kept (one is enough in
/// principle; three leave room for a frame whose bind groups were not ready yet).
const FILTERED_FRAMES: u32 = 3;

/// The sky cube while it loads and filters, and whether it is cached on the eye camera yet.
#[derive(Resource, Default)]
pub struct SkyLight {
    cube: Option<Handle<Image>>,
    /// The generator is on the camera.
    filtering: bool,
    /// True once the sky light is cached on the eye camera, or there is none to wait for.
    pub ready: bool,
}

/// Frames the render world has filtered the sky with its pipelines ready (shared with it).
#[derive(Resource, Clone, Default)]
struct FilteredFrames(Arc<AtomicU32>);

/// Loads the sky cube and lights the world with it.
pub struct SkyLightPlugin;

impl Plugin for SkyLightPlugin {
    fn build(&self, app: &mut App) {
        let filtered = FilteredFrames::default();
        app.init_resource::<SkyLight>()
            .insert_resource(filtered.clone())
            .add_systems(Startup, load)
            .add_systems(Update, apply);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(filtered)
                .add_systems(Render, count_filtered_frames.in_set(RenderSystems::Cleanup));
        }
    }
}

/// The flat ambient the world keeps: low under the sky light, the old flat fill without one.
pub fn ambient_cd_m2(cfg: &SkyLightConfig) -> f32 {
    if cfg.cube.is_empty() {
        cfg.ambient_without_sky_cd_m2
    } else {
        cfg.ambient_cd_m2
    }
}

/// Window glass: premultiplied alpha, so what it reflects is added in full and what is behind it
/// is dimmed by `alpha` only (plain alpha blending would scale the reflection down by alpha too).
pub fn glass_material(look: &GlassLook) -> StandardMaterial {
    let [r, g, b] = look.tint;
    StandardMaterial {
        base_color: Color::linear_rgba(r, g, b, look.alpha),
        alpha_mode: AlphaMode::Premultiplied,
        perceptual_roughness: look.roughness,
        metallic: 0.0,
        reflectance: look.reflectance,
        ..default()
    }
}

fn load(mut sky: ResMut<SkyLight>, data: Res<Data>, assets: Res<AssetServer>) {
    let cfg = &data.0.sky_light;
    if cfg.cube.is_empty() {
        sky.ready = true;
        return;
    }
    sky.cube = Some(assets.load(cfg.cube.clone()));
}

/// Once the stacked image is in, turns it into a cube and gives it to the eye camera to filter;
/// once the render world has filtered it, takes the generator off and keeps what it filtered.
fn apply(
    mut commands: Commands,
    mut sky: ResMut<SkyLight>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    data: Res<Data>,
    filtered: Res<FilteredFrames>,
    camera: Option<Single<(Entity, Has<EnvironmentMapLight>), With<PilotCamera>>>,
) {
    let (false, Some(handle), Some(camera)) = (sky.ready, sky.cube.clone(), camera) else {
        return;
    };
    let (camera, has_output) = *camera;
    if sky.filtering {
        if has_output && filtered.0.load(Ordering::Relaxed) >= FILTERED_FRAMES {
            commands.entity(camera).remove::<GeneratedEnvironmentMapLight>();
            info!("sky light: filtered and cached");
            sky.ready = true;
        }
        return;
    }
    if let Some(bevy::asset::LoadState::Failed(e)) = assets.get_load_state(&handle) {
        error!("sky light: {e}; the world keeps its flat ambient only");
        sky.ready = true;
        return;
    }
    let Some(mut image) = images.get_mut(&handle) else {
        return;
    };
    if let Err(e) = image.reinterpret_stacked_2d_as_array(6) {
        error!("sky light: {} is not six stacked faces: {e:?}", data.0.sky_light.cube);
        sky.ready = true;
        return;
    }
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    let cfg = &data.0.sky_light;
    commands.entity(camera).insert(GeneratedEnvironmentMapLight {
        environment_map: handle,
        intensity: cfg.sky_cd_m2,
        rotation: Quat::from_rotation_y(cfg.yaw_deg.to_radians()),
        affects_lightmapped_mesh_diffuse: true,
    });
    info!("sky light: {} on the eye camera at {} cd/m2 a unit", cfg.cube, cfg.sky_cd_m2);
    sky.filtering = true;
}

/// Counts the frames on which the generator could run: a map queued and all five of its compute
/// pipelines compiled.
fn count_filtered_frames(
    filtered: Res<FilteredFrames>,
    pipelines: Option<Res<GeneratorPipelines>>,
    cache: Res<PipelineCache>,
    maps: Query<(), With<RenderEnvironmentMap>>,
) {
    let Some(p) = pipelines else { return };
    let ready = [p.downsample_first, p.downsample_second, p.copy, p.radiance, p.irradiance]
        .into_iter()
        .all(|id| cache.get_compute_pipeline(id).is_some());
    if ready && !maps.is_empty() {
        filtered.0.fetch_add(1, Ordering::Relaxed);
    }
}
