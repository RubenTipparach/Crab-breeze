//! Road decks (openspec change `city-districts`, design section 5): every road surface that stands over the ground on
//! piers or over another road. A highway's viaduct, wherever its grade stands well over the land (the city's plateau
//! holds the land down under a highway climbing onto the trunk), and an exit's ramps. A deck is a centre line with its
//! top's height at each point, a width and a slab under its top; bodies stand on it when their feet are within a step of
//! it (as on a roof or the Stratostacean's deck) and pass under it with room to spare, so a car, the pilot, the people and
//! the mech each choose their level by their feet. Pure: no rendering, no engine.

use super::grid::CellIndex;
use glam::{Vec2, Vec3};

/// A body stands on a deck whose top is no more than this over its feet (a step up, as a roof).
pub const STEP_M: f32 = 1.0;
/// A deck's slab is in the way of a body whose feet are below it by less than this: room for a car, a lorry, a person
/// (the 4.5 m every deck keeps over the road under it).
pub const HEADROOM_M: f32 = 4.5;
/// Cell size of the decks' index.
const CELL_M: f32 = 128.0;

/// What a deck is part of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckKind {
    /// A highway's viaduct: highway `highway`.
    Viaduct {
        /// Index in `WorldPlan::highways`.
        highway: u32,
    },
    /// One of an exit's ramps: ramp `ramp` of `WorldPlan::exits`'s ramps.
    Ramp {
        /// Its index among the plan's ramps.
        ramp: u32,
    },
}

/// A pier under a deck: a column from the ground to the slab's underside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pier {
    /// Where it stands (x, z).
    pub at: Vec2,
    /// Its heading (radians about the vertical, as `Lot::heading_rad`): its long side across the deck.
    pub heading_rad: f32,
    /// The slab's underside it carries.
    pub top_m: f32,
    /// Across the deck and along it, metres.
    pub size_m: Vec2,
}

/// One deck.
#[derive(Clone, Debug, PartialEq)]
pub struct RoadDeck {
    /// Its centre line: (x, top of the road, z) at each point.
    pub pts: Vec<Vec3>,
    /// Paved width, metres.
    pub width_m: f32,
    /// The slab's depth under its top, metres.
    pub slab_m: f32,
    /// Below this height over the ground under it the deck stands on fill (walls down to the ground, nothing under
    /// it); above, on piers. 0 for a deck on piers all the way.
    pub fill_below_m: f32,
    /// What it is part of.
    pub kind: DeckKind,
    /// Its piers.
    pub piers: Vec<Pier>,
}

impl RoadDeck {
    /// Where `p` (x, z) projects on the centre line: the distance from it, the top there, and the length along it.
    /// `None` when the foot of the perpendicular falls off both ends.
    pub fn project(&self, p: Vec2) -> Option<(f32, f32, f32)> {
        let mut best: Option<(f32, f32, f32)> = None;
        let mut along = 0.0;
        let last = self.pts.len().saturating_sub(2);
        for (i, w) in self.pts.windows(2).enumerate() {
            let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
            let ab = b - a;
            let l2 = ab.length_squared();
            let len = l2.sqrt();
            if l2 > 1e-9 {
                let t = (p - a).dot(ab) / l2;
                // only the deck's own two ends stop it: at a bend, a point on its outside lies past one leg's end and
                // before the next one's start, and is on the deck by the joint between them
                let lo = if i == 0 { -1e-4 } else { f32::NEG_INFINITY };
                let hi = if i == last { 1.0 + 1e-4 } else { f32::INFINITY };
                if (lo..=hi).contains(&t) {
                    let t = t.clamp(0.0, 1.0);
                    let d = (a + ab * t).distance(p);
                    if best.is_none_or(|(bd, _, _)| d < bd) {
                        best = Some((d, w[0].y + (w[1].y - w[0].y) * t, along + len * t));
                    }
                }
            }
            along += len;
        }
        best
    }

    /// The top of the road at `p` (x, z) when `p` is on the deck (within half its width).
    pub fn top_at(&self, p: Vec2) -> Option<f32> {
        self.project(p).and_then(|(d, top, _)| (d <= self.width_m * 0.5).then_some(top))
    }

