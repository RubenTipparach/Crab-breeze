//! Houses and small businesses breaking (openspec change `small-buildings`, design sections 6 and 7.6):
//! the kit's support and collapse rules, ported from the mockup (`standing` in
//! `docs/mockups/small_buildings_kit.js`). A piece stands while it is reached from the ground through
//! `up` links and at most `max_side_steps` `side` links; a building whose standing wall area falls
//! below `collapse_wall_share` of its whole collapses above its floor. What breaks becomes chunks for
//! `debris-physics`. Only damaged buildings have a state; every other one is whole.

use super::kit::{Chunking, Kit};
use super::lots::Lots;
use crate::world::destruct::Chunk;
use glam::Vec3;
use std::collections::{BTreeMap, VecDeque};

/// A chunk's `building` for a house: this plus the lot id (city building ids stay below it).
pub const HOUSE_CHUNK_BASE: u32 = 1 << 30;

/// Chunks of a broken box are about this across...
const CHUNK_M: f32 = 1.5;
/// ...and about this in a collapse.
const COLLAPSE_CHUNK_M: f32 = 2.2;
/// A piece thinner than this turns to dust.
const DUST_M: f32 = 0.06;
/// A piece resting on the ground (its foot at or below this, in the building's frame).
const GROUND_Y_M: f32 = 0.05;
/// In a collapse, what stands on the floor slab stays: a piece falls when its foot is above this...
const COLLAPSE_FOOT_M: f32 = 0.6;
/// ...or its top above this.
const COLLAPSE_TOP_M: f32 = 1.2;

/// A ship holed at or under its waterline (openspec change `seaport-layout` section 2, the mockup's `sinkHulk`): it
/// goes down at once and whole, slowly for its size, leaning a little; once its keel meets the seabed it settles into it
/// at `SINK_BURY` of that speed, and when it is under the ground it is gone. Its pieces stay where they are on it, so it
/// is still shot and broken piece by piece as it goes down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sinking {
    /// How far it has gone down, metres.
    pub drop_m: f32,
    /// How fast it goes down now, metres a second.
    pub speed_m_s: f32,
    /// Its top speed, metres a second (`SINK_SPEED_M` over its length, within `SINK_MIN_M_S` and `SINK_MAX_M_S`).
    pub top_m_s: f32,
    /// Its lean now, radians, about its length's axis...
    pub tilt_rad: f32,
    /// ...and the lean it settles to.
    pub tilt_to_rad: f32,
    /// True when its length runs along its frame's x (it leans about x, else about z).
    pub along_x: bool,
    /// True once its keel has met the seabed.
    pub burying: bool,
}

/// A ship's top sinking speed is this over its length (the mockup's `HULK`: a 120 m freighter at 0.25 m/s)...
const SINK_SPEED_M: f32 = 30.0;
/// ...within these, metres a second (a launch at 2).
const SINK_MIN_M_S: f32 = 0.25;
const SINK_MAX_M_S: f32 = 2.0;
/// It gathers speed at this, metres a second squared.
const SINK_ACCEL_M_S2: f32 = 0.15;
/// It settles into the seabed at this share of its top speed.
const SINK_BURY: f32 = 0.4;
/// It leans this much, radians (less for a long ship: times 40 m over its length, at most 1).
const SINK_TILT_RAD: f32 = 0.12;
/// A ship holed by a piece reaching this far over its waterline goes down.
const SINK_HOLED_M: f32 = 1.5;

impl Sinking {
    fn start(kit: &Kit, lot: u32) -> Self {
        let size = kit.hi - kit.lo;
        let len = size.x.max(size.z).max(1.0);
        // Which way it leans is the lot's parity: the same ship leans the same way every run (CLAUDE.md 6.3).
        let sign = if lot.is_multiple_of(2) { 1.0 } else { -1.0 };
        Self {
            drop_m: 0.0,
            speed_m_s: 0.0,
            top_m_s: (SINK_SPEED_M / len).clamp(SINK_MIN_M_S, SINK_MAX_M_S),
            tilt_rad: 0.0,
            tilt_to_rad: sign * SINK_TILT_RAD * (40.0 / len.max(10.0)).min(1.0),
            along_x: size.x > size.z,
            burying: false,
        }
    }

