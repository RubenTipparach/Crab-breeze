//! Floorplans and furniture (openspec change `building-interiors`), on a building's cell grid.
//!
//! A storey gets a core (stairs and lifts) in a cell every storey shares, a corridor through it
//! along the long axis (single-loaded along the back when the plan is under 13 m deep or too small
//! for a core), and rooms in the bands either side, cut on the 3 m facade bay lines so a partition
//! always meets a pier. Rooms are typed by the storey's use and furnished by rules: wall items with
//! a clearance strip in front, centre items, desk grids; nothing in a door zone or on a column, and
//! nothing tall against the facade. Every wall and item rests on the floor bay under its middle
//! (`Rest`), so it falls with it. Pure and deterministic.

use crate::rng::SimRng;
use glam::{Vec2, Vec3};

/// What a storey is used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    /// Shops on the street side, stockrooms behind.
    Retail,
    /// Offices: open plan, private offices, meeting rooms.
    Office,
    /// Flats: kitchen and bath on the corridor, living room and bedroom on the facade.
    Flats,
}

/// A room's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomKind {
    /// A shop.
    Shop,
    /// A stockroom.
    Stockroom,
    /// An open-plan office.
    OpenOffice,
    /// A private office.
    Office,
    /// A meeting room.
    Meeting,
    /// A flat's living room.
    Living,
    /// A flat's bedroom.
    Bedroom,
    /// A flat's kitchen.
    Kitchen,
    /// A flat's bathroom.
    Bath,
}

/// A room: a rectangle in (x, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    /// Least corner.
    pub lo: Vec2,
    /// Greatest corner.
    pub hi: Vec2,
    /// Its type.
    pub kind: RoomKind,
}

/// The floor bay something rests on: cell (i, j) at the top of storey `k - 1`, or the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rest {
    /// The ground (storey 0).
    Ground,
    /// The floor bay of cell (i, j) at the top of storey `k`.
    Slab {
        /// Storey whose ceiling it is.
        k: u32,
        /// Cell along x.
        i: u32,
        /// Cell along z.
        j: u32,
    },
}

/// A wall segment: a box, and what it rests on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Partition {
    /// Least corner.
    pub min: Vec3,
    /// Greatest corner.
    pub max: Vec3,
    /// True for the core's concrete walls.
    pub core: bool,
    /// What holds it up.
    pub rest: Rest,
}

/// A furniture item: boxes in world space, and what it rests on.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Its kit name.
    pub name: &'static str,
    /// Its boxes (least corner, greatest corner, linear colour).
    pub boxes: Vec<(Vec3, Vec3, [f32; 3])>,
    /// Its footprint with its clearance strip, (x, z) least and greatest.
    pub clear_lo: Vec2,
    /// Greatest corner of the footprint with clearance.
    pub clear_hi: Vec2,
    /// What holds it up.
    pub rest: Rest,
}

/// One storey's plan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StoreyPlan {
    /// Rooms.
    pub rooms: Vec<Room>,
    /// Corridor rectangles (x, z).
    pub corridor: Vec<(Vec2, Vec2)>,
    /// Door centres (x, z).
    pub doors: Vec<Vec2>,
    /// Wall segments, door gaps already cut.
    pub walls: Vec<Partition>,
    /// Furniture.
    pub items: Vec<Item>,
}

const PARTITION_M: f32 = 0.12;
const CORE_WALL_M: f32 = 0.25;
const CORRIDOR_M: f32 = 2.0;
const BAY_M: f32 = 3.0;
const DOOR_W_M: f32 = 1.0;
const DOOR_H_M: f32 = 2.1;

/// An axis-aligned plan built in (u, v): u along the long axis.
struct Frame2 {
    long_x: bool,
}

impl Frame2 {
    fn rect(&self, u0: f32, v0: f32, u1: f32, v1: f32) -> (Vec2, Vec2) {
        let (a, b) = (u0.min(u1), u0.max(u1));
        let (c, d) = (v0.min(v1), v0.max(v1));
        if self.long_x {
            (Vec2::new(a, c), Vec2::new(b, d))
        } else {
            (Vec2::new(c, a), Vec2::new(d, b))
        }
    }
    fn point(&self, u: f32, v: f32) -> Vec2 {
        if self.long_x { Vec2::new(u, v) } else { Vec2::new(v, u) }
    }
}

