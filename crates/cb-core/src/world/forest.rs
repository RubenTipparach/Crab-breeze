//! Trees (openspec change `vegetation`): seeded woods on open ground and in park blocks, and the
//! trees' state as the fight goes on. A tree hit with more than its break impulse snaps at the base
//! and falls as a rigid rod pivoting on its stump (`theta'' = 3 g sin(theta) / 2 L`), then lies
//! where it fell. Unbroken trunks block like walls. Pure and deterministic.

use super::config::TreeConfig;
use super::{Ground, WorldPlan};
use crate::rng::SimRng;
use glam::{Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

const G_M_S2: f32 = 9.81;
const BUCKET_M: f32 = 32.0;

/// A tree as it was grown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tree {
    /// Stable id, in growth order.
    pub id: u32,
    /// Index into the species list in data.
    pub species: u8,
    /// Base of the trunk, on the ground.
    pub base: Vec3,
    /// Height to the top.
    pub height_m: f32,
    /// Trunk radius at the base.
    pub trunk_radius_m: f32,
    /// Momentum needed to snap it.
    pub break_impulse_n_s: f32,
    /// Turn about its own axis (looks only).
    pub yaw_rad: f32,
}

impl Tree {
    /// Mass of the trunk as a cylinder of green wood (600 kg/m3).
    pub fn mass_kg(&self) -> f32 {
        600.0 * std::f32::consts::PI * self.trunk_radius_m.powi(2) * self.height_m
    }
}

/// What a tree is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TreeState {
    /// Upright.
    Standing,
    /// Toppling toward `dir` (x, z unit), `angle_rad` from upright.
    Falling {
        /// Direction of the fall.
        dir: Vec2,
        /// Angle from upright.
        angle_rad: f32,
        /// Angular speed.
        omega_rad_s: f32,
    },
    /// Lying toward `dir`.
    Fallen {
        /// Direction it fell.
        dir: Vec2,
    },
}

impl TreeState {
    /// Fall direction and angle from upright (zero when standing).
    pub fn tilt(&self) -> (Vec2, f32) {
        match *self {
            TreeState::Standing => (Vec2::Y, 0.0),
            TreeState::Falling { dir, angle_rad, .. } => (dir, angle_rad),
            TreeState::Fallen { dir } => (dir, FRAC_PI_2),
        }
    }
}

/// Every tree and its state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Forest {
    /// Trees, by id.
    pub trees: Vec<Tree>,
    /// State of each tree, by id.
    pub states: Vec<TreeState>,
    buckets: Vec<Vec<u32>>,
    origin_m: f32,
    side: usize,
}

/// What a body bumping into trees did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bump {
    /// Where the body ends up (x, z).
    pub pos: Vec2,
    /// Trees it snapped.
    pub broke: Vec<u32>,
    /// True when an unbroken trunk stopped it.
    pub blocked: bool,
}

impl Forest {
    fn bucket(&self, p: Vec2) -> (usize, usize) {
        let f = |v: f32| (((v - self.origin_m) / BUCKET_M).floor().max(0.0) as usize).min(self.side - 1);
        (f(p.x), f(p.y))
    }

    /// Ids of trees whose trunk might be within `r` of `p`, ascending.
    pub fn near(&self, p: Vec2, r: f32) -> Vec<u32> {
        if self.buckets.is_empty() {
            return Vec::new();
        }
        let (x0, z0) = self.bucket(p - Vec2::splat(r + 2.0));
        let (x1, z1) = self.bucket(p + Vec2::splat(r + 2.0));
        let mut out = Vec::new();
        for z in z0..=z1 {
            for x in x0..=x1 {
                out.extend_from_slice(&self.buckets[z * self.side + x]);
            }
        }
        out.sort_unstable();
        out
    }

    /// Snaps a standing tree if `impulse_n_s` exceeds its break impulse, starting its fall toward
    /// `dir`. Returns true if it snapped.
    pub fn hit(&mut self, id: u32, dir: Vec2, impulse_n_s: f32) -> bool {
        let t = self.trees[id as usize];
        // A non-finite impulse never snaps a tree.
        if self.states[id as usize] != TreeState::Standing || !impulse_n_s.is_finite() || impulse_n_s < t.break_impulse_n_s {
            return false;
        }
        // The hit lands a third of the way up: omega0 = J (L/3) / (m L^2 / 3) = J / (m L).
        let excess = impulse_n_s - t.break_impulse_n_s;
        self.states[id as usize] = TreeState::Falling {
            dir: dir.normalize_or(Vec2::X),
            angle_rad: 0.02,
            omega_rad_s: (excess / (t.mass_kg() * t.height_m)).min(2.0),
        };
        true
    }

