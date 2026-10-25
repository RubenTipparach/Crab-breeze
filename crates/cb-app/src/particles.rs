//! GPU particles (openspec change `combat-fx` section 6, survey T5): every `Burst` layer of an effect
//! recipe is a `bevy_hanabi` effect built from the layer's own numbers. A burst is one effect
//! entity placed at the point, turned so its local +Y is the burst's direction and scaled by the
//! recipe's scale (hanabi carries a new particle's position and velocity through the emitter's
//! transform in world space), with the size scale as a property. Colour and size change over each
//! particle's life on the GPU; sparks stretch along their path. A rocket's smoke trail is one
//! continuous effect riding its motor. Presentation only.

use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use cb_core::fx::{Layer, ParticleKind};
use std::collections::HashMap;

/// (gravity m/s2, drag per s, growth per s as a share of size, additive glow).
fn motion(kind: ParticleKind) -> (f32, f32, f32, bool) {
    match kind {
        ParticleKind::Spray => (9.81, 0.6, 0.8, false),
        ParticleKind::Steam => (-1.5, 1.2, 1.6, false),
        ParticleKind::Dirt => (9.81, 0.3, 0.0, false),
        ParticleKind::Dust => (-0.3, 1.5, 1.2, false),
        ParticleKind::Smoke => (-2.5, 0.8, 0.9, false),
        ParticleKind::Spark => (9.81, 0.2, -0.5, true),
        ParticleKind::Fire => (-4.0, 1.5, 0.4, true),
        ParticleKind::Flame => (0.0, 3.0, 2.0, true),
        ParticleKind::Debris => (9.81, 0.3, 0.8, false),
        ParticleKind::Rubble => (9.81, 0.2, 0.0, false),
        ParticleKind::Trail => (-0.1, 0.6, 0.3, false),
        ParticleKind::Mist => (0.4, 1.8, 1.4, false),
    }
}

