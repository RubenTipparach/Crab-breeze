//! The ground: the island of `island-biomes` (the island mockup's land, `island::Island`, from the
//! seed: a range on a curved spine, the desert plateau with its canyons, beaches, sea cliffs and an
//! archipelago), shaped flat under cities and highways and carved by rivers and lakes.
//! `height_at` is the one ground query:
//! the drawn chunks at every level of detail, the colliders, the pilot's feet and the mech's root
//! all read it (procgen-world design section 3).
//!
//! The ground is a grid of samples `cell_m` apart, but the grid is not stored: a sample is a pure
//! function of its position, worked out when asked for. A 40 km world at 8 m would be 26 million
//! stored heights (105 MB); asking costs a few microseconds a sample instead (island-world design
//! section 2).

use super::config::WorldConfig;
use super::island::{Fields, Island};
use super::layout::{City, Highway, Rect};
use super::water::Channels;
use crate::rng::SimRng;
use glam::{Vec2, Vec3};
use std::sync::Arc;

/// The island's shape: the mockup's land under the game's sea, its sea floor rising or falling to
/// the flat floor drawn past the world's edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    island: Arc<Island>,
    half_world_m: f32,
    sea_level_m: f32,
    sea_depth_m: f32,
    edge_fade_m: f32,
    snowline_m: f32,
}

impl Shape {
    /// The shape for the world of `seed` (the mockup's seed: the game's seed 42 is its island).
    pub fn new(seed: u64, cfg: &WorldConfig) -> Self {
        Self {
            island: Arc::new(Island::new(seed as u32)),
            half_world_m: cfg.world_size_m * 0.5,
            sea_level_m: cfg.island.sea_level_m,
            sea_depth_m: cfg.island.sea_depth_m,
            edge_fade_m: cfg.island.edge_fade_m,
            snowline_m: cfg.island.snowline_m,
        }
    }

    /// The island the land comes from.
    pub fn island(&self) -> &Island {
        &self.island
    }

    /// Height of the snowline.
    pub fn snowline_m(&self) -> f32 {
        self.snowline_m
    }

    /// Signed distance past the coast at `p`: negative inland, positive out to sea (islets count
    /// as sea: nothing is built on them).
    pub fn past_land_edge_m(&self, p: Vec2) -> f32 {
        -(self.island.coast_dist(f64::from(p.x), f64::from(p.y)) as f32)
    }

    /// Ground height at `p` before cities, roads and rivers, and what it came from.
    pub fn height_and_fields(&self, p: Vec2) -> (f32, Fields) {
        let (h, f) = self.island.height(p);
        let edge = p.x.abs().max(p.y.abs());
        let fade = smoothstep(self.half_world_m - self.edge_fade_m, self.half_world_m, edge);
        let h = if h < 0.0 { h + (-self.sea_depth_m - h) * fade } else { h };
        (self.sea_level_m + h, f)
    }

    /// Ground height at `p` before cities, roads and rivers.
    pub fn height(&self, p: Vec2) -> f32 {
        self.height_and_fields(p).0
    }
}

/// Flat ground under cities and graded ground under highways, looked up by an index so a sample
/// only visits the few that reach it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flats {
    cities: Vec<(Rect, f32)>,
    highways: Vec<Highway>,
    plateau_margin_m: f32,
    plateau_blend_m: f32,
    road_flat_m: f32,
    road_blend_m: f32,
    /// City ids, then highway ids offset by the city count: the order they are applied in.
    index: super::grid::CellIndex,
}

/// Cell size of the flats' index.
const FLATS_CELL_M: f32 = 512.0;