    /// A body of `radius_m` at `p` moving at `velocity` (x, z) with `mass_kg`: snaps the trees its
    /// momentum breaks and is stopped by the ones it does not.
    pub fn bump(&mut self, p: Vec2, radius_m: f32, velocity: Vec2, mass_kg: f32) -> Bump {
        let mut out = Bump {
            pos: p,
            ..Default::default()
        };
        for id in self.near(p, radius_m) {
            let t = self.trees[id as usize];
            if self.states[id as usize] != TreeState::Standing {
                continue;
            }
            let c = Vec2::new(t.base.x, t.base.z);
            let reach = radius_m + t.trunk_radius_m;
            let d = out.pos - c;
            if d.length() >= reach {
                continue;
            }
            let normal = d.normalize_or(Vec2::X);
            let closing = (-velocity.dot(normal)).max(0.0);
            if self.hit(id, -normal, mass_kg * closing) {
                out.broke.push(id);
            } else {
                out.pos = c + normal * reach;
                out.blocked = true;
            }
        }
        out
    }

    /// A trunk-only push for a body that cannot break trees (the pilot on foot).
    pub fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        let mut p = p;
        for id in self.near(p, radius_m) {
            if self.states[id as usize] != TreeState::Standing {
                continue;
            }
            let t = &self.trees[id as usize];
            let c = Vec2::new(t.base.x, t.base.z);
            let reach = radius_m + t.trunk_radius_m;
            let d = p - c;
            if d.length() < reach {
                p = c + d.normalize_or(Vec2::X) * reach;
            }
        }
        p
    }

    /// A blast at `centre` (x, z): snaps trees within `radius_m`, harder near the centre, away from it.
    pub fn blast(&mut self, centre: Vec2, radius_m: f32, impulse_n_s: f32) -> Vec<u32> {
        let mut broke = Vec::new();
        for id in self.near(centre, radius_m) {
            let t = self.trees[id as usize];
            let c = Vec2::new(t.base.x, t.base.z);
            let d = c.distance(centre);
            if d < radius_m && self.hit(id, c - centre, impulse_n_s * (1.0 - d / radius_m)) {
                broke.push(id);
            }
        }
        broke
    }

    /// Advances falling trees by `dt_s`. Returns the ids that hit the ground this step.
    pub fn step(&mut self, dt_s: f32) -> Vec<u32> {
        let mut landed = Vec::new();
        for (i, s) in self.states.iter_mut().enumerate() {
            if let TreeState::Falling {
                dir,
                angle_rad,
                omega_rad_s,
            } = *s
            {
                let l = self.trees[i].height_m.max(1.0);
                let omega = omega_rad_s + 1.5 * G_M_S2 / l * angle_rad.sin() * dt_s;
                let angle = angle_rad + omega * dt_s;
                *s = if angle >= FRAC_PI_2 {
                    landed.push(i as u32);
                    TreeState::Fallen { dir }
                } else {
                    TreeState::Falling {
                        dir,
                        angle_rad: angle,
                        omega_rad_s: omega,
                    }
                };
            }
        }
        landed
    }
}

