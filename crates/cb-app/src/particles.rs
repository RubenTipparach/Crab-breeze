//! GPU particles (openspec change `combat-fx` section 6, survey T5): every `Burst` layer of an effect
//! recipe is a `bevy_hanabi` effect built from the layer's own numbers. A burst is one effect
//! entity placed at the point, turned so its local +Y is the burst's direction and scaled by the
//! recipe's scale (hanabi carries a new particle's position and velocity through the emitter's
//! transform in world space), with the size scale as a property. Colour and size change over each
//! particle's life on the GPU; sparks stretch along their path. A rocket's smoke trail is one
//! continuous effect riding its motor. Each kind draws from its Material Maker sprite sheet (openspec change
//! `fx-sprites`): smoke a random billow, fire its flipbook over its life, dust a wisp, dirt a clod; sparks and spray keep
//! the soft disc. Presentation only.

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
        // The clod sheet carries the soil's colour; this only tints it.
        ParticleKind::Dirt => vec![
            (0.0, v(0.85, 0.8, 0.75, 1.0)),
            (0.8, v(0.85, 0.8, 0.75, 1.0)),
            (1.0, v(0.85, 0.8, 0.75, 0.0)),
        ],
        // Pale, thin and mostly grey: the wisp sheet is already tan, so this only dims it.
        ParticleKind::Dust => vec![
            (0.0, v(0.62, 0.6, 0.57, 0.0)),
            (0.1, v(0.62, 0.6, 0.57, 0.28)),
            (0.6, v(0.6, 0.59, 0.57, 0.16)),
            (1.0, v(0.6, 0.59, 0.57, 0.0)),
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
            (0.0, v(1.1, 1.08, 1.05, 1.0)),
            (0.85, v(1.1, 1.08, 1.05, 1.0)),
            (1.0, v(1.1, 1.08, 1.05, 0.0)),
        ],
        ParticleKind::Trail => vec![
            (0.0, v(0.85, 0.84, 0.82, 0.0)),
            (0.05, v(0.85, 0.84, 0.82, 0.5)),
            (0.5, v(0.8, 0.79, 0.77, 0.3)),
            (1.0, v(0.8, 0.79, 0.77, 0.0)),
        ],
    }
}

/// The Material Maker sprite sheet a kind draws from (`assets/textures/fx/<sheet>/albedo.png`, 4 by 4 cells), or None
/// for the soft disc. One table, so a new kind names its sheet here.
pub fn sheet(kind: ParticleKind) -> Option<&'static str> {
    match kind {
        ParticleKind::Smoke | ParticleKind::Steam | ParticleKind::Trail => Some("smoke_puffs"),
        ParticleKind::Fire | ParticleKind::Flame => Some("fire_flipbook"),
        ParticleKind::Dust => Some("dust_wisps"),
        ParticleKind::Dirt | ParticleKind::Debris | ParticleKind::Rubble => Some("dirt_clods"),
        ParticleKind::Spark | ParticleKind::Spray => None,
    }
}

/// Every sheet `sheet` names.
pub const SHEETS: [&str; 4] = ["smoke_puffs", "fire_flipbook", "dust_wisps", "dirt_clods"];

/// Cells across a sheet.
const GRID: u32 = 4;

