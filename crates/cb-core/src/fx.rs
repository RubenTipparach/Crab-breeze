//! Effect recipes (openspec change `combat-fx`): `data/fx/fx.ron` names every effect the game plays
//! (a muzzle flash, an impact on armour, a ground blast) as layers of particles, lights and shapes,
//! with their numbers. Data only: `cb-app::fx` draws them. It lives here so the one data loader
//! checks it at startup and `cb-tools validate` checks it before a push.

use crate::data::{Validate, finite, finite3, positive, within};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Every recipe the game asks for. A file without one of these stops startup.
pub const RECIPES: [&str; 33] = [
    "muzzle_autocannon",
    "muzzle_cannon",
    "muzzle_rocket",
    "impact_mech",
    "impact_building",
    "impact_ground",
    "impact_water",
    "explosion_ground",
    "explosion_water",
    "explosion_air",
    "rocket_trail",
    "foot_strike_dirt",
    "foot_strike_concrete",
    "landing_dirt",
    "landing_concrete",
    "foot_spray",
    "jet_flame",
    "skate_dirt",
    "skate_grit",
    "skate_spray",
    "wall_smash",
    "tree_fall",
    "part_lost",
    "burning",
    "smoking",
    "burning_hull",
    "fall_dust",
    "jet_wash",
    "creak_dust",
    "member_fail",
    "collapse_dust",
    "vent_steam",
    "vent_mist",
];

/// What a particle looks like and how it moves (its colour and motion are the drawing's).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ParticleKind {
    /// White water thrown up.
    Spray,
    /// Rising white steam.
    Steam,
    /// Brown clods that fall.
    Dirt,
    /// Tan dust.
    Dust,
    /// Dark rising smoke.
    Smoke,
    /// Bright sparks that fall.
    Spark,
    /// Orange fire.
    Fire,
    /// Blue-white jet and rocket flame.
    Flame,
    /// Burning chunks that arc out and trail smoke.
    Debris,
    /// Grey concrete chunks that fall.
    Rubble,
    /// A rocket's hanging smoke trail.
    Trail,
    /// Cold, pale mist that sinks and spreads (a cabin opening, `boarding-vents`).
    Mist,
}

/// One layer of an effect. Sizes, radii, offsets and speeds scale with the effect's `scale`;
/// counts with its `density`; light with `scale` cubed (energy).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Layer {
    /// Billboard particles of one kind.
    Burst {
        /// Kind.
        kind: ParticleKind,
        /// Particles.
        count: u32,
        /// Mean speed.
        speed_m_s: f32,
        /// Cone spread, 0 straight to 1 a hemisphere.
        spread: f32,
        /// Particle size.
        size_m: f32,
        /// Lifetime.
        life_s: f32,
        /// Moved this far along the effect's direction.
        #[serde(default)]
        offset_m: f32,
        /// Thrown along the effect's direction rather than straight up.
        #[serde(default)]
        along_dir: bool,
    },
    /// A point light that fades as (1 - t)^2.
    Light {
        /// Peak intensity, lumens.
        lumens: f32,
        /// Range.
        range_m: f32,
        /// Lifetime.
        life_s: f32,
        /// Colour, sRGB 0..1.
        colour: [f32; 3],
        /// Raised this far above the effect's point.
        #[serde(default)]
        lift_m: f32,
    },
    /// An emissive sphere that grows, then cools from white through orange to soot and fades.
    Fireball {
        /// Full radius.
        radius_m: f32,
        /// Time to reach it.
        grow_s: f32,
        /// Lifetime.
        life_s: f32,
    },
    /// A thin translucent shell that expands fast.
    Shock {
        /// Final radius of the shell.
        radius_m: f32,
        /// Lifetime of the shell.
        life_s: f32,
    },
    /// Three crossed glowing blades, turned at random: a muzzle flash.
    Star {
        /// Blade length.
        size_m: f32,
        /// Lifetime.
        life_s: f32,
        /// Moved this far along the effect's direction.
        #[serde(default)]
        offset_m: f32,
    },
    /// A ring spreading on water.
    Ripple {
        /// Final radius.
        radius_m: f32,
    },
}

