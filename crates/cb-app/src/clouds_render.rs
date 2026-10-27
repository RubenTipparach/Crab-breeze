//! The sky's clouds drawn as billboards (openspec change `cloud-weather`; owner, 2026-10-24: "I
//! prefer clouds to be billboards or a mix of some cheap raymarch", "make it look like there's
//! actual weather"), replacing the lumpy blob meshes of `cloud-puffs`.
//!
//! Every puff the weather laid out (`cb_core::world::clouds`) within the draw radius of the eye is
//! one camera-facing quad, every rain shaft three quads turned about the vertical, every cirrus
//! streak one flat quad: all of them items of one mesh, one entity and one material, sorted from
//! the farthest to the nearest so they blend in order, and gathered and sorted again when the eye
//! has moved `resort_m` through the drifting sky. One draw. The wind moves the whole mesh by its
//! transform. The light is the scene's own: the sun's direction and radiance and the ambient light,
//! through the water's light helper, so a cloud sits in the same light as the ground. The shader
//! (`assets/shaders/cloud.wesl`) erodes each puff's edge with noise, lights it as a sphere and
//! fades it where it meets the ground behind it and near the eye (inside, the cloud fog takes over);
//! its colour goes toward the fog's colour (the sky-view LUT, `fog_colour.wesl`) by the air between it and the eye (the
//! ground fog's layer thickened by the mist, and the haze, `AirFog`), so a far one sinks into the horizon's haze. `CB_NO_CLOUDS=1` leaves the sky clear, for the benchmark.
//!
//! The ground under a storm is darkened by a texture on the sun (Bevy's `DirectionalLightTexture`,
//! a light cookie): the core's `Clouds::sun_at` at the cloud base, baked along the sun's rays into
//! one 1024 px map over the weather, which the wind carries by moving the sun's transform. So the
//! ground, the buildings, the trees, the grass and the mechs all darken under a storm, through
//! Bevy's own lighting, with no shader of their own changed.

use crate::pilot::PilotCamera;
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use crate::water_render::{AirFog, FogTint, GroundFogOn, SkyAxes, water_light};
use crate::world::{Plan, Sun};
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::DirectionalLightTexture;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexFormat};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use cb_core::world::Clouds;

const SHADER: &str = "shaders/cloud.wesl";

/// A puff: rx across, ry up, seed, unused. A shaft: radius, height, seed, unused. Cirrus: length,
/// width, seed, heading.
const ATTRIBUTE_SHAPE: MeshVertexAttribute = MeshVertexAttribute::new("CloudShape", 0x0C10_D5A1, VertexFormat::Float32x4);
/// Kind, darkness, opacity, unused.
const ATTRIBUTE_LOOK: MeshVertexAttribute = MeshVertexAttribute::new("CloudLook", 0x0C10_D5A2, VertexFormat::Float32x4);
/// The base and top of the item's system.
const ATTRIBUTE_LAYER: MeshVertexAttribute = MeshVertexAttribute::new("CloudLayer", 0x0C10_D5A3, VertexFormat::Float32x2);

/// The shader's kind number for a rain shaft and a cirrus streak (after the puff kinds).
const KIND_SHAFT: f32 = 7.0;
const KIND_CIRRUS: f32 = 8.0;

/// The cloud material's uniform; `CloudLight` in `cloud.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct CloudUniform {
    /// xyz toward the sun; w how far the sun wraps round a puff (0 hard, 1 soft).
    pub sun_dir: Vec4,
    /// rgb the sun's light times exposure; w the silver lining's strength.
    pub sun: Vec4,
    /// rgb the ambient light times exposure; w light scattered inside a puff, a share of the sun.
    pub ambient: Vec4,
    /// x white albedo, y storm albedo, z white base shade, w storm base shade.
    pub albedo: Vec4,
    /// x fade-in distance from the eye (m), y edge erosion, z soft depth against the ground (m), w the shadow floor
    /// (`look_shadow_floor`).
    pub fade: Vec4,
    /// The ground fog: x density at sea level per m (its strength now in it), y scale height m, z sea level m, w unused.
    /// Zero with the ground fog off.
    pub fog: Vec4,
    /// The aerial haze: x density at sea level per m, y scale height m; zw unused. Zero with the ground fog off.
    pub haze: Vec4,
    /// How many times the mist thickens the ground fog's layer toward 16 level bearings round the eye, four to a vec4
    /// (`thick_round`).
    pub thick: [Vec4; 4],
    /// The scattering sky's frame round the eye and the LUT's size (`SkyAxes`, as the fog pass has them).
    pub sky_x: Vec4,
    pub sky_y: Vec4,
    pub sky_z: Vec4,
    /// xy the sky-view LUT's size, z the view's exposure, w how much darker the fog is under the clouds' shade.
    pub sky_lut: Vec4,
    /// The fog's own colour without the sky (`FogTint`): rgb the ambient scattered back, w the phase asymmetry.
    pub fog_ambient: Vec4,
    /// rgb the sun the fog scatters forward; w unused.
    pub fog_sun: Vec4,
    /// xyz toward the sun; w unused.
    pub fog_sun_dir: Vec4,
}

