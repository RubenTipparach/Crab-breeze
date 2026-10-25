//! Levels of detail of the drawn ground (openspec change `island-world`, design section 1): which
//! step each terrain chunk is drawn at for an eye, with hysteresis, how deep its skirt hangs, and
//! what that costs in triangles. Pure, so the drawing (cb-app) and the tests share one rule.

use super::config::LodConfig;
use super::layout::Rect;
use super::terrain::Heightfield;
use glam::{Vec2, Vec3};

/// The step a chunk `distance_m` from the eye is drawn at: 1 inside the first ring, doubling ring
/// by ring, `max_step` past the last.
pub fn step_for(distance_m: f32, cfg: &LodConfig) -> usize {
    let ring = cfg.ring_m.iter().position(|r| distance_m < *r).unwrap_or(cfg.ring_m.len());
    1 << ring
}

/// The step a chunk drawn at `current` should be drawn at now, `distance_m` from the eye: it moves
/// to a coarser step only once it is `hysteresis_m` past that ring's inner edge, and to a finer one
/// only once it is `hysteresis_m` inside, so a chunk on a ring's edge does not flicker.
pub fn next_step(current: usize, distance_m: f32, cfg: &LodConfig) -> usize {
    let coarser = step_for(distance_m - cfg.hysteresis_m, cfg);
    let finer = step_for(distance_m + cfg.hysteresis_m, cfg);
    if coarser > current {
        coarser
    } else if finer < current {
        finer
    } else {
        current
    }
}

/// Distance from the eye to a chunk: across the ground to its nearest point, and up to the eye's
/// height above `ground_m`.
pub fn eye_distance(rect: &Rect, eye: Vec3, ground_m: f32) -> f32 {
    let across = rect.distance(Vec2::new(eye.x, eye.z));
    let up = (eye.y - ground_m).max(0.0);
    across.hypot(up)
}

/// The ground chunk `(cx, cz)` of `cells` cells a side covers.
pub fn chunk_rect(t: &Heightfield, cells: usize, cx: usize, cz: usize) -> Rect {
    let side = cells as f32 * t.cell_m;
    let min = Vec2::splat(t.origin_m) + Vec2::new(cx as f32, cz as f32) * side;
    Rect {
        min,
        max: min + Vec2::splat(side),
    }
}

/// How deep a chunk's skirt hangs at `step`: deeper than the most a coarser neighbour's edge can
/// sag below or bulge above this one's (a cell of the next step up), plus a margin.
pub fn skirt_m(step: usize, cell_m: f32) -> f32 {
    4.0 + 2.0 * step as f32 * cell_m
}

/// Triangles in a chunk of `cells` drawn at `step`, its skirt included.
pub fn chunk_triangles(cells: usize, step: usize) -> usize {
    let k = cells / step.max(1);
    2 * k * k + 4 * 2 * k
}

/// A node of the ground's quadtree (owner, 2026-10-24: "make sure to lower draw calls"): level 0
/// is one chunk drawn at step 1; a node of level `L` covers `2^L` chunks a side and is drawn as
/// one mesh at step `2^L`, so every node costs the same triangles and the far island is a handful
/// of meshes. Its `(x, z)` count nodes of its own size from the world's corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Node {
    /// 0 for a chunk; each level up doubles the side.
    pub level: u32,
    /// Column, in nodes of this level.
    pub x: u32,
    /// Row, in nodes of this level.
    pub z: u32,
}

impl Node {
    /// The step this node is drawn at.
    pub fn step(&self) -> usize {
        1 << self.level
    }

    /// Terrain cells along this node's side, for chunks of `cells`.
    pub fn cells(&self, cells: usize) -> usize {
        cells << self.level
    }

    /// The ground this node covers.
    pub fn rect(&self, t: &Heightfield, cells: usize) -> Rect {
        chunk_rect(t, self.cells(cells), self.x as usize, self.z as usize)
    }

    /// Its four children (none for a chunk).
    pub fn children(&self) -> Vec<Node> {
        if self.level == 0 {
            return Vec::new();
        }
        let (l, x, z) = (self.level - 1, self.x * 2, self.z * 2);
        vec![
            Node { level: l, x, z },
            Node { level: l, x: x + 1, z },
            Node { level: l, x, z: z + 1 },
            Node {
                level: l,
                x: x + 1,
                z: z + 1,
            },
        ]
    }

    /// True when `other` is this node or lies inside it.
    pub fn contains(&self, other: &Node) -> bool {
        other.level <= self.level && (other.x >> (self.level - other.level)) == self.x && (other.z >> (self.level - other.level)) == self.z
    }