impl Flats {
    /// The flats of a world's cities and highways.
    pub fn new(cities: &[City], highways: &[Highway], cfg: &WorldConfig) -> Self {
        let plateau_reach = cfg.plateau_flat_margin_m + cfg.plateau_blend_m;
        let road_flat_m = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
        let road_reach = road_flat_m + cfg.highway_blend_m;
        let mut boxes = Vec::new();
        for (i, c) in cities.iter().enumerate() {
            let r = c.rect();
            boxes.push((i as u32, r.min - Vec2::splat(plateau_reach), r.max + Vec2::splat(plateau_reach)));
        }
        for (k, h) in highways.iter().enumerate() {
            let lo = h.path.iter().fold(Vec2::splat(f32::MAX), |m, p| m.min(*p));
            let hi = h.path.iter().fold(Vec2::splat(f32::MIN), |m, p| m.max(*p));
            boxes.push((
                (cities.len() + k) as u32,
                lo - Vec2::splat(road_reach),
                hi + Vec2::splat(road_reach),
            ));
        }
        let half = cfg.world_size_m * 0.5;
        let side = (cfg.world_size_m / FLATS_CELL_M).ceil() as usize;
        Self {
            cities: cities.iter().map(|c| (c.rect(), c.ground_m)).collect(),
            highways: highways.to_vec(),
            plateau_margin_m: cfg.plateau_flat_margin_m,
            plateau_blend_m: cfg.plateau_blend_m,
            road_flat_m,
            road_blend_m: cfg.highway_blend_m,
            index: super::grid::CellIndex::build(-half, FLATS_CELL_M, side, &boxes),
        }
    }

    /// `h` at `p` blended toward each highway's grade, then each city's ground: a city's square
    /// stays flat at its ground even where a routed road runs close by.
    pub fn apply(&self, p: Vec2, h: f32) -> f32 {
        let mut h = h;
        let ids = self.index.at(p);
        for &id in ids.iter().filter(|&&id| id as usize >= self.cities.len()) {
            let hw = &self.highways[id as usize - self.cities.len()];
            let (d, t) = hw.project(p);
            let w = 1.0 - smoothstep(0.0, self.road_blend_m, d - self.road_flat_m);
            if w > 0.0 {
                h += (hw.grade_at(t) - h) * w;
            }
        }
        for &id in ids.iter().filter(|&&id| (id as usize) < self.cities.len()) {
            let (rect, ground_m) = &self.cities[id as usize];
            let d = rect.distance(p) - self.plateau_margin_m;
            let w = 1.0 - smoothstep(0.0, self.plateau_blend_m, d);
            h += (ground_m - h) * w;
        }
        h
    }
}

/// The ground of a world. Sample `(ix, iz)` sits at `origin_m + (ix, iz) * cell_m`; `n` samples a
/// side span the world, but the ground goes on past its edge (the open sea floor).
#[derive(Clone, Debug, PartialEq)]
pub struct Heightfield {
    /// Samples along one side of the world.
    pub n: usize,
    /// Spacing between samples.
    pub cell_m: f32,
    /// World x and z of sample (0, 0).
    pub origin_m: f32,
    shape: Shape,
    flats: Flats,
    channels: Option<Channels>,
}

/// The height inside one lattice cell at `(u, v)` (0..1 across it, x then z) from its corners
/// `[h00, h10, h01, h11]`, on the two triangles every ground mesh draws: the cell split along its
/// (0,0)-(1,1) diagonal. The one copy of the rule on the CPU: the terrain, the dug ground and the
/// grass tiles read it (the grass shader, `assets/shaders/grass.wesl`, repeats it on the GPU).
/// Where `u >= v` the `h01` corner is not read, and otherwise `h10` is not.
pub fn cell_height(h: [f32; 4], u: f32, v: f32) -> f32 {
    let [h00, h10, h01, h11] = h;
    if u >= v {
        h00 + u * (h10 - h00) + v * (h11 - h10)
    } else {
        h00 + v * (h01 - h00) + u * (h11 - h01)
    }
}

/// One chunk of the terrain mesh, in world metres.
#[derive(Clone, Debug, Default)]
pub struct ChunkMesh {
    /// Vertex positions.
    pub positions: Vec<Vec3>,
    /// Vertex normals.
    pub normals: Vec<Vec3>,
    /// Triangle list, counter-clockwise seen from above (skirts: seen from outside the chunk).
    pub indices: Vec<u32>,
}

impl Heightfield {
    /// The ground of a world: its shape and flats, with no rivers carved yet.
    pub fn new(cfg: &WorldConfig, shape: Shape, flats: Flats) -> Self {
        Self {
            n: cfg.samples_per_side(),
            cell_m: cfg.terrain_cell_m,
            origin_m: -cfg.world_size_m * 0.5,
            shape,
            flats,
            channels: None,
        }
    }

    /// The same ground with river channels carved into it.
    pub fn with_channels(mut self, channels: Channels) -> Self {
        self.channels = Some(channels);
        self
    }

