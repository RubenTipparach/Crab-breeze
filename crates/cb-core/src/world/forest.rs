//! Trees (openspec changes `vegetation`, `island-world`): seeded woods on open ground and in park
//! blocks, and the trees' state as the fight goes on. A tree hit with more than its break impulse
//! snaps at the base and falls as a rigid rod pivoting on its stump (`theta'' = 3 g sin(theta) / 2
//! L`), then lies where it fell. Unbroken trunks block like walls. Pure and deterministic.
//!
//! An island 40 km across would hold over a million trees, so they are grown by tile, only near the
//! bodies in the world (the pilot and every mech), and let go again far from all of them
//! (island-world design section 3). A tree is a pure function of its site, so a tile grown twice is
//! the same tile; a tree's id is its site's index, so it never depends on what was grown first.
//! Trees felled in a tile that is let go are remembered, and lie felled when it grows again.

use super::config::TreeConfig;
use super::cover::{Clearance, Cover};
use super::layout::Rect;
use super::{Ground, WorldPlan};
use crate::rng::SimRng;
use glam::{Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::FRAC_PI_2;

const G_M_S2: f32 = 9.81;
/// How far trees keep from what is built: 20 m round a city, 6 m inside a park's edge, 6 m off a
/// highway's paved edge, 0.6 of a bridge's length from its centre, 2 m from a building.
const TREE_CLEARANCE: Clearance = Clearance {
    city_m: 20.0,
    park_inset_m: 6.0,
    road_m: 6.0,
    bridge_share: 0.6,
    building_m: 2.0,
};
/// Marks a tile site with no tree.
const NO_TREE: u16 = u16::MAX;

/// A tree as it was grown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tree {
    /// Stable id: the index of its site, `iz * sites_per_side + ix`.
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
    /// Crown radius.
    pub crown_radius_m: f32,
    /// Height of the crown's underside above the base.
    pub crown_base_m: f32,
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

    /// The state a tree keeps while its tile is let go: a falling tree has landed.
    fn settled(self) -> TreeState {
        match self {
            TreeState::Falling { dir, .. } => TreeState::Fallen { dir },
            s => s,
        }
    }
}

/// A tile of grown trees.
#[derive(Clone, Debug, PartialEq)]
struct TreeTile {
    /// Trees in id order.
    trees: Vec<Tree>,
    /// State of each tree.
    states: Vec<TreeState>,
    /// Index into `trees` of each of the tile's sites (row by row), or `NO_TREE`.
    slot: Vec<u16>,
}

/// Where trees may grow: the site grid and what decides each site.
#[derive(Clone, Debug, PartialEq)]
struct Grower {
    seed: u64,
    mask: super::terrain::ValueNoise,
    cfg: TreeConfig,
    origin_m: f32,
    sites_per_side: u32,
    tiles_per_side: u32,
}

/// Every grown tree and its state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Forest {
    grower: Option<Grower>,
    tiles: BTreeMap<(u32, u32), TreeTile>,
    /// Trees falling now, ascending.
    falling: BTreeSet<u32>,
    /// Trees felled in tiles that have been let go.
    remembered: BTreeMap<u32, TreeState>,
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
    /// Share of its velocity the body keeps after the trees it snapped took theirs.
    pub kept: f32,
}

/// The shape and heft of a body pushing through trees (a mech).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Feet height.
    pub feet_y: f32,
    /// Leg reach against trunks.
    pub trunk_radius_m: f32,
    /// Hull and arm reach through crowns.
    pub brush_radius_m: f32,
    /// Height from the feet to the top.
    pub height_m: f32,
    /// Mass.
    pub mass_kg: f32,
}

/// How a body's push becomes a tree's (the `vegetation` design section 5 numbers).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Push {
    /// A push in the crown counts this many times a push on the trunk.
    pub crown_leverage: f32,
    /// Share of the speed past a tree that still pushes it.
    pub graze_share: f32,
}

/// Tiles grown and let go by one call to `Forest::keep_around`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileChanges {
    /// Tiles grown, nearest first.
    pub grown: Vec<(u32, u32)>,
    /// Tiles let go, in key order.
    pub released: Vec<(u32, u32)>,
}