impl Layer {
    fn validate(&self, at: &str) -> Result<(), String> {
        let f = |k: &str| format!("{at}.{k}");
        match self {
            Layer::Burst {
                speed_m_s,
                spread,
                size_m,
                life_s,
                offset_m,
                ..
            } => {
                finite(&f("speed_m_s"), *speed_m_s)?;
                within(&f("spread"), *spread, 0.0, 2.0)?;
                positive(&f("size_m"), *size_m)?;
                positive(&f("life_s"), *life_s)?;
                finite(&f("offset_m"), *offset_m)
            }
            Layer::Light {
                lumens,
                range_m,
                life_s,
                colour,
                lift_m,
            } => {
                positive(&f("lumens"), *lumens)?;
                positive(&f("range_m"), *range_m)?;
                positive(&f("life_s"), *life_s)?;
                finite3(&f("colour"), *colour)?;
                finite(&f("lift_m"), *lift_m)
            }
            Layer::Fireball { radius_m, grow_s, life_s } => {
                positive(&f("radius_m"), *radius_m)?;
                positive(&f("grow_s"), *grow_s)?;
                positive(&f("life_s"), *life_s)
            }
            Layer::Shock { radius_m, life_s } => {
                positive(&f("radius_m"), *radius_m)?;
                positive(&f("life_s"), *life_s)
            }
            Layer::Star { size_m, life_s, offset_m } => {
                positive(&f("size_m"), *size_m)?;
                positive(&f("life_s"), *life_s)?;
                finite(&f("offset_m"), *offset_m)
            }
            Layer::Ripple { radius_m } => positive(&f("radius_m"), *radius_m),
        }
    }
}

/// `data/fx/fx.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FxConfig {
    /// A tracer's streak is the distance its shot covers in this long.
    pub tracer_s: f32,
    /// A light round's streak width.
    pub tracer_width_m: f32,
    /// A heavy shell's (1 MJ or more) streak width.
    pub tracer_heavy_width_m: f32,
    /// How many times brighter than white a tracer burns, so the bloom catches it (HDR; owner,
    /// 2026-10-25: "bullets need to be brighter, like affected by bloom more or something? HDR maybe?").
    pub tracer_glow: f32,
    /// A rocket drops a `rocket_trail` puff every this far.
    pub trail_spacing_m: f32,
    /// A rocket motor's light.
    pub motor_lumens: f32,
    /// A rocket's exhaust plume's length and width, metres (openspec change `rocket-plumes`).
    pub plume_length_m: f32,
    /// See `plume_length_m`.
    pub plume_width_m: f32,
    /// How bright a rocket's plume glows, a share of its look's own (1: as drawn in `plumes.rs`).
    pub plume_glow: f32,
    /// A mech's jet plume's length and width while it fires, metres.
    pub jet_plume_m: [f32; 2],
    /// Jets fired this close above the ground blow dust (`jet_wash`), metres.
    pub jet_wash_m: f32,
    /// Most particles alive at once; a burst that does not fit is cut.
    pub max_particles: u32,
    /// Most effect point lights alive at once; the oldest goes first.
    pub max_lights: u32,
    /// Most fireballs alive at once; one that does not fit is not drawn.
    pub max_fireballs: u32,
    /// The recipes, by name.
    pub recipes: BTreeMap<String, Vec<Layer>>,
    /// The view's shake from blasts nearby.
    pub shake: ShakeConfig,
    /// Recipes played as a continuous stream from an emitter that rides its body (a jet's flame,
    /// a wreck's fire), by name: its bursts' particle counts, this many times a second. One stream
    /// per layer instead of a burst entity per frame (2,700 jet bursts and 3,400 fire bursts in 110
    /// s of combat cost about 6 ms a frame, 2026-10-24).
    pub stream_per_s: BTreeMap<String, f32>,
    /// Particle bursts start only within this of the eye, metres, times the effect's scale (and
    /// four times as far for an explosion): every burst is an effect of its own on the GPU side,
    /// and those far off cost about 2 ms of a combat frame while barely showing (2026-10-24).
    pub burst_range_m: f32,
    /// Recipes whose blended bursts each get a sprite of their own, so bevy_hanabi draws them
    /// apart, each in a stable order, instead of merging them into one draw whose particles reorder
    /// every frame: big translucent clouds (explosion smoke, collapse dust) flickered merged (owner,
    /// 2026-10-25: "this smoke here is reall flickery still"). Each costs its own draw.
    pub sorted: Vec<String>,
    /// Most soft smoke sprites alive at once (openspec change `particle-depth`): the blended smoke,
    /// trail, dust, steam and mist particles, drawn sorted back to front and faded where they meet the
    /// world instead of by bevy_hanabi. Past it, the oldest go first.
    pub soft_max_particles: u32,
    /// A soft sprite fades out over this many metres in front of the surface behind it, so it never
    /// shows a straight cut where it crosses the ground or a wall.
    pub soft_m: f32,
}

