//! Soft smoke sprites (openspec change `particle-depth`, variant C; owner, 2026-10-25: "lots of zfighting on
//! distant smoke still", "the softer ones are alright, but still suffer from zbuffer issues"). bevy_hanabi
//! draws an effect's particles in the order they were spawned and writes no depth for blended ones, with no
//! fade where a sprite meets the world: overlapping smoke flipped as the view turned, and a puff crossing the
//! ground or a wall was cut along a straight line. The blended smoke kinds (smoke, trail, dust, steam, mist)
//! are drawn here instead, the clouds' way (`clouds_render`): every live particle a quad of one mesh, sorted
//! back to front along the view each frame, faded in front of the depth prepass (`soft_sprite.wesl`).
//! bevy_hanabi keeps the rest (sparks, fire, flame, dirt, debris, rubble, spray).
//!
//! A particle's life (thrown, pulled, slowed, growing, its colour over its life) reads the same tables
//! hanabi's effects are built from (`particles::motion`, `particles::colour_over_life`), so the two
//! renderers cannot drift. The sheet is mapped the right way up: hanabi's quad runs its texture rows from the
//! bottom, so the smoke sheet, painted lit from above, drew lit from below (owner, 2026-10-25: "why are the smoke
//! darker at top?", "it lookslike inverted lighting to me"). `CB_HANABI_SMOKE=1` sends the smoke back to
//! bevy_hanabi, for the A/B.

use crate::particles::{GRID, colour_over_life, motion};
use crate::pilot::PilotCamera;
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexFormat};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor, ShaderType,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use cb_core::fx::ParticleKind;

const SHADER: &str = "shaders/soft_sprite.wesl";

/// Size, spin, sheet cell, sheet.
const ATTRIBUTE_SHAPE: MeshVertexAttribute = MeshVertexAttribute::new("SoftShape", 0x50F7_5A01, VertexFormat::Float32x4);
/// Colour over life.
const ATTRIBUTE_COLOUR: MeshVertexAttribute = MeshVertexAttribute::new("SoftColour", 0x50F7_5A02, VertexFormat::Float32x4);

/// A sprite fades in over this far from the eye, metres (so smoke round the cockpit does not blank it).
const NEAR_FADE_M: f32 = 2.0;

/// Whether `kind` is drawn here rather than by bevy_hanabi: the blended smoke kinds, unless
/// `CB_HANABI_SMOKE=1` sends them back for the A/B.
pub fn soft(kind: ParticleKind) -> bool {
    matches!(
        kind,
        ParticleKind::Smoke | ParticleKind::Trail | ParticleKind::Dust | ParticleKind::Steam | ParticleKind::Mist
    ) && !crate::cli::switched_off("CB_HANABI_SMOKE")
}

/// The numbers a soft particle is started from (a recipe's `Burst` layer).
#[derive(Clone, Copy, Debug)]
pub struct SoftSpec {
    /// Which particle.
    pub kind: ParticleKind,
    /// Starting speed, m/s.
    pub speed_m_s: f32,
    /// How far from the direction it may be thrown, 0..1 and over.
    pub spread: f32,
    /// Starting size, metres.
    pub size_m: f32,
    /// Life, seconds.
    pub life_s: f32,
}

/// One live particle.
#[derive(Clone, Copy, Debug)]
struct Part {
    pos: Vec3,
    vel: Vec3,
    age_s: f32,
    life_s: f32,
    size0_m: f32,
    spin_rad: f32,
    cell: f32,
    kind: ParticleKind,
    /// Over the cap, the age it began fading out at (`CAP_FADE_S`); never, otherwise.
    fade_from_s: f32,
    /// Its own colour (linear, HDR), in place of its kind's; its kind's curve still fades it in and out.
    tint: Option<Vec3>,
}

/// The live soft particles, oldest first.
#[derive(Resource, Default)]
pub struct SoftSprites {
    parts: Vec<Part>,
    rng: u64,
}