impl CloudUniform {
    /// The air between the eye and a cloud, as the fog pass fogs the sky behind it (`AirFog`), so a far cloud takes
    /// the horizon's colour (owner, 2026-10-26: "make sure distanrt clouds blends with fog too"); nothing with the ground
    /// fog off. The ground fog's layer and the haze only: the rain's haze, 1.5 km deep, would fade the storm's own base
    /// overhead onto the clear sky drawn behind it (`air_between` in `cloud.wesl` says why).
    pub(crate) fn air(air: &AirFog, on: bool) -> (Vec4, Vec4) {
        if !on {
            return (Vec4::ZERO, Vec4::ZERO);
        }
        (
            Vec4::new(air.layer_per_m, air.layer_height_m, air.sea_level_m, 0.0),
            Vec4::new(air.haze_per_m, air.haze_height_m, 0.0, 0.0),
        )
    }
}

/// The one cloud material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CloudMaterial {
    /// The light and look, rewritten each frame.
    #[uniform(0)]
    pub light: CloudUniform,
    /// Bevy's sky-view LUT for the pilot's view, the fog's colour (`mirror_sky_lut`): a 1 x 1 stand-in until the sky is
    /// drawn, never read without it (`sky_z.w` 0).
    #[texture(1)]
    #[sampler(2)]
    pub sky_lut: Handle<Image>,
}

/// The image the cloud material reads the sky-view LUT through; in the render world its texture is the pilot view's
/// LUT each frame (`mirror_sky_lut`).
#[derive(Resource, Clone)]
struct CloudSkyLut(Handle<Image>);

/// Points the cloud material's LUT image at the pilot view's sky-view LUT. Bevy's atmosphere makes the LUT a texture of
/// its own each view (no copy source, no asset), so the material's image is re-pointed at it rather than copied: a
/// handle swap, no GPU work. Only the pilot's camera has the scattering sky (`pilot.rs`), so there is one LUT to point
/// at. The material is prepared afresh each frame (its uniform changes), from the LUT the texture cache handed this view
/// last frame, which is the same texture: the cache gives one view's request the same texture frame after frame.
fn mirror_sky_lut(
    lut: Option<Res<CloudSkyLut>>,
    views: Query<&bevy::pbr::resources::AtmosphereTextures>,
    mut images: ResMut<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
) {
    let (Some(lut), Some(textures)) = (lut, views.iter().next()) else {
        return;
    };
    let Some(gpu) = images.get_mut(&lut.0) else {
        return;
    };
    let sky = &textures.sky_view_lut;
    gpu.texture = sky.texture.clone();
    gpu.texture_view = sky.default_view.clone();
    gpu.texture_descriptor.size = sky.texture.size();
    gpu.texture_descriptor.format = sky.texture.format();
}

fn extract_sky_lut(mut commands: Commands, lut: bevy::render::Extract<Option<Res<CloudSkyLut>>>) {
    if let Some(lut) = lut.as_ref() {
        commands.insert_resource(CloudSkyLut(lut.0.clone()));
    }
}

/// How many times the mist thickens the ground fog's layer along 16 level bearings from `eye` (bearing k at k / 16 of a
/// turn from +x toward +z), read as the fog pass reads it along a sight line out to `mist_taps_m`
/// (`cb_core::world::mist::thickening`), four to a vec4. The cloud shader takes the bearing toward each cloud between
/// the two nearest: a far cloud low over misty land is thickened by the same mist as the land under it.
pub(crate) fn thick_round(
    cfg: &cb_core::world::mist::FogConfig,
    strength: cb_core::world::mist::FogStrength,
    map: &cb_core::world::mist::MistMap,
    eye: Vec3,
) -> [Vec4; 4] {
    let mut out = [Vec4::ONE; 4];
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        out[k / 4][k % 4] = cb_core::world::mist::thickening(cfg, strength, map, eye, dir, cfg.mist_taps_m);
    }
    out
}

