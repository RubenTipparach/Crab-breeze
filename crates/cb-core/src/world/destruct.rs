//! Destructible buildings (openspec change `building-destruction`): every building is a grid of
//! cells about 6 m square and one storey tall. Blasts and impacts break cells; a cell with no cell
//! under it falls, and a storey with too little left standing brings everything above it down.
//! Fallen cells come back as chunks for the physics debris. Pure and deterministic.

use super::WorldPlan;
use super::layout::Rect;
use glam::{Vec2, Vec3};

/// Target cell width.
const CELL_M: f32 = 6.0;
/// A storey with less than this share of its cells standing gives way.
const STOREY_HOLDS: f32 = 0.35;

/// One building's cells.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    /// Cells along x, y (storeys) and z.
    pub n: [usize; 3],
    /// Cell size.
    pub size: Vec3,
    /// World position of the cell grid's least corner.
    pub origin: Vec3,
    /// Intact flags, index `(k * n[2] + j) * n[0] + i` for cell (i, k, j) = (x, storey, z).
    pub intact: Vec<bool>,
    /// Cells left standing.
    pub standing: usize,
}

impl Cells {
    fn index(&self, i: usize, k: usize, j: usize) -> usize {
        (k * self.n[2] + j) * self.n[0] + i
    }

    /// True when cell (i, storey k, j) stands.
    pub fn at(&self, i: usize, k: usize, j: usize) -> bool {
        self.intact[self.index(i, k, j)]
    }

    /// True when no cell has been lost.
    pub fn whole(&self) -> bool {
        self.standing == self.intact.len()
    }

    /// Centre of a cell.
    pub fn centre(&self, i: usize, k: usize, j: usize) -> Vec3 {
        self.origin + self.size * Vec3::new(i as f32 + 0.5, k as f32 + 0.5, j as f32 + 0.5)
    }

    /// The cell holding `p`, if inside the grid.
    pub fn cell_of(&self, p: Vec3) -> Option<(usize, usize, usize)> {
        let q = (p - self.origin) / self.size;
        if q.min_element() < 0.0 || q.x >= self.n[0] as f32 || q.y >= self.n[1] as f32 || q.z >= self.n[2] as f32 {
            return None;
        }
        Some((q.x as usize, q.y as usize, q.z as usize))
    }

    /// True when the point is inside a standing cell.
    pub fn solid_at(&self, p: Vec3) -> bool {
        self.cell_of(p).is_some_and(|(i, k, j)| self.at(i, k, j))
    }
}

/// A piece that fell: a block of cells, to become debris.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chunk {
    /// The building it came from.
    pub building: u32,
    /// Centre.
    pub centre: Vec3,
    /// Size.
    pub size: Vec3,
}

/// Every building's cells.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Structures {
    /// Cells by building id.
    pub buildings: Vec<Cells>,
}

impl Structures {
    /// Every building of a plan, whole.
    pub fn new(plan: &WorldPlan) -> Self {
        let buildings = plan
            .buildings
            .iter()
            .map(|b| {
                let size2 = b.footprint.size();
                let n = [
                    ((size2.x / CELL_M).round() as usize).max(1),
                    b.storeys as usize,
                    ((size2.y / CELL_M).round() as usize).max(1),
                ];
                let ground = plan.cities[b.city as usize].ground_m;
                let count = n[0] * n[1] * n[2];
                Cells {
                    n,
                    size: Vec3::new(size2.x / n[0] as f32, b.height_m / n[1] as f32, size2.y / n[2] as f32),
                    origin: Vec3::new(b.footprint.min.x, ground, b.footprint.min.y),
                    intact: vec![true; count],
                    standing: count,
                }
            })
            .collect();
        Self { buildings }
    }

    /// Breaks every standing cell of building `b` whose centre is within `radius_m` of `centre`,
    /// then lets what lost its support fall. Returns the chunks that fell (broken cells vanish as
    /// dust; unsupported ones fall as chunks).
    pub fn break_sphere(&mut self, b: u32, centre: Vec3, radius_m: f32) -> (usize, Vec<Chunk>) {
        let Some(c) = self.buildings.get_mut(b as usize) else {
            return (0, Vec::new());
        };
        let mut broke = 0;
        for k in 0..c.n[1] {
            for j in 0..c.n[2] {
                for i in 0..c.n[0] {
                    let idx = c.index(i, k, j);
                    if c.intact[idx] && c.centre(i, k, j).distance(centre) <= radius_m {
                        c.intact[idx] = false;
                        c.standing -= 1;
                        broke += 1;
                    }
                }
            }
        }
        let chunks = if broke > 0 { self.settle(b) } else { Vec::new() };
        (broke, chunks)
    }

