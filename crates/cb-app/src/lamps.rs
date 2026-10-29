//! The street lamps in the game (openspec change `day-night-cycle`, design section 5): every lamp of
//! `cb_core::world::lamps::place`, drawn as poles and heads merged into one mesh per kilometre tile
//! (two draws a tile: the metal and the lit heads), and the nearest of them, up to `real_lamps` within
//! `real_m` of the eye, made real spot lights without shadows. Those are a fixed pool of light
//! entities moved from lamp to lamp, never spawned per frame; each one's light fades in when it is
//! given a lamp and fades with distance, so a lamp never pops on. Lamps come on at dusk from the
//! hour (`day_night::Light`). Its own module: drawing and lighting the lamps is one capability.

use crate::day_night::Light;
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use crate::world::Plan;
use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cb_core::world::lamps::{Lamp, place};
use std::collections::HashMap;

/// A tile of merged poles, metres.
const TILE_M: f32 = 1000.0;
/// The lamp index's cell, metres.
const CELL_M: f32 = 64.0;
/// How fast a newly given light fades in, seconds.
const ADMIT_S: f32 = 0.5;

const GLOW_SHADER: &str = "shaders/lamp_glow.wesl";

/// `LampGlow` in `lamp_glow.wesl`.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct LampGlowUniform {
    /// rgb the glow at its heart, on screen; w unused.
    pub colour: Vec4,
    /// x the lamps' switch-on share; y the least size, pixels; z fully in from this far, m; w its
    /// size in the world, m.
    pub glow: Vec4,
}

/// The far lamps' glow (tier 3): every lamp a quad around its head, sized on screen.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct LampGlowMaterial {
    /// The look, rewritten as the lamps come on.
    #[uniform(0)]
    pub lamp: LampGlowUniform,
}