/// The use of storey `k` of a building of `storeys`: shops on the ground, flats in a walk-up,
/// offices above the ground otherwise.
pub fn use_of(k: usize, storeys: usize) -> Use {
    if k == 0 {
        Use::Retail
    } else if storeys <= 6 {
        Use::Flats
    } else {
        Use::Office
    }
}

/// Plans storey `k` of building `id`: `n` = [cells along x, storeys, cells along z], `origin` its
/// least corner, `size` one cell (as `destruct::Cells`). The street is taken to be on the -z side.
pub fn plan_storey(id: u32, k: usize, n: [usize; 3], origin: Vec3, size: Vec3, seed: &SimRng) -> StoreyPlan {
    let [nx, ns, nz] = n;
    let mut plan = StoreyPlan::default();
    if nx == 0 || nz == 0 || k >= ns {
        return plan;
    }
    let mut rng = seed.fork(&format!("building/{id}/plan/{k}"));
    let use_ = use_of(k, ns);
    let (x0, z0) = (origin.x, origin.z);
    let (x1, z1) = (origin.x + size.x * nx as f32, origin.z + size.z * nz as f32);
    let long_x = x1 - x0 >= z1 - z0;
    let f2 = Frame2 { long_x };
    let (u0, u1, v0, v1) = if long_x { (x0, x1, z0, z1) } else { (z0, z1, x0, x1) };
    let floor_y = origin.y + size.y * k as f32;
    let ceil_y = floor_y + size.y - 0.25;
    let rest_at = |p: Vec2| -> Rest {
        if k == 0 {
            Rest::Ground
        } else {
            Rest::Slab {
                k: k as u32 - 1,
                i: (((p.x - x0) / size.x) as u32).min(nx as u32 - 1),
                j: (((p.y - z0) / size.z) as u32).min(nz as u32 - 1),
            }
        }
    };
    let mut walls: Vec<(f32, f32, f32, f32, Option<f32>, bool)> = Vec::new(); // u0 v0 u1 v1 door core
    let core = (nx >= 3 && nz >= 3).then(|| {
        let (ci, cj) = ((nx - 1) / 2, (nz - 1) / 2);
        let (a, b) = (x0 + size.x * ci as f32, z0 + size.z * cj as f32);
        if long_x {
            (a, b, a + size.x, b + size.z)
        } else {
            (b, a, b + size.z, a + size.x)
        }
    });
    let cw = CORRIDOR_M;
    struct Band {
        u0: f32,
        u1: f32,
        va: f32,
        vb: f32,
        front: bool,
    }
    let mut bands: Vec<Band> = Vec::new();
    // The street is on -z; in (u, v) that is the low-v side when long along x, else the low-u end.
    let front_low_v = long_x;
    match core {
        Some((cu0, cv0, cu1, cv1)) if v1 - v0 >= 13.0 => {
            let vc = (cv0 + cv1) * 0.5;
            let (s0, s1) = (vc - cw * 0.5, vc + cw * 0.5);
            let (lu0, lu1) = (cu0 - cw, cu1 + cw);
            plan.corridor.push(f2.rect(u0, s0, lu0, s1));
            plan.corridor.push(f2.rect(lu1, s0, u1, s1));
            plan.corridor.push(f2.rect(lu0, cv0 - cw, lu1, cv0));
            plan.corridor.push(f2.rect(lu0, cv1, lu1, cv1 + cw));
            plan.corridor.push(f2.rect(lu0, cv0, cu0, cv1));
            plan.corridor.push(f2.rect(cu1, cv0, lu1, cv1));
            for (fv, near, edge) in [(v1, s1, cv1 + cw), (v0, s0, cv0 - cw)] {
                let front = (fv == v0) == front_low_v;
                if lu0 - u0 > 2.5 {
                    bands.push(Band {
                        u0,
                        u1: lu0,
                        va: near,
                        vb: fv,
                        front,
                    });
                }
                bands.push(Band {
                    u0: lu0,
                    u1: lu1,
                    va: edge,
                    vb: fv,
                    front,
                });
                if u1 - lu1 > 2.5 {
                    bands.push(Band {
                        u0: lu1,
                        u1,
                        va: near,
                        vb: fv,
                        front,
                    });
                }
            }
            // One wall where bands meet, from the corridor edge nearest the spine to the facade.
            for sgn in [1.0f32, -1.0] {
                let side: Vec<&Band> = bands.iter().filter(|b| (b.vb - b.va).signum() == sgn).collect();
                let mut us: Vec<f32> = side
                    .iter()
                    .flat_map(|b| [b.u0, b.u1])
                    .filter(|&u| u > u0 + 0.01 && u < u1 - 0.01)
                    .collect();
                us.sort_by(f32::total_cmp);
                us.dedup_by(|a, b| (*a - *b).abs() < 0.01);
                for u in us {
                    let touching: Vec<&&Band> = side.iter().filter(|b| (b.u0 - u).abs() < 0.01 || (b.u1 - u).abs() < 0.01).collect();
                    let va = touching
                        .iter()
                        .map(|b| b.va)
                        .min_by(|a, b| (a - vc).abs().total_cmp(&(b - vc).abs()))
                        .unwrap_or(touching[0].va);
                    walls.push((u, va, u, touching[0].vb, None, false));
                }
            }
            // The core's concrete walls, with lift and stair doors onto the far lane.
            walls.push((cu0, cv0, cu0, cv1, None, true));
            walls.push((cu1, cv0, cu1, cv1, None, true));
            walls.push((cu0, cv0, cu1, cv0, None, true));
            walls.push((cu0, cv1, cu1, cv1, Some((cu0 + cu1) * 0.5), true));
        }
        _ => {
            // Single-loaded: corridor along the back, stair at the end away from the street.
            let back_high = front_low_v;
            let (bv, sgn, fv) = if back_high { (v1, -1.0, v0) } else { (v0, 1.0, v1) };
            let cv1 = bv + sgn * cw;
            let sv = cv1 + sgn * 3.5;
            plan.corridor.push(f2.rect(u0, bv, u1, cv1));
            // The street is on -z: the far end of u is away from it either way.
            let (su0, su1) = (u1 - 5.5, u1);
            bands.push(Band {
                u0,
                u1: su0,
                va: cv1,
                vb: fv,
                front: true,
            });
            if (fv - sv).abs() > 2.5 {
                bands.push(Band {
                    u0: su0,
                    u1: su1,
                    va: sv,
                    vb: fv,
                    front: true,
                });
                walls.push((su0, sv, su0, fv, None, false));
            }
            walls.push((su0, cv1, su0, sv, None, true));
            walls.push((su0, sv, su1, sv, None, true));
        }
    }
    // Rooms on the bay lines.
    for b in &bands {
        let mut u = b.u0;
        let snap = |x: f32| b.u0 + (((x - b.u0) / BAY_M).round() * BAY_M).max(BAY_M);
        while b.u1 - u > 0.5 {
            let (kind, w) = match use_ {
                Use::Retail => (
                    if b.front { RoomKind::Shop } else { RoomKind::Stockroom },
                    [6.0, 9.0, 12.0][rng.next_u64() as usize % 3],
                ),
                Use::Flats => (RoomKind::Living, [6.0, 6.0, 9.0][rng.next_u64() as usize % 3]),
                Use::Office => {
                    let r = rng.next_f32();
                    if r < 0.45 {
                        (RoomKind::OpenOffice, [9.0, 12.0, 15.0][rng.next_u64() as usize % 3])
                    } else if r < 0.8 {
                        (RoomKind::Office, [3.0, 3.0, 6.0][rng.next_u64() as usize % 3])
                    } else {
                        (RoomKind::Meeting, 6.0)
                    }
                }
            };
            let mut ue = snap(u + w).min(b.u1);
            if b.u1 - ue < 2.9 {
                ue = b.u1;
            }
            let (lo, hi) = f2.rect(u, b.va, ue, b.vb);
            plan.rooms.push(Room { lo, hi, kind });
            // A flat narrower than a bath and a kitchen stays one room.
            let flat = kind == RoomKind::Living && ue - u >= 4.6;
            let kitchen_mid = u + 2.4 + (ue - u - 2.4) * 0.5;
            let door = if flat { kitchen_mid } else { u + (ue - u).min(2.4) * 0.5 };
            walls.push((u, b.va, ue, b.va, Some(door), false));
            if ue < b.u1 - 0.01 {
                walls.push((ue, b.va, ue, b.vb, None, false));
            }
            if flat {
                // A flat: kitchen and bath along the corridor, the living room (and a bedroom when
                // it is wide) on the facade. The kitchen is the hall: doors to the corridor, the
                // bath and the living room all open off it.
                let sg = (b.vb - b.va).signum();
                let vb = b.va + sg * 2.6;
                walls.push((u, vb, ue, vb, Some(kitchen_mid), false));
                walls.push((u + 2.4, b.va, u + 2.4, vb, Some((b.va + vb) * 0.5), false));
                let last = plan.rooms.len() - 1;
                let (bl, bh) = f2.rect(u, b.va, u + 2.4, vb);
                let (kl, kh) = f2.rect(u + 2.4, b.va, ue, vb);
                plan.rooms.push(Room {
                    lo: bl,
                    hi: bh,
                    kind: RoomKind::Bath,
                });
                plan.rooms.push(Room {
                    lo: kl,
                    hi: kh,
                    kind: RoomKind::Kitchen,
                });
                if ue - u >= 8.5 {
                    let us = u + (ue - u) * 0.55;
                    walls.push((us, vb, us, b.vb, Some((vb + b.vb) * 0.5), false));
                    let (ll, lh) = f2.rect(u, vb, us, b.vb);
                    let (rl, rh) = f2.rect(us, vb, ue, b.vb);
                    plan.rooms[last] = Room {
                        lo: ll,
                        hi: lh,
                        kind: RoomKind::Living,
                    };
                    plan.rooms.push(Room {
                        lo: rl,
                        hi: rh,
                        kind: RoomKind::Bedroom,
                    });
                } else {
                    let (ll, lh) = f2.rect(u, vb, ue, b.vb);
                    plan.rooms[last] = Room {
                        lo: ll,
                        hi: lh,
                        kind: RoomKind::Living,
                    };
                }
            }
            u = ue;
        }
    }
    // Walls to boxes, door gaps cut, ends tucked 2 cm into what they meet.
    for (wu0, wv0, wu1, wv1, door, core_wall) in walls {
        let along_u = (wv1 - wv0).abs() < 1e-4;
        let th = if core_wall { CORE_WALL_M } else { PARTITION_M };
        let (a0, a1, fixed) = if along_u {
            (wu0.min(wu1), wu0.max(wu1), wv0)
        } else {
            (wv0.min(wv1), wv0.max(wv1), wu0)
        };
        let mut spans = vec![(a0, a1, floor_y - 0.02, ceil_y + 0.03)];
        if let Some(d) = door {
            let d0 = (d - DOOR_W_M * 0.5).clamp(a0 + 0.1, (a1 - DOOR_W_M - 0.1).max(a0 + 0.1));
            spans = vec![
                (a0, d0, floor_y - 0.02, ceil_y + 0.03),
                (d0 + DOOR_W_M, a1, floor_y - 0.02, ceil_y + 0.03),
                (d0, d0 + DOOR_W_M, floor_y + DOOR_H_M, ceil_y + 0.03),
            ];
            let dp = if along_u {
                f2.point(d0 + DOOR_W_M * 0.5, fixed)
            } else {
                f2.point(fixed, d0 + DOOR_W_M * 0.5)
            };
            plan.doors.push(dp);
        }
        for (s0, s1, y0, y1) in spans {
            if s1 - s0 < 0.05 {
                continue;
            }
            let (lo, hi) = if along_u {
                f2.rect(s0, fixed - th * 0.5, s1, fixed + th * 0.5)
            } else {
                f2.rect(fixed - th * 0.5, s0, fixed + th * 0.5, s1)
            };
            plan.walls.push(Partition {
                min: Vec3::new(lo.x, y0, lo.y),
                max: Vec3::new(hi.x, y1, hi.y),
                core: core_wall,
                rest: rest_at((lo + hi) * 0.5),
            });
        }
    }
    furnish(&mut plan, floor_y, [x0, z0, x1, z1], n, origin, size, &mut rng, &rest_at);
    plan
}

