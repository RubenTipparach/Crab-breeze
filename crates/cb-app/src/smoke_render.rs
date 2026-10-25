//! Lit smoke puffs (openspec change `smoke-puffs`; owner, 2026-10-25: "can we render smoke like
//! that? especially stuff from explosions?", "use the new smoke trails, and make them larger and more
//! conencted"). Each puff is a lumpy blob drawn by the cloud shader (`assets/shaders/cloud.wesl` with
//! `SMOKE`) in the scene's own light, its life (`cb_core::smoke::Puff`) stepped here each frame: it
//! grows, rises, drifts with the wind and fades. Every puff is its own entity, so the transparent pass
//! sorts each on its own; puffs share one of a few lumpy meshes and one material per colour and
//! opacity step, so neighbours batch.
//!
//! Trails are laid by distance, not by frame (`cb_core::fx::lay_along`): a puff every `spacing_m`
//! along the path, each wider than the spacing, so a trail is one connected stream at any frame rate.
//! The jets lay one while a mech lifts on its rockets; a missile lays one along its flight.

use crate::clouds_render::{CloudUniform, SHADER, lump, scene_light};
use crate::combat::Shots;
use crate::mech::{MechRig, MechSim};
use crate::pilot::PilotCamera;
use crate::sim::{Data, PresentSet, SimSet};
use crate::world::Plan;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cb_core::fx::{PuffTrail, lay_along};
use cb_core::smoke::{Puff, PuffSpec};
use std::collections::{HashMap, VecDeque};

/// One puff to put in the sky.
#[derive(Message, Clone, Copy, Debug)]
pub struct PuffAt {
    /// Its centre, world metres.
    pub at: Vec3,
    /// Thrown at, metres per second.
    pub vel: Vec3,
    /// Its life.
    pub spec: PuffSpec,
    /// Its colour, linear RGB.
    pub colour: [f32; 3],
}

impl PuffAt {
    /// A puff of trail `t` at `at`, thrown at `vel`.
    pub fn trail(t: &PuffTrail, at: Vec3, vel: Vec3) -> Self {
        Self {
            at,
            vel,
            spec: t.spec(),
            colour: t.colour,
        }
    }
}

/// A smoke puff's material: the clouds' light, and the puff's colour and opacity step.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SmokeMaterial {
    /// The scene's light, rewritten each frame (`clouds_render::scene_light`).
    #[uniform(0)]
    pub light: CloudUniform,
    /// rgb the colour (linear), a the opacity.
    #[uniform(1)]
    pub tint: Vec4,
}

impl Material for SmokeMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
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
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push("SMOKE".into());
        }
        Ok(())
    }
}

/// Opacity steps a puff's material is chosen from as it fades.
const LEVELS: u8 = 16;
/// Lumpy shapes the puffs share.
const SHAPES: u32 = 4;
/// Icosphere subdivisions of a puff: 2 is 42 vertices, 80 triangles; lumpy enough at smoke's size.
const DETAIL: u32 = 2;
/// A puff fades out within this of the eye, so the pilot never looks through its inside.
const NEAR_FADE_M: f32 = 3.0;
/// A source that moves further than this in a step has jumped (a respawn): its trail starts afresh.
const MAX_STEP_M: f32 = 200.0;

/// Smoke puffs.
pub struct SmokeRenderPlugin;

impl Plugin for SmokeRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SmokeMaterial>::default())
            .add_message::<PuffAt>()
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, missile_trails.in_set(SimSet::Mech).after(crate::combat::fly_shots))
            .add_systems(
                Update,
                (jet_trails, spawn_puffs, step_puffs, refresh_light)
                    .chain()
                    .in_set(PresentSet::Pose),
            );
    }
}

/// The puff pool and what it draws with.
#[derive(Resource)]
struct Smoke {
    shapes: Vec<Handle<Mesh>>,
    materials: HashMap<([u16; 3], u8), Handle<SmokeMaterial>>,
    /// Live puffs, oldest first.
    live: VecDeque<Entity>,
    /// Spent puffs, hidden, to be used again.
    free: Vec<Entity>,
    /// Puffs made so far, for picking each one's shape and spin.
    made: u32,
}

/// A puff in the sky.
#[derive(Component)]
struct SmokePuff {
    puff: Puff,
    colour: [f32; 3],
    level: u8,
    spin: Quat,
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let shapes = (0..SHAPES).map(|i| meshes.add(puff_mesh(i * 7 + 3))).collect();
    commands.insert_resource(Smoke {
        shapes,
        materials: HashMap::new(),
        live: VecDeque::new(),
        free: Vec::new(),
        made: 0,
    });
}