    /// True when this node holds chunk `(cx, cz)`.
    pub fn holds_chunk(&self, cx: usize, cz: usize) -> bool {
        (cx >> self.level) == self.x as usize && (cz >> self.level) == self.z as usize
    }
}

/// The nodes to draw for an eye, in order, and the nodes split to reach them. Walks down from the
/// roots (level `cfg.max_level`, covering the world's `chunks` a side), splitting a node while its
/// distance wants a finer step than its own (by `next_step`, so a node split last time, in
/// `split_before`, stays split inside the hysteresis band) or while it holds a chunk in `fine`
/// (crater chunks, drawn at full detail).
pub fn select(
    t: &Heightfield,
    cells: usize,
    chunks: usize,
    eye: Vec3,
    ground_m: f32,
    cfg: &LodConfig,
    split_before: &std::collections::BTreeSet<Node>,
    fine: &std::collections::BTreeSet<(usize, usize)>,
) -> (Vec<Node>, std::collections::BTreeSet<Node>) {
    let top = cfg.max_level;
    let roots = chunks >> top;
    let mut stack: Vec<Node> = Vec::new();
    for z in (0..roots as u32).rev() {
        for x in (0..roots as u32).rev() {
            stack.push(Node { level: top, x, z });
        }
    }
    let mut leaves = Vec::new();
    let mut split = std::collections::BTreeSet::new();
    while let Some(n) = stack.pop() {
        let d = eye_distance(&n.rect(t, cells), eye, ground_m);
        let drawn_step = if split_before.contains(&n) { n.step() / 2 } else { n.step() };
        let too_near = n.level > 0 && next_step(drawn_step.max(1), d, cfg) < n.step();
        let holds_fine = n.level > 0 && fine.iter().any(|&(cx, cz)| n.holds_chunk(cx, cz));
        if too_near || holds_fine {
            split.insert(n);
            for c in n.children().into_iter().rev() {
                stack.push(c);
            }
        } else {
            leaves.push(n);
        }
    }
    (leaves, split)
}

/// What the ground costs to draw for an eye with nothing split before: `(meshes, triangles)`.
pub fn draws(t: &Heightfield, cells: usize, chunks: usize, eye: Vec3, ground_m: f32, cfg: &LodConfig) -> (usize, usize) {
    let (leaves, _) = select(t, cells, chunks, eye, ground_m, cfg, &Default::default(), &Default::default());
    let tris = leaves.iter().map(|n| chunk_triangles(n.cells(cells), n.step())).sum();
    (leaves.len(), tris)
}