impl Material for CloudMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    // Behind every other see-through thing (smoke and dust in front of the sky), after the ground
    // marks. The bias only orders the transparent pass.
    fn depth_bias(&self) -> f32 {
        -1.0e8
    }

    // Transparent: no depth prepass and no shadows (it reads the prepass, for its soft edges).
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            ATTRIBUTE_SHAPE.at_shader_location(2),
            ATTRIBUTE_LOOK.at_shader_location(3),
            ATTRIBUTE_LAYER.at_shader_location(4),
        ])?];
        // A cirrus streak is seen from below; a turned quad from either side.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Clouds in the sky.
pub struct CloudsRenderPlugin;

impl Plugin for CloudsRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CloudMaterial>::default())
            .add_world_spawn(setup)
            .add_game_systems(Update, (gather, refresh_light, shade_sun).chain().in_set(PresentSet::Pose));
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.add_systems(bevy::render::ExtractSchedule, extract_sky_lut).add_systems(
                bevy::render::Render,
                mirror_sky_lut.in_set(bevy::render::RenderSystems::PrepareBindGroups),
            );
        }
    }
}

/// The drawn sky.
#[derive(Resource)]
struct CloudDraw {
    material: Handle<CloudMaterial>,
    mesh: Handle<Mesh>,
    entity: Entity,
    /// The eye in the sky's frame (as at time zero) and the view direction when the mesh was last
    /// gathered and sorted.
    sorted_at: Option<Vec3>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mesh = meshes.add(item_mesh(&[]));
    // The LUT's own format, one texel, until `mirror_sky_lut` points it at the sky; sampled linearly, as the fog pass does.
    let mut lut = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    lut.sampler = bevy::image::ImageSampler::linear();
    let sky_lut = images.add(lut);
    commands.insert_resource(CloudSkyLut(sky_lut.clone()));
    let material = materials.add(CloudMaterial {
        light: CloudUniform::default(),
        sky_lut,
    });
    let entity = commands
        .spawn((
            Name::new("clouds"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            NotShadowCaster,
            NoFrustumCulling,
        ))
        .id();
    commands.insert_resource(CloudDraw {
        material,
        mesh,
        entity,
        sorted_at: None,
    });
}

/// One quad to draw, in the sky's frame at time zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// Its centre.
    pub centre: Vec3,
    /// See `ATTRIBUTE_SHAPE`.
    pub shape: Vec4,
    /// Kind, darkness, opacity, unused.
    pub look: Vec4,
    /// The base and top of its system.
    pub layer: Vec2,
}

/// Every item within `radius_m` of `eye` (in the sky's frame at time zero, x and z) in the order
/// they are drawn: the farthest first, by the distance to its near side (its centre's distance less
/// its size, so a big puff beside a small one is ordered by the surface the eye meets). A puff's
/// billboard faces the eye's position (`facing` in `cloud.wesl`), so the order depends on where the
/// eye is and never on where it looks: turning the head cannot re-sort the sky (owner, 2026-10-26:
/// "its causing clouds to popover each other when I turn my camerea"). Puffs and shafts within the
/// radius, every cirrus streak.
pub fn items(clouds: &Clouds, eye: Vec3, radius_m: f32) -> Vec<Item> {
    let flat = |p: Vec3| Vec2::new(p.x, p.z).distance(Vec2::new(eye.x, eye.z));
    let mut out: Vec<Item> = clouds
        .puffs_near(Vec2::new(eye.x, eye.z), radius_m, 0.0)
        .into_iter()
        .map(|i| &clouds.puffs[i as usize])
        .filter(|p| flat(p.centre) <= radius_m)
        .map(|p| Item {
            centre: p.centre,
            shape: Vec4::new(p.radii.x, p.radii.y, p.seed, 0.0),
            look: Vec4::new(p.kind as u8 as f32, p.dark, p.opacity, 0.0),
            layer: Vec2::from(p.layer_m),
        })
        .collect();
    for (k, s) in clouds.shafts.iter().enumerate() {
        // Three sheets a shaft, offset across and along, the middle one widest.
        for j in 0..3 {
            let off = (j as f32 - 1.0) * s.radius_m;
            let at = s.centre + Vec2::new(off * 0.5, off * 0.3);
            let centre = Vec3::new(at.x, s.top_m * 0.5, at.y);
            if flat(centre) > radius_m + s.radius_m {
                continue;
            }
            let r = s.radius_m * if j == 1 { 0.9 } else { 0.6 };
            out.push(Item {
                centre,
                shape: Vec4::new(r, s.top_m, (k * 3 + j) as f32 * 7.3, 0.0),
                look: Vec4::new(KIND_SHAFT, 0.0, 1.0, 0.0),
                layer: Vec2::new(0.0, s.top_m),
            });
        }
    }
    out.extend(clouds.cirrus.iter().map(|c| Item {
        centre: c.centre,
        shape: Vec4::new(c.length_m, c.width_m, c.seed, c.angle_rad),
        look: Vec4::new(KIND_CIRRUS, 0.0, 1.0, 0.0),
        layer: Vec2::new(c.centre.y, c.centre.y),
    }));
    // Farthest first; ties keep the gathering order, so the same place always draws the same way.
    let mut keyed: Vec<(f32, usize)> = out.iter().enumerate().map(|(i, it)| (near_side_m(it, eye), i)).collect();
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    keyed.into_iter().map(|(_, i)| out[i]).collect()
}