/// How blasts nearby shake the view (owner, 2026-10-24: "screen needs to shake a bit when
/// explosions go off nearby"). The shake rides on top of where the player looks and never moves
/// their aim.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ShakeConfig {
    /// The most the view turns at full shake, degrees.
    pub max_deg: f32,
    /// A blast shakes the view within this far, metres, per cube root of its megajoules.
    pub reach_m_per_cbrt_mj: f32,
    /// A full shake dies away over this long.
    pub decay_s: f32,
    /// The mech's own body (openspec change `cockpit-shake`; owner, 2026-10-28: "thruster shakes, impact shakes, skate
    /// shakes", "I want to the player to feel the weight of this mech when it hits things", "When mech gets shot and
    /// punches stuff"). A landing kicks from this downward speed, metres a second...
    pub land_from_m_s: f32,
    /// ...this much a metre a second over it.
    pub land_per_m_s: f32,
    /// A ram: speed lost in one step beyond this, metres a second (no brake takes that much), kicks...
    pub ram_from_m_s: f32,
    /// ...this much a metre a second over it.
    pub ram_per_m_s: f32,
    /// A hit on the pilot's own mech kicks this much a megajoule of blast...
    pub hit_per_mj: f32,
    /// ...and this much a millimetre of the round's penetration.
    pub hit_per_mm: f32,
    /// Firing (owner, 2026-10-28: "firing guns should also cause some vibrations", "and missiles"): every round or
    /// missile sets the rumble to at least this, 0..1...
    pub fire_rumble: f32,
    /// ...held this long after the last one, seconds, then eased away.
    pub fire_hold_s: f32,
    /// A heavy round (a cannon, a slug, a shotgun) also kicks this much...
    pub heavy_kick: f32,
    /// ...and dips the eye at this speed, metres a second.
    pub heavy_jolt_m_s: f32,
    /// A missile leaving its rail kicks this much.
    pub missile_kick: f32,
    /// The pilot's punch or strike landing kicks this much...
    pub melee_kick: f32,
    /// ...and drops the eye at this speed, metres a second.
    pub melee_jolt_m_s: f32,
    /// The jets lighting kick this much.
    pub ignite_kick: f32,
    /// A footstep of the pilot's own mech (owner, 2026-10-29: "can you add some shakes from footsteps back in?",
    /// "similar to the landing shake, but like 50% that strength") kicks as a landing at this downward speed, metres a
    /// second...
    pub step_as_landing_m_s: f32,
    /// ...times this share (0: footsteps do not move the view).
    pub step_share: f32,
    /// The rumble's most, degrees.
    pub rumble_deg: f32,
    /// The rumble's buzz, hertz.
    pub rumble_hz: f32,
    /// The rumble swells and fades over this, seconds.
    pub rumble_ease_s: f32,
    /// The jets' rumble while boosting, 0..1.
    pub jets_rumble: f32,
    /// The skates' rumble at `skate_full_m_s` on rough ground, 0..1...
    pub skate_rumble: f32,
    /// ...reached at this speed, metres a second...
    pub skate_full_m_s: f32,
    /// ...and this share of it on paved ground.
    pub paved_share: f32,
    /// A landing or a ram drops the eye this many metres a second for each metre a second of its speed...
    pub jolt_per_m_s: f32,
    /// ...never further than this, metres...
    pub jolt_max_m: f32,
    /// ...and it springs back at this rate, hertz...
    pub jolt_hz: f32,
    /// ...damped by this share of critical.
    pub jolt_damping: f32,
}

impl Default for ShakeConfig {
    fn default() -> Self {
        Self {
            max_deg: 1.2,
            reach_m_per_cbrt_mj: 60.0,
            decay_s: 0.7,
            land_from_m_s: 3.0,
            land_per_m_s: 0.06,
            ram_from_m_s: 2.0,
            ram_per_m_s: 0.08,
            hit_per_mj: 0.08,
            hit_per_mm: 0.0015,
            fire_rumble: 0.6,
            fire_hold_s: 0.15,
            heavy_kick: 0.4,
            heavy_jolt_m_s: 0.2,
            missile_kick: 0.3,
            melee_kick: 0.45,
            melee_jolt_m_s: 0.6,
            ignite_kick: 0.15,
            step_as_landing_m_s: 10.0,
            step_share: 0.5,
            rumble_deg: 0.25,
            rumble_hz: 23.0,
            rumble_ease_s: 0.3,
            jets_rumble: 0.7,
            skate_rumble: 0.5,
            skate_full_m_s: 30.0,
            paved_share: 0.5,
            jolt_per_m_s: 0.12,
            jolt_max_m: 0.25,
            jolt_hz: 1.6,
            jolt_damping: 0.45,
        }
    }
}

