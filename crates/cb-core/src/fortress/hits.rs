//! How a part takes a hit (design section 3): a round that beats the armour where it struck does its damage,
//! doubled through a weak point. Where it struck and which way it was going are given in the part's own frame
//! (axis-aligned with the fortress's), so the armour is the round's to find: a lift thruster's outboard plate
//! stops everything, its inboard side is 60 mm, its nozzle and grille are weak; a hull thruster's port counts only
//! once its doors are open and only to a round within 30 degrees of the fortress's vertical.

use super::{BeamPhase, Fortress, FortressData};
use glam::Vec3;
use serde::Deserialize;

/// A kind's weak point, and for a lift thruster its plate.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub enum Weak {
    /// None: its armour everywhere.
    #[default]
    None,
    /// A lift thruster: the outboard half is a `plate_mm` plate; below `below_m` (the nozzle) and above
    /// `above_m` (the intake grille) it is weak; the inboard side is its armour.
    Lift {
        /// The outboard plate, millimetres.
        plate_mm: f32,
        /// The nozzle: hits this far below its middle or more.
        below_m: f32,
        /// The grille: hits this far above its middle or more.
        above_m: f32,
    },
    /// A hull thruster: weak through its ports once the doors are open, to a round within `within_deg` of the
    /// fortress's vertical; anything else meets its armour.
    Port {
        /// Degrees from vertical.
        within_deg: f32,
    },
    /// Weak while it fires a burst (a missile launcher's open tube faces).
    WhileFiring,
    /// Weak while it charges (a beam turret's lens).
    WhileCharging,
    /// Weak above `y_m` in its own frame (the bridge's eye lenses).
    Above {
        /// Height.
        y_m: f32,
    },
}

/// What a hit did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitOutcome {
    /// It did not get through: the armour it met.
    NoPen {
        /// Millimetres.
        armour_mm: f32,
    },
    /// It got through.
    Hurt {
        /// Damage done.
        damage: f32,
        /// Through the weak point.
        weak: bool,
    },
    /// It got through and destroyed the part.
    Destroyed {
        /// Damage done.
        damage: f32,
        /// Through the weak point.
        weak: bool,
    },
    /// The part was already destroyed.
    Wreck,
}

impl Fortress {
    /// A round of `penetration_mm` and `damage` strikes part `part` at `local_at` going `local_dir`, both in the
    /// part's own frame.
    pub fn hit(&mut self, d: &FortressData, part: usize, local_at: Vec3, local_dir: Vec3, penetration_mm: f32, damage: f32) -> HitOutcome {
        let Some(state) = self.parts.get(part).copied() else {
            return HitOutcome::Wreck;
        };
        if !state.alive {
            return HitOutcome::Wreck;
        }
        let k = d.kind(part);
        let side = f32::from(d.shape.parts[part].side);
        let weak = match k.weak {
            Weak::None => false,
            Weak::Lift { below_m, above_m, .. } => local_at.y <= -below_m || local_at.y >= above_m,
            Weak::Port { within_deg } => {
                let open = self.doors(&d.cfg) >= 1.0;
                open && local_dir.normalize_or_zero().y.abs() >= within_deg.to_radians().cos()
            }
            Weak::WhileFiring => k.gun.as_ref().is_some_and(|g| state.gun.firing(g)),
            Weak::WhileCharging => state.gun.beam == BeamPhase::Charging,
            Weak::Above { y_m } => local_at.y >= y_m,
        };
        let armour = if weak {
            k.weak_mm
        } else {
            match k.weak {
                Weak::Lift { plate_mm, .. } if local_at.x * side > 0.0 => plate_mm,
                _ => k.armour_mm,
            }
        };
        if penetration_mm < armour {
            return HitOutcome::NoPen { armour_mm: armour };
        }
        let damage = damage * if weak { k.weak_damage } else { 1.0 };
        let p = &mut self.parts[part];
        p.hp -= damage;
        if p.hp <= 0.0 {
            p.hp = 0.0;
            p.alive = false;
            return HitOutcome::Destroyed { damage, weak };
        }
        HitOutcome::Hurt { damage, weak }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fortress, kill, run};
    use super::*;