impl SoftSprites {
    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 40) as f32) / (1u64 << 24) as f32
    }

    /// Starts `count` particles of `spec` at `at`, thrown along `dir`, at `scale`, as `emit` does, each its own colour
    /// `tint` (linear, HDR) with its kind's fade in and out: a nuclear cloud's puffs, coloured by its heat (`nuke`).
    pub fn emit_tinted(&mut self, spec: SoftSpec, count: u32, at: Vec3, dir: Vec3, scale: f32, tint: Vec3) {
        let from = self.parts.len();
        self.emit(spec, count, at, dir, scale);
        for p in &mut self.parts[from..] {
            p.tint = Some(tint);
        }
    }

    /// Starts `count` particles of `spec` at `at`, thrown along `dir`, at `scale`, as a hanabi burst of
    /// the same layer would (the same jitter, speed, life and size spreads).
    pub fn emit(&mut self, spec: SoftSpec, count: u32, at: Vec3, dir: Vec3, scale: f32) {
        let turn = Quat::from_rotation_arc(Vec3::Y, dir.normalize_or(Vec3::Y));
        let scale = scale.max(0.01);
        for _ in 0..count {
            let jitter = Vec3::new(self.rand(), self.rand(), self.rand()) * 2.0 - Vec3::ONE;
            let jitter = jitter * spec.spread;
            let local_pos = jitter * 0.5 * spec.size_m.min(1.0);
            let local_dir = (Vec3::Y + jitter).normalize_or(Vec3::Y);
            let speed = spec.speed_m_s * (0.5 + self.rand());
            let life = spec.life_s * (0.6 + 0.8 * self.rand());
            let size0 = spec.size_m * scale * (0.6 + 0.8 * self.rand());
            let spin = (self.rand() - 0.5) * 0.6;
            let cell = (self.rand() * (GRID * GRID) as f32).floor().min((GRID * GRID - 1) as f32);
            self.parts.push(Part {
                pos: at + turn * local_pos * scale,
                vel: turn * local_dir * speed * scale,
                age_s: 0.0,
                life_s: life.max(0.01),
                size0_m: size0,
                spin_rad: spin,
                cell,
                kind: spec.kind,
                fade_from_s: f32::INFINITY,
                tint: None,
            });
        }
    }
}

/// A steady stream of soft particles riding an entity (a jet's trail, a wreck's smoke): `rate` a second
/// while `on`, laid evenly along the path the entity moved this frame so a fast emitter leaves a line,
/// not clumps.
#[derive(Component)]
pub struct SoftEmitter {
    /// What it emits.
    pub spec: SoftSpec,
    /// Particles a second.
    pub rate: f32,
    /// Emitting now.
    pub on: bool,
    /// Particles owed from earlier frames.
    carry: f32,
    /// Where it was last frame.
    last: Option<Vec3>,
}

impl SoftEmitter {
    /// An emitter of `spec` at `rate` a second, on.
    pub fn new(spec: SoftSpec, rate: f32) -> Self {
        Self {
            spec,
            rate,
            on: true,
            carry: 0.0,
            last: None,
        }
    }
}

/// The look's uniform; `SoftLook` in `soft_sprite.wesl`.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
struct SoftLook {
    /// x soft depth against the world (m), y fade-in distance from the eye (m).
    fade: Vec4,
    /// The air between the eye and a sprite, as the clouds take it (`CloudUniform::air`): the ground fog's layer (x
    /// density at sea level per m, y scale height m, z sea level m) and the aerial haze (x density, y scale height). A
    /// far sprite thins away into the haze the fog pass lays over what is behind it (owner, 2026-10-30: "if it's really
    /// far away fog should Apply").
    fog: Vec4,
    haze: Vec4,
}

/// The one soft sprite material: the smoke and dust sheets.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SoftMaterial {
    #[uniform(0)]
    look: SoftLook,
    #[texture(1)]
    #[sampler(2)]
    smoke: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    dust: Handle<Image>,
}

