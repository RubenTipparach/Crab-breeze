//! Destructible buildings as the rest of the game sees them: a grid of cells about 6 m square and
//! one storey tall, each open or closed. What breaks and what falls is decided by the building's
//! structural frame (`structure.rs`, openspec change `structural-destruction`), built the first time
//! the building is hit; until then a building is whole and costs nothing. Fallen elements come back
//! as chunks for the physics debris. Pure and deterministic.

use super::WorldPlan;
use super::layout::Rect;
use super::structure::{Footprint, Frame, StructureConfig};
use glam::{Vec2, Vec3};

/// Target cell width.
const CELL_M: f32 = 6.0;

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
    /// The structural frame, from the first hit on.
    pub frame: Option<Box<Frame>>,
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

/// A part of a building on its way down: it lands at `at_s` on the floor below it or the ground.
#[derive(Clone, Debug, PartialEq)]
pub struct Fall {
    /// The building it fell from.
    pub building: u32,
    /// When it lands.
    pub at_s: f32,
    /// Where it is.
    pub part: Footprint,
}

/// Rubble that reached the ground: a heap to raise over `lo..hi` (x, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// Least (x, z) of the fallen part.
    pub lo: Vec2,
    /// Greatest (x, z).
    pub hi: Vec2,
    /// Heap volume (crushed, fines only).
    pub volume_m3: f32,
}

/// A moment worth seeing or hearing, for the effects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Moment {
    /// A member is overloaded and will fail soon (dust trickles, it groans).
    Creak(Vec3),
    /// A member gave way.
    Fail(Vec3),
    /// A falling part hit a floor or the ground.
    Impact {
        /// Where it hit.
        at: Vec3,
        /// Its mass.
        mass_kg: f32,
    },
}

/// Every building's cells.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Structures {
    /// Cells by building id.
    pub buildings: Vec<Cells>,
    /// Frame tuning.
    pub cfg: StructureConfig,
    /// Blast radius per cube root of energy (the crater rule), to turn a reach into an energy.
    pub radius_m_per_cbrt_mj: f32,
    /// Simulation time, for creaks and falls.
    pub time_s: f32,
    /// Parts on their way down, by landing time.
    pub falls: Vec<Fall>,
    /// Heaps waiting to be raised on the ground (taken by `take_landings`).
    pub landings: Vec<Landing>,
    /// Moments since the last `take_moments`.
    pub moments: Vec<Moment>,
}

impl Structures {
    /// Every building of a plan, whole, with the default frame tuning.
    pub fn new(plan: &WorldPlan) -> Self {
        Self::with_config(plan, StructureConfig::default(), 2.2)
    }