/// What a body event does to the view: a kick to the trauma (0..1), a drop of the eye, metres a second, and a hold of
/// the firing rumble, seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Kick {
    /// Added to the trauma.
    pub trauma: f32,
    /// The eye's downward speed it starts, metres a second.
    pub jolt_m_s: f32,
    /// The firing rumble is held this long from now, seconds (0: none).
    pub fire_s: f32,
}

/// What the pilot's own mech fired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fired {
    /// A light round (an autocannon's, a machine gun's).
    Light,
    /// A heavy round: a cannon's, a slug, a shotgun's.
    Heavy,
    /// A missile or rocket.
    Missile,
}

/// The eye's dip and its spring back (`ShakeConfig::jolt_*`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Jolt {
    /// How far the eye is down, metres.
    pub y: f32,
    /// How fast it is going down, metres a second.
    pub v: f32,
}

impl Jolt {
    /// A kick's drop added, metres a second.
    pub fn kick(&mut self, v_m_s: f32) {
        if v_m_s.is_finite() {
            self.v += v_m_s.max(0.0);
        }
    }

    /// `dt_s` of the damped spring back to rest, the dip held within `jolt_max_m`.
    pub fn step(&mut self, cfg: &ShakeConfig, dt_s: f32) {
        // No spring or no room to dip: the jolt is off.
        if cfg.jolt_hz <= 0.0 || cfg.jolt_max_m <= 0.0 {
            *self = Jolt::default();
            return;
        }
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        let w = std::f32::consts::TAU * cfg.jolt_hz;
        // Small steps, so a long frame cannot throw the spring.
        let n = (dt_s / 0.004).ceil().clamp(1.0, 64.0) as usize;
        let h = dt_s / n as f32;
        for _ in 0..n {
            let a = -w * w * self.y - 2.0 * cfg.jolt_damping * w * self.v;
            self.v += a * h;
            self.y += self.v * h;
            if self.y > cfg.jolt_max_m {
                self.y = cfg.jolt_max_m;
                self.v = self.v.min(0.0);
            }
        }
        if self.y.abs() < 1e-5 && self.v.abs() < 1e-4 {
            *self = Jolt::default();
        }
    }
}

impl FxConfig {
    /// How far from the eye recipe `recipe` played at `scale` starts its particle bursts.
    pub fn burst_range(&self, recipe: &str, scale: f32) -> f32 {
        let big = if recipe.starts_with("explosion") { 4.0 } else { 1.0 };
        self.burst_range_m * scale.max(1.0) * big
    }
}

impl ShakeConfig {
    /// The trauma `dt_s` later: falling by a full shake over `decay_s`; with no decay time there is no trauma at all.
    pub fn decay(&self, trauma: f32, dt_s: f32) -> f32 {
        if self.decay_s <= 0.0 || !trauma.is_finite() {
            return 0.0;
        }
        (trauma - dt_s.max(0.0) / self.decay_s).max(0.0)
    }

    /// The rumble `dt_s` later, eased from `now` toward `want` over `rumble_ease_s` (0: at once).
    pub fn ease_rumble(&self, now: f32, want: f32, dt_s: f32) -> f32 {
        let k = if self.rumble_ease_s <= 0.0 {
            1.0
        } else {
            (dt_s.max(0.0) / self.rumble_ease_s).clamp(0.0, 1.0)
        };
        let r = now + (want - now) * k;
        if r.is_finite() { r.clamp(0.0, 1.0) } else { 0.0 }
    }

    /// A landing at `down_m_s`, the downward speed at touchdown.
    pub fn landing(&self, down_m_s: f32) -> Kick {
        let over = (down_m_s - self.land_from_m_s).max(0.0);
        if !over.is_finite() || over == 0.0 {
            return Kick::default();
        }
        Kick {
            trauma: (over * self.land_per_m_s).min(1.0),
            jolt_m_s: down_m_s * self.jolt_per_m_s,
            fire_s: 0.0,
        }
    }

    /// A ram: `lost_m_s` of speed lost in one step.
    pub fn ram(&self, lost_m_s: f32) -> Kick {
        let over = (lost_m_s - self.ram_from_m_s).max(0.0);
        if !over.is_finite() || over == 0.0 {
            return Kick::default();
        }
        Kick {
            trauma: (over * self.ram_per_m_s).min(1.0),
            jolt_m_s: lost_m_s * self.jolt_per_m_s,
            fire_s: 0.0,
        }
    }

    /// A hit on the pilot's own mech by a round of `penetration_mm` and `blast_mj`.
    pub fn hit(&self, penetration_mm: f32, blast_mj: f32) -> Kick {
        let k = penetration_mm.max(0.0) * self.hit_per_mm + blast_mj.max(0.0) * self.hit_per_mj;
        Kick {
            trauma: if k.is_finite() { k.min(1.0) } else { 0.0 },
            jolt_m_s: 0.0,
            fire_s: 0.0,
        }
    }

