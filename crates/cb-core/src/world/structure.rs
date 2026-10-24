//! A building's structural frame (openspec change `structural-destruction`): columns on the nodes
//! of the building's 6 m cell grid, a floor bay per cell per storey and a wall panel per facade cell
//! per storey, each resting on the elements below it.
//!
//! Connectivity from the ground decides what falls: anything left standing on nothing drops as an
//! island. A top-down load pass decides what gives: floors put their weight on the corner columns
//! whose column line still reaches the ground, columns pass it down, and a member loaded past its
//! capacity (twice its intact load, less for a floor held on one edge) creaks, then fails. Pure and
//! deterministic: the creak time comes from the element's id, never a clock or a shared stream.
//! It lives in the core because the outcome (what stands) is simulation; `destruct.rs` keeps the
//! cell view the rest of the game queries.

use crate::rng::SimRng;
use glam::Vec3;

/// The ground, as a support.
pub const GROUND: u32 = u32::MAX;
/// `Element::side` of anything that is not a wall.
pub const NO_SIDE: u8 = u8::MAX;

/// What an element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A column on a grid node, one storey tall.
    Column,
    /// A floor bay: one cell's ceiling (the floor of the storey above).
    Slab,
    /// A facade panel: one cell's outer face for one storey.
    Wall,
}

impl Kind {
    /// Solve order within a storey: what rests on a thing is solved before it.
    fn rank(self) -> u8 {
        match self {
            Kind::Wall => 0,
            Kind::Slab => 1,
            Kind::Column => 2,
        }
    }
}

/// One piece of the frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    /// What it is.
    pub kind: Kind,
    /// Storey, from 0 at the ground.
    pub storey: u32,
    /// Cell (slab, wall) or node (column) on the grid, as (x, z) indices.
    pub at: [u32; 2],
    /// Facade side of a wall (0 faces -z, 1 +z, 2 -x, 3 +x); `NO_SIDE` for the rest.
    pub side: u8,
    /// Least corner of its box.
    pub min: Vec3,
    /// Greatest corner of its box.
    pub max: Vec3,
    /// Mass.
    pub mass_kg: f32,
    /// Blast energy it can still take before it breaks.
    pub hp_mj: f32,
    /// True while it stands.
    pub alive: bool,
    /// Load over capacity at the last solve (0 for walls); over 1 means it is failing.
    pub stress: f32,
    /// What it stands on: `GROUND` or element indices. A column lists the column below first.
    rests: Vec<u32>,
    /// Neighbouring floor bays it can hang from (slabs only).
    lateral: Vec<u32>,
    /// What stands on it.
    carries: Vec<u32>,
    /// 0 unsupported, 1 held from the ground, 2 hanging from a neighbour.
    sup: u8,
    /// A column whose column line reaches the ground (it carries load).
    line: bool,
    load_kn: f32,
    cap_kn: f32,
    /// Time it fails at, once overloaded.
    due_s: Option<f32>,
}

impl Element {
    /// Centre of its box.
    pub fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn distance_to(&self, p: Vec3) -> f32 {
        (self.min - p).max(p - self.max).max(Vec3::ZERO).length()
    }
}