impl Forest {
    /// A forest for a world `world_size_m` across, with nothing grown yet.
    pub fn new(rng: &SimRng, cfg: &TreeConfig, world_size_m: f32) -> Self {
        let sites_per_side = (world_size_m / cfg.spacing_m).max(1.0) as u32;
        Self {
            grower: Some(Grower {
                seed: rng.fork("sites").next_u64(),
                mask: super::terrain::ValueNoise::new(&rng.fork("mask")),
                cfg: cfg.clone(),
                origin_m: -world_size_m * 0.5,
                sites_per_side,
                tiles_per_side: sites_per_side.div_ceil(cfg.tile_sites),
            }),
            ..Default::default()
        }
    }

    fn site_of(&self, id: u32) -> Option<(u32, u32)> {
        let g = self.grower.as_ref()?;
        Some((id % g.sites_per_side, id / g.sites_per_side))
    }

    fn tile_and_slot(&self, id: u32) -> Option<(&TreeTile, usize)> {
        let g = self.grower.as_ref()?;
        let (ix, iz) = self.site_of(id)?;
        let ts = g.cfg.tile_sites;
        let tile = self.tiles.get(&(ix / ts, iz / ts))?;
        let local = ((iz % ts) * ts + ix % ts) as usize;
        let slot = tile.slot[local];
        (slot != NO_TREE).then_some((tile, slot as usize))
    }

    /// A grown tree, if its tile is grown and its site has one.
    pub fn tree(&self, id: u32) -> Option<&Tree> {
        self.tile_and_slot(id).map(|(t, s)| &t.trees[s])
    }

    /// What a tree is doing (standing, unless felled).
    pub fn state(&self, id: u32) -> TreeState {
        match self.tile_and_slot(id) {
            Some((t, s)) => t.states[s],
            None => self.remembered.get(&id).copied().unwrap_or(TreeState::Standing),
        }
    }

    fn set_state(&mut self, id: u32, state: TreeState) {
        let Some(g) = self.grower.as_ref() else { return };
        let ts = g.cfg.tile_sites;
        let Some((ix, iz)) = self.site_of(id) else { return };
        let Some(tile) = self.tiles.get_mut(&(ix / ts, iz / ts)) else {
            return;
        };
        let slot = tile.slot[((iz % ts) * ts + ix % ts) as usize];
        if slot != NO_TREE {
            tile.states[slot as usize] = state;
            if matches!(state, TreeState::Falling { .. }) {
                self.falling.insert(id);
            } else {
                self.falling.remove(&id);
            }
        }
    }

    /// Trees grown now.
    pub fn grown(&self) -> usize {
        self.tiles.values().map(|t| t.trees.len()).sum()
    }

    /// Tiles grown now, in key order, with their trees.
    pub fn grown_tiles(&self) -> impl Iterator<Item = ((u32, u32), &[Tree])> {
        self.tiles.iter().map(|(k, t)| (*k, t.trees.as_slice()))
    }

    /// The ground a tile covers.
    pub fn tile_rect(&self, key: (u32, u32)) -> Rect {
        let Some(g) = &self.grower else {
            return Rect {
                min: Vec2::ZERO,
                max: Vec2::ZERO,
            };
        };
        let side = g.cfg.tile_sites as f32 * g.cfg.spacing_m;
        let min = Vec2::splat(g.origin_m) + Vec2::new(key.0 as f32, key.1 as f32) * side;
        Rect {
            min,
            max: min + Vec2::splat(side),
        }
    }

    /// Tiles whose ground lies within `r` of `p`, in key order.
    fn tiles_within(&self, p: Vec2, r: f32) -> Vec<(u32, u32)> {
        let Some(g) = &self.grower else { return Vec::new() };
        let side = g.cfg.tile_sites as f32 * g.cfg.spacing_m;
        let max = g.tiles_per_side as i64 - 1;
        let t = |v: f32| (((v - g.origin_m) / side).floor() as i64).clamp(0, max.max(0)) as u32;
        let mut out = Vec::new();
        if !p.is_finite() || !r.is_finite() {
            return out;
        }
        for z in t(p.y - r)..=t(p.y + r) {
            for x in t(p.x - r)..=t(p.x + r) {
                if self.tile_rect((x, z)).distance(p) <= r {
                    out.push((x, z));
                }
            }
        }
        out
    }

