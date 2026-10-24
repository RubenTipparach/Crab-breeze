//! The one building collision query, shared by the pilot and the mech (procgen-world design
//! section 3). Bodies are circles on the ground; buildings are their footprints.

use super::layout::{Building, Rect};
use glam::Vec2;

/// Buildings bucketed on a square grid for fast lookup. Candidates are visited in id order, so
/// the result never depends on bucket order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingGrid {
    origin_m: f32,
    bucket_m: f32,
    side: usize,
    buckets: Vec<Vec<u32>>,
    footprints: Vec<Rect>,
}

impl BuildingGrid {
    /// A grid over a square world `world_size_m` wide, with `bucket_m` buckets.
    pub fn new(buildings: &[Building], world_size_m: f32, bucket_m: f32) -> Self {
        let side = (world_size_m / bucket_m).ceil() as usize + 1;
        let mut grid = Self {
            origin_m: -world_size_m * 0.5,
            bucket_m,
            side,
            buckets: vec![Vec::new(); side * side],
            footprints: Vec::with_capacity(buildings.len()),
        };
        for (i, b) in buildings.iter().enumerate() {
            grid.footprints.push(b.footprint);
            let (x0, z0) = grid.bucket(b.footprint.min);
            let (x1, z1) = grid.bucket(b.footprint.max);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    grid.buckets[z * side + x].push(i as u32);
                }
            }
        }
        grid
    }

    fn bucket(&self, p: Vec2) -> (usize, usize) {
        let f = |v: f32| (((v - self.origin_m) / self.bucket_m).floor().max(0.0) as usize).min(self.side - 1);
        (f(p.x), f(p.y))
    }

    /// Indices of buildings whose footprint might be within `radius_m` of `p`, ascending.
    fn candidates(&self, p: Vec2, radius_m: f32) -> Vec<u32> {
        let (x0, z0) = self.bucket(p - Vec2::splat(radius_m));
        let (x1, z1) = self.bucket(p + Vec2::splat(radius_m));
        let mut out = Vec::new();
        for z in z0..=z1 {
            for x in x0..=x1 {
                out.extend_from_slice(&self.buckets[z * self.side + x]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
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

fn push_out_of_rect(p: Vec2, r: f32, rect: &Rect) -> Vec2 {
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