    /// Every building of a plan, whole, with the shipped tuning (`structure:` and the crater rule).
    pub fn with_config(plan: &WorldPlan, cfg: StructureConfig, radius_m_per_cbrt_mj: f32) -> Self {
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
                    frame: None,
                }
            })
            .collect();
        Self {
            buildings,
            cfg,
            radius_m_per_cbrt_mj,
            time_s: 0.0,
            falls: Vec::new(),
            landings: Vec::new(),
            moments: Vec::new(),
        }
    }

    /// The frame of building `b`, built on first use.
    fn frame_mut(&mut self, b: usize) -> &mut Frame {
        let cfg = &self.cfg;
        let c = &mut self.buildings[b];
        c.frame
            .get_or_insert_with(|| Box::new(Frame::new(b as u32, c.n, c.origin, c.size, cfg)))
    }

    /// A blast reaching `radius_m` from `centre` on building `b`, with the energy the crater rule
    /// gives that reach. Breaks the elements it overcomes, then lets the frame settle. Returns how
    /// many elements broke and the chunks that fell.
    pub fn break_sphere(&mut self, b: u32, centre: Vec3, radius_m: f32) -> (usize, Vec<Chunk>) {
        if b as usize >= self.buildings.len() || radius_m.is_nan() || radius_m <= 0.0 {
            return (0, Vec::new());
        }
        let energy_mj = (radius_m / self.radius_m_per_cbrt_mj).powi(3);
        let broke = self.frame_mut(b as usize).blast(centre, radius_m, energy_mj);
        let chunks = if broke > 0 { self.settle(b) } else { Vec::new() };
        (broke, chunks)
    }

    /// Lets building `b`'s frame find its balance now; returns the chunks that fell. What fell is
    /// dropped: it lands later on the floor below or the ground.
    pub fn settle(&mut self, b: u32) -> Vec<Chunk> {
        let (now, cfg) = (self.time_s, self.cfg.clone());
        let settled = self.frame_mut(b as usize).settle(now, &cfg);
        if let Some(f) = self.buildings[b as usize].frame.as_ref() {
            self.moments
                .extend(settled.creaking.iter().map(|&id| Moment::Creak(f.elements[id as usize].centre())));
        }
        let chunks = self.chunks_of(b, &settled.fell);
        if let Some(part) = self.buildings[b as usize].frame.as_ref().and_then(|f| f.footprint(&settled.fell)) {
            self.drop_part(b, part, now);
        }
        self.sync(b as usize);
        chunks
    }

    /// Schedules a falling part's landing: on the highest standing floor under it, or the ground.
    fn drop_part(&mut self, b: u32, part: Footprint, from_s: f32) {
        let c = &self.buildings[b as usize];
        let target = c
            .frame
            .as_ref()
            .and_then(|f| f.floor_below(&part.cells, part.bottom_m))
            .map_or(c.origin.y, |(top, _)| top);
        let drop = (part.bottom_m - target).max(0.0);
        let at_s = from_s + (2.0 * drop / 9.81).sqrt();
        let i = self.falls.partition_point(|f| f.at_s <= at_s);
        self.falls.insert(i, Fall { building: b, at_s, part });
    }

    /// A part lands. On a floor it loads the bays under it; if any give way it keeps falling
    /// through them. On the ground its fines become a heap.
    fn land(&mut self, fall: Fall) -> Vec<Chunk> {
        let b = fall.building;
        let cfg = self.cfg.clone();
        let mut part = fall.part;
        let below = self.buildings[b as usize]
            .frame
            .as_ref()
            .and_then(|f| f.floor_below(&part.cells, part.bottom_m));
        let ground = self.buildings[b as usize].origin.y;
        let centre = (part.lo + part.hi) * 0.5;
        self.moments.push(Moment::Impact {
            at: Vec3::new(centre.x, below.as_ref().map_or(ground, |(top, _)| *top), centre.y),
            mass_kg: part.mass_kg,
        });
        let Some((top, slabs)) = below else {
            self.landings.push(Landing {
                lo: part.lo,
                hi: part.hi,
                volume_m3: part.volume_m3 * cfg.rubble_fines * cfg.rubble_bulking,
            });
            return Vec::new();
        };
        let drop = (part.bottom_m - top).max(0.0);
        let kn = part.mass_kg * 9.81e-3 * cfg.impact_factor * (1.0 + drop / 3.5).min(2.0) / slabs.len().max(1) as f32;
        let gave = self.frame_mut(b as usize).impact(&slabs, kn, &cfg);
        if gave.is_empty() {
            return Vec::new();
        }
        // It punches through: what stood on the broken bays falls too, and the part goes on down
        // over the bays that gave.
        let mut chunks = self.chunks_of(b, &gave);
        chunks.extend(self.settle(b));
        if let Some(f) = self.buildings[b as usize].frame.as_ref() {
            part.cells.retain(|c| gave.iter().any(|&s| f.elements[s as usize].at == *c));
            part.bottom_m = top - cfg.slab_m;
        }
        if !part.cells.is_empty() {
            self.drop_part(b, part, self.time_s);
        }
        self.sync(b as usize);
        chunks
    }

    /// Moments since the last call, for the effects.
    pub fn take_moments(&mut self) -> Vec<Moment> {
        std::mem::take(&mut self.moments)
    }

    /// Heaps that reached the ground since the last call, for the ground to raise.
    pub fn take_landings(&mut self) -> Vec<Landing> {
        std::mem::take(&mut self.landings)
    }

    /// Advances time by `dt_s`: every creaking member whose time is up fails, and its building
    /// settles. Returns the buildings that changed and the chunks that fell.
    pub fn step(&mut self, dt_s: f32) -> (Vec<u32>, Vec<Chunk>) {
        self.time_s += dt_s;
        let now = self.time_s;
        let due: Vec<u32> = (0..self.buildings.len())
            .filter(|&b| {
                self.buildings[b]
                    .frame
                    .as_ref()
                    .and_then(|f| f.next_due())
                    .is_some_and(|t| t <= now)
            })
            .map(|b| b as u32)
            .collect();
        let mut chunks = Vec::new();
        for &b in &due {
            if let Some(f) = self.buildings[b as usize].frame.as_mut() {
                let failed = f.fail_due(now);
                let at: Vec<Vec3> = failed.iter().map(|&id| f.elements[id as usize].centre()).collect();
                self.moments.extend(at.into_iter().map(Moment::Fail));
            }
            chunks.extend(self.settle(b));
        }
        let mut changed = due;
        while self.falls.first().is_some_and(|f| f.at_s <= now) {
            let fall = self.falls.remove(0);
            changed.push(fall.building);
            chunks.extend(self.land(fall));
        }
        changed.sort_unstable();
        changed.dedup();
        (changed, chunks)
    }

    /// Falling elements as chunks: one per storey per cell, the union of their boxes. (2 x 2 cell
    /// groups fell as 12 m slabs in the 2026-10-24 playtest; a cell is about 6 m.)
    fn chunks_of(&self, b: u32, fell: &[u32]) -> Vec<Chunk> {
        let Some(f) = self.buildings[b as usize].frame.as_ref() else {
            return Vec::new();
        };
        let mut groups: Vec<((u32, u32, u32), Vec3, Vec3)> = Vec::new();
        for &id in fell {
            let e = &f.elements[id as usize];
            let i = e.at[0].min(f.n[0] as u32 - 1);
            let j = e.at[1].min(f.n[2] as u32 - 1);
            let key = (e.storey, i, j);
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => {
                    g.1 = g.1.min(e.min);
                    g.2 = g.2.max(e.max);
                }
                None => groups.push((key, e.min, e.max)),
            }
        }
        groups.sort_by_key(|g| g.0);
        groups
            .into_iter()
            .map(|(_, lo, hi)| Chunk {
                building: b,
                centre: (lo + hi) * 0.5,
                size: hi - lo,
            })
            .collect()
    }

    /// Brings building `b`'s cells in line with its frame.
    fn sync(&mut self, b: usize) {
        let c = &mut self.buildings[b];
        let Some(f) = c.frame.as_ref() else {
            return;
        };
        let mut standing = 0;
        for k in 0..c.n[1] {
            for j in 0..c.n[2] {
                for i in 0..c.n[0] {
                    let closed = f.cell_closed(i, k, j);
                    let idx = (k * c.n[2] + j) * c.n[0] + i;
                    c.intact[idx] = closed;
                    standing += closed as usize;
                }
            }
        }
        c.standing = standing;
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

    /// A point on the outer face of cell (i, k, 0), half a metre in.
    fn on_face(c: &Cells, i: usize, k: usize) -> Vec3 {
        c.centre(i, k, 0) - Vec3::Z * (c.size.z * 0.5 - 0.5)
    }

    /// Steps the structures for `secs` at the fixed rate, collecting chunks.
    fn run(s: &mut Structures, secs: f32) -> Vec<Chunk> {
        let mut out = Vec::new();
        for _ in 0..(secs * 60.0) as usize {
            out.extend(s.step(1.0 / 60.0).1);
        }
        out
    }

    #[test]
    fn a_shell_holes_a_wall_without_bringing_the_building_down() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        let at = on_face(&c, c.n[0] / 2, c.n[1] / 2);
        let (broke, fell) = s.break_sphere(b, at, 3.4);
        assert!(broke >= 1);
        let fell: Vec<Chunk> = fell.into_iter().chain(run(&mut s, 3.0)).collect();
        let cells = &s.buildings[b as usize];
        assert!(fell.is_empty(), "a hole in a wall drops nothing");
        assert!(cells.standing >= c.standing - 2, "the tower stands");
        assert!(!cells.solid_at(at), "the hole is open");
    }

    #[test]
    fn taking_out_the_ground_floor_brings_down_the_storeys_above() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        let mut chunks = Vec::new();
        for j in 0..=c.n[2] {
            for i in 0..=c.n[0] {
                let node = c.origin + Vec3::new(c.size.x * i as f32, 1.5, c.size.z * j as f32);
                chunks.extend(s.break_sphere(b, node, 3.0).1);
            }
        }
        chunks.extend(run(&mut s, 5.0));
        assert!(
            s.buildings[b as usize].standing <= c.n[0] * c.n[2],
            "nothing above the ground storey stands"
        );
        assert!(chunks.len() >= c.n[1] - 1, "every storey above fell as chunks");
    }

    #[test]
    fn a_mech_walks_through_a_hole_it_blew() {
        let mut s = Structures::new(plan());
        let b = plan().buildings.iter().find(|b| b.storeys <= 4).unwrap().id;
        let c = s.buildings[b as usize].clone();
        let full = s.solid_rects(b, c.origin.y, c.origin.y + 3.0).len();
        s.break_sphere(b, on_face(&c, 0, 0), 4.5);
        assert!(s.solid_rects(b, c.origin.y, c.origin.y + 3.0).len() < full);
    }

    #[test]
    fn a_falling_top_punches_down_through_the_floors_below() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        let k = c.n[1] / 2;
        // Every column of the middle storey goes: the top half falls onto the storey below.
        let cfg = s.cfg.clone();
        {
            let f = s.frame_mut(b as usize);
            for a in 0..=c.n[0] {
                for cc in 0..=c.n[2] {
                    let id = f.column(k, a, cc) as usize;
                    f.elements[id].alive = false;
                }
            }
            let _ = cfg;
        }
        s.settle(b);
        run(&mut s, 12.0);
        let closed_below = (0..k)
            .filter(|&kk| (0..c.n[0]).any(|i| (0..c.n[2]).any(|j| s.buildings[b as usize].intact[(kk * c.n[2] + j) * c.n[0] + i])))
            .count();
        assert!(
            closed_below < k,
            "the falling top crushed storeys below the cut ({closed_below} of {k} left)"
        );
    }

    #[test]
    fn a_collapse_leaves_a_heap_on_the_ground() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        for j in 0..=c.n[2] {
            for i in 0..=c.n[0] {
                let node = c.origin + Vec3::new(c.size.x * i as f32, 1.5, c.size.z * j as f32);
                s.break_sphere(b, node, 3.0);
            }
        }
        run(&mut s, 12.0);
        let heaps = s.take_landings();
        assert!(!heaps.is_empty(), "rubble reached the ground");
        let moments = s.take_moments();
        assert!(
            moments.iter().any(|m| matches!(m, Moment::Impact { .. })),
            "the landing is a moment for the effects"
        );
        assert!(heaps.iter().map(|h| h.volume_m3).sum::<f32>() > 1000.0, "a tower makes a big heap");
    }

    #[test]
    fn the_same_collapse_lands_the_same_heaps() {
        let go = || {
            let mut s = Structures::new(plan());
            let b = tallest();
            let c = s.buildings[b as usize].clone();
            for i in 0..=c.n[0] {
                let node = c.origin + Vec3::new(c.size.x * i as f32, 1.5, 0.0);
                s.break_sphere(b, node, 3.0);
            }
            run(&mut s, 10.0);
            (s.take_landings(), s.buildings[b as usize].intact.clone())
        };
        assert_eq!(go(), go());
    }

    #[test]
    fn an_unhit_building_has_no_frame() {
        let mut s = Structures::new(plan());
        let b = tallest();
        let c = s.buildings[b as usize].clone();
        s.break_sphere(b, on_face(&c, 0, 1), 3.4);
        assert!(s.buildings[b as usize].frame.is_some());
        assert_eq!(s.buildings.iter().filter(|c| c.frame.is_some()).count(), 1);
    }
}
