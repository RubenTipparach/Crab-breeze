//! Mech damage by part (openspec change `mech-damage`): each part has armour and structure in data;
//! a hit that beats the armour takes structure, a hit that does not only chips it; a part at zero is
//! lost. Losing an arm or the head leaves the mech fighting; losing a leg makes it limp; losing the
//! torso destroys it. Pure.

use crate::data::{Validate, positive};
use serde::Deserialize;

/// What a part is, which decides what losing it does.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum PartKind {
    /// Left arm and its weapon.
    ArmL,
    /// Right arm and its weapon.
    ArmR,
    /// Left leg: the mech limps.
    LegL,
    /// Right leg: the mech limps.
    LegR,
    /// The sensor head: the main cameras.
    Head,
    /// The torso: the mech is destroyed.
    Torso,
}

/// A part in data.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartSpec {
    /// Its kind.
    pub kind: PartKind,
    /// The bone cut off when it is lost (it and everything under it); empty for none.
    #[serde(default)]
    pub cut_bone: String,
    /// Armour, millimetres of rolled steel equivalent.
    pub armour_mm: f32,
    /// Structure points.
    pub structure: f32,
}

/// The default parts of a mech on the rig contract.
pub fn default_parts() -> Vec<PartSpec> {
    let p = |kind, cut: &str, armour_mm, structure| PartSpec {
        kind,
        cut_bone: cut.into(),
        armour_mm,
        structure,
    };
    vec![
        p(PartKind::ArmL, "upper_arm_L", 80.0, 80.0),
        p(PartKind::ArmR, "upper_arm_R", 80.0, 80.0),
        p(PartKind::LegL, "shin_L", 120.0, 140.0),
        p(PartKind::LegR, "shin_R", 120.0, 140.0),
        p(PartKind::Head, "head", 60.0, 40.0),
        p(PartKind::Torso, "", 150.0, 200.0),
    ]
}

/// Validates a part list.
pub fn validate_parts(parts: &[PartSpec]) -> Result<(), String> {
    for p in parts {
        positive(&format!("parts[{:?}].armour_mm", p.kind), p.armour_mm)?;
        positive(&format!("parts[{:?}].structure", p.kind), p.structure)?;
    }
    Ok(())
}

impl Validate for PartSpec {
    fn validate(&self) -> Result<(), String> {
        validate_parts(std::slice::from_ref(self))
    }
}

/// A hit on one part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The part's index in the mech's part list.
    pub part: usize,
    /// Penetration, millimetres of rolled steel equivalent.
    pub penetration_mm: f32,
    /// Structure points it takes if it gets through.
    pub damage: f32,
}

/// A mech's damage.
#[derive(Clone, Debug, PartialEq)]
pub struct Damage {
    /// Structure left, per part.
    pub structure: Vec<f32>,
    /// Lost, per part.
    pub lost: Vec<bool>,
}

/// Share of damage that a hit stopped by the armour still does (spalling and shock): small, so a
/// gun that cannot beat the armour wears a part down only over hundreds of hits.
const CHIP: f32 = 0.02;

impl Damage {
    /// A whole mech.
    pub fn new(parts: &[PartSpec]) -> Self {
        Self {
            structure: parts.iter().map(|p| p.structure).collect(),
            lost: vec![false; parts.len()],
        }
    }

    /// Resolves a hit. Returns the index of a part lost by it, if any.
    pub fn resolve(&mut self, parts: &[PartSpec], hit: Hit) -> Option<usize> {
        let i = hit.part;
        if i >= parts.len() || self.lost[i] || !(hit.damage.is_finite() && hit.penetration_mm.is_finite()) {
            return None;
        }
        let through = if hit.penetration_mm > parts[i].armour_mm { 1.0 } else { CHIP };
        self.structure[i] = (self.structure[i] - hit.damage.max(0.0) * through).max(0.0);
        (self.structure[i] == 0.0).then(|| {
            self.lost[i] = true;
            i
        })
    }

