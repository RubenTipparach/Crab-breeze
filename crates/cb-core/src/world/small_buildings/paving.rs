//! The paving that joins every built lot to the roads (openspec change `lot-access`): the suburbs' streets and the
//! connectors that join them to a city or a highway, each house's drive, each business's car park with its stalls
//! and curb cuts. Plain shapes on the ground (x, z), each with a height over it; the game drapes them on the
//! terrain (`cb-app::world`), so they need no ground of their own. Built with the lots (`lots::place`), pure and
//! deterministic.

use crate::world::grid::CellIndex;
use glam::Vec2;

/// What a piece of paving is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// Streets, car parks and curb cuts.
    Asphalt,
    /// Drives.
    Concrete,
    /// The stalls' white lines.
    Paint,
}

/// A street: a strip `width_m` wide along a centre line, `lift_m` over the ground.
#[derive(Clone, Debug, PartialEq)]
pub struct Street {
    /// Its centre line (x, z), a point every few metres.
    pub path: Vec<Vec2>,
    /// Its paved width.
    pub width_m: f32,
    /// How far over the ground it is drawn.
    pub lift_m: f32,
}

/// A four-cornered piece of paving (a drive, a car park, a curb cut, a painted line), corners in order round it,
/// each with its own height over the ground.
#[derive(Clone, Debug, PartialEq)]
pub struct Slab {
    /// Its corners (x, z), in order round it.
    pub corners: [Vec2; 4],
    /// How far over the ground each corner is drawn.
    pub lift_m: [f32; 4],
    /// What it is made of.
    pub surface: Surface,
}

impl Slab {
    /// A strip `width_m` wide from `a` to `b`, `lift_m` over the ground at `a` and `lift_b_m` at `b`.
    pub fn strip(a: Vec2, b: Vec2, width_m: f32, lift_m: f32, lift_b_m: f32, surface: Surface) -> Self {
        Self::taper(a, b, width_m, width_m, lift_m, lift_b_m, surface)
    }

    /// A strip from `a` (`width_a_m` wide) to `b` (`width_b_m` wide).
    pub fn taper(a: Vec2, b: Vec2, width_a_m: f32, width_b_m: f32, lift_m: f32, lift_b_m: f32, surface: Surface) -> Self {
        let along = (b - a).normalize_or(Vec2::Y);
        let side = Vec2::new(-along.y, along.x);
        Self {
            corners: [
                a - side * width_a_m * 0.5,
                a + side * width_a_m * 0.5,
                b + side * width_b_m * 0.5,
                b - side * width_b_m * 0.5,
            ],
            lift_m: [lift_m, lift_m, lift_b_m, lift_b_m],
            surface,
        }
    }

    /// How far `p` is outside it (0 inside). Corners may wind either way.
    pub fn distance(&self, p: Vec2) -> f32 {
        let c = &self.corners;
        // Twice the signed area, taken from the first corner: from the world's origin, f32 products of coordinates
        // kilometres out cancel and lose the sign.
        let wind = (1..3).map(|i| (c[i] - c[0]).perp_dot(c[i + 1] - c[0])).sum::<f32>().signum();
        let inside = (0..4).all(|i| (c[(i + 1) % 4] - c[i]).perp_dot(p - c[i]) * wind >= 0.0);
        if inside {
            return 0.0;
        }
        (0..4).map(|i| segment_distance(c[i], c[(i + 1) % 4], p)).fold(f32::MAX, f32::min)
    }
}

/// The distance from `p` to the segment `a`-`b`.
pub fn segment_distance(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 1e-12 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(p)
}

/// The distance from `p` to a centre line.
pub fn path_distance(path: &[Vec2], p: Vec2) -> f32 {
    match path {
        [] => f32::MAX,
        [a] => a.distance(p),
        _ => path.windows(2).map(|w| segment_distance(w[0], w[1], p)).fold(f32::MAX, f32::min),
    }
}

