//! Effect recipes (openspec change `combat-fx`): `data/fx/fx.ron` names every effect the game plays
//! (a muzzle flash, an impact on armour, a ground blast) as layers of particles, lights and shapes,
//! with their numbers. Data only: `cb-app::fx` draws them. It lives here so the one data loader
//! checks it at startup and `cb-tools validate` checks it before a push.

use crate::data::{Validate, finite, finite3, positive, within};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Every recipe the game asks for. A file without one of these stops startup.
pub const RECIPES: [&str; 31] = [
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
    /// Lit smoke puffs (openspec change `smoke-puffs`): 3D blobs lit like the clouds, each its own
    /// draw so it sorts on its own; they grow, rise, drift and fade (`crate::smoke::Puff`).
    Puffs {
        /// Puffs (times the effect's density).
        count: u32,
        /// Starting diameter.
        size_m: f32,
        /// Diameter added per second.
        grow_m_s: f32,
        /// Speed thrown at, mean.
        speed_m_s: f32,
        /// Cone spread, 0 straight to 1 a hemisphere.
        spread: f32,
        /// Upward drift of the warm smoke.
        rise_m_s: f32,
        /// Lifetime.
        life_s: f32,
        /// Share of the life spent fading out at the end, 0..1.
        fade: f32,
        /// Colour, linear RGB 0..1 (lit by the scene).
        colour: [f32; 3],
        /// Opacity at full strength, 0..1.
        opacity: f32,
        /// Moved this far along the effect's direction.
        #[serde(default)]
        offset_m: f32,
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
            Layer::Puffs {
                size_m,
                grow_m_s,
                speed_m_s,
                spread,
                rise_m_s,
                life_s,
                fade,
                colour,
                opacity,
                offset_m,
                ..
            } => {
                positive(&f("size_m"), *size_m)?;
                finite(&f("grow_m_s"), *grow_m_s)?;
                finite(&f("speed_m_s"), *speed_m_s)?;
                within(&f("spread"), *spread, 0.0, 2.0)?;
                finite(&f("rise_m_s"), *rise_m_s)?;
                positive(&f("life_s"), *life_s)?;
                within(&f("fade"), *fade, 0.0, 1.0)?;
                finite3(&f("colour"), *colour)?;
                within(&f("opacity"), *opacity, 0.0, 1.0)?;
                finite(&f("offset_m"), *offset_m)
            }
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
}

impl Default for ShakeConfig {
    fn default() -> Self {
        Self {
            max_deg: 1.2,
            reach_m_per_cbrt_mj: 60.0,
            decay_s: 0.7,
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
        }
    }
}

impl Validate for FxConfig {
    fn validate(&self) -> Result<(), String> {
        positive("tracer_s", self.tracer_s)?;
        positive("plume_length_m", self.plume_length_m)?;
        positive("plume_width_m", self.plume_width_m)?;
        positive("jet_plume_m[0]", self.jet_plume_m[0])?;
        positive("jet_plume_m[1]", self.jet_plume_m[1])?;
        positive("jet_wash_m", self.jet_wash_m)?;
        positive("shake.max_deg", self.shake.max_deg)?;
        positive("shake.reach_m_per_cbrt_mj", self.shake.reach_m_per_cbrt_mj)?;
        positive("shake.decay_s", self.shake.decay_s)?;
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
    fn a_puffs_layer_is_validated() {
        let good = Layer::Puffs {
            count: 6,
            size_m: 3.0,
            grow_m_s: 2.0,
            speed_m_s: 4.0,
            spread: 0.8,
            rise_m_s: 1.5,
            life_s: 6.0,
            fade: 0.4,
            colour: [0.3, 0.3, 0.3],
            opacity: 0.8,
            offset_m: 0.0,
        };
        assert!(good.validate("x").is_ok());
        let Layer::Puffs { .. } = good else { unreachable!() };
        let bad = match good.clone() {
            Layer::Puffs {
                count,
                size_m,
                grow_m_s,
                speed_m_s,
                spread,
                rise_m_s,
                life_s,
                colour,
                offset_m,
                ..
            } => Layer::Puffs {
                count,
                size_m,
                grow_m_s,
                speed_m_s,
                spread,
                rise_m_s,
                life_s,
                fade: 1.5,
                colour,
                opacity: 0.8,
                offset_m,
            },
            _ => unreachable!(),
        };
        assert!(bad.validate("x").unwrap_err().contains("fade"));
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