/// Whether a kind plays its sheet as a flipbook over its life (fire) rather than holding one random cell.
fn flipbook(kind: ParticleKind) -> bool {
    matches!(kind, ParticleKind::Fire | ParticleKind::Flame)
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

/// Effect assets by (recipe, layer, particle count); a continuous trail by recipe and layer.
#[derive(Resource, Default)]
pub struct ParticleLibrary {
    bursts: HashMap<(&'static str, usize, u32), Handle<EffectAsset>>,
    trails: HashMap<(&'static str, usize), Handle<EffectAsset>>,
    disc: Handle<Image>,
    /// Copies of the disc, one per blended burst in turn (see `set_sprite`).
    discs: Vec<Handle<Image>>,
    next_disc: usize,
    /// Each sheet and its copies for blended bursts (fewer than the disc's: a sheet is 1024 px).
    sheets: HashMap<&'static str, (Handle<Image>, Vec<Handle<Image>>, usize)>,
    /// Particles started by bursts still alive, for the cap.
    pub live: u32,
}

/// Copies of the sprite handed out to blended bursts in turn.
const DISC_COPIES: usize = 48;
/// Copies of each sheet for blended bursts: 12 of a 1024 px sheet is about 64 MiB.
const SHEET_COPIES: usize = 12;

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

/// Gives the library a kind's sprite sheet (`fx-sprites`), and copies of it for blended bursts.
pub fn set_sheet(lib: &mut ParticleLibrary, name: &'static str, image: Image, images: &mut Assets<Image>) {
    // `CB_NO_SHEET_COPIES` hands every burst the one sheet (an instrument, as `CB_NO_FX_SHEETS`).
    let n = if std::env::var("CB_NO_SHEET_COPIES").is_ok() {
        0
    } else {
        SHEET_COPIES
    };
    let copies = (0..n).map(|_| images.add(image.clone())).collect();
    lib.sheets.insert(name, (images.add(image), copies, 0));
}

impl ParticleLibrary {
    /// The sprite for a burst of `kind`: its sheet, or the disc when it has none or the sheet did not load; a copy of
    /// its own if it is blended.
    fn sprite(&mut self, kind: ParticleKind, blended: bool) -> Handle<Image> {
        if let Some((image, copies, next)) = sheet(kind).and_then(|n| self.sheets.get_mut(n)) {
            if !blended || copies.is_empty() {
                return image.clone();
            }
            *next = (*next + 1) % copies.len();
            return copies[*next].clone();
        }
        if !blended || self.discs.is_empty() {
            return self.disc.clone();
        }
        self.next_disc = (self.next_disc + 1) % self.discs.len();
        self.discs[self.next_disc].clone()
    }

    /// Whether `kind` draws from a loaded sheet (its effect asset then carries the flipbook).
    fn has_sheet(&self, kind: ParticleKind) -> bool {
        sheet(kind).is_some_and(|n| self.sheets.contains_key(n))
    }
}

/// The texture slot's name for a kind: its sheet's, or the disc's (see `build`).
fn slot_name(kind: ParticleKind, sheet: bool) -> &'static str {
    match sheet.then(|| self::sheet(kind)).flatten() {
        Some(name) => name,
        None => "disc",
    }
}

/// Builds a burst's effect: `count` particles at once, or a steady `rate` a second; `sheet` when the kind draws from
/// a sprite sheet.
fn build(spec: BurstSpec, spawner: SpawnerSettings, capacity: u32, sheet: bool) -> EffectAsset {
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
    // A sheet's sprites are lit from above, so they turn only a little and keep their lit side up.
    let spin = if sheet {
        (w.rand(ScalarType::Float) - w.lit(0.5)) * w.lit(0.6)
    } else {
        w.rand(ScalarType::Float) * w.lit(std::f32::consts::TAU)
    };
    let cells = (GRID * GRID) as f32;
    // A random cell for the life of the particle, or the frame its age has reached through a flipbook.
    let cell = if flipbook(spec.kind) {
        (w.attr(Attribute::AGE) / w.attr(Attribute::LIFETIME) * w.lit(cells))
            .floor()
            .min(w.lit(cells - 1.0))
            .cast(ScalarType::Int)
    } else {
        (w.rand(ScalarType::Float) * w.lit(cells))
            .floor()
            .min(w.lit(cells - 1.0))
            .cast(ScalarType::Int)
    };
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
    let set_cell = SetAttributeModifier::new(Attribute::SPRITE_INDEX, cell.expr());
    let mut module = w.finish();
    // The slot is named after the image it takes. bevy_hanabi (rev 77630ff) caches an effect's texture bind group by
    // its property buffer and texture layout, not by the images bound, so every one-slot effect drew whichever image
    // the first one bound: the smoke column took the fire flipbook and stood twenty seconds as an orange "tornado"
    // (owner, 2026-10-25). A slot name is part of the layout, so one name per image keeps the bind groups apart.
    module.add_texture_slot(slot_name(spec.kind, sheet), SlotDimension::D2);
    let orient = if spec.kind == ParticleKind::Spark {
        OrientModifier::new(OrientMode::AlongVelocity)
    } else {
        OrientModifier {
            mode: OrientMode::FaceCameraPosition,
            rotation: Some(rotation),
        }
    };
    let effect = EffectAsset::new(capacity.max(32), spawner, module)
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
        .render(orient);
    if !sheet {
        return effect;
    }
    let effect = if flipbook(spec.kind) {
        effect.update(set_cell)
    } else {
        effect.init(set_cell)
    };
    effect.render(FlipbookModifier {
        sprite_grid_size: UVec2::splat(GRID),
    })
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
    sorted: bool,
) {
    let Some(spec) = spec_of(layer) else { return };
    if count == 0 {
        return;
    }
    let sheet = lib.has_sheet(spec.kind);
    let handle = lib
        .bursts
        .entry((recipe, index, count))
        .or_insert_with(|| effects.add(build(spec, SpawnerSettings::once((count as f32).into()), count, sheet)))
        .clone();
    let mut props = EffectProperties::default();
    props.set("size_scale", scale.into());
    lib.live += count;
    // Only the `sorted` recipes' blended layers get a sprite of their own: per-burst sprites stop
    // bevy_hanabi batching, and for every burst they cost about 6 ms a frame in combat
    // (2026-10-24). The big blended clouds (explosion smoke, collapse dust) are what showed the
    // wrong order, and flickered as merged bursts came and went.
    let blended = !motion(spec.kind).3 && sorted;
    let sprite = lib.sprite(spec.kind, blended);
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
/// to ride an entity; switch it with `EffectSpawner::active`.
pub fn stream(
    lib: &mut ParticleLibrary,
    effects: &mut Assets<EffectAsset>,
    recipe: &'static str,
    index: usize,
    layer: &Layer,
    rate: f32,
) -> Option<(ParticleEffect, EffectMaterial)> {
    let spec = spec_of(layer)?;
    let sheet = lib.has_sheet(spec.kind);
    let handle = lib
        .trails
        .entry((recipe, index))
        .or_insert_with(|| {
            let capacity = (rate * spec.life_s * 1.4).ceil() as u32;
            effects.add(build(spec, SpawnerSettings::rate(rate.into()), capacity, sheet))
        })
        .clone();
    Some((
        ParticleEffect::new(handle),
        // A copy of the sprite of its own, as a blended burst gets: streams sharing an asset and an
        // image are merged by bevy_hanabi into one draw, and with several mechs' jets streaming at
        // once only one of them drew (2026-10-25: enemies in flight showed no flame or trail).
        EffectMaterial {
            images: vec![lib.sprite(spec.kind, true)],
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_particle_kind_has_a_shipped_sheet_or_the_disc() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/textures/fx");
        for kind in [
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
        ] {
            if let Some(name) = sheet(kind) {
                assert!(SHEETS.contains(&name), "{kind:?} names {name}, not in SHEETS");
                let img = image::open(root.join(name).join("albedo.png")).expect("the sheet is shipped");
                assert_eq!(img.width() % GRID, 0, "{name} splits into {GRID} cells");
            }
        }
    }

    /// bevy_hanabi shares one texture bind group between effects with equal texture layouts, so two kinds drawing
    /// different images must not have equal layouts (the orange smoke column, 2026-10-25).
    #[test]
    fn kinds_drawing_different_images_never_share_a_texture_layout() {
        let spec = |kind| BurstSpec {
            kind,
            speed_m_s: 1.0,
            spread: 0.5,
            size_m: 1.0,
            life_s: 1.0,
        };
        let layout = |kind, sheet| {
            build(spec(kind), SpawnerSettings::once(1.0.into()), 32, sheet)
                .module()
                .texture_layout()
        };
        let kinds = [ParticleKind::Smoke, ParticleKind::Fire, ParticleKind::Dust, ParticleKind::Dirt];
        for (i, a) in kinds.iter().enumerate() {
            for b in &kinds[i + 1..] {
                assert_ne!(layout(*a, true), layout(*b, true), "{a:?} and {b:?} would share a bind group");
            }
            assert_ne!(layout(*a, true), layout(ParticleKind::Spark, false), "{a:?} would share the disc's");
        }
    }
}