    /// The island's shape.
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// The world position of sample `(ix, iz)`.
    pub fn sample_xz(&self, ix: usize, iz: usize) -> Vec2 {
        self.lattice_xz(ix as i64, iz as i64)
    }

    fn lattice_xz(&self, ix: i64, iz: i64) -> Vec2 {
        Vec2::new(self.origin_m + ix as f32 * self.cell_m, self.origin_m + iz as f32 * self.cell_m)
    }

    /// The height of sample `(ix, iz)`.
    pub fn sample(&self, ix: usize, iz: usize) -> f32 {
        self.lattice(ix as i64, iz as i64)
    }

    /// The height of any lattice point, inside the world or past its edge.
    fn lattice(&self, ix: i64, iz: i64) -> f32 {
        let p = self.lattice_xz(ix, iz);
        let h = self.flats.apply(p, self.shape.height(p));
        match &self.channels {
            Some(c) => c.carve(p, h),
            None => h,
        }
    }

    /// Ground height at world `(x, z)`, on the same two triangles per cell the full-detail mesh
    /// draws: the cell is split along its (0,0)-(1,1) diagonal. Past the world's edge the ground
    /// goes on (the open sea floor). A non-finite position reads zero.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let gx = (x - self.origin_m) / self.cell_m;
        let gz = (z - self.origin_m) / self.cell_m;
        if !gx.is_finite() || !gz.is_finite() {
            return 0.0;
        }
        let (fx, fz) = (gx.floor(), gz.floor());
        let (ix, iz) = (fx as i64, fz as i64);
        let u = gx - fx;
        let v = gz - fz;
        let h00 = self.lattice(ix, iz);
        let h11 = self.lattice(ix + 1, iz + 1);
        // Only the corner on the point's side of the diagonal is read.
        let (h10, h01) = if u >= v {
            (self.lattice(ix + 1, iz), 0.0)
        } else {
            (0.0, self.lattice(ix, iz + 1))
        };
        cell_height([h00, h10, h01, h11], u, v)
    }

    /// The full-detail mesh for the chunk of `cells` x `cells` cells starting at cell
    /// `(cx * cells, cz * cells)`.
    pub fn chunk_mesh(&self, cx: usize, cz: usize, cells: usize) -> ChunkMesh {
        self.chunk_mesh_lod(cx, cz, cells, 1, 0.0, |_, _| false)
    }

    /// The mesh for a chunk drawn with every `step`-th sample (`cells` must be a multiple of it),
    /// with a skirt `skirt_m` deep round its edge to hide the cracks where it meets a chunk drawn at
    /// another step (zero for none). At step 1, the cells `skip(ix, iz)` names are left out (where
    /// a finer crater tile is drawn); a coarser step draws every cell.
    pub fn chunk_mesh_lod(
        &self,
        cx: usize,
        cz: usize,
        cells: usize,
        step: usize,
        skirt_m: f32,
        skip: impl Fn(usize, usize) -> bool,
    ) -> ChunkMesh {
        let step = step.max(1);
        let x0 = cx * cells;
        let z0 = cz * cells;
        let x1 = (x0 + cells).min(self.n - 1);
        let z1 = (z0 + cells).min(self.n - 1);
        let w = (x1 - x0) / step + 1;
        let d = (z1 - z0) / step + 1;
        // Heights with a one-sample border, for normals by central difference at this step.
        let bw = w + 2;
        let mut grid = Vec::with_capacity(bw * (d + 2));
        for j in 0..d + 2 {
            for i in 0..bw {
                let ix = x0 as i64 + (i as i64 - 1) * step as i64;
                let iz = z0 as i64 + (j as i64 - 1) * step as i64;
                grid.push(self.lattice(ix, iz));
            }
        }
        let at = |i: usize, j: usize| grid[(j + 1) * bw + i + 1];
        let span = 2.0 * step as f32 * self.cell_m;
        let mut mesh = ChunkMesh::default();
        mesh.positions.reserve(w * d + 4 * (w.max(d)) * 2);
        for j in 0..d {
            for i in 0..w {
                let p = self.sample_xz(x0 + i * step, z0 + j * step);
                mesh.positions.push(Vec3::new(p.x, at(i, j), p.y));
                let (l, r) = (grid[(j + 1) * bw + i], grid[(j + 1) * bw + i + 2]);
                let (dn, up) = (grid[j * bw + i + 1], grid[(j + 2) * bw + i + 1]);
                mesh.normals.push(Vec3::new(l - r, span, dn - up).normalize());
            }
        }
        for j in 0..d - 1 {
            for i in 0..w - 1 {
                if step == 1 && skip(x0 + i, z0 + j) {
                    continue;
                }
                let a = (j * w + i) as u32;
                let b = a + 1;
                let c = a + w as u32;
                let e = c + 1;
                // Diagonal a-e, matching `height_at`. Counter-clockwise from above (+y).
                mesh.indices.extend_from_slice(&[a, e, b, a, c, e]);
            }
        }
        if skirt_m > 0.0 {
            let north: Vec<u32> = (0..w).map(|i| i as u32).collect();
            let south: Vec<u32> = (0..w).map(|i| ((d - 1) * w + i) as u32).collect();
            let west: Vec<u32> = (0..d).map(|j| (j * w) as u32).collect();
            let east: Vec<u32> = (0..d).map(|j| (j * w + w - 1) as u32).collect();
            for (edge, out) in [(north, -Vec3::Z), (south, Vec3::Z), (west, -Vec3::X), (east, Vec3::X)] {
                add_skirt(&mut mesh, &edge, out, skirt_m);
            }
        }
        mesh
    }
}

