//! Where people walk (openspec change `civilians`, design section 1): every person's route is a loop
//! round a rectangle, eight waypoints to a lap (the four corners and the four sides' midpoints). On a
//! city block it is the middle of the pavement, kerb to shopfront; in a park it is a path inside the
//! kerb with a bench at each side's midpoint; on an air base it is the apron walk in front of the
//! hangars. From a pavement corner a person may cross the street to the corner of the block opposite,
//! which shares the grid's row or column, so the crossing is straight and square.
//!
//! Fleeing runs along the pavement's own line, away from the danger, or for the door of a building
//! when the world knows its street doors (`Doors`): `StreetDoors` is every building's real street door
//! (openspec change `building-interiors`, `world::interior`), `NoDoors` a world with none.

use super::lines::Place;
use crate::rng::SimRng;
use crate::world::interior::{InteriorConfig, Site, shell};
use crate::world::{AirBase, BasesConfig, City, Rect, Structures, WorldConfig, WorldPlan};
use glam::Vec2;
use std::collections::BTreeMap;

/// Where a person was brought in and walks: the key their home is peopled and let go by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Home {
    /// A city block (pavement) or park: `(city, block index in the city)`.
    Block(u32, u32),
    /// An air base, by id.
    Base(u32),
}

/// Waypoints to one lap of a loop.
pub const LAP: u8 = 8;

/// Waypoint `k` (0..8) of the loop round `r`: even `k` the corners from the least corner
/// counter-clockwise seen from above (+x then +z), odd `k` the midpoints between them.
pub fn waypoint(r: &Rect, k: u8) -> Vec2 {
    let corner = |c: u8| match c % 4 {
        0 => r.min,
        1 => Vec2::new(r.max.x, r.min.y),
        2 => r.max,
        _ => Vec2::new(r.min.x, r.max.y),
    };
    let k = k % LAP;
    if k.is_multiple_of(2) {
        corner(k / 2)
    } else {
        (corner(k / 2) + corner(k / 2 + 1)) * 0.5
    }
}

/// The waypoint of the loop round `r` nearest `p`.
pub fn nearest_waypoint(r: &Rect, p: Vec2) -> u8 {
    (0..LAP)
        .min_by(|a, b| waypoint(r, *a).distance_squared(p).total_cmp(&waypoint(r, *b).distance_squared(p)))
        .unwrap_or(0)
}

/// The loop a home's people walk, and what kind of place it is. `None` for a home that is not in
/// the plan (or a block too small to walk round).
pub fn home_loop(
    cities: &[City],
    bases: &[AirBase],
    cfg: &WorldConfig,
    park_inset_m: f32,
    apron_m: f32,
    home: Home,
) -> Option<(Rect, Place)> {
    match home {
        Home::Block(c, b) => {
            let block = cities.get(c as usize)?.blocks.get(b as usize)?;
            if block.park {
                Some((block.rect.inset(park_inset_m)?, Place::Park))
            } else {
                Some((block.rect.inset(cfg.sidewalk_m * 0.5)?, Place::Pavement))
            }
        }
        Home::Base(id) => {
            let base = bases.iter().find(|b| b.id == id)?;
            Some((apron_walk(base, &cfg.bases, apron_m)?, Place::Base))
        }
    }
}

/// The apron walk in front of a base's hangars: from the first bay's outer side to the last's, `apron_m`
/// deep, starting 4 m out from the bays' open fronts.
pub fn apron_walk(base: &AirBase, cfg: &BasesConfig, apron_m: f32) -> Option<Rect> {
    let bays = base.hangars(cfg);
    let (first, last) = (bays.first()?, bays.last()?);
    let front = first.min.y - 4.0;
    Some(Rect {
        min: Vec2::new(first.min.x, front - apron_m),
        max: Vec2::new(last.max.x, front),
    })
}

/// Park benches: one at each side's midpoint of the park path, set 1.2 m outside the path and facing
/// it. `(seat centre, facing)`, facing a unit vector on the ground. The one place benches are decided:
/// the game draws them here and people sit on them here.
pub fn park_benches(path: &Rect) -> [(Vec2, Vec2); 4] {
    let c = path.centre();
    [1u8, 3, 5, 7].map(|k| {
        let mid = waypoint(path, k);
        let out = axis_toward(mid - c);
        (mid + out * 1.2, -out)
    })
}