    /// The slab's underside at a top of `top_m` over ground at `ground_m`: on fill (the ground) where it is low, else
    /// the slab's depth under the top.
    pub fn underside_m(&self, top_m: f32, ground_m: f32) -> f32 {
        if top_m - ground_m < self.fill_below_m {
            ground_m
        } else {
            top_m - self.slab_m
        }
    }

    /// Length of its centre line, metres.
    pub fn length_m(&self) -> f32 {
        self.pts
            .windows(2)
            .map(|w| Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length())
            .sum()
    }

    /// The box round it (x, z), grown by `grow_m`.
    fn bounds(&self, grow_m: f32) -> (Vec2, Vec2) {
        let lo = self.pts.iter().fold(Vec2::splat(f32::MAX), |m, p| m.min(Vec2::new(p.x, p.z)));
        let hi = self.pts.iter().fold(Vec2::splat(f32::MIN), |m, p| m.max(Vec2::new(p.x, p.z)));
        (lo - Vec2::splat(grow_m), hi + Vec2::splat(grow_m))
    }
}

/// Every deck of a world, with an index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decks {
    /// The decks, by id.
    pub decks: Vec<RoadDeck>,
    index: CellIndex,
}

impl Decks {
    /// The decks of a world `world_size_m` across.
    pub fn new(decks: Vec<RoadDeck>, world_size_m: f32) -> Self {
        let boxes: Vec<(u32, Vec2, Vec2)> = decks
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let (lo, hi) = d.bounds(d.width_m * 0.5 + 1.0);
                (i as u32, lo, hi)
            })
            .collect();
        let side = (world_size_m / CELL_M).ceil() as usize;
        Self {
            index: CellIndex::build(-world_size_m * 0.5, CELL_M, side, &boxes),
            decks,
        }
    }

    /// The decks whose box reaches `p`.
    pub fn near(&self, p: Vec2) -> impl Iterator<Item = &RoadDeck> {
        self.index.at(p).iter().map(|&i| &self.decks[i as usize])
    }

    /// The highest deck top at `p` no higher than a step over `feet_y`: what a body there stands on, if a deck.
    pub fn top_under(&self, p: Vec2, feet_y: f32) -> Option<f32> {
        self.near(p)
            .filter_map(|d| d.top_at(p))
            .filter(|&top| top <= feet_y + STEP_M)
            .reduce(f32::max)
    }

    /// True when `p` is on any deck (a highway's viaduct there is a deck, not the grade under it).
    pub fn covers(&self, p: Vec2) -> bool {
        self.near(p).any(|d| d.top_at(p).is_some())
    }

    /// Pushes a body of radius `radius_m` at `p` with its feet at `feet_y` out from a deck's slab that is in its way:
    /// a top more than a step over its feet and an underside less than `HEADROOM_M` over them (on fill, the ground
    /// `ground` gives). Out sideways, to the nearer edge.
    pub fn push_out(&self, p: Vec2, radius_m: f32, feet_y: f32, ground: &dyn Fn(Vec2) -> f32) -> Vec2 {
        let mut p = p;
        for d in self.near(p) {
            let Some((dist, top, _)) = d.project(p) else { continue };
            let reach = d.width_m * 0.5 + radius_m;
            if dist >= reach || top <= feet_y + STEP_M || d.underside_m(top, ground(p)) >= feet_y + HEADROOM_M {
                continue;
            }
            // out along the perpendicular from the centre line, on the side the body is
            let Some(foot) = foot_of(d, p) else { continue };
            let away = (p - foot).normalize_or(Vec2::X);
            p = foot + away * reach;
        }
        p
    }
}

/// A highway is a deck where its road stands at least this far over the land under it: on fill (walls to the ground)
/// while it is low, on piers once there is room under it (`FILL_BELOW_M`), so no road ever floats over the land.
pub const VIADUCT_OVER_M: f32 = 0.3;
/// A deck stands on fill below this height over the land, on piers above it: 4.5 m of headroom under a 1.2 m slab,
/// and a little to spare.
pub const FILL_BELOW_M: f32 = HEADROOM_M + VIADUCT_SLAB_M + 0.2;
/// A viaduct's slab, metres deep.
pub const VIADUCT_SLAB_M: f32 = 1.2;
/// A pier every so far along a viaduct, where it may stand.
const PIER_EVERY_M: f32 = 32.0;
/// Along the highway, the land is looked at this often.
const SAMPLE_M: f32 = 4.0;

