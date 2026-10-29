//! Every boat on the island (openspec change `harbours`): the harbours by the coastal cities, the sea lanes
//! between them, the freighters, tankers and fishing boats at sea, the boats on the lakes and the islet ferry;
//! how they float, are hit, sink, flee and are shoved by a mech; the standing rule for harming them; and the
//! timetable that makes them real round the pilot. Pure and seeded: the same plan, seed and pilot path give the
//! same boats in the same places. `cb-app::boats` draws them and wires them to the world. Tuning is
//! `data/world/ships.ron`.

pub mod boat;
pub mod float;
pub mod harbour;
pub mod harm;
pub mod lanes;
pub mod traffic;

pub use boat::{BOAT_CHUNK_BASE, Boat, BoatView, Errand, Neighbour, Phase};
pub use harbour::{Footprint, Harbour};
pub use lanes::{End, Route, SeaGrid};
pub use traffic::Traffic;

use crate::data::{Validate, finite3, positive, within};
use crate::world::Ground;
use glam::Vec2;
use serde::Deserialize;
use std::collections::BTreeMap;

/// One kind of boat, in data: its true size and how it handles, floats, takes hits and sinks.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoatSpec {
    /// Its size, x across (the beam), y from the keel to the masthead, z along (the length), metres.
    pub size_m: [f32; 3],
    /// How deep its keel sits under the waterline, metres.
    pub draft_m: f32,
    /// Its cruising speed, metres a second.
    pub speed_m_s: f32,
    /// How fast it gathers or loses way, metres a second squared.
    pub accel_m_s2: f32,
    /// Its fastest turn under way, degrees a second.
    pub turn_deg_s: f32,
    /// Hit points.
    pub hp: f32,
    /// Armour a round must beat to do damage, millimetres of rolled steel equivalent.
    pub armour_mm: f32,
    /// Civil crew aboard (0: none, an empty boat or a raider, whose harm costs no standing).
    pub crew: u32,
    /// Seconds from destroyed to the bottom.
    pub sink_s: f32,
    /// Its model, under `assets/` (the block-out).
    pub model: String,
    /// The block-out's hull colour, sRGB 0..1 (the stand-in drawn when the model is missing).
    pub tint: [f32; 3],
    /// A tanker: its sinking throws a fire.
    #[serde(default)]
    pub burns: bool,
}

impl BoatSpec {
    /// Its length, metres.
    pub fn length_m(&self) -> f32 {
        self.size_m[2]
    }

    /// Its beam, metres.
    pub fn beam_m(&self) -> f32 {
        self.size_m[0]
    }
}

/// Where harbours go and what is built in them, `harbours:` in `ships.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct HarbourConfig {
    /// At most this many harbours (survey HB2: four).
    pub most: usize,
    /// A city is coastal when berth-deep water lies within this of its square's edge, metres.
    pub coast_reach_m: f32,
    /// Water at the berth is at least this deep, metres.
    pub berth_depth_m: f32,
    /// The farthest the pier runs out from the shoreline to reach the berth, metres.
    pub pier_max_m: f32,
    /// The quay deck's least height above sea level, metres.
    pub deck_m: f32,
    /// ... and its most: ground higher than this under the apron is a cliff coast, with no harbour, metres.
    pub max_deck_m: f32,
    /// The apron along the shore and the pier head along the berth: their length, metres.
    pub quay_len_m: f32,
    /// The apron runs this far inland of the shoreline, metres.
    pub apron_m: f32,
    /// The pier's width, metres.
    pub pier_w_m: f32,
    /// The pier head's width, metres.
    pub head_w_m: f32,
    /// Cranes along the pier head.
    pub cranes: usize,
    /// A crane: its portal's width, its height and its jib's reach, metres.
    pub crane_m: [f32; 3],
    /// Warehouses behind the apron.
    pub warehouses: usize,
    /// A warehouse: along the shore, high, deep, metres.
    pub warehouse_m: [f32; 3],
    /// The harbour office: along the shore, high, deep, metres.
    pub office_m: [f32; 3],
    /// The office's terminal stands this far in front of its door, metres.
    pub terminal_out_m: f32,
    /// A harbour's shore keeps this far from an air base's site, metres.
    pub base_clear_m: f32,
    /// Harbours stand at least this far apart, metres.
    pub spacing_m: f32,
    /// The anchorage (the roads) lies this far out from the berth, metres.
    pub anchorage_m: f32,
    /// Ships at anchor in each harbour's roads.
    pub at_anchor: usize,
    /// Ships turn in at the mouth, this far out from the berth, and slow from there, metres.
    pub approach_m: f32,
    /// Fishing grounds lie this far off the harbour's mouth, least and most, metres.
    pub ground_m: (f32, f32),
}

