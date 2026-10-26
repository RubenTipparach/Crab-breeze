//! The sky's clouds (openspec changes `clouds`, `island-world`, `cloud-weather`): every puff the
//! weather systems (`weather`) laid out, with its kind, darkness and layer, the rain shafts under
//! the storms, the ground they darken and the cirrus, all drifting with the wind. `density_at` and
//! `rain_at` are the one cloud queries: the view fogs by the first, the glass takes rain by both,
//! and later radar and sensors read them. Puffs are indexed by tile, so a query visits only the
//! puffs near it and the drawing can gather only the puffs near the eye.

use super::grid::CellIndex;
use super::terrain::{ValueNoise, smoothstep};
use super::weather::Sky;
use crate::rng::{SimRng, label_hash};
use glam::{Vec2, Vec3};

/// What kind of cloud a puff is part of: it sets how opaque it is and how its edge is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CloudKind {
    /// Fair-weather cumulus, humilis to congestus.
    Cumulus = 0,
    /// A storm cell's tower, or a tower in the front.
    Tower = 1,
    /// A storm's anvil.
    Anvil = 2,
    /// Layered cloud: the stratocumulus deck, the front's layers and the scud under a storm.
    Layer = 3,
    /// Cloud banked against the rim.
    Orographic = 4,
    /// A smooth lenticular cap over a peak.
    Cap = 5,
    /// Small flat altocumulus high up.
    Altocumulus = 6,
}

/// One puff: an ellipsoid, drawn as one billboard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Centre at time zero.
    pub centre: Vec3,
    /// Radii along x, y, z (x and z across, y up).
    pub radii: Vec3,
    /// The system it belongs to.
    pub kind: CloudKind,
    /// 0 white to 1 storm grey.
    pub dark: f32,
    /// How opaque it is drawn, 0..1.
    pub opacity: f32,
    /// The base and top of its system: it is lit darker toward the base.
    pub layer_m: [f32; 2],
    /// A per-puff number for its noise, 0..100.
    pub seed: f32,
}

/// Rain falling from a storm's base to the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RainShaft {
    /// Centre (x, z) at time zero.
    pub centre: Vec2,
    /// Radius.
    pub radius_m: f32,
    /// The shaft's top (the cloud base above it).
    pub top_m: f32,
}

/// Ground darkened under a storm: the sun is cut by `shade` out to 45 % of the radius, back to full
/// at the radius, the edge wandering by `wobble_m`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StormShade {
    /// Centre (x, z) at time zero.
    pub centre: Vec2,
    /// Radius.
    pub radius_m: f32,
    /// Share of the sun taken away at the centre, 0..1.
    pub shade: f32,
    /// How far its edge wanders in and out.
    pub wobble_m: f32,
}

/// A cirrus streak: a flat quad high up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cirrus {
    /// Centre at time zero.
    pub centre: Vec3,
    /// Length along `angle_rad`.
    pub length_m: f32,
    /// Width across it.
    pub width_m: f32,
    /// Its heading in the ground plane, radians from +x toward +z.
    pub angle_rad: f32,
    /// A per-streak number for its noise, 0..100.
    pub seed: f32,
}

/// Side of a cloud tile: the unit the puffs are indexed in.
pub const CLOUD_TILE_M: f32 = 2048.0;

/// The sky's clouds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clouds {
    /// Every puff, system by system.
    pub puffs: Vec<Puff>,
    /// Rain shafts.
    pub shafts: Vec<RainShaft>,
    /// The ground darkened under storms.
    pub shades: Vec<StormShade>,
    /// Cirrus streaks.
    pub cirrus: Vec<Cirrus>,
    /// Wind drift (x, z), metres a second.
    pub wind_m_s: Vec2,
    /// Rain on the glass inside a shaft, 0..1.
    pub shaft_rain: f32,
    /// Puffs by the tile their centre (at time zero) is in.
    by_tile: CellIndex,
    /// The most any puff reaches from its centre across the ground.
    reach_m: f32,
    /// The noise the storm shade's edge wanders by.
    edge: ValueNoise,
}