    const AUTOCANNON: (f32, f32) = (90.0, 12.0);
    const CANNON: (f32, f32) = (260.0, 90.0);

    #[test]
    fn a_round_on_the_outboard_plate_does_nothing() {
        let (d, mut f) = fortress();
        for id in ["L2", "R2"] {
            let i = d.part(id).unwrap();
            let side = f32::from(d.shape.parts[i].side);
            let out = f.hit(
                &d,
                i,
                Vec3::new(16.0 * side, 0.0, 0.0),
                Vec3::new(-side, 0.0, 0.0),
                CANNON.0,
                CANNON.1,
            );
            assert_eq!(out, HitOutcome::NoPen { armour_mm: 300.0 }, "{id}");
        }
    }

    #[test]
    fn the_autocannon_cannot_hurt_a_cowl_but_doubles_on_a_grille() {
        // The inboard side is 60 mm: the autocannon (90 mm) gets through; its nozzle and grille, 40 mm, double it.
        let (d, mut f) = fortress();
        let i = d.part("L2").unwrap();
        let inboard = f.hit(&d, i, Vec3::new(-16.0, 0.0, 0.0), Vec3::X, AUTOCANNON.0, AUTOCANNON.1);
        assert_eq!(inboard, HitOutcome::Hurt { damage: 12.0, weak: false });
        let grille = f.hit(&d, i, Vec3::new(0.0, 21.0, 0.0), Vec3::NEG_Y, AUTOCANNON.0, AUTOCANNON.1);
        assert_eq!(grille, HitOutcome::Hurt { damage: 24.0, weak: true });
        let nozzle = f.hit(&d, i, Vec3::new(0.0, -21.0, 0.0), Vec3::Y, AUTOCANNON.0, AUTOCANNON.1);
        assert_eq!(nozzle, HitOutcome::Hurt { damage: 24.0, weak: true });
        let outboard = f.hit(&d, i, Vec3::new(16.0, 0.0, 0.0), Vec3::NEG_X, AUTOCANNON.0, AUTOCANNON.1);
        assert!(matches!(outboard, HitOutcome::NoPen { .. }));
    }

    #[test]
    fn a_hull_port_takes_damage_only_from_within_30_deg_of_vertical() {
        let (d, mut f) = fortress();
        let i = d.part("HL2").unwrap();
        // sealed: nothing gets in
        assert!(matches!(
            f.hit(&d, i, Vec3::ZERO, Vec3::Y, CANNON.0, CANNON.1),
            HitOutcome::NoPen { .. }
        ));
        kill(&d, &mut f, &["L1", "L2", "L3"]);
        run(&d, &mut f, 10.0);
        let below = f.hit(&d, i, Vec3::new(0.0, -30.0, 0.0), Vec3::Y, CANNON.0, CANNON.1);
        assert_eq!(below, HitOutcome::Hurt { damage: 180.0, weak: true });
        let slant = Vec3::new(0.7, 0.7, 0.0); // 45 degrees off vertical
        assert!(matches!(f.hit(&d, i, Vec3::ZERO, slant, CANNON.0, CANNON.1), HitOutcome::NoPen { armour_mm } if armour_mm >= 300.0));
        let steep = Vec3::new(0.3, -1.0, 0.0); // about 17 degrees off, from above
        assert!(matches!(
            f.hit(&d, i, Vec3::ZERO, steep, CANNON.0, CANNON.1),
            HitOutcome::Hurt { weak: true, .. }
        ));
    }

    #[test]
    fn a_destroyed_part_takes_no_more() {
        let (d, mut f) = fortress();
        let i = d.part("flak1").unwrap();
        let mut last = HitOutcome::Wreck;
        for _ in 0..10 {
            last = f.hit(&d, i, Vec3::ZERO, Vec3::X, CANNON.0, CANNON.1);
            if matches!(last, HitOutcome::Destroyed { .. }) {
                break;
            }
        }
        assert!(matches!(last, HitOutcome::Destroyed { .. }));
        assert!(!f.parts[i].alive);
        assert_eq!(f.hit(&d, i, Vec3::ZERO, Vec3::X, CANNON.0, CANNON.1), HitOutcome::Wreck);
    }
}
