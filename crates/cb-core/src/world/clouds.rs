//! The cloud layer (openspec change `clouds`): cumulus clusters placed from the seed between the
//! cloud base and top, each a few ellipsoids, drifting with the wind. `density_at` is the one cloud
//! query: the view fogs by it, and later radar and sensors read it.

use super::config::CloudConfig;
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

/// The sky's clouds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clouds {
    /// Every puff, cluster by cluster.
    pub puffs: Vec<Puff>,
    /// Wind drift (x, z), metres a second.
    pub wind_m_s: Vec2,
}

impl Clouds {
    /// Cloud density at `p` at time `t_s`, 0 clear to 1 at a puff's heart.
    pub fn density_at(&self, p: Vec3, t_s: f32) -> f32 {
        let drift = Vec3::new(self.wind_m_s.x, 0.0, self.wind_m_s.y) * t_s;
        let mut d: f32 = 0.0;
        for puff in &self.puffs {
            let q = (p - puff.centre - drift) / puff.radii;
            d = d.max(1.0 - q.length_squared());
        }
        d.clamp(0.0, 1.0)
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
    Clouds {
        puffs,
        wind_m_s: Vec2::from_array(cfg.wind_m_s),
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
}