    /// Its motion in its kit's frame (applied to the kit before the lot's own pose): down by `drop_m`, leaning about
    /// the middle of its keel.
    pub fn transform(&self, kit: &Kit) -> glam::Affine3A {
        let c = Vec3::new((kit.lo.x + kit.hi.x) * 0.5, 0.0, (kit.lo.z + kit.hi.z) * 0.5);
        let axis = if self.along_x { Vec3::X } else { Vec3::Z };
        glam::Affine3A::from_translation(c - Vec3::Y * self.drop_m)
            * glam::Affine3A::from_axis_angle(axis, self.tilt_rad)
            * glam::Affine3A::from_translation(-c)
    }
}

/// One damaged building: which pieces stand.
#[derive(Clone, Debug, PartialEq)]
pub struct HouseState {
    /// By piece.
    pub alive: Vec<bool>,
    /// True once it has collapsed.
    pub collapsed: bool,
    /// Bumped on every change, so its colliders and meshes know to follow.
    pub version: u32,
    /// A ship going down.
    pub sinking: Option<Sinking>,
}

/// What a hit did to a building.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Broken {
    /// The pieces that came down (struck, or left without support), ascending.
    pub pieces: Vec<u32>,
    /// What they became.
    pub chunks: Vec<Chunk>,
    /// True when this hit collapsed it.
    pub collapsed: bool,
}

/// The pieces of `kit` that stand, given which are `alive`: reached from a piece on the ground
/// through `up` links and at most `max_side_steps` `side` links.
pub fn standing(kit: &Kit, alive: &[bool]) -> Vec<bool> {
    standing_pieces(&kit.pieces, kit.max_side_steps, alive)
}

/// `standing` over a list of pieces.
pub fn standing_pieces(pieces: &[super::kit::Piece], max_side_steps: u32, alive: &[bool]) -> Vec<bool> {
    let n = pieces.len();
    let mut best: Vec<Option<u32>> = vec![None; n];
    let mut queue = VecDeque::new();
    for (i, p) in pieces.iter().enumerate() {
        if alive[i] && p.lo.y <= GROUND_Y_M {
            best[i] = Some(0);
            queue.push_back(i);
        }
    }
    while let Some(q) = queue.pop_front() {
        let lat = best[q].unwrap_or(0);
        for &nb in &pieces[q].up {
            let nb = nb as usize;
            if alive[nb] && best[nb].is_none_or(|b| b > 0) {
                best[nb] = Some(0);
                queue.push_back(nb);
            }
        }
        if lat < max_side_steps {
            for &nb in &pieces[q].side {
                let nb = nb as usize;
                if alive[nb] && best[nb].is_none_or(|b| b > lat + 1) {
                    best[nb] = Some(lat + 1);
                    queue.push_back(nb);
                }
            }
        }
    }
    best.iter().map(Option::is_some).collect()
}

/// The chunks a piece of `kit` on `lot` becomes (world frame), `collapse` making them coarser; `sinking` moves them
/// with a ship going down.
fn chunks_of(kit: &Kit, piece: usize, lots: &Lots, lot: u32, collapse: bool, sinking: Option<&Sinking>) -> Vec<Chunk> {
    let p = &kit.pieces[piece];
    let l = &lots.lots[lot as usize];
    let size = p.hi - p.lo;
    if size.min_element() < DUST_M {
        return Vec::new();
    }
    let building = HOUSE_CHUNK_BASE + lot;
    let moved = sinking.map(|s| s.transform(kit));
    let world = |c: Vec3| l.point_to_world(moved.map_or(c, |m| m.transform_point3(c)));
    // The world size of a box turned by the lot's heading (its bounds).
    let turned = |s: Vec3| {
        let (sn, cs) = l.heading_rad.sin_cos();
        Vec3::new(s.x * cs.abs() + s.z * sn.abs(), s.y, s.x * sn.abs() + s.z * cs.abs())
    };
    match &p.chunking {
        Chunking::Box { .. } => {
            let step = if collapse { COLLAPSE_CHUNK_M } else { CHUNK_M };
            let n = (size / step).ceil().max(Vec3::ONE);
            let (nx, ny, nz) = (n.x as u32, n.y as u32, n.z as u32);
            let cell = size / n;
            let mut out = Vec::with_capacity((nx * ny * nz) as usize);
            for i in 0..nx {
                for j in 0..ny {
                    for k in 0..nz {
                        let c = p.lo + cell * (Vec3::new(i as f32, j as f32, k as f32) + 0.5);
                        out.push(Chunk {
                            building,
                            centre: world(c),
                            size: turned(cell),
                        });
                    }
                }
            }
            out
        }
        Chunking::Panel { .. } | Chunking::Whole => vec![Chunk {
            building,
            centre: world((p.lo + p.hi) * 0.5),
            size: turned(size.min(Vec3::splat(4.0))),
        }],
    }
}

