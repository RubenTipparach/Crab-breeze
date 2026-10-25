//! Effect recipes (openspec change `combat-fx`): `data/fx/fx.ron` names every effect the game plays
//! (a muzzle flash, an impact on armour, a ground blast) as layers of particles, lights and shapes,
//! with their numbers. Data only: `cb-app::fx` draws them. It lives here so the one data loader
//! checks it at startup and `cb-tools validate` checks it before a push.

use crate::data::{Validate, finite, finite3, positive, within};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Every recipe the game asks for. A file without one of these stops startup.
pub const RECIPES: [&str; 29] = [
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
            max_particles: 5000,
            max_lights: 24,
            max_fireballs: 64,
            recipes: BTreeMap::new(),
            shake: ShakeConfig::default(),
            stream_per_s: BTreeMap::new(),
            burst_range_m: 400.0,
        }
    }
}

impl Validate for FxConfig {
    fn validate(&self) -> Result<(), String> {
        positive("tracer_s", self.tracer_s)?;
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