impl Default for HarbourConfig {
    fn default() -> Self {
        Self {
            most: 4,
            coast_reach_m: 1800.0,
            berth_depth_m: 10.0,
            pier_max_m: 450.0,
            deck_m: 2.5,
            max_deck_m: 8.0,
            quay_len_m: 220.0,
            apron_m: 40.0,
            pier_w_m: 30.0,
            head_w_m: 30.0,
            cranes: 3,
            crane_m: [12.0, 40.0, 35.0],
            warehouses: 3,
            warehouse_m: [50.0, 12.0, 24.0],
            office_m: [16.0, 9.0, 12.0],
            terminal_out_m: 3.0,
            base_clear_m: 600.0,
            spacing_m: 3000.0,
            anchorage_m: 900.0,
            at_anchor: 2,
            approach_m: 1200.0,
            ground_m: (2000.0, 4000.0),
        }
    }
}

/// The sea lanes, `lanes:` in `ships.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LanesConfig {
    /// The sea grid's cell, metres.
    pub cell_m: f32,
    /// Lanes keep this far from land, metres.
    pub coast_clear_m: f32,
    /// Lanes run on water at least this deep, metres.
    pub lane_depth_m: f32,
    /// Lanes out to sea end this far from the island's centre (the offing), metres.
    pub offing_m: f32,
    /// Ships keep this far to the right of a lane's line, metres.
    pub keep_right_m: f32,
    /// An escort's freighter starts this far out along a lane from the harbour's mouth, metres.
    pub escort_out_m: f32,
}

impl Default for LanesConfig {
    fn default() -> Self {
        Self {
            cell_m: 200.0,
            coast_clear_m: 300.0,
            lane_depth_m: 12.0,
            offing_m: 17000.0,
            keep_right_m: 80.0,
            escort_out_m: 2500.0,
        }
    }
}

/// How many boats there are and where they are real, `traffic:` in `ships.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TrafficConfig {
    /// A timetabled boat is made real within this of the pilot, metres.
    pub spawn_m: f32,
    /// ... and stops being real past this, metres.
    pub despawn_m: f32,
    /// Lake boats are real within this of the pilot, metres.
    pub lake_spawn_m: f32,
    /// Ships on a sea lane run this far apart, least and most (drawn per lane), metres.
    pub headway_m: (f32, f32),
    /// The share of a lane's ships that are tankers (the rest are freighters).
    pub tanker_share: f32,
    /// Fishing boats a harbour.
    pub fishing_per_harbour: usize,
    /// A lake carries a boat for each this much of its radius, metres.
    pub lake_boat_radius_m: f32,
    /// ... and at most this many.
    pub lake_boats_most: usize,
    /// The share of lake boats that are launches (the rest row).
    pub launch_share: f32,
    /// A boat moors at a berth for this long, least and most, seconds.
    pub moor_s: (f32, f32),
    /// A fishing boat works its ground for this long, least and most, seconds.
    pub work_s: (f32, f32),
    /// A fishing boat works its ground at this speed, metres a second.
    pub work_m_s: f32,
    /// ... in a circle this wide, metres (radius).
    pub work_radius_m: f32,
    /// A ferry runs to an islet when the world has one.
    pub ferry: bool,
}

impl Default for TrafficConfig {
    fn default() -> Self {
        Self {
            spawn_m: 6000.0,
            despawn_m: 8000.0,
            lake_spawn_m: 3000.0,
            headway_m: (3000.0, 5000.0),
            tanker_share: 0.35,
            fishing_per_harbour: 3,
            lake_boat_radius_m: 150.0,
            lake_boats_most: 4,
            launch_share: 0.4,
            moor_s: (120.0, 240.0),
            work_s: (300.0, 600.0),
            work_m_s: 1.5,
            work_radius_m: 300.0,
            ferry: true,
        }
    }
}

/// How boats steer, `steer:` in `ships.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SteerConfig {
    /// A boat looks this much of its travel ahead for shallows, seconds.
    pub probe_s: f32,
    /// ... and keeps this much water under its keel, metres.
    pub keel_clear_m: f32,
    /// A boat slows behind another within this many of its own lengths.
    pub gap_lengths: f32,
    /// A boat comes alongside its berth at this speed, metres a second.
    pub berth_m_s: f32,
    /// A boat steers for the point this far past its own length ahead on its route, metres.
    pub lookahead_m: f32,
    /// A fleeing boat runs at this share of its cruising speed.
    pub flee_speed_share: f32,
}

