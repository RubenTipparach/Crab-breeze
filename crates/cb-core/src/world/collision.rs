//! The one building collision query, shared by the pilot and the mech (procgen-world design
//! section 3). Bodies are circles on the ground; buildings are their footprints.

use super::grid::CellIndex;
use super::layout::{Building, Rect};
use glam::Vec2;

/// Buildings bucketed on a square grid for fast lookup. Candidates are visited in id order, so
/// the result never depends on bucket order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingGrid {
    index: CellIndex,
    footprints: Vec<Rect>,
    /// Roof height of each building; empty when unknown (then every building is a wall).
    roofs: Vec<f32>,
}

impl BuildingGrid {
    /// A grid over a square world `world_size_m` wide, with `bucket_m` buckets.
    pub fn new(buildings: &[Building], world_size_m: f32, bucket_m: f32) -> Self {
        let side = (world_size_m / bucket_m).ceil() as usize + 1;
        let boxes: Vec<(u32, Vec2, Vec2)> = buildings
            .iter()
            .enumerate()
            .map(|(i, b)| (i as u32, b.footprint.min, b.footprint.max))
            .collect();
        Self {
            index: CellIndex::build(-world_size_m * 0.5, bucket_m, side, &boxes),
            footprints: buildings.iter().map(|b| b.footprint).collect(),
            roofs: Vec::new(),
        }
    }

    /// The same grid knowing each building's roof height, so bodies can fly over and stand on them.
    pub fn with_roofs(mut self, roofs: Vec<f32>) -> Self {
        self.roofs = roofs;
        self
    }

    /// Like `push_out`, but only buildings whose roof is above `feet_y` are walls.
    pub fn push_out_at(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        if !p.is_finite() {
            return p;
        }
        let mut p = p;
        for _ in 0..2 {
            for i in self.candidates(p, radius_m) {
                if self.roofs.get(i as usize).is_some_and(|top| *top <= feet_y + 0.5) {
                    continue;
                }
                p = push_out_of_rect(p, radius_m, &self.footprints[i as usize]);
            }
        }
        p
    }

    /// The highest roof under `p` that is no higher than `feet_y + 1 m`, if any.
    pub fn roof_under(&self, p: Vec2, feet_y: f32) -> Option<f32> {
        self.candidates(p, 0.0)
            .iter()
            .filter(|&&i| self.footprints[i as usize].distance(p) == 0.0)
            .filter_map(|&i| self.roofs.get(i as usize).copied())
            .filter(|top| *top <= feet_y + 1.0)
            .reduce(f32::max)
    }

    /// Indices of buildings whose footprint might be within `radius_m` of `p`, ascending.
    pub fn candidates(&self, p: Vec2, radius_m: f32) -> Vec<u32> {
        self.index.within(p - Vec2::splat(radius_m), p + Vec2::splat(radius_m))
    }

    /// Moves a circle of `radius_m` at `p` (x, z) out of every building footprint it overlaps,
    /// by the shortest way. A non-finite position is returned unchanged.
    pub fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        if !p.is_finite() {
            return p;
        }
        let mut p = p;
        // Two passes settle a body wedged in a corner between two buildings.
        for _ in 0..2 {
            for i in self.candidates(p, radius_m) {
                p = push_out_of_rect(p, radius_m, &self.footprints[i as usize]);
            }
        }
        p
    }

    /// True when a circle at `p` overlaps any footprint.
    pub fn overlaps(&self, p: Vec2, radius_m: f32) -> bool {
        self.candidates(p, radius_m)
            .iter()
            .any(|&i| self.footprints[i as usize].distance(p) < radius_m - 1e-3)
    }
}

/// Moves a circle of radius `r` at `p` out of `rect` by the shortest way.
pub fn push_out_of_rect(p: Vec2, r: f32, rect: &Rect) -> Vec2 {
    let closest = p.clamp(rect.min, rect.max);
    let d = p - closest;
    if d != Vec2::ZERO {
        let len = d.length();
        return if len < r { closest + d / len * r } else { p };
    }
    // Centre inside the footprint: leave through the nearest wall.
    let exits = [
        (p.x - rect.min.x, Vec2::new(rect.min.x - r, p.y)),
        (rect.max.x - p.x, Vec2::new(rect.max.x + r, p.y)),
        (p.y - rect.min.y, Vec2::new(p.x, rect.min.y - r)),
        (rect.max.y - p.y, Vec2::new(p.x, rect.max.y + r)),
    ];
    exits.iter().min_by(|a, b| a.0.total_cmp(&b.0)).map(|e| e.1).unwrap_or(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_building() -> BuildingGrid {
        let b = Building {
            id: 0,
            city: 0,
            footprint: Rect {
                min: Vec2::new(0.0, 0.0),
                max: Vec2::new(20.0, 10.0),
            },
            storeys: 4,
            height_m: 14.0,
        };
        BuildingGrid::new(&[b], 512.0, 64.0)
    }

    #[test]
    fn a_body_inside_a_building_is_pushed_to_its_wall() {
        let g = one_building();
        let p = g.push_out(Vec2::new(10.0, 1.0), 0.35);
        assert!((p - Vec2::new(10.0, -0.35)).length() < 1e-4, "{p}");
        assert!(!g.overlaps(p, 0.35));
    }

    #[test]
    fn a_body_touching_a_wall_is_pushed_back_by_its_radius() {
        let g = one_building();
        let p = g.push_out(Vec2::new(-1.0, 5.0), 3.0);
        assert!((p.x + 3.0).abs() < 1e-4, "{p}");
    }

    #[test]
    fn a_body_in_the_open_does_not_move() {
        let g = one_building();
        let p = Vec2::new(-50.0, -50.0);
        assert_eq!(g.push_out(p, 3.0), p);
    }
}