    /// A round or missile fired by the pilot's own mech: the firing rumble held, and a kick for the heavy ones.
    pub fn fired(&self, what: Fired) -> Kick {
        let (trauma, jolt_m_s) = match what {
            Fired::Light => (0.0, 0.0),
            Fired::Heavy => (self.heavy_kick, self.heavy_jolt_m_s),
            Fired::Missile => (self.missile_kick, 0.0),
        };
        Kick {
            trauma,
            jolt_m_s,
            fire_s: self.fire_hold_s,
        }
    }

    /// The pilot's punch or strike landing on anything.
    pub fn melee(&self) -> Kick {
        Kick {
            trauma: self.melee_kick,
            jolt_m_s: self.melee_jolt_m_s,
            fire_s: 0.0,
        }
    }

    /// A footstep: `step_share` of a landing at `step_as_landing_m_s`.
    pub fn step(&self) -> Kick {
        let k = self.landing(self.step_as_landing_m_s);
        Kick {
            trauma: k.trauma * self.step_share,
            jolt_m_s: k.jolt_m_s * self.step_share,
            fire_s: 0.0,
        }
    }

    /// The jets lighting.
    pub fn ignite(&self) -> Kick {
        Kick {
            trauma: self.ignite_kick,
            jolt_m_s: 0.0,
            fire_s: 0.0,
        }
    }

    /// The rumble the body asks for, 0..1: the jets while boosting, the skates by their speed and the ground.
    pub fn rumble(&self, boosting: bool, skating: bool, speed_m_s: f32, paved: bool) -> f32 {
        let jets = if boosting { self.jets_rumble } else { 0.0 };
        let skates = if skating && speed_m_s.is_finite() && self.skate_full_m_s > 0.0 {
            let ground = if paved { self.paved_share } else { 1.0 };
            self.skate_rumble * (speed_m_s.abs() / self.skate_full_m_s).min(1.0) * ground
        } else {
            0.0
        };
        jets.max(skates).clamp(0.0, 1.0)
    }

    /// How much a blast of `energy_mj` at `distance_m` shakes the view, 0..1: full at the blast,
    /// none at its reach, falling off with the square of the distance's share of the reach.
    pub fn kick(&self, energy_mj: f32, distance_m: f32) -> f32 {
        let reach = self.reach_m_per_cbrt_mj * energy_mj.max(0.0).cbrt();
        if reach <= 0.0 || !distance_m.is_finite() {
            return 0.0;
        }
        (1.0 - distance_m / reach).clamp(0.0, 1.0).powi(2)
    }
}

impl Default for FxConfig {
    fn default() -> Self {
        Self {
            tracer_s: 0.008,
            tracer_width_m: 0.25,
            tracer_heavy_width_m: 0.5,
            tracer_glow: 12.0,
            trail_spacing_m: 3.0,
            motor_lumens: 2.0e5,
            plume_length_m: 4.5,
            plume_width_m: 0.9,
            plume_glow: 1.0,
            jet_plume_m: [6.0, 1.6],
            jet_wash_m: 15.0,
            max_particles: 5000,
            max_lights: 24,
            max_fireballs: 64,
            recipes: BTreeMap::new(),
            shake: ShakeConfig::default(),
            stream_per_s: BTreeMap::new(),
            burst_range_m: 400.0,
            sorted: vec!["explosion_ground".into(), "explosion_air".into(), "explosion_water".into()],
            soft_max_particles: 6000,
            soft_m: 2.0,
        }
    }
}