    /// Grows the tiles near the bodies at `bodies` (x, z) and lets go of tiles far from all of
    /// them: every tile within `urgent_radius_m` of a body at once, then up to `tiles_per_step`
    /// more within `keep_radius_m`, nearest first; tiles beyond `release_radius_m` of every body
    /// are let go. The result depends only on the bodies' positions and what was grown before.
    pub fn keep_around(&mut self, plan: &WorldPlan, bodies: &[Vec2]) -> TileChanges {
        self.keep_around_seen(plan, bodies, bodies.len())
    }

    /// As `keep_around`, but only the first `seen` bodies (the pilot, whose eye sees the woods to
    /// the draw radius) keep tiles out to `keep_radius_m`; every other body (a mech that only has
    /// to collide with the trees near it) keeps just its `urgent_radius_m`. Growing the full radius
    /// round ten raiders grew woods nobody saw and cost about 8 ms a frame (2026-10-24).
    pub fn keep_around_seen(&mut self, plan: &WorldPlan, bodies: &[Vec2], seen: usize) -> TileChanges {
        let Some(g) = self.grower.clone() else {
            return TileChanges::default();
        };
        let cfg = &g.cfg;
        // How far round body `i` tiles are kept, and let go past.
        let reach = |i: usize| {
            if i < seen {
                (cfg.keep_radius_m, cfg.release_radius_m)
            } else {
                let margin = (cfg.release_radius_m - cfg.keep_radius_m).max(0.0);
                (cfg.urgent_radius_m, cfg.urgent_radius_m + margin)
            }
        };
        // A tile's distance to the nearest body, for growing nearest first.
        let nearest = |f: &Forest, key: (u32, u32)| {
            let r = f.tile_rect(key);
            bodies.iter().map(|b| r.distance(*b)).fold(f32::INFINITY, f32::min)
        };
        let released = |f: &Forest, key: (u32, u32)| {
            let r = f.tile_rect(key);
            bodies.iter().enumerate().all(|(i, b)| r.distance(*b) > reach(i).1)
        };
        let mut changes = TileChanges::default();
        let far: Vec<(u32, u32)> = self.tiles.keys().copied().filter(|k| released(self, *k)).collect();
        for key in far {
            if let Some(tile) = self.tiles.remove(&key) {
                for (t, s) in tile.trees.iter().zip(&tile.states) {
                    self.falling.remove(&t.id);
                    if *s != TreeState::Standing {
                        self.remembered.insert(t.id, s.settled());
                    }
                }
                changes.released.push(key);
            }
        }
        let mut wanted: Vec<(f32, (u32, u32))> = Vec::new();
        for (i, b) in bodies.iter().enumerate() {
            for key in self.tiles_within(*b, reach(i).0) {
                if !self.tiles.contains_key(&key) {
                    wanted.push((nearest(self, key), key));
                }
            }
        }
        wanted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        wanted.dedup_by_key(|w| w.1);
        let mut budget = cfg.tiles_per_step as usize;
        for (d, key) in wanted {
            if d > cfg.urgent_radius_m {
                if budget == 0 {
                    break;
                }
                budget -= 1;
            }
            self.grow_tile(plan, key);
            changes.grown.push(key);
        }
        changes
    }

    /// Grows every tile within `r` of `p` that is not grown yet (a blast far from every body).
    pub fn grow_around(&mut self, plan: &WorldPlan, p: Vec2, r: f32) {
        for key in self.tiles_within(p, r) {
            if !self.tiles.contains_key(&key) {
                self.grow_tile(plan, key);
            }
        }
    }

    fn grow_tile(&mut self, plan: &WorldPlan, key: (u32, u32)) {
        let Some(g) = &self.grower else { return };
        let trees = g.grow(plan, key);
        let ts = g.cfg.tile_sites;
        let mut slot = vec![NO_TREE; (ts * ts) as usize];
        let mut states = Vec::with_capacity(trees.len());
        for (i, t) in trees.iter().enumerate() {
            let (ix, iz) = (t.id % g.sites_per_side, t.id / g.sites_per_side);
            slot[((iz % ts) * ts + ix % ts) as usize] = i as u16;
            states.push(self.remembered.remove(&t.id).unwrap_or(TreeState::Standing));
        }
        self.tiles.insert(key, TreeTile { trees, states, slot });
    }

