//! The cloud layer (openspec changes `clouds`, `island-world`): cumulus clusters placed from the
//! seed between the cloud base and top over the island and the sea round it, each a few ellipsoids,
//! drifting with the wind. `density_at` is the one cloud query: the view fogs by it, and later radar
//! and sensors read it. Puffs are indexed by tile, so a query visits only the puffs near it and the
//! drawing can show only the tiles near the eye.

use super::config::CloudConfig;
use super::grid::CellIndex;
use crate::rng::SimRng;
use glam::{Vec2, Vec3};

/// One puff of a cluster: an ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Centre at time zero.
    pub centre: Vec3,
    /// Radii along x, y, z.
    pub radii: Vec3,
}

/// Side of a cloud tile: the unit the puffs are indexed and drawn in.
pub const CLOUD_TILE_M: f32 = 2048.0;

/// The sky's clouds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clouds {
    /// Every puff, cluster by cluster.
    pub puffs: Vec<Puff>,
    /// Wind drift (x, z), metres a second.
    pub wind_m_s: Vec2,
    /// Puffs by the tile their centre (at time zero) is in.
    by_tile: CellIndex,
    /// The tile grid's least corner and tiles a side.
    origin_m: f32,
    side: usize,
    /// The most any puff reaches from its centre across the ground.
    reach_m: f32,
}

impl Clouds {
    /// Cloud density at `p` at time `t_s`, 0 clear to 1 at a puff's heart.
    pub fn density_at(&self, p: Vec3, t_s: f32) -> f32 {
        let drift = Vec3::new(self.wind_m_s.x, 0.0, self.wind_m_s.y) * t_s;
        let q = p - drift;
        let at = Vec2::new(q.x, q.z);
        let mut d: f32 = 0.0;
        for i in self.by_tile.within(at - Vec2::splat(self.reach_m), at + Vec2::splat(self.reach_m)) {
            let puff = &self.puffs[i as usize];
            let k = (p - puff.centre - drift) / puff.radii;
            d = d.max(1.0 - k.length_squared());
        }
        d.clamp(0.0, 1.0)
    }

    /// The tile `(x, z)` holding a ground point (as placed at time zero).
    pub fn tile_of(&self, p: Vec2) -> (i32, i32) {
        (
            ((p.x - self.origin_m) / CLOUD_TILE_M).floor() as i32,
            ((p.y - self.origin_m) / CLOUD_TILE_M).floor() as i32,
        )
    }

    /// Every tile with puffs in it, in order, with its centre (x, z) at time zero.
    pub fn tiles(&self) -> Vec<((i32, i32), Vec2)> {
        let mut out = Vec::new();
        for z in 0..self.side as i32 {
            for x in 0..self.side as i32 {
                let c = Vec2::new(
                    self.origin_m + (x as f32 + 0.5) * CLOUD_TILE_M,
                    self.origin_m + (z as f32 + 0.5) * CLOUD_TILE_M,
                );
                if !self.by_tile.at(c).is_empty() {
                    out.push(((x, z), c));
                }
            }
        }
        out
    }

    /// The puffs whose centre lies in a tile, in order.
    pub fn puffs_in(&self, tile: (i32, i32)) -> Vec<u32> {
        if tile.0 < 0 || tile.1 < 0 || tile.0 as usize >= self.side || tile.1 as usize >= self.side {
            return Vec::new();
        }
        let c = Vec2::new(
            self.origin_m + (tile.0 as f32 + 0.5) * CLOUD_TILE_M,
            self.origin_m + (tile.1 as f32 + 0.5) * CLOUD_TILE_M,
        );
        self.by_tile.at(c).to_vec()
    }
}

