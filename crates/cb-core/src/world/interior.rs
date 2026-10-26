//! Floorplans, furniture, the street entrance and the stair (openspec change `building-interiors`),
//! on a building's cell grid.
//!
//! A storey gets a core (stairs and lifts) in a cell every storey shares, a corridor through it
//! along the long axis (single-loaded along the back when the plan is under 13 m deep or too small
//! for a core), and rooms in the bands either side, cut on the 3 m facade bay lines so a partition
//! always meets a pier. Rooms are typed by the storey's use and furnished by rules: wall items with
//! a clearance strip in front, centre items, desk grids; nothing in a door zone or on a column, and
//! nothing tall against the facade. Every wall and item rests on the floor bay under its middle
//! (`Rest`), so it falls with it.
//!
//! Every building has one street entrance (`Entrance`) on the facade that faces the street, leading
//! into the corridor or a lobby lined up with it, and on to a dog-leg stair (`Stair`) from the
//! ground to the top storey (owner, 2026-10-25: "each building needs doors and staairs that i as a
//! player can walk into them"). Pure and deterministic.

use super::dressing::{self, Style};
use super::layout::Rect;
use crate::rng::SimRng;
use glam::{Vec2, Vec3};

/// Interior tuning, from the `interiors:` section of `data/world/world.ron`.
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct InteriorConfig {
    /// Highest riser a stair may have.
    pub stair_max_rise_m: f32,
    /// Deepest tread (the run used when the core has room).
    pub stair_max_run_m: f32,
    /// Shallowest tread a stair may have; a core too short for it is a bug the tests catch.
    pub stair_min_run_m: f32,
    /// Width of one flight.
    pub stair_width_m: f32,
    /// Depth of the landing at the stair's door, on every storey.
    pub stair_landing_m: f32,
    /// Least depth of the half landing between the flights.
    pub stair_half_landing_m: f32,
    /// A walk-up's front door.
    pub front_door_w_m: f32,
    /// An office or tower's pair of lobby doors.
    pub lobby_door_w_m: f32,
    /// A warehouse's personnel door.
    pub personnel_door_w_m: f32,
    /// Clear height of the street door above its sill.
    pub door_clear_h_m: f32,
    /// A person steps onto anything this far above their feet, or lower (a tread, a kerb, a stoop).
    pub step_m: f32,
    /// A person's height for collision: anything starting below this above the feet is in the way.
    pub head_m: f32,
    /// On foot, buildings within this of the pilot get their detailed colliders (owner, 2026-10-26).
    pub collision_radius_m: f32,
    /// ...and those within this get them in the same step, not on a worker thread.
    pub collision_sync_m: f32,
    /// ...and they are freed past the radius plus this margin.
    pub collision_margin_m: f32,
}

impl Default for InteriorConfig {
    fn default() -> Self {
        Self {
            stair_max_rise_m: 0.18,
            stair_max_run_m: 0.28,
            stair_min_run_m: 0.22,
            stair_width_m: 1.2,
            stair_landing_m: 1.3,
            stair_half_landing_m: 1.1,
            front_door_w_m: 1.2,
            lobby_door_w_m: 1.9,
            personnel_door_w_m: 1.0,
            door_clear_h_m: 2.2,
            step_m: 0.45,
            head_m: 1.8,
            collision_radius_m: 60.0,
            collision_sync_m: 12.0,
            collision_margin_m: 20.0,
        }
    }
}

impl crate::data::Validate for InteriorConfig {
    fn validate(&self) -> Result<(), String> {
        use crate::data::positive;
        for (name, v) in [
            ("interiors.stair_max_rise_m", self.stair_max_rise_m),
            ("interiors.stair_max_run_m", self.stair_max_run_m),
            ("interiors.stair_min_run_m", self.stair_min_run_m),
            ("interiors.stair_width_m", self.stair_width_m),
            ("interiors.stair_landing_m", self.stair_landing_m),
            ("interiors.stair_half_landing_m", self.stair_half_landing_m),
            ("interiors.front_door_w_m", self.front_door_w_m),
            ("interiors.lobby_door_w_m", self.lobby_door_w_m),
            ("interiors.personnel_door_w_m", self.personnel_door_w_m),
            ("interiors.door_clear_h_m", self.door_clear_h_m),
            ("interiors.step_m", self.step_m),
            ("interiors.head_m", self.head_m),
            ("interiors.collision_radius_m", self.collision_radius_m),
            ("interiors.collision_sync_m", self.collision_sync_m),
            ("interiors.collision_margin_m", self.collision_margin_m),
        ] {
            positive(name, v)?;
        }
        if self.stair_min_run_m > self.stair_max_run_m {
            return Err("`interiors.stair_min_run_m` must not exceed `interiors.stair_max_run_m`".into());
        }
        if self.step_m >= self.head_m {
            return Err("`interiors.step_m` must be below `interiors.head_m`".into());
        }
        if self.door_clear_h_m <= self.head_m {
            return Err("`interiors.door_clear_h_m` must clear `interiors.head_m`".into());
        }
        if self.collision_sync_m > self.collision_radius_m {
            return Err("`interiors.collision_sync_m` must not exceed `interiors.collision_radius_m`".into());
        }
        Ok(())
    }
}

/// Where a building stands, as its plans need it: the cell grid (as `destruct::Cells`), the side its
/// street is on (0 faces -z, 1 +z, 2 -x, 3 +x: `WorldPlan::street_sides`) and the clear distance in
/// front of each facade (`WorldPlan::fronts`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    /// Building id.
    pub id: u32,
    /// Cells along x, storeys, cells along z.
    pub n: [usize; 3],
    /// Least corner of the grid, on the ground.
    pub origin: Vec3,
    /// One cell.
    pub size: Vec3,
    /// The facade that faces the street.
    pub street: u8,
    /// Clear distance in front of each facade.
    pub fronts: [f32; 4],
}

impl Site {
    /// The site of building `id` of `plan`, on its cells.
    pub fn of(plan: &super::WorldPlan, id: u32, n: [usize; 3], origin: Vec3, size: Vec3) -> Self {
        Self {
            id,
            n,
            origin,
            size,
            street: plan.street_sides.get(id as usize).copied().unwrap_or(0),
            fronts: plan.fronts.get(id as usize).copied().unwrap_or([super::FRONT_LIMIT_M; 4]),
        }
    }

    /// Greatest corner of the grid, on the ground.
    pub fn far(&self) -> Vec3 {
        self.origin + Vec3::new(self.size.x * self.n[0] as f32, 0.0, self.size.z * self.n[2] as f32)
    }

    /// The footprint.
    pub fn footprint(&self) -> Rect {
        let far = self.far();
        Rect {
            min: Vec2::new(self.origin.x, self.origin.z),
            max: Vec2::new(far.x, far.z),
        }
    }

    /// The outward unit normal of facade `side`.
    pub fn out_of(side: u8) -> Vec2 {
        match side {
            0 => Vec2::new(0.0, -1.0),
            1 => Vec2::new(0.0, 1.0),
            2 => Vec2::new(-1.0, 0.0),
            _ => Vec2::new(1.0, 0.0),
        }
    }

    /// The plane of facade `side`: its z (sides 0, 1) or x (2, 3).
    pub fn plane_of(&self, side: u8) -> f32 {
        let far = self.far();
        match side {
            0 => self.origin.z,
            1 => far.z,
            2 => self.origin.x,
            _ => far.x,
        }
    }

    /// A box on facade `side`: `s0..s1` along it (world x for sides 0, 1, else world z), `d0..d1` in
    /// from its plane (negative stands out), `y0..y1` up.
    #[allow(clippy::too_many_arguments)]
    pub fn facade_box(&self, side: u8, s0: f32, s1: f32, d0: f32, d1: f32, y0: f32, y1: f32) -> (Vec3, Vec3) {
        let inward = -Self::out_of(side);
        let plane = self.plane_of(side);
        let (a, b) = if side < 2 {
            (plane + inward.y * d0, plane + inward.y * d1)
        } else {
            (plane + inward.x * d0, plane + inward.x * d1)
        };
        let (lo, hi) = (a.min(b), a.max(b));
        if side < 2 {
            (Vec3::new(s0.min(s1), y0, lo), Vec3::new(s0.max(s1), y1, hi))
        } else {
            (Vec3::new(lo, y0, s0.min(s1)), Vec3::new(hi, y1, s0.max(s1)))
        }
    }
}

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
    /// The ground floor's lobby behind the street door.
    Lobby,
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