/// How far from `eye` an item's near side is: its centre's distance less its size (a puff's larger
/// radius, a shaft's radius; a cirrus streak is a flat sheet, its centre).
fn near_side_m(it: &Item, eye: Vec3) -> f32 {
    let size = if it.look.x > KIND_CIRRUS - 0.5 {
        0.0
    } else if it.look.x > KIND_SHAFT - 0.5 {
        it.shape.x
    } else {
        it.shape.x.max(it.shape.y)
    };
    it.centre.distance(eye) - size
}

/// The quads of `items` as one mesh, four corners and two triangles an item, in order.
fn item_mesh(items: &[Item]) -> Mesh {
    let n = items.len() * 4;
    let (mut centre, mut corner, mut shape, mut look, mut layer) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let mut indices: Vec<u32> = Vec::with_capacity(items.len() * 6);
    for (i, it) in items.iter().enumerate() {
        for c in [[-1.0f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] {
            centre.push(it.centre.to_array());
            corner.push(c);
            shape.push(it.shape.to_array());
            look.push(it.look.to_array());
            layer.push(it.layer.to_array());
        }
        let b = i as u32 * 4;
        indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, centre)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, corner)
        .with_inserted_attribute(ATTRIBUTE_SHAPE, shape)
        .with_inserted_attribute(ATTRIBUTE_LOOK, look)
        .with_inserted_attribute(ATTRIBUTE_LAYER, layer)
        .with_inserted_indices(Indices::U32(indices))
}

/// Carries the sky with the wind, and gathers and sorts the items near the eye again once it has
/// moved `resort_m` through the sky (turning does not change the order). The draw radius grows with the eye's height (twice it), so a high eye sees the weather to the horizon.
fn gather(
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    draw: Option<ResMut<CloudDraw>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(mut draw) = draw else { return };
    let clouds = &plan.clouds;
    let d = clouds.drift(time.elapsed_secs());
    let drift = Vec3::new(d.x, 0.0, d.y);
    if let Ok(mut t) = transforms.get_mut(draw.entity) {
        t.translation = drift;
    }
    let eye = camera.translation() - drift;
    let c = &data.0.world.clouds;
    if draw.sorted_at.is_some_and(|at| at.distance(eye) < c.resort_m) {
        return;
    }
    draw.sorted_at = Some(eye);
    let _s = crate::perf_hud::section("clouds: resort");
    if let Some(mut mesh) = meshes.get_mut(&draw.mesh) {
        let list = items(clouds, eye, c.draw_radius_m + 2.0 * eye.y.max(0.0));
        crate::perf_hud::count("cloud quads", list.len() as u32);
        *mesh = item_mesh(&list);
    }
}