    /// Lets unsupported cells fall: a cell needs the cell below it, and a storey with under 35 % of
    /// its cells standing gives way under everything above it.
    pub fn settle(&mut self, b: u32) -> Vec<Chunk> {
        let c = &mut self.buildings[b as usize];
        let per_storey = c.n[0] * c.n[2];
        let mut falling = vec![false; c.intact.len()];
        let mut give_way_at = None;
        for k in 0..c.n[1] {
            let standing = (0..c.n[2])
                .flat_map(|j| (0..c.n[0]).map(move |i| (i, j)))
                .filter(|&(i, j)| c.at(i, k, j))
                .count();
            if (standing as f32) < STOREY_HOLDS * per_storey as f32 && standing < per_storey {
                give_way_at = Some(k);
                break;
            }
        }
        for k in 0..c.n[1] {
            for j in 0..c.n[2] {
                for i in 0..c.n[0] {
                    let idx = c.index(i, k, j);
                    if !c.intact[idx] {
                        continue;
                    }
                    let below_gone = k > 0 && (!c.intact[c.index(i, k - 1, j)] || falling[c.index(i, k - 1, j)]);
                    if below_gone || give_way_at.is_some_and(|g| k > g) {
                        falling[idx] = true;
                    }
                }
            }
        }
        // Group falling cells into chunks of up to 2 x 2 cells per storey.
        let mut chunks = Vec::new();
        for k in 0..c.n[1] {
            for j in (0..c.n[2]).step_by(2) {
                for i in (0..c.n[0]).step_by(2) {
                    let mut lo = Vec3::splat(f32::MAX);
                    let mut hi = Vec3::splat(f32::MIN);
                    for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (ii, jj) = (i + di, j + dj);
                        if ii >= c.n[0] || jj >= c.n[2] {
                            continue;
                        }
                        let idx = c.index(ii, k, jj);
                        if falling[idx] {
                            c.intact[idx] = false;
                            c.standing -= 1;
                            let m = c.origin + c.size * Vec3::new(ii as f32, k as f32, jj as f32);
                            lo = lo.min(m);
                            hi = hi.max(m + c.size);
                        }
                    }
                    if lo.x <= hi.x {
                        chunks.push(Chunk {
                            building: b,
                            centre: (lo + hi) * 0.5,
                            size: hi - lo,
                        });
                    }
                }
            }
        }
        chunks
    }

    /// The standing cells' footprints of building `b` whose storeys overlap `[y0, y1]`, as rects.
    pub fn solid_rects(&self, b: u32, y0: f32, y1: f32) -> Vec<Rect> {
        let c = &self.buildings[b as usize];
        let k0 = (((y0 - c.origin.y) / c.size.y).floor().max(0.0) as usize).min(c.n[1]);
        let k1 = (((y1 - c.origin.y) / c.size.y).ceil().max(0.0) as usize).min(c.n[1]);
        let mut out = Vec::new();
        for j in 0..c.n[2] {
            for i in 0..c.n[0] {
                if (k0..k1).any(|k| c.at(i, k, j)) {
                    let m = c.origin + c.size * Vec3::new(i as f32, 0.0, j as f32);
                    out.push(Rect {
                        min: Vec2::new(m.x, m.z),
                        max: Vec2::new(m.x + c.size.x, m.z + c.size.z),
                    });
                }
            }
        }
        out
    }

    /// Height of the highest standing cell top under `p` at or below `feet_y + 1 m`, if any.
    pub fn roof_under(&self, b: u32, p: Vec2, feet_y: f32) -> Option<f32> {
        let c = &self.buildings[b as usize];
        let (i, _, j) = c.cell_of(Vec3::new(p.x, c.origin.y, p.y))?;
        (0..c.n[1])
            .rev()
            .filter(|&k| c.at(i, k, j))
            .map(|k| c.origin.y + c.size.y * (k as f32 + 1.0))
            .find(|top| *top <= feet_y + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;
    use crate::world::{WorldConfig, generate};
    use std::sync::OnceLock;

    fn plan() -> &'static WorldPlan {
        static P: OnceLock<WorldPlan> = OnceLock::new();
        P.get_or_init(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
            generate(42, &load_ron::<WorldConfig>(&p).unwrap())
        })
    }

    fn tallest() -> u32 {
        plan().buildings.iter().max_by_key(|b| b.storeys).unwrap().id
    }

    #[test]
    fn a_new_building_is_whole() {
        let s = Structures::new(plan());
        assert!(s.buildings.iter().all(Cells::whole));
        assert_eq!(s.buildings.len(), plan().buildings.len());
    }

    #[test]
    fn a_shell_holes_a_wall_without_bringing_the_building_down() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        let at = c.centre(0, c.n[1] / 2, 0);
        let (broke, fell) = s.break_sphere(b, at, 4.0);
        assert!(broke >= 1);
        let cells = &s.buildings[b as usize];
        assert!(cells.standing > c.standing / 2, "most of the tower stands");
        assert!(!fell.is_empty(), "the cells above the hole fall");
        assert!(!cells.solid_at(at));
    }

    #[test]
    fn taking_out_the_ground_floor_brings_down_the_storeys_above() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        let mut chunks = Vec::new();
        for j in 0..c.n[2] {
            for i in 0..c.n[0] {
                chunks.extend(s.break_sphere(b, c.centre(i, 0, j), 0.1).1);
            }
        }
        assert_eq!(s.buildings[b as usize].standing, 0, "nothing stands");
        assert!(chunks.len() >= c.n[1] - 1, "every storey above fell as chunks");
    }

    #[test]
    fn a_mech_walks_through_a_hole_it_blew() {
        let mut s = Structures::new(plan());
        let b = plan().buildings.iter().find(|b| b.storeys <= 4).unwrap().id;
        let c = s.buildings[b as usize].clone();
        let full = s.solid_rects(b, c.origin.y, c.origin.y + 10.0).len();
        for k in 0..c.n[1] {
            s.break_sphere(b, c.centre(0, k, 0), 0.1);
        }
        assert!(s.solid_rects(b, c.origin.y, c.origin.y + 10.0).len() < full);
    }
}