/// A kit item: boxes in its own frame (origin at its back edge on the floor, facing +z), as
/// (centre x, centre y, centre z, size x, size y, size z, colour).
type KitBox = (f32, f32, f32, f32, f32, f32, [f32; 3]);
const WOOD: [f32; 3] = [0.42, 0.27, 0.16];
const FABRIC: [f32; 3] = [0.38, 0.30, 0.32];
const BEIGE: [f32; 3] = [0.80, 0.76, 0.64];
const STEEL: [f32; 3] = [0.30, 0.32, 0.34];
const WHITE: [f32; 3] = [0.90, 0.89, 0.86];

fn kit(name: &str) -> (&'static [KitBox], bool, f32) {
    // (boxes, tall, clearance in front)
    match name {
        "desk" => (
            &[
                (0.0, 0.73, 0.375, 1.5, 0.04, 0.75, WOOD),
                (-0.55, 0.36, 0.375, 0.4, 0.71, 0.7, WOOD),
                (0.1, 0.95, 0.25, 0.42, 0.38, 0.42, BEIGE),
                (0.0, 0.45, 1.05, 0.5, 0.08, 0.5, FABRIC),
            ],
            false,
            0.6,
        ),
        "filing" => (&[(0.0, 0.66, 0.3, 0.47, 1.32, 0.6, [0.44, 0.47, 0.42])], true, 0.7),
        "shelf" => (&[(0.0, 1.0, 0.17, 0.9, 2.0, 0.34, WOOD)], true, 0.5),
        "sofa" => (
            &[(0.0, 0.22, 0.45, 2.0, 0.44, 0.9, FABRIC), (0.0, 0.62, 0.1, 2.0, 0.5, 0.2, FABRIC)],
            false,
            0.9,
        ),
        "tv" => (
            &[
                (0.0, 0.3, 0.25, 1.0, 0.6, 0.45, WOOD),
                (0.0, 0.86, 0.25, 0.66, 0.52, 0.5, [0.15, 0.15, 0.16]),
            ],
            false,
            1.6,
        ),
        "bed" => (
            &[
                (0.0, 0.25, 1.0, 1.5, 0.5, 2.0, WOOD),
                (0.0, 0.56, 1.05, 1.42, 0.2, 1.9, WHITE),
                (0.0, 0.6, 0.03, 1.6, 1.2, 0.06, WOOD),
            ],
            false,
            0.6,
        ),
        "wardrobe" => (&[(0.0, 1.0, 0.3, 1.2, 2.0, 0.6, WOOD)], true, 0.8),
        "counter" => (
            &[
                (0.0, 0.45, 0.3, 2.4, 0.9, 0.6, [0.66, 0.6, 0.44]),
                (0.0, 0.92, 0.31, 2.42, 0.04, 0.62, WHITE),
            ],
            false,
            0.9,
        ),
        "fridge" => (&[(0.0, 0.9, 0.35, 0.7, 1.8, 0.7, [0.86, 0.82, 0.68])], true, 0.8),
        "bath" => (&[(0.0, 0.28, 0.38, 1.6, 0.56, 0.75, WHITE)], false, 0.5),
        "meeting" => (
            &[(0.0, 0.73, 0.6, 3.6, 0.05, 1.2, WOOD), (0.0, 0.36, 0.6, 0.3, 0.71, 0.6, WOOD)],
            false,
            0.9,
        ),
        "gondola" => (&[(0.0, 0.75, 0.45, 3.0, 1.5, 0.9, [0.78, 0.78, 0.76])], true, 1.0),
        "till" => (
            &[(0.0, 0.5, 0.3, 2.4, 1.0, 0.6, WOOD), (-0.6, 1.1, 0.3, 0.45, 0.2, 0.4, STEEL)],
            false,
            1.0,
        ),
        "crates" => (&[(0.0, 0.6, 0.6, 1.2, 1.2, 1.2, [0.62, 0.48, 0.3])], false, 0.5),
        _ => (
            &[
                (0.0, 0.2, 0.2, 0.35, 0.4, 0.35, [0.5, 0.33, 0.22]),
                (0.0, 0.85, 0.2, 0.6, 0.9, 0.6, [0.27, 0.42, 0.2]),
            ],
            false,
            0.2,
        ), // plant
    }
}