    /// Ids of grown trees whose trunk might be within `r` of `p`, ascending.
    pub fn near(&self, p: Vec2, r: f32) -> Vec<u32> {
        let Some(g) = &self.grower else { return Vec::new() };
        let mut out = Vec::new();
        if !p.is_finite() || !r.is_finite() {
            return out;
        }
        let s = g.cfg.spacing_m;
        let max = g.sites_per_side as i64 - 1;
        let site = |v: f32| (((v - g.origin_m) / s).floor() as i64).clamp(0, max.max(0)) as u32;
        let pad = r + 2.0;
        for iz in site(p.y - pad)..=site(p.y + pad) {
            for ix in site(p.x - pad)..=site(p.x + pad) {
                let id = iz * g.sites_per_side + ix;
                if self.tile_and_slot(id).is_some() {
                    out.push(id);
                }
            }
        }
        out
    }

    /// Snaps a standing tree if `impulse_n_s` exceeds its break impulse, starting its fall toward
    /// `dir`. Returns true if it snapped.
    pub fn hit(&mut self, id: u32, dir: Vec2, impulse_n_s: f32) -> bool {
        let Some(t) = self.tree(id).copied() else { return false };
        // A non-finite impulse never snaps a tree.
        if self.state(id) != TreeState::Standing || !impulse_n_s.is_finite() || impulse_n_s < t.break_impulse_n_s {
            return false;
        }
        // The hit lands a third of the way up: omega0 = J (L/3) / (m L^2 / 3) = J / (m L).
        let excess = impulse_n_s - t.break_impulse_n_s;
        self.set_state(
            id,
            TreeState::Falling {
                dir: dir.normalize_or(Vec2::X),
                angle_rad: 0.02,
                omega_rad_s: (excess / (t.mass_kg() * t.height_m)).min(2.0),
            },
        );
        true
    }

    /// The widest crown any species grows, for the contact search.
    fn max_crown_m(&self) -> f32 {
        self.grower.as_ref().map_or(0.0, |g| {
            g.cfg
                .species
                .iter()
                .map(|s| s.height_m[1] * s.crown_radius_share)
                .fold(0.0, f32::max)
        })
    }

    /// A body at `p` (x, z) moving at `velocity` (x, z): its legs snap the trunks they hit and are
    /// stopped by the ones they do not; its hull and arms snap the crowns they pass through (a crown
    /// that holds bends and lets it by). Each snapped tree takes some of the body's momentum.
    pub fn bump(&mut self, p: Vec2, body: &Body, velocity: Vec2, push: &Push) -> Bump {
        let mut out = Bump {
            pos: p,
            kept: 1.0,
            ..Default::default()
        };
        if !velocity.is_finite() {
            return out;
        }
        let speed = velocity.length();
        let travel = velocity.normalize_or_zero();
        for id in self.near(p, body.brush_radius_m.max(body.trunk_radius_m) + self.max_crown_m()) {
            let Some(t) = self.tree(id).copied() else { continue };
            if self.state(id) != TreeState::Standing {
                continue;
            }
            let c = Vec2::new(t.base.x, t.base.z);
            let d = out.pos - c;
            let dist = d.length();
            let normal = d.normalize_or(Vec2::X);
            let into = (-velocity.dot(normal)).max(0.0) + push.graze_share * speed;
            let fall = (travel * 0.6 - normal * 0.4).normalize_or(-normal);
            let legs = body.feet_y < t.base.y + t.height_m * 0.4 && dist < body.trunk_radius_m + t.trunk_radius_m;
            let crown = body.feet_y < t.base.y + t.height_m
                && body.feet_y + body.height_m > t.base.y + t.crown_base_m
                && dist < body.brush_radius_m + t.crown_radius_m;
            let impulse = body.mass_kg * into;
            let snapped = if legs {
                let snapped = self.hit(id, fall, impulse);
                if !snapped {
                    out.pos = c + normal * (body.trunk_radius_m + t.trunk_radius_m);
                    out.blocked = true;
                }
                snapped
            } else {
                crown && self.hit(id, fall, impulse * push.crown_leverage)
            };
            if snapped {
                out.broke.push(id);
                // The tree takes half its mass along.
                out.kept *= body.mass_kg / (body.mass_kg + 0.5 * t.mass_kg());
            }
        }
        out
    }

