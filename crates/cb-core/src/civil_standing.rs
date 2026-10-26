//! The one place harm to civilians costs the pilot standing (survey W34, W37, W42). Four features report
//! harm, each its own way: people killed (`civilians`, a `Casualty`), cars harmed (`traffic`, a `Harm`),
//! civil aircraft hit (`air_traffic`) and boats harmed (`ships::harm`, harbours HB16). Each is turned into a `CivilHarm` here, and `Charges::charge` applies
//! the rules once and lowers the career's standing with the base nearest where it happened:
//!
//! - only the pilot's own rounds, blasts, feet and car count; the enemy's kills cost nothing (W34, W42);
//! - a car counts only with somebody in it (W37): a parked car is property, not a civilian; a boat only with a civil
//!   crew aboard (harbours HB16);
//! - each victim costs once: a person dies once, a car or an aircraft hit by a whole burst is charged for
//!   its first pilot-caused harm only.
//!
//! How much a victim costs is `CareerConfig::standing_per_civilian` (`data/career/career.ron`).

use crate::career::{Career, CareerConfig};
use crate::world::AirBase;
use glam::Vec2;
use std::collections::BTreeSet;

/// Who was harmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Victim {
    /// A person on foot, by id.
    Person(u32),
    /// A car, by id; `occupied` when somebody other than the pilot was in it.
    Car {
        /// The car's id.
        id: u32,
        /// Somebody other than the pilot was in it.
        occupied: bool,
    },
    /// A civil aircraft, by id. It always carries a crew.
    Aircraft(u32),
    /// A boat, by id; `crewed` when it had a civil crew aboard when it was harmed (harbours HB16).
    Boat {
        /// The boat's id.
        id: u32,
        /// It carried a civil crew.
        crewed: bool,
    },
}

/// One civilian harmed, as each feature reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CivilHarm {
    /// Who.
    pub victim: Victim,
    /// Where (x, z), metres.
    pub at: Vec2,
    /// The pilot's own rounds, blasts, feet or car did it.
    pub by_pilot: bool,
}

impl CivilHarm {
    /// True when this harm costs standing at all (W34, W37, W42), before the once-per-victim rule.
    pub fn counts(&self) -> bool {
        self.by_pilot
            && match self.victim {
                Victim::Car { occupied, .. } => occupied,
                Victim::Boat { crewed, .. } => crewed,
                Victim::Person(_) | Victim::Aircraft(_) => true,
            }
    }

    /// The victim's key for the once-per-victim rule (a car's key ignores whether it is occupied).
    fn key(&self) -> (u8, u32) {
        match self.victim {
            Victim::Person(id) => (0, id),
            Victim::Car { id, .. } => (1, id),
            Victim::Aircraft(id) => (2, id),
            Victim::Boat { id, .. } => (3, id),
        }
    }
}

/// The base whose centre is nearest `at` (x, z), ties to the lower id: whose standing a harm there costs.
pub fn nearest_base(bases: &[AirBase], at: Vec2) -> Option<u32> {
    bases
        .iter()
        .min_by(|a, b| {
            a.centre
                .distance_squared(at)
                .total_cmp(&b.centre.distance_squared(at))
                .then(a.id.cmp(&b.id))
        })
        .map(|b| b.id)
}

/// The victims already charged this run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Charges {
    charged: BTreeSet<(u8, u32)>,
}

