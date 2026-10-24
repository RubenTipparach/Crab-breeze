//! Craters (openspec change `terrain-deformation`): a sparse layer of fine height deltas over the
//! terrain, dug by blasts in the order they happen. Tiles are 64 m (eight terrain cells) with a
//! 0.5 m grid, aligned with the terrain grid, so the base mesh can leave a hole for each tile and the
//! tile's finer mesh meets it edge to edge. Roads, bridges and city ground are not dug.

use super::WorldPlan;
use super::config::CraterConfig;
use super::terrain::ChunkMesh;
use glam::{Vec2, Vec3};
use std::collections::HashMap;

/// Side of a deformation tile.
pub const TILE_M: f32 = 64.0;
/// Samples along a tile's side (0.5 m apart).
pub const TILE_N: usize = 129;
const STEP_M: f32 = TILE_M / (TILE_N - 1) as f32;

/// A blast that dug the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    /// Centre (x, z).
    pub centre: Vec2,
    /// Energy, megajoules.
    pub energy_mj: f32,
}

/// The dug ground: tile deltas and the blasts that made them, in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Deformation {
    tiles: HashMap<(i32, i32), Vec<f32>>,
    /// Scorch per tile sample, 0..1 (drawn darker).
    scorch: HashMap<(i32, i32), Vec<f32>>,
    /// Every blast, in order.
    pub blasts: Vec<Blast>,
    origin_m: f32,
}

impl Deformation {
    /// An undug world `world_size_m` across.
    pub fn new(world_size_m: f32) -> Self {
        Self {
            origin_m: -world_size_m * 0.5,
            ..Default::default()
        }
    }

    /// The tile holding `p`.
    pub fn tile_of(&self, p: Vec2) -> (i32, i32) {
        (
            ((p.x - self.origin_m) / TILE_M).floor() as i32,
            ((p.y - self.origin_m) / TILE_M).floor() as i32,
        )
    }

    /// World (x, z) of a tile's sample.
    pub fn sample_xz(&self, key: (i32, i32), i: usize, j: usize) -> Vec2 {
        Vec2::new(
            self.origin_m + key.0 as f32 * TILE_M + i as f32 * STEP_M,
            self.origin_m + key.1 as f32 * TILE_M + j as f32 * STEP_M,
        )
    }

    /// True when a tile has been dug or scorched.
    pub fn has_tile(&self, key: (i32, i32)) -> bool {
        self.tiles.contains_key(&key)
    }

    /// Every touched tile, in key order.
    pub fn tile_keys(&self) -> Vec<(i32, i32)> {
        let mut k: Vec<_> = self.tiles.keys().copied().collect();
        k.sort_unstable();
        k
    }

    /// The height change at `p`.
    pub fn delta_at(&self, p: Vec2) -> f32 {
        let key = self.tile_of(p);
        let Some(t) = self.tiles.get(&key) else { return 0.0 };
        let o = self.sample_xz(key, 0, 0);
        let gx = ((p.x - o.x) / STEP_M).clamp(0.0, (TILE_N - 1) as f32);
        let gz = ((p.y - o.y) / STEP_M).clamp(0.0, (TILE_N - 1) as f32);
        let (i, j) = ((gx as usize).min(TILE_N - 2), (gz as usize).min(TILE_N - 2));
        let (u, v) = (gx - i as f32, gz - j as f32);
        let s = |a: usize, b: usize| t[b * TILE_N + a];
        // The same two triangles per cell as the tile mesh.
        if u >= v {
            s(i, j) + u * (s(i + 1, j) - s(i, j)) + v * (s(i + 1, j + 1) - s(i + 1, j))
        } else {
            s(i, j) + v * (s(i, j + 1) - s(i, j)) + u * (s(i + 1, j + 1) - s(i, j + 1))
        }
    }