/// Tuning for frames, from the `structure:` section of `data/world/world.ron`.
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct StructureConfig {
    /// Capacity of a column or floor bay over the load it carries intact.
    pub safety_factor: f32,
    /// Shortest and longest creak before an overloaded member fails.
    pub creak_s: [f32; 2],
    /// Share of capacity a floor bay keeps held on four corners.
    pub held_four: f32,
    /// ...on three corners.
    pub held_three: f32,
    /// ...on two corners across a diagonal.
    pub held_diagonal: f32,
    /// ...on the two corners of one edge (a cantilever).
    pub held_one_edge: f32,
    /// ...on one corner.
    pub held_one: f32,
    /// ...hanging from a neighbouring bay.
    pub held_hung: f32,
    /// Blast energy a column takes before it breaks.
    pub column_hp_mj: f32,
    /// Blast energy a floor bay takes.
    pub slab_hp_mj: f32,
    /// Blast energy a wall panel takes.
    pub wall_hp_mj: f32,
    /// Floor bay mass per square metre (0.25 m of concrete).
    pub slab_kg_m2: f32,
    /// Column mass per metre of height (0.6 m square concrete).
    pub column_kg_m: f32,
    /// Facade mass per square metre (brick and glass).
    pub wall_kg_m2: f32,
    /// Floor bay thickness.
    pub slab_m: f32,
    /// Column width.
    pub column_m: f32,
    /// Facade thickness.
    pub wall_m: f32,
    /// A falling part loads the floor it lands on with its weight times this (up to twice that
    /// again for a long drop).
    pub impact_factor: f32,
    /// Share of a fallen part's volume that becomes heap (the rest is the loose pieces).
    pub rubble_fines: f32,
    /// Crushed volume over solid volume.
    pub rubble_bulking: f32,
    /// Steepest a heap stands: rise per metre (0.75 is about 37 degrees).
    pub repose_slope: f32,
    /// Highest a heap stands above the ground it fell on.
    pub max_rubble_m: f32,
    /// How far past a fallen part's footprint its heap spreads.
    pub rubble_spread_m: f32,
}

impl Default for StructureConfig {
    fn default() -> Self {
        Self {
            safety_factor: 2.0,
            creak_s: [0.5, 1.6],
            held_four: 1.0,
            held_three: 0.85,
            held_diagonal: 0.7,
            held_one_edge: 0.45,
            held_one: 0.25,
            held_hung: 0.4,
            column_hp_mj: 2.5,
            slab_hp_mj: 3.0,
            wall_hp_mj: 0.7,
            slab_kg_m2: 600.0,
            column_kg_m: 864.0,
            wall_kg_m2: 400.0,
            slab_m: 0.25,
            column_m: 0.6,
            wall_m: 0.3,
            impact_factor: 2.0,
            rubble_fines: 0.5,
            rubble_bulking: 1.4,
            repose_slope: 0.75,
            max_rubble_m: 18.0,
            rubble_spread_m: 4.0,
        }
    }
}

impl crate::data::Validate for StructureConfig {
    fn validate(&self) -> Result<(), String> {
        use crate::data::{positive, positive_range, within};
        positive("structure.safety_factor", self.safety_factor)?;
        positive_range("structure.creak_s", self.creak_s)?;
        for (name, v) in [
            ("structure.held_four", self.held_four),
            ("structure.held_three", self.held_three),
            ("structure.held_diagonal", self.held_diagonal),
            ("structure.held_one_edge", self.held_one_edge),
            ("structure.held_one", self.held_one),
            ("structure.held_hung", self.held_hung),
        ] {
            within(name, v, 0.0, 1.0)?;
        }
        within("structure.rubble_fines", self.rubble_fines, 0.0, 1.0)?;
        for (name, v) in [
            ("structure.column_hp_mj", self.column_hp_mj),
            ("structure.slab_hp_mj", self.slab_hp_mj),
            ("structure.wall_hp_mj", self.wall_hp_mj),
            ("structure.slab_kg_m2", self.slab_kg_m2),
            ("structure.column_kg_m", self.column_kg_m),
            ("structure.wall_kg_m2", self.wall_kg_m2),
            ("structure.slab_m", self.slab_m),
            ("structure.column_m", self.column_m),
            ("structure.wall_m", self.wall_m),
            ("structure.impact_factor", self.impact_factor),
            ("structure.rubble_bulking", self.rubble_bulking),
            ("structure.repose_slope", self.repose_slope),
            ("structure.max_rubble_m", self.max_rubble_m),
            ("structure.rubble_spread_m", self.rubble_spread_m),
        ] {
            positive(name, v)?;
        }
        Ok(())
    }
}

/// A building's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// The building's id (seeds the creak times).
    pub building: u32,
    /// Every element, in generation order (storey, then columns, slabs, walls).
    pub elements: Vec<Element>,
    /// Cells along x, storeys, cells along z.
    pub n: [usize; 3],
    /// Least corner of the grid.
    pub origin: Vec3,
    /// Cell size.
    pub size: Vec3,
    /// Solve order: storey down, then kind rank, then index.
    order: Vec<u32>,
    /// Element index per node per storey.
    columns: Vec<u32>,
    /// Element index per cell per storey.
    slabs: Vec<u32>,
    /// Element index per facade side, position and storey (`GROUND` where a cell is not on that side).
    walls: Vec<u32>,
}