impl Default for SteerConfig {
    fn default() -> Self {
        Self {
            probe_s: 20.0,
            keel_clear_m: 2.0,
            gap_lengths: 4.0,
            berth_m_s: 1.0,
            lookahead_m: 60.0,
            flee_speed_share: 1.25,
        }
    }
}

/// Boats in a fight, `fight:` in `ships.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FightConfig {
    /// A sinking boat lists this far, degrees.
    pub list_deg: f32,
    /// A wreck lies on the bottom this long, seconds.
    pub wreck_s: f32,
    /// An oil slick fades over this long, seconds.
    pub slick_s: f32,
    /// A mech shoves boats no longer than this, metres.
    pub shove_max_m: f32,
    /// ... and swamps one it meets faster than this, metres a second.
    pub swamp_m_s: f32,
    /// A boat flees an explosion within this, metres.
    pub flee_blast_m: f32,
    /// ... or a hostile within this, metres.
    pub flee_hostile_m: f32,
    /// ... for this long, seconds.
    pub flee_s: f32,
    /// A crab goes for a crewed ship within this, metres...
    pub crab_reach_m: f32,
    /// ... when the pilot is farther than this, metres.
    pub crab_pilot_m: f32,
}

impl Default for FightConfig {
    fn default() -> Self {
        Self {
            list_deg: 25.0,
            wreck_s: 600.0,
            slick_s: 300.0,
            shove_max_m: 25.0,
            swamp_m_s: 5.0,
            flee_blast_m: 600.0,
            flee_hostile_m: 400.0,
            flee_s: 60.0,
            crab_reach_m: 700.0,
            crab_pilot_m: 1500.0,
        }
    }
}

/// `data/world/ships.ron`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ShipsConfig {
    /// Harbours.
    pub harbours: HarbourConfig,
    /// Sea lanes.
    pub lanes: LanesConfig,
    /// The timetable.
    pub traffic: TrafficConfig,
    /// Steering.
    pub steer: SteerConfig,
    /// Fighting.
    pub fight: FightConfig,
    /// Every kind of boat by name.
    pub kinds: BTreeMap<String, BoatSpec>,
}

/// The kinds every world needs, by role.
pub const FREIGHTER: &str = "freighter";
/// A tanker.
pub const TANKER: &str = "tanker";
/// A fishing boat.
pub const FISHING: &str = "fishing_boat";
/// A rowing boat.
pub const ROWING: &str = "rowing_boat";
/// A motor launch.
pub const LAUNCH: &str = "launch";
/// The islet ferry.
pub const FERRY: &str = "ferry";
/// The raiders' gunboat (its fighting numbers are a vehicle's, in `units.ron`).
pub const GUNBOAT: &str = "gunboat";

fn ordered(field: &str, r: (f32, f32)) -> Result<(), String> {
    positive(field, r.0)?;
    positive(field, r.1)?;
    if r.1 < r.0 {
        return Err(format!("`{field}`: the most is less than the least"));
    }
    Ok(())
}