/// The ground axis (+-x or +-z) nearest the direction `d`; +x for a zero vector.
pub fn axis_toward(d: Vec2) -> Vec2 {
    if d.x.abs() >= d.y.abs() {
        Vec2::new(if d.x < 0.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        Vec2::new(0.0, if d.y < 0.0 { -1.0 } else { 1.0 })
    }
}

/// A street crossing from corner waypoint `k` of block `b`'s pavement loop in city `city`, outward
/// along `out` (a ground axis pointing off the block): the block across the street in the same row or
/// column, the point on its pavement loop straight across, and that point's waypoint on its loop.
/// `None` when there is no block across (the city's edge).
pub fn crossing(city: &City, cfg: &WorldConfig, b: u32, k: u8, out: Vec2) -> Option<(u32, Vec2, u8)> {
    let here = city.blocks.get(b as usize)?.rect;
    let from = waypoint(&here.inset(cfg.sidewalk_m * 0.5)?, k);
    let along_x = out.x != 0.0;
    let mut best: Option<(f32, u32)> = None;
    for (i, other) in city.blocks.iter().enumerate() {
        if i as u32 == b || other.park {
            continue;
        }
        // same row (for a crossing along x) or column (along z), and ahead
        let (same, gap) = if along_x {
            (
                (other.rect.min.y - here.min.y).abs() < 0.5,
                if out.x > 0.0 {
                    other.rect.min.x - here.max.x
                } else {
                    here.min.x - other.rect.max.x
                },
            )
        } else {
            (
                (other.rect.min.x - here.min.x).abs() < 0.5,
                if out.y > 0.0 {
                    other.rect.min.y - here.max.y
                } else {
                    here.min.y - other.rect.max.y
                },
            )
        };
        if same && gap > 0.0 && best.is_none_or(|(g, _)| gap < g) {
            best = Some((gap, i as u32));
        }
    }
    let (_, to_b) = best?;
    let there = city.blocks[to_b as usize].rect.inset(cfg.sidewalk_m * 0.5)?;
    let to = if along_x {
        Vec2::new(if out.x > 0.0 { there.min.x } else { there.max.x }, from.y)
    } else {
        Vec2::new(from.x, if out.y > 0.0 { there.min.y } else { there.max.y })
    };
    Some((to_b, to, nearest_waypoint(&there, to)))
}

/// The two ways off a block at corner waypoint `k` (even), as ground axes pointing away from it.
pub fn corner_outs(k: u8) -> [Vec2; 2] {
    match (k % LAP) / 2 {
        0 => [Vec2::NEG_X, Vec2::NEG_Y],
        1 => [Vec2::X, Vec2::NEG_Y],
        2 => [Vec2::X, Vec2::Y],
        _ => [Vec2::NEG_X, Vec2::Y],
    }
}

/// The world's street doors, as a fleeing person needs them: `StreetDoors` in the game, `NoDoors` where
/// there are none.
pub trait Doors {
    /// The street door nearest `p` within `reach_m` that does not lie toward `danger` (a door the
    /// danger is nearer than the person is not run for), if any.
    fn refuge(&self, p: Vec2, danger: Vec2, reach_m: f32) -> Option<Vec2>;
}

/// A world with no street doors yet: nobody can run inside.
pub struct NoDoors;

impl Doors for NoDoors {
    fn refuge(&self, _p: Vec2, _danger: Vec2, _reach_m: f32) -> Option<Vec2> {
        None
    }
}

/// The side of a `StreetDoors` grid cell, metres.
const DOOR_CELL_M: f32 = 64.0;

/// Every building's street door, as somebody running from danger makes for it: the point just outside
/// the door, where its way in begins (`interior::Entrance::route`). Built once from the same rule the
/// game hangs the doors by (`interior::shell`), so a person runs for the door the pilot can walk through.
/// Kept in a grid of `DOOR_CELL_M` cells, so a search looks only at the doors near the person.
#[derive(Clone, Debug, Default)]
pub struct StreetDoors {
    grid: BTreeMap<(i32, i32), Vec<Vec2>>,
}

impl StreetDoors {
    /// Doors at these points (x, z).
    pub fn new(doors: impl IntoIterator<Item = Vec2>) -> Self {
        let mut grid: BTreeMap<(i32, i32), Vec<Vec2>> = BTreeMap::new();
        for d in doors.into_iter().filter(|d| d.is_finite()) {
            grid.entry(Self::cell(d)).or_default().push(d);
        }
        Self { grid }
    }

    /// The street door of every building in `plan`, on its cells in `structures`, laid out with the dressing
    /// seed `seed` (the one the game builds its interiors with).
    pub fn of_world(plan: &WorldPlan, structures: &Structures, cfg: &InteriorConfig, seed: &SimRng) -> Self {
        Self::new(structures.buildings.iter().enumerate().map(|(b, c)| {
            let site = Site::of(plan, b as u32, c.n, c.origin, c.size);
            let e = shell(&site, cfg, seed).entrance;
            e.route.first().copied().unwrap_or(e.centre)
        }))
    }

    /// How many doors.
    pub fn len(&self) -> usize {
        self.grid.values().map(Vec::len).sum()
    }

    /// True when there are none.
    pub fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }

    fn cell(p: Vec2) -> (i32, i32) {
        ((p.x / DOOR_CELL_M).floor() as i32, (p.y / DOOR_CELL_M).floor() as i32)
    }
}