impl Material for LampGlowMaterial {
    fn vertex_shader() -> ShaderRef {
        GLOW_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        GLOW_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

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
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// One tile's glows: four corners a lamp, each at its head (the shader spreads them on screen).
fn glow_mesh(heads: &[Vec3]) -> Mesh {
    let mut pos = Vec::with_capacity(heads.len() * 4);
    let mut uv = Vec::with_capacity(heads.len() * 4);
    let mut idx = Vec::with_capacity(heads.len() * 6);
    for (i, h) in heads.iter().enumerate() {
        let b = (i * 4) as u32;
        for c in [[-1.0f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] {
            pos.push(h.to_array());
            uv.push(c);
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(idx))
}

/// Every lamp, and an index of them by cell.
#[derive(Resource)]
pub struct Lamps {
    /// Every lamp, its index its id.
    pub list: Vec<Lamp>,
    cells: HashMap<(i32, i32), Vec<u32>>,
    /// The lit heads' material, brightened at dusk.
    heads: Handle<StandardMaterial>,
    /// The far glow's material.
    glow: Handle<LampGlowMaterial>,
}

/// One light of the pool: the lamp it lights now and how far it has faded in.
#[derive(Component, Default)]
struct PoolLight {
    lamp: Option<u32>,
    admitted: f32,
}

/// Draws and lights the lamps.
pub struct LampsPlugin;

impl Plugin for LampsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LampGlowMaterial>::default())
            .add_world_spawn(spawn_lamps)
            .add_game_systems(Update, light_lamps.in_set(PresentSet::Pose));
    }
}

fn cell_of(p: Vec3) -> (i32, i32) {
    ((p.x / CELL_M).floor() as i32, (p.z / CELL_M).floor() as i32)
}

/// One lamp's pole, arm and head as meshes in world space: the metal and the head.
fn lamp_meshes(l: &Lamp) -> (Mesh, Mesh) {
    let top = l.head.y - l.base.y;
    let pole = Mesh::from(Cuboid::new(0.22, top, 0.22)).transformed_by(Transform::from_translation(l.base + Vec3::Y * (top * 0.5)));
    let reach = Vec3::new(l.head.x - l.base.x, 0.0, l.head.z - l.base.z);
    let yaw = Quat::from_rotation_arc(Vec3::X, reach.normalize_or(Vec3::X));
    let arm = Mesh::from(Cuboid::new(reach.length().max(0.1), 0.14, 0.14))
        .transformed_by(Transform::from_translation(l.base + Vec3::Y * (top - 0.05) + reach * 0.5).with_rotation(yaw));
    let mut metal = pole;
    // Both are plain cuboids with the same attributes.
    let _ = metal.merge(&arm);
    let head =
        Mesh::from(Cuboid::new(0.8, 0.22, 0.42)).transformed_by(Transform::from_translation(l.head - Vec3::Y * 0.1).with_rotation(yaw));
    (metal, head)
}

/// Places every lamp, draws their poles and heads by tile and spawns the pool of real lights.
fn spawn_lamps(
    mut commands: Commands,
    data: Res<Data>,
    plan: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glows: ResMut<Assets<LampGlowMaterial>>,
) {
    let cfg = &data.0.day_night.lamps;
    let started = std::time::Instant::now();
    let list = place(&plan.0, cfg);
    let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
    let mut tiles: HashMap<(i32, i32), (Mesh, Mesh)> = HashMap::new();
    let mut tile_heads: HashMap<(i32, i32), Vec<Vec3>> = HashMap::new();
    for (id, l) in list.iter().enumerate() {
        cells.entry(cell_of(l.head)).or_default().push(id as u32);
        let tile = ((l.base.x / TILE_M).floor() as i32, (l.base.z / TILE_M).floor() as i32);
        tile_heads.entry(tile).or_default().push(l.head);
        let (metal, head) = lamp_meshes(l);
        match tiles.get_mut(&tile) {
            Some((m, h)) => {
                let _ = m.merge(&metal);
                let _ = h.merge(&head);
            }
            None => {
                tiles.insert(tile, (metal, head));
            }
        }
    }
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.29, 0.30),
        perceptual_roughness: 0.6,
        metallic: 0.6,
        ..default()
    });
    let [r, g, b] = cfg.colour;
    let heads = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.58),
        emissive: LinearRgba::BLACK,
        perceptual_roughness: 0.4,
        ..default()
    });
    let glow = glows.add(LampGlowMaterial {
        lamp: LampGlowUniform::default(),
    });
    let mut keys: Vec<_> = tiles.keys().copied().collect();
    keys.sort();
    for key in keys {
        let (m, h) = tiles.remove(&key).expect("a tile listed");
        let g = glow_mesh(&tile_heads[&key]);
        commands.spawn((
            Name::new("lamp glow"),
            Mesh3d(meshes.add(g)),
            MeshMaterial3d(glow.clone()),
            NotShadowCaster,
        ));
        commands.spawn((Name::new("lamp poles"), Mesh3d(meshes.add(m)), MeshMaterial3d(metal.clone())));
        commands.spawn((
            Name::new("lamp heads"),
            Mesh3d(meshes.add(h)),
            MeshMaterial3d(heads.clone()),
            NotShadowCaster,
        ));
    }
    let (inner, outer) = (cfg.cone_deg[0].to_radians(), cfg.cone_deg[1].to_radians());
    for _ in 0..cfg.real_lamps {
        commands.spawn((
            Name::new("street lamp light"),
            PoolLight::default(),
            SpotLight {
                color: Color::srgb(r, g, b),
                intensity: 0.0,
                range: cfg.range_m,
                inner_angle: inner,
                outer_angle: outer,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    info!(
        "lamps: {} street lamps in {} tiles, placed and meshed in {:.0} ms",
        list.len(),
        cells.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    commands.insert_resource(Lamps { list, cells, heads, glow });
}

/// The nearest lamps to `eye` within `reach_m`, nearest first (ties by id), at most `n`.
pub fn nearest(lamps: &Lamps, eye: Vec3, reach_m: f32, n: usize) -> Vec<(u32, f32)> {
    let (cx, cz) = cell_of(eye);
    let r = (reach_m / CELL_M).ceil() as i32;
    let mut found: Vec<(u32, f32)> = Vec::new();
    for x in cx - r..=cx + r {
        for z in cz - r..=cz + r {
            for &id in lamps.cells.get(&(x, z)).map_or(&[][..], |v| v.as_slice()) {
                let d = lamps.list[id as usize].head.distance(eye);
                if d <= reach_m {
                    found.push((id, d));
                }
            }
        }
    }
    found.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    found.truncate(n);
    found
}

/// Lights the lamps for the hour: the heads glow from dusk, and the nearest are given real lights.
fn light_lamps(
    data: Res<Data>,
    light: Res<Light>,
    time: Res<Time>,
    lamps: Option<Res<Lamps>>,
    eye: Option<Single<&Transform, (With<crate::pilot::PilotCamera>, Without<PoolLight>)>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glows: ResMut<Assets<LampGlowMaterial>>,
    mut pool: Query<(&mut PoolLight, &mut SpotLight, &mut Transform, &mut Visibility)>,
) {
    let (Some(lamps), Some(eye)) = (lamps, eye) else { return };
    let cfg = &data.0.day_night.lamps;
    let sun_deg = cb_core::day_night::elevation_deg(light.0.sun_dir);
    let on = cfg.on_share(sun_deg);
    if let Some(mut heads) = materials.get_mut(&lamps.heads) {
        let [r, g, b] = cfg.colour;
        let want = LinearRgba::from(Color::srgb(r, g, b)) * (cfg.head_cd_m2 * on);
        if heads.emissive != want {
            heads.emissive = want;
        }
    }
    let [r, g, b] = cfg.colour;
    let want = LampGlowUniform {
        colour: (Vec3::new(r, g, b) * cfg.glow_brightness).extend(0.0),
        glow: Vec4::new(on, cfg.glow_px, cfg.glow_from_m, cfg.glow_m),
    };
    if glows
        .get(&lamps.glow)
        .is_some_and(|m| m.lamp.colour != want.colour || m.lamp.glow != want.glow)
        && let Some(mut m) = glows.get_mut(&lamps.glow)
    {
        m.lamp = want;
    }
    // `CB_NO_LAMPS=1` gives no lamp a real light (an instrument, to measure the night: openspec change
    // `night-frame`); the heads still glow.
    let near = if on > 0.0 && !crate::cli::switched_off("CB_NO_LAMPS") {
        nearest(&lamps, eye.translation, cfg.real_m, cfg.real_lamps as usize)
    } else {
        Vec::new()
    };
    let wanted: HashMap<u32, f32> = near.iter().copied().collect();
    // Lights keep a lamp still wanted; the rest are freed, then given the wanted lamps without one.
    let mut held: Vec<u32> = Vec::new();
    for (mut p, ..) in &mut pool {
        if p.lamp.is_some_and(|id| !wanted.contains_key(&id)) {
            p.lamp = None;
        }
        if let Some(id) = p.lamp {
            held.push(id);
        }
    }
    let mut waiting = near.iter().map(|(id, _)| *id).filter(|id| !held.contains(id));
    let dt = time.delta_secs();
    for (mut p, mut spot, mut transform, mut visibility) in &mut pool {
        if p.lamp.is_none()
            && let Some(id) = waiting.next()
        {
            p.lamp = Some(id);
            p.admitted = 0.0;
        }
        let Some(id) = p.lamp else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        let lamp = &lamps.list[id as usize];
        p.admitted = (p.admitted + dt / ADMIT_S).min(1.0);
        let d = wanted.get(&id).copied().unwrap_or(cfg.real_m);
        let far = 1.0 - cb_core::world::terrain::smoothstep(cfg.real_m - cfg.fade_m, cfg.real_m, d);
        spot.intensity = cfg.lumens * on * far * p.admitted;
        // Down, leaning a little over the road.
        let aim = lamp.head + Vec3::new(lamp.toward.x, 0.0, lamp.toward.y) * 2.0 - Vec3::Y * 6.0;
        *transform = Transform::from_translation(lamp.head - Vec3::Y * 0.2).looking_at(aim, Vec3::Y);
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
}