/// A particle's colour over its life (linear RGB, values over 1 glow), keyed 0 to 1.
fn colour_over_life(kind: ParticleKind) -> Vec<(f32, Vec4)> {
    let v = |r: f32, g: f32, b: f32, a: f32| Vec4::new(r, g, b, a);
    match kind {
        ParticleKind::Spray => vec![
            (0.0, v(0.8, 0.9, 0.95, 0.0)),
            (0.08, v(0.8, 0.9, 0.95, 0.7)),
            (1.0, v(0.8, 0.9, 0.95, 0.0)),
        ],
        ParticleKind::Steam => vec![
            (0.0, v(0.9, 0.9, 0.9, 0.0)),
            (0.15, v(0.9, 0.9, 0.9, 0.35)),
            (1.0, v(0.9, 0.9, 0.9, 0.0)),
        ],
        ParticleKind::Dirt => vec![
            (0.0, v(0.22, 0.16, 0.1, 1.0)),
            (0.8, v(0.22, 0.16, 0.1, 1.0)),
            (1.0, v(0.22, 0.16, 0.1, 0.0)),
        ],
        ParticleKind::Dust => vec![
            (0.0, v(0.55, 0.48, 0.38, 0.0)),
            (0.1, v(0.55, 0.48, 0.38, 0.45)),
            (0.6, v(0.5, 0.45, 0.37, 0.3)),
            (1.0, v(0.5, 0.45, 0.37, 0.0)),
        ],
        ParticleKind::Smoke => vec![
            (0.0, v(0.2, 0.19, 0.18, 0.0)),
            (0.1, v(0.2, 0.19, 0.18, 0.55)),
            (0.6, v(0.28, 0.27, 0.26, 0.4)),
            (1.0, v(0.33, 0.32, 0.31, 0.0)),
        ],
        ParticleKind::Spark => vec![
            (0.0, v(9.0, 6.0, 2.0, 1.0)),
            (0.5, v(5.0, 1.8, 0.3, 1.0)),
            (1.0, v(1.5, 0.3, 0.05, 0.0)),
        ],
        ParticleKind::Fire => vec![
            (0.0, v(8.0, 6.0, 2.5, 0.0)),
            (0.08, v(7.0, 3.5, 1.0, 0.9)),
            (0.5, v(3.0, 0.8, 0.15, 0.6)),
            (1.0, v(0.4, 0.1, 0.02, 0.0)),
        ],
        ParticleKind::Flame => vec![
            (0.0, v(3.0, 4.0, 8.0, 0.9)),
            (0.4, v(1.2, 1.8, 4.0, 0.6)),
            (1.0, v(0.3, 0.4, 1.0, 0.0)),
        ],
        // A burning chunk: white-hot, cooling through orange to a smoking cinder.
        ParticleKind::Debris => vec![
            (0.0, v(8.0, 4.0, 1.2, 1.0)),
            (0.25, v(3.0, 0.9, 0.2, 1.0)),
            (0.5, v(0.25, 0.22, 0.2, 0.7)),
            (1.0, v(0.3, 0.3, 0.3, 0.0)),
        ],
        ParticleKind::Rubble => vec![
            (0.0, v(0.4, 0.39, 0.37, 1.0)),
            (0.85, v(0.4, 0.39, 0.37, 1.0)),
            (1.0, v(0.4, 0.39, 0.37, 0.0)),
        ],
        ParticleKind::Trail => vec![
            (0.0, v(0.85, 0.84, 0.82, 0.0)),
            (0.05, v(0.85, 0.84, 0.82, 0.5)),
            (0.5, v(0.8, 0.79, 0.77, 0.3)),
            (1.0, v(0.8, 0.79, 0.77, 0.0)),
        ],
        // Cold mist: a faint blue-white that thickens as it spreads, then thins.
        ParticleKind::Mist => vec![
            (0.0, v(0.82, 0.88, 0.92, 0.0)),
            (0.2, v(0.82, 0.88, 0.92, 0.22)),
            (0.7, v(0.85, 0.89, 0.92, 0.16)),
            (1.0, v(0.85, 0.89, 0.92, 0.0)),
        ],
    }
}

/// The burst numbers an effect asset is built from.
#[derive(Clone, Copy, Debug)]
struct BurstSpec {
    kind: ParticleKind,
    speed_m_s: f32,
    spread: f32,
    size_m: f32,
    life_s: f32,
}

/// Effect assets by (recipe, layer, particle count); a continuous trail by recipe, layer and rate.
#[derive(Resource, Default)]
pub struct ParticleLibrary {
    bursts: HashMap<(&'static str, usize, u32), Handle<EffectAsset>>,
    trails: HashMap<(&'static str, usize, u32), Handle<EffectAsset>>,
    disc: Handle<Image>,
    /// Copies of the disc, one per blended burst in turn (see `set_sprite`).
    discs: Vec<Handle<Image>>,
    next_disc: usize,
    /// Particles started by bursts still alive, for the cap.
    pub live: u32,
}

/// Copies of the sprite handed out to blended bursts in turn.
const DISC_COPIES: usize = 48;

/// A burst's effect entity, gone when its last particle is.
#[derive(Component)]
struct Spent {
    count: u32,
    until_s: f32,
}

/// The particle effects.
pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
            .init_resource::<ParticleLibrary>()
            .add_systems(Update, retire);
    }
}

/// Gives the library the sprite every particle uses (the soft disc), and copies of it for blended
/// bursts. bevy_hanabi merges effects that share an asset and a texture into one draw, sorted by
/// one emitter's position and with its particles in no order, so a far smoke burst drew over a
/// near explosion (owner, 2026-10-24). A blended burst takes the next copy in turn, so bursts alive
/// at once never merge and each sorts by its own position; additive glow merges freely (its order
/// does not show).
pub fn set_sprite(lib: &mut ParticleLibrary, disc: Handle<Image>, images: &mut Assets<Image>) {
    let image = images.get(&disc).cloned();
    lib.discs = image.map_or_else(Vec::new, |i| (0..DISC_COPIES).map(|_| images.add(i.clone())).collect());
    lib.disc = disc;
}

