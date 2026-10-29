//! The lamps in the game (openspec changes `day-night-cycle`, design section 5, and `night-voxel-light`, section
//! 2d): every lamp of `cb_core::world::lamps::place`, the street lamps and the airport's and seaport's masts and
//! floodlights, drawn as poles and heads merged into one mesh per kilometre tile (the metal, and the lit heads one
//! draw a colour), with every glow dot of `place_dots` (runway, approach and taxiway lights, ships' lights: lights
//! that light nothing); and the nearest lamps that light, up to `real_lamps` within `real_m` of the eye, made real
//! spot lights without shadows. Those are a fixed pool of light entities moved from lamp to lamp, never spawned per
//! frame; each one's light fades in when it is given a lamp and fades with distance, so a lamp never pops on. Lamps
//! and dots come on at dusk from the hour (`day_night::Light`); a light standing on a ship or a crane goes out once
//! that is damaged. Its own module: drawing and lighting the lamps is one capability.

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
use cb_core::world::lamps::{GlowDot, Lamp, LampKind, LightColour, place, place_dots};
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
    /// The lit heads' materials by colour, brightened at dusk.
    heads: Vec<(LightColour, Handle<StandardMaterial>)>,
    /// The heads' far glows' materials by colour.
    glows: Vec<(LightColour, Handle<LampGlowMaterial>)>,
    /// The glow dots' materials by colour.
    dot_glows: Vec<(LightColour, Handle<LampGlowMaterial>)>,
}

/// One light of the pool: the lamp it lights now and how far it has faded in.
#[derive(Component, Default)]
struct PoolLight {
    lamp: Option<u32>,
    admitted: f32,
}

/// The glow dots standing on one lot (a ship, a crane): hidden once that lot is damaged.
#[derive(Component)]
struct LotLights(u32);

/// Draws and lights the lamps.
pub struct LampsPlugin;

impl Plugin for LampsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LampGlowMaterial>::default())
            .add_world_spawn(spawn_lamps)
            .add_game_systems(Update, (light_lamps, dark_lots).in_set(PresentSet::Pose));
    }
}

fn cell_of(p: Vec3) -> (i32, i32) {
    ((p.x / CELL_M).floor() as i32, (p.z / CELL_M).floor() as i32)
}

fn tile_of(p: Vec3) -> (i32, i32) {
    ((p.x / TILE_M).floor() as i32, (p.z / TILE_M).floor() as i32)
}

/// One lamp's pole, arm and head as meshes in world space: the metal and the head. A street lamp is a slim pole with
/// an arm over the road; a mast a thick pole with a floodlight box on top, turned toward what it lights (smaller on a
/// short post).
fn lamp_meshes(l: &Lamp) -> (Mesh, Mesh) {
    let top = l.head.y - l.base.y;
    let mast = l.kind == LampKind::Mast;
    let thick = if mast { 0.6 } else { 0.22 };
    let pole = Mesh::from(Cuboid::new(thick, top, thick)).transformed_by(Transform::from_translation(l.base + Vec3::Y * (top * 0.5)));
    let reach = Vec3::new(l.head.x - l.base.x, 0.0, l.head.z - l.base.z);
    let facing = if mast { Vec3::new(l.toward.x, 0.0, l.toward.y) } else { reach };
    let yaw = Quat::from_rotation_arc(Vec3::X, facing.normalize_or(Vec3::X));
    let mut metal = pole;
    if !mast {
        let arm = Mesh::from(Cuboid::new(reach.length().max(0.1), 0.14, 0.14))
            .transformed_by(Transform::from_translation(l.base + Vec3::Y * (top - 0.05) + reach * 0.5).with_rotation(yaw));
        // Both are plain cuboids with the same attributes.
        let _ = metal.merge(&arm);
    }
    let head = if mast {
        let k = (top / 12.0).min(1.0);
        Mesh::from(Cuboid::new(1.2 * k, 0.6 * k, 3.0 * k)).transformed_by(Transform::from_translation(l.head).with_rotation(yaw))
    } else {
        Mesh::from(Cuboid::new(0.8, 0.22, 0.42)).transformed_by(Transform::from_translation(l.head - Vec3::Y * 0.1).with_rotation(yaw))
    };
    (metal, head)
}

/// Adds `b` to the mesh at `key`, or puts it there.
fn merge_into<K: std::hash::Hash + Eq>(map: &mut HashMap<K, Mesh>, key: K, b: Mesh) {
    match map.get_mut(&key) {
        Some(m) => {
            // Same attributes throughout.
            let _ = m.merge(&b);
        }
        None => {
            map.insert(key, b);
        }
    }
}