/// Where a fallen part is (see `Frame::footprint`).
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    /// Grid cells under it, sorted.
    pub cells: Vec<[u32; 2]>,
    /// Its lowest point.
    pub bottom_m: f32,
    /// Its mass.
    pub mass_kg: f32,
    /// Its solid volume.
    pub volume_m3: f32,
    /// Least (x, z).
    pub lo: glam::Vec2,
    /// Greatest (x, z).
    pub hi: glam::Vec2,
}

/// What a settle did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settled {
    /// Elements that lost their support and fell, as islands.
    pub fell: Vec<u32>,
    /// Elements broken on the spot (a floor bay with nothing left to hang from).
    pub broke: usize,
    /// Members that started to creak (overloaded, failing soon).
    pub creaking: Vec<u32>,
}

impl Frame {
    /// The frame of a box building: `n` = [cells along x, storeys, cells along z], `origin` its
    /// least corner (on the ground), `size` one cell. Capacities come from a design solve of the
    /// intact frame.
    pub fn new(building: u32, n: [usize; 3], origin: Vec3, size: Vec3, cfg: &StructureConfig) -> Self {
        let [nx, ns, nz] = n;
        let mut f = Frame {
            building,
            elements: Vec::new(),
            n,
            origin,
            size,
            order: Vec::new(),
            columns: vec![GROUND; ns * (nx + 1) * (nz + 1)],
            slabs: vec![GROUND; ns * nx * nz],
            walls: vec![GROUND; ns * 4 * nx.max(nz)],
        };
        let t = cfg.slab_m;
        let half = cfg.column_m * 0.5;
        for k in 0..ns {
            let y0 = origin.y + size.y * k as f32;
            let y1 = y0 + size.y;
            for a in 0..=nx {
                for c in 0..=nz {
                    let p = origin + Vec3::new(size.x * a as f32, 0.0, size.z * c as f32);
                    let mut rests = Vec::new();
                    if k == 0 {
                        rests.push(GROUND);
                    } else {
                        rests.push(f.column(k - 1, a, c));
                        for (i, j) in [
                            (a.wrapping_sub(1), c.wrapping_sub(1)),
                            (a, c.wrapping_sub(1)),
                            (a.wrapping_sub(1), c),
                            (a, c),
                        ] {
                            if i < nx && j < nz {
                                rests.push(f.slab(k - 1, i, j));
                            }
                        }
                    }
                    let id = f.push(
                        Kind::Column,
                        k,
                        [a, c],
                        Vec3::new(p.x - half, y0, p.z - half),
                        Vec3::new(p.x + half, y1 - t, p.z + half),
                        cfg.column_kg_m * (size.y - t),
                        cfg.column_hp_mj,
                        rests,
                    );
                    f.columns[(k * (nx + 1) + a) * (nz + 1) + c] = id;
                }
            }
            for i in 0..nx {
                for j in 0..nz {
                    let lo = origin + Vec3::new(size.x * i as f32, size.y * (k + 1) as f32 - t, size.z * j as f32);
                    let rests = [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)]
                        .map(|(a, c)| f.column(k, a, c))
                        .to_vec();
                    let id = f.push(
                        Kind::Slab,
                        k,
                        [i, j],
                        lo,
                        lo + Vec3::new(size.x, t, size.z),
                        cfg.slab_kg_m2 * size.x * size.z,
                        cfg.slab_hp_mj,
                        rests,
                    );
                    f.slabs[(k * nx + i) * nz + j] = id;
                }
            }
            for i in 0..nx {
                for j in 0..nz {
                    let s = f.slab(k, i, j) as usize;
                    let mut lateral = Vec::new();
                    for (di, dj) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                        let (ii, jj) = (i as i64 + di, j as i64 + dj);
                        if ii >= 0 && jj >= 0 && (ii as usize) < nx && (jj as usize) < nz {
                            lateral.push(f.slab(k, ii as usize, jj as usize));
                        }
                    }
                    f.elements[s].lateral = lateral;
                }
            }
            // Facade: side 0 faces -z, 1 +z, 2 -x, 3 +x.
            let w = cfg.wall_m;
            for side in 0..4usize {
                let count = if side < 2 { nx } else { nz };
                for idx in 0..count {
                    let (cell, nodes) = match side {
                        0 => ([idx, 0], [[idx, 0], [idx + 1, 0]]),
                        1 => ([idx, nz - 1], [[idx, nz], [idx + 1, nz]]),
                        2 => ([0, idx], [[0, idx], [0, idx + 1]]),
                        _ => ([nx - 1, idx], [[nx, idx], [nx, idx + 1]]),
                    };
                    let lo_c = Vec3::new(origin.x + size.x * cell[0] as f32, y0, origin.z + size.z * cell[1] as f32);
                    let hi_c = lo_c + Vec3::new(size.x, size.y - t, size.z);
                    let (lo, hi) = match side {
                        0 => (lo_c, Vec3::new(hi_c.x, hi_c.y, lo_c.z + w)),
                        1 => (Vec3::new(lo_c.x, lo_c.y, hi_c.z - w), hi_c),
                        2 => (lo_c, Vec3::new(lo_c.x + w, hi_c.y, hi_c.z)),
                        _ => (Vec3::new(hi_c.x - w, lo_c.y, lo_c.z), hi_c),
                    };
                    let mut rests = vec![if k == 0 { GROUND } else { f.slab(k - 1, cell[0], cell[1]) }];
                    rests.extend(nodes.map(|[a, c]| f.column(k, a, c)));
                    let along = if side < 2 { size.x } else { size.z };
                    let id = f.push(
                        Kind::Wall,
                        k,
                        cell,
                        lo,
                        hi,
                        cfg.wall_kg_m2 * along * (size.y - t),
                        cfg.wall_hp_mj,
                        rests,
                    );
                    f.elements[id as usize].side = side as u8;
                    let wi = f.wall_index(k, side, idx);
                    f.walls[wi] = id;
                }
            }
        }
        for id in 0..f.elements.len() {
            for r in f.elements[id].rests.clone() {
                if r != GROUND {
                    f.elements[r as usize].carries.push(id as u32);
                }
            }
        }
        let mut order: Vec<u32> = (0..f.elements.len() as u32).collect();
        order.sort_by(|&p, &q| {
            let (ep, eq) = (&f.elements[p as usize], &f.elements[q as usize]);
            eq.storey.cmp(&ep.storey).then(ep.kind.rank().cmp(&eq.kind.rank())).then(p.cmp(&q))
        });
        f.order = order;
        f.solve(cfg, true);
        for e in &mut f.elements {
            if matches!(e.kind, Kind::Column | Kind::Slab) {
                e.cap_kn = e.load_kn.max(1.0) * cfg.safety_factor;
            }
        }
        f.solve(cfg, false);
        f
    }

    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, kind: Kind, storey: usize, at: [usize; 2], min: Vec3, max: Vec3, mass_kg: f32, hp_mj: f32, rests: Vec<u32>) -> u32 {
        self.elements.push(Element {
            kind,
            storey: storey as u32,
            at: [at[0] as u32, at[1] as u32],
            side: NO_SIDE,
            min,
            max,
            mass_kg,
            hp_mj,
            alive: true,
            stress: 0.0,
            rests,
            lateral: Vec::new(),
            carries: Vec::new(),
            sup: 0,
            line: false,
            load_kn: 0.0,
            cap_kn: f32::INFINITY,
            due_s: None,
        });
        (self.elements.len() - 1) as u32
    }

    /// The column on node (a, c) of storey k.
    pub fn column(&self, k: usize, a: usize, c: usize) -> u32 {
        self.columns[(k * (self.n[0] + 1) + a) * (self.n[2] + 1) + c]
    }

    /// The floor bay of cell (i, j) at the top of storey k.
    pub fn slab(&self, k: usize, i: usize, j: usize) -> u32 {
        self.slabs[(k * self.n[0] + i) * self.n[2] + j]
    }

    /// The facade panel of storey k on `side` at position `idx`, or `GROUND` if there is none.
    pub fn wall(&self, k: usize, side: usize, idx: usize) -> u32 {
        self.walls.get(self.wall_index(k, side, idx)).copied().unwrap_or(GROUND)
    }

    fn wall_index(&self, k: usize, side: usize, idx: usize) -> usize {
        (k * 4 + side) * self.n[0].max(self.n[2]) + idx
    }

    /// True when cell (i, storey k, j) is still closed: its floor bay stands and so do the facade
    /// panels on its outer faces.
    pub fn cell_closed(&self, i: usize, k: usize, j: usize) -> bool {
        let alive = |id: u32| id != GROUND && self.elements[id as usize].alive;
        if !alive(self.slab(k, i, j)) {
            return false;
        }
        let [nx, _, nz] = self.n;
        let faces = [(j == 0, 0, i), (j + 1 == nz, 1, i), (i == 0, 2, j), (i + 1 == nx, 3, j)];
        faces
            .iter()
            .filter(|f| f.0)
            .all(|&(_, side, idx)| alive(self.walls[self.wall_index(k, side, idx)]))
    }

    /// Takes a blast of `energy_mj` centred at `centre` reaching `radius_m`: each standing element
    /// within reach loses `E x (1 - d/R)^2` and breaks at zero. Returns how many broke.
    pub fn blast(&mut self, centre: Vec3, radius_m: f32, energy_mj: f32) -> usize {
        if !(radius_m > 0.0 && energy_mj > 0.0 && centre.is_finite()) {
            return 0;
        }
        let mut broke = 0;
        for e in &mut self.elements {
            if !e.alive {
                continue;
            }
            let d = e.distance_to(centre);
            if d >= radius_m {
                continue;
            }
            e.hp_mj -= energy_mj * (1.0 - d / radius_m).powi(2);
            if e.hp_mj <= 0.0 {
                e.alive = false;
                broke += 1;
            }
        }
        broke
    }

    /// Lets the frame find its new balance at time `now_s`: islands fall, floor bays with nothing
    /// to hang from break, overloaded members are given a creak and a time to fail.
    pub fn settle(&mut self, now_s: f32, cfg: &StructureConfig) -> Settled {
        let mut out = Settled::default();
        for _ in 0..16 {
            let (fell, fail_now) = self.solve(cfg, false);
            out.fell.extend(&fell);
            out.broke += fail_now.len();
            for id in &fail_now {
                self.elements[*id as usize].alive = false;
            }
            if fell.is_empty() && fail_now.is_empty() {
                break;
            }
        }
        let [lo, hi] = cfg.creak_s;
        let building = self.building;
        for (id, e) in self.elements.iter_mut().enumerate() {
            if e.alive && e.stress > 1.0 && e.due_s.is_none() {
                let mut r = SimRng::new(((building as u64) << 32) | id as u64).fork("creak");
                e.due_s = Some(now_s + lo + (hi - lo) * r.next_f32());
                out.creaking.push(id as u32);
            } else if e.stress <= 1.0 {
                e.due_s = None;
            }
        }
        out
    }

    /// The earliest time an overloaded member fails, if any is creaking.
    pub fn next_due(&self) -> Option<f32> {
        self.elements.iter().filter(|e| e.alive).filter_map(|e| e.due_s).reduce(f32::min)
    }

    /// Breaks every member whose creak has run out by `now_s`. Returns the ones that broke.
    pub fn fail_due(&mut self, now_s: f32) -> Vec<u32> {
        let mut broke = Vec::new();
        for (id, e) in self.elements.iter_mut().enumerate() {
            if e.alive && e.due_s.is_some_and(|t| t <= now_s) {
                e.alive = false;
                e.due_s = None;
                broke.push(id as u32);
            }
        }
        broke
    }

    /// Where a set of fallen elements is: the grid cells under them, the lowest point, their mass
    /// and solid volume, and their (x, z) extent.
    pub fn footprint(&self, ids: &[u32]) -> Option<Footprint> {
        let mut out = Footprint {
            cells: Vec::new(),
            bottom_m: f32::MAX,
            mass_kg: 0.0,
            volume_m3: 0.0,
            lo: glam::Vec2::splat(f32::MAX),
            hi: glam::Vec2::splat(f32::MIN),
        };
        for &id in ids {
            let e = &self.elements[id as usize];
            let cell = [e.at[0].min(self.n[0] as u32 - 1), e.at[1].min(self.n[2] as u32 - 1)];
            if !out.cells.contains(&cell) {
                out.cells.push(cell);
            }
            out.bottom_m = out.bottom_m.min(e.min.y);
            out.mass_kg += e.mass_kg;
            let d = e.max - e.min;
            out.volume_m3 += d.x * d.y * d.z;
            out.lo = out.lo.min(glam::Vec2::new(e.min.x, e.min.z));
            out.hi = out.hi.max(glam::Vec2::new(e.max.x, e.max.z));
        }
        out.cells.sort_unstable();
        (!ids.is_empty()).then_some(out)
    }

    /// The highest standing floor under any of `cells` whose top is at or below `y_m`: its height
    /// and the standing floor bays at that level under `cells`.
    pub fn floor_below(&self, cells: &[[u32; 2]], y_m: f32) -> Option<(f32, Vec<u32>)> {
        let mut best: Option<usize> = None;
        for &[i, j] in cells {
            for k in (0..self.n[1]).rev() {
                let e = &self.elements[self.slab(k, i as usize, j as usize) as usize];
                if e.alive && e.max.y <= y_m + 0.1 {
                    if best.is_none_or(|b| k > b) {
                        best = Some(k);
                    }
                    break;
                }
            }
        }
        let k = best?;
        let slabs: Vec<u32> = cells
            .iter()
            .map(|&[i, j]| self.slab(k, i as usize, j as usize))
            .filter(|&s| self.elements[s as usize].alive)
            .collect();
        Some((self.origin.y + self.size.y * (k + 1) as f32, slabs))
    }

    /// A falling part lands on `slabs`, loading each with `kn_each` on top of what it carries.
    /// Returns the bays that give way (they are broken; settle afterwards).
    pub fn impact(&mut self, slabs: &[u32], kn_each: f32, cfg: &StructureConfig) -> Vec<u32> {
        let mut gave = Vec::new();
        for &s in slabs {
            let e = &self.elements[s as usize];
            if !e.alive || !kn_each.is_finite() {
                continue;
            }
            let cap = e.cap_kn * held_share(&self.elements, s as usize, cfg);
            if e.load_kn + kn_each > cap {
                gave.push(s);
            }
        }
        for &s in &gave {
            self.elements[s as usize].alive = false;
        }
        gave
    }

    /// Standing elements.
    pub fn standing(&self) -> usize {
        self.elements.iter().filter(|e| e.alive).count()
    }

    /// One pass: connectivity, islands (removed and returned), load line, load pass, stress. In a
    /// design pass nothing is removed and capacities are not checked.
    fn solve(&mut self, cfg: &StructureConfig, design: bool) -> (Vec<u32>, Vec<u32>) {
        let els = &mut self.elements;
        for e in els.iter_mut() {
            e.sup = 0;
        }
        // 1. Connectivity from the ground.
        let mut queue: Vec<u32> = Vec::new();
        for (id, e) in els.iter_mut().enumerate() {
            if e.alive && e.rests.contains(&GROUND) {
                e.sup = 1;
                queue.push(id as u32);
            }
        }
        fn spread(els: &mut [Element], queue: &mut Vec<u32>, mark: u8) {
            while let Some(id) = queue.pop() {
                for ci in 0..els[id as usize].carries.len() {
                    let c = els[id as usize].carries[ci] as usize;
                    if els[c].alive && els[c].sup == 0 {
                        els[c].sup = mark;
                        queue.push(c as u32);
                    }
                }
            }
        }
        spread(els, &mut queue, 1);
        // A floor bay with nothing under it may hang from a neighbour held directly (one hop).
        let mut hung = Vec::new();
        for id in 0..els.len() {
            let e = &els[id];
            if e.kind == Kind::Slab && e.alive && e.sup == 0 && e.lateral.iter().any(|&n| els[n as usize].alive && els[n as usize].sup == 1)
            {
                hung.push(id as u32);
            }
        }
        for &h in &hung {
            els[h as usize].sup = 2;
        }
        spread(els, &mut hung, 2);
        // 2. Islands.
        let mut fell = Vec::new();
        if !design {
            for (id, e) in els.iter_mut().enumerate() {
                if e.alive && e.sup == 0 {
                    e.alive = false;
                    e.due_s = None;
                    fell.push(id as u32);
                }
            }
        }
        // 3. Load line, bottom up (the order runs top down).
        for oi in (0..self.order.len()).rev() {
            let id = self.order[oi] as usize;
            if els[id].kind != Kind::Column {
                continue;
            }
            let first = els[id].rests[0];
            let line = els[id].alive
                && els[id].sup > 0
                && (first == GROUND || (els[first as usize].kind == Kind::Column && els[first as usize].alive && els[first as usize].line));
            els[id].line = line;
        }
        // 4. Load pass, top down: each element splits its load over what carries it.
        for e in els.iter_mut() {
            e.load_kn = if e.alive && e.sup > 0 { e.mass_kg * 9.81e-3 } else { 0.0 };
        }
        let mut fail_now = Vec::new();
        let mut targets: Vec<u32> = Vec::with_capacity(8);
        for oi in 0..self.order.len() {
            let id = self.order[oi] as usize;
            if !els[id].alive || els[id].sup == 0 {
                continue;
            }
            targets.clear();
            let e = &els[id];
            match e.kind {
                Kind::Slab => {
                    targets.extend(e.rests.iter().copied().filter(|&r| els[r as usize].alive && els[r as usize].line));
                    if targets.is_empty() {
                        targets.extend(e.lateral.iter().copied().filter(|&n| {
                            let ne = &els[n as usize];
                            ne.alive && ne.sup == 1 && ne.rests.iter().any(|&r| els[r as usize].alive && els[r as usize].line)
                        }));
                    }
                    if targets.is_empty() {
                        if !design {
                            fail_now.push(id as u32);
                        }
                        continue;
                    }
                }
                Kind::Column if e.line => {
                    if e.rests[0] != GROUND {
                        targets.push(e.rests[0]);
                    }
                }
                _ => targets.extend(
                    e.rests
                        .iter()
                        .copied()
                        .filter(|&r| r != GROUND && els[r as usize].alive && els[r as usize].sup > 0),
                ),
            }
            if targets.is_empty() {
                continue;
            }
            let share = els[id].load_kn / targets.len() as f32;
            for &t in &targets {
                els[t as usize].load_kn += share;
            }
        }
        // 5. Stress.
        for id in 0..els.len() {
            let e = &els[id];
            let stress = if design || !e.alive || !e.cap_kn.is_finite() {
                0.0
            } else {
                let held = if e.kind == Kind::Slab { held_share(els, id, cfg) } else { 1.0 };
                e.load_kn / (e.cap_kn * held)
            };
            els[id].stress = stress;
        }
        (fell, fail_now)
    }
}

