//! Visual effects: billboard particles (spray, steam, dirt, dust, smoke, sparks, fire, jet flame),
//! ripple rings on water, explosion flashes and falling debris. Presentation only: nothing here
//! decides an outcome, so it may use its own cheap random numbers. Other plugins ask for effects
//! by writing `Burst`, `Ripple` and `Flash` messages.

use crate::pilot::PilotCamera;
use bevy::prelude::*;

/// What a particle looks like and how it moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FxKind {
    /// White water thrown up by a foot or a shell.
    Spray,
    /// Rising white steam.
    Steam,
    /// Brown clods thrown by a blast.
    Dirt,
    /// Tan dust.
    Dust,
    /// Dark rising smoke.
    Smoke,
    /// Bright sparks.
    Spark,
    /// Orange fire.
    Fire,
    /// Blue-white jet flame.
    Flame,
    /// A shot's glowing streak.
    Tracer,
}

impl FxKind {
    const ALL: [FxKind; 8] = [
        FxKind::Spray,
        FxKind::Steam,
        FxKind::Dirt,
        FxKind::Dust,
        FxKind::Smoke,
        FxKind::Spark,
        FxKind::Fire,
        FxKind::Flame,
    ];

    /// (gravity m/s2, drag per s, growth per s as a share of size, colour, emissive).
    fn motion(self) -> (f32, f32, f32, Color, bool) {
        match self {
            FxKind::Spray => (9.81, 0.6, 0.8, Color::srgba(0.85, 0.92, 0.95, 0.75), false),
            FxKind::Steam => (-1.5, 1.2, 1.6, Color::srgba(0.95, 0.95, 0.95, 0.45), false),
            FxKind::Dirt => (9.81, 0.3, 0.0, Color::srgb(0.30, 0.22, 0.14), false),
            FxKind::Dust => (-0.3, 1.5, 1.2, Color::srgba(0.62, 0.55, 0.44, 0.5), false),
            FxKind::Smoke => (-2.5, 0.8, 0.9, Color::srgba(0.16, 0.15, 0.15, 0.7), false),
            FxKind::Spark => (9.81, 0.2, -0.5, Color::srgb(1.0, 0.75, 0.3), true),
            FxKind::Fire => (-4.0, 1.5, 0.4, Color::srgba(1.0, 0.45, 0.1, 0.85), true),
            FxKind::Flame => (0.0, 3.0, 2.0, Color::srgba(0.7, 0.85, 1.0, 0.85), true),
            FxKind::Tracer => (0.0, 0.0, 0.0, Color::srgb(1.0, 0.85, 0.45), true),
        }
    }
}

/// A burst of particles.
#[derive(Message, Clone, Copy, Debug)]
pub struct Burst {
    /// Kind.
    pub kind: FxKind,
    /// Where.
    pub at: Vec3,
    /// Mean direction of travel (normalised or zero).
    pub dir: Vec3,
    /// Particles.
    pub count: u32,
    /// Mean speed.
    pub speed_m_s: f32,
    /// Cone spread, 0 straight to 1 a hemisphere.
    pub spread: f32,
    /// Particle size.
    pub size_m: f32,
    /// Lifetime.
    pub life_s: f32,
}

/// A ripple ring on water.
#[derive(Message, Clone, Copy, Debug)]
pub struct Ripple {
    /// Centre, on the water surface.
    pub at: Vec3,
    /// Final radius.
    pub radius_m: f32,
}

/// A flash of light (an explosion).
#[derive(Message, Clone, Copy, Debug)]
pub struct Flash {
    /// Where.
    pub at: Vec3,
    /// Peak intensity, lumens.
    pub lumens: f32,
}

#[derive(Component)]
struct Particle {
    vel: Vec3,
    age: f32,
    life: f32,
    gravity: f32,
    drag: f32,
    grow: f32,
    size: f32,
}

#[derive(Component)]
struct Ring {
    age: f32,
    radius: f32,
}

#[derive(Component)]
struct FlashLight {
    age: f32,
    lumens: f32,
}

#[derive(Resource)]
struct FxAssets {
    quad: Handle<Mesh>,
    ring: Handle<Mesh>,
    ring_mat: Handle<StandardMaterial>,
    mats: Vec<(FxKind, Handle<StandardMaterial>)>,
}

#[derive(Resource, Default)]
struct Rand(u64);

impl Rand {
    fn f(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / (1u64 << 24) as f32
    }
    fn signed(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
}

/// Most particles alive at once.
const MAX_PARTICLES: usize = 5000;

/// Effects.
pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Burst>()
            .add_message::<Ripple>()
            .add_message::<Flash>()
            .insert_resource(Rand(0x5EED))
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (spawn_bursts, spawn_ripples, spawn_flashes, move_particles, grow_rings, fade_flashes),
            );
    }
}

