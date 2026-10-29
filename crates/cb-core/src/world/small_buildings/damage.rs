//! Houses and small businesses breaking (openspec change `small-buildings`, design sections 6 and 7.6):
//! the kit's support and collapse rules, ported from the mockup (`standing` in
//! `docs/mockups/small_buildings_kit.js`). A piece stands while it is reached from the ground through
//! `up` links and at most `max_side_steps` `side` links; a building whose standing wall area falls
//! below `collapse_wall_share` of its whole collapses above its floor. What breaks becomes chunks for
//! `debris-physics`. Only damaged buildings have a state; every other one is whole.

use super::kit::Kit;
use super::lots::Lots;
use crate::world::destruct::Chunk;
use glam::{Affine3A, Vec3};
use std::collections::{BTreeMap, VecDeque};

/// A chunk's `building` for a house: this plus the lot id (city building ids stay below it).
pub const HOUSE_CHUNK_BASE: u32 = 1 << 30;

/// How a broken house's pieces become chunks (`structure:` in `data/world/world.ron`: `house_chunk_m`,
/// `house_collapse_chunk_m`, `house_dust_m3`). Every chunk is a physics body near the pilot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chunking {
    /// A broken box is cut into chunks about this across, metres...
    pub chunk_m: f32,
    /// ...and about this in a collapse.
    pub collapse_chunk_m: f32,
    /// A piece of less volume than this turns to dust, cubic metres.
    pub dust_m3: f32,
}

impl Default for Chunking {
    fn default() -> Self {
        let c = crate::world::structure::StructureConfig::default();
        Self::of(&c)
    }
}

impl Chunking {
    /// The structure tuning's.
    pub fn of(c: &crate::world::structure::StructureConfig) -> Self {
        Self {
            chunk_m: c.house_chunk_m,
            collapse_chunk_m: c.house_collapse_chunk_m,
            dust_m3: c.house_dust_m3,
        }
    }
}

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
    pub fn transform(&self, kit: &Kit) -> Affine3A {
        let c = Vec3::new((kit.lo.x + kit.hi.x) * 0.5, 0.0, (kit.lo.z + kit.hi.z) * 0.5);
        let axis = if self.along_x { Vec3::X } else { Vec3::Z };
        Affine3A::from_translation(c - Vec3::Y * self.drop_m)
            * Affine3A::from_axis_angle(axis, self.tilt_rad)
            * Affine3A::from_translation(-c)
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

/// The chunks a piece of `kit` becomes, `pose` taking its frame to the world (its lot's or its ship's, with a sinking
/// ship's motion), `collapse` making them coarser, `sizes` how coarse.
fn chunks_of(kit: &Kit, piece: usize, pose: Affine3A, building: u32, collapse: bool, sizes: &Chunking) -> Vec<Chunk> {
    use super::kit::Chunking as Kind;
    let p = &kit.pieces[piece];
    let size = p.hi - p.lo;
    if size.min_element() < DUST_M || size.x * size.y * size.z < sizes.dust_m3 {
        return Vec::new();
    }
    // The world size of a box turned by the pose (its bounds).
    let m = pose.matrix3;
    let turned = |s: Vec3| -> Vec3 { (m.x_axis.abs() * s.x + m.y_axis.abs() * s.y + m.z_axis.abs() * s.z).into() };

    match &p.chunking {
        Kind::Box { .. } => {
            let step = if collapse { sizes.collapse_chunk_m } else { sizes.chunk_m };
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
                            centre: pose.transform_point3(c),
                            size: turned(cell),
                        });
                    }
                }
            }
            out
        }
        Kind::Panel { .. } | Kind::Whole => vec![Chunk {
            building,
            centre: pose.transform_point3((p.lo + p.hi) * 0.5),
            size: turned(size.min(Vec3::splat(4.0))),
        }],
    }
}