impl Charges {
    /// Charges `harm` to `career`: when it counts and its victim has not been charged before, lowers the
    /// standing with the nearest base by `cfg.standing_per_civilian` (never below zero). Returns the base and
    /// the standing after, or `None` when nothing was charged (not the pilot's, an empty car, a victim
    /// already charged, or an island with no bases).
    pub fn charge(&mut self, career: &mut Career, cfg: &CareerConfig, bases: &[AirBase], harm: &CivilHarm) -> Option<(u32, i64)> {
        if !harm.counts() {
            return None;
        }
        let base = nearest_base(bases, harm.at)?;
        if !self.charged.insert(harm.key()) {
            return None;
        }
        Some((base, career.civilian_harmed(base, cfg)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bases() -> Vec<AirBase> {
        [Vec2::new(0.0, 0.0), Vec2::new(1000.0, 0.0)]
            .into_iter()
            .enumerate()
            .map(|(i, centre)| AirBase {
                id: i as u32,
                centre,
                half_m: Vec2::splat(100.0),
                ground_m: 0.0,
                highway: 0,
                runway: crate::world::bases::Runway::beside(
                    centre,
                    Vec2::splat(100.0),
                    crate::world::bases::RunwaySide::Front,
                    1800.0,
                    0.0,
                    &crate::world::BasesConfig::default(),
                ),
            })
            .collect()
    }

    fn career_with(standing: i64) -> (Career, CareerConfig) {
        let cfg = CareerConfig::default();
        let mut c = Career::new("t", 1, &cfg, &[1.0]);
        c.standing.insert(0, standing);
        c.standing.insert(1, standing);
        (c, cfg)
    }

    fn harm(victim: Victim, x: f32, by_pilot: bool) -> CivilHarm {
        CivilHarm {
            victim,
            at: Vec2::new(x, 0.0),
            by_pilot,
        }
    }

    #[test]
    fn a_crewed_boat_the_pilot_harms_costs_once_even_when_it_then_sinks() {
        // Harbours HB16: only crewed boats, only the pilot's own harm, once per boat.
        let (mut c, cfg) = career_with(5);
        let mut ch = Charges::default();
        let boat = |id, crewed| Victim::Boat { id, crewed };
        assert_eq!(
            ch.charge(&mut c, &cfg, &bases(), &harm(boat(4, false), 100.0, true)),
            None,
            "no crew aboard"
        );
        assert_eq!(
            ch.charge(&mut c, &cfg, &bases(), &harm(boat(5, true), 100.0, false)),
            None,
            "not the pilot's"
        );
        let first = ch.charge(&mut c, &cfg, &bases(), &harm(boat(6, true), 100.0, true));
        assert_eq!(first, Some((0, 5 - cfg.standing_per_civilian)));
        assert_eq!(
            ch.charge(&mut c, &cfg, &bases(), &harm(boat(6, true), 120.0, true)),
            None,
            "its sinking, reported again, is not charged twice"
        );
        assert_eq!(
            ch.charge(&mut c, &cfg, &bases(), &harm(Victim::Aircraft(6), 100.0, true)),
            Some((0, 5 - 2 * cfg.standing_per_civilian)),
            "an aircraft with the same id is another victim"
        );
    }

    #[test]
    fn a_person_the_pilot_kills_costs_standing_with_the_nearest_base() {
        let (mut c, cfg) = career_with(5);
        let mut ch = Charges::default();
        let got = ch.charge(&mut c, &cfg, &bases(), &harm(Victim::Person(7), 900.0, true));
        assert_eq!(got, Some((1, 5 - cfg.standing_per_civilian)));
        assert_eq!(c.standing_at(0), 5, "the far base keeps its standing");
    }

    #[test]
    fn a_person_the_enemy_kills_costs_nothing() {
        let (mut c, cfg) = career_with(5);
        let got = Charges::default().charge(&mut c, &cfg, &bases(), &harm(Victim::Person(7), 0.0, false));
        assert_eq!(got, None);
        assert_eq!(c.standing_at(0), 5);
    }

    #[test]
    fn a_car_costs_standing_only_with_a_driver_in_it() {
        let (mut c, cfg) = career_with(5);
        let mut ch = Charges::default();
        let parked = harm(Victim::Car { id: 3, occupied: false }, 0.0, true);
        assert_eq!(ch.charge(&mut c, &cfg, &bases(), &parked), None, "a parked car is not a civilian");
        let driven = harm(Victim::Car { id: 3, occupied: true }, 0.0, true);
        assert_eq!(ch.charge(&mut c, &cfg, &bases(), &driven), Some((0, 5 - cfg.standing_per_civilian)));
        assert_eq!(ch.charge(&mut c, &cfg, &bases(), &driven), None, "the same car is charged once");
        let not_mine = harm(Victim::Car { id: 4, occupied: true }, 0.0, false);
        assert_eq!(ch.charge(&mut c, &cfg, &bases(), &not_mine), None);
    }

    #[test]
    fn a_civil_aircraft_costs_standing_once_however_many_rounds_hit_it() {
        let (mut c, cfg) = career_with(5);
        let mut ch = Charges::default();
        let hit = harm(Victim::Aircraft(9), 100.0, true);
        assert_eq!(ch.charge(&mut c, &cfg, &bases(), &hit), Some((0, 5 - cfg.standing_per_civilian)));
        for _ in 0..20 {
            assert_eq!(ch.charge(&mut c, &cfg, &bases(), &hit), None);
        }
        assert_eq!(c.standing_at(0), 5 - cfg.standing_per_civilian);
        let crab_fire = harm(Victim::Aircraft(10), 100.0, false);
        assert_eq!(
            ch.charge(&mut c, &cfg, &bases(), &crab_fire),
            None,
            "a crab's hit costs the pilot nothing"
        );
    }

    #[test]
    fn standing_never_falls_below_zero() {
        let (mut c, cfg) = career_with(0);
        let got = Charges::default().charge(&mut c, &cfg, &bases(), &harm(Victim::Person(1), 0.0, true));
        assert_eq!(got, Some((0, 0)));
    }

    #[test]
    fn an_island_with_no_bases_charges_nothing() {
        let (mut c, cfg) = career_with(5);
        let got = Charges::default().charge(&mut c, &cfg, &[], &harm(Victim::Person(1), 0.0, true));
        assert_eq!(got, None);
    }
}