fn menu(kind: RoomKind) -> &'static [(&'static str, u32, u32)] {
    match kind {
        RoomKind::Office => &[("desk", 1, 1), ("filing", 1, 2), ("shelf", 0, 1), ("plant", 0, 1)],
        RoomKind::OpenOffice => &[("desk", 4, 10), ("filing", 2, 4), ("plant", 1, 2)],
        RoomKind::Meeting => &[("meeting", 1, 1), ("shelf", 0, 1)],
        RoomKind::Living => &[("sofa", 1, 1), ("tv", 1, 1), ("shelf", 0, 2), ("plant", 0, 1)],
        RoomKind::Bedroom => &[("bed", 1, 1), ("wardrobe", 1, 1)],
        RoomKind::Kitchen => &[("counter", 1, 1), ("fridge", 1, 1)],
        RoomKind::Bath => &[("bath", 1, 1)],
        RoomKind::Shop => &[("till", 1, 1), ("gondola", 1, 3), ("shelf", 1, 2)],
        RoomKind::Stockroom => &[("shelf", 2, 4), ("crates", 1, 3)],
    }
}

#[allow(clippy::too_many_arguments)]
fn furnish(
    plan: &mut StoreyPlan,
    floor_y: f32,
    outer: [f32; 4],
    n: [usize; 3],
    origin: Vec3,
    size: Vec3,
    rng: &mut SimRng,
    rest_at: &dyn Fn(Vec2) -> Rest,
) {
    let door_zones: Vec<(Vec2, Vec2)> = plan.doors.iter().map(|d| (*d - Vec2::splat(0.9), *d + Vec2::splat(0.9))).collect();
    let mut columns = Vec::new();
    for a in 0..=n[0] {
        for c in 0..=n[2] {
            let p = Vec2::new(origin.x + size.x * a as f32, origin.z + size.z * c as f32);
            columns.push((p - Vec2::splat(0.45), p + Vec2::splat(0.45)));
        }
    }
    let overlaps = |a: (Vec2, Vec2), b: &(Vec2, Vec2)| a.0.x < b.1.x && a.1.x > b.0.x && a.0.y < b.1.y && a.1.y > b.0.y;
    let rooms = plan.rooms.clone();
    for room in rooms {
        let on = |v: f32, edge: f32| (v - edge).abs() < 0.01;
        let inset = |v: f32, edge: f32| if on(v, edge) { 0.32 } else { 0.1 };
        let lo = Vec2::new(room.lo.x + inset(room.lo.x, outer[0]), room.lo.y + inset(room.lo.y, outer[1]));
        let hi = Vec2::new(room.hi.x - inset(room.hi.x, outer[2]), room.hi.y - inset(room.hi.y, outer[3]));
        if hi.x - lo.x < 1.0 || hi.y - lo.y < 1.0 {
            continue;
        }
        // Which walls of the room are facade: -z, +x, +z, -x.
        let facade = [
            on(room.lo.y, outer[1]),
            on(room.hi.x, outer[2]),
            on(room.hi.y, outer[3]),
            on(room.lo.x, outer[0]),
        ];
        let mut placed: Vec<(Vec2, Vec2)> = Vec::new();
        for &(name, lo_n, hi_n) in menu(room.kind) {
            let (boxes, tall, front) = kit(name);
            let count = lo_n + (rng.next_u64() % (hi_n - lo_n + 1) as u64) as u32;
            let half_w = boxes.iter().map(|b| b.0.abs() + b.3 * 0.5).fold(0.0, f32::max);
            let depth = boxes.iter().map(|b| b.2 + b.5 * 0.5).fold(0.0, f32::max);
            for _ in 0..count {
                for _ in 0..30 {
                    let side = (rng.next_u64() % 4) as usize;
                    if tall && facade[side] {
                        continue;
                    }
                    let t = rng.next_f32();
                    // Position of the item's back-centre and its facing (+z rotated by `rot` quarter turns).
                    let (p, rot) = match side {
                        0 => (Vec2::new(lo.x + t * (hi.x - lo.x), lo.y), 0u8),
                        2 => (Vec2::new(lo.x + t * (hi.x - lo.x), hi.y), 2),
                        3 => (Vec2::new(lo.x, lo.y + t * (hi.y - lo.y)), 1),
                        _ => (Vec2::new(hi.x, lo.y + t * (hi.y - lo.y)), 3),
                    };
                    let turn = |lx: f32, lz: f32| -> Vec2 {
                        match rot {
                            0 => Vec2::new(lx, lz),
                            1 => Vec2::new(lz, -lx),
                            2 => Vec2::new(-lx, -lz),
                            _ => Vec2::new(-lz, lx),
                        }
                    };
                    let foot = |extra: f32| -> (Vec2, Vec2) {
                        let a = p + turn(-half_w, 0.0);
                        let b = p + turn(half_w, depth + extra);
                        (a.min(b), a.max(b))
                    };
                    let body = foot(0.0);
                    let clear = foot(front);
                    if body.0.x < lo.x - 0.01 || body.0.y < lo.y - 0.01 || body.1.x > hi.x + 0.01 || body.1.y > hi.y + 0.01 {
                        continue;
                    }
                    if door_zones
                        .iter()
                        .chain(columns.iter())
                        .chain(placed.iter())
                        .any(|z| overlaps(clear, z))
                    {
                        continue;
                    }
                    placed.push(clear);
                    let world: Vec<(Vec3, Vec3, [f32; 3])> = boxes
                        .iter()
                        .map(|&(cx, cy, cz, sx, sy, sz, col)| {
                            let c = p + turn(cx, cz);
                            let half = if rot % 2 == 0 {
                                Vec2::new(sx, sz) * 0.5
                            } else {
                                Vec2::new(sz, sx) * 0.5
                            };
                            (
                                Vec3::new(c.x - half.x, floor_y + cy - sy * 0.5 + 0.002, c.y - half.y),
                                Vec3::new(c.x + half.x, floor_y + cy + sy * 0.5 + 0.002, c.y + half.y),
                                col,
                            )
                        })
                        .collect();
                    plan.items.push(Item {
                        name,
                        boxes: world,
                        clear_lo: clear.0,
                        clear_hi: clear.1,
                        rest: rest_at((body.0 + body.1) * 0.5),
                    });
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storey(k: usize, n: [usize; 3]) -> StoreyPlan {
        plan_storey(11, k, n, Vec3::new(10.0, 5.0, 20.0), Vec3::new(6.0, 3.5, 6.0), &SimRng::new(42))
    }

    #[test]
    fn the_same_storey_is_planned_the_same() {
        assert_eq!(storey(3, [5, 12, 4]), storey(3, [5, 12, 4]));
    }

    #[test]
    fn every_room_has_a_door() {
        for (k, n) in [(0, [5, 12, 4]), (4, [5, 12, 4]), (2, [3, 5, 5]), (1, [2, 4, 5])] {
            let p = storey(k, n);
            assert!(!p.rooms.is_empty(), "storey {k} of {n:?} has rooms");
            for r in &p.rooms {
                let near = |d: &Vec2| d.x >= r.lo.x - 0.2 && d.x <= r.hi.x + 0.2 && d.y >= r.lo.y - 0.2 && d.y <= r.hi.y + 0.2;
                assert!(
                    p.doors.iter().any(near),
                    "{:?} at {:?}..{:?} on storey {k} of {n:?} has a door",
                    r.kind,
                    r.lo,
                    r.hi
                );
            }
        }
    }

    #[test]
    fn no_item_blocks_a_door() {
        for (k, n) in [(0, [5, 12, 4]), (6, [5, 12, 4]), (3, [3, 5, 5])] {
            let p = storey(k, n);
            assert!(!p.items.is_empty());
            for it in &p.items {
                for d in &p.doors {
                    let inside =
                        d.x > it.clear_lo.x + 0.9 && d.x < it.clear_hi.x - 0.9 && d.y > it.clear_lo.y + 0.9 && d.y < it.clear_hi.y - 0.9;
                    assert!(!inside, "{} blocks the door at {d:?}", it.name);
                }
            }
        }
    }

    #[test]
    fn walls_and_items_rest_on_the_floor_below() {
        let p = storey(4, [5, 12, 4]);
        for w in &p.walls {
            assert!(matches!(w.rest, Rest::Slab { k: 3, .. }));
        }
        let g = storey(0, [5, 12, 4]);
        assert!(g.walls.iter().all(|w| w.rest == Rest::Ground));
    }

    #[test]
    fn walk_ups_hold_flats_and_towers_hold_offices() {
        assert_eq!(use_of(0, 5), Use::Retail);
        assert_eq!(use_of(2, 5), Use::Flats);
        assert_eq!(use_of(2, 20), Use::Office);
        let flats = storey(2, [3, 5, 5]);
        assert!(flats.rooms.iter().any(|r| r.kind == RoomKind::Kitchen) && flats.rooms.iter().any(|r| r.kind == RoomKind::Bath));
    }
}