/// Every damaged house and small business.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Houses {
    /// By lot id.
    pub states: BTreeMap<u32, HouseState>,
}

impl Houses {
    /// Which pieces of `lot` stand (`None`: all of them).
    pub fn alive(&self, lot: u32) -> Option<&[bool]> {
        self.states.get(&lot).map(|s| s.alive.as_slice())
    }

    /// Its state's version (0 while whole).
    pub fn version(&self, lot: u32) -> u32 {
        self.states.get(&lot).map_or(0, |s| s.version)
    }

    /// A world point in `lot`'s kit frame, through a sinking ship's motion.
    fn to_kit(&self, lots: &Lots, lot: u32, p: Vec3) -> Vec3 {
        let l = &lots.lots[lot as usize];
        let q = l.point_to_local(p);
        match self.states.get(&lot).and_then(|s| s.sinking.as_ref()) {
            Some(s) => s.transform(lots.kit(lot)).inverse().transform_point3(q),
            None => q,
        }
    }

    /// A sinking ship's motion in its kit's frame (`Sinking::transform`), if `lot` is one.
    pub fn sink_transform(&self, lots: &Lots, lot: u32) -> Option<glam::Affine3A> {
        let s = self.states.get(&lot)?.sinking.as_ref()?;
        Some(s.transform(lots.kit(lot)))
    }

    /// Takes every sinking ship down by `dt` seconds over the ground `seabed` (x, z to its height): each gathers speed
    /// to its top, leans to its lean, settles into the seabed once its keel meets it, and is gone (every piece) once
    /// it is under. Returns the lots that moved, and of them those just gone.
    pub fn step_sinking(&mut self, lots: &Lots, dt: f32, seabed: &dyn Fn(glam::Vec2) -> f32) -> (Vec<u32>, Vec<u32>) {
        let (mut moved, mut gone) = (Vec::new(), Vec::new());
        if !dt.is_finite() || dt <= 0.0 {
            return (moved, gone);
        }
        for (&lot, st) in self.states.iter_mut() {
            let Some(s) = st.sinking.as_mut() else { continue };
            if st.alive.iter().all(|a| !a) {
                continue;
            }
            let l = &lots.lots[lot as usize];
            let kit = lots.kit(lot);
            let floor = seabed(l.centre);
            if s.burying {
                s.drop_m += s.top_m_s * SINK_BURY * dt;
            } else {
                s.speed_m_s = (s.speed_m_s + SINK_ACCEL_M_S2 * dt).min(s.top_m_s);
                s.drop_m += s.speed_m_s * dt;
                s.tilt_rad += (s.tilt_to_rad - s.tilt_rad) * (dt * 0.15).min(1.0);
                if l.ground_m + kit.lo.y - s.drop_m <= floor {
                    s.burying = true;
                }
            }
            moved.push(lot);
            if l.ground_m + kit.hi.y - s.drop_m < floor - 0.5 {
                st.alive.iter_mut().for_each(|a| *a = false);
                st.version += 1;
                gone.push(lot);
            }
        }
        (moved, gone)
    }

    /// The standing piece of `lot` that holds world point `p`, if any.
    pub fn solid_at(&self, lots: &Lots, lot: u32, p: Vec3) -> Option<u32> {
        lots.lots.get(lot as usize)?;
        let kit = lots.kit(lot);
        let q = self.to_kit(lots, lot, p);
        if q.cmplt(kit.lo - Vec3::splat(0.1)).any() || q.cmpgt(kit.hi + Vec3::splat(0.1)).any() {
            return None;
        }
        let alive = self.alive(lot);
        kit.pieces
            .iter()
            .enumerate()
            .find(|(i, piece)| alive.is_none_or(|a| a[*i]) && piece.contains(q))
            .map(|(i, _)| i as u32)
    }