    /// True when a part of `kind` is lost.
    pub fn is_lost(&self, parts: &[PartSpec], kind: PartKind) -> bool {
        parts.iter().zip(&self.lost).any(|(p, l)| p.kind == kind && *l)
    }

    /// Legs still on: [left, right].
    pub fn legs(&self, parts: &[PartSpec]) -> [bool; 2] {
        [!self.is_lost(parts, PartKind::LegL), !self.is_lost(parts, PartKind::LegR)]
    }

    /// True when the torso is gone.
    pub fn destroyed(&self, parts: &[PartSpec]) -> bool {
        self.is_lost(parts, PartKind::Torso)
    }

    /// How whole part `i` is: its structure left over its full structure, 0 to 1, and 0 when it is lost or out of
    /// range. The one measure the cockpit shows a part's hurt by (openspec change `mfd-damage-page`).
    pub fn health(&self, parts: &[PartSpec], i: usize) -> f32 {
        match (parts.get(i), self.structure.get(i), self.lost.get(i)) {
            (Some(p), Some(s), Some(false)) if s.is_finite() => (s / p.structure.max(1.0)).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }

    /// How whole the mech's part of `kind` is (the worse, when it has two of a kind), and whether one is lost; `None`
    /// when it has no such part.
    pub fn health_of(&self, parts: &[PartSpec], kind: PartKind) -> Option<(f32, bool)> {
        (0..parts.len())
            .filter(|&i| parts[i].kind == kind)
            .map(|i| (self.health(parts, i), self.lost.get(i).copied().unwrap_or(true)))
            .reduce(|a, b| (a.0.min(b.0), a.1 || b.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armour_stops_a_weak_hit() {
        let parts = default_parts();
        let mut d = Damage::new(&parts);
        d.resolve(
            &parts,
            Hit {
                part: 0,
                penetration_mm: 40.0,
                damage: 50.0,
            },
        );
        assert_eq!(d.structure[0], 79.0, "only a fiftieth gets through");
    }

    #[test]
    fn an_arm_at_zero_structure_is_lost_and_the_rest_works() {
        let parts = default_parts();
        let mut d = Damage::new(&parts);
        assert_eq!(
            d.resolve(
                &parts,
                Hit {
                    part: 0,
                    penetration_mm: 200.0,
                    damage: 100.0
                }
            ),
            Some(0)
        );
        assert!(d.is_lost(&parts, PartKind::ArmL));
        assert_eq!(d.legs(&parts), [true, true]);
        assert!(!d.destroyed(&parts));
    }

    #[test]
    fn health_is_structure_left_over_full_and_zero_once_lost() {
        let parts = default_parts();
        let mut d = Damage::new(&parts);
        assert_eq!(d.health(&parts, 0), 1.0);
        d.resolve(
            &parts,
            Hit {
                part: 0,
                penetration_mm: 200.0,
                damage: 30.0,
            },
        );
        assert!((d.health(&parts, 0) - (parts[0].structure - 30.0) / parts[0].structure).abs() < 1e-6);
        assert_eq!(d.health_of(&parts, PartKind::ArmL), Some((d.health(&parts, 0), false)));
        d.resolve(
            &parts,
            Hit {
                part: 0,
                penetration_mm: 200.0,
                damage: 1000.0,
            },
        );
        assert_eq!(d.health(&parts, 0), 0.0);
        assert_eq!(d.health_of(&parts, PartKind::ArmL), Some((0.0, true)));
        assert_eq!(d.health(&parts, 99), 0.0, "a part out of range is not whole");
    }

    #[test]
    fn losing_the_torso_destroys_the_mech() {
        let parts = default_parts();
        let mut d = Damage::new(&parts);
        d.resolve(
            &parts,
            Hit {
                part: 5,
                penetration_mm: 300.0,
                damage: 500.0,
            },
        );
        assert!(d.destroyed(&parts));
    }
}