impl Validate for FxConfig {
    fn validate(&self) -> Result<(), String> {
        positive("tracer_s", self.tracer_s)?;
        positive("plume_length_m", self.plume_length_m)?;
        positive("plume_width_m", self.plume_width_m)?;
        within("plume_glow", self.plume_glow, 0.0, 10.0)?;
        positive("jet_plume_m[0]", self.jet_plume_m[0])?;
        positive("jet_plume_m[1]", self.jet_plume_m[1])?;
        positive("jet_wash_m", self.jet_wash_m)?;
        // Every shake number may be 0, which turns its part off (owner, 2026-10-28: "make sure theyre all numbers we
        // can tweak later", "and that 0 works peroperly").
        let s = &self.shake;
        for (name, v) in [
            ("shake.max_deg", s.max_deg),
            ("shake.reach_m_per_cbrt_mj", s.reach_m_per_cbrt_mj),
            ("shake.decay_s", s.decay_s),
            ("shake.rumble_hz", s.rumble_hz),
            ("shake.rumble_ease_s", s.rumble_ease_s),
            ("shake.skate_full_m_s", s.skate_full_m_s),
            ("shake.jolt_max_m", s.jolt_max_m),
            ("shake.jolt_hz", s.jolt_hz),
        ] {
            within(name, v, 0.0, 1000.0)?;
        }
        for (name, v) in [
            ("shake.land_from_m_s", s.land_from_m_s),
            ("shake.land_per_m_s", s.land_per_m_s),
            ("shake.ram_from_m_s", s.ram_from_m_s),
            ("shake.ram_per_m_s", s.ram_per_m_s),
            ("shake.hit_per_mj", s.hit_per_mj),
            ("shake.hit_per_mm", s.hit_per_mm),
            ("shake.fire_hold_s", s.fire_hold_s),
            ("shake.heavy_jolt_m_s", s.heavy_jolt_m_s),
            ("shake.melee_jolt_m_s", s.melee_jolt_m_s),
            ("shake.rumble_deg", s.rumble_deg),
            ("shake.jolt_per_m_s", s.jolt_per_m_s),
            ("shake.step_as_landing_m_s", s.step_as_landing_m_s),
        ] {
            within(name, v, 0.0, 1000.0)?;
        }
        for (name, v) in [
            ("shake.melee_kick", s.melee_kick),
            ("shake.ignite_kick", s.ignite_kick),
            ("shake.fire_rumble", s.fire_rumble),
            ("shake.heavy_kick", s.heavy_kick),
            ("shake.missile_kick", s.missile_kick),
            ("shake.jets_rumble", s.jets_rumble),
            ("shake.skate_rumble", s.skate_rumble),
            ("shake.paved_share", s.paved_share),
            ("shake.step_share", s.step_share),
        ] {
            within(name, v, 0.0, 1.0)?;
        }
        within("shake.jolt_damping", s.jolt_damping, 0.0, 2.0)?;
        positive("burst_range_m", self.burst_range_m)?;
        for (name, per_s) in &self.stream_per_s {
            positive(&format!("stream_per_s.{name}"), *per_s)?;
            if !self.recipes.contains_key(name) {
                return Err(format!("`stream_per_s.{name}` names no recipe"));
            }
        }
        positive("tracer_width_m", self.tracer_width_m)?;
        positive("tracer_heavy_width_m", self.tracer_heavy_width_m)?;
        positive("tracer_glow", self.tracer_glow)?;
        positive("trail_spacing_m", self.trail_spacing_m)?;
        positive("motor_lumens", self.motor_lumens)?;
        for name in RECIPES {
            if !self.recipes.contains_key(name) {
                return Err(format!("`recipes` has no `{name}`, which the game plays"));
            }
        }
        for (name, layers) in &self.recipes {
            for (i, l) in layers.iter().enumerate() {
                l.validate(&format!("recipes.{name}[{i}]"))?;
            }
        }
        for name in &self.sorted {
            if !self.recipes.contains_key(name) {
                return Err(format!("`sorted` names no recipe: {name}"));
            }
        }
        Ok(())
    }
}

/// Burst sizes come in this many steps up to a layer's own count (`particle-steps`).
pub const BURST_STEPS: u32 = 4;

/// How many particles a burst of a layer with `count` particles spawns at `density`, with `room`
/// left under the particle budget: a whole number of steps of `count / BURST_STEPS` (rounded to
/// the nearest, at least one step while any were asked for), cut to whole steps that fit. Each size
/// is its own GPU effect, built when first used; a footstep's density follows its speed, so without
/// steps every stride built a new one mid-fight (`particle-steps`).
pub fn burst_count(count: u32, density: f32, room: u32) -> u32 {
    let step = count.div_ceil(BURST_STEPS).max(1);
    let asked = (count as f32 * density.max(0.0)).round();
    if !asked.is_finite() || asked < 1.0 {
        return 0;
    }
    let steps = ((asked / step as f32).round() as u32).max(1);
    steps.min(room / step) * step
}

#[cfg(test)]
mod tests {