    /// Digs a crater for `blast`. Returns the tiles it touched (to be remeshed).
    pub fn dig(&mut self, plan: &WorldPlan, blast: Blast, cfg: &CraterConfig) -> Vec<(i32, i32)> {
        if !(blast.energy_mj.is_finite() && blast.energy_mj > 0.0 && blast.centre.is_finite()) {
            return Vec::new();
        }
        self.blasts.push(blast);
        let r = cfg.radius_m_per_cbrt_mj * blast.energy_mj.cbrt();
        let reach = r * 1.8;
        let (k0, k1) = (
            self.tile_of(blast.centre - Vec2::splat(reach)),
            self.tile_of(blast.centre + Vec2::splat(reach)),
        );
        let mut touched = Vec::new();
        let protected = |p: Vec2| {
            plan.cities.iter().any(|c| c.rect().distance(p) < 14.0)
                || plan.highways.iter().any(|h| h.project(p).0 < h.width_m * 0.5 + 3.0)
                || plan.waters.bridges.iter().any(|b| b.centre.distance(p) < b.length_m * 0.6)
        };
        for kz in k0.1..=k1.1 {
            for kx in k0.0..=k1.0 {
                let key = (kx, kz);
                touched.push(key);
                let mut delta = self.tiles.remove(&key).unwrap_or_else(|| vec![0.0; TILE_N * TILE_N]);
                let mut scorch = self.scorch.remove(&key).unwrap_or_else(|| vec![0.0; TILE_N * TILE_N]);
                for j in 0..TILE_N {
                    for i in 0..TILE_N {
                        let p = self.sample_xz(key, i, j);
                        let d = p.distance(blast.centre) / r;
                        if d >= 1.8 {
                            continue;
                        }
                        let k = j * TILE_N + i;
                        scorch[k] = scorch[k].max((1.0 - d / 1.8).clamp(0.0, 1.0));
                        if protected(p) {
                            continue;
                        }
                        let bowl = if d < 1.0 {
                            -cfg.depth_ratio * r * 0.5 * (1.0 + (std::f32::consts::PI * d).cos())
                        } else {
                            0.0
                        };
                        let rim = cfg.rim_ratio * r * (-((d - 1.0) / 0.35).powi(2)).exp();
                        delta[k] = (delta[k] + bowl + rim).max(-cfg.max_dig_m);
                    }
                }
                self.tiles.insert(key, delta);
                self.scorch.insert(key, scorch);
            }
        }
        touched
    }

    /// Scorch at a tile sample, 0..1.
    pub fn scorch_at(&self, key: (i32, i32), i: usize, j: usize) -> f32 {
        self.scorch.get(&key).map_or(0.0, |s| s[j * TILE_N + i])
    }

    /// Only a tile and its eight neighbours (what `tile_mesh` and `scorch_at` read for that tile),
    /// to hand to a worker thread without copying every dug tile.
    pub fn around(&self, key: (i32, i32)) -> Self {
        let mut out = Self {
            origin_m: self.origin_m,
            ..Default::default()
        };
        for dz in -1..=1 {
            for dx in -1..=1 {
                let k = (key.0 + dx, key.1 + dz);
                if let Some(t) = self.tiles.get(&k) {
                    out.tiles.insert(k, t.clone());
                }
                if let Some(t) = self.scorch.get(&k) {
                    out.scorch.insert(k, t.clone());
                }
            }
        }
        out
    }