/// How many chunks are drawn at each step, and their triangles, for an eye: one row per step,
/// finest first, `(step, chunks, triangles)`. The chunks of a world `chunks` a side.
pub fn census(t: &Heightfield, cells: usize, chunks: usize, eye: Vec3, ground_m: f32, cfg: &LodConfig) -> Vec<(usize, usize, usize)> {
    let mut rows: Vec<(usize, usize, usize)> = (0..=cfg.ring_m.len()).map(|r| (1 << r, 0, 0)).collect();
    for cz in 0..chunks {
        for cx in 0..chunks {
            let s = step_for(eye_distance(&chunk_rect(t, cells, cx, cz), eye, ground_m), cfg);
            let row = &mut rows[s.trailing_zeros() as usize];
            row.1 += 1;
            row.2 += chunk_triangles(cells, s);
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> LodConfig {
        LodConfig::default()
    }

    #[test]
    fn a_chunk_farther_out_is_drawn_coarser() {
        let c = cfg();
        let mut last = 0;
        for d in (0..30_000).step_by(50) {
            let s = step_for(d as f32, &c);
            assert!(s >= last, "step fell from {last} to {s} at {d} m");
            last = s;
        }
        assert_eq!(step_for(0.0, &c), 1);
        assert_eq!(step_for(c.ring_m[0] + 1.0, &c), 2);
        assert_eq!(step_for(1e9, &c), c.max_step());
    }

    #[test]
    fn a_chunk_on_a_ring_edge_does_not_flicker() {
        let c = cfg();
        let edge = c.ring_m[1];
        let mut s = step_for(edge - 1.0, &c);
        let start = s;
        for k in 0..100 {
            let wobble = if k % 2 == 0 { 1.0 } else { -1.0 } * (c.hysteresis_m * 0.9);
            s = next_step(s, edge + wobble, &c);
            assert_eq!(s, start, "changed step on a wobble of {wobble} m");
        }
        assert_eq!(
            next_step(s, edge + c.hysteresis_m + 1.0, &c),
            start * 2,
            "past the band it coarsens"
        );
        assert_eq!(
            next_step(start * 2, edge - c.hysteresis_m - 1.0, &c),
            start,
            "and back inside it refines"
        );
    }

    #[test]
    fn a_jump_moves_straight_to_the_right_step() {
        let c = cfg();
        assert_eq!(next_step(1, 1e6, &c), c.max_step());
        assert_eq!(next_step(c.max_step(), 0.0, &c), 1);
    }

    #[test]
    fn the_eye_high_above_sees_its_own_chunk_coarser() {
        let r = Rect {
            min: Vec2::ZERO,
            max: Vec2::splat(512.0),
        };
        assert_eq!(eye_distance(&r, Vec3::new(10.0, 2.0, 10.0), 0.0), 2.0);
        assert_eq!(eye_distance(&r, Vec3::new(10.0, 3000.0, 10.0), 0.0), 3000.0);
    }

    /// An 8 km ground of 16 x 16 chunks: one root at level 4.
    fn ground() -> Heightfield {
        let cfg = crate::world::WorldConfig {
            world_size_m: 8192.0,
            ..Default::default()
        };
        let shape = crate::world::terrain::Shape::new(3, &cfg);
        Heightfield::new(&cfg, shape, crate::world::terrain::Flats::default())
    }

    #[test]
    fn the_drawn_nodes_cover_the_ground_once() {
        let t = ground();
        let eye = Vec3::new(-3000.0, 2.0, 1200.0);
        let (leaves, _) = select(&t, 64, 16, eye, 0.0, &cfg(), &Default::default(), &Default::default());
        let mut covered = vec![0; 16 * 16];
        for n in &leaves {
            for cz in 0..16 {
                for cx in 0..16 {
                    if n.holds_chunk(cx, cz) {
                        covered[cz * 16 + cx] += 1;
                    }
                }
            }
        }
        assert!(covered.iter().all(|c| *c == 1), "every chunk under exactly one drawn node");
        assert!(leaves.len() < 60, "{} meshes for 256 chunks", leaves.len());
    }

    #[test]
    fn nodes_near_the_eye_are_fine_and_far_ones_coarse() {
        let t = ground();
        let eye = Vec3::new(-3900.0, 2.0, -3900.0);
        let (leaves, _) = select(&t, 64, 16, eye, 0.0, &cfg(), &Default::default(), &Default::default());
        let under = leaves.iter().find(|n| n.holds_chunk(0, 0)).unwrap();
        let far = leaves.iter().find(|n| n.holds_chunk(15, 15)).unwrap();
        assert_eq!(under.level, 0);
        assert!(far.level >= 2, "the far corner is drawn at level {}", far.level);
        for n in &leaves {
            let d = eye_distance(&n.rect(&t, 64), eye, 0.0);
            assert!(n.step() <= step_for(d, &cfg()), "a node is never coarser than its distance allows");
        }
    }

    #[test]
    fn a_crater_chunk_is_drawn_at_full_detail_however_far() {
        let t = ground();
        let fine = std::collections::BTreeSet::from([(15usize, 15usize)]);
        let (leaves, _) = select(
            &t,
            64,
            16,
            Vec3::new(-3900.0, 2.0, -3900.0),
            0.0,
            &cfg(),
            &Default::default(),
            &fine,
        );
        assert!(leaves.contains(&Node { level: 0, x: 15, z: 15 }));
    }

    #[test]
    fn a_split_node_on_a_ring_edge_stays_split() {
        let t = ground();
        let c = cfg();
        let eye = Vec3::new(-4096.0 + 256.0 + c.ring_m[0] + 600.0, 2.0, -3800.0);
        let (first, split) = select(&t, 64, 16, eye, 0.0, &c, &Default::default(), &Default::default());
        let wobble = eye + Vec3::X * (c.hysteresis_m * 0.5);
        let (again, _) = select(&t, 64, 16, wobble, 0.0, &c, &split, &Default::default());
        let back = eye - Vec3::X * (c.hysteresis_m * 0.5);
        let (third, _) = select(&t, 64, 16, back, 0.0, &c, &split, &Default::default());
        assert_eq!(first, again);
        assert_eq!(first, third);
    }

    #[test]
    fn a_skirt_covers_the_most_a_coarser_neighbour_can_differ() {
        assert!(skirt_m(1, 8.0) >= 2.0 * 8.0);
        assert!(skirt_m(16, 8.0) > skirt_m(8, 8.0));
        assert_eq!(chunk_triangles(64, 1), 2 * 64 * 64 + 8 * 64);
        assert_eq!(chunk_triangles(64, 32), 2 * 4 + 8 * 2);
    }
}