    /// A trunk-only push for a body that cannot break trees (the pilot on foot).
    pub fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        let mut p = p;
        for id in self.near(p, radius_m) {
            let Some(t) = self.tree(id) else { continue };
            if self.state(id) != TreeState::Standing {
                continue;
            }
            let c = Vec2::new(t.base.x, t.base.z);
            let reach = radius_m + t.trunk_radius_m;
            let d = p - c;
            if d.length() < reach {
                p = c + d.normalize_or(Vec2::X) * reach;
            }
        }
        p
    }

    /// A blast at `centre` (x, z): snaps grown trees within `radius_m`, harder near the centre, away
    /// from it.
    pub fn blast(&mut self, centre: Vec2, radius_m: f32, impulse_n_s: f32) -> Vec<u32> {
        let mut broke = Vec::new();
        for id in self.near(centre, radius_m) {
            let Some(t) = self.tree(id).copied() else { continue };
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
        let falling: Vec<u32> = self.falling.iter().copied().collect();
        for id in falling {
            let (
                Some(t),
                TreeState::Falling {
                    dir,
                    angle_rad,
                    omega_rad_s,
                },
            ) = (self.tree(id).copied(), self.state(id))
            else {
                self.falling.remove(&id);
                continue;
            };
            let l = t.height_m.max(1.0);
            let omega = omega_rad_s + 1.5 * G_M_S2 / l * angle_rad.sin() * dt_s;
            let angle = angle_rad + omega * dt_s;
            if angle >= FRAC_PI_2 {
                landed.push(id);
                self.set_state(id, TreeState::Fallen { dir });
            } else {
                self.set_state(
                    id,
                    TreeState::Falling {
                        dir,
                        angle_rad: angle,
                        omega_rad_s: omega,
                    },
                );
            }
        }
        landed
    }

    /// Ids of the trees falling now, ascending.
    pub fn falling(&self) -> Vec<u32> {
        self.falling.iter().copied().collect()
    }
}

impl Grower {
    /// The trees of one tile: a jittered grid, thick where the forest mask is high, sparse
    /// elsewhere, scaled by the biome (none in desert or snow), never on roads, in water, in cities (except parks) or on buildings.
    fn grow(&self, plan: &WorldPlan, key: (u32, u32)) -> Vec<Tree> {
        let cfg = &self.cfg;
        let ts = cfg.tile_sites;
        let total_share: f32 = cfg.species.iter().map(|s| s.share).sum();
        let mut trees = Vec::new();
        for iz in key.1 * ts..((key.1 + 1) * ts).min(self.sites_per_side) {
            for ix in key.0 * ts..((key.0 + 1) * ts).min(self.sites_per_side) {
                let mut r = SimRng::new(
                    self.seed ^ u64::from(ix).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ u64::from(iz).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
                );
                let jitter = Vec2::new(r.next_f32(), r.next_f32()) * cfg.spacing_m;
                let p = Vec2::new(self.origin_m + ix as f32 * cfg.spacing_m, self.origin_m + iz as f32 * cfg.spacing_m) + jitter;
                let pick = r.next_f32();
                let roll = r.next_f32() * total_share;
                let size = r.next_f32();
                let yaw = r.next_f32() * std::f32::consts::TAU;
                let chance = match plan.cover_at(p, &TREE_CLEARANCE) {
                    Cover::Bare => continue,
                    Cover::Park => cfg.park_chance,
                    Cover::Open => {
                        let forest = self.mask.height(p.x, p.y, cfg.mask_wavelength_m, 1.0, 3, 0.5) > cfg.forest_threshold;
                        if forest { cfg.forest_chance } else { cfg.scatter_chance }
                    }
                };
                if pick >= chance {
                    continue;
                }
                let ground = plan.height_at(p.x, p.y);
                // As thick as the biome grows them: none in the desert or the snow.
                let rise = Vec2::new(plan.height_at(p.x + 4.0, p.y) - ground, plan.height_at(p.x, p.y + 4.0) - ground).length() / 4.0;
                if pick >= chance * plan.biomes.tree_share(p, ground, rise) {
                    continue;
                }
                // Dry ground, and dry a trunk's width either side (a bank, not the water's edge).
                let w = &plan.waters;
                if w.water_at(p, ground).is_some()
                    || w.water_at(p + Vec2::X * 3.0, ground - 0.5).is_some()
                    || w.water_at(p - Vec2::X * 3.0, ground - 0.5).is_some()
                {
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
                    id: iz * self.sites_per_side + ix,
                    species: si as u8,
                    base: Vec3::new(p.x, ground, p.y),
                    height_m,
                    trunk_radius_m,
                    break_impulse_n_s: sp.break_impulse_kn_s * 1000.0 * (0.6 + 0.8 * size),
                    yaw_rad: yaw,
                    crown_radius_m: height_m * sp.crown_radius_share,
                    crown_base_m: height_m * sp.crown_base_share,
                });
            }
        }
        trees
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A forest with one tree at the origin, its tile grown, on a 512 m world.
    fn one(break_kn_s: f32) -> Forest {
        let cfg = TreeConfig {
            spacing_m: 16.0,
            tile_sites: 8,
            ..Default::default()
        };
        let mut f = Forest::new(&SimRng::new(1), &cfg, 512.0);
        let g = f.grower.clone().unwrap();
        let (ix, iz) = (16u32, 16u32);
        let id = iz * g.sites_per_side + ix;
        let t = Tree {
            id,
            species: 0,
            base: Vec3::ZERO,
            height_m: 20.0,
            trunk_radius_m: 0.3,
            break_impulse_n_s: break_kn_s * 1000.0,
            yaw_rad: 0.0,
            crown_radius_m: 4.0,
            crown_base_m: 4.0,
        };
        let mut slot = vec![NO_TREE; 64];
        slot[0] = 0;
        f.tiles.insert(
            (2, 2),
            TreeTile {
                trees: vec![t],
                states: vec![TreeState::Standing],
                slot,
            },
        );
        f
    }

