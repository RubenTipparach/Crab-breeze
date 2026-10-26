//! The toll (openspec change `civilians`; owner, survey W34): people killed by the pilot's own fire or
//! under their own mech's feet cost standing with the air base nearest where it happened. Standing per
//! base belongs to the career (its own change, not on main yet), so this is the hook it reads: every
//! death the pilot caused, counted by the base it counts against. Nothing here decides how much
//! standing a death costs.

use crate::world::AirBase;
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;

/// What killed someone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cause {
    /// A round passing through them.
    Shot,
    /// A blast.
    Blast,
    /// A mech's foot coming down on them.
    Crushed,
}

/// Someone killed this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Casualty {
    /// The person's id.
    pub person: u32,
    /// Where they fell (their feet).
    pub at: Vec3,
    /// What killed them.
    pub cause: Cause,
    /// The mech (or vehicle) whose fire or foot it was, when known.
    pub by: Option<u32>,
}

/// A death the pilot caused, as the career's standing reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StandingLoss {
    /// The base nearest where it happened (`None` on an island with no bases).
    pub base: Option<u32>,
    /// Where.
    pub at: Vec3,
    /// What killed them.
    pub cause: Cause,
}

/// The pilot's toll so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toll {
    /// Deaths the pilot caused, by the base each counts against.
    pub by_base: BTreeMap<u32, u32>,
    /// Deaths the pilot caused on an island with no bases.
    pub unplaced: u32,
    /// Deaths anyone else caused (the enemy's fire), counted but costing the pilot nothing.
    pub by_others: u32,
}

/// The base whose centre is nearest `at` (ties to the lower id).
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

impl Toll {
    /// Counts a death. When `pilot_mech` caused it, returns the standing loss for the career.
    pub fn record(&mut self, c: &Casualty, pilot_mech: u32, bases: &[AirBase]) -> Option<StandingLoss> {
        if c.by != Some(pilot_mech) {
            self.by_others += 1;
            return None;
        }
        let base = nearest_base(bases, Vec2::new(c.at.x, c.at.z));
        match base {
            Some(b) => *self.by_base.entry(b).or_default() += 1,
            None => self.unplaced += 1,
        }
        Some(StandingLoss {
            base,
            at: c.at,
            cause: c.cause,
        })
    }

    /// Every death the pilot caused.
    pub fn by_pilot(&self) -> u32 {
        self.by_base.values().sum::<u32>() + self.unplaced
    }
}