    /// Every shake number at 0 is accepted and turns its part off: nothing shakes, nothing is non-finite.
    #[test]
    fn every_shake_number_at_zero_turns_its_part_off() {
        let z = ShakeConfig {
            max_deg: 0.0,
            reach_m_per_cbrt_mj: 0.0,
            decay_s: 0.0,
            land_from_m_s: 0.0,
            land_per_m_s: 0.0,
            ram_from_m_s: 0.0,
            ram_per_m_s: 0.0,
            hit_per_mj: 0.0,
            hit_per_mm: 0.0,
            fire_rumble: 0.0,
            fire_hold_s: 0.0,
            heavy_kick: 0.0,
            heavy_jolt_m_s: 0.0,
            missile_kick: 0.0,
            melee_kick: 0.0,
            melee_jolt_m_s: 0.0,
            ignite_kick: 0.0,
            step_as_landing_m_s: 0.0,
            step_share: 0.0,
            rumble_deg: 0.0,
            rumble_hz: 0.0,
            rumble_ease_s: 0.0,
            jets_rumble: 0.0,
            skate_rumble: 0.0,
            skate_full_m_s: 0.0,
            paved_share: 0.0,
            jolt_per_m_s: 0.0,
            jolt_max_m: 0.0,
            jolt_hz: 0.0,
            jolt_damping: 0.0,
        };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fx/fx.ron");
        let shipped: FxConfig = load_ron(&root).expect("the shipped fx.ron loads");
        let fx = FxConfig {
            shake: z.clone(),
            ..shipped
        };
        fx.validate().expect("zeros are allowed");
        assert_eq!(z.kick(8.0, 1.0), 0.0);
        for k in [
            z.landing(20.0),
            z.ram(10.0),
            z.hit(300.0, 8.0),
            z.melee(),
            z.ignite(),
            z.step(),
            z.fired(Fired::Heavy),
            z.fired(Fired::Missile),
        ] {
            assert_eq!((k.trauma, k.jolt_m_s, k.fire_s), (0.0, 0.0, 0.0), "{k:?}");
        }
        assert_eq!(z.rumble(true, true, 30.0, false), 0.0);
        assert_eq!(z.decay(0.8, 0.016), 0.0);
        assert_eq!(z.ease_rumble(0.5, 0.0, 0.016), 0.0, "no ease time: at once");
        let mut j = Jolt { y: 0.1, v: 1.0 };
        j.step(&z, 0.016);
        assert_eq!(j, Jolt::default(), "no spring: no dip");
        // And the shipped numbers turned off one at a time stay finite.
        let one = ShakeConfig {
            jolt_damping: 0.0,
            ..ShakeConfig::default()
        };
        let mut j = Jolt::default();
        j.kick(1.0);
        for _ in 0..600 {
            j.step(&one, 1.0 / 60.0);
            assert!(j.y.is_finite() && j.y <= one.jolt_max_m + 1e-6);
        }
    }

    #[test]
    fn a_hard_landing_kicks_and_dips_the_eye_and_a_step_down_does_not() {
        let c = ShakeConfig::default();
        assert_eq!(c.landing(2.0), Kick::default(), "a step down is not a landing");
        let k = c.landing(10.0);
        assert!(k.trauma > 0.3 && k.jolt_m_s > 0.3, "{k:?}");
        assert!(c.landing(20.0).trauma > k.trauma, "harder is more");
    }

    #[test]
    fn a_footstep_kicks_half_a_hard_landing_and_a_zero_share_none() {
        let c = ShakeConfig::default();
        let (step, land) = (c.step(), c.landing(c.step_as_landing_m_s));
        assert!((step.trauma - land.trauma * 0.5).abs() < 1e-6, "{step:?}");
        assert!((step.jolt_m_s - land.jolt_m_s * 0.5).abs() < 1e-6, "{step:?}");
        let off = ShakeConfig { step_share: 0.0, ..c };
        assert_eq!(off.step(), Kick::default());
    }

    #[test]
    fn the_jolt_dips_and_springs_back_to_rest_within_two_seconds() {
        let c = ShakeConfig::default();
        let mut j = Jolt::default();
        j.kick(c.landing(12.0).jolt_m_s);
        let mut deepest = 0.0_f32;
        for _ in 0..120 {
            j.step(&c, 1.0 / 60.0);
            deepest = deepest.max(j.y);
            assert!(j.y <= c.jolt_max_m + 1e-6);
        }
        assert!(deepest > 0.03, "a hard landing dips the eye a few centimetres ({deepest})");
        assert!(j.y.abs() < 0.005, "and it is back by two seconds ({})", j.y);
        j.step(&c, f32::NAN);
        assert!(j.y.is_finite());
    }

    #[test]
    fn hits_punches_and_guns_kick_by_their_power() {
        let c = ShakeConfig::default();
        assert!(
            c.hit(300.0, 8.0).trauma > c.hit(30.0, 0.0).trauma,
            "a heavy shell shakes more than a bullet"
        );
        assert!(c.melee().trauma > 0.3 && c.melee().jolt_m_s > 0.0, "a punch lands hard");
        assert!(c.fired(Fired::Light).fire_s > 0.0, "every round holds the firing rumble");
        assert!(
            c.fired(Fired::Heavy).trauma > 0.0 && c.fired(Fired::Missile).trauma > 0.0,
            "cannons and missiles kick"
        );
        assert_eq!(c.fired(Fired::Light).trauma, 0.0, "a light round only buzzes");
        assert_eq!(c.hit(f32::NAN, 0.0).trauma, 0.0);
    }