    /// Breaks every standing piece of `lot` within `radius_m` of world point `centre`, then whatever
    /// that leaves without support, then, when too little wall is left, everything above the floor.
    pub fn break_sphere(&mut self, lots: &Lots, lot: u32, centre: Vec3, radius_m: f32) -> Broken {
        let Some(l) = lots.lots.get(lot as usize) else {
            return Broken::default();
        };
        if !centre.is_finite() || !radius_m.is_finite() || radius_m <= 0.0 {
            return Broken::default();
        }
        let _ = l;
        let kit = lots.kit(lot);
        let c = self.to_kit(lots, lot, centre);
        let hit: Vec<usize> = kit
            .pieces
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let nearest = c.clamp(p.lo, p.hi);
                nearest.distance(c) <= radius_m
            })
            .map(|(i, _)| i)
            .collect();
        self.remove(lots, lot, &hit)
    }

    /// Removes `pieces` of `lot` and settles it (`break_sphere`'s second half).
    pub fn remove(&mut self, lots: &Lots, lot: u32, pieces: &[usize]) -> Broken {
        let kit = lots.kit(lot);
        let n = kit.pieces.len();
        let state = self.states.entry(lot).or_insert_with(|| HouseState {
            alive: vec![true; n],
            collapsed: false,
            version: 0,
            sinking: None,
        });
        let before = state.alive.clone();
        for &i in pieces {
            if i < n {
                state.alive[i] = false;
            }
        }
        if state.alive == before {
            // Nothing was touched: a whole building is left without a state.
            if state.version == 0 {
                self.states.remove(&lot);
            }
            return Broken::default();
        }
        // A ship holed at or under its waterline goes down, whole (it never collapses as a building does).
        if let Some(w) = kit.waterline_m
            && state.sinking.is_none()
            && pieces.iter().any(|&i| i < n && kit.pieces[i].lo.y < w + SINK_HOLED_M)
        {
            state.sinking = Some(Sinking::start(kit, lot));
        }
        let mut collapsed = false;
        let stands = standing(kit, &state.alive);
        for (a, s) in state.alive.iter_mut().zip(&stands) {
            *a = *a && *s;
        }
        let wall: f32 = kit
            .pieces
            .iter()
            .zip(&state.alive)
            .filter(|(_, a)| **a)
            .map(|(p, _)| p.wall_area_m2)
            .sum();
        if !state.collapsed && kit.waterline_m.is_none() && kit.wall_area_m2 > 0.0 && wall < kit.collapse_wall_share * kit.wall_area_m2 {
            collapsed = true;
            state.collapsed = true;
            for (p, a) in kit.pieces.iter().zip(state.alive.iter_mut()) {
                if p.lo.y >= COLLAPSE_FOOT_M || p.hi.y >= COLLAPSE_TOP_M {
                    *a = false;
                }
            }
        }
        state.version += 1;
        let gone: Vec<u32> = (0..n).filter(|&i| before[i] && !state.alive[i]).map(|i| i as u32).collect();
        let sinking = state.sinking;
        let chunks = gone
            .iter()
            .flat_map(|&i| chunks_of(kit, i as usize, lots, lot, collapsed, sinking.as_ref()))
            .collect();
        Broken {
            pieces: gone,
            chunks,
            collapsed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::Rect;
    use crate::world::small_buildings::kit::shipped;
    use crate::world::small_buildings::lots::{Lot, Zone};
    use glam::Vec2;
    use std::sync::Arc;

    fn one(id: &str) -> Lots {
        let kits = shipped();
        let kit = kits.index_of(id).unwrap() as u16;
        let lot = Lot {
            kit,
            centre: Vec2::new(100.0, 50.0),
            ground_m: 10.0,
            heading_rad: 0.7,
            rect: Rect::centred(Vec2::new(100.0, 50.0), Vec2::splat(30.0)),
            zone: Zone::Suburb,
        };
        let placed = crate::world::small_buildings::Placed {
            lots: vec![lot],
            ..Default::default()
        };
        Lots::new(Arc::new(kits), placed, 4096.0)
    }

    #[test]
    fn a_whole_building_stands_entire() {
        let kits = shipped();
        for kit in &kits.kits {
            let all = vec![true; kit.pieces.len()];
            let s = standing(kit, &all);
            let floating: Vec<usize> = (0..s.len()).filter(|&i| !s[i]).collect();
            assert!(floating.is_empty(), "{}: pieces {floating:?} stand on nothing", kit.id);
        }
    }

    #[test]
    fn a_piece_with_nothing_under_it_falls() {
        let lots = one("ranch");
        let mut h = Houses::default();
        let kit = lots.kit(0).clone();
        // Take every piece on the ground: nothing is left standing.
        let ground: Vec<usize> = (0..kit.pieces.len()).filter(|&i| kit.pieces[i].lo.y <= GROUND_Y_M).collect();
        let b = h.remove(&lots, 0, &ground);
        assert_eq!(b.pieces.len(), kit.pieces.len());
        assert!(h.alive(0).unwrap().iter().all(|a| !a));
    }

    #[test]
    fn a_shell_holes_a_house_and_a_hit_far_away_does_nothing() {
        let lots = one("ranch");
        let mut h = Houses::default();
        let far = h.break_sphere(&lots, 0, Vec3::new(500.0, 10.0, 500.0), 3.0);
        assert!(far.pieces.is_empty() && h.states.is_empty(), "a miss leaves no state");
        let front = lots.lots[0].point_to_world(Vec3::new(-2.0, 1.5, 4.7));
        let b = h.break_sphere(&lots, 0, front, 3.2);
        assert!(!b.pieces.is_empty() && !b.chunks.is_empty());
        assert!(!b.collapsed, "one shell does not bring a house down");
        assert!(h.alive(0).unwrap().iter().filter(|a| **a).count() > lots.kit(0).pieces.len() / 2);
    }

    #[test]
    fn a_light_blast_on_the_front_wall_holes_it_and_no_more() {
        // The stills' autocannon burst: 1 MJ at the ranch's front wall, a 2.9 m sphere.
        let lots = one("ranch");
        let mut h = Houses::default();
        let at = lots.lots[0].point_to_world(Vec3::new(-2.0, 1.6, 5.0));
        let b = h.break_sphere(&lots, 0, at, 2.2 * 1.3);
        let n = lots.kit(0).pieces.len();
        let gone = b.pieces.len();
        eprintln!("broke {gone} of {n}, collapsed {}", b.collapsed);
        assert!(!b.collapsed && gone < n / 4, "broke {gone} of {n}");
    }

    #[test]
    fn a_house_losing_half_its_walls_collapses() {
        let lots = one("bungalow");
        let mut h = Houses::default();
        let kit = lots.kit(0).clone();
        let mut walls: Vec<usize> = (0..kit.pieces.len()).filter(|&i| kit.pieces[i].wall_area_m2 > 0.0).collect();
        walls.truncate(walls.len() * 2 / 3);
        let b = h.remove(&lots, 0, &walls);
        assert!(b.collapsed);
        let alive = h.alive(0).unwrap();
        assert!(
            kit.pieces
                .iter()
                .zip(alive)
                .all(|(p, a)| !a || (p.hi.y < COLLAPSE_TOP_M && p.lo.y < COLLAPSE_FOOT_M))
        );
    }

    #[test]
    fn a_shot_finds_the_wall_it_hits() {
        let lots = one("colonial");
        let h = Houses::default();
        let l = &lots.lots[0];
        let kit = lots.kit(0);
        let wall = kit.pieces.iter().position(|p| p.wall_area_m2 > 0.0).unwrap();
        let p = kit.pieces[wall].solid_boxes().next().unwrap();
        let inside = l.point_to_world((p.0 + p.1) * 0.5);
        assert!(h.solid_at(&lots, 0, inside).is_some());
        assert_eq!(h.solid_at(&lots, 0, l.point_to_world(Vec3::new(0.0, 30.0, 0.0))), None);
    }
}