/// A unit-diameter lumpy blob (the clouds' lumps, `clouds_render::lump`), with smooth normals.
fn puff_mesh(seed: u32) -> Mesh {
    let unit = Sphere::new(1.0).mesh().ico(DETAIL).expect("an icosphere");
    let (Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)), Some(Indices::U32(idx))) =
        (unit.attribute(Mesh::ATTRIBUTE_POSITION).cloned(), unit.indices().cloned())
    else {
        return Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    };
    let positions: Vec<[f32; 3]> = pos
        .iter()
        .map(|v| {
            let d = Vec3::from_array(*v).normalize_or(Vec3::Y);
            (d * 0.5 * (0.8 + 0.4 * lump(d, seed))).to_array()
        })
        .collect();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(idx));
    mesh.compute_smooth_normals();
    mesh
}

/// The opacity step for an opacity, 0 hidden.
fn level(opacity: f32) -> u8 {
    (opacity.clamp(0.0, 1.0) * (LEVELS - 1) as f32).ceil() as u8
}

fn colour_key(c: [f32; 3]) -> [u16; 3] {
    c.map(|v| (v.clamp(0.0, 4.0) * 1000.0).round() as u16)
}

/// The shared material for a colour and an opacity step, made the first time it is asked for.
fn material(
    smoke: &mut Smoke,
    materials: &mut Assets<SmokeMaterial>,
    light: CloudUniform,
    colour: [f32; 3],
    level: u8,
) -> Handle<SmokeMaterial> {
    smoke
        .materials
        .entry((colour_key(colour), level))
        .or_insert_with(|| {
            materials.add(SmokeMaterial {
                light,
                tint: Vec4::new(colour[0], colour[1], colour[2], level as f32 / (LEVELS - 1) as f32),
            })
        })
        .clone()
}

/// Puts each asked-for puff in the sky, from the free pool or a new entity; past `max_puffs` the
/// oldest is used again.
fn spawn_puffs(
    mut commands: Commands,
    mut asks: MessageReader<PuffAt>,
    data: Res<Data>,
    smoke: Option<ResMut<Smoke>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
) {
    let Some(mut smoke) = smoke else { return };
    let max = data.0.fx.puffs.max_puffs as usize;
    let light = smoke
        .materials
        .values()
        .next()
        .and_then(|h| materials.get(h))
        .map(|m| m.light)
        .unwrap_or_default();
    for ask in asks.read() {
        if !ask.at.is_finite() || !ask.vel.is_finite() {
            continue;
        }
        let i = smoke.made;
        smoke.made = smoke.made.wrapping_add(1);
        let shape = smoke.shapes[(i % SHAPES) as usize].clone();
        let spin = Quat::from_euler(EulerRot::YXZ, i as f32 * 2.399, i as f32 * 1.13, i as f32 * 0.71);
        let mat = material(&mut smoke, &mut materials, light, ask.colour, 1);
        let puff = SmokePuff {
            puff: Puff::new(ask.at, ask.vel, ask.spec),
            colour: ask.colour,
            level: 1,
            spin,
        };
        let size = ask.spec.size_m;
        let at = Transform::from_translation(ask.at)
            .with_rotation(spin)
            .with_scale(Vec3::splat(size));
        let reuse = smoke
            .free
            .pop()
            .or_else(|| if smoke.live.len() >= max { smoke.live.pop_front() } else { None });
        let e = match reuse {
            Some(e) => {
                commands
                    .entity(e)
                    .insert((puff, at, Mesh3d(shape), MeshMaterial3d(mat), Visibility::Inherited));
                e
            }
            None => commands
                .spawn((
                    Name::new("smoke puff"),
                    puff,
                    at,
                    Mesh3d(shape),
                    MeshMaterial3d(mat),
                    Visibility::Inherited,
                    NotShadowCaster,
                ))
                .id(),
        };
        smoke.live.push_back(e);
    }
}

/// Every puff lives a step: it grows, rises, drifts and fades, taking the material of its opacity
/// step; a spent puff is hidden and freed.
fn step_puffs(
    time: Res<Time>,
    plan: Res<Plan>,
    smoke: Option<ResMut<Smoke>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    mut q: Query<(&mut SmokePuff, &mut Transform, &mut MeshMaterial3d<SmokeMaterial>, &mut Visibility)>,
) {
    let Some(mut smoke) = smoke else { return };
    let dt = time.delta_secs();
    let wind = Vec3::new(plan.clouds.wind_m_s.x, 0.0, plan.clouds.wind_m_s.y);
    let light = smoke
        .materials
        .values()
        .next()
        .and_then(|h| materials.get(h))
        .map(|m| m.light)
        .unwrap_or_default();
    let live: Vec<Entity> = smoke.live.iter().copied().collect();
    let mut spent = Vec::new();
    for e in live {
        let Ok((mut p, mut t, mut m, mut v)) = q.get_mut(e) else {
            spent.push(e);
            continue;
        };
        p.puff.step(dt, wind);
        if p.puff.spent() {
            *v = Visibility::Hidden;
            spent.push(e);
            continue;
        }
        *t = Transform::from_translation(p.puff.pos)
            .with_rotation(p.spin)
            .with_scale(Vec3::splat(p.puff.size_m()));
        let l = level(p.puff.opacity());
        if l != p.level {
            p.level = l;
            if l == 0 {
                *v = Visibility::Hidden;
            } else {
                *v = Visibility::Inherited;
                m.0 = material(&mut smoke, &mut materials, light, p.colour, l);
            }
        }
    }
    if !spent.is_empty() {
        smoke.live.retain(|e| !spent.contains(e));
        smoke.free.extend(spent);
    }
}

