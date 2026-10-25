//! Visual effects (openspec change `combat-fx`): GPU particles (`particles`, on bevy_hanabi),
//! point-light flashes, fireballs, shock shells, muzzle stars and water rings, played from the
//! recipes in `data/fx/fx.ron`; one glowing streak per shot in flight; a motor flame, light and
//! smoke trail on every rocket.
//! Presentation only: nothing here decides an outcome, so it may use its own cheap random numbers.
//! Other plugins ask for an effect by name with an `Fx` message; tracers read `Shots` directly.

use crate::combat::Shots;
use crate::particles::{self, ParticleLibrary};
use crate::pilot::PilotCamera;
use crate::sim::Data;
use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectAsset, EffectSpawner};
use cb_core::fx::{FxConfig, Layer};

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

#[derive(Component)]
struct Ring {
    age: f32,
    life: f32,
    radius: f32,
    /// Its own material, so it fades on its own.
    material: Handle<StandardMaterial>,
    /// Opacity at the start.
    alpha: f32,
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
    sphere: Handle<Mesh>,
    /// A ripple: soft crests near the edge of a disc, uneven round it.
    ripple: Handle<Image>,
    glow: Handle<StandardMaterial>,
    /// Tracers: the glow's colour, HDR-bright (`tracer_glow`) so the bloom catches them.
    tracer: Handle<StandardMaterial>,
    flame: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct Rand(u64);

impl Rand {
    fn f(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / (1u64 << 24) as f32
    }
}

/// The tracer and motor pools, in `Shots` order.
#[derive(Resource, Default)]
struct Pools {
    tracers: Vec<Entity>,
    motors: Vec<Entity>,
    /// Each motor's smoke trail: its own entity, always drawn, so the smoke it laid hangs on
    /// after the rocket is gone; only its spawner turns off.
    trails: Vec<Entity>,
    /// Each motor's exhaust flame (openspec change `rocket-plumes`).
    plumes: Vec<Entity>,
}

/// A rocket's exhaust flame in the motor pool.
#[derive(Component)]
struct RocketPlume;

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
                    grow_rings,
                    fade_flashes,
                    cool_fireballs,
                    expand_shells,
                    fade_stars,
                    attach_streams,
                    switch_streams,
                    warm_up,
                ),
            );
    }
}

/// Plays every recipe once behind the loading screen, in view but under the ground, so each
/// effect's shaders and pipelines are built before the take starts instead of at the first
/// explosion of its kind mid-fight.
fn warm_up(
    ready: Res<crate::loading::GameReady>,
    data: Res<Data>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
    mut fx: MessageWriter<Fx>,
    mut frames: Local<u32>,
) {
    // Every burst size (`cb_core::fx::burst_count`): density one step to two, a size a frame from
    // frame 30, so the particle budget never cuts one out.
    let last = 30 + 2 * cb_core::fx::BURST_STEPS;
    if ready.0 || *frames > last {
        return;
    }
    *frames += 1;
    if *frames < 30 || *frames > last {
        return;
    }
    let density = (*frames - 29) as f32 / cb_core::fx::BURST_STEPS as f32;
    let Ok(eye) = camera.single() else { return };
    // 200 m out and 100 m down: inside the view, hidden by the ground.
    let at = eye.translation() + eye.forward().with_y(0.0).normalize_or(Vec3::NEG_Z) * 200.0 - Vec3::Y * 100.0;
    let names: Vec<&'static str> = cb_core::fx::RECIPES.to_vec();
    for name in names {
        if data.0.fx.recipes.contains_key(name) {
            fx.write(Fx::new(name, at).dense(density));
        }
    }
    info!(
        "fx: warmed up {} recipes at density {density:.2} behind the loading screen",
        data.0.fx.recipes.len()
    );
}

/// A body that plays a recipe as a continuous stream (`FxConfig::stream_per_s`): one emitter per
/// burst layer, children of this entity, emitting while `on`. `dir` is the way the layers marked
/// `along_dir` throw, in this entity's frame.
///
/// It brings `Transform` and `Visibility` with it: its emitters are children, and a child is drawn
/// only when its parent carries visibility down to it. The jet trail, the smoking parts and the
/// burning hull were spawned with a transform alone, so their emitters were culled and no trail or
/// damage smoke ever drew (owner, 2026-10-25: "I still cant see the players thruster trails").
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct FxStream {
    /// The recipe.
    pub recipe: &'static str,
    /// The direction of throw, local.
    pub dir: Vec3,
    /// Emitting now.
    pub on: bool,
    /// Multiplies the recipe's stream rate (a boarding vent's own rate; 1 for the rest).
    pub density: f32,
}