/// Hangs a skirt `depth_m` deep under the edge vertices `edge`, facing `out`.
fn add_skirt(mesh: &mut ChunkMesh, edge: &[u32], out: Vec3, depth_m: f32) {
    let first = mesh.positions.len() as u32;
    for &v in edge {
        let p = mesh.positions[v as usize];
        let n = mesh.normals[v as usize];
        mesh.positions.push(p - Vec3::Y * depth_m);
        mesh.normals.push(n);
    }
    for k in 0..edge.len().saturating_sub(1) {
        let (t0, t1) = (edge[k], edge[k + 1]);
        let (b0, b1) = (first + k as u32, first + k as u32 + 1);
        let (p0, p1, q0) = (
            mesh.positions[t0 as usize],
            mesh.positions[t1 as usize],
            mesh.positions[b0 as usize],
        );
        // Wind each quad so its front (counter-clockwise) faces out of the chunk.
        if (p1 - p0).cross(q0 - p0).dot(out) > 0.0 {
            mesh.indices.extend_from_slice(&[t0, t1, b0, t1, b1, b0]);
        } else {
            mesh.indices.extend_from_slice(&[t0, b0, t1, t1, b0, b1]);
        }
    }
}

/// Seeded value noise: a lattice of random heights, smoothly interpolated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueNoise {
    seed: u64,
}

impl ValueNoise {
    /// Noise drawn from a stream.
    pub fn new(rng: &SimRng) -> Self {
        Self {
            seed: rng.clone().next_u64(),
        }
    }