    fn the(f: &Forest) -> u32 {
        f.grown_tiles().next().unwrap().1[0].id
    }

    fn mech(feet_y: f32) -> Body {
        Body {
            feet_y,
            trunk_radius_m: 3.0,
            brush_radius_m: 5.5,
            height_m: 12.0,
            mass_kg: 50_000.0,
        }
    }

    const PUSH: Push = Push {
        crown_leverage: 2.0,
        graze_share: 0.25,
    };

    #[test]
    fn a_skating_mech_brushing_a_crown_snaps_the_tree_and_keeps_its_speed() {
        let mut f = one(60.0);
        // Passing 6 m to the side at 30 m/s: the legs miss the trunk, the hull goes through the crown.
        let b = f.bump(Vec2::new(6.0, 0.0), &mech(0.0), Vec2::new(0.0, -30.0), &PUSH);
        assert_eq!(b.broke, vec![the(&f)]);
        assert!(!b.blocked, "a crown does not stop a mech");
        assert!(b.kept > 0.9 && b.kept < 1.0, "a little momentum lost: {}", b.kept);
    }

    #[test]
    fn a_mech_flying_over_the_treetops_snaps_nothing() {
        let mut f = one(60.0);
        let b = f.bump(Vec2::new(0.0, 3.1), &mech(25.0), Vec2::new(0.0, -30.0), &PUSH);
        assert!(b.broke.is_empty() && !b.blocked);
    }

    #[test]
    fn a_walking_mech_snaps_a_pine_and_it_falls_the_way_it_was_pushed() {
        let mut f = one(60.0);
        let id = the(&f);
        let b = f.bump(Vec2::new(0.0, 3.1), &mech(0.0), Vec2::new(0.0, -2.0), &PUSH);
        assert_eq!(b.broke, vec![id]);
        let mut landed = Vec::new();
        for _ in 0..600 {
            landed.extend(f.step(1.0 / 60.0));
        }
        assert_eq!(landed, vec![id], "the tree comes down within 10 s");
        let TreeState::Fallen { dir } = f.state(id) else { panic!() };
        assert!(dir.y < -0.9, "it falls the way the mech pushed: {dir}");
    }

    #[test]
    fn a_trunk_too_strong_to_break_stops_the_mech() {
        let mut f = one(250.0);
        let b = f.bump(Vec2::new(0.0, 3.1), &mech(0.0), Vec2::new(0.0, -0.5), &PUSH);
        assert!(b.broke.is_empty() && b.blocked);
        assert!((b.pos.length() - 3.3).abs() < 1e-4);
    }

    #[test]
    fn a_tall_tree_takes_longer_to_fall_than_a_short_one() {
        let time = |h: f32| {
            let mut f = one(10.0);
            let id = the(&f);
            f.tiles.get_mut(&(2, 2)).unwrap().trees[0].height_m = h;
            f.hit(id, Vec2::X, 20_000.0);
            (1..2000).find(|_| !f.step(0.01).is_empty()).unwrap()
        };
        assert!(time(30.0) > time(10.0));
    }
}
