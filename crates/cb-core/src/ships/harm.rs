//! Who answers for harm to a boat, and who the fighting picks on (openspec change `harbours`, design sections 5
//! and 6). Harm costs standing by the same rule as cars (survey W37), civilians (W42) and civil aircraft (W34):
//! only a crewed boat, and only the pilot's own rounds, blasts or mech. It is reported once when the pilot first
//! harms a boat and once more if the pilot's harm sinks it, naming the nearest air base. Crabs take a crewed ship
//! near them when the pilot is far; raiders' gunboats go for their job's boats until the pilot comes within range.

use super::Boat;
use glam::Vec2;

/// Who did it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// A round or blast fired by the pilot's mech.
    PilotFire,
    /// The pilot's mech shoving or swamping it.
    PilotMech,
    /// Anyone else: the enemy, crabs, gunboats, the sea.
    Other,
}

/// A harm report: what standing reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Harm {
    /// The pilot's harm sank it.
    pub sunk: bool,
}

/// The standing rule: whether this harm to `boat` is reported, given that it was `crewed` when it was harmed and
/// whether this harm `sank` it. Reported once on the pilot's first harm, and once more if the pilot's harm sinks
/// it; never for an uncrewed boat or anyone else's harm.
pub fn report(boat: &mut Boat, cause: Cause, crewed: bool, sank: bool) -> Option<Harm> {
    if cause == Cause::Other || !crewed {
        return None;
    }
    if !boat.harmed_by_pilot {
        boat.harmed_by_pilot = true;
        boat.reported_sunk = sank;
        return Some(Harm { sunk: sank });
    }
    if sank && !boat.reported_sunk {
        boat.reported_sunk = true;
        return Some(Harm { sunk: true });
    }
    None
}

/// The index of the nearest of `points` to `p` (ties to the lower index), if there are any.
pub fn nearest(points: &[Vec2], p: Vec2) -> Option<usize> {
    (0..points.len()).min_by(|a, b| points[*a].distance(p).total_cmp(&points[*b].distance(p)).then(a.cmp(b)))
}

/// A boat as the fighting sees it: its id, where it is, and whether it is crewed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prey {
    /// Its id.
    pub id: u32,
    /// Where it is (x, z).
    pub pos: Vec2,
    /// Crewed now.
    pub crewed: bool,
}

/// The ship a crab at `crab` goes for: the nearest crewed one within `reach_m` (ties to the lower id), but only
/// while the pilot at `pilot` is farther than `pilot_m` (the pilot near, it fights the pilot).
pub fn crab_prey(crab: Vec2, pilot: Vec2, ships: &[Prey], reach_m: f32, pilot_m: f32) -> Option<u32> {
    if crab.distance(pilot) <= pilot_m {
        return None;
    }
    ships
        .iter()
        .filter(|s| s.crewed && s.pos.distance(crab) <= reach_m)
        .min_by(|a, b| a.pos.distance(crab).total_cmp(&b.pos.distance(crab)).then(a.id.cmp(&b.id)))
        .map(|s| s.id)
}

/// What a raider's gunboat at `at` fights: the pilot once within `fire_range_m`, else the nearest of its job's
/// boats still afloat (the escort's freighter, the fishing fleet), else the pilot.
pub fn raider_target(at: Vec2, pilot: Vec2, marks: &[Prey], fire_range_m: f32) -> Option<u32> {
    if at.distance(pilot) <= fire_range_m {
        return None;
    }
    marks
        .iter()
        .filter(|m| m.crewed)
        .min_by(|a, b| a.pos.distance(at).total_cmp(&b.pos.distance(at)).then(a.id.cmp(&b.id)))
        .map(|m| m.id)
}

#[cfg(test)]
mod tests {
    use super::super::test_world::shipped;
    use super::super::{Errand, FISHING};
    use super::*;

    fn boat() -> Boat {
        let cfg = shipped();
        Boat::new(7, FISHING, &cfg.kinds[FISHING], Errand::Anchor { at: Vec2::ZERO }, Vec2::ZERO, 0.0)
    }

    #[test]
    fn only_the_pilots_harm_to_a_crewed_boat_is_reported() {
        let mut b = boat();
        assert_eq!(report(&mut b, Cause::Other, true, true), None, "a crab's sinking costs nothing");
        assert_eq!(report(&mut b, Cause::PilotFire, false, false), None, "an empty boat costs nothing");
        assert_eq!(report(&mut b, Cause::PilotFire, true, false), Some(Harm { sunk: false }));
    }

    #[test]
    fn harm_is_reported_once_and_sinking_once_more() {
        let mut b = boat();
        assert!(report(&mut b, Cause::PilotFire, true, false).is_some());
        assert_eq!(
            report(&mut b, Cause::PilotFire, true, false),
            None,
            "the second round is the same harm"
        );
        assert_eq!(report(&mut b, Cause::PilotFire, true, true), Some(Harm { sunk: true }));
        assert_eq!(report(&mut b, Cause::PilotFire, true, true), None);
    }

    #[test]
    fn swamping_a_boat_at_one_blow_is_one_report() {
        let mut b = boat();
        assert_eq!(report(&mut b, Cause::PilotMech, true, true), Some(Harm { sunk: true }));
        assert_eq!(report(&mut b, Cause::PilotMech, true, true), None);
    }

    #[test]
    fn the_nearest_base_is_named() {
        let bases = [Vec2::new(0.0, 0.0), Vec2::new(1000.0, 0.0), Vec2::new(1000.0, 0.0)];
        assert_eq!(nearest(&bases, Vec2::new(900.0, 10.0)), Some(1), "ties to the lower index");
        assert_eq!(nearest(&[], Vec2::ZERO), None);
    }

    #[test]
    fn a_crab_goes_for_a_crewed_ship_near_it_only_while_the_pilot_is_far() {
        let ships = [
            Prey {
                id: 3,
                pos: Vec2::new(500.0, 0.0),
                crewed: true,
            },
            Prey {
                id: 1,
                pos: Vec2::new(100.0, 0.0),
                crewed: false,
            },
            Prey {
                id: 2,
                pos: Vec2::new(-500.0, 0.0),
                crewed: true,
            },
        ];
        let far = Vec2::new(5000.0, 0.0);
        assert_eq!(
            crab_prey(Vec2::ZERO, far, &ships, 700.0, 1500.0),
            Some(2),
            "nearest crewed, ties to the lower id"
        );
        assert_eq!(
            crab_prey(Vec2::ZERO, Vec2::new(1000.0, 0.0), &ships, 700.0, 1500.0),
            None,
            "the pilot is near"
        );
        assert_eq!(crab_prey(Vec2::ZERO, far, &ships, 300.0, 1500.0), None, "none in reach");
    }

    #[test]
    fn a_raider_fights_the_pilot_in_range_else_its_mark() {
        let marks = [Prey {
            id: 9,
            pos: Vec2::new(3000.0, 0.0),
            crewed: true,
        }];
        assert_eq!(raider_target(Vec2::ZERO, Vec2::new(10_000.0, 0.0), &marks, 1500.0), Some(9));
        assert_eq!(raider_target(Vec2::ZERO, Vec2::new(1000.0, 0.0), &marks, 1500.0), None);
        assert_eq!(raider_target(Vec2::ZERO, Vec2::new(10_000.0, 0.0), &[], 1500.0), None);
    }
}