    fn lattice(&self, ix: i64, iz: i64, octave: u32) -> f32 {
        let mut r = SimRng::new(
            self.seed
                ^ (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (iz as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ u64::from(octave).wrapping_mul(0x1656_67B1_9E37_79F9),
        );
        r.next_f32() * 2.0 - 1.0
    }

    /// One octave at lattice coordinates, in -1..1.
    fn octave(&self, x: f32, z: f32, octave: u32) -> f32 {
        let fx = x.floor();
        let fz = z.floor();
        let (ix, iz) = (fx as i64, fz as i64);
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let (u, v) = (s(x - fx), s(z - fz));
        let a = self.lattice(ix, iz, octave);
        let b = self.lattice(ix + 1, iz, octave);
        let c = self.lattice(ix, iz + 1, octave);
        let d = self.lattice(ix + 1, iz + 1, octave);
        let top = a + (b - a) * u;
        let bottom = c + (d - c) * u;
        top + (bottom - top) * v
    }

    /// Fractal noise at world `(x, z)`, in metres.
    pub fn height(&self, x: f32, z: f32, wavelength_m: f32, amplitude_m: f32, octaves: u32, persistence: f32) -> f32 {
        let mut sum = 0.0;
        let mut freq = 1.0 / wavelength_m;
        let mut amp = amplitude_m;
        for o in 0..octaves {
            sum += self.octave(x * freq, z * freq, o) * amp;
            freq *= 2.0;
            amp *= persistence;
        }
        sum
    }

    /// Ridged fractal noise at world `(x, z)`, 0..1: each octave folded about zero into a crest and
    /// weighted by the one before (Musgrave), so detail gathers on the ridges and the valleys stay
    /// smooth.
    pub fn ridged(&self, x: f32, z: f32, wavelength_m: f32, octaves: u32) -> f32 {
        let (mut sum, mut norm, mut amp, mut weight) = (0.0, 0.0, 0.5, 1.0);
        let mut freq = 1.0 / wavelength_m;
        for o in 0..octaves {
            let mut n = 1.0 - self.octave(x * freq, z * freq, o).abs();
            n = n * n * weight;
            weight = (n * 2.0).clamp(0.0, 1.0);
            sum += n * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm.max(1e-6)
    }

    /// Fractal noise scaled to -1..1 (the sum of the octaves' amplitudes is one).
    pub fn unit(&self, x: f32, z: f32, wavelength_m: f32, octaves: u32, persistence: f32) -> f32 {
        let total: f32 = (0..octaves).map(|o| persistence.powi(o as i32)).sum();
        self.height(x, z, wavelength_m, 1.0, octaves, persistence) / total.max(1e-6)
    }
}

/// Hermite smoothstep of `x` between `edge0` and `edge1`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small world: the island's middle, no cities, no rivers.
    fn hills() -> Heightfield {
        let cfg = WorldConfig {
            world_size_m: 512.0,
            ..Default::default()
        };
        Heightfield::new(&cfg, Shape::new(7, &cfg), Flats::default())
    }

    #[test]
    fn height_at_matches_the_grid_at_every_sample() {
        let h = hills();
        for iz in (0..h.n).step_by(5) {
            for ix in (0..h.n).step_by(7) {
                let p = h.sample_xz(ix, iz);
                assert_eq!(h.height_at(p.x, p.y), h.sample(ix, iz), "sample ({ix}, {iz})");
            }
        }
    }

    #[test]
    fn height_at_lies_on_the_drawn_triangles() {
        let h = hills();
        let mesh = h.chunk_mesh(0, 0, 16);
        for tri in mesh.indices.chunks(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| mesh.positions[i as usize]);
            let centroid = (a + b + c) / 3.0;
            let got = h.height_at(centroid.x, centroid.z);
            assert!((got - centroid.y).abs() < 1e-3, "{got} vs {}", centroid.y);
            let normal = (b - a).cross(c - a);
            assert!(normal.y > 0.0, "triangles face up");
        }
    }

    #[test]
    fn a_coarse_chunk_draws_every_step_th_sample() {
        let h = hills();
        let mesh = h.chunk_mesh_lod(0, 0, 32, 4, 0.0, |_, _| false);
        assert_eq!(mesh.positions.len(), 9 * 9);
        assert_eq!(mesh.indices.len(), 8 * 8 * 6);
        let p = mesh.positions[9 + 1];
        assert_eq!(p.y, h.sample(4, 4));
    }

    #[test]
    fn a_skirt_hangs_below_every_edge_and_faces_out() {
        let h = hills();
        let cells = 16;
        let plain = h.chunk_mesh_lod(0, 0, cells, 2, 0.0, |_, _| false);
        let skirted = h.chunk_mesh_lod(0, 0, cells, 2, 10.0, |_, _| false);
        let edge = cells / 2;
        assert_eq!(
            skirted.indices.len() - plain.indices.len(),
            4 * edge * 6,
            "two triangles per edge segment"
        );
        let centre = plain.positions.iter().fold(Vec3::ZERO, |a, p| a + *p) / plain.positions.len() as f32;
        for tri in skirted.indices[plain.indices.len()..].chunks(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| skirted.positions[i as usize]);
            let n = (b - a).cross(c - a);
            let mid = (a + b + c) / 3.0 - centre;
            assert!(n.x * mid.x + n.z * mid.z > 0.0, "a skirt triangle faces out of the chunk");
        }
        let lowest = skirted.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let lowest_top = plain.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        assert!((lowest - (lowest_top - 10.0)).abs() < 1e-3);
    }

    #[test]
    fn the_ground_goes_on_past_the_worlds_edge() {
        let h = hills();
        let far = h.height_at(5000.0, -7000.0);
        assert!(far.is_finite());
        assert_ne!(far, h.height_at(256.0, -256.0), "not the edge sample stretched out");
    }
}
