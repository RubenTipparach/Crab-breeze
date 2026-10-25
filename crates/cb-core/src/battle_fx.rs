//! The numbers of the battle's lasting marks and damage effects (openspec changes `blast-scorch` and `mech-damage-fx`):
//! scorch decals over craters on soil, hit marks on mechs, when a damaged mech smokes and burns, and the dust a falling
//! body throws. `data/fx/battle_fx.ron`. The core decides nothing with them; it owns them so they are validated with
//! the rest of the data, and `hit_kind` is the one rule for which mark a hit leaves.

use crate::data::Validate;
use serde::Deserialize;

/// Scorch decals over craters on soil.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ScorchConfig {
    /// A decal's side over the crater's radius.
    pub scorch_per_radius: f32,
    /// Most standing at once; the oldest goes first.
    pub max_scorches: usize,
    /// How far below the decal's plane a surface still takes it, metres (the bowl).
    pub depth_fade_m: f32,
    /// How long one stands, seconds.
    pub life_s: f32,
}

impl Default for ScorchConfig {
    fn default() -> Self {
        Self {
            scorch_per_radius: 2.6,
            max_scorches: 64,
            depth_fade_m: 6.0,
            life_s: 240.0,
        }
    }
}

/// Hit marks on mechs.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct HitMarksConfig {
    /// Most on one mech; the oldest goes first.
    pub per_mech: usize,
    /// Most in the world.
    pub max: usize,
    /// Side of a puncture, a dent, a scorch and a gouge, metres (on the stand-in's scale).
    pub size_m: [f32; 4],
    /// Penetration from which a hit dents rather than punctures, millimetres.
    pub dent_from_mm: f32,
    /// Blast energy from which a hit leaves a scorch, megajoules.
    pub scorch_from_mj: f32,
    /// One small hit in this many leaves a gouge.
    pub gouge_every: u32,
    /// How far from its plane a surface still takes a mark, metres.
    pub depth_fade_m: f32,
}

impl Default for HitMarksConfig {
    fn default() -> Self {
        Self {
            per_mech: 48,
            max: 400,
            size_m: [0.7, 1.8, 3.5, 1.6],
            dent_from_mm: 100.0,
            scorch_from_mj: 1.0,
            gouge_every: 4,
            depth_fade_m: 0.6,
        }
    }
}

/// When a damaged mech smokes and burns, and the dust of a fall.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DamageFxConfig {
    /// A part with less than this share of its structure smokes; a mech with less in all burns.
    pub critical_frac: f32,
    /// How long a thrown limb burns on the ground, seconds.
    pub limb_burn_s: f32,
    /// How long a stump burns, seconds.
    pub stump_burn_s: f32,
    /// A fall's dust scale is the cube root of the body's mass over this, tonnes.
    pub dust_per_cbrt_t: f32,
    /// A body this close above the ground has landed, metres.
    pub land_m: f32,
}

impl Default for DamageFxConfig {
    fn default() -> Self {
        Self {
            critical_frac: 0.25,
            limb_burn_s: 30.0,
            stump_burn_s: 25.0,
            dust_per_cbrt_t: 10.0,
            land_m: 1.2,
        }
    }
}

/// `data/fx/battle_fx.ron`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BattleFxConfig {
    /// Scorches over craters.
    pub scorch: ScorchConfig,
    /// Hit marks on mechs.
    pub hit_marks: HitMarksConfig,
    /// Damage fire, smoke and dust.
    pub damage: DamageFxConfig,
}

/// The kind of mark a hit leaves: a row of the `hit_marks` sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitMark {
    /// A small round's hole.
    Puncture = 0,
    /// A heavy round's dent.
    Dent = 1,
    /// A blast's burn.
    Scorch = 2,
    /// A glancing round's scratch.
    Gouge = 3,
}

impl HitMarksConfig {
    /// The mark a hit leaves: a blast scorches, a heavy round dents, a small one punctures, and one small hit in
    /// `gouge_every` (by the shot's `seed`) glances off in a gouge.
    pub fn hit_kind(&self, penetration_mm: f32, blast_mj: f32, seed: u32) -> HitMark {
        if blast_mj >= self.scorch_from_mj {
            HitMark::Scorch
        } else if penetration_mm >= self.dent_from_mm {
            HitMark::Dent
        } else if self.gouge_every > 0 && seed.is_multiple_of(self.gouge_every) {
            HitMark::Gouge
        } else {
            HitMark::Puncture
        }
    }
}

fn positive(name: &str, v: f32) -> Result<(), String> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(format!("`{name}` must be a positive number, got {v}"))
    }
}

impl Validate for BattleFxConfig {
    fn validate(&self) -> Result<(), String> {
        let s = &self.scorch;
        positive("scorch.scorch_per_radius", s.scorch_per_radius)?;
        positive("scorch.depth_fade_m", s.depth_fade_m)?;
        positive("scorch.life_s", s.life_s)?;
        if !(1..=1024).contains(&s.max_scorches) {
            return Err(format!("`scorch.max_scorches` must be within 1 and 1024, got {}", s.max_scorches));
        }
        let h = &self.hit_marks;
        for (i, v) in h.size_m.iter().enumerate() {
            positive(&format!("hit_marks.size_m[{i}]"), *v)?;
        }
        positive("hit_marks.dent_from_mm", h.dent_from_mm)?;
        positive("hit_marks.scorch_from_mj", h.scorch_from_mj)?;
        positive("hit_marks.depth_fade_m", h.depth_fade_m)?;
        if h.per_mech == 0 || h.per_mech > h.max || h.max > 4096 {
            return Err(format!(
                "`hit_marks`: need 0 < per_mech ({}) <= max ({}) <= 4096",
                h.per_mech, h.max
            ));
        }
        let d = &self.damage;
        if !(d.critical_frac > 0.0 && d.critical_frac < 1.0) {
            return Err(format!("`damage.critical_frac` must be within 0 and 1, got {}", d.critical_frac));
        }
        positive("damage.limb_burn_s", d.limb_burn_s)?;
        positive("damage.stump_burn_s", d.stump_burn_s)?;
        positive("damage.dust_per_cbrt_t", d.dust_per_cbrt_t)?;
        positive("damage.land_m", d.land_m)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blast_scorches_a_heavy_round_dents_and_small_rounds_puncture_or_gouge() {
        let h = HitMarksConfig::default();
        assert_eq!(h.hit_kind(40.0, 6.0, 1), HitMark::Scorch);
        assert_eq!(h.hit_kind(300.0, 0.2, 1), HitMark::Dent);
        assert_eq!(h.hit_kind(60.0, 0.0, 1), HitMark::Puncture);
        assert_eq!(h.hit_kind(60.0, 0.0, 8), HitMark::Gouge);
    }

    #[test]
    fn the_shipped_battle_fx_file_loads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fx/battle_fx.ron");
        let c: BattleFxConfig = crate::data::load_ron(&root).unwrap();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn a_mech_cannot_keep_more_marks_than_the_world() {
        let mut c = BattleFxConfig::default();
        c.hit_marks.per_mech = 500;
        assert!(c.validate().is_err());
    }
}