    /// The fine mesh of a touched tile, on the base ground plus the deltas.
    pub fn tile_mesh(&self, plan: &WorldPlan, key: (i32, i32)) -> ChunkMesh {
        // Every grid point's height once, with a one-point border for the normals: the terrain's
        // height is costly (noise), and sampling it five times a vertex took 56-260 ms a crater.
        let g = TILE_N + 2;
        let tile = self.tiles.get(&key);
        let at = |ii: usize, jj: usize| {
            Vec2::new(
                self.origin_m + key.0 as f32 * TILE_M + (ii as f32 - 1.0) * STEP_M,
                self.origin_m + key.1 as f32 * TILE_M + (jj as f32 - 1.0) * STEP_M,
            )
        };
        let mut h = vec![0.0f32; g * g];
        for jj in 0..g {
            for ii in 0..g {
                let p = at(ii, jj);
                let inside = (1..=TILE_N).contains(&ii) && (1..=TILE_N).contains(&jj);
                // Inside, the delta exactly at the grid point (no interpolation error at edges).
                let d = match (inside, tile) {
                    (true, Some(t)) => t[(jj - 1) * TILE_N + (ii - 1)],
                    (true, None) => 0.0,
                    (false, _) => self.delta_at(p),
                };
                h[jj * g + ii] = plan.terrain.height_at(p.x, p.y) + d;
            }
        }
        let mut m = ChunkMesh::default();
        for j in 0..TILE_N {
            for i in 0..TILE_N {
                let (ii, jj) = (i + 1, j + 1);
                let p = at(ii, jj);
                m.positions.push(Vec3::new(p.x, h[jj * g + ii], p.y));
                let n = Vec3::new(
                    h[jj * g + ii - 1] - h[jj * g + ii + 1],
                    2.0 * STEP_M,
                    h[(jj - 1) * g + ii] - h[(jj + 1) * g + ii],
                )
                .normalize();
                m.normals.push(n);
            }
        }
        for j in 0..TILE_N - 1 {
            for i in 0..TILE_N - 1 {
                let a = (j * TILE_N + i) as u32;
                let (b, c) = (a + 1, a + TILE_N as u32);
                let d = c + 1;
                m.indices.extend_from_slice(&[a, d, b, a, c, d]);
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;
    use crate::world::{Ground, WorldConfig, generate};
    use std::sync::OnceLock;

    fn plan() -> &'static WorldPlan {
        static P: OnceLock<WorldPlan> = OnceLock::new();
        P.get_or_init(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
            generate(42, &load_ron::<WorldConfig>(&p).unwrap())
        })
    }

    /// An open spot: no city, road, water or bridge within 60 m.
    fn open_ground() -> Vec2 {
        let p = plan();
        (0..400)
            .map(|k| Vec2::new(-1800.0 + (k % 20) as f32 * 180.0, -1800.0 + (k / 20) as f32 * 180.0))
            .find(|&c| {
                p.cities.iter().all(|x| x.rect().distance(c) > 60.0)
                    && p.highways.iter().all(|h| h.project(c).0 > 60.0)
                    && p.water_at(c.x, c.y).is_none()
                    && p.waters.rivers.iter().all(|r| r.path.iter().all(|q| q.distance(c) > 120.0))
            })
            .unwrap()
    }

    #[test]
    fn a_crater_digs_a_bowl_and_raises_a_rim() {
        let cfg = CraterConfig::default();
        let c = open_ground();
        let mut d = Deformation::new(4096.0);
        d.dig(plan(), Blast { centre: c, energy_mj: 4.0 }, &cfg);
        let r = cfg.radius_m_per_cbrt_mj * 4f32.cbrt();
        assert!((d.delta_at(c) + cfg.depth_ratio * r).abs() < 0.05, "centre {}", d.delta_at(c));
        assert!(d.delta_at(c + Vec2::X * r) > 0.0, "rim");
        assert_eq!(d.delta_at(c + Vec2::X * 3.0 * r), 0.0, "untouched beyond");
    }

    #[test]
    fn craters_never_dig_below_the_floor() {
        let cfg = CraterConfig::default();
        let c = open_ground();
        let mut d = Deformation::new(4096.0);
        for _ in 0..20 {
            d.dig(
                plan(),
                Blast {
                    centre: c,
                    energy_mj: 30.0,
                },
                &cfg,
            );
        }
        assert!(d.delta_at(c) >= -cfg.max_dig_m - 1e-4);
    }

    #[test]
    fn a_road_is_scorched_not_dug() {
        let h = &plan().highways[0];
        let c = h.ends[0].lerp(h.ends[1], 0.37);
        let mut d = Deformation::new(4096.0);
        d.dig(plan(), Blast { centre: c, energy_mj: 4.0 }, &CraterConfig::default());
        assert_eq!(d.delta_at(c), 0.0);
        let key = d.tile_of(c);
        assert!(d.has_tile(key));
    }

    #[test]
    fn the_same_explosions_give_the_same_ground() {
        let c = open_ground();
        let run = || {
            let mut d = Deformation::new(4096.0);
            for k in 0..5 {
                d.dig(
                    plan(),
                    Blast {
                        centre: c + Vec2::X * k as f32 * 3.0,
                        energy_mj: 2.0 + k as f32,
                    },
                    &CraterConfig::default(),
                );
            }
            d
        };
        assert!(run() == run());
    }

    #[test]
    fn a_tile_mesh_meets_the_base_ground_at_its_edges() {
        let c = open_ground();
        let mut d = Deformation::new(4096.0);
        let keys = d.dig(plan(), Blast { centre: c, energy_mj: 4.0 }, &CraterConfig::default());
        for key in keys {
            let m = d.tile_mesh(plan(), key);
            for (k, p) in m.positions.iter().enumerate() {
                let (i, j) = (k % TILE_N, k / TILE_N);
                let edge = i == 0 || j == 0 || i == TILE_N - 1 || j == TILE_N - 1;
                if edge && !d.has_tile(d.tile_of(Vec2::new(p.x, p.z) + Vec2::splat(-0.01))) {
                    continue;
                }
                if edge {
                    let base = plan().terrain.height_at(p.x, p.z) + d.delta_at(Vec2::new(p.x, p.z));
                    assert!((p.y - base).abs() < 1e-3);
                }
            }
        }
    }

    #[test]
    fn a_tile_meshes_the_same_from_its_neighbourhood_alone() {
        let mut d = Deformation::new(4096.0);
        let cfg = CraterConfig::default();
        let c = open_ground();
        // Two craters near each other (one may spill into a neighbour) and one far off.
        d.dig(
            plan(),
            Blast {
                centre: c,
                energy_mj: 25.0,
            },
            &cfg,
        );
        d.dig(
            plan(),
            Blast {
                centre: c + Vec2::new(40.0, 0.0),
                energy_mj: 25.0,
            },
            &cfg,
        );
        d.dig(
            plan(),
            Blast {
                centre: c + Vec2::new(900.0, 0.0),
                energy_mj: 25.0,
            },
            &cfg,
        );
        let key = d.tile_of(c);
        let (a, b) = (d.tile_mesh(plan(), key), d.around(key).tile_mesh(plan(), key));
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.normals, b.normals);
    }
}