/// The scene's light into every smoke material.
fn refresh_light(
    data: Res<Data>,
    ambient: Res<GlobalAmbientLight>,
    sun: Query<(&DirectionalLight, &GlobalTransform)>,
    camera: Query<Option<&Exposure>, With<PilotCamera>>,
    smoke: Option<Res<Smoke>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
) {
    let Some(smoke) = smoke else { return };
    let exposure = camera.iter().next().flatten().copied().unwrap_or_default().exposure();
    let light = scene_light(&data, &ambient, sun.iter().next(), exposure, NEAR_FADE_M);
    for h in smoke.materials.values() {
        if let Some(mut m) = materials.get_mut(h)
            && m.light.sun_dir != light.sun_dir
        {
            m.light = light;
        }
    }
}

/// Each mech lifting on its rockets lays a trail from between its jets, `offset_m` under them.
fn jet_trails(
    data: Res<Data>,
    mechs: Query<(Entity, &MechSim, &MechRig)>,
    globals: Query<&GlobalTransform>,
    mut laid: Local<HashMap<Entity, (Vec3, f32)>>,
    mut out: MessageWriter<PuffAt>,
) {
    let t = &data.0.fx.puffs.jet;
    for (e, sim, rig) in &mechs {
        let at = ["JET_L", "JET_R"]
            .iter()
            .filter_map(|s| rig.socket(s).and_then(|j| globals.get(j).ok()).map(|g| g.translation()))
            .fold((Vec3::ZERO, 0.0), |(sum, n), p| (sum + p, n + 1.0));
        if !(sim.curr.boosting && !sim.dead) || at.1 == 0.0 {
            laid.remove(&e);
            continue;
        }
        let now = at.0 / at.1 - Vec3::Y * t.offset_m;
        let throw = Vec3::NEG_Y * 2.0;
        match laid.get_mut(&e) {
            Some((last, walked)) => {
                for p in lay_along(*last, now, t.spacing_m, walked, MAX_STEP_M) {
                    out.write(PuffAt::trail(t, p, throw));
                }
                *last = now;
            }
            None => {
                out.write(PuffAt::trail(t, now, throw));
                laid.insert(e, (now, 0.0));
            }
        }
    }
}

/// Every rocket in flight lays a trail behind it: the puffs every `spacing_m` along the path it
/// flew this step, found from how far it has flown (its age times its speed), so no shot needs an id.
fn missile_trails(data: Res<Data>, shots: Res<Shots>, time: Res<Time<Fixed>>, mut out: MessageWriter<PuffAt>) {
    let t = &data.0.fx.puffs.missile;
    let dt = time.delta_secs();
    for shot in shots.0.iter().filter(|s| s.rocket) {
        let speed = shot.vel.length();
        if speed.is_nan() || speed <= 0.0 || !shot.pos.is_finite() {
            continue;
        }
        let dir = shot.vel / speed;
        let flown = shot.age_s * speed;
        let before = (flown - speed * dt).max(0.0);
        let first = (before / t.spacing_m).floor() as i64 + 1;
        let last = (flown / t.spacing_m).floor() as i64;
        for k in first..=last {
            let back = flown - k as f32 * t.spacing_m + t.offset_m;
            out.write(PuffAt::trail(t, shot.pos - dir * back, -dir * 0.5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_puff_takes_a_lower_step_as_it_fades_and_none_when_gone() {
        assert_eq!(level(1.0), LEVELS - 1);
        assert_eq!(level(0.0), 0);
        assert!(level(0.3) < level(0.8));
        assert_eq!(level(0.001), 1, "a trace of smoke still shows");
    }

    #[test]
    fn a_puff_mesh_is_a_closed_blob_about_one_metre_across() {
        let m = puff_mesh(3);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("positions")
        };
        let r = pos.iter().map(|p| Vec3::from_array(*p).length());
        let (lo, hi) = r.fold((f32::MAX, 0.0f32), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert!(lo > 0.35 && hi < 0.65, "radius {lo}..{hi}");
        assert!(m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }
}
