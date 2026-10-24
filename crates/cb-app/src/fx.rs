//! Visual effects (openspec change `combat-fx`): billboard particles, point-light flashes,
//! fireballs, shock shells, muzzle stars and water rings, played from the recipes in
//! `data/fx/fx.ron`; one glowing streak per shot in flight; a motor flame and light on every rocket.
//! Presentation only: nothing here decides an outcome, so it may use its own cheap random numbers.
//! Other plugins ask for an effect by name with an `Fx` message; tracers read `Shots` directly.

use crate::combat::Shots;
use crate::pilot::PilotCamera;
use crate::sim::Data;
use bevy::prelude::*;
use cb_core::fx::{FxConfig, Layer, ParticleKind};

/// Play the recipe `recipe` at `at`.
#[derive(Message, Clone, Copy, Debug)]
pub struct Fx {
    /// A name from `cb_core::fx::RECIPES`.
    pub recipe: &'static str,
    /// Where.
    pub at: Vec3,
    /// The effect's direction (a barrel, back along a shot, a thrust); normalised or zero.
    pub dir: Vec3,
    /// Multiplies sizes, radii, offsets and speeds; lumens by its cube.
    pub scale: f32,
    /// Multiplies particle counts.
    pub density: f32,
}

impl Fx {
    /// The recipe at a point, pointing up, at scale and density 1.
    pub fn new(recipe: &'static str, at: Vec3) -> Self {
        Self {
            recipe,
            at,
            dir: Vec3::Y,
            scale: 1.0,
            density: 1.0,
        }
    }

    /// Pointing along `dir`.
    pub fn toward(self, dir: Vec3) -> Self {
        Self {
            dir: dir.normalize_or(Vec3::Y),
            ..self
        }
    }

    /// Scaled.
    pub fn scaled(self, scale: f32) -> Self {
        Self { scale, ..self }
    }

    /// With `density` times the particles.
    pub fn dense(self, density: f32) -> Self {
        Self { density, ..self }
    }
}

/// (gravity m/s2, drag per s, growth per s as a share of size, colour, glow).
fn motion(kind: ParticleKind) -> (f32, f32, f32, Color, f32) {
    match kind {
        ParticleKind::Spray => (9.81, 0.6, 0.8, Color::srgba(0.85, 0.92, 0.95, 0.75), 0.0),
        ParticleKind::Steam => (-1.5, 1.2, 1.6, Color::srgba(0.95, 0.95, 0.95, 0.45), 0.0),
        ParticleKind::Dirt => (9.81, 0.3, 0.0, Color::srgb(0.30, 0.22, 0.14), 0.0),
        ParticleKind::Dust => (-0.3, 1.5, 1.2, Color::srgba(0.62, 0.55, 0.44, 0.5), 0.0),
        ParticleKind::Smoke => (-2.5, 0.8, 0.9, Color::srgba(0.16, 0.15, 0.15, 0.7), 0.0),
        ParticleKind::Spark => (9.81, 0.2, -0.5, Color::srgb(1.0, 0.75, 0.3), 6.0),
        ParticleKind::Fire => (-4.0, 1.5, 0.4, Color::srgba(1.0, 0.45, 0.1, 0.85), 6.0),
        ParticleKind::Flame => (0.0, 3.0, 2.0, Color::srgba(0.7, 0.85, 1.0, 0.85), 6.0),
        ParticleKind::Debris => (9.81, 0.3, -0.2, Color::srgb(1.0, 0.55, 0.2), 8.0),
        ParticleKind::Rubble => (9.81, 0.2, 0.0, Color::srgb(0.42, 0.41, 0.39), 0.0),
        ParticleKind::Trail => (-0.1, 0.6, 0.3, Color::srgba(0.78, 0.77, 0.75, 0.55), 0.0),
    }
}