/// Places every lamp and glow dot, draws their poles, heads and glows by tile and colour, and spawns the pool of real
/// lights.
fn spawn_lamps(
    mut commands: Commands,
    data: Res<Data>,
    plan: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glow_assets: ResMut<Assets<LampGlowMaterial>>,
) {
    let cfg = &data.0.day_night.lamps;
    let started = std::time::Instant::now();
    let list = place(&plan.0, cfg);
    let dots = place_dots(&plan.0, cfg);
    let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
    let mut metal_tiles: HashMap<(i32, i32), Mesh> = HashMap::new();
    let mut head_tiles: HashMap<((i32, i32), LightColour), Mesh> = HashMap::new();
    let mut glow_heads: HashMap<((i32, i32), LightColour), Vec<Vec3>> = HashMap::new();
    for (id, l) in list.iter().enumerate() {
        if l.power > 0.0 {
            cells.entry(cell_of(l.head)).or_default().push(id as u32);
        }
        if !l.pole {
            continue;
        }
        let tile = tile_of(l.base);
        glow_heads.entry((tile, l.colour)).or_default().push(l.head);
        let (metal, head) = lamp_meshes(l);
        merge_into(&mut metal_tiles, tile, metal);
        merge_into(&mut head_tiles, (tile, l.colour), head);
    }
    // Dots by tile and colour, those on a lot (a ship, a crane) by lot and colour, so a damaged one can go dark.
    let mut dot_groups: HashMap<(Option<u32>, (i32, i32), LightColour), Vec<Vec3>> = HashMap::new();
    for d in &dots {
        let tile = if d.lot.is_some() { (0, 0) } else { tile_of(d.at) };
        dot_groups.entry((d.lot, tile, d.colour)).or_default().push(d.at);
    }
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.29, 0.30),
        perceptual_roughness: 0.6,
        metallic: 0.6,
        ..default()
    });
    let heads: Vec<(LightColour, Handle<StandardMaterial>)> = LightColour::ALL
        .iter()
        .map(|c| {
            let h = materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.6, 0.58),
                emissive: LinearRgba::BLACK,
                perceptual_roughness: 0.4,
                ..default()
            });
            (*c, h)
        })
        .collect();
    let mut glow_set = || -> Vec<(LightColour, Handle<LampGlowMaterial>)> {
        LightColour::ALL
            .iter()
            .map(|c| {
                (
                    *c,
                    glow_assets.add(LampGlowMaterial {
                        lamp: LampGlowUniform::default(),
                    }),
                )
            })
            .collect()
    };
    let glows = glow_set();
    let dot_glows = glow_set();
    let of = |set: &[(LightColour, Handle<LampGlowMaterial>)], c: LightColour| {
        set.iter()
            .find(|(k, _)| *k == c)
            .map(|(_, h)| h.clone())
            .expect("every colour has a material")
    };
    let mut keys: Vec<_> = metal_tiles.keys().copied().collect();
    keys.sort();
    for key in keys {
        let m = metal_tiles.remove(&key).expect("a tile listed");
        commands.spawn((Name::new("lamp poles"), Mesh3d(meshes.add(m)), MeshMaterial3d(metal.clone())));
    }
    let mut keys: Vec<_> = head_tiles.keys().copied().collect();
    keys.sort();
    for key in keys {
        let h = head_tiles.remove(&key).expect("a tile listed");
        let material = heads
            .iter()
            .find(|(c, _)| *c == key.1)
            .map(|(_, h)| h.clone())
            .expect("every colour");
        commands.spawn((
            Name::new("lamp heads"),
            Mesh3d(meshes.add(h)),
            MeshMaterial3d(material),
            NotShadowCaster,
        ));
        commands.spawn((
            Name::new("lamp glow"),
            Mesh3d(meshes.add(glow_mesh(&glow_heads[&key]))),
            MeshMaterial3d(of(&glows, key.1)),
            NotShadowCaster,
        ));
    }
    let mut keys: Vec<_> = dot_groups.keys().copied().collect();
    keys.sort();
    for key in keys {
        let mut e = commands.spawn((
            Name::new("light dots"),
            Mesh3d(meshes.add(glow_mesh(&dot_groups[&key]))),
            MeshMaterial3d(of(&dot_glows, key.2)),
            NotShadowCaster,
        ));
        if let Some(lot) = key.0 {
            e.insert(LotLights(lot));
        }
    }
    for _ in 0..cfg.real_lamps {
        commands.spawn((
            Name::new("lamp light"),
            PoolLight::default(),
            SpotLight {
                intensity: 0.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let masts = list.iter().filter(|l| l.kind == LampKind::Mast).count();
    info!(
        "lamps: {} street lamps and {masts} masts and floodlights in {} cells, {} glow dots, placed and meshed in {:.0} ms",
        list.len() - masts,
        cells.len(),
        dots.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    commands.insert_resource(Lamps {
        list,
        cells,
        heads,
        glows,
        dot_glows,
    });
}

/// The nearest lamps that light, to `eye` within `reach_m`, nearest first (ties by id), at most `n`, leaving out any
/// `dark` says is out (one on a damaged lot).
pub fn nearest(lamps: &Lamps, eye: Vec3, reach_m: f32, n: usize, dark: impl Fn(&Lamp) -> bool) -> Vec<(u32, f32)> {
    let (cx, cz) = cell_of(eye);
    let r = (reach_m / CELL_M).ceil() as i32;
    let mut found: Vec<(u32, f32)> = Vec::new();
    for x in cx - r..=cx + r {
        for z in cz - r..=cz + r {
            for &id in lamps.cells.get(&(x, z)).map_or(&[][..], |v| v.as_slice()) {
                let l = &lamps.list[id as usize];
                let d = l.head.distance(eye);
                if d <= reach_m && !dark(l) {
                    found.push((id, d));
                }
            }
        }
    }
    found.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    found.truncate(n);
    found
}

/// Hides the glow dots of a lot once it is damaged (its lights are out, as a damaged building's rooms are).
fn dark_lots(live: Option<Res<crate::world::Live>>, mut dots: Query<(&LotLights, &mut Visibility)>) {
    let Some(live) = live else { return };
    for (lot, mut visibility) in &mut dots {
        let want = if live.structures.houses.states.contains_key(&lot.0) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Lights the lamps for the hour: the heads and the dots glow from dusk, and the nearest are given real lights.
#[allow(clippy::too_many_arguments)]
fn light_lamps(
    data: Res<Data>,
    light: Res<Light>,
    time: Res<Time>,
    lamps: Option<Res<Lamps>>,
    live: Option<Res<crate::world::Live>>,
    eye: Option<Single<&Transform, (With<crate::pilot::PilotCamera>, Without<PoolLight>)>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glows: ResMut<Assets<LampGlowMaterial>>,
    mut pool: Query<(&mut PoolLight, &mut SpotLight, &mut Transform, &mut Visibility)>,
) {
    let (Some(lamps), Some(eye)) = (lamps, eye) else { return };
    let cfg = &data.0.day_night.lamps;
    let sun_deg = cb_core::day_night::elevation_deg(light.0.sun_dir);
    let on = cfg.on_share(sun_deg);
    for (colour, h) in &lamps.heads {
        let [r, g, b] = colour.rgb(cfg);
        let want = LinearRgba::from(Color::srgb(r, g, b)) * (cfg.head_cd_m2 * on);
        if materials.get(h).is_some_and(|m| m.emissive != want)
            && let Some(mut m) = materials.get_mut(h)
        {
            m.emissive = want;
        }
    }
    let looks = lamps
        .glows
        .iter()
        .map(|(c, h)| (*c, h, cfg.glow_brightness, cfg.glow_m))
        .chain(lamps.dot_glows.iter().map(|(c, h)| (*c, h, cfg.dot_brightness, cfg.dot_m)));
    for (colour, h, brightness, size_m) in looks {
        let [r, g, b] = colour.rgb(cfg);
        let want = LampGlowUniform {
            colour: (Vec3::new(r, g, b) * brightness).extend(0.0),
            glow: Vec4::new(on, cfg.glow_px, cfg.glow_from_m, size_m),
        };
        if glows
            .get(h)
            .is_some_and(|m| m.lamp.colour != want.colour || m.lamp.glow != want.glow)
            && let Some(mut m) = glows.get_mut(h)
        {
            m.lamp = want;
        }
    }
    let damaged = |l: &Lamp| {
        l.lot
            .is_some_and(|lot| live.as_ref().is_some_and(|live| live.structures.houses.states.contains_key(&lot)))
    };
    let near = if on > 0.0 {
        nearest(&lamps, eye.translation, cfg.real_m, cfg.real_lamps as usize, damaged)
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
    let (street_inner, street_outer) = (cfg.cone_deg[0].to_radians(), cfg.cone_deg[1].to_radians());
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
        let (lumens, inner) = if lamp.kind == LampKind::Mast {
            // A floodlight's soft edge: the cone's inner 30 % at full (the pages' penumbra 0.7).
            (cfg.mast_lumens, lamp.cone_rad * 0.3)
        } else {
            (cfg.lumens, street_inner.min(lamp.cone_rad))
        };
        spot.intensity = lumens * lamp.power * on * far * p.admitted;
        let [r, g, b] = lamp.colour.rgb(cfg);
        spot.color = Color::srgb(r, g, b);
        spot.range = lamp.range_m;
        spot.outer_angle = if lamp.kind == LampKind::Mast { lamp.cone_rad } else { street_outer };
        spot.inner_angle = inner;
        let from = lamp.head - Vec3::Y * if lamp.pole { 0.2 } else { 0.0 };
        *transform = Transform::from_translation(from).looking_at(
            lamp.aim,
            if (lamp.aim - from).normalize_or(Vec3::NEG_Y).y.abs() > 0.99 {
                Vec3::X
            } else {
                Vec3::Y
            },
        );
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
}