/// The scene's light into the cloud material each frame, and the fog's: the air between the eye and a cloud and the
/// fog's colour, from the same sources the fog pass reads (`AirFog`, `FogTint`, `SkyAxes`).
#[allow(clippy::too_many_arguments)]
fn refresh_light(
    data: Res<Data>,
    plan: Res<Plan>,
    fog_on: Option<Res<GroundFogOn>>,
    fog_now: Res<crate::ground_fog::FogNow>,
    weather: Res<crate::ground_fog::EyeWeather>,
    ambient: Res<GlobalAmbientLight>,
    sun: Query<(&DirectionalLight, &GlobalTransform), With<Sun>>,
    camera: Query<(&GlobalTransform, Option<&Exposure>, Option<&bevy::pbr::AtmosphereSettings>), With<PilotCamera>>,
    atmosphere: Query<(&bevy::light::Atmosphere, &GlobalTransform)>,
    draw: Option<Res<CloudDraw>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
) {
    let Some(draw) = draw else { return };
    let Some((eye, exposure, sky)) = camera.iter().next() else {
        return;
    };
    let eye = eye.translation();
    let exposure = exposure.copied().unwrap_or_default().exposure();
    let light = water_light(sun.iter().next(), &ambient, exposure);
    let a = LinearRgba::from(ambient.color);
    let amb = Vec3::new(a.red, a.green, a.blue) * ambient.brightness * exposure;
    let c = &data.0.world.clouds;
    let cfg = &data.0.world.fog;
    let air = AirFog::now(cfg, fog_now.strength, plan.waters.sea_level_m, 0.0);
    let (fog, haze) = CloudUniform::air(&air, fog_on.is_some_and(|f| f.0));
    let tint = FogTint::now(cfg, &light, amb, weather.sun, exposure);
    // The atmosphere nearest the eye, as Bevy picks it for the view (there is one).
    let axes = match (
        sky,
        atmosphere
            .iter()
            .min_by(|x, y| eye.distance(x.1.translation()).total_cmp(&eye.distance(y.1.translation()))),
    ) {
        (Some(settings), Some((planet, at))) => SkyAxes::at(
            SkyAxes::world_to_atmosphere(at),
            planet.inner_radius,
            settings.sky_view_lut_size.as_vec2(),
            eye,
        ),
        _ => SkyAxes::NONE,
    };
    let thick = thick_round(cfg, fog_now.strength, &plan.mist, eye);
    let Some(mut m) = materials.get_mut(&draw.material) else { return };
    m.light = CloudUniform {
        sun_dir: light.sun_dir.extend(c.look_wrap),
        sun: light.sun.extend(c.look_silver),
        ambient: amb.extend(c.look_scattered),
        albedo: Vec4::new(c.look_albedo, c.look_storm_albedo, c.look_base_shade, c.look_storm_base_shade),
        fade: Vec4::new(c.look_near_fade_m, c.look_erode, c.look_soft_m, c.look_shadow_floor),
        fog,
        haze,
        thick,
        sky_x: axes.x,
        sky_y: axes.y,
        sky_z: axes.z,
        sky_lut: axes.lut.extend(tint.exposure).extend(tint.storm),
        fog_ambient: tint.ambient,
        fog_sun: tint.sun.extend(0.0),
        fog_sun_dir: tint.sun_dir.extend(0.0),
    };
}

/// Texels a side of the storm shade map.
const SHADE_TEXELS: usize = 1024;
/// The map covers this far either side of the island's centre, across the sun's rays (metres).
const SHADE_HALF_M: f32 = 40000.0;
/// The shade is laid at this height: where the sun's ray crosses the storms' base.
const SHADE_AT_M: f32 = 700.0;

/// The storm shade map: the share of the sun reaching the cloud base where each texel's ray
/// crosses it, R8 (255 full sun). The texel at local `(x, y)` (-1..1 across the light) is the ray
/// through `R (x s, y s, 0)`, where `R` turns the light's frame to the world's and `s` is
/// `SHADE_HALF_M`; Bevy samples it at `uv = local.xy * (-0.5, 0.5) + 0.5`.
pub fn shade_map(clouds: &Clouds, rotation: Quat, texels: usize) -> Vec<u8> {
    let travel = rotation * Vec3::NEG_Z;
    let mut out = vec![255u8; texels * texels];
    if travel.y > -0.05 {
        // The sun at the horizon or below: no storm shadow worth drawing.
        return out;
    }
    for j in 0..texels {
        for i in 0..texels {
            let u = (i as f32 + 0.5) / texels as f32;
            let v = (j as f32 + 0.5) / texels as f32;
            let local = Vec3::new((0.5 - u) * 2.0, (v - 0.5) * 2.0, 0.0) * SHADE_HALF_M;
            let w = rotation * local;
            // Along the ray to the cloud base.
            let at = w + travel * ((SHADE_AT_M - w.y) / travel.y);
            out[j * texels + i] = (clouds.sun_at(Vec2::new(at.x, at.z), 0.0) * 255.0).round() as u8;
        }
    }
    out
}