impl Rest {
    /// The frame element it is, as the dressing names elements.
    pub fn anchor(self) -> dressing::Anchor {
        match self {
            Rest::Ground => dressing::Anchor::Whole,
            Rest::Slab { k, i, j } => dressing::Anchor::Slab { k, i, j },
        }
    }
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

/// What a furniture box is made of: the texture set it is drawn with (openspec change
/// `building-surfaces`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Surface {
    /// Columns, slabs, core walls.
    Concrete,
    /// Room partitions.
    Plaster,
    /// Desks, shelves, beds, tables.
    Wood,
    /// Seats, sofas, bedding.
    Fabric,
    /// Painted steel: filing cabinets, shelving, fridges.
    Metal,
    /// Monitors, screens, baths, worktops, pots.
    Plastic,
    /// Crates.
    Cardboard,
    /// Plant foliage.
    Leaves,
}

impl Surface {
    /// Every surface, in drawing order.
    pub const ALL: [Surface; 8] = [
        Surface::Concrete,
        Surface::Plaster,
        Surface::Wood,
        Surface::Fabric,
        Surface::Metal,
        Surface::Plastic,
        Surface::Cardboard,
        Surface::Leaves,
    ];
}

/// A furniture item: boxes in world space, and what it rests on.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Its kit name.
    pub name: &'static str,
    /// Its boxes (least corner, greatest corner, linear colour, surface).
    pub boxes: Vec<(Vec3, Vec3, [f32; 3], Surface)>,
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
    /// Wall segments, door gaps already cut (the stair's spine and balustrades among them).
    pub walls: Vec<Partition>,
    /// Concrete you stand on: the stair's treads and half landing, and on the ground storey its
    /// floor slab. `core` is unused.
    pub steps: Vec<Partition>,
    /// Furniture.
    pub items: Vec<Item>,
}

const PARTITION_M: f32 = 0.12;
const CORE_WALL_M: f32 = 0.25;
const CORRIDOR_M: f32 = 2.0;
const BAY_M: f32 = 3.0;
const DOOR_W_M: f32 = 1.0;
const DOOR_H_M: f32 = 2.1;
/// Floor slabs (as `structure.slab_m`) and the stair's landings.
pub const SLAB_M: f32 = 0.25;
/// The concrete under a tread's top.
const WAIST_M: f32 = 0.2;
/// The stair's spine wall and balustrades.
const STAIR_WALL_M: f32 = 0.1;
/// The skin's depth (`dressing`): facade walls run this far in.
pub const SKIN_M: f32 = 0.3;
/// The ground floor's slab top, above the pavement.
pub const GROUND_FLOOR_M: f32 = 0.02;
/// The single-loaded plan's stair room, along and across the corridor.
const STAIR_ROOM_M: [f32; 2] = [5.5, 3.5];
/// A walk-up's stoop: three risers.
const STOOP_RISE_M: f32 = 0.12;
const STOOP_TREAD_M: f32 = 0.35;

/// An axis-aligned plan built in (u, v): u along the long axis.
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// How a building's storeys are laid out, the same on every storey.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
    f2: Frame2,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
    /// The core cell (u0, v0, u1, v1), when the plan has one.
    core: Option<(f32, f32, f32, f32)>,
    /// A corridor each side of the core (else single-loaded along the back).
    double: bool,
    /// The street is on an end (across u), not a long face.
    street_end: bool,
    /// The street is on the high side (high v for a long face, high u for an end).
    street_high: bool,
    /// Double-loaded: the core's door is on its high-v wall.
    door_high: bool,
    /// Single-loaded: the corridor runs along the high-v facade.
    back_high: bool,
    /// Single-loaded: the stair is at the high-u end.
    stair_high: bool,
}

fn layout(site: &Site) -> Layout {
    let [nx, _, nz] = site.n;
    let (x0, z0) = (site.origin.x, site.origin.z);
    let far = site.far();
    let long_x = far.x - x0 >= far.z - z0;
    let (u0, u1, v0, v1) = if long_x { (x0, far.x, z0, far.z) } else { (z0, far.z, x0, far.x) };
    let core = (nx >= 3 && nz >= 3).then(|| {
        let (ci, cj) = ((nx - 1) / 2, (nz - 1) / 2);
        let (a, b) = (x0 + site.size.x * ci as f32, z0 + site.size.z * cj as f32);
        if long_x {
            (a, b, a + site.size.x, b + site.size.z)
        } else {
            (b, a, b + site.size.z, a + site.size.x)
        }
    });
    let street_end = (site.street < 2) != long_x;
    let street_high = site.street % 2 == 1;
    Layout {
        f2: Frame2 { long_x },
        u0,
        u1,
        v0,
        v1,
        core,
        double: core.is_some() && v1 - v0 >= 13.0,
        street_end,
        street_high,
        door_high: street_end || street_high,
        back_high: street_end || !street_high,
        stair_high: !(street_end && street_high),
    }
}

impl Layout {
    /// Single-loaded: the back facade, the corridor's far edge, the way into the plan (+1 or -1 in v).
    fn corridor(&self) -> (f32, f32, f32) {
        let (bv, sgn) = if self.back_high { (self.v1, -1.0) } else { (self.v0, 1.0) };
        (bv, bv + sgn * CORRIDOR_M, sgn)
    }

    /// Single-loaded: the stair room's u range.
    fn stair_room(&self) -> (f32, f32) {
        if self.stair_high {
            (self.u1 - STAIR_ROOM_M[0], self.u1)
        } else {
            (self.u0, self.u0 + STAIR_ROOM_M[0])
        }
    }
}

/// A dog-leg stair in the core, from the ground to the top storey, in its own frame: `a` runs from
/// the stair's door wall away from it (landing, flights, half landing), `b` across (flight one's
/// lane, the spine wall, flight two's lane).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stair {
    /// World (x, z) of a = 0, b = 0.
    pub at: Vec2,
    /// The way `a` runs, a unit axis.
    pub dir_a: Vec2,
    /// The way `b` runs, a unit axis.
    pub dir_b: Vec2,
    /// Clear length along `a`, door wall to far wall.
    pub length_m: f32,
    /// Clear room across `b`.
    pub room_m: f32,
    /// One lane.
    pub lane_m: f32,
    /// The landing at the door.
    pub landing_m: f32,
    /// Risers a flight (two flights a storey).
    pub flight: u32,
    /// One riser.
    pub rise_m: f32,
    /// One tread.
    pub run_m: f32,
    /// Floor height of the ground storey.
    pub y0: f32,
    /// One storey.
    pub storey_m: f32,
    /// Storeys.
    pub storeys: usize,
    /// A balustrade along b = 0 (a single-loaded stair opens off the corridor there).
    pub guard_near: bool,
}

impl Stair {
    /// Both lanes and the spine.
    pub fn width_m(&self) -> f32 {
        2.0 * self.lane_m + STAIR_WALL_M
    }

    /// A point in the stair's frame, in the world.
    pub fn point(&self, a: f32, b: f32) -> Vec2 {
        self.at + self.dir_a * a + self.dir_b * b
    }

    /// A rectangle in the stair's frame, in the world.
    pub fn rect(&self, a0: f32, a1: f32, b0: f32, b1: f32) -> (Vec2, Vec2) {
        let (p, q) = (self.point(a0, b0), self.point(a1, b1));
        (p.min(q), p.max(q))
    }

    /// Where the half landing starts.
    fn half_from(&self) -> f32 {
        self.landing_m + (self.flight - 1) as f32 * self.run_m
    }

    /// The stairwell cut out of every floor above the ground: both flights and the half landing.
    pub fn hole(&self) -> (Vec2, Vec2) {
        self.rect(self.landing_m, self.length_m, -0.001, self.width_m())
    }

    /// Floor height of storey `k`.
    pub fn floor_y(&self, k: usize) -> f32 {
        self.y0 + self.storey_m * k as f32
    }

    /// The treads and half landing that climb from storey `k` to `k + 1` (none from the top).
    pub fn treads(&self, k: usize) -> Vec<(Vec3, Vec3)> {
        let mut out = Vec::new();
        if k + 1 >= self.storeys {
            return out;
        }
        let y = self.floor_y(k);
        let n = self.flight;
        let lane_a = (0.0, self.lane_m);
        let lane_b = (self.lane_m + STAIR_WALL_M, self.width_m());
        let boxed = |r: (Vec2, Vec2), top: f32, depth: f32| (Vec3::new(r.0.x, top - depth, r.0.y), Vec3::new(r.1.x, top, r.1.y));
        for i in 1..n {
            let a = self.landing_m + (i - 1) as f32 * self.run_m;
            let top = y + i as f32 * self.rise_m;
            out.push(boxed(self.rect(a, a + self.run_m, lane_a.0, lane_a.1), top, self.rise_m + WAIST_M));
        }
        let top = y + n as f32 * self.rise_m;
        out.push(boxed(self.rect(self.half_from(), self.length_m, 0.0, self.width_m()), top, SLAB_M));
        for j in 1..n {
            let a = self.landing_m + (n - 1 - j) as f32 * self.run_m;
            let top = y + (n + j) as f32 * self.rise_m;
            out.push(boxed(self.rect(a, a + self.run_m, lane_b.0, lane_b.1), top, self.rise_m + WAIST_M));
        }
        out
    }