impl ParticleLibrary {
    /// The sprite for a burst: a copy of its own if it is blended.
    fn sprite(&mut self, blended: bool) -> Handle<Image> {
        if !blended || self.discs.is_empty() {
            return self.disc.clone();
        }
        self.next_disc = (self.next_disc + 1) % self.discs.len();
        self.discs[self.next_disc].clone()
    }
}

/// Builds a burst's effect: `count` particles at once, or a steady `rate` a second.
fn build(spec: BurstSpec, spawner: SpawnerSettings, capacity: u32) -> EffectAsset {
    let (gravity, drag, grow, glow) = motion(spec.kind);
    let w = ExprWriter::new();
    let size_scale = w.add_property("size_scale", 1.0.into());
    // In the emitter's frame: +Y is the burst's direction; the transform turns and scales it.
    let jitter = (w.rand(VectorType::VEC3F) * w.lit(2.0) - w.lit(1.0)) * w.lit(spec.spread);
    let pos = jitter.clone() * w.lit(0.5 * spec.size_m.min(1.0));
    let dir = (w.lit(Vec3::Y) + jitter).normalized();
    let speed = w.lit(spec.speed_m_s) * (w.lit(0.5) + w.rand(ScalarType::Float));
    let life = w.lit(spec.life_s) * (w.lit(0.6) + w.lit(0.8) * w.rand(ScalarType::Float));
    let size0 = w.lit(spec.size_m) * w.prop(size_scale) * (w.lit(0.6) + w.lit(0.8) * w.rand(ScalarType::Float));
    let spin = w.rand(ScalarType::Float) * w.lit(std::f32::consts::TAU);
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, pos.expr());
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, (dir * speed).expr());
    let init_age = SetAttributeModifier::new(Attribute::AGE, w.lit(0.0).expr());
    let init_life = SetAttributeModifier::new(Attribute::LIFETIME, life.expr());
    let init_size0 = SetAttributeModifier::new(Attribute::F32_0, size0.expr());
    let init_spin = SetAttributeModifier::new(Attribute::F32_1, spin.expr());
    // Grows (or shrinks) with age from its starting size; never below 1 cm.
    let size = (w.attr(Attribute::F32_0) * (w.lit(1.0) + w.lit(grow) * w.attr(Attribute::AGE))).max(w.lit(0.01));
    let set_size = SetAttributeModifier::new(Attribute::SIZE, size.expr());
    let accel = AccelModifier::new(w.lit(Vec3::new(0.0, -gravity, 0.0)).expr());
    let slow = LinearDragModifier::new(w.lit(drag).expr());
    let rotation = w.attr(Attribute::F32_1).expr();
    let mut gradient = bevy_hanabi::Gradient::new();
    for (t, c) in colour_over_life(spec.kind) {
        gradient.add_key(t, c);
    }
    let mut module = w.finish();
    module.add_texture_slot("sprite", SlotDimension::D2);
    let orient = if spec.kind == ParticleKind::Spark {
        OrientModifier::new(OrientMode::AlongVelocity)
    } else {
        OrientModifier {
            mode: OrientMode::FaceCameraPosition,
            rotation: Some(rotation),
        }
    };
    EffectAsset::new(capacity.max(32), spawner, module)
        .with_name(format!("{:?}", spec.kind))
        .with_alpha_mode(if glow {
            bevy_hanabi::AlphaMode::Add
        } else {
            bevy_hanabi::AlphaMode::Blend
        })
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_life)
        .init(init_size0)
        .init(init_spin)
        .update(set_size)
        .update(accel)
        .update(slow)
        .render(ColorOverLifetimeModifier {
            gradient,
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        .render(ParticleTextureModifier {
            texture_slot: 0,
            sample_mapping: ImageSampleMapping::Modulate,
        })
        .render(orient)
}

/// The burst numbers of a recipe's `Burst` layer.
fn spec_of(layer: &Layer) -> Option<BurstSpec> {
    match *layer {
        Layer::Burst {
            kind,
            speed_m_s,
            spread,
            size_m,
            life_s,
            ..
        } => Some(BurstSpec {
            kind,
            speed_m_s,
            spread,
            size_m,
            life_s,
        }),
        _ => None,
    }
}

/// Starts `count` particles of recipe `recipe`'s layer `index` at `at`, thrown along `dir`, at
/// `scale`. Cut to `room` (the particle cap); nothing when the layer is not a burst.
#[allow(clippy::too_many_arguments)]
pub fn burst(
    commands: &mut Commands,
    lib: &mut ParticleLibrary,
    effects: &mut Assets<EffectAsset>,
    recipe: &'static str,
    index: usize,
    layer: &Layer,
    count: u32,
    at: Vec3,
    dir: Vec3,
    scale: f32,
    now_s: f32,
) {
    let Some(spec) = spec_of(layer) else { return };
    if count == 0 {
        return;
    }
    let handle = lib
        .bursts
        .entry((recipe, index, count))
        .or_insert_with(|| effects.add(build(spec, SpawnerSettings::once((count as f32).into()), count)))
        .clone();
    let mut props = EffectProperties::default();
    props.set("size_scale", scale.into());
    lib.live += count;
    // Only an explosion's smoke gets a sprite of its own: per-burst sprites stop bevy_hanabi
    // batching, and for every burst they cost about 6 ms a frame in combat (2026-10-24). The big
    // blended clouds are what showed the wrong order.
    let blended = !motion(spec.kind).3 && recipe.starts_with("explosion");
    let sprite = lib.sprite(blended);
    commands.spawn((
        Name::new(format!("fx {recipe}")),
        ParticleEffect::new(handle),
        EffectMaterial { images: vec![sprite] },
        props,
        Transform::from_translation(at)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, dir.normalize_or(Vec3::Y)))
            .with_scale(Vec3::splat(scale.max(0.01))),
        Spent {
            count,
            until_s: now_s + spec.life_s * 1.4 + 0.2,
        },
    ));
}