impl Doors for StreetDoors {
    fn refuge(&self, p: Vec2, danger: Vec2, reach_m: f32) -> Option<Vec2> {
        if !(reach_m.is_finite() && reach_m >= 0.0) {
            return None;
        }
        let (lo, hi) = (Self::cell(p - Vec2::splat(reach_m)), Self::cell(p + Vec2::splat(reach_m)));
        let mut best: Option<(f32, Vec2)> = None;
        for x in lo.0..=hi.0 {
            for z in lo.1..=hi.1 {
                for &d in self.grid.get(&(x, z)).into_iter().flatten() {
                    let dist = d.distance(p);
                    if dist > reach_m || d.distance(danger) < dist {
                        continue;
                    }
                    // nearest first; a tie to the lower x, then z, so the search order never decides
                    let better =
                        best.is_none_or(|(bd, bp)| dist.total_cmp(&bd).then(d.x.total_cmp(&bp.x)).then(d.y.total_cmp(&bp.y)).is_lt());
                    if better {
                        best = Some((dist, d));
                    }
                }
            }
        }
        best.map(|(_, d)| d)
    }
}

/// Which way to run from `danger`, standing at `p` on a loop round `r` (a pavement's line), or in the
/// open when `r` is `None`. On a pavement the run follows the side's own line (along x on a side
/// running along x), away from the danger; at a corner either line will do. In the open, straight away.
pub fn flee_dir(p: Vec2, danger: Vec2, r: Option<&Rect>) -> Vec2 {
    let away = p - danger;
    let open = away.normalize_or(Vec2::X);
    let Some(r) = r else { return open };
    let on_x_side = (p.y - r.min.y).abs() < 0.5 || (p.y - r.max.y).abs() < 0.5;
    let on_z_side = (p.x - r.min.x).abs() < 0.5 || (p.x - r.max.x).abs() < 0.5;
    let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
    match (on_x_side, on_z_side) {
        (true, false) => Vec2::new(sign(away.x), 0.0),
        (false, true) => Vec2::new(0.0, sign(away.y)),
        _ => axis_toward(away),
    }
}

/// The block in `city` whose pavement loop passes within `within_m` of `p`, nearest first (ties to the
/// lower index), with the loop's waypoint nearest `p`. Parks are left out (their paths are inside).
pub fn pavement_near(city: &City, cfg: &WorldConfig, p: Vec2, within_m: f32) -> Option<(u32, u8)> {
    let mut best: Option<(f32, u32, u8)> = None;
    for (i, b) in city.blocks.iter().enumerate() {
        if b.park || b.rect.distance(p) > within_m + cfg.sidewalk_m {
            continue;
        }
        let Some(r) = b.rect.inset(cfg.sidewalk_m * 0.5) else { continue };
        // distance to the loop's line: outside it, to the rectangle; inside, to the nearest side
        let d = if r.distance(p) > 0.0 {
            r.distance(p)
        } else {
            (p.x - r.min.x).min(r.max.x - p.x).min(p.y - r.min.y).min(r.max.y - p.y)
        };
        if d <= within_m && best.is_none_or(|(bd, _, _)| d < bd) {
            best = Some((d, i as u32, nearest_waypoint(&r, p)));
        }
    }
    best.map(|(_, b, k)| (b, k))
}