    /// The stair's walls on storey `k`, a storey tall: the spine between the flights, a balustrade on
    /// its open side (and along b = 0 when it opens off a corridor); on the top storey, a guard across
    /// the stairwell above flight one too.
    pub fn walls(&self, k: usize) -> Vec<(Vec3, Vec3)> {
        let (y0, y1) = (self.floor_y(k), self.floor_y(k + 1) - SLAB_M);
        let boxed = |r: (Vec2, Vec2)| (Vec3::new(r.0.x, y0, r.0.y), Vec3::new(r.1.x, y1, r.1.y));
        let w = self.width_m();
        let mut out = vec![boxed(self.rect(
            self.landing_m,
            self.half_from(),
            self.lane_m,
            self.lane_m + STAIR_WALL_M,
        ))];
        if self.room_m > w + 0.15 {
            out.push(boxed(self.rect(self.landing_m, self.length_m, w, w + STAIR_WALL_M)));
        }
        if self.guard_near {
            out.push(boxed(self.rect(self.landing_m, self.length_m, -PARTITION_M, 0.0)));
        }
        if k + 1 == self.storeys && self.storeys > 1 {
            out.push(boxed(self.rect(self.landing_m, self.landing_m + STAIR_WALL_M, 0.0, self.lane_m)));
        }
        out
    }

    /// Where someone waits on storey `k`'s landing to climb (feet): at the foot of flight one.
    pub fn landing_spot(&self, k: usize) -> Vec3 {
        let p = self.point(self.landing_m * 0.5, self.lane_m * 0.5);
        Vec3::new(p.x, self.floor_y(k), p.y)
    }

    /// The way up from storey `k`'s landing to storey `k + 1`'s (feet, in order): up flight one, across
    /// the half landing, up flight two, onto the landing above.
    pub fn climb(&self, k: usize) -> Vec<Vec3> {
        let mid = (self.half_from() + self.length_m) * 0.5;
        let lane_b = self.lane_m * 1.5 + STAIR_WALL_M;
        let at = |a: f32, b: f32, y: f32| {
            let p = self.point(a, b);
            Vec3::new(p.x, y, p.y)
        };
        let (y, half) = (self.floor_y(k), self.floor_y(k) + self.storey_m * 0.5);
        vec![
            self.landing_spot(k),
            at(mid, self.lane_m * 0.5, half),
            at(mid, lane_b, half),
            at(self.landing_m * 0.5, lane_b, y + self.storey_m),
            self.landing_spot(k + 1),
        ]
    }
}

/// How the street door is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorKind {
    /// A walk-up's timber front door, up a stone stoop.
    Front,
    /// An office or tower's pair of glass lobby doors.
    Lobby,
    /// A warehouse's steel personnel door.
    Personnel,
}

/// A building's street entrance: where the door is, what it is, what you stand on to reach it, its
/// leaves (standing open: survey W30) and the way in from the pavement to the stair.
#[derive(Clone, Debug, PartialEq)]
pub struct Entrance {
    /// The facade it is in (0 faces -z, 1 +z, 2 -x, 3 +x).
    pub side: u8,
    /// The bay (cell along that facade).
    pub bay: u32,
    /// Its centre from the bay's start, along the facade (+x for sides 0, 1, else +z).
    pub along_m: f32,
    /// Its centre on the facade's plane, world (x, z).
    pub centre: Vec2,
    /// Clear width.
    pub width_m: f32,
    /// Its threshold above the ground floor.
    pub sill_m: f32,
    /// Clear height above the sill.
    pub clear_m: f32,
    /// What it is.
    pub kind: DoorKind,
    /// Concrete and stone to stand on: the stoop's steps and the threshold.
    pub steps: Vec<(Vec3, Vec3)>,
    /// The door's leaves, standing open inside, at right angles to the facade.
    pub leaves: Vec<(Vec3, Vec3)>,
    /// The way in (world x, z): outside the door, the door, through the lobby or corridor, the stair's
    /// door, the foot of the stair.
    pub route: Vec<Vec2>,
    /// The lobby's corridor door along u, when the door opens into a lobby.
    lobby_u: Option<f32>,
}

/// A building's entrance and stair, which every storey's plan, the dressing and the colliders share.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    /// The street entrance.
    pub entrance: Entrance,
    /// The stair.
    pub stair: Stair,
    layout: Layout,
}

/// The entrance and stair of `site`.
pub fn shell(site: &Site, cfg: &InteriorConfig, seed: &SimRng) -> Shell {
    let l = layout(site);
    let stair = stair_of(site, &l, cfg);
    let entrance = entrance_of(site, &l, &stair, cfg, seed);
    Shell {
        entrance,
        stair,
        layout: l,
    }
}

fn stair_of(site: &Site, l: &Layout, cfg: &InteriorConfig) -> Stair {
    let f2 = l.f2;
    let dir = |du: f32, dv: f32| f2.point(du, dv);
    let half_core = CORE_WALL_M * 0.5;
    let (at, dir_a, dir_b, length_m, room_m, guard_near) = match (l.double, l.core) {
        (true, Some((cu0, cv0, cu1, cv1))) => {
            let (door_v, a_sign) = if l.door_high {
                (cv1 - half_core, -1.0)
            } else {
                (cv0 + half_core, 1.0)
            };
            (
                f2.point(cu0 + half_core, door_v),
                dir(0.0, a_sign),
                dir(1.0, 0.0),
                cv1 - cv0 - CORE_WALL_M,
                cu1 - cu0 - CORE_WALL_M,
                false,
            )
        }
        _ => {
            let (_, cv1, sgn) = l.corridor();
            let (su0, su1) = l.stair_room();
            let b0 = cv1 + sgn * PARTITION_M;
            let room = STAIR_ROOM_M[1] - PARTITION_M - half_core;
            if l.stair_high {
                (
                    f2.point(su0 + half_core, b0),
                    dir(1.0, 0.0),
                    dir(0.0, sgn),
                    su1 - 0.32 - (su0 + half_core),
                    room,
                    true,
                )
            } else {
                (
                    f2.point(su1 - half_core, b0),
                    dir(-1.0, 0.0),
                    dir(0.0, sgn),
                    su1 - half_core - (su0 + 0.32),
                    room,
                    true,
                )
            }
        }
    };
    let storey = site.size.y;
    let risers = (2.0 * (storey / (2.0 * cfg.stair_max_rise_m)).ceil()).max(2.0) as u32;
    let flight = risers / 2;
    let lane_m = cfg.stair_width_m.min((room_m - STAIR_WALL_M) * 0.5);
    let run_m = cfg
        .stair_max_run_m
        .min((length_m - cfg.stair_landing_m - cfg.stair_half_landing_m) / (flight.max(2) - 1) as f32);
    Stair {
        at,
        dir_a,
        dir_b,
        length_m,
        room_m,
        lane_m,
        landing_m: cfg.stair_landing_m,
        flight,
        rise_m: storey / risers as f32,
        run_m,
        y0: site.origin.y,
        storey_m: storey,
        storeys: site.n[1],
        guard_near,
    }
}