impl Validate for ShipsConfig {
    fn validate(&self) -> Result<(), String> {
        let h = &self.harbours;
        for (n, v) in [
            ("harbours.coast_reach_m", h.coast_reach_m),
            ("harbours.berth_depth_m", h.berth_depth_m),
            ("harbours.pier_max_m", h.pier_max_m),
            ("harbours.deck_m", h.deck_m),
            ("harbours.max_deck_m", h.max_deck_m),
            ("harbours.quay_len_m", h.quay_len_m),
            ("harbours.apron_m", h.apron_m),
            ("harbours.pier_w_m", h.pier_w_m),
            ("harbours.head_w_m", h.head_w_m),
            ("harbours.terminal_out_m", h.terminal_out_m),
            ("harbours.base_clear_m", h.base_clear_m),
            ("harbours.spacing_m", h.spacing_m),
            ("harbours.anchorage_m", h.anchorage_m),
            ("harbours.approach_m", h.approach_m),
        ] {
            positive(n, v)?;
        }
        finite3("harbours.crane_m", h.crane_m)?;
        finite3("harbours.warehouse_m", h.warehouse_m)?;
        finite3("harbours.office_m", h.office_m)?;
        for (n, v) in [("crane_m", h.crane_m), ("warehouse_m", h.warehouse_m), ("office_m", h.office_m)] {
            for (i, x) in v.iter().enumerate() {
                positive(&format!("harbours.{n}[{i}]"), *x)?;
            }
        }
        ordered("harbours.ground_m", h.ground_m)?;
        if h.anchorage_m >= h.approach_m {
            return Err("harbours: the anchorage must lie inside the mouth (anchorage_m < approach_m)".into());
        }
        let l = &self.lanes;
        for (n, v) in [
            ("lanes.cell_m", l.cell_m),
            ("lanes.coast_clear_m", l.coast_clear_m),
            ("lanes.lane_depth_m", l.lane_depth_m),
            ("lanes.offing_m", l.offing_m),
            ("lanes.escort_out_m", l.escort_out_m),
        ] {
            positive(n, v)?;
        }
        within("lanes.keep_right_m", l.keep_right_m, 0.0, 1000.0)?;
        let t = &self.traffic;
        positive("traffic.spawn_m", t.spawn_m)?;
        positive("traffic.lake_spawn_m", t.lake_spawn_m)?;
        if t.despawn_m <= t.spawn_m {
            return Err("traffic: despawn_m must be beyond spawn_m, or boats flicker at the edge".into());
        }
        ordered("traffic.headway_m", t.headway_m)?;
        ordered("traffic.moor_s", t.moor_s)?;
        ordered("traffic.work_s", t.work_s)?;
        within("traffic.tanker_share", t.tanker_share, 0.0, 1.0)?;
        within("traffic.launch_share", t.launch_share, 0.0, 1.0)?;
        positive("traffic.lake_boat_radius_m", t.lake_boat_radius_m)?;
        positive("traffic.work_m_s", t.work_m_s)?;
        positive("traffic.work_radius_m", t.work_radius_m)?;
        let s = &self.steer;
        for (n, v) in [
            ("steer.probe_s", s.probe_s),
            ("steer.keel_clear_m", s.keel_clear_m),
            ("steer.gap_lengths", s.gap_lengths),
            ("steer.berth_m_s", s.berth_m_s),
            ("steer.lookahead_m", s.lookahead_m),
            ("steer.flee_speed_share", s.flee_speed_share),
        ] {
            positive(n, v)?;
        }
        let f = &self.fight;
        within("fight.list_deg", f.list_deg, 0.0, 90.0)?;
        for (n, v) in [
            ("fight.wreck_s", f.wreck_s),
            ("fight.slick_s", f.slick_s),
            ("fight.shove_max_m", f.shove_max_m),
            ("fight.swamp_m_s", f.swamp_m_s),
            ("fight.flee_blast_m", f.flee_blast_m),
            ("fight.flee_hostile_m", f.flee_hostile_m),
            ("fight.flee_s", f.flee_s),
            ("fight.crab_reach_m", f.crab_reach_m),
            ("fight.crab_pilot_m", f.crab_pilot_m),
        ] {
            positive(n, v)?;
        }
        for (name, k) in &self.kinds {
            let f = |field: &str| format!("kinds.{name}.{field}");
            finite3(&f("size_m"), k.size_m)?;
            for (i, s) in k.size_m.iter().enumerate() {
                positive(&f(&format!("size_m[{i}]")), *s)?;
            }
            positive(&f("draft_m"), k.draft_m)?;
            if k.draft_m >= k.size_m[1] {
                return Err(format!("`{}` must be less than its height", f("draft_m")));
            }
            for (n, v) in [
                ("speed_m_s", k.speed_m_s),
                ("accel_m_s2", k.accel_m_s2),
                ("turn_deg_s", k.turn_deg_s),
                ("hp", k.hp),
                ("sink_s", k.sink_s),
            ] {
                positive(&f(n), v)?;
            }
            within(&f("armour_mm"), k.armour_mm, 0.0, 10_000.0)?;
            finite3(&f("tint"), k.tint)?;
            if k.model.is_empty() {
                return Err(format!("`{}` must name a model", f("model")));
            }
        }
        for need in [FREIGHTER, TANKER, FISHING, ROWING, LAUNCH, FERRY, GUNBOAT] {
            if !self.kinds.contains_key(need) {
                return Err(format!("kinds: no `{need}`"));
            }
        }
        Ok(())
    }
}

/// The water the boats sail on: the world's ground and sea level.
pub struct Sea<'a> {
    /// The ground, with the lakes and the sea (`Ground::water_at`).
    pub ground: &'a dyn Ground,
    /// The sea's level, metres.
    pub sea_level_m: f32,
}

impl Sea<'_> {
    /// How far under the sea's surface the ground is at `p` (negative on land), metres.
    pub fn sea_depth(&self, p: Vec2) -> f32 {
        self.sea_level_m - self.ground.height_at(p.x, p.y)
    }

    /// The water at `p`, sea or lake: its surface and depth, metres; `None` on dry ground.
    pub fn water(&self, p: Vec2) -> Option<(f32, f32)> {
        self.ground.water_at(p.x, p.y).map(|w| (w.surface_m, w.depth_m))
    }

    /// The water's depth at `p` (0 on dry ground), metres.
    pub fn depth(&self, p: Vec2) -> f32 {
        self.water(p).map_or(0.0, |w| w.1)
    }
}