/// The emitters a stream put under its body.
#[derive(Component)]
struct StreamEmitters(Vec<Entity>);

fn attach_streams(
    mut commands: Commands,
    data: Res<Data>,
    mut lib: ResMut<ParticleLibrary>,
    mut effects: ResMut<Assets<EffectAsset>>,
    added: Query<(Entity, &FxStream), Without<StreamEmitters>>,
) {
    let cfg = &data.0.fx;
    for (e, s) in &added {
        let (Some(layers), Some(per_s)) = (cfg.recipes.get(s.recipe), cfg.stream_per_s.get(s.recipe)) else {
            commands.entity(e).insert(StreamEmitters(Vec::new()));
            continue;
        };
        let mut kids = Vec::new();
        for (i, layer) in layers.iter().enumerate() {
            let Layer::Burst {
                count,
                offset_m,
                along_dir,
                ..
            } = *layer
            else {
                continue;
            };
            let rate = count as f32 * per_s * s.density.max(0.0);
            let dir = if along_dir { s.dir.normalize_or(Vec3::Y) } else { Vec3::Y };
            if let Some(spec) = particles::spec_of(layer).filter(|b| crate::soft_sprites::soft(b.kind)) {
                let mut emitter = crate::soft_sprites::SoftEmitter::new(spec.soft(), rate);
                emitter.on = s.on;
                let kid = commands
                    .spawn((
                        Name::new(format!("fx soft stream {}", s.recipe)),
                        emitter,
                        Transform::from_translation(dir * offset_m).with_rotation(Quat::from_rotation_arc(Vec3::Y, dir)),
                        ChildOf(e),
                    ))
                    .id();
                kids.push(kid);
                continue;
            }
            let Some(effect) = particles::stream(&mut lib, &mut effects, s.recipe, i, layer, rate) else {
                continue;
            };
            let kid = commands
                .spawn((
                    Name::new(format!("fx stream {}", s.recipe)),
                    effect,
                    Transform::from_translation(dir * offset_m).with_rotation(Quat::from_rotation_arc(Vec3::Y, dir)),
                    ChildOf(e),
                ))
                .id();
            kids.push(kid);
        }
        commands.entity(e).insert(StreamEmitters(kids));
    }
}