    #[test]
    fn jets_and_skates_rumble_and_a_still_mech_does_not() {
        let c = ShakeConfig::default();
        assert_eq!(c.rumble(false, false, 0.0, false), 0.0);
        assert!(c.rumble(true, false, 0.0, false) > 0.5, "the jets rumble");
        let dirt = c.rumble(false, true, 30.0, false);
        assert!(dirt > c.rumble(false, true, 30.0, true), "rough ground buzzes more than paving");
        assert!(dirt > c.rumble(false, true, 10.0, false), "faster buzzes more");
    }

    use super::burst_count;

    #[test]
    fn bursts_come_in_quarter_steps_of_their_count() {
        assert_eq!(burst_count(40, 1.0, 1000), 40);
        assert_eq!(burst_count(40, 0.6, 1000), 20);
        assert_eq!(burst_count(40, 0.7, 1000), 30);
        assert_eq!(burst_count(40, 0.05, 1000), 10, "a light step still shows one step");
        assert_eq!(burst_count(40, 0.0, 1000), 0);
        assert_eq!(burst_count(40, 2.0, 1000), 80);
        assert_eq!(burst_count(3, 1.0, 1000), 3);
    }

    #[test]
    fn a_burst_takes_only_whole_steps_that_fit_the_budget() {
        assert_eq!(burst_count(40, 1.0, 25), 20);
        assert_eq!(burst_count(40, 1.0, 9), 0);
    }

    #[test]
    fn a_footstep_at_any_speed_is_one_of_a_few_sizes() {
        let sizes: std::collections::BTreeSet<u32> = (0..=200).map(|i| burst_count(24, i as f32 / 100.0, 10_000)).collect();
        assert!(sizes.len() <= 2 * super::BURST_STEPS as usize + 1, "{sizes:?}");
    }
    #[test]
    fn far_small_bursts_are_left_out_and_explosions_reach_further() {
        let c = super::FxConfig::default();
        assert_eq!(c.burst_range("foot_dust", 1.0), c.burst_range_m);
        assert_eq!(c.burst_range("explosion_ground", 1.0), c.burst_range_m * 4.0);
        assert!(c.burst_range("impact_mech", 2.0) > c.burst_range("impact_mech", 1.0));
    }

    #[test]
    fn a_blast_shakes_the_view_less_the_further_off_it_is() {
        let s = super::ShakeConfig::default();
        assert_eq!(s.kick(8.0, 0.0), 1.0);
        assert!(s.kick(8.0, 30.0) > s.kick(8.0, 60.0));
        assert_eq!(s.kick(8.0, s.reach_m_per_cbrt_mj * 2.0), 0.0);
        // A bigger blast reaches further.
        assert!(s.kick(27.0, 100.0) > s.kick(1.0, 100.0));
    }

    use super::*;
    use crate::data::{load_ron, parse_ron};
    use std::path::Path;

    #[test]
    fn every_recipe_the_game_asks_for_is_in_the_file() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fx/fx.ron");
        let cfg: FxConfig = load_ron(&p).unwrap_or_else(|e| panic!("{e}"));
        for name in RECIPES {
            assert!(!cfg.recipes[name].is_empty(), "{name} has no layers");
        }
    }

    #[test]
    fn a_sorted_name_must_be_a_recipe() {
        let cfg = FxConfig {
            recipes: RECIPES.iter().map(|n| (n.to_string(), Vec::new())).collect(),
            sorted: vec!["no_such_recipe".into()],
            ..FxConfig::default()
        };
        let e = cfg.validate().unwrap_err();
        assert!(e.contains("no_such_recipe"), "{e}");
    }

    #[test]
    fn a_missing_recipe_stops_startup_naming_it() {
        let e = parse_ron::<FxConfig>("(recipes: {})", Path::new("data/fx/fx.ron"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("data/fx/fx.ron") && e.contains("muzzle_autocannon"), "{e}");
    }

    #[test]
    fn an_unknown_layer_key_is_an_error() {
        let e = parse_ron::<FxConfig>(
            "(recipes: {\"x\": [Star(size_m: 1.0, life_s: 0.1, glow: 3.0)]})",
            Path::new("data/fx/fx.ron"),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("glow"), "{e}");
    }
}
