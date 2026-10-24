//! The ground: base hills from seeded value noise, shaped flat under cities and highways, stored
//! as a height grid. `height_at` is the one ground query: the terrain mesh, the pilot's feet and
//! the mech's root all read it (procgen-world design section 3).

use crate::rng::SimRng;
use glam::{Vec2, Vec3};

/// A square grid of ground heights, `n` samples a side, `cell_m` apart, centred on the origin.
#[derive(Clone, Debug, PartialEq)]
pub struct Heightfield {
    /// Samples along one side.
    pub n: usize,
    /// Spacing between samples.
    pub cell_m: f32,
    /// World x and z of sample (0, 0).
    pub origin_m: f32,
    /// Heights, row-major: index `iz * n + ix`.
    pub heights_m: Vec<f32>,
}

/// One chunk of the terrain mesh, in world metres.
#[derive(Clone, Debug, Default)]
pub struct ChunkMesh {
    /// Vertex positions.
    pub positions: Vec<Vec3>,
    /// Vertex normals.
    pub normals: Vec<Vec3>,
    /// Triangle list, counter-clockwise seen from above.
    pub indices: Vec<u32>,
}

impl Heightfield {
    /// The world position of sample `(ix, iz)`.
    pub fn sample_xz(&self, ix: usize, iz: usize) -> Vec2 {
        Vec2::new(self.origin_m + ix as f32 * self.cell_m, self.origin_m + iz as f32 * self.cell_m)
    }

    /// The stored height of sample `(ix, iz)`.
    pub fn sample(&self, ix: usize, iz: usize) -> f32 {
        self.heights_m[iz * self.n + ix]
    }

    /// Ground height at world `(x, z)`, on the same two triangles per cell the mesh draws: the cell
    /// is split along its (0,0)-(1,1) diagonal. Outside the grid, the edge is extended.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let max = (self.n - 1) as f32;
        let gx = ((x - self.origin_m) / self.cell_m).clamp(0.0, max);
        let gz = ((z - self.origin_m) / self.cell_m).clamp(0.0, max);
        if !gx.is_finite() || !gz.is_finite() {
            return 0.0;
        }
        let ix = (gx.floor() as usize).min(self.n - 2);
        let iz = (gz.floor() as usize).min(self.n - 2);
        let u = gx - ix as f32;
        let v = gz - iz as f32;
        let h00 = self.sample(ix, iz);
        let h10 = self.sample(ix + 1, iz);
        let h01 = self.sample(ix, iz + 1);
        let h11 = self.sample(ix + 1, iz + 1);
        if u >= v {
            h00 + u * (h10 - h00) + v * (h11 - h10)
        } else {
            h00 + v * (h01 - h00) + u * (h11 - h01)
        }
    }

    /// The mesh for the chunk of `cells` x `cells` cells starting at cell `(cx * cells, cz * cells)`.
    pub fn chunk_mesh(&self, cx: usize, cz: usize, cells: usize) -> ChunkMesh {
        let x0 = cx * cells;
        let z0 = cz * cells;
        let x1 = (x0 + cells).min(self.n - 1);
        let z1 = (z0 + cells).min(self.n - 1);
        let w = x1 - x0 + 1;
        let mut mesh = ChunkMesh::default();
        for iz in z0..=z1 {
            for ix in x0..=x1 {
                let p = self.sample_xz(ix, iz);
                mesh.positions.push(Vec3::new(p.x, self.sample(ix, iz), p.y));
                mesh.normals.push(self.normal(ix, iz));
            }
        }
        for iz in 0..(z1 - z0) {
            for ix in 0..(x1 - x0) {
                let a = (iz * w + ix) as u32;
                let b = a + 1;
                let c = a + w as u32;
                let d = c + 1;
                // Diagonal a-d, matching `height_at`. Counter-clockwise from above (+y).
                mesh.indices.extend_from_slice(&[a, d, b, a, c, d]);
            }
        }
        mesh
    }

    fn normal(&self, ix: usize, iz: usize) -> Vec3 {
        let l = self.sample(ix.saturating_sub(1), iz);
        let r = self.sample((ix + 1).min(self.n - 1), iz);
        let d = self.sample(ix, iz.saturating_sub(1));
        let u = self.sample(ix, (iz + 1).min(self.n - 1));
        Vec3::new(l - r, 2.0 * self.cell_m, d - u).normalize()
    }
}

/// Seeded value noise: a lattice of random heights, smoothly interpolated.
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
}

/// Hermite smoothstep of `x` between `edge0` and `edge1`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tilted() -> Heightfield {
        let n = 5;
        let mut heights_m = Vec::new();
        for iz in 0..n {
            for ix in 0..n {
                heights_m.push((ix * 3 + iz * iz) as f32);
            }
        }
        Heightfield {
            n,
            cell_m: 8.0,
            origin_m: -16.0,
            heights_m,
        }
    }

    #[test]
    fn height_at_matches_the_grid_at_every_sample() {
        let h = tilted();
        for iz in 0..h.n {
            for ix in 0..h.n {
                let p = h.sample_xz(ix, iz);
                assert_eq!(h.height_at(p.x, p.y), h.sample(ix, iz), "sample ({ix}, {iz})");
            }
        }
    }

    #[test]
    fn height_at_lies_on_the_drawn_triangles() {
        let h = tilted();
        let mesh = h.chunk_mesh(0, 0, 4);
        for tri in mesh.indices.chunks(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| mesh.positions[i as usize]);
            let centroid = (a + b + c) / 3.0;
            let got = h.height_at(centroid.x, centroid.z);
            assert!((got - centroid.y).abs() < 1e-4, "{got} vs {}", centroid.y);
            let normal = (b - a).cross(c - a);
            assert!(normal.y > 0.0, "triangles face up");
        }
    }
}