/// One breakable kit somewhere in the world: a house's lot, a site's building, a ship at sea. `pose` takes the kit's
/// frame to the world before a sinking ship's own motion; `parity` picks a ship's lean; `building` names its chunks.
/// The one place a kit breaks, settles, collapses and sinks: `Houses` for the lots, `ships::Boat` for the ships under
/// way (owner, 2026-10-29: "make sure this damage system works on moving ships too").
pub struct Breakable<'a> {
    /// The kit.
    pub kit: &'a Kit,
    /// Its frame in the world.
    pub pose: Affine3A,
    /// Which way a ship leans going down.
    pub parity: u32,
    /// Its chunks' `building`.
    pub building: u32,
    /// How its broken pieces become chunks.
    pub sizes: Chunking,
}

impl Breakable<'_> {
    /// Its frame in the world now, a sinking ship's motion included.
    pub fn pose_now(&self, state: Option<&HouseState>) -> Affine3A {
        match state.and_then(|s| s.sinking.as_ref()) {
            Some(s) => self.pose * s.transform(self.kit),
            None => self.pose,
        }
    }

    /// The standing piece holding world point `p`, if any.
    pub fn solid_at(&self, state: Option<&HouseState>, p: Vec3) -> Option<u32> {
        let q = self.pose_now(state).inverse().transform_point3(p);
        let kit = self.kit;
        if q.cmplt(kit.lo - Vec3::splat(0.1)).any() || q.cmpgt(kit.hi + Vec3::splat(0.1)).any() {
            return None;
        }
        let alive = state.map(|s| s.alive.as_slice());
        kit.pieces
            .iter()
            .enumerate()
            .find(|(i, piece)| alive.is_none_or(|a| a[*i]) && piece.contains(q))
            .map(|(i, _)| i as u32)
    }

    /// Breaks every standing piece within `radius_m` of world point `centre`, then what that leaves without support,
    /// then, when too little wall is left, everything above the floor; a ship holed at its waterline goes down.
    pub fn break_sphere(&self, state: &mut Option<HouseState>, centre: Vec3, radius_m: f32) -> Broken {
        if !centre.is_finite() || !radius_m.is_finite() || radius_m <= 0.0 {
            return Broken::default();
        }
        let c = self.pose_now(state.as_ref()).inverse().transform_point3(centre);
        let hit: Vec<usize> = self
            .kit
            .pieces
            .iter()
            .enumerate()
            .filter(|(_, p)| c.clamp(p.lo, p.hi).distance(c) <= radius_m)
            .map(|(i, _)| i)
            .collect();
        self.remove(state, &hit)
    }

    /// Removes `pieces` and settles it (`break_sphere`'s second half). A state stays `None` while nothing is broken.
    pub fn remove(&self, slot: &mut Option<HouseState>, pieces: &[usize]) -> Broken {
        let kit = self.kit;
        let n = kit.pieces.len();
        let state = slot.get_or_insert_with(|| HouseState {
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
                *slot = None;
            }
            return Broken::default();
        }
        let mut collapsed = false;
        let stands = standing(kit, &state.alive);
        for (a, s) in state.alive.iter_mut().zip(&stands) {
            *a = *a && *s;
        }
        // A ship holed at or under its waterline goes down, whole (it never collapses as a building does): at the first
        // hole, or once its kit's `sink_share` of that hull is gone (a tanker or a freighter takes a quarter).
        if state.sinking.is_none()
            && let Some(share) = holed_share(kit, &state.alive)
            && share > 0.0
            && share >= kit.sink_share
        {
            state.sinking = Some(Sinking::start(kit, self.parity));
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
        let pose = self.pose_now(Some(state));
        let chunks = gone
            .iter()
            .flat_map(|&i| chunks_of(kit, i as usize, pose, self.building, collapsed, &self.sizes))
            .collect();
        Broken {
            pieces: gone,
            chunks,
            collapsed,
        }
    }

    /// Starts a ship going down whole now (sunk by its hit points or swamped, not holed piece by piece): its state
    /// made if it had none. False when it is no ship or already going down.
    pub fn sink(&self, slot: &mut Option<HouseState>) -> bool {
        if self.kit.waterline_m.is_none() {
            return false;
        }
        let n = self.kit.pieces.len();
        let state = slot.get_or_insert_with(|| HouseState {
            alive: vec![true; n],
            collapsed: false,
            version: 0,
            sinking: None,
        });
        if state.sinking.is_some() {
            return false;
        }
        state.sinking = Some(Sinking::start(self.kit, self.parity));
        state.version += 1;
        true
    }

    /// Takes a sinking ship down by `dt` seconds over a seabed at `floor_m` (its keel's world height as placed is the
    /// pose's): it gathers speed to its top, leans to its lean, settles into the seabed once its keel meets it, and once
    /// it is under, every piece is gone. True when it moved; `state.alive` all false once it is gone.
    pub fn step_sinking(&self, state: &mut HouseState, floor_m: f32, dt: f32) -> bool {
        let Some(s) = state.sinking.as_mut() else { return false };
        if !dt.is_finite() || dt <= 0.0 || state.alive.iter().all(|a| !a) {
            return false;
        }
        let keel = self.pose.translation.y + self.kit.lo.y;
        if s.burying {
            s.drop_m += s.top_m_s * SINK_BURY * dt;
        } else {
            s.speed_m_s = (s.speed_m_s + SINK_ACCEL_M_S2 * dt).min(s.top_m_s);
            s.drop_m += s.speed_m_s * dt;
            s.tilt_rad += (s.tilt_to_rad - s.tilt_rad) * (dt * 0.15).min(1.0);
            if keel - s.drop_m <= floor_m {
                s.burying = true;
            }
        }
        if self.pose.translation.y + self.kit.hi.y - s.drop_m < floor_m - 0.5 {
            state.alive.iter_mut().for_each(|a| *a = false);
            state.version += 1;
        }
        true
    }
}