/// The sun's storm shade: baked once for the sun's direction (again if it turns), and carried with
/// the wind by the sun's transform (a directional light's translation and scale light nothing but
/// its texture; its shadows read only its rotation).
fn shade_sun(
    mut commands: Commands,
    plan: Res<Plan>,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
    mut sun: Query<(Entity, &mut Transform, Option<&DirectionalLightTexture>), With<Sun>>,
    mut baked_for: Local<Option<Quat>>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
) {
    let Ok((entity, mut transform, texture)) = sun.single_mut() else {
        return;
    };
    if plan.clouds.shades.is_empty() {
        return;
    }
    let d = plan.clouds.drift(time.elapsed_secs());
    transform.translation = Vec3::new(d.x, 0.0, d.y);
    transform.scale = Vec3::splat(SHADE_HALF_M);
    let turned = baked_for.is_none_or(|q| q.angle_between(transform.rotation) > 0.01);
    if texture.is_some() && !turned {
        return;
    }
    *baked_for = Some(transform.rotation);
    if let Some(device) = device.as_ref() {
        // Bevy draws a light texture only where texture binding arrays work (not on Metal).
        let needs = bevy::render::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY
            | bevy::render::settings::WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
        // ...and binding arrays of at least 24 textures and samplers a stage (Bevy's
        // `binding_arrays_are_usable`: the standard material's 16 plus 8 light probes).
        let limits = device.limits();
        let usable = device.features().contains(needs)
            && limits.max_binding_array_elements_per_shader_stage >= 24
            && limits.max_binding_array_sampler_elements_per_shader_stage >= 24;
        info!(
            "clouds: storm shade on the sun; this device {} light textures (binding arrays: {} textures, {} samplers a stage)",
            if usable { "draws" } else { "does NOT draw" },
            limits.max_binding_array_elements_per_shader_stage,
            limits.max_binding_array_sampler_elements_per_shader_stage
        );
    }
    let started = std::time::Instant::now();
    let texels = shade_map(&plan.clouds, transform.rotation, SHADE_TEXELS);
    let image = Image::new(
        Extent3d {
            width: SHADE_TEXELS as u32,
            height: SHADE_TEXELS as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texels,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.entity(entity).insert(DirectionalLightTexture {
        image: images.add(image),
        tiled: false,
    });
    info!(
        "clouds: storm shade baked for {} darkened cells in {:.0} ms",
        plan.clouds.shades.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_core::world::clouds::{Cirrus, CloudKind, Puff, RainShaft};
    use cb_core::world::weather::Sky;

    fn sky() -> Clouds {
        let mut puffs = Vec::new();
        for k in 0..200u32 {
            let x = -9000.0 + (k % 20) as f32 * 900.0;
            let z = -9000.0 + (k / 20) as f32 * 1700.0;
            puffs.push(Puff {
                centre: Vec3::new(x, 700.0 + (k % 3) as f32 * 300.0, z),
                radii: Vec3::new(200.0, 150.0, 200.0),
                kind: CloudKind::Cumulus,
                dark: 0.1,
                opacity: 0.95,
                layer_m: [600.0, 1500.0],
                seed: k as f32,
            });
        }
        let shafts = vec![RainShaft {
            centre: Vec2::new(1000.0, 500.0),
            radius_m: 1500.0,
            top_m: 720.0,
        }];
        let cirrus = vec![Cirrus {
            centre: Vec3::new(0.0, 8500.0, 0.0),
            length_m: 6000.0,
            width_m: 800.0,
            angle_rad: 0.2,
            seed: 3.0,
        }];
        Clouds::new(
            Sky {
                puffs,
                shafts,
                cirrus,
                ..Sky::default()
            },
            Vec2::new(6.0, 0.0),
            0.7,
        )
    }

    #[test]
    fn items_are_drawn_from_the_farthest_near_side_to_the_nearest() {
        let eye = Vec3::new(300.0, 2.0, -400.0);
        let list = items(&sky(), eye, 6000.0);
        assert!(list.len() > 20);
        for w in list.windows(2) {
            assert!(near_side_m(&w[0], eye) >= near_side_m(&w[1], eye));
        }
    }

    #[test]
    fn a_big_puff_sorts_by_its_near_side_not_its_centre() {
        // A big puff whose centre is farther than a small one's, but whose near side is nearer, is
        // drawn over it.
        let puff = |centre: Vec3, r: f32| Puff {
            centre,
            radii: Vec3::splat(r),
            kind: CloudKind::Tower,
            dark: 0.5,
            opacity: 1.0,
            layer_m: [700.0, 9000.0],
            seed: 1.0,
        };
        let c = Clouds::new(
            Sky {
                puffs: vec![
                    puff(Vec3::new(1000.0, 1000.0, -2400.0), 900.0),
                    puff(Vec3::new(0.0, 1000.0, -2000.0), 100.0),
                ],
                ..Sky::default()
            },
            Vec2::ZERO,
            0.7,
        );
        let list = items(&c, Vec3::new(0.0, 1000.0, 0.0), 9000.0);
        assert_eq!(list[0].centre.z, -2000.0, "the small one, its near side farther, first");
        assert_eq!(list[1].centre.x, 1000.0, "the big one drawn over it");
    }

    #[test]
    fn only_puffs_within_the_radius_are_gathered_and_every_one_of_them_is() {
        let c = sky();
        let eye = Vec3::new(-2000.0, 50.0, 1000.0);
        let list = items(&c, eye, 4000.0);
        let puffs: Vec<&Item> = list.iter().filter(|i| i.look.x < KIND_SHAFT - 0.5).collect();
        let within = c
            .puffs
            .iter()
            .filter(|p| Vec2::new(p.centre.x, p.centre.z).distance(Vec2::new(eye.x, eye.z)) <= 4000.0)
            .count();
        assert_eq!(puffs.len(), within);
        assert_eq!(
            list.iter().filter(|i| i.look.x == KIND_SHAFT).count(),
            3,
            "three sheets for the shaft"
        );
        assert_eq!(list.iter().filter(|i| i.look.x == KIND_CIRRUS).count(), 1);
    }

    #[test]
    fn the_shade_map_darkens_where_the_sun_crosses_a_storm() {
        use cb_core::world::clouds::StormShade;
        let shade = StormShade {
            centre: Vec2::new(-8000.0, 3000.0),
            radius_m: 5000.0,
            shade: 0.6,
            wobble_m: 0.0,
        };
        let c = Clouds::new(
            Sky {
                shades: vec![shade],
                ..Sky::default()
            },
            Vec2::X,
            0.7,
        );
        let rotation = Quat::from_euler(EulerRot::YXZ, 0.8, -0.75, 0.0);
        let n = 64;
        let map = shade_map(&c, rotation, n);
        assert!(map.iter().any(|&t| t < 110), "60 % of the sun taken at the storm's heart");
        assert!(map.iter().filter(|&&t| t == 255).count() > n * n / 2, "most of the map in full sun");
        // The darkest texel's ray crosses the cloud base near the storm's centre, as Bevy reads it.
        let (k, _) = map.iter().enumerate().min_by_key(|(_, t)| **t).unwrap();
        let (u, v) = (((k % n) as f32 + 0.5) / n as f32, ((k / n) as f32 + 0.5) / n as f32);
        let w = rotation * Vec3::new((0.5 - u) * 2.0, (v - 0.5) * 2.0, 0.0) * SHADE_HALF_M;
        let travel = rotation * Vec3::NEG_Z;
        let at = w + travel * ((SHADE_AT_M - w.y) / travel.y);
        assert!(Vec2::new(at.x, at.z).distance(shade.centre) < 0.45 * shade.radius_m + 2.0 * SHADE_HALF_M / n as f32);
    }

    #[test]
    fn one_quad_an_item() {
        let list = items(&sky(), Vec3::ZERO, 6000.0);
        let m = item_mesh(&list);
        assert_eq!(m.count_vertices(), list.len() * 4);
        assert_eq!(m.indices().map(|i| i.len()), Some(list.len() * 6));
    }

    /// The share of a cloud's light that crosses the air to the eye, as `air_between` in `cloud.wesl` works it out
    /// from the uniform's `fog`, `haze` and the mist's thickening toward it (the core's closed form of each layer); the
    /// shader mixes the cloud's colour toward the fog's by the rest.
    fn air_between(fog: Vec4, haze: Vec4, thick: f32, eye: Vec3, cloud: Vec3) -> f32 {
        let to = cloud - eye;
        let (dist, dir_y) = (to.length(), to.normalize().y);
        let layer = cb_core::world::mist::optical_depth(
            &cb_core::world::mist::FogConfig {
                density_per_m: fog.x,
                scale_height_m: fog.y,
                ..Default::default()
            },
            fog.z,
            eye.y,
            dir_y,
            dist,
            thick,
        );
        let haze = cb_core::world::mist::haze_depth(
            &cb_core::world::mist::FogConfig {
                haze_density_per_m: haze.x,
                haze_scale_height_m: haze.y,
                ..Default::default()
            },
            fog.z,
            eye.y,
            dir_y,
            dist,
        );
        (-(layer + haze)).exp()
    }

    #[test]
    fn a_far_cloud_on_the_horizon_takes_the_fogs_colour_and_one_overhead_does_not() {
        use crate::water_render::AirFog;
        let cfg = cb_core::world::mist::FogConfig::default();
        let morning = cfg.presets.morning;
        let air = AirFog::now(&cfg, morning, 0.0, 0.0);
        let (fog, haze) = CloudUniform::air(&air, true);
        let eye = Vec3::new(0.0, 2.0, 0.0);
        let far = air_between(fog, haze, 1.0, eye, Vec3::new(18000.0, 800.0, 0.0));
        let overhead = air_between(fog, haze, 1.0, eye, Vec3::new(300.0, 1500.0, 0.0));
        assert!(
            far < 0.7,
            "a cloud 18 km off on the horizon keeps {far:.2} of its own colour: it should take the fog's"
        );
        assert!(
            overhead > 0.95,
            "a cloud 1.5 km overhead keeps {overhead:.2}: the air should barely touch it"
        );
        // Over misty land the mist round the eye thickens the layer toward the cloud as it does toward the land under
        // it (owner, 2026-10-27: "cloud fog blend should be a thing"): the eye 80 m over full mist.
        let side = 64;
        let misty = cb_core::world::mist::MistMap {
            side,
            origin_m: -(side as f32) * 250.0,
            cell_m: 500.0,
            mist: vec![1.0; side * side],
        };
        let low = Vec3::new(0.0, 80.0, 0.0);
        let thick = thick_round(&cfg, morning, &misty, low);
        assert!(
            thick.iter().all(|v| (v.x - v.w).abs() < 1e-5 && v.x > 5.0),
            "even mist thickens every bearing alike: {thick:?}"
        );
        let cloud = Vec3::new(12000.0, 900.0, 0.0);
        let (in_mist, in_clear) = (
            air_between(fog, haze, thick[0].x, low, cloud),
            air_between(fog, haze, 1.0, low, cloud),
        );
        assert!(
            in_mist < 0.8 * in_clear,
            "over mist a far cloud keeps {in_mist:.2}, over clear land {in_clear:.2}"
        );
        // Seen from above the layer, a cloud stays crisp however misty the ground: the transmittance through the layer,
        // not the distance.
        let above = air_between(fog, haze, thick[0].x, Vec3::new(0.0, 2000.0, 0.0), Vec3::new(3000.0, 1500.0, 0.0));
        assert!(above > 0.95, "from above the fog a cloud keeps {above:.2}");
        // Rain at the eye does not tint a cloud: all of its haze lies under the storm's own base.
        let wet = AirFog::now(&cfg, morning, 0.0, 1.0);
        assert_eq!(CloudUniform::air(&wet, true), (fog, haze));
        // With the ground fog off (`CB_NO_GROUND_FOG=1`) no cloud takes the fog's colour.
        let (off_fog, off_haze) = CloudUniform::air(&air, false);
        assert_eq!(air_between(off_fog, off_haze, thick[0].x, eye, Vec3::new(18000.0, 800.0, 0.0)), 1.0);
    }

    #[test]
    fn the_cloud_shader_compiles_against_bevy_pbr_and_its_uniform_matches() {
        use crate::shader_check::{compile, struct_size, with_bevy};
        let Some(resolver) = with_bevy() else {
            eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
            return;
        };
        let base = [("VERTEX_POSITIONS", true), ("CLUSTERED_FORWARD_STORAGE_BUFFER", true)];
        for (msaa, prepass, fog) in [(false, true, true), (true, true, true), (false, false, false)] {
            let mut flags = base.to_vec();
            flags.extend([("MULTISAMPLED", msaa), ("DEPTH_PREPASS", prepass), ("DISTANCE_FOG", fog)]);
            let module = compile(&resolver, "cloud", &flags);
            assert_eq!(
                u64::from(struct_size(&module, "CloudLight")),
                CloudUniform::min_size().get(),
                "the Rust uniform must be the WESL struct's size"
            );
        }
    }
}
