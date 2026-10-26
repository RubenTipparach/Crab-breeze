//! What an air base's hangar bay does for a mech (openspec change `air-bases`, design section 4): the
//! repair and the rearm, part by part in a fixed order, and the report the MECH MFD reads back. Free for
//! now (owner, 2026-10-25: "just need to enter airbase hangar, and I would get repairs and free ammo (free
//! repairs fornow)"), so nothing is charged. Pure: the game calls it when the piloted mech stops in a bay
//! and shows exactly what it returns (CLAUDE.md 6.1).

use crate::mech::{Damage, PartKind, PartSpec};
use crate::weapons::WeaponSpec;

/// The order parts are repaired in (design section 4): torso, legs, arms, head.
pub const REPAIR_ORDER: [PartKind; 6] = [
    PartKind::Torso,
    PartKind::LegL,
    PartKind::LegR,
    PartKind::ArmL,
    PartKind::ArmR,
    PartKind::Head,
];

/// What a hangar bay did to a mech.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Service {
    /// Parts brought back to full, in the order they were repaired.
    pub repaired: Vec<PartKind>,
    /// Structure points put back, summed over the parts.
    pub structure_restored: f32,
    /// Parts lost and not refitted (a lost limb stays lost: the bay has no spare).
    pub lost: Vec<PartKind>,
    /// Rounds put back, summed over the weapons with a limit.
    pub rounds_restored: u32,
}

impl Service {
    /// True when the bay had nothing to do.
    pub fn is_empty(&self) -> bool {
        self.repaired.is_empty() && self.rounds_restored == 0
    }

    /// The MECH MFD's lines for it: what was repaired and rearmed (or that nothing was needed), and what
    /// it cost (nothing: repairs are free for now).
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.repaired.is_empty() {
            out.push("REPAIR: NONE NEEDED".to_string());
        } else {
            out.push(format!(
                "REPAIRED {} PART{} +{:.0}",
                self.repaired.len(),
                if self.repaired.len() == 1 { "" } else { "S" },
                self.structure_restored
            ));
        }
        if !self.lost.is_empty() {
            out.push(format!("{} LOST, NOT REFITTED", self.lost.len()));
        }
        out.push(if self.rounds_restored == 0 {
            "REARM: FULL".to_string()
        } else {
            format!("REARMED +{} RDS", self.rounds_restored)
        });
        out.push("COST 0 CR".to_string());
        out
    }
}

/// Repairs every part not lost back to its full structure, in `REPAIR_ORDER`, and fills every weapon with a
/// round limit back to its full load. `rounds_left` is the mech's per-weapon count (`None`: no limit, or not
/// yet drawn on); it comes back holding the full load of each weapon.
pub fn service(damage: &mut Damage, parts: &[PartSpec], rounds_left: &mut Vec<Option<u32>>, weapons: &[WeaponSpec]) -> Service {
    let mut out = Service::default();
    for kind in REPAIR_ORDER {
        for (i, p) in parts.iter().enumerate().filter(|(_, p)| p.kind == kind) {
            let (Some(s), Some(lost)) = (damage.structure.get_mut(i), damage.lost.get(i)) else {
                continue;
            };
            if *lost {
                out.lost.push(kind);
                continue;
            }
            let missing = p.structure - *s;
            if missing > 1e-3 && missing.is_finite() {
                *s = p.structure;
                out.structure_restored += missing;
                out.repaired.push(kind);
            }
        }
    }
    let before: u32 = weapons
        .iter()
        .enumerate()
        .filter_map(|(i, w)| w.rounds.map(|full| rounds_left.get(i).copied().flatten().unwrap_or(full).min(full)))
        .sum();
    let full: u32 = weapons.iter().filter_map(|w| w.rounds).sum();
    out.rounds_restored = full - before.min(full);
    *rounds_left = weapons.iter().map(|w| w.rounds).collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mech::default_parts;
    use crate::weapons::default_weapons;

    #[test]
    fn a_bay_repairs_every_part_not_lost_torso_first_and_says_so() {
        let parts = default_parts();
        let mut d = Damage::new(&parts);
        let at = |k: PartKind| parts.iter().position(|p| p.kind == k).unwrap();
        d.structure[at(PartKind::Head)] *= 0.5;
        d.structure[at(PartKind::Torso)] *= 0.25;
        d.structure[at(PartKind::ArmL)] = 0.0;
        d.lost[at(PartKind::ArmL)] = true;
        let mut rounds = Vec::new();
        let s = service(&mut d, &parts, &mut rounds, &default_weapons());
        assert_eq!(s.repaired, vec![PartKind::Torso, PartKind::Head], "torso first, head last");
        assert_eq!(s.lost, vec![PartKind::ArmL], "a lost arm stays lost");
        assert_eq!(d.structure[at(PartKind::Torso)], parts[at(PartKind::Torso)].structure);
        let restored = parts[at(PartKind::Torso)].structure * 0.75 + parts[at(PartKind::Head)].structure * 0.5;
        assert!((s.structure_restored - restored).abs() < 1e-3);
        assert!(s.lines().iter().any(|l| l == "COST 0 CR"), "free for now");
    }

    #[test]
    fn a_bay_fills_the_missiles_back_to_their_load() {
        let parts = default_parts();
        let weapons = default_weapons();
        let mut d = Damage::new(&parts);
        let missiles = weapons.iter().position(|w| w.rounds.is_some()).unwrap();
        let mut rounds: Vec<Option<u32>> = weapons.iter().map(|w| w.rounds).collect();
        rounds[missiles] = Some(7);
        let s = service(&mut d, &parts, &mut rounds, &weapons);
        assert_eq!(s.rounds_restored, weapons[missiles].rounds.unwrap() - 7);
        assert_eq!(rounds[missiles], weapons[missiles].rounds);
        assert!(s.repaired.is_empty());
        assert_eq!(s.lines()[0], "REPAIR: NONE NEEDED");
    }

    #[test]
    fn a_whole_full_mech_needs_nothing() {
        let parts = default_parts();
        let weapons = default_weapons();
        let mut d = Damage::new(&parts);
        let mut rounds: Vec<Option<u32>> = weapons.iter().map(|w| w.rounds).collect();
        assert!(service(&mut d, &parts, &mut rounds, &weapons).is_empty());
        let mut never_fired = Vec::new();
        assert!(
            service(&mut d, &parts, &mut never_fired, &weapons).is_empty(),
            "guns not yet drawn on are full"
        );
    }
}