/// The share of a ship's hull at and under its waterline (pieces whose foot is under `SINK_HOLED_M` over it) that is
/// gone, by volume; `None` for a kit that does not float.
pub fn holed_share(kit: &Kit, alive: &[bool]) -> Option<f32> {
    let w = kit.waterline_m?;
    let (mut all, mut gone) = (0.0f32, 0.0f32);
    for (p, a) in kit.pieces.iter().zip(alive) {
        if p.lo.y < w + SINK_HOLED_M {
            let v = (p.hi - p.lo).max(Vec3::ZERO).element_product();
            all += v;
            if !*a {
                gone += v;
            }
        }
    }
    Some(if all > 0.0 { gone / all } else { 0.0 })
}

/// A lot's kit frame in the world: its centre at its ground, turned by its heading.
pub fn lot_pose(l: &super::lots::Lot) -> Affine3A {
    Affine3A::from_rotation_translation(
        glam::Quat::from_rotation_y(l.heading_rad),
        Vec3::new(l.centre.x, l.ground_m, l.centre.y),
    )
}

/// Every damaged house and small business.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Houses {
    /// By lot id.
    pub states: BTreeMap<u32, HouseState>,
    /// How broken pieces become chunks.
    pub chunking: Chunking,
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

    /// `lot` as a breakable kit.
    fn breakable<'a>(lots: &'a Lots, lot: u32, sizes: Chunking) -> Breakable<'a> {
        Breakable {
            kit: lots.kit(lot),
            pose: lot_pose(&lots.lots[lot as usize]),
            parity: lot,
            building: HOUSE_CHUNK_BASE + lot,
            sizes,
        }
    }

    /// A sinking ship's motion in its kit's frame (`Sinking::transform`), if `lot` is one.
    pub fn sink_transform(&self, lots: &Lots, lot: u32) -> Option<Affine3A> {
        let s = self.states.get(&lot)?.sinking.as_ref()?;
        Some(s.transform(lots.kit(lot)))
    }

    /// Takes every sinking ship down by `dt` seconds over the ground `seabed` (x, z to its height)
    /// (`Breakable::step_sinking`). Returns the lots that moved, and of them those just gone.
    pub fn step_sinking(&mut self, lots: &Lots, dt: f32, seabed: &dyn Fn(glam::Vec2) -> f32) -> (Vec<u32>, Vec<u32>) {
        let (mut moved, mut gone) = (Vec::new(), Vec::new());
        let sizes = self.chunking;
        for (&lot, st) in self.states.iter_mut() {
            let was = st.alive.iter().any(|a| *a);
            if Self::breakable(lots, lot, sizes).step_sinking(st, seabed(lots.lots[lot as usize].centre), dt) {
                moved.push(lot);
                if was && st.alive.iter().all(|a| !a) {
                    gone.push(lot);
                }
            }
        }
        (moved, gone)
    }

    /// The standing piece of `lot` that holds world point `p`, if any.
    pub fn solid_at(&self, lots: &Lots, lot: u32, p: Vec3) -> Option<u32> {
        lots.lots.get(lot as usize)?;
        Self::breakable(lots, lot, self.chunking).solid_at(self.states.get(&lot), p)
    }

    /// Breaks every standing piece of `lot` within `radius_m` of world point `centre` (`Breakable::break_sphere`).
    pub fn break_sphere(&mut self, lots: &Lots, lot: u32, centre: Vec3, radius_m: f32) -> Broken {
        if lots.lots.get(lot as usize).is_none() {
            return Broken::default();
        }
        let mut slot = self.states.remove(&lot);
        let b = Self::breakable(lots, lot, self.chunking).break_sphere(&mut slot, centre, radius_m);
        if let Some(st) = slot {
            self.states.insert(lot, st);
        }
        b
    }

    /// Removes `pieces` of `lot` and settles it (`Breakable::remove`).
    pub fn remove(&mut self, lots: &Lots, lot: u32, pieces: &[usize]) -> Broken {
        let mut slot = self.states.remove(&lot);
        let b = Self::breakable(lots, lot, self.chunking).remove(&mut slot, pieces);
        if let Some(st) = slot {
            self.states.insert(lot, st);
        }
        b
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

    /// Measurement instrument (`city-frame` section 6): the chunks each shipped type gives when it is brought down
    /// whole (every load-bearing wall removed), for a few chunk sizes and dust volumes, and in all.
    /// `cargo test -p cb-core chunk_census -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn chunk_census() {
        let ids: Vec<String> = shipped().kits.iter().map(|k| k.id.clone()).collect();
        for collapse_chunk_m in [2.2, 3.0, 4.0] {
            for dust_m3 in [0.0, 0.05, 0.1, 0.25, 0.5] {
                let sizes = Chunking {
                    chunk_m: 1.5,
                    collapse_chunk_m,
                    dust_m3,
                };
                let mut counts = Vec::new();
                for id in &ids {
                    let lots = one(id);
                    let kit = lots.kit(0).clone();
                    let walls: Vec<usize> = (0..kit.pieces.len()).filter(|&i| kit.pieces[i].wall_area_m2 > 0.0).collect();
                    let mut h = Houses {
                        chunking: sizes,
                        ..Default::default()
                    };
                    let b = h.remove(&lots, 0, &walls);
                    if b.collapsed {
                        counts.push(b.chunks.len());
                    }
                }
                let total: usize = counts.iter().sum();
                eprintln!(
                    "collapse {collapse_chunk_m} m, dust {dust_m3} m3: {} types, {} chunks a house on average, {} to {}",
                    counts.len(),
                    total / counts.len().max(1),
                    counts.iter().min().unwrap_or(&0),
                    counts.iter().max().unwrap_or(&0)
                );
            }
        }
    }
}