/// Places the clusters on a jittered grid, more where the coverage noise is high.
pub fn make_clouds(rng: &SimRng, cfg: &CloudConfig, world_size_m: f32) -> Clouds {
    let reach = world_size_m * 0.5 + cfg.beyond_world_m;
    let n = (2.0 * reach / cfg.spacing_m) as i32;
    let mut puffs = Vec::new();
    for iz in 0..n {
        for ix in 0..n {
            let mut r = rng.fork(&format!("cluster/{ix}/{iz}"));
            if r.next_f32() > cfg.coverage {
                continue;
            }
            let c = Vec2::new(
                -reach + (ix as f32 + r.next_f32()) * cfg.spacing_m,
                -reach + (iz as f32 + r.next_f32()) * cfg.spacing_m,
            );
            let size = cfg.cluster_size_m[0] + (cfg.cluster_size_m[1] - cfg.cluster_size_m[0]) * r.next_f32();
            let thick = (cfg.top_m - cfg.base_m).min(size * 0.6);
            let count = 3 + (r.next_u64() % 6) as usize;
            for _ in 0..count {
                let off = Vec2::new(r.next_f32() - 0.5, r.next_f32() - 0.5) * size;
                let rx = size * (0.25 + 0.2 * r.next_f32());
                let ry = thick * (0.3 + 0.25 * r.next_f32());
                puffs.push(Puff {
                    // Flat bottoms at the cloud base.
                    centre: Vec3::new(c.x + off.x, cfg.base_m + ry, c.y + off.y),
                    radii: Vec3::new(rx, ry, rx * (0.8 + 0.4 * r.next_f32())),
                });
            }
        }
    }
    let reach_m = puffs.iter().map(|p| p.radii.x.max(p.radii.z)).fold(0.0, f32::max);
    let origin_m = -reach - cfg.spacing_m;
    let side = ((2.0 * (reach + cfg.spacing_m)) / CLOUD_TILE_M).ceil() as usize;
    let boxes: Vec<(u32, Vec2, Vec2)> = puffs
        .iter()
        .enumerate()
        .map(|(i, p)| (i as u32, Vec2::new(p.centre.x, p.centre.z), Vec2::new(p.centre.x, p.centre.z)))
        .collect();
    Clouds {
        by_tile: CellIndex::build(origin_m, CLOUD_TILE_M, side, &boxes),
        puffs,
        wind_m_s: Vec2::from_array(cfg.wind_m_s),
        origin_m,
        side,
        reach_m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clouds_sit_between_base_and_top() {
        let cfg = CloudConfig::default();
        let c = make_clouds(&SimRng::new(42), &cfg, 4096.0);
        assert!(!c.puffs.is_empty());
        for p in &c.puffs {
            assert!(p.centre.y - p.radii.y >= cfg.base_m - 1e-3);
            assert!(p.centre.y + p.radii.y <= cfg.top_m + 1e-3);
        }
    }

    #[test]
    fn a_puff_is_dense_at_its_heart_and_clear_far_away() {
        let c = make_clouds(&SimRng::new(42), &CloudConfig::default(), 4096.0);
        let p = c.puffs[0];
        assert!(c.density_at(p.centre, 0.0) > 0.99);
        assert_eq!(c.density_at(Vec3::new(0.0, 5.0, 0.0), 0.0), 0.0);
    }

    #[test]
    fn the_indexed_density_is_the_density_over_every_puff() {
        let c = make_clouds(&SimRng::new(9), &CloudConfig::default(), 8192.0);
        let every = |p: Vec3, t: f32| {
            let drift = Vec3::new(c.wind_m_s.x, 0.0, c.wind_m_s.y) * t;
            c.puffs
                .iter()
                .map(|q| 1.0 - ((p - q.centre - drift) / q.radii).length_squared())
                .fold(0.0f32, f32::max)
                .clamp(0.0, 1.0)
        };
        for k in 0..400 {
            let p = Vec3::new(
                -6000.0 + (k % 20) as f32 * 631.0,
                700.0 + (k % 7) as f32 * 50.0,
                -6000.0 + (k / 20) as f32 * 617.0,
            );
            let t = (k % 5) as f32 * 100.0;
            assert_eq!(c.density_at(p, t), every(p, t), "at {p}");
        }
    }

    #[test]
    fn every_puff_is_in_exactly_one_tile() {
        let c = make_clouds(&SimRng::new(4), &CloudConfig::default(), 8192.0);
        let mut seen: Vec<u32> = c.tiles().iter().flat_map(|(t, _)| c.puffs_in(*t)).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..c.puffs.len() as u32).collect::<Vec<_>>());
    }
}