fn entrance_of(site: &Site, l: &Layout, stair: &Stair, cfg: &InteriorConfig, seed: &SimRng) -> Entrance {
    let f2 = l.f2;
    let side = site.street;
    let style = dressing::style_for(site.id, site.n, site.size, seed);
    let (kind, mut width_m, sill_m) = match style {
        Style::WalkUp => (DoorKind::Front, cfg.front_door_w_m, STOOP_RISE_M * 3.0),
        Style::Office | Style::Tower => (DoorKind::Lobby, cfg.lobby_door_w_m, 0.0),
        Style::Warehouse => (DoorKind::Personnel, cfg.personnel_door_w_m, 0.0),
    };
    // A door into a corridor's end fits the corridor, clear of the corner column.
    let into_corridor = l.street_end;
    if into_corridor {
        width_m = width_m.min(CORRIDOR_M - 0.5);
    }
    let landing = stair.point(stair.landing_m * 0.5, stair.lane_m * 0.5);
    let u_land = if l.f2.long_x { landing.x } else { landing.y };
    // (u, v) of the door on the facade, the route inside, and the lobby's corridor door.
    let (door_uv, inside, lobby_u): ((f32, f32), Vec<Vec2>, Option<f32>) = match (l.double, l.core) {
        (true, Some((cu0, cv0, cu1, cv1))) => {
            let umid = (cu0 + cu1) * 0.5;
            let vc = (cv0 + cv1) * 0.5;
            let (core_door_v, lane_v) = if l.door_high {
                (cv1, cv1 + CORRIDOR_M * 0.5)
            } else {
                (cv0, cv0 - CORRIDOR_M * 0.5)
            };
            // Through the stair's door straight onto its landing, then along the landing.
            let into = if l.door_high { -1.0 } else { 1.0 };
            let on_landing = f2.point(umid, core_door_v + into * (half_core_wall() + stair.landing_m * 0.5));
            if l.street_end {
                let (uf, s) = if l.street_high { (l.u1, -1.0) } else { (l.u0, 1.0) };
                let u_side = if l.street_high {
                    cu1 + CORRIDOR_M * 0.5
                } else {
                    cu0 - CORRIDOR_M * 0.5
                };
                (
                    (uf, vc),
                    vec![
                        f2.point(uf + s * 0.6, vc),
                        f2.point(u_side, vc),
                        f2.point(u_side, lane_v),
                        f2.point(umid, lane_v),
                        f2.point(umid, core_door_v),
                        on_landing,
                        landing,
                    ],
                    None,
                )
            } else {
                let (vf, s) = if l.street_high { (l.v1, -1.0) } else { (l.v0, 1.0) };
                (
                    (umid, vf),
                    vec![
                        f2.point(umid, vf + s * 0.6),
                        f2.point(umid, lane_v),
                        f2.point(umid, core_door_v),
                        on_landing,
                        landing,
                    ],
                    Some(umid),
                )
            }
        }
        _ => {
            let (bv, cv1, sgn) = l.corridor();
            let vcc = (bv + cv1) * 0.5;
            let open = f2.point(u_land, vcc);
            if l.street_end {
                let (uf, s) = if l.street_high { (l.u1, -1.0) } else { (l.u0, 1.0) };
                // Clear of the corner column (0.3 m) by 5 cm.
                let vd = bv + sgn * (CORRIDOR_M * 0.5).max(0.35 + width_m * 0.5);
                (
                    (uf, vd),
                    vec![f2.point(uf + s * 0.6, vd), f2.point(uf + s * 1.2, vcc), open, landing],
                    None,
                )
            } else {
                // The front band: the bay nearest the middle of what the stair leaves.
                let (su0, su1) = l.stair_room();
                let (a, b) = if l.stair_high { (l.u0, su0) } else { (su1, l.u1) };
                let cell_u = if l.f2.long_x { site.size.x } else { site.size.z };
                let mid = (a + b) * 0.5;
                let cells = ((l.u1 - l.u0) / cell_u).round() as usize;
                let ud = (0..cells)
                    .map(|i| l.u0 + cell_u * (i as f32 + 0.5))
                    .filter(|&u| u - width_m * 0.5 - 0.6 >= a && u + width_m * 0.5 + 0.6 <= b)
                    .min_by(|p, q| (p - mid).abs().total_cmp(&(q - mid).abs()))
                    .unwrap_or(mid);
                let vf = if l.back_high { l.v0 } else { l.v1 };
                (
                    (ud, vf),
                    vec![f2.point(ud, vf - sgn * 0.6), f2.point(ud, vcc), open, landing],
                    Some(ud),
                )
            }
        }
    };
    let centre = f2.point(door_uv.0, door_uv.1);
    let out = Site::out_of(side);
    // Along the facade, and its bay.
    let (s_c, start, cell) = if side < 2 {
        (centre.x, site.origin.x, site.size.x)
    } else {
        (centre.y, site.origin.z, site.size.z)
    };
    let count = if side < 2 { site.n[0] } else { site.n[2] };
    let bay = (((s_c - start) / cell).floor().max(0.0) as usize).min(count.saturating_sub(1)) as u32;
    let along_m = s_c - (start + cell * bay as f32);
    let y0 = site.origin.y;
    let (s0, s1) = (s_c - width_m * 0.5, s_c + width_m * 0.5);
    // Half the gap in front is this building's to stand things in.
    let front = (site.fronts[side as usize] * 0.5 - 0.05).max(0.0);
    let mut steps = Vec::new();
    if kind == DoorKind::Front {
        // Three steps up to the threshold, the bottom one deepest; never past the party line.
        for i in 0..3 {
            let top = y0 + STOOP_RISE_M * (i + 1) as f32;
            let depth = (STOOP_TREAD_M * (3 - i) as f32).min(front);
            if depth > 0.05 {
                steps.push(site.facade_box(side, s0 - 0.25, s1 + 0.25, -depth, 0.0, top - STOOP_RISE_M, top));
            }
        }
    }
    if sill_m > 0.0 {
        steps.push(site.facade_box(side, s0, s1, 0.0, SKIN_M, y0, y0 + sill_m));
    }
    let (ya, yb) = (y0 + sill_m, y0 + sill_m + cfg.door_clear_h_m);
    let leaf_t = 0.05;
    let leaves = match kind {
        DoorKind::Lobby => vec![
            site.facade_box(side, s0, s0 + leaf_t, SKIN_M, SKIN_M + width_m * 0.5, ya, yb),
            site.facade_box(side, s1 - leaf_t, s1, SKIN_M, SKIN_M + width_m * 0.5, ya, yb),
        ],
        _ => vec![site.facade_box(side, s0, s0 + leaf_t, SKIN_M, SKIN_M + width_m, ya, yb)],
    };
    let outside = centre + out * (1.6f32).min(site.fronts[side as usize] - 0.5).max(0.6);
    let mut route = vec![outside, centre];
    route.extend(inside);
    Entrance {
        side,
        bay,
        along_m,
        centre,
        width_m,
        sill_m,
        clear_m: cfg.door_clear_h_m,
        kind,
        steps,
        leaves,
        route,
        lobby_u,
    }
}

fn half_core_wall() -> f32 {
    CORE_WALL_M * 0.5
}

/// `r` with `hole` cut out of it, as up to four rectangles (x, z).
pub fn rect_minus(r: (Vec2, Vec2), hole: (Vec2, Vec2)) -> Vec<(Vec2, Vec2)> {
    let (lo, hi) = r;
    let (hl, hh) = (hole.0.max(lo), hole.1.min(hi));
    if hl.x >= hh.x || hl.y >= hh.y {
        return vec![r];
    }
    let mut out = Vec::new();
    let mut push = |a: Vec2, b: Vec2| {
        if b.x - a.x > 1e-3 && b.y - a.y > 1e-3 {
            out.push((a, b));
        }
    };
    push(lo, Vec2::new(hi.x, hl.y));
    push(Vec2::new(lo.x, hh.y), hi);
    push(Vec2::new(lo.x, hl.y), Vec2::new(hl.x, hh.y));
    push(Vec2::new(hh.x, hl.y), Vec2::new(hi.x, hh.y));
    out
}