const KINDS: [ParticleKind; 11] = [
    ParticleKind::Spray,
    ParticleKind::Steam,
    ParticleKind::Dirt,
    ParticleKind::Dust,
    ParticleKind::Smoke,
    ParticleKind::Spark,
    ParticleKind::Fire,
    ParticleKind::Flame,
    ParticleKind::Debris,
    ParticleKind::Rubble,
    ParticleKind::Trail,
];

#[derive(Component)]
struct Particle {
    vel: Vec3,
    age: f32,
    life: f32,
    gravity: f32,
    drag: f32,
    grow: f32,
    size: f32,
    /// Burning debris drops a smoke puff every this long.
    smokes_every_s: Option<f32>,
    smoke_in_s: f32,
}

#[derive(Component)]
struct Ring {
    age: f32,
    life: f32,
    radius: f32,
}

#[derive(Component)]
struct FlashLight {
    age: f32,
    life: f32,
    lumens: f32,
}

#[derive(Component)]
struct Fireball {
    age: f32,
    grow: f32,
    life: f32,
    radius: f32,
    mat: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Shell {
    age: f32,
    life: f32,
    radius: f32,
    mat: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Star {
    age: f32,
    life: f32,
    size: f32,
}

/// A shot's streak, or a rocket's motor (flame and light), by slot in `Shots` order.
#[derive(Component)]
struct Tracer;

#[derive(Component)]
struct Motor;

#[derive(Resource)]
struct FxAssets {
    quad: Handle<Mesh>,
    ring: Handle<Mesh>,
    sphere: Handle<Mesh>,
    water_ring: Handle<StandardMaterial>,
    dust_ring: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
    mats: Vec<(ParticleKind, Handle<StandardMaterial>)>,
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

/// The tracer and motor pools, in `Shots` order.
#[derive(Resource, Default)]
struct Pools {
    tracers: Vec<Entity>,
    motors: Vec<Entity>,
}

/// Effects.
pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Fx>()
            .insert_resource(Rand(0x5EED))
            .init_resource::<Pools>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    play,
                    draw_shots,
                    move_particles,
                    grow_rings,
                    fade_flashes,
                    cool_fireballs,
                    expand_shells,
                    fade_stars,
                ),
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

fn unlit(color: Color, texture: Option<Handle<Image>>, alpha_mode: AlphaMode, glow: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        base_color_texture: texture,
        emissive: color.to_linear() * glow,
        alpha_mode,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let disc = soft_disc(&mut images);
    let mats = KINDS
        .iter()
        .map(|k| {
            let (_, _, _, color, glow) = motion(*k);
            let mode = if glow > 0.0 { AlphaMode::Add } else { AlphaMode::Blend };
            (*k, materials.add(unlit(color, Some(disc.clone()), mode, glow)))
        })
        .collect();
    commands.insert_resource(FxAssets {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        ring: meshes.add(Annulus::new(0.85, 1.0)),
        sphere: meshes.add(Sphere::new(1.0).mesh().uv(32, 16)),
        water_ring: materials.add(unlit(Color::srgba(0.9, 0.96, 1.0, 0.45), None, AlphaMode::Blend, 0.0)),
        dust_ring: materials.add(unlit(Color::srgba(0.62, 0.55, 0.44, 0.35), None, AlphaMode::Blend, 0.0)),
        glow: materials.add(unlit(Color::srgb(1.0, 0.8, 0.45), Some(disc.clone()), AlphaMode::Add, 8.0)),
        mats,
    });
}

fn kind_mat(fx: &FxAssets, kind: ParticleKind) -> Handle<StandardMaterial> {
    fx.mats
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, m)| m.clone())
        .unwrap_or_else(|| fx.glow.clone())
}