/// The viaducts of `highways` (by index): every run of road whose top (its grade plus `road_lift_m`) stands at least
/// `VIADUCT_OVER_M` over `ground` anywhere across its width, carried one sample past each end so it lands on its embankment; a pier every
/// `PIER_EVERY_M` where the land is under the slab and `pier_ok` lets it stand (never on a street).
pub fn viaducts(
    highways: &[super::layout::Highway],
    road_lift_m: f32,
    ground: &dyn Fn(Vec2) -> f32,
    pier_ok: &dyn Fn(Vec2, f32) -> bool,
) -> Vec<RoadDeck> {
    let mut out = Vec::new();
    for (k, h) in highways.iter().enumerate() {
        let len = h.length_m();
        if len <= 0.0 {
            continue;
        }
        let n = (len / SAMPLE_M).ceil().max(1.0) as usize;
        let at = |i: usize| {
            let t = (i as f32 / n as f32).clamp(0.0, 1.0);
            let xz = h.point_at(t);
            (xz, h.grade_at(t) + road_lift_m)
        };
        // across its whole width: where the land falls away across the road, an edge stands over it first
        let over: Vec<bool> = (0..=n)
            .map(|i| {
                let (xz, top) = at(i);
                let t = (i as f32 / n as f32).clamp(0.0, 1.0);
                let across = h.dir_at(t).perp() * h.width_m * 0.5;
                [xz - across, xz, xz + across].iter().any(|&p| top - ground(p) >= VIADUCT_OVER_M)
            })
            .collect();
        let mut i = 0;
        while i <= n {
            if !over[i] {
                i += 1;
                continue;
            }
            let start = i.saturating_sub(1);
            while i <= n && over[i] {
                i += 1;
            }
            let end = i.min(n);
            let pts: Vec<Vec3> = (start..=end)
                .map(|j| {
                    let (xz, top) = at(j);
                    Vec3::new(xz.x, top, xz.y)
                })
                .collect();
            if pts.len() < 2 {
                continue;
            }
            let mut deck = RoadDeck {
                pts,
                width_m: h.width_m,
                slab_m: VIADUCT_SLAB_M,
                fill_below_m: FILL_BELOW_M,
                kind: DeckKind::Viaduct { highway: k as u32 },
                piers: Vec::new(),
            };
            deck.piers = piers_under(&deck, ground, pier_ok)
                .into_iter()
                .filter(|p| p.top_m + deck.slab_m - ground(p.at) >= FILL_BELOW_M)
                .collect();
            out.push(deck);
        }
    }
    out
}

/// Piers every `PIER_EVERY_M` along `deck`, where the land is under its slab by a metre or more and `pier_ok` lets
/// one stand (a column `width_m` 0.55 across the deck and 2.2 m along it).
pub fn piers_under(deck: &RoadDeck, ground: &dyn Fn(Vec2) -> f32, pier_ok: &dyn Fn(Vec2, f32) -> bool) -> Vec<Pier> {
    let mut out = Vec::new();
    let mut s = PIER_EVERY_M * 0.5;
    let total = deck.length_m();
    let size = Vec2::new(deck.width_m * 0.55, 2.2);
    while s < total {
        let mut along = 0.0;
        for w in deck.pts.windows(2) {
            let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
            let len = a.distance(b);
            if s <= along + len && len > 1e-6 {
                let t = (s - along) / len;
                let at = a + (b - a) * t;
                let top = w[0].y + (w[1].y - w[0].y) * t - deck.slab_m;
                let dir = (b - a) / len;
                // the pier's long side across the deck: its heading turns its +x (across) to the deck's normal
                let heading_rad = dir.x.atan2(dir.y) + std::f32::consts::FRAC_PI_2;
                if top - ground(at) >= 1.0 && pier_ok(at, size.x * 0.5 + 1.0) {
                    out.push(Pier {
                        at,
                        heading_rad,
                        top_m: top,
                        size_m: size,
                    });
                }
                break;
            }
            along += len;
        }
        s += PIER_EVERY_M;
    }
    out
}