/// Plans storey `k` of `site`.
pub fn plan_storey(site: &Site, k: usize, cfg: &InteriorConfig, seed: &SimRng) -> StoreyPlan {
    let [nx, ns, nz] = site.n;
    let mut plan = StoreyPlan::default();
    if nx == 0 || nz == 0 || k >= ns {
        return plan;
    }
    let shell = shell(site, cfg, seed);
    let l = shell.layout;
    let f2 = l.f2;
    let (origin, size) = (site.origin, site.size);
    let mut rng = seed.fork(&format!("building/{}/plan/{k}", site.id));
    let use_ = use_of(k, ns);
    let (x0, z0) = (origin.x, origin.z);
    let (u0, u1, v0, v1) = (l.u0, l.u1, l.v0, l.v1);
    let floor_y = origin.y + size.y * k as f32;
    let ceil_y = floor_y + size.y - SLAB_M;
    let rest_at = |p: Vec2| -> Rest {
        if k == 0 {
            Rest::Ground
        } else {
            Rest::Slab {
                k: k as u32 - 1,
                i: (((p.x - x0) / size.x).max(0.0) as u32).min(nx as u32 - 1),
                j: (((p.y - z0) / size.z).max(0.0) as u32).min(nz as u32 - 1),
            }
        }
    };
    let mut walls: Vec<(f32, f32, f32, f32, Option<f32>, bool)> = Vec::new(); // u0 v0 u1 v1 door core
    let cw = CORRIDOR_M;
    struct Band {
        u0: f32,
        u1: f32,
        va: f32,
        vb: f32,
        front: bool,
    }
    let mut bands: Vec<Band> = Vec::new();
    // The street door's lobby: the ground storey's room behind it, its corridor door lined up.
    let lobby = if k == 0 { shell.entrance.lobby_u } else { None };
    let street_v = if l.street_high { v1 } else { v0 };
    match (l.double, l.core) {
        (true, Some((cu0, cv0, cu1, cv1))) => {
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
                let front = l.street_end || fv == street_v;
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
            // The core's concrete walls, with the stair door onto the lane the street door reaches.
            let mid = Some((cu0 + cu1) * 0.5);
            walls.push((cu0, cv0, cu0, cv1, None, true));
            walls.push((cu1, cv0, cu1, cv1, None, true));
            walls.push((cu0, cv0, cu1, cv0, if l.door_high { None } else { mid }, true));
            walls.push((cu0, cv1, cu1, cv1, if l.door_high { mid } else { None }, true));
        }
        _ => {
            // Single-loaded: corridor along the back, stair at the end away from the street.
            let (bv, cv1, sgn) = l.corridor();
            let fv = if l.back_high { v0 } else { v1 };
            let sv = cv1 + sgn * STAIR_ROOM_M[1];
            plan.corridor.push(f2.rect(u0, bv, u1, cv1));
            let (su0, su1) = l.stair_room();
            let (inner_u, band_u) = if l.stair_high { (su0, (u0, su0)) } else { (su1, (su1, u1)) };
            bands.push(Band {
                u0: band_u.0,
                u1: band_u.1,
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
                walls.push((inner_u, sv, inner_u, fv, None, false));
            }
            walls.push((inner_u, cv1, inner_u, sv, None, true));
            walls.push((su0, sv, su1, sv, None, true));
        }
    }
    // Rooms on the bay lines.
    for b in &bands {
        let mut u = b.u0;
        let snap = |x: f32| b.u0 + (((x - b.u0) / BAY_M).round() * BAY_M).max(BAY_M);
        let lobby_here = lobby.filter(|&ud| ud > b.u0 && ud < b.u1 && b.vb == if l.double { street_v } else { b.vb });
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
            let lobby_door = lobby_here.filter(|&ud| ud > u && ud < ue);
            let kind = if lobby_door.is_some() { RoomKind::Lobby } else { kind };
            let (lo, hi) = f2.rect(u, b.va, ue, b.vb);
            plan.rooms.push(Room { lo, hi, kind });
            // A flat narrower than a bath and a kitchen stays one room.
            let flat = kind == RoomKind::Living && ue - u >= 4.6;
            let kitchen_mid = u + 2.4 + (ue - u - 2.4) * 0.5;
            let door = match lobby_door {
                Some(ud) => ud,
                None if flat => kitchen_mid,
                None => u + (ue - u).min(2.4) * 0.5,
            };
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
    // The stair: its walls on this storey and the flights up from it; the ground storey's floor.
    let st = &shell.stair;
    let mid = |a: Vec3, b: Vec3| Vec2::new((a.x + b.x) * 0.5, (a.z + b.z) * 0.5);
    for (min, max) in st.walls(k) {
        plan.walls.push(Partition {
            min,
            max,
            core: true,
            rest: rest_at(mid(min, max)),
        });
    }
    for (min, max) in st.treads(k) {
        plan.steps.push(Partition {
            min,
            max,
            core: true,
            rest: rest_at(mid(min, max)),
        });
    }
    if k == 0 {
        let far = site.far();
        plan.steps.push(Partition {
            min: Vec3::new(origin.x + SKIN_M, origin.y - SLAB_M, origin.z + SKIN_M),
            max: Vec3::new(far.x - SKIN_M, origin.y + GROUND_FLOOR_M, far.z - SKIN_M),
            core: true,
            rest: Rest::Ground,
        });
    }
    // Furniture keeps off the way in, a strip either side of it past the door's half-width.
    let keep: Vec<(Vec2, Vec2)> = if k == 0 {
        let e = &shell.entrance;
        let half = Vec2::splat(e.width_m * 0.5 + 0.4);
        e.route.windows(2).map(|w| (w[0].min(w[1]) - half, w[0].max(w[1]) + half)).collect()
    } else {
        Vec::new()
    };
    furnish(
        &mut plan,
        floor_y,
        [x0, z0, site.far().x, site.far().z],
        site.n,
        origin,
        size,
        &keep,
        &mut rng,
        &rest_at,
    );
    plan
}

/// A kit item: boxes in its own frame (origin at its back edge on the floor, facing +z), as
/// (centre x, centre y, centre z, size x, size y, size z, colour, surface).
type KitBox = (f32, f32, f32, f32, f32, f32, [f32; 3], Surface);
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
                (0.0, 0.73, 0.375, 1.5, 0.04, 0.75, WOOD, Surface::Wood),
                (-0.55, 0.36, 0.375, 0.4, 0.71, 0.7, WOOD, Surface::Wood),
                (0.1, 0.95, 0.25, 0.42, 0.38, 0.42, BEIGE, Surface::Plastic),
                (0.0, 0.45, 1.05, 0.5, 0.08, 0.5, FABRIC, Surface::Fabric),
            ],
            false,
            0.6,
        ),
        "filing" => (&[(0.0, 0.66, 0.3, 0.47, 1.32, 0.6, [0.44, 0.47, 0.42], Surface::Metal)], true, 0.7),
        "shelf" => (&[(0.0, 1.0, 0.17, 0.9, 2.0, 0.34, WOOD, Surface::Wood)], true, 0.5),
        "sofa" => (
            &[
                (0.0, 0.22, 0.45, 2.0, 0.44, 0.9, FABRIC, Surface::Fabric),
                (0.0, 0.62, 0.1, 2.0, 0.5, 0.2, FABRIC, Surface::Fabric),
            ],
            false,
            0.9,
        ),
        "tv" => (
            &[
                (0.0, 0.3, 0.25, 1.0, 0.6, 0.45, WOOD, Surface::Wood),
                (0.0, 0.86, 0.25, 0.66, 0.52, 0.5, [0.15, 0.15, 0.16], Surface::Plastic),
            ],
            false,
            1.6,
        ),
        "bed" => (
            &[
                (0.0, 0.25, 1.0, 1.5, 0.5, 2.0, WOOD, Surface::Wood),
                (0.0, 0.56, 1.05, 1.42, 0.2, 1.9, WHITE, Surface::Fabric),
                (0.0, 0.6, 0.03, 1.6, 1.2, 0.06, WOOD, Surface::Wood),
            ],
            false,
            0.6,
        ),
        "wardrobe" => (&[(0.0, 1.0, 0.3, 1.2, 2.0, 0.6, WOOD, Surface::Wood)], true, 0.8),
        "counter" => (
            &[
                (0.0, 0.45, 0.3, 2.4, 0.9, 0.6, [0.66, 0.6, 0.44], Surface::Wood),
                (0.0, 0.92, 0.31, 2.42, 0.04, 0.62, WHITE, Surface::Plastic),
            ],
            false,
            0.9,
        ),
        "fridge" => (&[(0.0, 0.9, 0.35, 0.7, 1.8, 0.7, [0.86, 0.82, 0.68], Surface::Metal)], true, 0.8),
        "bath" => (&[(0.0, 0.28, 0.38, 1.6, 0.56, 0.75, WHITE, Surface::Plastic)], false, 0.5),
        "meeting" => (
            &[
                (0.0, 0.73, 0.6, 3.6, 0.05, 1.2, WOOD, Surface::Wood),
                (0.0, 0.36, 0.6, 0.3, 0.71, 0.6, WOOD, Surface::Wood),
            ],
            false,
            0.9,
        ),
        "gondola" => (&[(0.0, 0.75, 0.45, 3.0, 1.5, 0.9, [0.78, 0.78, 0.76], Surface::Metal)], true, 1.0),
        "till" => (
            &[
                (0.0, 0.5, 0.3, 2.4, 1.0, 0.6, WOOD, Surface::Wood),
                (-0.6, 1.1, 0.3, 0.45, 0.2, 0.4, STEEL, Surface::Metal),
            ],
            false,
            1.0,
        ),
        "crates" => (&[(0.0, 0.6, 0.6, 1.2, 1.2, 1.2, [0.62, 0.48, 0.3], Surface::Cardboard)], false, 0.5),
        _ => (
            &[
                (0.0, 0.2, 0.2, 0.35, 0.4, 0.35, [0.5, 0.33, 0.22], Surface::Plastic),
                (0.0, 0.85, 0.2, 0.6, 0.9, 0.6, [0.27, 0.42, 0.2], Surface::Leaves),
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
        RoomKind::Lobby => &[("plant", 1, 2), ("sofa", 0, 1)],
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
    keep: &[(Vec2, Vec2)],
    rng: &mut SimRng,
    rest_at: &dyn Fn(Vec2) -> Rest,
) {
    let door_zones: Vec<(Vec2, Vec2)> = plan
        .doors
        .iter()
        .map(|d| (*d - Vec2::splat(0.9), *d + Vec2::splat(0.9)))
        .chain(keep.iter().copied())
        .collect();
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
                    let world: Vec<(Vec3, Vec3, [f32; 3], Surface)> = boxes
                        .iter()
                        .map(|&(cx, cy, cz, sx, sy, sz, col, surface)| {
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
                                surface,
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

/// One solid box of a building as a person on foot meets it, and the frame element it needs
/// standing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solid {
    /// Least corner.
    pub min: Vec3,
    /// Greatest corner.
    pub max: Vec3,
    /// The element it hangs on (`Whole`: always there).
    pub hold: dressing::Anchor,
    /// Only once the building has a frame (columns, which a whole building does not draw).
    pub framed_only: bool,
}

/// A building's detailed colliders for the pilot on foot (design section 4): the facade bays with
/// the entrance cut out, the floors with the stairwell cut out, the roof, every storey's walls,
/// stair and furniture, the stoop and the open door leaves. One rule for all of them: a box no
/// higher than `step_m` above the feet is stood on, a taller one that starts below `head_m` is a
/// wall. Built from the same plans that are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Walkable {
    /// The building.
    pub id: u32,
    /// Every box.
    pub solids: Vec<Solid>,
    /// Box indices by storey band (the last band is the roof and above).
    buckets: Vec<Vec<u32>>,
    y0: f32,
    storey_m: f32,
}

impl Walkable {
    /// The colliders of `site`. `column_m` is the frame's column size (`structure.column_m`).
    pub fn build(site: &Site, cfg: &InteriorConfig, column_m: f32, seed: &SimRng) -> Self {
        use dressing::Anchor;
        let [nx, ns, nz] = site.n;
        let shell = shell(site, cfg, seed);
        let (origin, size) = (site.origin, site.size);
        let mut solids = Vec::new();
        let mut put = |min: Vec3, max: Vec3, hold: Anchor, framed_only: bool| {
            if (max - min).min_element() > 1e-4 {
                solids.push(Solid {
                    min,
                    max,
                    hold,
                    framed_only,
                });
            }
        };
        let floor = |k: usize| origin.y + size.y * k as f32;
        // Floors: the bay of cell (i, j) at the top of storey k, the stairwell cut out below the top.
        let hole = shell.stair.hole();
        for k in 0..ns {
            for i in 0..nx {
                for j in 0..nz {
                    let lo = Vec2::new(origin.x + size.x * i as f32, origin.z + size.z * j as f32);
                    let cell = (lo, lo + Vec2::new(size.x, size.z));
                    let pieces = if k + 1 < ns { rect_minus(cell, hole) } else { vec![cell] };
                    let top = floor(k + 1);
                    for (a, b) in pieces {
                        put(
                            Vec3::new(a.x, top - SLAB_M, a.y),
                            Vec3::new(b.x, top, b.y),
                            Anchor::Slab {
                                k: k as u32,
                                i: i as u32,
                                j: j as u32,
                            },
                            false,
                        );
                    }
                }
            }
        }
        // Facade bays, the skin's depth, the street door cut out of its bay.
        let e = &shell.entrance;
        for k in 0..ns {
            for side in 0..4u8 {
                let (count, start, cell) = if side < 2 { (nx, origin.x, size.x) } else { (nz, origin.z, size.z) };
                for idx in 0..count {
                    let (s0, s1) = (start + cell * idx as f32, start + cell * (idx + 1) as f32);
                    let (y0, y1) = (floor(k), floor(k + 1));
                    let hold = Anchor::Wall {
                        k: k as u32,
                        side,
                        idx: idx as u32,
                    };
                    let spans: Vec<(f32, f32, f32, f32)> = if k == 0 && side == e.side && idx as u32 == e.bay {
                        let (d0, d1) = (s0 + e.along_m - e.width_m * 0.5, s0 + e.along_m + e.width_m * 0.5);
                        let top = y0 + e.sill_m + e.clear_m;
                        vec![(s0, d0, y0, y1), (d1, s1, y0, y1), (d0, d1, top, y1)]
                    } else {
                        vec![(s0, s1, y0, y1)]
                    };
                    for (a, b, ya, yb) in spans {
                        let (min, max) = site.facade_box(side, a, b, 0.0, SKIN_M, ya, yb);
                        put(min, max, hold, false);
                    }
                }
            }
        }
        for &(min, max) in &e.steps {
            put(min, max, Anchor::Whole, false);
        }
        let leaf_hold = Anchor::Wall {
            k: 0,
            side: e.side,
            idx: e.bay,
        };
        for &(min, max) in &e.leaves {
            put(min, max, leaf_hold, false);
        }
        // Every storey's walls, stair and furniture.
        for k in 0..ns {
            let plan = plan_storey(site, k, cfg, seed);
            for w in plan.walls.iter().chain(&plan.steps) {
                put(w.min, w.max, w.rest.anchor(), false);
            }
            for it in &plan.items {
                for (min, max, _, _) in &it.boxes {
                    put(*min, *max, it.rest.anchor(), false);
                }
            }
        }
        // Columns, once there is a frame to draw them.
        let half = column_m * 0.5;
        for k in 0..ns {
            for a in 0..=nx {
                for c in 0..=nz {
                    let p = Vec2::new(origin.x + size.x * a as f32, origin.z + size.z * c as f32);
                    put(
                        Vec3::new(p.x - half, floor(k), p.y - half),
                        Vec3::new(p.x + half, floor(k + 1) - SLAB_M, p.y + half),
                        Anchor::Column {
                            k: k as u32,
                            a: a as u32,
                            c: c as u32,
                        },
                        true,
                    );
                }
            }
        }
        let mut buckets = vec![Vec::new(); ns + 1];
        let band = |y: f32| (((y - origin.y) / size.y).floor().max(0.0) as usize).min(ns);
        for (i, s) in solids.iter().enumerate() {
            let (lo, hi) = (band(s.min.y), band(s.max.y - 1e-3));
            for bucket in &mut buckets[lo..=hi.max(lo)] {
                bucket.push(i as u32);
            }
        }
        Self {
            id: site.id,
            solids,
            buckets,
            y0: origin.y,
            storey_m: size.y,
        }
    }

    fn band(&self, y: f32) -> usize {
        (((y - self.y0) / self.storey_m).floor().max(0.0) as usize).min(self.buckets.len() - 1)
    }

    /// True when `s` stands: always while the building is whole (`frame` is `None`), else while its
    /// element does.
    fn stands(s: &Solid, frame: Option<&super::structure::Frame>) -> bool {
        use super::structure::GROUND;
        use dressing::Anchor;
        let Some(f) = frame else { return !s.framed_only };
        let alive = |id: u32| id != GROUND && f.elements.get(id as usize).is_some_and(|e| e.alive);
        match s.hold {
            Anchor::Whole => true,
            Anchor::Slab { k, i, j } => alive(f.slab(k as usize, i as usize, j as usize)),
            Anchor::Wall { k, side, idx } => alive(f.wall(k as usize, side as usize, idx as usize)),
            Anchor::Column { k, a, c } => alive(f.column(k as usize, a as usize, c as usize)),
        }
    }

    /// A body of `radius_m` at `p` (x, z) with its feet at `feet_y`, moved out of every standing box
    /// that is a wall to it.
    pub fn push_out(&self, p: Vec2, radius_m: f32, feet_y: f32, cfg: &InteriorConfig, frame: Option<&super::structure::Frame>) -> Vec2 {
        if !p.is_finite() || !feet_y.is_finite() {
            return p;
        }
        let (lo, hi) = (self.band(feet_y), self.band(feet_y + cfg.head_m));
        let mut p = p;
        for _ in 0..2 {
            for b in lo..=hi {
                for &i in &self.buckets[b] {
                    let s = &self.solids[i as usize];
                    if s.max.y <= feet_y + cfg.step_m || s.min.y >= feet_y + cfg.head_m || !Self::stands(s, frame) {
                        continue;
                    }
                    let r = Rect {
                        min: Vec2::new(s.min.x, s.min.z),
                        max: Vec2::new(s.max.x, s.max.z),
                    };
                    if r.distance(p) < radius_m {
                        p = super::collision::push_out_of_rect(p, radius_m, &r);
                    }
                }
            }
        }
        p
    }

    /// The highest standing top under `p` within a step of feet at `feet_y`, if any.
    pub fn support(&self, p: Vec2, feet_y: f32, cfg: &InteriorConfig, frame: Option<&super::structure::Frame>) -> Option<f32> {
        if !p.is_finite() || !feet_y.is_finite() {
            return None;
        }
        let reach = feet_y + cfg.step_m;
        for b in (0..=self.band(reach)).rev() {
            let best = self.buckets[b]
                .iter()
                .map(|&i| &self.solids[i as usize])
                .filter(|s| {
                    s.max.y <= reach && p.x >= s.min.x && p.x <= s.max.x && p.y >= s.min.z && p.y <= s.max.z && Self::stands(s, frame)
                })
                .map(|s| s.max.y)
                .reduce(f32::max);
            if best.is_some() {
                return best;
            }
        }
        None
    }
}

/// The detailed colliders built near the pilot on foot, by building id, and the rules they follow.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interiors {
    /// The tuning (step and head heights).
    pub cfg: InteriorConfig,
    /// Built colliders.
    pub buildings: std::collections::BTreeMap<u32, Walkable>,
}

impl Interiors {
    /// The buildings that should have colliders now: none unless the pilot is on foot (owner,
    /// 2026-10-26: "I think you would generate detailed colliders for buildings only when the player
    /// is on foot"); on foot, every standing one whose footprint is within `collision_radius_m` of
    /// `here`, nearest first (ties by id).
    pub fn wanted(&self, on_foot: bool, here: Vec2, plan: &super::WorldPlan) -> Vec<(u32, f32)> {
        if !on_foot || !here.is_finite() {
            return Vec::new();
        }
        let r = self.cfg.collision_radius_m;
        let mut out: Vec<(u32, f32)> = plan
            .collision
            .candidates(here, r)
            .into_iter()
            .map(|b| (b, plan.buildings[b as usize].footprint.distance(here)))
            .filter(|(_, d)| *d <= r)
            .collect();
        out.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        out
    }

    /// Drops colliders that should go: all of them when the pilot is not on foot, else those past
    /// the radius plus the margin (never one the pilot stands in, which is at distance zero).
    pub fn prune(&mut self, on_foot: bool, here: Vec2, plan: &super::WorldPlan) {
        if !on_foot || !here.is_finite() {
            self.buildings.clear();
            return;
        }
        let keep = self.cfg.collision_radius_m + self.cfg.collision_margin_m;
        self.buildings
            .retain(|&b, _| plan.buildings.get(b as usize).is_some_and(|x| x.footprint.distance(here) <= keep));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(n: [usize; 3]) -> Site {
        Site {
            id: 11,
            n,
            origin: Vec3::new(10.0, 5.0, 20.0),
            size: Vec3::new(6.0, 3.5, 6.0),
            street: 0,
            fronts: [crate::world::FRONT_LIMIT_M; 4],
        }
    }

    fn storey(k: usize, n: [usize; 3]) -> StoreyPlan {
        plan_storey(&site(n), k, &InteriorConfig::default(), &SimRng::new(42))
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
    fn furniture_is_drawn_in_furniture_surfaces_never_as_structure() {
        let names = [
            "desk", "filing", "shelf", "sofa", "tv", "bed", "wardrobe", "counter", "fridge", "bath", "meeting", "gondola", "till",
            "crates", "plant",
        ];
        for name in names {
            let (boxes, _, _) = kit(name);
            assert!(!boxes.is_empty(), "{name} has boxes");
            for b in boxes {
                assert!(
                    !matches!(b.7, Surface::Concrete | Surface::Plaster),
                    "{name} has a box drawn as structure"
                );
            }
        }
        let p = storey(4, [5, 12, 4]);
        assert!(
            p.items.iter().flat_map(|it| &it.boxes).any(|b| b.3 == Surface::Wood),
            "an office storey has wooden furniture"
        );
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

    mod walking {
        //! The owner's request walked: in off the pavement through the street door, up the stair to
        //! the top storey (2026-10-25), on foot only, intact or breached (2026-10-26).
        use super::super::*;
        use crate::input::Actions;
        use crate::pilot::{PilotConfig, PilotState};
        use crate::world::destruct::Cells;
        use crate::world::{Deformation, Ground, Scene, Structures, WorldConfig, WorldPlan, generate};
        use std::sync::OnceLock;

        fn world() -> &'static (WorldPlan, WorldConfig) {
            static W: OnceLock<(WorldPlan, WorldConfig)> = OnceLock::new();
            W.get_or_init(|| {
                let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
                let cfg = crate::data::load_ron::<WorldConfig>(&p).unwrap();
                (generate(42, &cfg), cfg)
            })
        }

        fn seed() -> SimRng {
            SimRng::new(42).fork("dressing")
        }

        fn site_of(st: &Structures, b: usize) -> Site {
            let c: &Cells = &st.buildings[b];
            Site::of(&world().0, b as u32, c.n, c.origin, c.size)
        }

        /// Least distance from a segment to a rectangle (zero when they touch).
        fn segment_to_rect(a: Vec2, b: Vec2, r: &Rect) -> f32 {
            let steps = ((b - a).length() / 0.02).ceil().max(1.0) as usize;
            (0..=steps)
                .map(|i| r.distance(a.lerp(b, i as f32 / steps as f32)))
                .fold(f32::INFINITY, f32::min)
        }

        #[test]
        fn every_building_has_its_door_on_its_street_side_and_every_stair_is_walkable() {
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let ic = &cfg.interiors;
            assert_eq!(plan.street_sides.len(), plan.buildings.len());
            for b in 0..plan.buildings.len() {
                let site = site_of(&st, b);
                let sh = shell(&site, ic, &seed());
                assert_eq!(sh.entrance.side, plan.street_sides[b], "building {b}'s door is on its street side");
                let s = &sh.stair;
                assert!(s.rise_m <= ic.stair_max_rise_m + 1e-4, "building {b}: rise {}", s.rise_m);
                assert!(s.run_m >= ic.stair_min_run_m - 1e-4, "building {b}: run {}", s.run_m);
                assert!(s.lane_m >= 0.9, "building {b}: lane {}", s.lane_m);
                let top = s.floor_y(s.storeys - 1) - s.floor_y(0);
                let climbed: f32 = (0..s.storeys - 1).map(|_| s.rise_m * (2 * s.flight) as f32).sum();
                assert!((climbed - top).abs() < 1e-3, "building {b}'s stair reaches the top storey");
            }
        }

        #[test]
        fn every_door_in_the_capital_is_reached_from_the_pavement() {
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let city = &plan.cities[0];
            let (cell, r) = (0.5f32, PilotConfig::default().body_radius_m);
            let mut checked = 0;
            for block in city.blocks.iter().filter(|k| !k.park) {
                let ids: Vec<usize> = (0..plan.buildings.len())
                    .filter(|&b| plan.buildings[b].city == city.id && block.rect.distance(plan.buildings[b].footprint.centre()) == 0.0)
                    .collect();
                if ids.is_empty() {
                    continue;
                }
                let size = block.rect.size();
                let (w, h) = ((size.x / cell).ceil() as usize, (size.y / cell).ceil() as usize);
                let at = |i: usize, j: usize| block.rect.min + Vec2::new((i as f32 + 0.5) * cell, (j as f32 + 0.5) * cell);
                let mut free = vec![true; w * h];
                for &b in &ids {
                    let f = plan.buildings[b].footprint;
                    for j in 0..h {
                        for i in 0..w {
                            if f.distance(at(i, j)) < r {
                                free[j * w + i] = false;
                            }
                        }
                    }
                }
                let mut seen = vec![false; w * h];
                let mut todo = Vec::new();
                for j in 0..h {
                    for i in 0..w {
                        let p = at(i, j);
                        let edge = (p - block.rect.min).min(block.rect.max - p).min_element();
                        if edge < cfg.sidewalk_m && free[j * w + i] {
                            seen[j * w + i] = true;
                            todo.push((i, j));
                        }
                    }
                }
                while let Some((i, j)) = todo.pop() {
                    for (di, dj) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                        let (ni, nj) = (i as i64 + di, j as i64 + dj);
                        if ni < 0 || nj < 0 || ni as usize >= w || nj as usize >= h {
                            continue;
                        }
                        let n = nj as usize * w + ni as usize;
                        if free[n] && !seen[n] {
                            seen[n] = true;
                            todo.push((ni as usize, nj as usize));
                        }
                    }
                }
                for &b in &ids {
                    let e = shell(&site_of(&st, b), &cfg.interiors, &seed()).entrance;
                    let q = (e.route[0] - block.rect.min) / cell;
                    let (i, j) = ((q.x as usize).min(w - 1), (q.y as usize).min(h - 1));
                    assert!(
                        seen[j * w + i],
                        "building {b}'s door ({:?} on side {}) is reached from the pavement",
                        e.route[0],
                        e.side
                    );
                    checked += 1;
                }
            }
            assert!(checked > 50, "the capital's doors were walked to: {checked}");
        }

        #[test]
        fn nothing_stands_on_the_way_in() {
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let ic = &cfg.interiors;
            let body = PilotConfig::default().body_radius_m;
            for b in (0..plan.buildings.len()).step_by(3) {
                let site = site_of(&st, b);
                let w = Walkable::build(&site, ic, cfg.structure.column_m, &seed());
                let e = shell(&site, ic, &seed()).entrance;
                let y0 = site.origin.y;
                for seg in e.route.windows(2) {
                    for s in &w.solids {
                        // In the way of someone walking the ground floor: over a step, under a head.
                        if s.max.y <= y0 + e.sill_m + ic.step_m || s.min.y >= y0 + ic.head_m {
                            continue;
                        }
                        let r = Rect {
                            min: Vec2::new(s.min.x, s.min.z),
                            max: Vec2::new(s.max.x, s.max.z),
                        };
                        let d = segment_to_rect(seg[0], seg[1], &r);
                        assert!(d > body, "building {b}: {s:?} is {d} m from the way in {seg:?}");
                    }
                }
            }
        }

        /// A pilot walking `points` (feet) through `scene` at walking pace, steering at each in turn.
        /// Returns where it ended, or where it got stuck.
        fn walk(scene: &Scene, p: &mut PilotState, points: &[Vec2]) -> Result<(), String> {
            let cfg = PilotConfig::default();
            let fwd = Actions {
                move_xy: Vec2::new(0.0, 1.0),
                ..Default::default()
            };
            for (n, &goal) in points.iter().enumerate() {
                let mut best = f32::INFINITY;
                let mut since = 0;
                loop {
                    let here = Vec2::new(p.feet.x, p.feet.z);
                    let d = goal - here;
                    if d.length() < 0.25 {
                        break;
                    }
                    if d.length() < best - 0.02 {
                        best = d.length();
                        since = 0;
                    } else {
                        since += 1;
                        if since > 240 {
                            return Err(format!("stuck at {} on the way to point {n} {goal:?}", p.feet));
                        }
                    }
                    p.yaw_rad = (-d.x).atan2(-d.y);
                    p.step(&fwd, &cfg, scene, None, 1.0 / 60.0);
                }
            }
            Ok(())
        }

        fn interiors_for(st: &Structures, b: usize) -> Interiors {
            let (_, cfg) = world();
            let mut i = Interiors {
                cfg: cfg.interiors.clone(),
                ..Default::default()
            };
            i.buildings.insert(
                b as u32,
                Walkable::build(&site_of(st, b), &cfg.interiors, cfg.structure.column_m, &seed()),
            );
            i
        }

        fn scene<'a>(st: &'a Structures, dug: &'a Deformation, i: &'a Interiors) -> Scene<'a> {
            Scene {
                plan: &world().0,
                dug,
                trees: None,
                structures: Some(st),
                platforms: &[],
                interiors: Some(i),
            }
        }

        /// One building of each layout and door, low enough to climb in a test.
        fn samples(st: &Structures) -> Vec<usize> {
            let (plan, cfg) = world();
            let mut seen = Vec::new();
            let mut out = Vec::new();
            for b in 0..plan.buildings.len() {
                let site = site_of(st, b);
                if !(2..=7).contains(&site.n[1]) {
                    continue;
                }
                let sh = shell(&site, &cfg.interiors, &seed());
                let key = (sh.layout.double, sh.layout.street_end, sh.entrance.kind);
                if !seen.contains(&key) {
                    seen.push(key);
                    out.push(b);
                }
            }
            out
        }

        #[test]
        fn a_pilot_walks_in_off_the_street_and_climbs_to_the_top_storey() {
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let dug = Deformation::new(cfg.world_size_m);
            let picks = samples(&st);
            assert!(picks.len() >= 4, "the sample covers the layouts: {picks:?}");
            for b in picks {
                let site = site_of(&st, b);
                let sh = shell(&site, &cfg.interiors, &seed());
                let inside = interiors_for(&st, b);
                let sc = scene(&st, &dug, &inside);
                let start = sh.entrance.route[0];
                let mut p = PilotState::standing(
                    Vec3::new(start.x, sc.support_at(start.x, start.y, site.origin.y + 1.0), start.y),
                    0.0,
                );
                walk(&sc, &mut p, &sh.entrance.route[1..]).unwrap_or_else(|e| panic!("building {b} ({:?}): {e}", sh.entrance.kind));
                assert!(
                    site.footprint().distance(Vec2::new(p.feet.x, p.feet.z)) == 0.0,
                    "building {b}: inside"
                );
                for k in 0..site.n[1] - 1 {
                    let up: Vec<Vec2> = sh.stair.climb(k).iter().map(|q| Vec2::new(q.x, q.z)).collect();
                    walk(&sc, &mut p, &up).unwrap_or_else(|e| panic!("building {b}, storey {k}: {e}"));
                    let floor = sh.stair.floor_y(k + 1);
                    assert!(
                        (p.feet.y - floor).abs() < 0.05,
                        "building {b}: on storey {} at {} not {floor}",
                        k + 1,
                        p.feet.y
                    );
                }
            }
        }

        #[test]
        fn the_facade_stops_a_pilot_away_from_the_door() {
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let dug = Deformation::new(cfg.world_size_m);
            let b = samples(&st)[0];
            let site = site_of(&st, b);
            let e = shell(&site, &cfg.interiors, &seed()).entrance;
            let inside = interiors_for(&st, b);
            let sc = scene(&st, &dug, &inside);
            // Two metres along the facade from the door, walking straight at it.
            let along = if e.side < 2 { Vec2::X } else { Vec2::Y };
            let out = Site::out_of(e.side);
            let at = e.centre + along * (e.width_m * 0.5 + 1.5);
            let mut p = PilotState::standing(Vec3::new((at + out * 2.0).x, site.origin.y, (at + out * 2.0).y), 0.0);
            let _ = walk(&sc, &mut p, &[at - out * 2.0]);
            let feet = Vec2::new(p.feet.x, p.feet.z);
            assert!((feet - e.centre).dot(out) > 0.3, "building {b}: stopped outside at {feet:?}");
        }

        #[test]
        fn a_breach_is_a_way_in() {
            let (plan, cfg) = world();
            let mut st = Structures::new(plan);
            let dug = Deformation::new(cfg.world_size_m);
            let b = samples(&st)[0];
            let site = site_of(&st, b);
            let e = shell(&site, &cfg.interiors, &seed()).entrance;
            // Blow out the next bay along from the door, on the ground floor.
            let along = if e.side < 2 { Vec2::X } else { Vec2::Y };
            let out = Site::out_of(e.side);
            let cell = if e.side < 2 { site.size.x } else { site.size.z };
            let bay = if e.bay > 0 { -1.0 } else { 1.0 };
            let hole = e.centre - along * e.along_m + along * cell * (0.5 + bay);
            let (broke, _) = st.break_sphere(
                b as u32,
                Vec3::new(hole.x, site.origin.y + 1.5, hole.y) - Vec3::new(out.x, 0.0, out.y) * 0.3,
                2.0,
            );
            assert!(broke > 0, "the blast broke something");
            let inside = interiors_for(&st, b);
            let sc = scene(&st, &dug, &inside);
            let from = hole + out * 2.0;
            let mut p = PilotState::standing(Vec3::new(from.x, site.origin.y, from.y), 0.0);
            walk(&sc, &mut p, &[hole - out * 1.0]).unwrap_or_else(|e| panic!("building {b}: {e}"));
        }

        #[test]
        fn colliders_are_chosen_by_mode_and_distance() {
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let b = samples(&st)[0];
            let mut i = interiors_for(&st, b);
            let door = shell(&site_of(&st, b), &cfg.interiors, &seed()).entrance.route[0];
            assert!(i.wanted(false, door, plan).is_empty(), "none while not on foot");
            let near = i.wanted(true, door, plan);
            assert!(
                near.iter().any(|(x, _)| *x == b as u32),
                "on foot at its door, the building is wanted"
            );
            assert!(near.windows(2).all(|w| w[0].1 <= w[1].1), "nearest first");
            assert!(near.iter().all(|(_, d)| *d <= cfg.interiors.collision_radius_m));
            i.prune(true, door, plan);
            assert!(i.buildings.contains_key(&(b as u32)), "kept while near on foot");
            let far = door + Vec2::splat(cfg.interiors.collision_radius_m + cfg.interiors.collision_margin_m + 50.0);
            i.prune(true, far, plan);
            assert!(i.buildings.is_empty(), "freed once walked away from");
            let mut j = interiors_for(&st, b);
            j.prune(false, door, plan);
            assert!(j.buildings.is_empty(), "freed the moment the pilot boards");
        }

        #[test]
        fn climbing_out_switches_to_colliders_cleanly() {
            // Where the pilot stands at the start (as at the rope's foot after climbing out): with
            // every building within the sync radius built, they stand where they stood, on the same
            // ground, inside no box, and the first step on foot neither drops nor lifts them.
            let (plan, cfg) = world();
            let st = Structures::new(plan);
            let dug = Deformation::new(cfg.world_size_m);
            let here = plan.start.pilot_xz;
            let mut i = Interiors {
                cfg: cfg.interiors.clone(),
                ..Default::default()
            };
            let coarse = Scene {
                plan,
                dug: &dug,
                trees: None,
                structures: Some(&st),
                platforms: &[],
                interiors: None,
            };
            let ground = coarse.support_at(here.x, here.y, plan.terrain.height_at(here.x, here.y));
            for (b, d) in i.wanted(true, here, plan) {
                if d <= cfg.interiors.collision_sync_m {
                    let w = Walkable::build(&site_of(&st, b as usize), &cfg.interiors, cfg.structure.column_m, &seed());
                    i.buildings.insert(b, w);
                }
            }
            let fine = scene(&st, &dug, &i);
            assert_eq!(fine.push_out_at(here, 0.35, ground), here, "inside no box");
            assert!(
                (fine.support_at(here.x, here.y, ground) - ground).abs() < 0.03,
                "on the same ground"
            );
            let mut p = PilotState::standing(Vec3::new(here.x, ground, here.y), 0.0);
            p.step(&Actions::default(), &PilotConfig::default(), &fine, None, 1.0 / 60.0);
            assert!((p.feet.y - ground).abs() < 0.03, "the first step stands still");
        }
    }
}