/// Every piece of paving, and an index over them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Paving {
    /// The streets: each suburb row's, then the connectors.
    pub streets: Vec<Street>,
    /// Drives, car parks, curb cuts and stall lines.
    pub slabs: Vec<Slab>,
    /// Street ids, then slab ids offset by the street count.
    index: CellIndex,
}

/// Cell size of the paving's index.
const PAVING_CELL_M: f32 = 64.0;

impl Paving {
    /// The paving, indexed over a world `world_size_m` across.
    pub fn new(streets: Vec<Street>, slabs: Vec<Slab>, world_size_m: f32) -> Self {
        let mut boxes = Vec::with_capacity(streets.len() + slabs.len());
        for (i, s) in streets.iter().enumerate() {
            let lo = s.path.iter().fold(Vec2::splat(f32::MAX), |m, p| m.min(*p)) - Vec2::splat(s.width_m * 0.5);
            let hi = s.path.iter().fold(Vec2::splat(f32::MIN), |m, p| m.max(*p)) + Vec2::splat(s.width_m * 0.5);
            boxes.push((i as u32, lo, hi));
        }
        for (i, s) in slabs.iter().enumerate() {
            let lo = s.corners.iter().fold(Vec2::splat(f32::MAX), |m, p| m.min(*p));
            let hi = s.corners.iter().fold(Vec2::splat(f32::MIN), |m, p| m.max(*p));
            boxes.push(((streets.len() + i) as u32, lo, hi));
        }
        let side = (world_size_m / PAVING_CELL_M).ceil() as usize;
        let index = CellIndex::build(-world_size_m * 0.5, PAVING_CELL_M, side, &boxes);
        Self { streets, slabs, index }
    }

    /// True when paving comes within `r` of `p`.
    pub fn paved(&self, p: Vec2, r: f32) -> bool {
        if (self.streets.is_empty() && self.slabs.is_empty()) || !p.is_finite() {
            return false;
        }
        let n = self.streets.len();
        self.index.within(p - Vec2::splat(r), p + Vec2::splat(r)).into_iter().any(|id| {
            let id = id as usize;
            if id < n {
                let s = &self.streets[id];
                path_distance(&s.path, p) <= s.width_m * 0.5 + r
            } else {
                self.slabs[id - n].distance(p) <= r
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_a_street_or_a_slab_is_paved_and_one_beside_them_is_not() {
        let street = Street {
            path: vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)],
            width_m: 8.0,
            lift_m: 0.13,
        };
        let drive = Slab::strip(Vec2::new(10.0, 4.0), Vec2::new(10.0, 10.0), 3.0, 0.06, 0.12, Surface::Concrete);
        let paving = Paving::new(vec![street], vec![drive], 1000.0);
        assert!(paving.paved(Vec2::new(20.0, 3.9), 0.0), "on the street");
        assert!(paving.paved(Vec2::new(10.5, 8.0), 0.0), "on the drive");
        assert!(!paving.paved(Vec2::new(20.0, 6.0), 0.0), "beside the street");
        assert!(paving.paved(Vec2::new(20.0, 6.0), 2.5), "within 2.5 m of it");
        assert!(!paving.paved(Vec2::new(13.0, 8.0), 0.0), "beside the drive");
    }

    #[test]
    fn a_slab_kilometres_out_still_knows_its_inside() {
        let s = Slab::strip(
            Vec2::new(-7075.7, 9664.14),
            Vec2::new(-7075.5, 9662.1),
            3.0,
            0.06,
            0.12,
            Surface::Concrete,
        );
        assert_eq!(s.distance(Vec2::new(-7075.69, 9664.04)), 0.0);
    }

    #[test]
    fn a_slab_is_inside_whichever_way_its_corners_wind() {
        let a = Slab::strip(Vec2::ZERO, Vec2::new(0.0, 10.0), 2.0, 0.0, 0.0, Surface::Asphalt);
        let mut b = a.clone();
        b.corners.reverse();
        for s in [&a, &b] {
            assert_eq!(s.distance(Vec2::new(0.5, 5.0)), 0.0);
            assert!((s.distance(Vec2::new(3.0, 5.0)) - 2.0).abs() < 1e-5);
        }
    }
}