/// How much of its capacity a floor bay keeps, by which of its corner columns still carry load.
fn held_share(els: &[Element], id: usize, cfg: &StructureConfig) -> f32 {
    let held: Vec<[u32; 2]> = els[id]
        .rests
        .iter()
        .filter(|&&r| els[r as usize].alive && els[r as usize].line)
        .map(|&r| els[r as usize].at)
        .collect();
    match held.len() {
        4.. => cfg.held_four,
        3 => cfg.held_three,
        2 if held[0][0] == held[1][0] || held[0][1] == held[1][1] => cfg.held_one_edge,
        2 => cfg.held_diagonal,
        1 => cfg.held_one,
        _ => cfg.held_hung,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 6 x 6 bay, 12-storey block of 6 m cells and 3.5 m storeys.
    fn block() -> Frame {
        Frame::new(7, [6, 12, 6], Vec3::ZERO, Vec3::new(6.0, 3.5, 6.0), &StructureConfig::default())
    }

    fn run(f: &mut Frame, from_s: f32, to_s: f32) -> Settled {
        let cfg = StructureConfig::default();
        let mut out = f.settle(from_s, &cfg);
        let mut t = from_s;
        while t < to_s {
            t += 1.0 / 60.0;
            if !f.fail_due(t).is_empty() {
                let s = f.settle(t, &cfg);
                out.fell.extend(s.fell);
                out.broke += s.broke;
            }
        }
        out
    }

    fn ground_columns(f: &Frame, pick: impl Fn(usize, usize) -> bool) -> Vec<usize> {
        (0..=f.n[0])
            .flat_map(|a| (0..=f.n[2]).map(move |c| (a, c)))
            .filter(|&(a, c)| pick(a, c))
            .map(|(a, c)| f.column(0, a, c) as usize)
            .collect()
    }

    #[test]
    fn a_new_frame_stands_with_every_member_at_half_its_capacity() {
        let f = block();
        assert_eq!(f.standing(), f.elements.len());
        for e in f.elements.iter().filter(|e| e.kind != Kind::Wall) {
            assert!((e.stress - 0.5).abs() < 1e-3, "{:?} at {}", e.kind, e.stress);
        }
    }

    #[test]
    fn losing_one_ground_column_brings_nothing_else_down() {
        let mut f = block();
        let id = f.column(0, 2, 2) as usize;
        f.elements[id].alive = false;
        let before = f.standing();
        let s = run(&mut f, 0.0, 5.0);
        assert!(s.fell.is_empty() && s.broke == 0);
        assert_eq!(f.standing(), before, "the floors above bridge over it");
    }

    #[test]
    fn losing_a_whole_ground_side_peels_off_the_bays_above_it() {
        let mut f = block();
        for id in ground_columns(&f, |_, c| c == 0) {
            f.elements[id].alive = false;
        }
        run(&mut f, 0.0, 5.0);
        let k_mid = f.n[1] / 2;
        assert!(!f.cell_closed(2, k_mid, 0), "the side bays fell");
        assert!(f.cell_closed(2, k_mid, 3), "the inner bays stand");
    }

    #[test]
    fn the_creak_comes_before_the_fall() {
        let mut f = block();
        for id in ground_columns(&f, |_, c| c == 0) {
            f.elements[id].alive = false;
        }
        let cfg = StructureConfig::default();
        f.settle(0.0, &cfg);
        assert!(
            f.next_due().is_some_and(|t| (0.5..=1.6).contains(&t)),
            "something creaks within the creak time"
        );
        assert!(f.cell_closed(2, 6, 0), "nothing has fallen yet");
    }

    #[test]
    fn losing_every_ground_column_brings_the_building_down() {
        let mut f = block();
        for id in ground_columns(&f, |_, _| true) {
            f.elements[id].alive = false;
        }
        run(&mut f, 0.0, 5.0);
        assert!(
            (0..f.n[0]).all(|i| (1..f.n[1]).all(|k| !f.cell_closed(i, k, 3))),
            "every storey above fell"
        );
    }

    #[test]
    fn a_120mm_shell_holes_a_wall_and_leaves_the_columns() {
        let mut f = block();
        let wall = f
            .elements
            .iter()
            .position(|e| e.kind == Kind::Wall && e.storey == 5 && e.at == [2, 0])
            .unwrap();
        let at = f.elements[wall].centre();
        let broke = f.blast(at, 3.5, 4.0);
        assert!(broke >= 1 && !f.elements[wall].alive);
        assert!(
            f.elements.iter().filter(|e| e.kind == Kind::Column).all(|e| e.alive),
            "columns 3 m away survive"
        );
        let s = run(&mut f, 0.0, 3.0);
        assert!(s.fell.is_empty(), "a hole in a wall drops nothing");
        assert!(!f.cell_closed(2, 5, 0), "the cell behind the hole is open");
    }

    #[test]
    fn the_same_hits_break_the_same_elements() {
        let go = || {
            let mut f = block();
            for id in ground_columns(&f, |a, c| a <= 1 && c <= 2) {
                f.elements[id].alive = false;
            }
            run(&mut f, 0.0, 6.0);
            f.elements.iter().map(|e| e.alive).collect::<Vec<_>>()
        };
        assert_eq!(go(), go());
    }

    #[test]
    fn a_blast_with_no_energy_or_a_bad_centre_changes_nothing() {
        let mut f = block();
        assert_eq!(f.blast(Vec3::new(3.0, 2.0, 0.0), 3.0, 0.0), 0);
        assert_eq!(f.blast(Vec3::splat(f32::NAN), 3.0, 4.0), 0);
        assert_eq!(f.standing(), f.elements.len());
    }
}