/// Spawns `n` particles of `kind` (cut to the room left).
#[allow(clippy::too_many_arguments)]
fn spawn_particles(
    commands: &mut Commands,
    fx: &FxAssets,
    rng: &mut Rand,
    kind: ParticleKind,
    at: Vec3,
    dir: Vec3,
    n: usize,
    speed_m_s: f32,
    spread: f32,
    size_m: f32,
    life_s: f32,
) {
    let (gravity, drag, grow, _, _) = motion(kind);
    let mat = kind_mat(fx, kind);
    for _ in 0..n {
        let jitter = Vec3::new(rng.signed(), rng.signed(), rng.signed()) * spread;
        let d = (dir + jitter).normalize_or(Vec3::Y);
        let size = size_m * (0.6 + 0.8 * rng.f());
        commands.spawn((
            Mesh3d(fx.quad.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(at + jitter * 0.5 * size_m.min(1.0)).with_scale(Vec3::splat(size)),
            bevy::light::NotShadowCaster,
            Particle {
                vel: d * speed_m_s * (0.5 + rng.f()),
                age: 0.0,
                life: life_s * (0.6 + 0.8 * rng.f()),
                gravity,
                drag,
                grow,
                size,
                smokes_every_s: (kind == ParticleKind::Debris).then_some(0.08),
                smoke_in_s: 0.0,
            },
        ));
    }
}

/// Plays every recipe asked for this frame, layer by layer, within the caps.
#[allow(clippy::too_many_arguments)]
fn play(
    mut commands: Commands,
    mut asked: MessageReader<Fx>,
    data: Res<Data>,
    fx: Res<FxAssets>,
    mut rng: ResMut<Rand>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    particles: Query<(), With<Particle>>,
    lights: Query<(Entity, &FlashLight)>,
    fireballs: Query<(), With<Fireball>>,
) {
    let cfg: &FxConfig = &data.0.fx;
    let mut room = (cfg.max_particles as usize).saturating_sub(particles.iter().count());
    let mut fireball_room = (cfg.max_fireballs as usize).saturating_sub(fireballs.iter().count());
    // Lights alive, oldest first: a new light past the cap puts out the oldest.
    let mut live_lights: Vec<(Entity, f32)> = lights.iter().map(|(e, l)| (e, l.age)).collect();
    live_lights.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut light_at = 0;
    for e in asked.read() {
        let Some(layers) = cfg.recipes.get(e.recipe) else {
            warn!("fx: no recipe `{}`", e.recipe);
            continue;
        };
        let s = e.scale.max(0.01);
        for layer in layers {
            match *layer {
                Layer::Burst {
                    kind,
                    count,
                    speed_m_s,
                    spread,
                    size_m,
                    life_s,
                    offset_m,
                    along_dir,
                } => {
                    let n = ((count as f32 * e.density).round() as usize).min(room);
                    room -= n;
                    let dir = if along_dir { e.dir } else { Vec3::Y };
                    let at = e.at + e.dir * offset_m * s;
                    spawn_particles(
                        &mut commands,
                        &fx,
                        &mut rng,
                        kind,
                        at,
                        dir,
                        n,
                        speed_m_s * s,
                        spread,
                        size_m * s,
                        life_s,
                    );
                }
                Layer::Light {
                    lumens,
                    range_m,
                    life_s,
                    colour,
                    lift_m,
                } => {
                    if live_lights.len() - light_at >= cfg.max_lights as usize {
                        if let Some((old, _)) = live_lights.get(light_at) {
                            commands.entity(*old).despawn();
                        }
                        light_at += 1;
                    }
                    let lm = lumens * s * s * s;
                    let id = commands
                        .spawn((
                            PointLight {
                                intensity: lm,
                                range: range_m * s,
                                color: Color::srgb(colour[0], colour[1], colour[2]),
                                ..default()
                            },
                            Transform::from_translation(e.at + Vec3::Y * lift_m * s),
                            FlashLight {
                                age: 0.0,
                                life: life_s,
                                lumens: lm,
                            },
                        ))
                        .id();
                    live_lights.push((id, 0.0));
                }
                Layer::Fireball { radius_m, grow_s, life_s } => {
                    if fireball_room == 0 {
                        continue;
                    }
                    fireball_room -= 1;
                    // Its own material: its colour cools as it lives.
                    let mat = materials.add(unlit(Color::WHITE, None, AlphaMode::Add, 10.0));
                    commands.spawn((
                        Mesh3d(fx.sphere.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation(e.at).with_scale(Vec3::splat(0.1)),
                        bevy::light::NotShadowCaster,
                        Fireball {
                            age: 0.0,
                            grow: grow_s,
                            life: life_s,
                            radius: radius_m * s,
                            mat,
                        },
                    ));
                }
                Layer::Shock { radius_m, life_s } => {
                    let mat = materials.add(unlit(Color::srgba(0.95, 0.92, 0.85, 0.25), None, AlphaMode::Blend, 0.0));
                    commands.spawn((
                        Mesh3d(fx.sphere.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation(e.at).with_scale(Vec3::splat(0.1)),
                        bevy::light::NotShadowCaster,
                        Shell {
                            age: 0.0,
                            life: life_s,
                            radius: radius_m * s,
                            mat,
                        },
                    ));
                    spawn_ring(&mut commands, &fx, fx.dust_ring.clone(), e.at, radius_m * s * 1.5, life_s * 4.0);
                }
                Layer::Star { size_m, life_s, offset_m } => {
                    let size = size_m * s;
                    let axis = e.dir.normalize_or(Vec3::Y);
                    let base = Quat::from_rotation_arc(Vec3::Y, axis);
                    let spin = rng.f() * std::f32::consts::TAU;
                    commands
                        .spawn((
                            Transform::from_translation(e.at + axis * (offset_m * s + size * 0.5)).with_rotation(base),
                            Visibility::default(),
                            Star {
                                age: 0.0,
                                life: life_s,
                                size,
                            },
                        ))
                        .with_children(|p| {
                            // Three blades round the barrel's axis, and a round glow across it.
                            for k in 0..3 {
                                let r = Quat::from_rotation_y(spin + k as f32 * std::f32::consts::FRAC_PI_3);
                                p.spawn((
                                    Mesh3d(fx.quad.clone()),
                                    MeshMaterial3d(fx.glow.clone()),
                                    Transform::from_rotation(r).with_scale(Vec3::new(0.35, 1.0, 1.0)),
                                    bevy::light::NotShadowCaster,
                                ));
                            }
                            p.spawn((
                                Mesh3d(fx.quad.clone()),
                                MeshMaterial3d(fx.glow.clone()),
                                Transform::from_translation(Vec3::Y * -0.4)
                                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                                    .with_scale(Vec3::splat(0.7)),
                                bevy::light::NotShadowCaster,
                            ));
                        });
                }
                Layer::Ripple { radius_m } => {
                    spawn_ring(&mut commands, &fx, fx.water_ring.clone(), e.at, radius_m * s, 2.5);
                }
            }
        }
    }
}

fn spawn_ring(commands: &mut Commands, fx: &FxAssets, mat: Handle<StandardMaterial>, at: Vec3, radius: f32, life: f32) {
    commands.spawn((
        Mesh3d(fx.ring.clone()),
        MeshMaterial3d(mat),
        Transform::from_translation(at + Vec3::Y * 0.05)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(0.5)),
        bevy::light::NotShadowCaster,
        Ring { age: 0.0, life, radius },
    ));
}

/// One streak per shell in flight and one motor per rocket, placed where the shot is this frame
/// (its last fixed step carried on by the overstep). A pool slot past the number of shots is hidden,
/// so a streak is gone the frame its shot stops.
#[allow(clippy::too_many_arguments)]
fn draw_shots(
    mut commands: Commands,
    shots: Res<Shots>,
    data: Res<Data>,
    fx: Res<FxAssets>,
    fixed: Res<Time<Fixed>>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut pools: ResMut<Pools>,
    mut q: Query<(&mut Transform, &mut Visibility), Or<(With<Tracer>, With<Motor>)>>,
    mechs: Query<(&crate::mech::MechId, &crate::mech::MechSpec)>,
    mut rng: ResMut<Rand>,
) {
    let cfg = &data.0.fx;
    // A heavy shell (1 MJ or more) draws a wider streak.
    let heavy = |owner: u32, weapon: usize| {
        mechs
            .iter()
            .find(|(id, _)| id.0 == owner)
            .and_then(|(_, spec)| spec.weapons.get(weapon))
            .is_some_and(|w| w.blast_mj >= 1.0)
    };
    let eye = camera.translation();
    let ahead = fixed.overstep().as_secs_f32();
    let (mut ti, mut mi) = (0usize, 0usize);
    for shot in &shots.0 {
        let pos = shot.pos + shot.vel * ahead;
        let dir = shot.vel.normalize_or(Vec3::NEG_Z);
        if shot.rocket {
            if pools.motors.len() <= mi {
                let e = commands
                    .spawn((
                        Mesh3d(fx.quad.clone()),
                        MeshMaterial3d(kind_mat(&fx, ParticleKind::Flame)),
                        Transform::default(),
                        Visibility::Hidden,
                        bevy::light::NotShadowCaster,
                        Motor,
                        PointLight {
                            intensity: cfg.motor_lumens,
                            range: 60.0,
                            color: Color::srgb(1.0, 0.8, 0.55),
                            ..default()
                        },
                    ))
                    .id();
                pools.motors.push(e);
            } else if let Ok((mut t, mut v)) = q.get_mut(pools.motors[mi]) {
                let tail = pos - dir * 1.2;
                let flicker = 1.6 + 0.6 * rng.f();
                *t = Transform::from_translation(tail)
                    .looking_at(eye, Vec3::Y)
                    .with_scale(Vec3::splat(flicker));
                *v = Visibility::Inherited;
            }
            mi += 1;
            continue;
        }
        if pools.tracers.len() <= ti {
            let e = commands
                .spawn((
                    Mesh3d(fx.quad.clone()),
                    MeshMaterial3d(fx.glow.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                    bevy::light::NotShadowCaster,
                    Tracer,
                ))
                .id();
            pools.tracers.push(e);
        } else if let Ok((mut t, mut v)) = q.get_mut(pools.tracers[ti]) {
            let len = shot.vel.length() * cfg.tracer_s;
            let width = if heavy(shot.owner, shot.weapon) {
                cfg.tracer_heavy_width_m
            } else {
                cfg.tracer_width_m
            };
            let centre = pos - dir * len * 0.5;
            // The quad's long side (y) along the flight; its face turned to the eye about that axis.
            let to_eye = (eye - centre).normalize_or(Vec3::Z);
            let x = dir.cross(to_eye).normalize_or(Vec3::X);
            let z = x.cross(dir);
            *t = Transform {
                translation: centre,
                rotation: Quat::from_mat3(&Mat3::from_cols(x, dir, z)),
                scale: Vec3::new(width, len, 1.0),
            };
            *v = Visibility::Inherited;
        }
        ti += 1;
    }
    for e in pools.tracers.iter().skip(ti).chain(pools.motors.iter().skip(mi)) {
        if let Ok((_, mut v)) = q.get_mut(*e) {
            *v = Visibility::Hidden;
        }
    }
}

fn move_particles(
    mut commands: Commands,
    time: Res<Time>,
    fx: Res<FxAssets>,
    mut rng: ResMut<Rand>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut q: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    let dt = time.delta_secs();
    let face = camera.rotation();
    let eye = camera.translation();
    let mut smoke = Vec::new();
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
        if let Some(every) = p.smokes_every_s {
            p.smoke_in_s -= dt;
            if p.smoke_in_s <= 0.0 {
                p.smoke_in_s = every;
                smoke.push(t.translation);
            }
        }
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
    // Burning debris trails smoke (a few dozen pieces at most, so no cap check).
    for at in smoke {
        spawn_particles(
            &mut commands,
            &fx,
            &mut rng,
            ParticleKind::Smoke,
            at,
            Vec3::Y,
            1,
            0.5,
            0.3,
            0.9,
            1.8,
        );
    }
}

fn grow_rings(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Transform, &mut Ring)>) {
    for (e, mut t, mut r) in &mut q {
        r.age += time.delta_secs();
        if r.age > r.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = r.age / r.life;
        t.scale = Vec3::splat(0.5 + r.radius * k.sqrt()) * Vec3::new(1.0, 1.0, 1.0 - k * 0.9);
    }
}

fn fade_flashes(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut PointLight, &mut FlashLight)>) {
    for (e, mut l, mut f) in &mut q {
        f.age += time.delta_secs();
        if f.age > f.life {
            commands.entity(e).despawn();
        } else {
            l.intensity = f.lumens * (1.0 - f.age / f.life).powi(2);
        }
    }
}

/// White-hot, then yellow-orange, deep orange and soot as it fades.
fn fireball_colour(k: f32) -> (Color, f32) {
    const KEYS: [(f32, [f32; 3], f32); 4] = [
        (0.0, [1.0, 0.97, 0.9], 12.0),
        (0.22, [1.0, 0.72, 0.3], 7.0),
        (0.55, [0.95, 0.38, 0.08], 3.0),
        (1.0, [0.12, 0.10, 0.09], 0.0),
    ];
    let i = KEYS.iter().rposition(|key| key.0 <= k).unwrap_or(0).min(KEYS.len() - 2);
    let (a, b) = (KEYS[i], KEYS[i + 1]);
    let t = ((k - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    let c = [0, 1, 2].map(|j| a.1[j] + (b.1[j] - a.1[j]) * t);
    (Color::srgba(c[0], c[1], c[2], 1.0 - k * k), a.2 + (b.2 - a.2) * t)
}

fn cool_fireballs(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Transform, &mut Fireball)>,
) {
    for (e, mut t, mut f) in &mut q {
        f.age += time.delta_secs();
        if f.age > f.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = f.age / f.life;
        let r = f.radius * (f.age / f.grow).min(1.0).sqrt() * (1.0 + 0.25 * k);
        t.scale = Vec3::splat(r.max(0.05));
        t.translation.y += 1.5 * time.delta_secs();
        if let Some(mut m) = materials.get_mut(&f.mat) {
            let (c, glow) = fireball_colour(k);
            m.base_color = c;
            m.emissive = c.to_linear() * glow;
            m.alpha_mode = if k < 0.6 { AlphaMode::Add } else { AlphaMode::Blend };
        }
    }
}

fn expand_shells(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Transform, &mut Shell)>,
) {
    for (e, mut t, mut s) in &mut q {
        s.age += time.delta_secs();
        if s.age > s.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = s.age / s.life;
        t.scale = Vec3::splat((s.radius * k.sqrt()).max(0.1));
        if let Some(mut m) = materials.get_mut(&s.mat) {
            m.base_color = m.base_color.with_alpha(0.25 * (1.0 - k));
        }
    }
}

fn fade_stars(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Transform, &mut Star)>) {
    for (e, mut t, mut s) in &mut q {
        s.age += time.delta_secs();
        if s.age > s.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = 1.0 - s.age / s.life;
        t.scale = Vec3::splat(s.size * (0.4 + 0.6 * k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fireball_cools_from_white_to_soot() {
        let (hot, g0) = fireball_colour(0.0);
        let (cold, g1) = fireball_colour(1.0);
        assert!(hot.to_srgba().blue > 0.8 && g0 > 10.0, "white-hot at the start");
        assert!(cold.to_srgba().red < 0.2 && g1 == 0.0, "soot at the end");
        assert!(cold.alpha() < 0.01, "and gone");
    }
}