/// The way a boat faces: heading 0 looks along -z, turning left is positive (`units::forward`, flat).
pub fn ahead(heading_rad: f32) -> Vec2 {
    Vec2::new(-heading_rad.sin(), -heading_rad.cos())
}

/// The heading that faces along `dir`.
pub fn heading_of(dir: Vec2) -> f32 {
    (-dir.x).atan2(-dir.y)
}

/// The angle from `a` to `b`, wrapped to -pi..pi.
pub fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI
}

#[cfg(test)]
pub(crate) mod test_world {
    //! A small round island in a deep sea, with a lake, for the rules' tests; and the shipped data.
    use super::*;
    use crate::world::WaterSample;

    /// An island of radius 3 km at the origin rising to 100 m, the sea 60 m deep past 4 km, a lake 300 m across at
    /// (1000, 0) 6 m deep with its surface at 40 m; the sea level is zero.
    pub struct Island;

    impl Island {
        pub fn ground_at(p: Vec2) -> f32 {
            let r = p.length();
            let lake = p.distance(Vec2::new(1000.0, 0.0));
            if lake < 300.0 {
                return 34.0 + lake / 300.0 * 6.0;
            }
            if r < 3000.0 {
                40.0 + (3000.0 - r) / 30.0
            } else if r < 4000.0 {
                40.0 - (r - 3000.0) / 1000.0 * 100.0
            } else {
                -60.0
            }
        }
    }

    impl Ground for Island {
        fn height_at(&self, x: f32, z: f32) -> f32 {
            Self::ground_at(Vec2::new(x, z))
        }
        fn push_out(&self, p: Vec2, _: f32) -> Vec2 {
            p
        }
        fn water_at(&self, x: f32, z: f32) -> Option<WaterSample> {
            let p = Vec2::new(x, z);
            let g = Self::ground_at(p);
            let surface = if p.distance(Vec2::new(1000.0, 0.0)) < 300.0 { 40.0 } else { 0.0 };
            (g < surface).then_some(WaterSample {
                surface_m: surface,
                depth_m: surface - g,
                flow_m_s: Vec2::ZERO,
            })
        }
    }

    pub fn sea() -> Sea<'static> {
        Sea {
            ground: &Island,
            sea_level_m: 0.0,
        }
    }

    pub fn shipped() -> ShipsConfig {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/ships.ron");
        crate::data::load_ron(&p).expect("ships.ron loads")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_data_loads_with_every_kind_the_world_needs() {
        let c = test_world::shipped();
        for k in [FREIGHTER, TANKER, FISHING, ROWING, LAUNCH, FERRY, GUNBOAT] {
            assert!(c.kinds.contains_key(k), "{k}");
        }
    }

    #[test]
    fn boats_are_at_their_true_sizes() {
        let c = test_world::shipped();
        let len = |k: &str| c.kinds[k].length_m();
        assert_eq!(len(FREIGHTER), 120.0);
        assert_eq!(len(TANKER), 150.0);
        assert_eq!(len(FISHING), 22.0);
        assert_eq!(len(ROWING), 4.2);
        assert_eq!(len(LAUNCH), 8.0);
        assert_eq!(len(FERRY), 60.0);
        assert_eq!(len(GUNBOAT), 30.0);
    }

    #[test]
    fn a_despawn_ring_inside_the_spawn_ring_is_refused() {
        let mut c = test_world::shipped();
        c.traffic.despawn_m = c.traffic.spawn_m;
        assert!(c.validate().unwrap_err().contains("despawn_m"));
    }

    #[test]
    fn the_sea_reads_depth_off_the_ground() {
        let s = test_world::sea();
        assert_eq!(s.sea_depth(Vec2::new(5000.0, 0.0)), 60.0);
        assert!(s.sea_depth(Vec2::ZERO) < 0.0);
        assert_eq!(s.water(Vec2::new(1000.0, 0.0)), Some((40.0, 6.0)), "the lake's own level");
        assert_eq!(s.depth(Vec2::new(0.0, 500.0)), 0.0, "dry");
    }

    #[test]
    fn ahead_and_heading_of_agree() {
        for h in [-2.0f32, -0.5, 0.0, 1.0, 3.0] {
            assert!(wrap(heading_of(ahead(h)) - h).abs() < 1e-5, "{h}");
        }
        assert!(ahead(0.0).distance(Vec2::new(0.0, -1.0)) < 1e-6);
    }
}