impl Clouds {
    /// Indexes a sky drifting with `wind_m_s`.
    pub fn new(sky: Sky, wind_m_s: Vec2, shaft_rain: f32) -> Self {
        let xz = |p: &Puff| Vec2::new(p.centre.x, p.centre.z);
        let (lo, hi) = sky
            .puffs
            .iter()
            .fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| {
                (lo.min(xz(p)), hi.max(xz(p)))
            });
        let (origin_m, side) = if sky.puffs.is_empty() {
            (0.0, 1)
        } else {
            let origin = lo.min_element() - CLOUD_TILE_M;
            (origin, ((hi.max_element() + CLOUD_TILE_M - origin) / CLOUD_TILE_M).ceil() as usize)
        };
        let boxes: Vec<(u32, Vec2, Vec2)> = sky.puffs.iter().enumerate().map(|(i, p)| (i as u32, xz(p), xz(p))).collect();
        Self {
            by_tile: CellIndex::build(origin_m, CLOUD_TILE_M, side, &boxes),
            reach_m: sky.puffs.iter().map(|p| p.radii.x.max(p.radii.z)).fold(0.0, f32::max),
            puffs: sky.puffs,
            shafts: sky.shafts,
            shades: sky.shades,
            cirrus: sky.cirrus,
            wind_m_s: if wind_m_s.is_finite() { wind_m_s } else { Vec2::ZERO },
            shaft_rain: shaft_rain.clamp(0.0, 1.0),
            edge: ValueNoise::new(&SimRng::new(label_hash("storm shade edge"))),
        }
    }

    /// Share of the sun reaching the cloud base's height over `p` (x, z) at `t_s`: 1 in the open,
    /// down to `1 - shade` under a storm (cloud-weather design section 2, storm darkening).
    pub fn sun_at(&self, p: Vec2, t_s: f32) -> f32 {
        let at = p - self.drift(t_s);
        self.shades.iter().fold(1.0f32, |sun, c| {
            let d = at.distance(c.centre) + c.wobble_m * self.edge.unit(at.x, at.y, 1500.0, 2, 0.5);
            sun.min(1.0 - c.shade * (1.0 - smoothstep(c.radius_m * 0.45, c.radius_m, d)))
        })
    }

    /// How far the wind has carried everything by `t_s` (x, z).
    pub fn drift(&self, t_s: f32) -> Vec2 {
        self.wind_m_s * t_s
    }

    /// Cloud density at `p` at time `t_s`, 0 clear to 1 at a puff's heart, over every kind.
    pub fn density_at(&self, p: Vec3, t_s: f32) -> f32 {
        let d = self.drift(t_s);
        let q = p - Vec3::new(d.x, 0.0, d.y);
        let at = Vec2::new(q.x, q.z);
        let mut out: f32 = 0.0;
        for i in self.by_tile.within(at - Vec2::splat(self.reach_m), at + Vec2::splat(self.reach_m)) {
            let puff = &self.puffs[i as usize];
            let k = (q - puff.centre) / puff.radii;
            out = out.max(1.0 - k.length_squared());
        }
        out.clamp(0.0, 1.0)
    }

    /// Rain at `p` at time `t_s`: `shaft_rain` inside a rain shaft's core (80 % of its radius)
    /// below its top, else 0.
    pub fn rain_at(&self, p: Vec3, t_s: f32) -> f32 {
        let at = Vec2::new(p.x, p.z) - self.drift(t_s);
        let inside = self
            .shafts
            .iter()
            .any(|s| p.y < s.top_m && at.distance(s.centre) < s.radius_m * 0.8);
        if inside { self.shaft_rain } else { 0.0 }
    }

    /// How wet the glass gets at `p`: inside a cloud by its density, under a storm by its rain.
    pub fn wetness_at(&self, p: Vec3, t_s: f32) -> f32 {
        self.density_at(p, t_s).max(self.rain_at(p, t_s))
    }

    /// The puffs whose centre (as drifted to `t_s`) may lie within `radius_m` of `p` (x, z), in
    /// index order. A superset by up to a tile: callers measure the distance themselves.
    pub fn puffs_near(&self, p: Vec2, radius_m: f32, t_s: f32) -> Vec<u32> {
        let at = p - self.drift(t_s);
        self.by_tile.within(at - Vec2::splat(radius_m), at + Vec2::splat(radius_m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn puff(centre: Vec3, radii: Vec3) -> Puff {
        Puff {
            centre,
            radii,
            kind: CloudKind::Cumulus,
            dark: 0.1,
            opacity: 0.95,
            layer_m: [600.0, 1100.0],
            seed: 1.0,
        }
    }

    /// A sky of scattered puffs and one shaft, for the index tests.
    fn scattered() -> Clouds {
        let mut puffs = Vec::new();
        for k in 0..600u32 {
            let x = -15000.0 + (k % 30) as f32 * 1013.0;
            let z = -15000.0 + (k / 30) as f32 * 1531.0;
            let r = 150.0 + (k % 7) as f32 * 120.0;
            puffs.push(puff(Vec3::new(x, 700.0 + (k % 5) as f32 * 60.0, z), Vec3::new(r, r * 0.6, r)));
        }
        let shafts = vec![RainShaft {
            centre: Vec2::new(2000.0, -3000.0),
            radius_m: 1800.0,
            top_m: 720.0,
        }];
        Clouds::new(
            Sky {
                puffs,
                shafts,
                ..Sky::default()
            },
            Vec2::new(6.0, 0.0),
            0.7,
        )
    }

    #[test]
    fn a_puff_is_dense_at_its_heart_and_clear_far_away() {
        let c = scattered();
        let p = c.puffs[0];
        assert!(c.density_at(p.centre, 0.0) > 0.99);
        assert_eq!(c.density_at(Vec3::new(0.0, 5.0, 0.0), 0.0), 0.0);
    }

    #[test]
    fn the_indexed_density_is_the_density_over_every_puff() {
        let c = scattered();
        let every = |p: Vec3, t: f32| {
            let d = c.drift(t);
            c.puffs
                .iter()
                .map(|q| 1.0 - ((p - Vec3::new(d.x, 0.0, d.y) - q.centre) / q.radii).length_squared())
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
    fn it_rains_inside_a_shaft_below_its_top_and_not_beside_it() {
        let c = scattered();
        let under = Vec3::new(2000.0, 200.0, -3000.0);
        assert_eq!(c.rain_at(under, 0.0), 0.7);
        assert_eq!(c.rain_at(under + Vec3::Y * 800.0, 0.0), 0.0, "above the base");
        assert_eq!(c.rain_at(under + Vec3::X * 2000.0, 0.0), 0.0, "beside the shaft");
        // The shaft drifts with the wind: 100 s later it is 600 m east.
        assert_eq!(c.rain_at(under + Vec3::X * 2000.0, 100.0), 0.7);
        assert_eq!(c.wetness_at(under, 0.0), 0.7);
    }

    #[test]
    fn puffs_near_finds_every_puff_within_the_radius() {
        let c = scattered();
        let here = Vec2::new(1200.0, -800.0);
        for t in [0.0, 250.0] {
            let near = c.puffs_near(here, 5000.0, t);
            let d = c.drift(t);
            for (i, p) in c.puffs.iter().enumerate() {
                if (Vec2::new(p.centre.x, p.centre.z) + d).distance(here) < 5000.0 {
                    assert!(near.contains(&(i as u32)), "puff {i} at t {t}");
                }
            }
        }
    }

    #[test]
    fn the_sun_is_darkened_under_a_storm_and_full_in_the_open() {
        let c = Clouds::new(
            Sky {
                shades: vec![StormShade {
                    centre: Vec2::new(-5000.0, 0.0),
                    radius_m: 4000.0,
                    shade: 0.6,
                    wobble_m: 600.0,
                }],
                ..Sky::default()
            },
            Vec2::new(6.0, 0.0),
            0.7,
        );
        assert!(
            (c.sun_at(Vec2::new(-5000.0, 0.0), 0.0) - 0.4).abs() < 1e-5,
            "60 % taken at the centre"
        );
        assert_eq!(c.sun_at(Vec2::new(5000.0, 0.0), 0.0), 1.0, "full sun far from it");
        // It drifts with the wind: 1,000 s later the centre is 6 km east.
        assert!((c.sun_at(Vec2::new(1000.0, 0.0), 1000.0) - 0.4).abs() < 1e-5);
        for k in 0..100 {
            let s = c.sun_at(Vec2::new(-9000.0 + k as f32 * 90.0, 300.0), 0.0);
            assert!((0.4 - 1e-5..=1.0).contains(&s), "{s}");
        }
    }

    #[test]
    fn an_empty_sky_is_clear() {
        let c = Clouds::new(Sky::default(), Vec2::ZERO, 0.7);
        assert_eq!(c.density_at(Vec3::new(0.0, 800.0, 0.0), 0.0), 0.0);
        assert!(c.puffs_near(Vec2::ZERO, 9000.0, 0.0).is_empty());
    }
}