impl Material for SoftMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    // Premultiplied OVER (the shader multiplies colour by alpha): no fringes at a sprite's soft rim.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
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
            ATTRIBUTE_COLOUR.at_shader_location(3),
        ])?];
        descriptor.primitive.cull_mode = None;
        // Colour: premultiplied OVER. Alpha: what of the scene behind still shows through (each sprite
        // multiplies it by 1 - its alpha). Nothing reads that share now: the ground fog is laid before
        // the transparent pass (`fog` in `water_post.wesl`), so the smoke is drawn over fogged ground
        // and carries only its own fog, and the fog over a puff cannot jump where the sky, a far
        // street or a near wall changes behind it (owner, 2026-10-25: "still smoke sorting issues
        // here").
        if let Some(target) = descriptor
            .fragment
            .as_mut()
            .and_then(|f| f.targets.get_mut(0))
            .and_then(|t| t.as_mut())
        {
            target.blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::Zero,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            });
        }
        Ok(())
    }
}

/// Soft smoke sprites.
pub struct SoftSpritesPlugin;

impl Plugin for SoftSpritesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SoftMaterial>::default())
            .init_resource::<SoftSprites>()
            .add_game_systems(Update, (emit_streams, step, draw).chain().in_set(PresentSet::Pose))
            .add_game_systems(Update, refresh_air);
    }
}

/// The drawn sprites.
#[derive(Resource)]
struct SoftDraw {
    mesh: Handle<Mesh>,
    material: Handle<SoftMaterial>,
}

/// Makes the mesh and material once the sheets are loaded (`fx::setup` hands them over).
pub fn make_draw(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<SoftMaterial>,
    smoke: Handle<Image>,
    dust: Handle<Image>,
    soft_m: f32,
) {
    let mesh = meshes.add(sprite_mesh(&[]));
    let material = materials.add(SoftMaterial {
        look: SoftLook {
            fade: Vec4::new(soft_m.max(0.01), NEAR_FADE_M, 0.0, 0.0),
            ..default()
        },
        smoke,
        dust,
    });
    commands.spawn((
        Name::new("soft sprites"),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::default(),
        NotShadowCaster,
        NoFrustumCulling,
    ));
    commands.insert_resource(SoftDraw { mesh, material });
}

/// Runs every stream's emitter.
fn emit_streams(time: Res<Time>, mut sprites: ResMut<SoftSprites>, mut q: Query<(&GlobalTransform, &mut SoftEmitter)>) {
    let dt = time.delta_secs();
    for (g, mut e) in &mut q {
        let now = g.translation();
        if !e.on {
            // Off, it forgets where it was: a pooled emitter handed to a new rocket starts afresh.
            e.carry = 0.0;
            e.last = None;
            continue;
        }
        let from = e.last.unwrap_or(now);
        e.last = Some(now);
        e.carry += e.rate * dt;
        let n = e.carry.floor() as u32;
        e.carry -= n as f32;
        let dir = g.rotation() * Vec3::Y;
        let scale = g.scale().max_element();
        for i in 0..n {
            let at = from.lerp(now, (i as f32 + 1.0) / n as f32);
            sprites.emit(e.spec, 1, at, dir, scale);
        }
    }
}

/// Steps every particle's life and drops the spent ones (and the oldest past the cap).
fn step(time: Res<Time>, data: Res<Data>, mut sprites: ResMut<SoftSprites>) {
    let dt = time.delta_secs();
    let cap = data.0.fx.soft_max_particles as usize;
    for p in &mut sprites.parts {
        let (gravity, drag, _, _) = motion(p.kind);
        p.vel += Vec3::new(0.0, -gravity, 0.0) * dt;
        p.vel -= p.vel * (drag * dt).min(1.0);
        p.pos += p.vel * dt;
        p.age_s += dt;
    }
    sprites.parts.retain(|p| p.age_s < p.life_s);
    // Over the cap, the oldest fade out over `CAP_FADE_S` rather than vanish (owner, 2026-10-30: "smoke despawn should
    // have it out. It looks like it's popping out"): their life is cut to end that soon, and the colour's own fade
    // takes them out.
    let fading = sprites.parts.iter().filter(|p| p.fade_from_s.is_finite()).count();
    let over = sprites.parts.len().saturating_sub(cap + fading);
    for p in sprites.parts.iter_mut().filter(|p| !p.fade_from_s.is_finite()).take(over) {
        p.fade_from_s = p.age_s;
        p.life_s = p.life_s.min(p.age_s + CAP_FADE_S);
    }
}

