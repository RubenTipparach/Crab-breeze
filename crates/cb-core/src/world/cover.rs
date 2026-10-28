//! What may grow at a point of the island (openspec changes `vegetation` and `gpu-grass`): open
//! ground, a city park, or bare ground (a road, a bridge, a building, a city's streets). Trees and
//! grass both ask here, each with its own clearances, so the rule of where plants stand is written
//! once (CLAUDE.md 6.1). What a foot presses into (soil or paving, openspec change `ground-marks`)
//! is the same rule with no clearance: where nothing may grow is paved. Water is not decided here: each asks the one water query its own way (a
//! tree wants a dry bank either side of its trunk; a blade wants only dry ground under it).

use super::WorldPlan;
use glam::Vec2;

/// What may grow at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cover {
    /// Open country outside the cities.
    Open,
    /// A park block inside a city.
    Park,
    /// Nothing grows: a road, a bridge, a building, a city's paved ground, or the strip round a
    /// city.
    Bare,
}

/// How far plants keep from what is built, metres (or a share, where named).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clearance {
    /// Outside a city, nothing grows within this distance of its square.
    pub city_m: f32,
    /// A park grows only this far inside its block's edge.
    pub park_inset_m: f32,
    /// Nothing grows within this distance of a highway's paved edge.
    pub road_m: f32,
    /// Nothing grows within this share of a bridge's length of its centre.
    pub bridge_share: f32,
    /// Nothing grows within this distance of a building's footprint.
    pub building_m: f32,
}

/// What the ground is made of at a point, for the marks a foot or a wheel leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// Soil or grass: open country and city parks.
    Dirt,
    /// Paving: highways and their bridges, city streets and plazas, building lots.
    Concrete,
}

impl Clearance {
    /// No clearance at all: the bare ground exactly as it is built.
    pub const NONE: Clearance = Clearance {
        city_m: 0.0,
        park_inset_m: 0.0,
        road_m: 0.0,
        bridge_share: 0.0,
        building_m: 0.0,
    };
}

impl WorldPlan {
    /// What the ground at `p` (x, z) is made of: paved exactly where `cover_at` with no clearance
    /// says nothing grows (a highway's paved width, a city outside its parks, a building's lot),
    /// soil elsewhere. Water is not decided here; a caller asks the one water query first.
    pub fn surface_at(&self, p: Vec2) -> Surface {
        match self.cover_at(p, &Clearance::NONE) {
            Cover::Bare => Surface::Concrete,
            Cover::Open | Cover::Park => Surface::Dirt,
        }
    }

    /// What may grow at `p` (x, z), keeping `clear` from what is built.
    pub fn cover_at(&self, p: Vec2, clear: &Clearance) -> Cover {
        let cover = match self.cities.iter().find(|c| c.rect().distance(p) < 1e-3) {
            Some(c) => {
                let park = c
                    .blocks
                    .iter()
                    .any(|b| b.park && b.rect.inset(clear.park_inset_m).is_some_and(|i| i.distance(p) < 1e-3));
                if park { Cover::Park } else { Cover::Bare }
            }
            None if self.cities.iter().any(|c| c.rect().distance(p) < clear.city_m) => Cover::Bare,
            // An air base's site is built ground, kept as clear as a city's edge (air-bases).
            None if self
                .bases
                .iter()
                .any(|b| b.rect().distance(p) <= clear.city_m || b.runway.strip().distance(p) <= clear.city_m) =>
            {
                Cover::Bare
            }
            None => Cover::Open,
        };
        if cover == Cover::Bare {
            return cover;
        }
        let road = self.highways.iter().any(|h| h.project(p).0 < h.width_m * 0.5 + clear.road_m)
            || self
                .waters
                .bridges
                .iter()
                .any(|b| b.centre.distance(p) < b.length_m * clear.bridge_share);
        // A suburb street, a drive or a car park is paving like a highway (openspec change `lot-access`).
        let paved = self.small.paving.paved(p, clear.road_m);
        if road || paved || self.collision.overlaps(p, clear.building_m) || self.small.overlaps(p, clear.building_m) {
            Cover::Bare
        } else {
            cover
        }
    }
}