/// A soft round sprite with a little breakup at the edge, so particles read as puffs, not squares.
fn soft_disc(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: usize = 64;
    let mut data = Vec::with_capacity(N * N * 4);
    for j in 0..N {
        for i in 0..N {
            let (u, v) = (i as f32 / (N - 1) as f32 * 2.0 - 1.0, j as f32 / (N - 1) as f32 * 2.0 - 1.0);
            let r = (u * u + v * v).sqrt();
            let wobble = ((u * 11.0).sin() * (v * 9.0).cos()).abs() * 0.12;
            let a = ((1.0 - r - wobble).clamp(0.0, 1.0) * 1.6).min(1.0).powf(1.5);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let disc = soft_disc(&mut images);
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let ring = meshes.add(Annulus::new(0.85, 1.0));
    let ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.96, 1.0, 0.45),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let mats = FxKind::ALL
        .iter()
        .map(|k| {
            let (_, _, _, color, glow) = k.motion();
            let lin = color.to_linear();
            (
                *k,
                materials.add(StandardMaterial {
                    base_color: color,
                    base_color_texture: Some(disc.clone()),
                    emissive: if glow { lin * 6.0 } else { LinearRgba::BLACK },
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                }),
            )
        })
        .collect();
    commands.insert_resource(FxAssets {
        quad,
        ring,
        ring_mat,
        mats,
    });
}

fn spawn_bursts(
    mut commands: Commands,
    mut bursts: MessageReader<Burst>,
    fx: Res<FxAssets>,
    mut rng: ResMut<Rand>,
    alive: Query<(), With<Particle>>,
) {
    let mut room = MAX_PARTICLES.saturating_sub(alive.iter().count());
    for b in bursts.read() {
        let (gravity, drag, grow, _, _) = b.kind.motion();
        let mat = fx
            .mats
            .iter()
            .find(|(k, _)| *k == b.kind)
            .map(|(_, m)| m.clone())
            .unwrap_or_default();
        let n = (b.count as usize).min(room);
        room -= n;
        for _ in 0..n {
            let jitter = Vec3::new(rng.signed(), rng.signed(), rng.signed()) * b.spread;
            let dir = (b.dir + jitter).normalize_or(Vec3::Y);
            let speed = b.speed_m_s * (0.5 + rng.f());
            let size = b.size_m * (0.6 + 0.8 * rng.f());
            commands.spawn((
                Mesh3d(fx.quad.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(b.at + jitter * 0.5).with_scale(Vec3::splat(size)),
                bevy::light::NotShadowCaster,
                Particle {
                    vel: dir * speed,
                    age: 0.0,
                    life: b.life_s * (0.6 + 0.8 * rng.f()),
                    gravity,
                    drag,
                    grow,
                    size,
                },
            ));
        }
    }
}

fn spawn_ripples(mut commands: Commands, mut ripples: MessageReader<Ripple>, fx: Res<FxAssets>) {
    for r in ripples.read() {
        commands.spawn((
            Mesh3d(fx.ring.clone()),
            MeshMaterial3d(fx.ring_mat.clone()),
            Transform::from_translation(r.at + Vec3::Y * 0.05)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(0.5)),
            bevy::light::NotShadowCaster,
            Ring {
                age: 0.0,
                radius: r.radius_m,
            },
        ));
    }
}

fn spawn_flashes(mut commands: Commands, mut flashes: MessageReader<Flash>) {
    for f in flashes.read() {
        commands.spawn((
            PointLight {
                intensity: f.lumens,
                range: 300.0,
                color: Color::srgb(1.0, 0.7, 0.4),
                ..default()
            },
            Transform::from_translation(f.at + Vec3::Y * 3.0),
            FlashLight {
                age: 0.0,
                lumens: f.lumens,
            },
        ));
    }
}

fn move_particles(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut q: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    let dt = time.delta_secs();
    let face = camera.rotation();
    let eye = camera.translation();
    for (e, mut t, mut p) in &mut q {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(e).despawn();
            continue;
        }
        let drag = p.drag;
        p.vel.y -= p.gravity * dt;
        p.vel *= (1.0 - drag * dt).max(0.0);
        t.translation += p.vel * dt;
        let k = p.age / p.life;
        // Grow, then shrink away over the last third of life.
        let fade = if k > 0.66 { 1.0 - (k - 0.66) / 0.34 } else { 1.0 };
        let size = (p.size * (1.0 + p.grow * p.age)).max(0.01) * fade.max(0.02);
        // A particle about to swallow the eye is hidden: smoke or dust against the camera would
        // blank the whole view (the stump burns beside the cockpit).
        let clear = ((t.translation.distance(eye) - size * 0.5) / 4.0).clamp(0.0, 1.0);
        t.scale = Vec3::splat(size * clear.max(0.001));
        t.rotation = face;
    }
}

fn grow_rings(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Transform, &mut Ring)>) {
    for (e, mut t, mut r) in &mut q {
        r.age += time.delta_secs();
        if r.age > 2.5 {
            commands.entity(e).despawn();
            continue;
        }
        let k = r.age / 2.5;
        t.scale = Vec3::splat(0.5 + r.radius * k.sqrt()) * Vec3::new(1.0, 1.0, 1.0 - k * 0.9);
    }
}

fn fade_flashes(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut PointLight, &mut FlashLight)>) {
    for (e, mut l, mut f) in &mut q {
        f.age += time.delta_secs();
        if f.age > 0.6 {
            commands.entity(e).despawn();
        } else {
            l.intensity = f.lumens * (1.0 - f.age / 0.6).powi(2);
        }
    }
}
