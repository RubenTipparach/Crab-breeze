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