fn switch_streams(
    streams: Query<(&FxStream, &StreamEmitters)>,
    mut spawners: Query<&mut EffectSpawner>,
    mut soft: Query<&mut crate::soft_sprites::SoftEmitter>,
) {
    for (s, kids) in &streams {
        for k in &kids.0 {
            if let Ok(mut sp) = spawners.get_mut(*k)
                && sp.active != s.on
            {
                sp.active = s.on;
            }
            if let Ok(mut sp) = soft.get_mut(*k)
                && sp.on != s.on
            {
                sp.on = s.on;
            }
        }
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
    mut lib: ResMut<ParticleLibrary>,
    data: Res<crate::sim::Data>,
    mut soft_materials: ResMut<Assets<crate::soft_sprites::SoftMaterial>>,
) {
    let disc = soft_disc(&mut images);
    let ripple = ripple_crests(&mut images);
    // Unlit, the colour is what is drawn: past 1.0 it is HDR, and the bloom spreads it.
    let hot = LinearRgba::from(Color::srgb(1.0, 0.8, 0.45)) * data.0.fx.tracer_glow;
    particles::set_sprite(&mut lib, disc.clone(), &mut images);
    // The Material Maker sprite sheets (openspec change `fx-sprites`); a kind whose sheet cannot be read keeps the disc.
    let root = crate::cli::repo_root().join("assets");
    // `CB_NO_FX_SHEETS=1` keeps the soft disc for every kind, to compare (an A/B switch, as `CB_NO_GROUND_MARKS`).
    let sheets = if std::env::var("CB_NO_FX_SHEETS").is_ok() {
        &[][..]
    } else {
        &particles::SHEETS[..]
    };
    // `CB_FX_SHEETS=a,b` loads only the named sheets (an instrument for finding which one draws wrong).
    let only = std::env::var("CB_FX_SHEETS").ok();
    for name in sheets.iter().copied() {
        if only.as_ref().is_some_and(|o| !o.split(',').any(|n| n == name)) {
            continue;
        }
        if let Some(img) = crate::pbr_set::load_tiling(&root, &format!("textures/fx/{name}/albedo.png"), true) {
            particles::set_sheet(&mut lib, name, img, &mut images);
        }
    }
    let smoke = lib.sheet_image("smoke_puffs").unwrap_or_else(|| disc.clone());
    let dust = lib.sheet_image("dust_wisps").unwrap_or_else(|| disc.clone());
    crate::soft_sprites::make_draw(&mut commands, &mut meshes, &mut soft_materials, smoke, dust, data.0.fx.soft_m);
    commands.insert_resource(FxAssets {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        sphere: meshes.add(Sphere::new(1.0).mesh().uv(32, 16)),
        ripple,
        glow: materials.add(unlit(Color::srgb(1.0, 0.8, 0.45), Some(disc.clone()), AlphaMode::Add, 8.0)),
        tracer: materials.add(unlit(Color::LinearRgba(hot), Some(disc.clone()), AlphaMode::Add, 8.0)),
        flame: materials.add(unlit(Color::srgba(0.7, 0.85, 1.0, 0.85), Some(disc), AlphaMode::Add, 6.0)),
    });
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
    mut lib: ResMut<ParticleLibrary>,
    mut effects: ResMut<Assets<EffectAsset>>,
    time: Res<Time>,
    lights: Query<(Entity, &FlashLight)>,
    fireballs: Query<(), With<Fireball>>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
    ready: Res<crate::loading::GameReady>,
    mut soft: ResMut<crate::soft_sprites::SoftSprites>,
) {
    let _t = crate::perf_hud::slow("play");
    let eye = camera.single().map(|g| g.translation()).ok();
    // Behind the loading screen (the warm-up) every burst plays, at every size: no budget and no range.
    let loading = !ready.0;
    let cfg: &FxConfig = &data.0.fx;
    let now = time.elapsed_secs();
    let mut room = if loading {
        u32::MAX
    } else {
        cfg.max_particles.saturating_sub(lib.live)
    };
    let mut fireball_room = (cfg.max_fireballs as usize).saturating_sub(fireballs.iter().count());
    // Lights alive, oldest first: a new light past the cap puts out the oldest.
    let mut live_lights: Vec<(Entity, f32)> = lights.iter().map(|(e, l)| (e, l.age)).collect();
    live_lights.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut light_at = 0;
    let no_particles = crate::cli::switched_off("CB_NO_PARTICLES");
    let no_lights = crate::cli::switched_off("CB_NO_FX_LIGHTS");
    for e in asked.read() {
        let Some(layers) = cfg.recipes.get(e.recipe) else {
            warn!("fx: no recipe `{}`", e.recipe);
            continue;
        };
        let s = e.scale.max(0.01);
        for (index, layer) in layers.iter().enumerate() {
            // Perf-bench switches: what the effects cost, measured by leaving a kind out.
            if (no_particles && matches!(layer, Layer::Burst { .. })) || (no_lights && matches!(layer, Layer::Light { .. })) {
                continue;
            }
            // Far from the eye a burst barely shows and costs a whole effect (`burst_range`).
            if !loading && matches!(layer, Layer::Burst { .. }) && eye.is_some_and(|p| p.distance(e.at) > cfg.burst_range(e.recipe, s)) {
                continue;
            }
            match *layer {
                Layer::Burst {
                    count,
                    offset_m,
                    along_dir,
                    ..
                } => {
                    let n = cb_core::fx::burst_count(count, e.density, room);
                    room -= n;
                    let dir = if along_dir { e.dir } else { Vec3::Y };
                    let at = e.at + e.dir * offset_m * s;
                    // The blended smoke kinds are drawn sorted and soft (`soft_sprites`).
                    if let Some(spec) = particles::spec_of(layer).filter(|b| crate::soft_sprites::soft(b.kind)) {
                        soft.emit(spec.soft(), n, at, dir, s);
                        continue;
                    }
                    particles::burst(
                        &mut commands,
                        &mut lib,
                        &mut effects,
                        e.recipe,
                        index,
                        layer,
                        n,
                        at,
                        dir,
                        s,
                        now,
                        cfg.sorted.iter().any(|r| r == e.recipe),
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
                    // Additive like the fireball, so the two never need sorting against each other.
                    let mat = materials.add(unlit(Color::srgb(0.24, 0.23, 0.21), None, AlphaMode::Add, 0.0));
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
                    // No flat dust ring on the ground: a disc read as flat geometry beside the
                    // volumetric fireball and smoke (owner, 2026-10-24); the dust bursts carry it.
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
                    // Two sets of crests, the second smaller and slower, so the water rings out.
                    spawn_ring(&mut commands, &fx, &mut materials, e.at, radius_m * s, 2.8, 0.55);
                    spawn_ring(&mut commands, &fx, &mut materials, e.at, radius_m * s * 0.6, 3.6, 0.35);
                }
            }
        }
    }
}

/// A ripple on the water at `at`: a disc of soft crests spreading to `radius` over `life` seconds and
/// fading from `alpha`, with its own material so it fades on its own.
fn spawn_ring(
    commands: &mut Commands,
    fx: &FxAssets,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
    radius: f32,
    life: f32,
    alpha: f32,
) {
    let material = materials.add(unlit(
        Color::srgba(0.92, 0.97, 1.0, alpha),
        Some(fx.ripple.clone()),
        AlphaMode::Blend,
        0.0,
    ));
    commands.spawn((
        Mesh3d(fx.quad.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(at + Vec3::Y * 0.05)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(1.0)),
        bevy::light::NotShadowCaster,
        Ring {
            age: 0.0,
            life,
            radius,
            material,
            alpha,
        },
    ));
}

/// The ripple's picture: three soft crests in the outer half of a disc, the leading one strongest,
/// broken up round the circle so no ripple is a perfect ring.
fn ripple_crests(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: usize = 256;
    let mut data = Vec::with_capacity(N * N * 4);
    for j in 0..N {
        for i in 0..N {
            let (u, v) = (i as f32 / (N - 1) as f32 * 2.0 - 1.0, j as f32 / (N - 1) as f32 * 2.0 - 1.0);
            let a = v.atan2(u);
            // The radius wanders a little round the circle, and the crests' strength more.
            let r = (u * u + v * v).sqrt() * (1.0 + 0.03 * (a * 5.0).sin() + 0.02 * (a * 11.0 + 1.3).sin());
            let smooth = |e0: f32, e1: f32, x: f32| {
                let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            };
            let crests = ((r - 0.5) / 0.5 * 3.0 * std::f32::consts::PI).sin().max(0.0).powf(1.6);
            let envelope = smooth(0.45, 0.8, r) * (1.0 - smooth(0.9, 1.0, r)) * (0.4 + 0.6 * smooth(0.5, 1.0, r));
            let breakup = 0.75 + 0.25 * ((a * 7.0 + 0.5).sin() * (a * 3.0).cos());
            let alpha = (crests * envelope * breakup).clamp(0.0, 1.0);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
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
    mut q: Query<(&mut Transform, &mut Visibility), Or<(With<Tracer>, With<Motor>, With<RocketPlume>)>>,
    plume: Res<crate::plumes::PlumeAssets>,
    mechs: Query<(&crate::mech::MechId, &crate::mech::MechSpec)>,
    mut rng: ResMut<Rand>,
    mut lib: ResMut<ParticleLibrary>,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut trails: Query<
        (
            &mut Transform,
            Option<&mut EffectSpawner>,
            Option<&mut crate::soft_sprites::SoftEmitter>,
        ),
        (Without<Tracer>, Without<Motor>, Without<RocketPlume>),
    >,
) {
    let cfg = &data.0.fx;
    // A rocket lays a smoke puff every `trail_spacing_m` at about 240 m/s.
    let trail_rate = 240.0 / cfg.trail_spacing_m;
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
                        MeshMaterial3d(fx.flame.clone()),
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
                let layer = cfg.recipes.get("rocket_trail").and_then(|layers| layers.first());
                let mut t = commands.spawn((Name::new("rocket trail"), Transform::from_translation(pos)));
                match layer.and_then(particles::spec_of) {
                    Some(spec) if crate::soft_sprites::soft(spec.kind) => {
                        t.insert(crate::soft_sprites::SoftEmitter::new(spec.soft(), trail_rate));
                    }
                    _ => {
                        if let Some(effect) =
                            layer.and_then(|l| particles::stream(&mut lib, &mut effects, "rocket_trail", 0, l, trail_rate))
                        {
                            t.insert(effect);
                        }
                    }
                }
                pools.trails.push(t.id());
                pools.plumes.push(
                    commands
                        .spawn((
                            Name::new("rocket plume"),
                            Mesh3d(plume.mesh.clone()),
                            MeshMaterial3d(plume.rocket.clone()),
                            Transform::default(),
                            Visibility::Hidden,
                            bevy::light::NotShadowCaster,
                            RocketPlume,
                        ))
                        .id(),
                );
            } else if let Ok((mut t, mut v)) = q.get_mut(pools.motors[mi]) {
                let tail = pos - dir * 1.2;
                let flicker = 1.6 + 0.6 * rng.f();
                *t = Transform::from_translation(tail)
                    .looking_at(eye, Vec3::Y)
                    .with_scale(Vec3::splat(flicker));
                *v = Visibility::Inherited;
                if let Some(&p) = pools.plumes.get(mi)
                    && let Ok((mut pt, mut pv)) = q.get_mut(p)
                {
                    // The exhaust: a flame from the nozzle back along the flight, flickering in length.
                    *pt = crate::plumes::placed(
                        pos - dir * 0.6,
                        -dir,
                        cfg.plume_length_m * (0.85 + 0.3 * rng.f()),
                        cfg.plume_width_m,
                    );
                    *pv = Visibility::Inherited;
                }
                if let Some(Ok((mut tt, spawner, soft))) = pools.trails.get(mi).map(|e| trails.get_mut(*e)) {
                    tt.translation = tail;
                    if let Some(mut sp) = spawner {
                        sp.active = true;
                    }
                    if let Some(mut sp) = soft {
                        sp.on = true;
                    }
                }
            }
            mi += 1;
            continue;
        }
        if pools.tracers.len() <= ti {
            let e = commands
                .spawn((
                    Mesh3d(fx.quad.clone()),
                    MeshMaterial3d(fx.tracer.clone()),
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
    for e in pools
        .tracers
        .iter()
        .skip(ti)
        .chain(pools.motors.iter().skip(mi))
        .chain(pools.plumes.iter().skip(mi))
    {
        if let Ok((_, mut v)) = q.get_mut(*e) {
            *v = Visibility::Hidden;
        }
    }
    for e in pools.trails.iter().skip(mi) {
        if let Ok((_, spawner, soft)) = trails.get_mut(*e) {
            if let Some(mut sp) = spawner {
                sp.active = false;
            }
            if let Some(mut sp) = soft {
                sp.on = false;
            }
        }
    }
}

fn grow_rings(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Transform, &mut Ring)>,
) {
    for (e, mut t, mut r) in &mut q {
        r.age += time.delta_secs();
        if r.age > r.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = r.age / r.life;
        // Spreading fast at first and slowing, fading as it goes (the quad is the disc's diameter).
        t.scale = Vec3::splat(2.0 * (0.5 + r.radius * k.sqrt()));
        if let Some(mut m) = materials.get_mut(&r.material) {
            m.base_color.set_alpha(r.alpha * (1.0 - k).powf(1.5));
        }
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
            // Additive all its life, fading through orange to nothing: a blended fireball had to be
            // sorted against the smoke round it and flickered as the order flipped (owner,
            // 2026-10-24: "big explosions get flickery"); switching blend mode rebuilt its pipeline.
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
            let v = 1.0 - k;
            m.base_color = Color::srgb(0.24 * v, 0.23 * v, 0.21 * v);
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

    /// A stream's emitters are its children, so it must carry visibility down to them: one spawned
    /// with nothing else still gets it (the jet trail drew nothing, 2026-10-25).
    #[test]
    fn a_stream_body_carries_visibility_to_its_emitters() {
        let mut world = World::new();
        let e = world
            .spawn(FxStream {
                recipe: "burning",
                dir: Vec3::NEG_Y,
                on: true,
                density: 1.0,
            })
            .id();
        let body = world.entity(e);
        assert!(body.contains::<Visibility>() && body.contains::<InheritedVisibility>());
        assert!(body.contains::<GlobalTransform>());
    }
}