/// How long a sprite over the cap takes to fade out, seconds.
const CAP_FADE_S: f32 = 1.5;

/// How much of a sprite is left as it fades out over the cap: 1, then down to 0 over `CAP_FADE_S`.
fn cap_fade(p: &Part) -> f32 {
    (1.0 - (p.age_s - p.fade_from_s) / CAP_FADE_S).clamp(0.0, 1.0)
}

/// The air into the material each frame, from the source the fog pass and the clouds read (`AirFog`).
fn refresh_air(
    data: Res<Data>,
    plan: Res<crate::world::Plan>,
    fog_on: Option<Res<crate::water_render::GroundFogOn>>,
    fog_now: Res<crate::ground_fog::FogNow>,
    draw: Option<Res<SoftDraw>>,
    mut materials: ResMut<Assets<SoftMaterial>>,
) {
    let Some(draw) = draw else { return };
    let air = crate::water_render::AirFog::now(&data.0.world.fog, fog_now.strength, plan.waters.sea_level_m, 0.0);
    let (fog, haze) = crate::clouds_render::CloudUniform::air(&air, fog_on.is_some_and(|f| f.0));
    // Only when it changed: a material written every frame is re-uploaded every frame.
    let stale = materials
        .get(&draw.material)
        .is_some_and(|m| m.look.fog != fog || m.look.haze != haze);
    if stale && let Some(mut m) = materials.get_mut(&draw.material) {
        m.look.fog = fog;
        m.look.haze = haze;
    }
}

/// A particle's colour at life share `t`, from its kind's keys.
fn colour_at(kind: ParticleKind, t: f32) -> Vec4 {
    let keys = colour_over_life(kind);
    let mut prev = keys[0];
    for k in keys {
        if t <= k.0 {
            let span = (k.0 - prev.0).max(1e-6);
            return prev.1.lerp(k.1, ((t - prev.0) / span).clamp(0.0, 1.0));
        }
        prev = k;
    }
    prev.1
}

/// One quad: centre, size, spin, cell, sheet, colour.
#[derive(Clone, Copy, Debug)]
struct Quad {
    centre: Vec3,
    shape: Vec4,
    colour: Vec4,
}

/// The quads of `parts` that show, deepest along `forward` from `eye` first: a billboard lies in a plane
/// parallel to the screen, so this is the exact back-to-front order where sprites do not cut each other.
fn sorted_quads(parts: &[Part], eye: Vec3, forward: Vec3) -> Vec<Quad> {
    let forward = forward.normalize_or(Vec3::NEG_Z);
    let mut keyed: Vec<(f32, usize)> = parts
        .iter()
        .enumerate()
        .map(|(i, p)| ((p.pos - eye).dot(forward), i))
        .filter(|(d, i)| *d > -parts[*i].size0_m * 4.0)
        .collect();
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    keyed
        .into_iter()
        .map(|(_, i)| {
            let p = &parts[i];
            let (_, _, grow, _) = motion(p.kind);
            let size = (p.size0_m * (1.0 + grow * p.age_s)).max(0.01);
            let sheet = if p.kind == ParticleKind::Dust { 1.0 } else { 0.0 };
            Quad {
                centre: p.pos,
                shape: Vec4::new(size, p.spin_rad, p.cell, sheet),
                colour: {
                    let c = colour_at(p.kind, p.age_s / p.life_s);
                    p.tint.map_or(c, |t| t.extend(c.w)) * Vec4::new(1.0, 1.0, 1.0, cap_fade(p))
                },
            }
        })
        .filter(|q| q.colour.w > 0.002)
        .collect()
}