/// A steady stream of recipe `recipe`'s layer `index` at `rate` a second (a rocket's smoke trail),
/// to ride an entity; switch it with `EffectSpawner::active`. It carries its properties (the size
/// scale, 1) as a burst does: an effect whose module declares a property draws nothing without them.
pub fn stream(
    lib: &mut ParticleLibrary,
    effects: &mut Assets<EffectAsset>,
    recipe: &'static str,
    index: usize,
    layer: &Layer,
    rate: f32,
) -> Option<(ParticleEffect, EffectMaterial, EffectProperties)> {
    let spec = spec_of(layer)?;
    let handle = lib
        .trails
        .entry((recipe, index, rate.to_bits()))
        .or_insert_with(|| {
            let capacity = (rate * spec.life_s * 1.4).ceil() as u32;
            effects.add(build(spec, SpawnerSettings::rate(rate.into()), capacity))
        })
        .clone();
    let mut props = EffectProperties::default();
    props.set("size_scale", 1.0.into());
    Some((
        ParticleEffect::new(handle),
        EffectMaterial {
            images: vec![lib.disc.clone()],
        },
        props,
    ))
}

fn retire(mut commands: Commands, time: Res<Time>, mut lib: ResMut<ParticleLibrary>, q: Query<(Entity, &Spent)>) {
    let now = time.elapsed_secs();
    for (e, s) in &q {
        if now > s.until_s {
            lib.live = lib.live.saturating_sub(s.count);
            commands.entity(e).despawn();
        }
    }
}