/// Grows the trees: a jittered grid, thick where the forest mask is high, sparse elsewhere, never on
/// roads, in water, in cities (except parks) or on buildings.
pub fn grow(rng: &SimRng, plan: &WorldPlan, cfg: &TreeConfig, world_size_m: f32) -> Forest {
    let half = world_size_m * 0.5;
    let mask = super::terrain::ValueNoise::new(&rng.fork("mask"));
    let flat_highway = |p: Vec2| {
        plan.highways.iter().any(|h| h.project(p).0 < h.width_m * 0.5 + 6.0)
            || plan.waters.bridges.iter().any(|b| b.centre.distance(p) < b.length_m * 0.6)
    };
    let mut trees = Vec::new();
    let n = (world_size_m / cfg.spacing_m) as i32;
    let total_share: f32 = cfg.species.iter().map(|s| s.share).sum();
    for iz in 0..n {
        for ix in 0..n {
            let mut r = rng.fork(&format!("cell/{ix}/{iz}"));
            let jitter = Vec2::new(r.next_f32(), r.next_f32()) * cfg.spacing_m;
            let p = Vec2::new(-half + ix as f32 * cfg.spacing_m, -half + iz as f32 * cfg.spacing_m) + jitter;
            let in_city = plan.cities.iter().find(|c| c.rect().distance(p) < 1e-3);
            let forest = mask.height(p.x, p.y, cfg.mask_wavelength_m, 1.0, 3, 0.5) > cfg.forest_threshold;
            let chance = match in_city {
                Some(c) => {
                    let park = c
                        .blocks
                        .iter()
                        .any(|b| b.park && b.rect.inset(6.0).is_some_and(|i| i.distance(p) < 1e-3));
                    if park { cfg.park_chance } else { 0.0 }
                }
                None if forest => cfg.forest_chance,
                None => cfg.scatter_chance,
            };
            let pick = r.next_f32();
            let roll = r.next_f32() * total_share;
            let size = r.next_f32();
            let yaw = r.next_f32() * std::f32::consts::TAU;
            if pick >= chance || in_city.is_none() && plan.cities.iter().any(|c| c.rect().distance(p) < 20.0) {
                continue;
            }
            if flat_highway(p) || plan.water_at(p.x, p.y).is_some() || plan.collision.overlaps(p, 2.0) {
                continue;
            }
            let ground = plan.height_at(p.x, p.y);
            if plan.water_at(p.x + 3.0, p.y).is_some() || plan.water_at(p.x - 3.0, p.y).is_some() {
                continue;
            }
            let mut acc = 0.0;
            let si = cfg
                .species
                .iter()
                .position(|s| {
                    acc += s.share;
                    roll < acc
                })
                .unwrap_or(0);
            let sp = &cfg.species[si];
            let height_m = sp.height_m[0] + (sp.height_m[1] - sp.height_m[0]) * size;
            let trunk_radius_m = sp.trunk_radius_m[0] + (sp.trunk_radius_m[1] - sp.trunk_radius_m[0]) * size;
            trees.push(Tree {
                id: trees.len() as u32,
                species: si as u8,
                base: Vec3::new(p.x, ground, p.y),
                height_m,
                trunk_radius_m,
                break_impulse_n_s: sp.break_impulse_kn_s * 1000.0 * (0.6 + 0.8 * size),
                yaw_rad: yaw,
            });
        }
    }
    let side = (world_size_m / BUCKET_M).ceil() as usize + 1;
    let mut forest = Forest {
        states: vec![TreeState::Standing; trees.len()],
        buckets: vec![Vec::new(); side * side],
        origin_m: -half,
        side,
        trees,
    };
    for t in &forest.trees {
        let (x, z) = forest.bucket(Vec2::new(t.base.x, t.base.z));
        forest.buckets[z * side + x].push(t.id);
    }
    forest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(break_kn_s: f32) -> Forest {
        let t = Tree {
            id: 0,
            species: 0,
            base: Vec3::ZERO,
            height_m: 20.0,
            trunk_radius_m: 0.3,
            break_impulse_n_s: break_kn_s * 1000.0,
            yaw_rad: 0.0,
        };
        let mut f = Forest {
            trees: vec![t],
            states: vec![TreeState::Standing],
            buckets: vec![Vec::new(); 9],
            origin_m: -48.0,
            side: 3,
        };
        let (x, z) = f.bucket(Vec2::ZERO);
        f.buckets[z * 3 + x].push(0);
        f
    }

    #[test]
    fn a_walking_mech_snaps_a_pine_and_it_falls_the_way_it_was_pushed() {
        let mut f = one(60.0);
        let b = f.bump(Vec2::new(0.0, 3.1), 3.0, Vec2::new(0.0, -2.0), 50_000.0);
        assert_eq!(b.broke, vec![0]);
        let mut landed = Vec::new();
        for _ in 0..600 {
            landed.extend(f.step(1.0 / 60.0));
        }
        assert_eq!(landed, vec![0], "the tree comes down within 10 s");
        let TreeState::Fallen { dir } = f.states[0] else { panic!() };
        assert!(dir.y < -0.9, "it falls the way the mech pushed: {dir}");
    }

    #[test]
    fn a_trunk_too_strong_to_break_stops_the_mech() {
        let mut f = one(250.0);
        let b = f.bump(Vec2::new(0.0, 3.1), 3.0, Vec2::new(0.0, -2.0), 50_000.0);
        assert!(b.broke.is_empty() && b.blocked);
        assert!((b.pos.length() - 3.3).abs() < 1e-4);
    }

    #[test]
    fn a_tall_tree_takes_longer_to_fall_than_a_short_one() {
        let time = |h: f32| {
            let mut f = one(10.0);
            f.trees[0].height_m = h;
            f.hit(0, Vec2::X, 20_000.0);
            (1..2000).find(|_| !f.step(0.01).is_empty()).unwrap()
        };
        assert!(time(30.0) > time(10.0));
    }
}