fn sprite_mesh(quads: &[Quad]) -> Mesh {
    let n = quads.len() * 4;
    let (mut centre, mut corner, mut shape, mut colour) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let mut indices: Vec<u32> = Vec::with_capacity(quads.len() * 6);
    for (i, q) in quads.iter().enumerate() {
        for c in [[-1.0f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] {
            centre.push(q.centre.to_array());
            corner.push(c);
            shape.push(q.shape.to_array());
            colour.push(q.colour.to_array());
        }
        let b = i as u32 * 4;
        indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, centre)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, corner)
        .with_inserted_attribute(ATTRIBUTE_SHAPE, shape)
        .with_inserted_attribute(ATTRIBUTE_COLOUR, colour)
        .with_inserted_indices(Indices::U32(indices))
}

/// Sorts and uploads every live sprite for this frame's view.
fn draw(
    sprites: Res<SoftSprites>,
    draw: Option<Res<SoftDraw>>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(draw) = draw else { return };
    let _s = crate::perf_hud::section("soft sprites: sort and mesh");
    let quads = sorted_quads(&sprites.parts, camera.translation(), camera.forward().as_vec3());
    if let Some(mut mesh) = meshes.get_mut(&draw.mesh) {
        *mesh = sprite_mesh(&quads);
    }
    let _ = &draw.material;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(at: Vec3) -> Part {
        Part {
            pos: at,
            vel: Vec3::ZERO,
            age_s: 0.5,
            life_s: 2.0,
            size0_m: 2.0,
            spin_rad: 0.0,
            cell: 0.0,
            kind: ParticleKind::Smoke,
            fade_from_s: f32::INFINITY,
            tint: None,
        }
    }

    #[test]
    fn over_the_cap_the_oldest_fade_out_rather_than_vanish() {
        let mut p = part(Vec3::ZERO);
        assert_eq!(cap_fade(&p), 1.0, "not over the cap: whole");
        p.fade_from_s = p.age_s;
        p.age_s += CAP_FADE_S * 0.5;
        assert!((cap_fade(&p) - 0.5).abs() < 1e-5, "half way through its fade: half there");
        p.age_s += CAP_FADE_S;
        assert_eq!(cap_fade(&p), 0.0, "gone once faded");
    }

    #[test]
    fn overlapping_smoke_is_drawn_farthest_first_whichever_was_spawned_first() {
        let near = part(Vec3::new(0.0, 0.0, -10.0));
        let far = part(Vec3::new(0.5, 0.0, -30.0));
        let quads = sorted_quads(&[near, far], Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].centre, far.pos);
        assert_eq!(quads[1].centre, near.pos);
        let turned = sorted_quads(&[far, near], Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(turned[0].centre, far.pos);
    }

    #[test]
    fn a_burst_starts_its_particles_where_hanabi_would_and_they_die_at_their_life() {
        let mut s = SoftSprites::default();
        let spec = SoftSpec {
            kind: ParticleKind::Smoke,
            speed_m_s: 2.0,
            spread: 0.3,
            size_m: 3.0,
            life_s: 1.0,
        };
        s.emit(spec, 10, Vec3::new(5.0, 2.0, 0.0), Vec3::Y, 1.0);
        assert_eq!(s.parts.len(), 10);
        for p in &s.parts {
            assert!(p.pos.distance(Vec3::new(5.0, 2.0, 0.0)) < 1.0);
            assert!(p.life_s >= 0.6 - 1e-4 && p.life_s <= 1.4 + 1e-4);
            assert!(p.vel.y > 0.0);
        }
    }

    #[test]
    fn colour_over_life_runs_from_the_first_key_to_the_last() {
        let first = colour_over_life(ParticleKind::Trail)[0].1;
        let last = colour_over_life(ParticleKind::Trail).last().unwrap().1;
        assert_eq!(colour_at(ParticleKind::Trail, 0.0), first);
        assert_eq!(colour_at(ParticleKind::Trail, 1.0), last);
    }
}