/// The point of `d`'s centre line nearest `p` (x, z), where the perpendicular falls on it.
fn foot_of(d: &RoadDeck, p: Vec2) -> Option<Vec2> {
    let mut best: Option<(f32, Vec2)> = None;
    for w in d.pts.windows(2) {
        let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
        let ab = b - a;
        let l2 = ab.length_squared();
        if l2 <= 1e-9 {
            continue;
        }
        let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
        let q = a + ab * t;
        let dd = q.distance(p);
        if best.is_none_or(|(bd, _)| dd < bd) {
            best = Some((dd, q));
        }
    }
    best.map(|b| b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A viaduct 9 m up along x from -100 to 100, 28 m wide, its slab 1.2 m deep.
    fn viaduct() -> Decks {
        Decks::new(
            vec![RoadDeck {
                pts: vec![Vec3::new(-100.0, 9.0, 0.0), Vec3::new(100.0, 9.0, 0.0)],
                width_m: 28.0,
                slab_m: 1.2,
                fill_below_m: 0.0,
                kind: DeckKind::Viaduct { highway: 0 },
                piers: Vec::new(),
            }],
            4096.0,
        )
    }

    #[test]
    fn a_car_on_the_deck_stands_on_it_and_one_under_it_stands_on_the_street() {
        let d = viaduct();
        assert_eq!(d.top_under(Vec2::new(0.0, 5.0), 9.0), Some(9.0), "on the deck");
        assert_eq!(d.top_under(Vec2::new(0.0, 5.0), 0.15), None, "under it: the street's own surface");
        assert_eq!(d.top_under(Vec2::new(0.0, 20.0), 9.0), None, "off its edge");
    }

    #[test]
    fn a_car_under_the_deck_passes_and_a_body_at_its_level_beside_it_is_kept_off_its_side() {
        let d = viaduct();
        let flat = |_: Vec2| 0.0;
        let under = Vec2::new(0.0, 3.0);
        assert_eq!(d.push_out(under, 1.0, 0.15, &flat), under, "7.8 m of room under the slab");
        // a body whose feet are 5 m up (on a ramp beside it, or climbing) meets the slab's side
        let beside = Vec2::new(0.0, 14.5);
        let out = d.push_out(beside, 1.0, 5.0, &flat);
        assert!(out.y >= 15.0 - 1e-3, "pushed out to the edge plus its radius: {out}");
    }

    #[test]
    fn a_low_deck_on_fill_is_a_wall_to_the_ground() {
        let mut d = viaduct();
        d.decks[0].pts = vec![Vec3::new(-50.0, 3.0, 0.0), Vec3::new(50.0, 3.0, 0.0)];
        d.decks[0].fill_below_m = 5.9;
        let d = Decks::new(d.decks, 4096.0);
        let out = d.push_out(Vec2::new(0.0, 14.0), 1.0, 0.0, &|_| 0.0);
        assert!(out.y >= 15.0 - 1e-3, "a car meets the fill's wall, it cannot pass under: {out}");
    }

    #[test]
    fn a_point_on_the_outside_of_a_bend_is_on_the_deck_and_one_past_its_end_is_not() {
        // a deck turning 90 degrees at the origin: along -x to it, then along +z from it
        let d = RoadDeck {
            pts: vec![Vec3::new(-100.0, 9.0, 0.0), Vec3::ZERO.with_y(9.0), Vec3::new(0.0, 9.0, 100.0)],
            width_m: 28.0,
            slab_m: 1.2,
            fill_below_m: 0.0,
            kind: DeckKind::Viaduct { highway: 0 },
            piers: Vec::new(),
        };
        // outside the corner, past the first leg's end and before the second's start
        assert_eq!(d.top_at(Vec2::new(8.0, -8.0)), Some(9.0), "the outside of the bend");
        assert_eq!(d.top_at(Vec2::new(-104.0, 0.0)), None, "past the deck's first end");
        assert_eq!(d.top_at(Vec2::new(0.0, 104.0)), None, "past its last end");
    }
}
