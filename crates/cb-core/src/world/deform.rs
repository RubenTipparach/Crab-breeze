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
        super::terrain::cell_height([s(i, j), s(i + 1, j), s(i, j + 1), s(i + 1, j + 1)], u, v)
    }

    /// Digs a crater for `blast`. Returns the tiles it touched (to be remeshed).
    pub fn dig(&mut self, plan: &WorldPlan, blast: Blast, cfg: &CraterConfig) -> Vec<(i32, i32)> {
        if !(blast.energy_mj.is_finite() && blast.energy_mj > 0.0 && blast.centre.is_finite()) {
            return Vec::new();
        }
        self.blasts.push(blast);
        let r = cfg.radius_m_per_cbrt_mj * blast.energy_mj.cbrt();
        let reach = cfg.reach_m(blast.energy_mj);
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

    /// Raises a heap of `volume_m3` over `lo..hi` (x, z) spread `spread_m` further, then lets it
    /// slump until no sample stands more than `repose_slope` per metre above a neighbour, capped
    /// at `max_m` above where the ground was. Unlike a crater it raises city ground too (rubble lies
    /// in the streets). Returns the tiles it touched.
    pub fn pile(&mut self, lo: Vec2, hi: Vec2, volume_m3: f32, spread_m: f32, repose_slope: f32, max_m: f32) -> Vec<(i32, i32)> {
        if !(volume_m3.is_finite() && volume_m3 > 0.0 && lo.is_finite() && hi.is_finite() && spread_m >= 0.0) {
            return Vec::new();
        }
        // Work on a window of global samples covering the heap and the room it can slump into.
        let room = spread_m + max_m / repose_slope.max(0.05);
        let g0 = ((lo - Vec2::splat(room) - Vec2::splat(self.origin_m)) / STEP_M).floor();
        let g1 = ((hi + Vec2::splat(room) - Vec2::splat(self.origin_m)) / STEP_M).ceil();
        let (gx0, gz0) = (g0.x as i64, g0.y as i64);
        let (w, h) = ((g1.x - g0.x) as usize + 1, (g1.y - g0.y) as usize + 1);
        let mut win = vec![0.0f32; w * h];
        let tile_n = (TILE_N - 1) as i64;
        let read = |s: &Self, gx: i64, gz: i64| -> f32 {
            let key = (gx.div_euclid(tile_n) as i32, gz.div_euclid(tile_n) as i32);
            s.tiles
                .get(&key)
                .map_or(0.0, |t| t[gz.rem_euclid(tile_n) as usize * TILE_N + gx.rem_euclid(tile_n) as usize])
        };
        for j in 0..h {
            for i in 0..w {
                win[j * w + i] = read(self, gx0 + i as i64, gz0 + j as i64);
            }
        }
        let base = win.clone();
        // The heap's shape: flat-topped over the footprint grown by `spread_m`, falling away at the
        // repose slope, capped at `max_m`. Its top height is found by bisection so the shape holds
        // `volume_m3`; built in its final shape, it needs only a few slump passes to settle onto
        // any heap already there.
        let (a, bnd) = (lo - Vec2::splat(spread_m), hi + Vec2::splat(spread_m));
        let at = |i: usize, j: usize| {
            Vec2::new(
                self.origin_m + (gx0 + i as i64) as f32 * STEP_M,
                self.origin_m + (gz0 + j as i64) as f32 * STEP_M,
            )
        };
        let outside: Vec<f32> = (0..w * h)
            .map(|k| {
                let p = at(k % w, k / w);
                (a - p).max(p - bnd).max(Vec2::ZERO).length()
            })
            .collect();
        let shape = |top: f32, k: usize| (top - repose_slope * outside[k]).clamp(0.0, max_m);
        let held = |top: f32| outside.iter().enumerate().map(|(k, _)| shape(top, k)).sum::<f32>() * STEP_M * STEP_M;
        let (mut t0, mut t1) = (0.0f32, max_m + repose_slope * room);
        for _ in 0..24 {
            let mid = 0.5 * (t0 + t1);
            if held(mid) < volume_m3 {
                t0 = mid;
            } else {
                t1 = mid;
            }
        }
        for (k, v) in win.iter_mut().enumerate() {
            *v = (*v + shape(t1, k)).min(base[k] + max_m);
        }
        // Slump where the new heap meets an old one.
        let lim = repose_slope * STEP_M;
        for _ in 0..24 {
            let mut moved = false;
            for j in 1..h - 1 {
                for i in 1..w - 1 {
                    let k = j * w + i;
                    for n in [k + 1, k - 1, k + w, k - w] {
                        let d = win[k] - win[n] - lim;
                        if d > 0.002 {
                            win[k] -= d * 0.25;
                            win[n] += d * 0.25;
                            moved = true;
                        }
                    }
                }
            }
            if !moved {
                break;
            }
        }
        // Write back to every tile holding each changed sample (tile edges are shared).
        let mut touched = Vec::new();
        for j in 0..h {
            for i in 0..w {
                let k = j * w + i;
                if (win[k] - base[k]).abs() < 1e-4 {
                    continue;
                }
                let (gx, gz) = (gx0 + i as i64, gz0 + j as i64);
                let (tx, tz) = (gx.div_euclid(tile_n), gz.div_euclid(tile_n));
                let (lx, lz) = (gx.rem_euclid(tile_n) as usize, gz.rem_euclid(tile_n) as usize);
                let mut homes = vec![((tx as i32, tz as i32), lx, lz)];
                if lx == 0 {
                    homes.push((((tx - 1) as i32, tz as i32), TILE_N - 1, lz));
                }
                if lz == 0 {
                    homes.push(((tx as i32, (tz - 1) as i32), lx, TILE_N - 1));
                }
                if lx == 0 && lz == 0 {
                    homes.push((((tx - 1) as i32, (tz - 1) as i32), TILE_N - 1, TILE_N - 1));
                }
                for (key, li, lj) in homes {
                    let t = self.tiles.entry(key).or_insert_with(|| vec![0.0; TILE_N * TILE_N]);
                    t[lj * TILE_N + li] = win[k];
                    self.scorch.entry(key).or_insert_with(|| vec![0.0; TILE_N * TILE_N]);
                    if !touched.contains(&key) {
                        touched.push(key);
                    }
                }
            }
        }
        touched.sort_unstable();
        touched
    }

    /// The height change at a tile sample (rubble above zero, craters below).
    pub fn delta_sample(&self, key: (i32, i32), i: usize, j: usize) -> f32 {
        self.tiles.get(&key).map_or(0.0, |t| t[j * TILE_N + i])
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
        let c = h.point_at(0.37);
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
    fn rubble_raises_the_ground_under_it() {
        let mut d = Deformation::new(4096.0);
        let touched = d.pile(Vec2::new(-10.0, -10.0), Vec2::new(10.0, 10.0), 2000.0, 4.0, 0.75, 18.0);
        assert!(!touched.is_empty());
        assert!(d.delta_at(Vec2::ZERO) > 1.0, "the heap stands over its footprint");
        assert!(d.delta_at(Vec2::new(60.0, 0.0)).abs() < 1e-4, "and nowhere far off");
    }

    #[test]
    fn a_heap_never_stands_steeper_than_its_angle_of_repose() {
        let mut d = Deformation::new(4096.0);
        d.pile(Vec2::new(-2.0, -2.0), Vec2::new(2.0, 2.0), 3000.0, 0.0, 0.75, 18.0);
        let mut worst = 0.0f32;
        for i in -60..60 {
            for j in -60..60 {
                let p = Vec2::new(i as f32 * STEP_M, j as f32 * STEP_M);
                let h = d.delta_at(p);
                for q in [p + Vec2::X * STEP_M, p + Vec2::Y * STEP_M] {
                    worst = worst.max((h - d.delta_at(q)).abs() / STEP_M);
                }
            }
        }
        assert!(worst <= 0.75 + 0.02, "steepest {worst}");
        assert!(d.delta_at(Vec2::ZERO) <= 18.0 + 1e-3);
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
