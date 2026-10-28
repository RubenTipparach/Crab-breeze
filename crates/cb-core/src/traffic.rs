//! Car traffic (openspec change `car-traffic`, design section 2): civilian cars that drive the lane
//! graph (`world::lanes`) near the pilot, keep their gap, take turns at junctions, and react to the
//! fight: they brake and stop near shelling or a moving mech, a mech that walks into one shoves it
//! aside, a foot that comes down on one crushes it, and a hit that beats its armour sets it alight.
//! Pure and fixed step, seeded: the same seed, data and events drive the same traffic.
//!
//! A car you take (`car-driving`) stays in this list as a free body (`car::CarBody`), driven by the
//! player's input, so it is hit, crushed and shoved by the same rules as every other car.
//!
//! Harm to a car is reported (`Harm`) with whether the player's own fire or feet did it, so the
//! career's standing with the nearest base (survey W34) can be wired to it without this module
//! knowing about careers.

use crate::car::{CarBody, CarSpec, Drive, Obstacle, forward2};
use crate::data::{Validate, positive, positive_range, within};
use crate::rng::{SimRng, label_hash};
use crate::world::Ground;
use crate::world::lanes::{Lane, LaneGraph, LaneKind, LaneLayout, right_of};
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

/// How far each kind of a car's parts is drawn, metres, each dithering out over the last `fade_share` of its range
/// (city-frame): the cab's insides, then the lamps and bumpers, then the wheels; the body and cabin to the traffic's
/// edge. Only the body and cabin cast the sun's shadow. A car is 16 meshes up close and 2 past `wheels_m`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CarDetail {
    /// The dashboard, the speedometer and its needle, the steering wheel, metres.
    pub interior_m: f32,
    /// Head and tail lamps, bumpers, metres.
    pub trim_m: f32,
    /// The wheels, metres.
    pub wheels_m: f32,
    /// The share of each range, at its far end, over which the parts dither out.
    pub fade_share: f32,
}

impl Default for CarDetail {
    fn default() -> Self {
        Self {
            interior_m: 30.0,
            trim_m: 200.0,
            wheels_m: 400.0,
            fade_share: 0.15,
        }
    }
}

impl CarDetail {
    fn validate(&self) -> Result<(), String> {
        for (k, v) in [
            ("interior_m", self.interior_m),
            ("trim_m", self.trim_m),
            ("wheels_m", self.wheels_m),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("`{k}` must be above 0, got {v}"));
            }
        }
        if !(self.interior_m <= self.trim_m && self.trim_m <= self.wheels_m) {
            return Err("ranges must run `interior_m` <= `trim_m` <= `wheels_m`".into());
        }
        if !(0.0..1.0).contains(&self.fade_share) {
            return Err(format!("`fade_share` must be 0 to under 1, got {}", self.fade_share));
        }
        Ok(())
    }
}

/// Traffic tuning, loaded from `data/world/traffic.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TrafficConfig {
    /// The car kinds in traffic, each `data/cars/<kind>.ron`, in this order.
    pub kinds: Vec<String>,
    /// Every car's heights are raised by this on load, lengths and widths kept: the body, the roof, the wheels, the
    /// ride height and the driver's eye (`CarSpec::raised`). The model generator (`tools/blender/build_cars.py`)
    /// reads the same number, so the drawn car and the simulated one stay one car. 1 is the real-world sizes in
    /// `data/cars/`; the owner found those short in the game (2026-10-26: "cars are a little too short I think lol").
    pub car_height_scale: f32,
    /// How lanes sit on their roads.
    pub lanes: LaneLayout,
    /// One car per this much city lane (survey J6), metres.
    pub city_spacing_m: f32,
    /// One car per this much highway lane (survey J6), metres.
    pub highway_spacing_m: f32,
    /// One parked car per this much kerb on a city street, metres.
    pub parked_spacing_m: f32,
    /// Lanes within this of the pilot's eye are kept filled, metres.
    pub spawn_within_m: f32,
    /// Cars farther than this from the eye are dropped, metres.
    pub despawn_beyond_m: f32,
    /// In view, a car may appear only farther than this and behind cover, metres.
    pub hidden_beyond_m: f32,
    /// How far a car's small parts are drawn (city-frame).
    pub detail: CarDetail,
    /// At most this many cars appear in one step.
    pub spawn_per_step: u32,
    /// Each city car's own cruising speed is drawn from this range, km/h.
    pub city_speed_km_h: [f32; 2],
    /// Each highway car's, km/h.
    pub highway_speed_km_h: [f32; 2],
    /// Speed through a turn at a junction, km/h.
    pub turn_speed_km_h: f32,
    /// A connector turning more than this is a turn (slower), degrees.
    pub turn_from_deg: f32,
    /// Following (intelligent driver model): the car's gentle acceleration, m/s2.
    pub idm_accel_m_s2: f32,
    /// Following: comfortable braking, m/s2.
    pub idm_brake_m_s2: f32,
    /// Following: the time gap kept to the car ahead, seconds.
    pub gap_s: f32,
    /// Following: the gap kept at a standstill, metres.
    pub min_gap_m: f32,
    /// Following: the hardest a driver ever brakes, m/s2.
    pub max_brake_m_s2: f32,
    /// A driver looks this far ahead for a car or something in the road, metres.
    pub lookahead_m: f32,
    /// Within this of a junction a car asks for its turn, metres.
    pub approach_m: f32,
    /// A waiting car stops this far short of the lane's end, metres.
    pub stop_short_m: f32,
    /// A car within this of an explosion brakes and stops, metres.
    pub fear_blast_m: f32,
    /// A car within this of a moving mech brakes and stops, metres.
    pub fear_mech_m: f32,
    /// A mech faster than this is moving, m/s.
    pub mech_moving_m_s: f32,
    /// How hard a frightened driver brakes, m/s2.
    pub fear_brake_m_s2: f32,
    /// A frightened driver stays stopped for a time drawn from this range, seconds.
    pub fear_stop_s: [f32; 2],
    /// A car a moving mech walks into is shoved aside at this speed, m/s.
    pub push_speed_m_s: f32,
    /// And loses this much health (1 whole, 0 wrecked).
    pub push_damage: f32,
    /// A foot crushes every car within this of the sole's middle, metres.
    pub foot_crush_radius_m: f32,
    /// A blast sets cars alight within this times the cube root of its energy in megajoules, metres.
    pub blast_burn_m_per_cbrt_mj: f32,
    /// A car burns this long before it is a burnt-out shell, seconds.
    pub burn_s: f32,
    /// A wreck stays this long, seconds (or until the pilot is past `despawn_beyond_m`).
    pub wreck_life_s: f32,
    /// A driver who leaves their car runs at this speed, m/s.
    pub runner_speed_m_s: f32,
    /// A running driver is gone after this long, seconds.
    pub runner_life_s: f32,
    /// The traffic lights' phases where an arterial meets other roads (survey W39).
    pub lights: LightTimes,
}

/// How long each phase of a junction's traffic lights shows each colour (survey W39, owner 2026-10-26: "Lights on the
/// arterials"). A junction's two phases take turns: one shows green, then amber, then both show red for `all_red_s`,
/// then the other phase does the same. Each junction is offset along the cycle by its id, so neighbours do not all
/// change at once.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LightTimes {
    /// Green, seconds.
    pub green_s: f32,
    /// Amber, seconds.
    pub amber_s: f32,
    /// Red both ways before the other phase's green, seconds.
    pub all_red_s: f32,
}

impl Default for LightTimes {
    fn default() -> Self {
        Self {
            green_s: 24.0,
            amber_s: 3.0,
            all_red_s: 2.0,
        }
    }
}

impl LightTimes {
    /// One whole cycle, both phases, seconds.
    pub fn cycle_s(&self) -> f32 {
        2.0 * (self.green_s + self.amber_s + self.all_red_s)
    }
}

/// What a traffic light shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    /// Go.
    Green,
    /// Stop if you can.
    Amber,
    /// Stop.
    Red,
}

/// What the light of phase `phase` at junction `node` shows at traffic time `t_s` (survey W39). The one rule the cars
/// obey and the lamps the game draws show.
pub fn light(times: &LightTimes, node: u32, phase: u8, t_s: f32) -> Light {
    let half = times.green_s + times.amber_s + times.all_red_s;
    let cycle = 2.0 * half;
    // the golden ratio spreads the junctions' offsets evenly round the cycle
    let offset = (node as f32 * 0.618_034).fract() * cycle;
    let t = (t_s + offset - f32::from(phase.min(1)) * half).rem_euclid(cycle);
    if t < times.green_s {
        Light::Green
    } else if t < times.green_s + times.amber_s {
        Light::Amber
    } else {
        Light::Red
    }
}

impl Default for TrafficConfig {
    fn default() -> Self {
        Self {
            kinds: vec!["sedan".into()],
            car_height_scale: 1.0,
            lanes: LaneLayout::default(),
            city_spacing_m: 60.0,
            highway_spacing_m: 300.0,
            parked_spacing_m: 90.0,
            spawn_within_m: 1500.0,
            despawn_beyond_m: 2000.0,
            hidden_beyond_m: 400.0,
            detail: CarDetail::default(),
            spawn_per_step: 4,
            city_speed_km_h: [40.0, 60.0],
            highway_speed_km_h: [90.0, 110.0],
            turn_speed_km_h: 20.0,
            turn_from_deg: 20.0,
            idm_accel_m_s2: 1.5,
            idm_brake_m_s2: 2.5,
            gap_s: 2.0,
            min_gap_m: 2.5,
            max_brake_m_s2: 9.0,
            lookahead_m: 150.0,
            approach_m: 30.0,
            stop_short_m: 1.0,
            fear_blast_m: 120.0,
            fear_mech_m: 60.0,
            mech_moving_m_s: 0.5,
            fear_brake_m_s2: 7.0,
            fear_stop_s: [20.0, 40.0],
            push_speed_m_s: 6.0,
            push_damage: 0.4,
            foot_crush_radius_m: 2.5,
            blast_burn_m_per_cbrt_mj: 4.0,
            burn_s: 120.0,
            wreck_life_s: 600.0,
            runner_speed_m_s: 5.0,
            runner_life_s: 40.0,
            lights: LightTimes::default(),
        }
    }
}

impl Validate for TrafficConfig {
    fn validate(&self) -> Result<(), String> {
        if self.kinds.is_empty() {
            return Err("`kinds` needs at least one car".into());
        }
        within("car_height_scale", self.car_height_scale, 0.5, 2.0)?;
        self.lanes.validate().map_err(|e| format!("lanes: {e}"))?;
        positive("city_spacing_m", self.city_spacing_m)?;
        positive("highway_spacing_m", self.highway_spacing_m)?;
        positive("parked_spacing_m", self.parked_spacing_m)?;
        positive("spawn_within_m", self.spawn_within_m)?;
        positive("despawn_beyond_m", self.despawn_beyond_m)?;
        if self.despawn_beyond_m <= self.spawn_within_m {
            return Err("`despawn_beyond_m` must be beyond `spawn_within_m`".into());
        }
        positive("hidden_beyond_m", self.hidden_beyond_m)?;
        self.detail.validate().map_err(|e| format!("detail: {e}"))?;
        positive_range("city_speed_km_h", self.city_speed_km_h)?;
        positive_range("highway_speed_km_h", self.highway_speed_km_h)?;
        positive("turn_speed_km_h", self.turn_speed_km_h)?;
        within("turn_from_deg", self.turn_from_deg, 0.0, 180.0)?;
        positive("idm_accel_m_s2", self.idm_accel_m_s2)?;
        positive("idm_brake_m_s2", self.idm_brake_m_s2)?;
        positive("gap_s", self.gap_s)?;
        positive("min_gap_m", self.min_gap_m)?;
        positive("max_brake_m_s2", self.max_brake_m_s2)?;
        positive("lookahead_m", self.lookahead_m)?;
        positive("approach_m", self.approach_m)?;
        within("stop_short_m", self.stop_short_m, 0.0, 20.0)?;
        positive("fear_blast_m", self.fear_blast_m)?;
        positive("fear_mech_m", self.fear_mech_m)?;
        within("mech_moving_m_s", self.mech_moving_m_s, 0.0, 50.0)?;
        positive("fear_brake_m_s2", self.fear_brake_m_s2)?;
        positive_range("fear_stop_s", self.fear_stop_s)?;
        positive("push_speed_m_s", self.push_speed_m_s)?;
        within("push_damage", self.push_damage, 0.0, 1.0)?;
        positive("foot_crush_radius_m", self.foot_crush_radius_m)?;
        positive("blast_burn_m_per_cbrt_mj", self.blast_burn_m_per_cbrt_mj)?;
        positive("burn_s", self.burn_s)?;
        positive("wreck_life_s", self.wreck_life_s)?;
        positive("runner_speed_m_s", self.runner_speed_m_s)?;
        positive("runner_life_s", self.runner_life_s)?;
        positive("lights.green_s", self.lights.green_s)?;
        positive("lights.amber_s", self.lights.amber_s)?;
        positive("lights.all_red_s", self.lights.all_red_s)
    }
}

/// The traffic's data: its tuning and every car kind in it, in `kinds` order.
#[derive(Clone, Debug, PartialEq)]
pub struct TrafficData {
    /// `data/world/traffic.ron`.
    pub cfg: TrafficConfig,
    /// `data/cars/<kind>.ron` for each of `cfg.kinds`.
    pub kinds: Vec<CarSpec>,
}

impl TrafficData {
    /// Loads `world/traffic.ron` and each car it names from `cars/` under `root`.
    pub fn load(root: &std::path::Path) -> Result<Self, crate::data::DataError> {
        let path = root.join("world/traffic.ron");
        let cfg: TrafficConfig = crate::data::load_ron(&path)?;
        let kinds = cfg
            .kinds
            .iter()
            .map(|k| crate::data::load_ron::<CarSpec>(&root.join(format!("cars/{k}.ron"))).map(|c| c.raised(cfg.car_height_scale)))
            .collect::<Result<Vec<_>, _>>()?;
        let d = Self { cfg, kinds };
        d.check().map_err(|reason| crate::data::DataError { path, reason })?;
        Ok(d)
    }

    /// Both city and highway traffic have a kind to draw from.
    pub fn check(&self) -> Result<(), String> {
        if !self.kinds.iter().any(|k| k.city_weight > 0.0) {
            return Err("no car kind has a `city_weight` above zero".into());
        }
        if !self.kinds.iter().any(|k| k.highway_weight > 0.0) {
            return Err("no car kind has a `highway_weight` above zero".into());
        }
        Ok(())
    }

    /// The kind by name.
    pub fn kind(&self, name: &str) -> Option<usize> {
        self.cfg.kinds.iter().position(|k| k == name)
    }
}

/// What a car is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarState {
    /// Driving its lane (stopping and stopped too: see `Car::fear_s` and `Car::hailed`).
    Driving,
    /// Parked at the kerb beside its lane.
    Parked,
    /// The player is driving it.
    Driven,
    /// Off the lanes with nobody driving: left by the player, or shoved off its lane.
    Loose,
    /// Flattened under a mech's foot; a wreck.
    Crushed,
    /// On fire, for this much longer, seconds; a wreck.
    Burning {
        /// Seconds of fire left.
        left_s: f32,
    },
    /// Burnt out; a wreck.
    Burnt,
}

impl CarState {
    /// A wreck: crushed, burning or burnt out.
    pub fn wreck(self) -> bool {
        matches!(self, Self::Crushed | Self::Burning { .. } | Self::Burnt)
    }
}

/// One car.
#[derive(Clone, Debug, PartialEq)]
pub struct Car {
    /// Stable id, in the order cars appear.
    pub id: u32,
    /// Its kind, an index into `TrafficData::kinds`.
    pub kind: u16,
    /// Its paint, an index into its kind's `paints`.
    pub paint: u8,
    /// What it is doing.
    pub state: CarState,
    /// The lane it is on (on a free body, the lane it was last on).
    pub lane: u32,
    /// Distance along the lane, metres.
    pub s_m: f32,
    /// Where it goes at the lane's end: a connector, or after a connector the lane it leads into.
    pub next: Option<u32>,
    /// Speed along the lane, m/s.
    pub speed_m_s: f32,
    /// Its own cruising speed on open road, m/s.
    pub cruise_m_s: f32,
    /// Stopped out of fear for this much longer, seconds (0: not afraid).
    pub fear_s: f32,
    /// Asked to stop by a pilot who wants it.
    pub hailed: bool,
    /// When it reached its junction's approach, seconds of traffic time.
    pub arrived_s: Option<f32>,
    /// Given its turn across the junction ahead.
    pub granted: bool,
    /// Junctions it has crossed (seeds its turn choices).
    pub turns: u32,
    /// Ground point under its middle (x, y, z).
    pub pos: Vec3,
    /// Heading, radians (0 faces -z, positive turns left).
    pub heading_rad: f32,
    /// Off the lanes: its body (driven, loose, or a wreck that was loose).
    pub body: Option<CarBody>,
    /// Health, 1 whole to 0 wrecked.
    pub health: f32,
    /// Seconds since it became a wreck.
    pub wreck_s: f32,
}

impl Car {
    /// Its velocity on the ground (x, z), m/s.
    pub fn vel(&self) -> Vec2 {
        match &self.body {
            Some(b) => b.vel,
            None => forward2(self.heading_rad) * self.speed_m_s,
        }
    }

    /// True while it is on its lane and following it.
    fn on_lane(&self) -> bool {
        self.body.is_none() && self.state != CarState::Parked
    }

    /// Somebody is in it who is not the player: a traffic car driving or stopped on its lane.
    pub fn occupied(&self) -> bool {
        self.state == CarState::Driving
    }
}

/// A driver who left their car and is running for cover.
#[derive(Clone, Debug, PartialEq)]
pub struct Runner {
    /// The car they left.
    pub car: u32,
    /// Where they are (x, z).
    pub at: Vec2,
    /// Where they run to (x, z): the nearest building's door.
    pub to: Vec2,
    /// Seconds since they got out.
    pub age_s: f32,
}

/// What harmed a car.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarmCause {
    /// A shot that beat its armour.
    Shot,
    /// A blast that set it alight.
    Blast,
    /// A mech's foot.
    Crushed,
    /// A mech walked into it, or the player's car rammed it.
    Shoved,
}

/// A car was harmed. The career can charge standing for `by_player` harm to an `occupied` car
/// (survey W34) without this module knowing about careers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Harm {
    /// The car.
    pub car: u32,
    /// Where.
    pub at: Vec3,
    /// What did it.
    pub cause: HarmCause,
    /// The player's own fire, feet or car did it.
    pub by_player: bool,
    /// Somebody other than the player was in it.
    pub occupied: bool,
}

/// A mech near the traffic this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MechNear {
    /// Its middle on the ground (x, z).
    pub at: Vec2,
    /// Its velocity on the ground (x, z), m/s.
    pub vel: Vec2,
    /// Its body's radius, metres: a car inside it is shoved aside.
    pub radius_m: f32,
    /// The player's mech.
    pub piloted: bool,
}

/// What the traffic sees this step besides itself.
pub struct TrafficView<'a, G: Ground> {
    /// The lanes.
    pub graph: &'a LaneGraph,
    /// The cars and their tuning.
    pub data: &'a TrafficData,
    /// The ground, buildings and all.
    pub ground: &'a G,
    /// The mechs nearby.
    pub mechs: &'a [MechNear],
    /// What else is in the road: people on foot, a mech's legs; cars brake for them and a free body
    /// collides with them.
    pub obstacles: &'a [Obstacle],
    /// The player's input to the car they drive, if any.
    pub driver: Option<Drive>,
}

/// Every car near the pilot.
#[derive(Clone, Debug)]
pub struct Traffic {
    /// The cars, in id order.
    pub cars: Vec<Car>,
    /// Drivers running from their cars.
    pub runners: Vec<Runner>,
    next_id: u32,
    rng: SimRng,
    seed: u64,
    time_s: f32,
    harms: Vec<Harm>,
    driver_thrown: bool,
    /// Road lanes within reach of the last refill's eye, with the driving and parked cars each wants there and
    /// the stretch of the lane (from, to, metres along it) that is in reach, and where that eye was.
    near: Vec<(u32, u32, u32, f32, f32)>,
    near_from: Option<Vec2>,
    /// Each lane's bounds (least, greatest corner), built on first use.
    bounds: Vec<(Vec2, Vec2)>,
    /// The ground's height at each point of a lane, read the first time a car is placed on it: the ground
    /// is procedural and costly to read, and a lane's road surface does not move.
    heights: Vec<Option<Vec<f32>>>,
}

/// The player's input this step, as the drive a traffic car's body takes.
const PARKED: Drive = Drive {
    throttle: 0.0,
    steer: 0.0,
    handbrake: true,
};

/// A car is as long as its kind.
fn length_of(data: &TrafficData, car: &Car) -> f32 {
    data.kinds[car.kind as usize].size_m[2]
}

impl Traffic {
    /// Empty traffic seeded from the world's seed.
    pub fn new(seed: u64) -> Self {
        Self {
            cars: Vec::new(),
            runners: Vec::new(),
            next_id: 0,
            rng: SimRng::new(seed).fork("traffic"),
            seed,
            time_s: 0.0,
            harms: Vec::new(),
            driver_thrown: false,
            near: Vec::new(),
            near_from: None,
            bounds: Vec::new(),
            heights: Vec::new(),
        }
    }

    /// The car with this id.
    pub fn get(&self, id: u32) -> Option<&Car> {
        self.cars.binary_search_by_key(&id, |c| c.id).ok().map(|i| &self.cars[i])
    }

    fn get_mut(&mut self, id: u32) -> Option<&mut Car> {
        self.cars.binary_search_by_key(&id, |c| c.id).ok().map(|i| &mut self.cars[i])
    }

    /// Traffic time, seconds: what the traffic lights run on (`light`).
    pub fn time_s(&self) -> f32 {
        self.time_s
    }

    /// The car the player drives, if any.
    pub fn driven(&self) -> Option<&Car> {
        self.cars.iter().find(|c| c.state == CarState::Driven)
    }

    /// The harm done since the last call, in the order it happened.
    pub fn take_harms(&mut self) -> Vec<Harm> {
        std::mem::take(&mut self.harms)
    }

    /// True once after the car the player drives was wrecked under them (they are thrown clear).
    pub fn take_driver_thrown(&mut self) -> bool {
        std::mem::take(&mut self.driver_thrown)
    }

    /// A stable 64-bit summary of every car, quantised to centimetres, for golden-seed tests.
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        let mut put = |v: i64| {
            for b in v.to_le_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        for c in &self.cars {
            put(i64::from(c.id));
            put(i64::from(c.lane));
            put((c.pos.x * 100.0).round() as i64);
            put((c.pos.z * 100.0).round() as i64);
            put((c.speed_m_s * 100.0).round() as i64);
        }
        h
    }

    fn harm(&mut self, i: usize, cause: HarmCause, by_player: bool) {
        let c = &self.cars[i];
        self.harms.push(Harm {
            car: c.id,
            at: c.pos,
            cause,
            by_player,
            occupied: c.occupied(),
        });
        if c.state == CarState::Driven {
            self.driver_thrown = true;
        }
    }

    // ----- what the fight does to cars -------------------------------------------------------

    /// A mech's foot comes down with its sole's middle at `sole` (x, z): every car under it is
    /// crushed flat and stays as a wreck. Returns how many.
    pub fn crush(&mut self, data: &TrafficData, sole: Vec2, by_player: bool) -> usize {
        let r = data.cfg.foot_crush_radius_m;
        let mut n = 0;
        for i in 0..self.cars.len() {
            let c = &self.cars[i];
            if matches!(c.state, CarState::Crushed) {
                continue;
            }
            let spec = &data.kinds[c.kind as usize];
            if !under_foot(c, spec, sole, r) {
                continue;
            }
            self.harm(i, HarmCause::Crushed, by_player);
            let c = &mut self.cars[i];
            c.state = CarState::Crushed;
            c.speed_m_s = 0.0;
            c.health = 0.0;
            c.wreck_s = 0.0;
            if let Some(b) = &mut c.body {
                b.vel = Vec2::ZERO;
            }
            n += 1;
        }
        n
    }

    /// A shot that beats `penetration_mm` against the car's armour sets it alight. True when it did.
    pub fn hit(&mut self, data: &TrafficData, car: u32, penetration_mm: f32, by_player: bool) -> bool {
        let Ok(i) = self.cars.binary_search_by_key(&car, |c| c.id) else {
            return false;
        };
        let c = &self.cars[i];
        if c.state.wreck() || penetration_mm <= data.kinds[c.kind as usize].armour_mm {
            return false;
        }
        self.harm(i, HarmCause::Shot, by_player);
        self.ignite(i, &data.cfg);
        true
    }

    /// An explosion of `energy_mj` at `at`: cars within its reach catch fire, and every driver within
    /// `fear_blast_m` brakes and stops. Returns how many caught fire.
    pub fn blast(&mut self, data: &TrafficData, at: Vec3, energy_mj: f32, by_player: bool) -> usize {
        if !(at.is_finite() && energy_mj.is_finite() && energy_mj > 0.0) {
            return 0;
        }
        let cfg = &data.cfg;
        let burn = cfg.blast_burn_m_per_cbrt_mj * energy_mj.cbrt();
        let mut n = 0;
        for i in 0..self.cars.len() {
            let d = self.cars[i].pos.distance(at);
            if d < burn && !self.cars[i].state.wreck() {
                self.harm(i, HarmCause::Blast, by_player);
                self.ignite(i, cfg);
                n += 1;
            } else if d < cfg.fear_blast_m {
                self.frighten(i, cfg);
            }
        }
        n
    }

    fn ignite(&mut self, i: usize, cfg: &TrafficConfig) {
        let c = &mut self.cars[i];
        c.state = CarState::Burning { left_s: cfg.burn_s };
        c.health = 0.0;
        c.wreck_s = 0.0;
        c.speed_m_s = 0.0;
        if let Some(b) = &mut c.body {
            b.vel *= 0.5;
        }
    }

    fn frighten(&mut self, i: usize, cfg: &TrafficConfig) {
        let id = self.cars[i].id;
        let c = &mut self.cars[i];
        if c.state != CarState::Driving {
            return;
        }
        if c.fear_s <= 0.0 {
            // each driver's nerve from the car and the moment, so a replay frightens the same
            let mut r = SimRng::new(self.seed ^ label_hash("fear")).fork(&format!("{id}/{}", (self.time_s * 10.0) as u32));
            c.fear_s = r.range_f32(cfg.fear_stop_s[0], cfg.fear_stop_s[1]);
        } else {
            c.fear_s = c.fear_s.max(cfg.fear_stop_s[0]);
        }
    }

    // ----- the pilot and a car ---------------------------------------------------------------

    /// A pilot wants this car: it brakes to a stop where it is.
    pub fn hail(&mut self, car: u32) {
        if let Some(c) = self.get_mut(car)
            && c.state == CarState::Driving
        {
            c.hailed = true;
        }
    }

    /// The pilot takes this (stopped) car: it leaves its lane as the player's free body, and a
    /// traffic car's driver gets out and runs to `refuge(where they stand)`, the nearest door. False
    /// when the car cannot be taken (a wreck, already driven, or still moving).
    pub fn take(&mut self, data: &TrafficData, car: u32, ground: &impl Ground, refuge: &dyn Fn(Vec2) -> Vec2) -> bool {
        let Some(c) = self.get_mut(car) else { return false };
        if c.state.wreck() || c.state == CarState::Driven || c.vel().length() > 0.5 {
            return false;
        }
        let spec = &data.kinds[c.kind as usize];
        let had_driver = c.state == CarState::Driving;
        let body = c.body.take().unwrap_or_else(|| {
            let mut b = CarBody::parked(spec, ground, Vec2::new(c.pos.x, c.pos.z), c.heading_rad);
            b.health = c.health;
            b
        });
        c.body = Some(body);
        c.state = CarState::Driven;
        c.hailed = false;
        c.fear_s = 0.0;
        c.speed_m_s = 0.0;
        if had_driver {
            // out of the far door: the side away from the player's door
            let door = CarSpec::to_world(c.pos, c.heading_rad, spec.door_driver_m);
            let away = CarSpec::to_world(c.pos, c.heading_rad, [-spec.door_driver_m[0], 0.0, spec.door_driver_m[2]]);
            let at = Vec2::new(away.x, away.z);
            let _ = door;
            let id = c.id;
            self.runners.push(Runner {
                car: id,
                at,
                to: refuge(at),
                age_s: 0.0,
            });
        }
        true
    }

    /// The player gets out: the car stays where it is, loose, with nobody in it.
    pub fn leave(&mut self, car: u32) {
        if let Some(c) = self.get_mut(car)
            && c.state == CarState::Driven
        {
            c.state = CarState::Loose;
        }
    }

    // ----- the step --------------------------------------------------------------------------

    /// Advances one fixed step of `dt_s` seconds: free bodies (the player's car, loose cars),
    /// following and junctions on the lanes, fear and shoving by mechs, fires burning down, runners,
    /// and wrecks and far cars dropped (from the eye at `eye`).
    pub fn step<G: Ground>(&mut self, view: &TrafficView<G>, eye: Vec3, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        self.time_s += dt_s;
        let cfg = &view.data.cfg;
        self.react_to_mechs(view);
        self.step_free(view, dt_s);
        self.step_lanes(view, dt_s);
        // fires burn down; wrecks age
        for c in &mut self.cars {
            if let CarState::Burning { left_s } = &mut c.state {
                *left_s -= dt_s;
                if *left_s <= 0.0 {
                    c.state = CarState::Burnt;
                }
            }
            if c.state.wreck() {
                c.wreck_s += dt_s;
            }
            c.fear_s = (c.fear_s - dt_s).max(0.0);
        }
        // runners make for their doors
        for r in &mut self.runners {
            r.age_s += dt_s;
            let d = r.to - r.at;
            let step = cfg.runner_speed_m_s * dt_s;
            r.at = if d.length() <= step { r.to } else { r.at + d.normalize() * step };
        }
        self.runners.retain(|r| r.age_s < cfg.runner_life_s && r.at.distance(r.to) > 0.5);
        // far cars and old wrecks go; the player's car never does
        let eye2 = Vec2::new(eye.x, eye.z);
        self.cars.retain(|c| {
            c.state == CarState::Driven
                || (Vec2::new(c.pos.x, c.pos.z).distance(eye2) <= cfg.despawn_beyond_m
                    && !(c.state.wreck() && c.wreck_s > cfg.wreck_life_s))
        });
    }

    fn react_to_mechs<G: Ground>(&mut self, view: &TrafficView<G>) {
        let cfg = &view.data.cfg;
        for m in view.mechs {
            let moving = m.vel.length() > cfg.mech_moving_m_s;
            if !moving {
                continue;
            }
            for i in 0..self.cars.len() {
                let p = Vec2::new(self.cars[i].pos.x, self.cars[i].pos.z);
                let d = p.distance(m.at);
                if d < cfg.fear_mech_m {
                    self.frighten(i, cfg);
                }
                let spec = &view.data.kinds[self.cars[i].kind as usize];
                let reach = m.radius_m + spec.size_m[0] * 0.5;
                let c = &self.cars[i];
                if d < reach && !c.state.wreck() && c.state != CarState::Driven {
                    // shoved aside, out of the mech's way, and hurt
                    let away = (p - m.at).normalize_or(right_of(m.vel.normalize_or(Vec2::X)));
                    self.harm(i, HarmCause::Shoved, m.piloted);
                    let c = &mut self.cars[i];
                    let mut body = c
                        .body
                        .take()
                        .unwrap_or_else(|| CarBody::parked(spec, view.ground, p, c.heading_rad));
                    body.vel = away * cfg.push_speed_m_s + m.vel;
                    body.health = (c.health - cfg.push_damage).max(0.0);
                    c.health = body.health;
                    c.body = Some(body);
                    c.speed_m_s = 0.0;
                    if c.health <= 0.0 {
                        c.state = CarState::Burning { left_s: cfg.burn_s };
                    } else {
                        c.state = CarState::Loose;
                    }
                }
            }
        }
    }

    /// The player's car and every loose car or loose wreck: a body on the ground.
    fn step_free<G: Ground>(&mut self, view: &TrafficView<G>, dt_s: f32) {
        let cfg = &view.data.cfg;
        let ids: Vec<usize> = (0..self.cars.len()).filter(|&i| self.cars[i].body.is_some()).collect();
        for i in ids {
            let me = &self.cars[i];
            let spec = &view.data.kinds[me.kind as usize];
            let p = Vec2::new(me.pos.x, me.pos.z);
            // the other cars close by are in the way, and so is everything the view says is
            let mut obstacles: Vec<Obstacle> = view.obstacles.iter().copied().filter(|o| o.at.distance(p) < 20.0).collect();
            for (j, o) in self.cars.iter().enumerate() {
                if j == i {
                    continue;
                }
                let q = Vec2::new(o.pos.x, o.pos.z);
                if q.distance(p) > 15.0 {
                    continue;
                }
                let os = &view.data.kinds[o.kind as usize];
                let r = os.size_m[0] * 0.5;
                let half = (os.size_m[2] * 0.5 - r).max(0.0);
                let f = forward2(o.heading_rad);
                for at in [q + f * half, q - f * half] {
                    obstacles.push(Obstacle {
                        id: o.id,
                        at,
                        radius_m: r,
                        vel: o.vel(),
                        mass_kg: os.mass_kg,
                    });
                }
            }
            let driven = me.state == CarState::Driven;
            let input = if driven && !me.state.wreck() {
                view.driver.unwrap_or(PARKED)
            } else {
                PARKED
            };
            let Some(body) = self.cars[i].body.as_mut() else { continue };
            let step = body.step(spec, input, view.ground, &obstacles, dt_s);
            let (pos, heading, health) = (body.pos, body.heading_rad, body.health);
            {
                let c = &mut self.cars[i];
                c.pos = pos;
                c.heading_rad = heading;
                c.health = health;
                c.speed_m_s = c.vel().length();
            }
            if health <= 0.0 && !self.cars[i].state.wreck() {
                self.harm(i, HarmCause::Shoved, false);
                self.ignite(i, cfg);
            }
            // the cars it pushed: a car shoved hard leaves its lane
            let mut seen = Vec::new();
            for (id, dv) in step.pushed {
                if seen.contains(&id) {
                    continue;
                }
                seen.push(id);
                let Ok(j) = self.cars.binary_search_by_key(&id, |c| c.id) else {
                    continue;
                };
                if dv.length() < 0.5 {
                    continue;
                }
                if driven && self.cars[j].occupied() {
                    self.harm(j, HarmCause::Shoved, true);
                }
                let o = &mut self.cars[j];
                let os = &view.data.kinds[o.kind as usize];
                let v = o.vel();
                let mut b = o
                    .body
                    .take()
                    .unwrap_or_else(|| CarBody::parked(os, view.ground, Vec2::new(o.pos.x, o.pos.z), o.heading_rad));
                b.vel = v + dv;
                o.body = Some(b);
                if !o.state.wreck() {
                    o.state = CarState::Loose;
                }
            }
        }
    }

    /// Following and junctions for every car on its lane. Accelerations are worked out from where
    /// every car was at the start of the step, then every car moves, so the order cars are visited in
    /// never changes the outcome.
    fn step_lanes<G: Ground>(&mut self, view: &TrafficView<G>, dt_s: f32) {
        let cfg = &view.data.cfg;
        let g = view.graph;
        // each lane's cars, nearest its start first
        let mut on: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, c) in self.cars.iter().enumerate() {
            if c.on_lane() {
                on.entry(c.lane).or_default().push(i);
            }
        }
        for list in on.values_mut() {
            list.sort_by(|&a, &b| {
                self.cars[a]
                    .s_m
                    .total_cmp(&self.cars[b].s_m)
                    .then(self.cars[a].id.cmp(&self.cars[b].id))
            });
        }
        self.grant_turns(g, &on, view.data);

        // free cars and mechs' legs are things in the road too
        let mut things: Vec<(Vec2, f32)> = view.obstacles.iter().map(|o| (o.at, o.radius_m)).collect();
        for c in self.cars.iter().filter(|c| c.body.is_some()) {
            things.push((Vec2::new(c.pos.x, c.pos.z), view.data.kinds[c.kind as usize].size_m[2] * 0.5));
        }
        for m in view.mechs {
            things.push((m.at, m.radius_m));
        }

        let mut accel = vec![0.0f32; self.cars.len()];
        for (i, c) in self.cars.iter().enumerate() {
            if !c.on_lane() {
                continue;
            }
            if c.state != CarState::Driving {
                accel[i] = -cfg.max_brake_m_s2;
                continue;
            }
            if c.fear_s > 0.0 || c.hailed {
                accel[i] = -cfg.fear_brake_m_s2;
                continue;
            }
            let lane = &g.lanes[c.lane as usize];
            let len = length_of(view.data, c);
            let (gap, lead_v) = self.leader(i, g, &on, view.data, &things);
            // stop at the line until the junction is ours
            let to_end = lane.length_m() - c.s_m;
            let line = match c.next {
                Some(n) if g.lanes[n as usize].kind == LaneKind::Connector && !c.granted => {
                    Some((to_end - len * 0.5 - cfg.stop_short_m).max(0.0))
                }
                _ => None,
            };
            let (gap, lead_v) = match line {
                Some(l) if l < gap => (l, 0.0),
                _ => (gap, lead_v),
            };
            let v0 = self.limit(c, lane, g, cfg);
            accel[i] = idm(c.speed_m_s, v0, gap, lead_v, cfg);
        }
        // move
        for (i, a) in accel.iter().enumerate() {
            if !self.cars[i].on_lane() {
                continue;
            }
            let c = &mut self.cars[i];
            c.speed_m_s = (c.speed_m_s + a * dt_s).max(0.0);
            c.s_m += c.speed_m_s * dt_s;
            // never into a junction without its turn: a car that could not stop in time stops at the line
            let end = g.lanes[c.lane as usize].length_m();
            let waits = c.next.is_some_and(|n| g.lanes[n as usize].kind == LaneKind::Connector) && !c.granted;
            if waits && c.s_m >= end {
                c.s_m = end - 0.01;
                c.speed_m_s = 0.0;
            }
            let mut guard = 0;
            while c.s_m > g.lanes[c.lane as usize].length_m() && guard < 4 {
                guard += 1;
                let Some(n) = c.next else {
                    c.s_m = g.lanes[c.lane as usize].length_m();
                    c.speed_m_s = 0.0;
                    break;
                };
                c.s_m -= g.lanes[c.lane as usize].length_m();
                c.lane = n;
                c.granted = false;
                c.arrived_s = None;
                let lane = &g.lanes[n as usize];
                c.next = if lane.kind == LaneKind::Connector {
                    lane.out.first().copied()
                } else {
                    c.turns += 1;
                    choose_turn(self.seed, c.id, c.turns, lane)
                };
            }
        }
        // place every lane car, on its lane's road surface
        if self.heights.len() != g.lanes.len() {
            self.heights = vec![None; g.lanes.len()];
        }
        for c in &mut self.cars {
            if c.body.is_some() {
                continue;
            }
            let lane = &g.lanes[c.lane as usize];
            let (p, d) = lane.line.at(c.s_m);
            let side = if c.state == CarState::Parked {
                lane.parking_m.unwrap_or(0.0)
            } else {
                0.0
            };
            let p = p + right_of(d) * side;
            let hs = self.heights[c.lane as usize]
                .get_or_insert_with(|| lane.line.points.iter().map(|q| view.ground.height_at(q.x, q.y)).collect());
            let (i, u) = lane.line.locate(c.s_m);
            let y = hs[i] + (hs[(i + 1).min(hs.len() - 1)] - hs[i]) * u;
            c.pos = Vec3::new(p.x, y, p.y);
            c.heading_rad = (-d.x).atan2(-d.y);
        }
    }

    /// The gap to whatever is ahead of car `i` within `lookahead_m`, metres, and its speed.
    fn leader(&self, i: usize, g: &LaneGraph, on: &BTreeMap<u32, Vec<usize>>, data: &TrafficData, things: &[(Vec2, f32)]) -> (f32, f32) {
        let cfg = &data.cfg;
        let c = &self.cars[i];
        let half = length_of(data, c) * 0.5;
        let mut best = (f32::INFINITY, 0.0);
        // along the lanes: this one, then the next two
        let mut lane = c.lane;
        let mut offset = -c.s_m;
        let mut next = c.next;
        for hop in 0..3 {
            if let Some(list) = on.get(&lane) {
                for &j in list {
                    if j == i {
                        continue;
                    }
                    let o = &self.cars[j];
                    let ahead = offset + o.s_m;
                    if ahead <= 0.0 && hop == 0 && (o.s_m < c.s_m || (o.s_m == c.s_m && o.id < c.id)) {
                        continue;
                    }
                    if ahead < -0.01 {
                        continue;
                    }
                    let gap = ahead - half - length_of(data, o) * 0.5;
                    if gap < best.0 {
                        best = (gap, o.speed_m_s);
                    }
                    break;
                }
            }
            if best.0.is_finite() {
                break;
            }
            offset += g.lanes[lane as usize].length_m();
            if offset > cfg.lookahead_m {
                break;
            }
            let Some(n) = next else { break };
            lane = n;
            next = g.lanes[n as usize]
                .out
                .first()
                .copied()
                .filter(|_| g.lanes[n as usize].kind == LaneKind::Connector);
        }
        // anything else in the road ahead: people, loose cars, mechs
        let f = forward2(c.heading_rad);
        let r = right_of(f);
        let p = Vec2::new(c.pos.x, c.pos.z);
        let width = data.kinds[c.kind as usize].size_m[0] * 0.5;
        for &(at, radius) in things {
            let d = at - p;
            let ahead = d.dot(f);
            if ahead <= 0.0 || ahead > cfg.lookahead_m {
                continue;
            }
            if d.dot(r).abs() > width + radius + 0.5 {
                continue;
            }
            let gap = ahead - half - radius;
            if gap < best.0 {
                best = (gap.max(0.0), 0.0);
            }
        }
        best
    }

    /// The speed car `c` aims for on its lane: its own cruise, or the turn speed on a turning connector.
    fn limit(&self, c: &Car, lane: &Lane, _g: &LaneGraph, cfg: &TrafficConfig) -> f32 {
        if lane.kind == LaneKind::Connector && lane.turn_rad.abs() > cfg.turn_from_deg.to_radians() {
            (cfg.turn_speed_km_h / 3.6).min(c.cruise_m_s)
        } else {
            c.cruise_m_s
        }
    }

    /// Right of way by arrival order: at each junction the front car of each lane waiting is taken earliest first
    /// (ties to the lower id); each may go when its connector meets no connector a car is on, none
    /// already given this step, and none an earlier car is still waiting for, and the lane it leads
    /// into has room for it. At a junction with traffic lights (survey W39) a car whose light is red
    /// stops at the line and holds no path; on amber it goes only if it could not stop comfortably short
    /// of the line; on green it takes its turn as above, so a turn across oncoming traffic still gives way.
    ///
    /// The room on a lane past the junction is shared out in arrival order, and a car held at red keeps
    /// its place in that order: the space it needs is kept for it, so the cars of the other phase cannot
    /// take every gap that opens on a slow lane both feed while its light is red, and its turn comes on
    /// its next green. Without that a street car turning onto a crawling arterial stood for minutes.
    fn grant_turns(&mut self, g: &LaneGraph, on: &BTreeMap<u32, Vec<usize>>, data: &TrafficData) {
        let cfg = &data.cfg;
        let now = self.time_s;
        // connectors with a car on them (wrecks too), by junction
        let mut busy: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (&lane, list) in on {
            let l = &g.lanes[lane as usize];
            if l.kind == LaneKind::Connector && !list.is_empty() {
                busy.entry(l.from_node).or_default().push(lane);
            }
        }
        // (arrived, id, car, connector, may go now: false while its light holds it)
        let mut waiting: BTreeMap<u32, Vec<(f32, u32, usize, u32, bool)>> = BTreeMap::new();
        for (i, c) in self.cars.iter_mut().enumerate() {
            if !c.on_lane() {
                continue;
            }
            let Some(n) = c.next else { continue };
            let lane = &g.lanes[c.lane as usize];
            let conn = &g.lanes[n as usize];
            if conn.kind != LaneKind::Connector {
                continue;
            }
            // a car given its turn holds its path until it is through
            if c.granted {
                busy.entry(lane.to_node).or_default().push(n);
                continue;
            }
            if c.state != CarState::Driving || lane.length_m() - c.s_m > cfg.approach_m {
                continue;
            }
            // only the front car of a lane asks: one behind it given a turn would hold a path the car in front
            // needs, and neither could move
            if on.get(&c.lane).and_then(|l| l.last()) != Some(&i) {
                continue;
            }
            let t = *c.arrived_s.get_or_insert(now);
            let go = match lane.phase {
                None => true,
                Some(phase) => {
                    let to_line = lane.length_m() - c.s_m - data.kinds[c.kind as usize].size_m[2] * 0.5 - cfg.stop_short_m;
                    let can_stop = c.speed_m_s * c.speed_m_s / (2.0 * cfg.idm_brake_m_s2) <= to_line.max(0.0);
                    match light(&cfg.lights, lane.to_node, phase, now) {
                        Light::Green => true,
                        Light::Amber => !can_stop,
                        Light::Red => false,
                    }
                }
            };
            waiting.entry(lane.to_node).or_default().push((t, c.id, i, n, go));
        }
        for (node, mut list) in waiting {
            list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let taken: Vec<u32> = busy.get(&node).cloned().unwrap_or_default();
            let mut held: Vec<u32> = Vec::new();
            // the room on each lane past this junction already promised to an earlier car, metres
            let mut promised: BTreeMap<u32, f32> = BTreeMap::new();
            for (_, _, i, conn, go) in list {
                let meets = |set: &[u32]| set.iter().any(|&o| o == conn || g.lanes[conn as usize].conflicts.contains(&o));
                let out = g.lanes[conn as usize].out.first().copied();
                // room on the far side for this car and its gap, after the earlier cars' share
                let need = data.kinds[self.cars[i].kind as usize].size_m[2] + cfg.min_gap_m;
                let room = out.is_none_or(|o| {
                    let free = on.get(&o).and_then(|l| l.first()).map_or(f32::INFINITY, |&j| {
                        let theirs = data.kinds[self.cars[j].kind as usize].size_m[2];
                        self.cars[j].s_m - theirs * 0.5
                    });
                    let p = promised.entry(o).or_insert(0.0);
                    let fits = free - *p > need;
                    *p += need;
                    fits
                });
                if go {
                    if !meets(&taken) && !meets(&held) && room {
                        self.cars[i].granted = true;
                    }
                    held.push(conn);
                }
            }
        }
    }

    // ----- appearing and going ---------------------------------------------------------------

    /// Keeps the lanes within `spawn_within_m` of `eye` filled to their density (survey J6): driving
    /// cars on every road lane, parked cars at the kerbs. A car appears only where `in_view` says the
    /// eye cannot see it, or beyond `hidden_beyond_m` where `covered` says something stands between.
    /// At most `budget` cars appear. Returns how many did.
    #[allow(clippy::too_many_arguments)]
    pub fn fill(
        &mut self,
        g: &LaneGraph,
        data: &TrafficData,
        ground: &impl Ground,
        eye: Vec3,
        in_view: &dyn Fn(Vec3) -> bool,
        covered: &dyn Fn(Vec3) -> bool,
        clear_of: &[(Vec2, f32)],
        budget: u32,
    ) -> u32 {
        let cfg = &data.cfg;
        let eye2 = Vec2::new(eye.x, eye.z);
        let range = cfg.spawn_within_m;
        if self.bounds.len() != g.lanes.len() {
            self.bounds = g
                .lanes
                .iter()
                .map(|l| {
                    let lo = l.line.points.iter().copied().fold(Vec2::splat(f32::INFINITY), Vec2::min);
                    let hi = l.line.points.iter().copied().fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
                    (lo, hi)
                })
                .collect();
        }
        if self.near_from.is_none_or(|f| f.distance(eye2) > 100.0) {
            // the lanes in reach, and each one's share of the density for its length in reach (worked out
            // again each time the eye has moved 100 m)
            self.near = g
                .road_lanes()
                .filter(|l| {
                    let (lo, hi) = self.bounds[l.id as usize];
                    let d = (lo - eye2).max(eye2 - hi).max(Vec2::ZERO).length();
                    d < range
                })
                .filter_map(|lane| {
                    let li = lane.id;
                    let (in_range, from, to) = span_within(&lane.line.points, eye2, range);
                    if in_range <= 0.0 {
                        return None;
                    }
                    let spacing = if lane.kind.in_city() {
                        cfg.city_spacing_m
                    } else {
                        cfg.highway_spacing_m
                    };
                    // a fixed share per lane, so the expected count is exact and does not flicker
                    let frac = |salt: &str| (label_hash(&format!("{salt}{li}")) >> 40) as f32 / (1u64 << 24) as f32;
                    let want = (in_range / spacing + frac("drive")).floor() as u32;
                    let want_parked = match lane.parking_m {
                        Some(_) => (in_range / cfg.parked_spacing_m + frac("park")).floor() as u32,
                        None => 0,
                    };
                    Some((li, want, want_parked, from, to))
                })
                .collect();
            self.near_from = Some(eye2);
        }
        // each lane's cars: (along, length, speed, parked)
        let mut on: BTreeMap<u32, Vec<(f32, f32, f32, bool)>> = BTreeMap::new();
        let mut count: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        for c in &self.cars {
            if c.body.is_some() {
                continue;
            }
            let parked = c.state == CarState::Parked;
            on.entry(c.lane).or_default().push((c.s_m, length_of(data, c), c.speed_m_s, parked));
            if Vec2::new(c.pos.x, c.pos.z).distance(eye2) > range {
                continue;
            }
            let e = count.entry(c.lane).or_default();
            if parked {
                e.1 += 1;
            } else {
                e.0 += 1;
            }
        }
        let mut made = 0;
        let near = std::mem::take(&mut self.near);
        for &(li, want, want_parked, from, to) in &near {
            if made >= budget {
                break;
            }
            let lane = &g.lanes[li as usize];
            let city = lane.kind.in_city();
            let (have, have_parked) = count.get(&li).copied().unwrap_or_default();
            for parked in [false, true] {
                let (want, mut have) = if parked { (want_parked, have_parked) } else { (want, have) };
                let mut tries = want.saturating_sub(have) * 3;
                while have < want && made < budget && tries > 0 {
                    tries -= 1;
                    // on the stretch in reach (a highway runs on far past the ring, and a place drawn along all
                    // of it would mostly miss: the lane filled to half its density inside 1 km), and clear of
                    // the lane's first 30 m, where a car leaving the junction behind would meet it
                    let lo = from.max(if parked { 0.0 } else { 30.0f32.min(lane.length_m() * 0.5) });
                    if lo >= to {
                        break;
                    }
                    let s = self.rng.range_f32(lo, to);
                    let (p, d) = lane.line.at(s);
                    let p = p + right_of(d) * if parked { lane.parking_m.unwrap_or(0.0) } else { 0.0 };
                    if p.distance(eye2) > range {
                        continue;
                    }
                    let kind = self.pick_kind(data, city);
                    let spec = &data.kinds[kind];
                    let paint = (self.rng.next_u64() % spec.paints.len() as u64) as u8;
                    let band = if city { cfg.city_speed_km_h } else { cfg.highway_speed_km_h };
                    let cruise = self.rng.range_f32(band[0], band[1]) / 3.6;
                    // a speed it can stop from before the lane's end, and room for it and the car behind
                    let len = spec.size_m[2];
                    let to_line = (lane.length_m() - s - len * 0.5 - cfg.stop_short_m).max(0.0);
                    let speed = if parked {
                        0.0
                    } else {
                        (cruise * 0.7).min((2.0 * cfg.idm_brake_m_s2 * to_line).sqrt())
                    };
                    let clear = on
                        .get(&li)
                        .into_iter()
                        .flatten()
                        .filter(|o| o.3 == parked)
                        .all(|&(cs, clen, v, _)| {
                            let both = (len + clen) * 0.5 + cfg.min_gap_m;
                            if cs <= s {
                                s - cs >= both + v * cfg.gap_s + v * v / (2.0 * cfg.idm_brake_m_s2)
                            } else {
                                cs - s >= both + speed * cfg.gap_s
                            }
                        });
                    // never under a mech's body (a car put there was crushed or shoved by a mech standing still,
                    // and charged to the player: the start's mech stands in the street)
                    let under = clear_of.iter().any(|&(at, r)| p.distance(at) < r + len * 0.5 + cfg.min_gap_m);
                    if !clear || under {
                        continue;
                    }
                    let at = Vec3::new(p.x, ground.height_at(p.x, p.y), p.y);
                    let hidden = !in_view(at) || (at.distance(eye) > cfg.hidden_beyond_m && covered(at));
                    if !hidden {
                        continue;
                    }
                    on.entry(li).or_default().push((s, len, speed, parked));
                    let id = self.next_id;
                    self.next_id += 1;
                    self.cars.push(Car {
                        id,
                        kind: kind as u16,
                        paint,
                        state: if parked { CarState::Parked } else { CarState::Driving },
                        lane: li,
                        s_m: s,
                        next: if parked { None } else { choose_turn(self.seed, id, 0, lane) },
                        speed_m_s: speed,
                        cruise_m_s: cruise,
                        fear_s: 0.0,
                        hailed: false,
                        arrived_s: None,
                        granted: false,
                        turns: 0,
                        pos: at,
                        heading_rad: (-d.x).atan2(-d.y),
                        body: None,
                        health: 1.0,
                        wreck_s: 0.0,
                    });
                    made += 1;
                    have += 1;
                }
            }
        }
        self.near = near;
        made
    }

    fn pick_kind(&mut self, data: &TrafficData, city: bool) -> usize {
        let w = |k: &CarSpec| if city { k.city_weight } else { k.highway_weight };
        let total: f32 = data.kinds.iter().map(w).sum();
        let mut x = self.rng.next_f32() * total;
        for (i, k) in data.kinds.iter().enumerate() {
            x -= w(k);
            if x < 0.0 && w(k) > 0.0 {
                return i;
            }
        }
        data.kinds.iter().rposition(|k| w(k) > 0.0).unwrap_or(0)
    }

    /// Every car's hit shape for the weapons (one capsule along its body), with its id, for the cars
    /// that are not yet wrecks.
    pub fn capsules(&self, data: &TrafficData) -> Vec<(u32, Vec3, Vec3, f32)> {
        self.cars
            .iter()
            .filter(|c| !c.state.wreck())
            .map(|c| {
                let s = &data.kinds[c.kind as usize];
                let r = s.size_m[0].max(s.size_m[1]) * 0.5;
                let f = forward2(c.heading_rad);
                let half = (s.size_m[2] * 0.5 - r).max(0.0);
                let mid = c.pos + Vec3::Y * s.size_m[1] * 0.5;
                let off = Vec3::new(f.x, 0.0, f.y) * half;
                (c.id, mid - off, mid + off, r)
            })
            .collect()
    }
}

/// The turn a car takes at the end of `lane`: one of its connectors, from a stream forked from the
/// car's id, the junction and how many junctions it has crossed, so a replay turns the same.
fn choose_turn(seed: u64, car: u32, turns: u32, lane: &Lane) -> Option<u32> {
    if lane.out.is_empty() {
        return None;
    }
    let mut r = SimRng::new(seed ^ label_hash("turns")).fork(&format!("{car}/{}/{turns}", lane.to_node));
    let k = (r.next_u64() % lane.out.len() as u64) as usize;
    Some(lane.out[k])
}

/// The intelligent driver model: acceleration toward `v0` with a gap `gap` (metres) to a leader at
/// `lead_v` (m/s), keeping `gap_s` seconds and `min_gap_m` metres.
fn idm(v: f32, v0: f32, gap: f32, lead_v: f32, cfg: &TrafficConfig) -> f32 {
    let a = cfg.idm_accel_m_s2;
    let free = 1.0 - (v / v0.max(0.1)).powi(4);
    let a_out = if gap.is_finite() {
        if gap <= 0.0 {
            -cfg.max_brake_m_s2
        } else {
            let want = cfg.min_gap_m + (v * cfg.gap_s + v * (v - lead_v) / (2.0 * (a * cfg.idm_brake_m_s2).sqrt())).max(0.0);
            a * (free - (want / gap).powi(2))
        }
    } else {
        a * free
    };
    a_out.clamp(-cfg.max_brake_m_s2, a)
}

/// The polyline within `range` of `p` (by segment, both ends inside): its length, and the first and last
/// distance along the line that it covers, metres. (0, 0, 0) when none of it is.
fn span_within(points: &[Vec2], p: Vec2, range: f32) -> (f32, f32, f32) {
    let (mut len, mut from, mut to, mut s) = (0.0, f32::INFINITY, 0.0f32, 0.0);
    for w in points.windows(2) {
        let seg = w[0].distance(w[1]);
        if w[0].distance(p) <= range && w[1].distance(p) <= range {
            len += seg;
            from = from.min(s);
            to = to.max(s + seg);
        }
        s += seg;
    }
    if len > 0.0 { (len, from, to) } else { (0.0, 0.0, 0.0) }
}

/// True when the sole's circle of radius `r` at `sole` overlaps the car's footprint.
fn under_foot(c: &Car, spec: &CarSpec, sole: Vec2, r: f32) -> bool {
    let f = forward2(c.heading_rad);
    let side = right_of(f);
    let d = sole - Vec2::new(c.pos.x, c.pos.z);
    let local = Vec2::new(d.dot(side), d.dot(f));
    let half = Vec2::new(spec.size_m[0] * 0.5, spec.size_m[2] * 0.5);
    let nearest = local.clamp(-half, half);
    nearest.distance(local) <= r
}

#[cfg(test)]
mod tests {
    #[test]
    fn car_detail_ranges_must_run_from_the_cab_out_to_the_wheels() {
        assert!(CarDetail::default().validate().is_ok());
        let backwards = CarDetail {
            trim_m: 500.0,
            ..CarDetail::default()
        };
        assert!(backwards.validate().is_err());
        let no_fade_end = CarDetail {
            fade_share: 1.0,
            ..CarDetail::default()
        };
        assert!(no_fade_end.validate().is_err());
    }

    use super::*;
    use crate::rng::SimRng;
    use crate::world::layout::{City, Highway, lay_out_city};
    use crate::world::{FlatGround, WorldConfig};

    const DT: f32 = 1.0 / 30.0;

    fn data() -> TrafficData {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        TrafficData::load(&root).expect("shipped traffic data loads")
    }

    fn town() -> (Vec<City>, Vec<Highway>, LaneGraph) {
        let cfg = WorldConfig::default();
        let mut rng = SimRng::new(7);
        let a = lay_out_city(&mut rng, 0, Vec2::new(0.0, 0.0), 400.0, 0.0, &cfg);
        let b = lay_out_city(&mut rng, 1, Vec2::new(4000.0, 0.0), 250.0, 0.0, &cfg);
        let h = Highway::straight(
            [0, 1],
            Vec2::new(400.0, 0.0),
            Vec2::new(3750.0, 0.0),
            [0.0, 0.0],
            cfg.highway_width_m,
            50.0,
            cfg.world_size_m,
        );
        let cities = vec![a, b];
        let hw = vec![h];
        let g = LaneGraph::build(&cities, &hw, &data().cfg.lanes, cfg.arterial_width_m);
        (cities, hw, g)
    }

    fn never_seen(_: Vec3) -> bool {
        false
    }

    /// The cars are raised by `car_height_scale` on load (owner, 2026-10-26: "cars are a little too short I think
    /// lol"): every height, the wheels, the ride and the driver's eye, never the length or the width.
    #[test]
    fn every_car_is_loaded_raised_by_the_height_scale_and_keeps_its_footprint() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let d = data();
        let k = d.cfg.car_height_scale;
        assert!((1.15..=1.2).contains(&k), "the cars are raised 15 to 20 %, not {k}");
        for (name, spec) in d.cfg.kinds.iter().zip(&d.kinds) {
            let raw: CarSpec = crate::data::load_ron(&root.join(format!("cars/{name}.ron"))).unwrap();
            assert_eq!(spec.size_m[0], raw.size_m[0], "{name} keeps its width");
            assert_eq!(spec.size_m[2], raw.size_m[2], "{name} keeps its length");
            assert!((spec.size_m[1] - raw.size_m[1] * k).abs() < 1e-5, "{name}'s roof");
            assert!((spec.wheel_radius_m - raw.wheel_radius_m * k).abs() < 1e-5, "{name}'s wheels");
            assert!((spec.ride_height_m - raw.ride_height_m * k).abs() < 1e-5, "{name}'s ride");
            assert!(
                (spec.eye_driver_m[1] - raw.eye_driver_m[1] * k).abs() < 1e-5,
                "{name}'s driver's eye"
            );
            assert_eq!(spec.door_driver_m, raw.door_driver_m, "{name}'s door stays on the ground");
        }
    }

    fn view<'a>(g: &'a LaneGraph, d: &'a TrafficData, ground: &'a FlatGround) -> TrafficView<'a, FlatGround> {
        TrafficView {
            graph: g,
            data: d,
            ground,
            mechs: &[],
            obstacles: &[],
            driver: None,
        }
    }

    fn filled(seed: u64) -> (Traffic, LaneGraph, TrafficData) {
        let (_, _, g) = town();
        let d = data();
        let mut t = Traffic::new(seed);
        t.fill(&g, &d, &FlatGround::default(), Vec3::ZERO, &never_seen, &never_seen, &[], u32::MAX);
        (t, g, d)
    }

    /// No car is put under a mech's body: one standing in the street is a wall the lanes fill round, not a crusher of
    /// cars that were never driven into it (they were crushed and charged to the player at the start).
    #[test]
    fn no_car_is_filled_in_under_a_mech_standing_in_the_road() {
        let (_, _, g) = town();
        let d = data();
        // a mech over the busiest point: the middle of the first few city lanes
        let at = g
            .lanes
            .iter()
            .find(|l| l.kind.in_city())
            .map(|l| l.line.at(l.length_m() * 0.5).0)
            .unwrap();
        let r = 7.5;
        let mut t = Traffic::new(5);
        t.fill(
            &g,
            &d,
            &FlatGround::default(),
            Vec3::ZERO,
            &never_seen,
            &never_seen,
            &[(at, r)],
            u32::MAX,
        );
        assert!(!t.cars.is_empty());
        for c in &t.cars {
            let len = d.kinds[c.kind as usize].size_m[2];
            assert!(
                Vec2::new(c.pos.x, c.pos.z).distance(at) >= r + len * 0.5,
                "car {} under the mech",
                c.id
            );
        }
    }

    #[test]
    fn the_shipped_traffic_data_is_the_seven_1980s_kinds() {
        let d = data();
        assert_eq!(d.cfg.kinds, ["sedan", "wagon", "pickup", "van", "taxi", "bus", "semi"]);
        assert_eq!(d.cfg.city_spacing_m, 60.0, "survey J6");
        assert_eq!(d.cfg.highway_spacing_m, 300.0, "survey J6");
        // survey J6 said 1.5 km; the owner cut it (2026-10-26: "car should be 1000m"), with 200 m of margin
        assert_eq!(d.cfg.spawn_within_m, 1000.0, "owner, 2026-10-26: car should be 1000m");
        assert_eq!(
            d.cfg.despawn_beyond_m, 1200.0,
            "200 m past the ring, so a car on the edge does not come and go"
        );
    }

    #[test]
    fn the_lanes_fill_to_one_car_per_60_m_in_town_and_300_m_on_the_highway() {
        let (t, g, d) = filled(1);
        let mut city_len = 0.0;
        let mut hw_len = 0.0;
        for l in g.road_lanes() {
            let len = span_within(&l.line.points, Vec2::ZERO, d.cfg.spawn_within_m).0;
            if l.kind.in_city() {
                city_len += len;
            } else {
                hw_len += len;
            }
        }
        let driving = |city: bool| {
            t.cars
                .iter()
                .filter(|c| c.state == CarState::Driving && g.lanes[c.lane as usize].kind.in_city() == city)
                .count() as f32
        };
        let (city_n, hw_n) = (driving(true), driving(false));
        let city_expect = city_len / 60.0;
        let hw_expect = hw_len / 300.0;
        assert!(
            (city_n - city_expect).abs() / city_expect < 0.15,
            "{city_n} city cars for {city_expect:.0} expected"
        );
        assert!(
            (hw_n - hw_expect).abs() <= hw_expect * 0.3 + 1.0,
            "{hw_n} highway cars for {hw_expect:.1} expected"
        );
        assert!(t.cars.iter().any(|c| c.state == CarState::Parked), "parked cars at the kerbs");
        for c in &t.cars {
            assert!(Vec2::new(c.pos.x, c.pos.z).length() <= d.cfg.spawn_within_m + 1.0);
        }
    }

    #[test]
    fn cars_never_appear_in_view_within_400_m_or_in_plain_sight_beyond() {
        let (_, _, g) = town();
        let d = data();
        let mut t = Traffic::new(3);
        let all = |_: Vec3| true;
        t.fill(&g, &d, &FlatGround::default(), Vec3::ZERO, &all, &|_: Vec3| false, &[], u32::MAX);
        assert!(t.cars.is_empty(), "everything is in plain sight");
        t.fill(&g, &d, &FlatGround::default(), Vec3::ZERO, &all, &all, &[], u32::MAX);
        assert!(!t.cars.is_empty());
        for c in &t.cars {
            assert!(c.pos.length() > d.cfg.hidden_beyond_m, "a car popped in at {}", c.pos);
        }
    }

    #[test]
    fn flying_3_km_away_leaves_no_car_behind() {
        let (mut t, g, d) = filled(2);
        assert!(!t.cars.is_empty());
        let ground = FlatGround::default();
        t.step(&view(&g, &d, &ground), Vec3::new(0.0, 0.0, 3000.0), DT);
        assert!(t.cars.is_empty(), "{} cars remain", t.cars.len());
    }

    #[test]
    fn city_and_highway_cars_cruise_at_their_survey_speeds() {
        let (t, g, _) = filled(4);
        for c in &t.cars {
            let kmh = c.cruise_m_s * 3.6;
            if g.lanes[c.lane as usize].kind.in_city() {
                assert!((40.0..=60.0).contains(&kmh), "{kmh}");
            } else {
                assert!((90.0..=110.0).contains(&kmh), "{kmh}");
            }
        }
    }

    /// Runs `s` seconds, checking every step that no two cars on a lane overlap.
    fn run(t: &mut Traffic, g: &LaneGraph, d: &TrafficData, s: f32) {
        let ground = FlatGround::default();
        for k in 0..(s / DT) as usize {
            t.step(&view(g, d, &ground), Vec3::ZERO, DT);
            let mut by_lane: BTreeMap<u32, Vec<&Car>> = BTreeMap::new();
            for c in t.cars.iter().filter(|c| c.on_lane()) {
                by_lane.entry(c.lane).or_default().push(c);
            }
            for list in by_lane.values_mut() {
                list.sort_by(|a, b| a.s_m.total_cmp(&b.s_m));
                for w in list.windows(2) {
                    let gap = w[1].s_m - w[0].s_m - (length_of(d, w[0]) + length_of(d, w[1])) * 0.5;
                    assert!(
                        gap > -0.5,
                        "cars {} and {} overlap on lane {} ({:?}): {gap}; s {} {} v {} {} kinds {} {} step {k}",
                        w[0].id,
                        w[1].id,
                        w[0].lane,
                        g.lanes[w[0].lane as usize].kind,
                        w[0].s_m,
                        w[1].s_m,
                        w[0].speed_m_s,
                        w[1].speed_m_s,
                        w[0].kind,
                        w[1].kind
                    );
                }
            }
        }
    }

    #[test]
    fn cars_drive_for_two_minutes_without_running_into_each_other() {
        let (mut t, g, d) = filled(5);
        let start: Vec<Vec3> = t.cars.iter().map(|c| c.pos).collect();
        run(&mut t, &g, &d, 120.0);
        let moved = t.cars.iter().zip(&start).filter(|(c, s)| c.pos.distance(**s) > 50.0).count();
        assert!(moved > t.cars.len() / 2, "traffic flows: {moved} of {} moved", t.cars.len());
        assert!(t.cars.iter().any(|c| c.turns > 1), "cars cross junctions");
    }

    #[test]
    fn two_cars_on_crossing_paths_are_never_in_a_junction_at_once() {
        let (mut t, g, d) = filled(6);
        let ground = FlatGround::default();
        for _ in 0..(90.0 / DT) as usize {
            t.step(&view(&g, &d, &ground), Vec3::ZERO, DT);
            let on_conn: Vec<&Car> = t
                .cars
                .iter()
                .filter(|c| c.on_lane() && g.lanes[c.lane as usize].kind == LaneKind::Connector)
                .collect();
            for a in &on_conn {
                for b in &on_conn {
                    if a.id < b.id && a.lane != b.lane {
                        assert!(
                            !g.lanes[a.lane as usize].conflicts.contains(&b.lane),
                            "cars {} and {} cross at junction {}",
                            a.id,
                            b.id,
                            g.lanes[a.lane as usize].from_node
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_queue_at_a_junction_stops_with_its_following_gap() {
        // a wreck on the connector: everyone behind it queues
        let (mut t, g, d) = filled(8);
        run(&mut t, &g, &d, 20.0);
        // the car in a junction with the most cars behind it on the lane that feeds it
        let feed_of = |lane: u32| g.lanes.iter().find(|l| l.out.contains(&lane)).map(|l| l.id).unwrap();
        let behind = |lane: u32| t.cars.iter().filter(|o| o.on_lane() && o.lane == feed_of(lane)).count();
        let Some(i) = (0..t.cars.len())
            .filter(|&i| t.cars[i].state == CarState::Driving && g.lanes[t.cars[i].lane as usize].kind == LaneKind::Connector)
            .max_by_key(|&i| (behind(t.cars[i].lane), std::cmp::Reverse(t.cars[i].id)))
        else {
            panic!("no car in a junction");
        };
        let blocked = t.cars[i].lane;
        let feed = feed_of(blocked);
        t.cars[i].state = CarState::Burnt;
        run(&mut t, &g, &d, 60.0);
        let mut queue: Vec<&Car> = t.cars.iter().filter(|c| c.lane == feed && c.on_lane()).collect();
        queue.sort_by(|a, b| b.s_m.total_cmp(&a.s_m));
        assert!(!queue.is_empty());
        for c in &queue {
            assert!(c.speed_m_s < 0.1, "car {} has stopped", c.id);
        }
        for w in queue.windows(2) {
            let gap = w[0].s_m - w[1].s_m - (length_of(&d, w[0]) + length_of(&d, w[1])) * 0.5;
            assert!(gap >= d.cfg.min_gap_m * 0.9, "the queue keeps its standstill gap: {gap}");
        }
    }

    #[test]
    fn following_on_open_road_keeps_a_two_second_gap() {
        let (_, _, g) = town();
        let d = data();
        let lane = g.road_lanes().find(|l| l.kind == LaneKind::Highway).unwrap().id;
        let mut t = Traffic::new(9);
        for (k, s) in [(0u32, 400.0f32), (1, 300.0)] {
            t.cars.push(Car {
                id: k,
                kind: 0,
                paint: 0,
                state: CarState::Driving,
                lane,
                s_m: s,
                next: g.lanes[lane as usize].out.first().copied(),
                speed_m_s: 25.0,
                cruise_m_s: if k == 0 { 20.0 } else { 30.0 },
                fear_s: 0.0,
                hailed: false,
                arrived_s: None,
                granted: false,
                turns: 0,
                pos: Vec3::ZERO,
                heading_rad: 0.0,
                body: None,
                health: 1.0,
                wreck_s: 0.0,
            });
        }
        t.next_id = 2;
        let ground = FlatGround::default();
        for _ in 0..(60.0 / DT) as usize {
            t.step(&view(&g, &d, &ground), Vec3::new(1500.0, 0.0, 0.0), DT);
        }
        let (a, b) = (&t.cars[0], &t.cars[1]);
        let gap = a.s_m - b.s_m - length_of(&d, a);
        assert!((b.speed_m_s - 20.0).abs() < 0.5, "the faster car settles behind: {}", b.speed_m_s);
        assert!(gap >= b.speed_m_s * 2.0, "gap {gap:.1} m at {:.1} m/s", b.speed_m_s);
    }

    #[test]
    fn turns_repeat_with_the_seed_and_differ_without_it() {
        let go = |seed: u64| {
            let (mut t, g, d) = filled(seed);
            run(&mut t, &g, &d, 30.0);
            t.fingerprint()
        };
        assert_eq!(go(11), go(11));
        assert_ne!(go(11), go(12));
    }

    #[test]
    fn lights_stand_where_an_arterial_meets_two_other_roads_and_nowhere_else() {
        // Survey W39, owner 2026-10-26: "Lights on the arterials"; other junctions take turns by arrival.
        let (_, _, g) = town();
        assert!(g.nodes.iter().any(|n| n.signal), "the town's arterials have lit junctions");
        assert!(g.nodes.iter().any(|n| !n.signal), "and its side streets take turns");
        for n in &g.nodes {
            let roads: Vec<&crate::world::lanes::Road> = g.roads.iter().filter(|r| r.nodes.contains(&n.id)).collect();
            let arterial = roads.iter().any(|r| r.kind == LaneKind::Arterial);
            assert_eq!(n.signal, arterial && roads.len() >= 3, "junction {}", n.id);
            let into: Vec<&Lane> = g.road_lanes().filter(|l| l.to_node == n.id).collect();
            assert!(into.iter().all(|l| l.phase.is_some() == n.signal), "junction {}", n.id);
            if n.signal {
                for ph in [0, 1] {
                    assert!(into.iter().any(|l| l.phase == Some(ph)), "junction {} has phase {ph}", n.id);
                }
            }
        }
    }

    #[test]
    fn a_junctions_two_phases_are_never_open_together_and_each_gets_its_green() {
        let times = data().cfg.lights;
        for node in 0..40 {
            let mut green = [0u32; 2];
            for k in 0..(times.cycle_s() * 20.0) as u32 {
                let t_s = k as f32 * 0.05 + 1000.0;
                let (a, b) = (light(&times, node, 0, t_s), light(&times, node, 1, t_s));
                assert!(a == Light::Red || b == Light::Red, "junction {node} at {t_s} s: {a:?} and {b:?}");
                green[0] += u32::from(a == Light::Green);
                green[1] += u32::from(b == Light::Green);
            }
            assert!(green[0] > 0 && green[1] > 0, "junction {node}: {green:?}");
        }
    }

    #[test]
    fn cars_stop_on_red_and_take_a_lit_junction_only_on_green_or_an_amber_they_cannot_stop_for() {
        let (mut t, g, d) = filled(5);
        let ground = FlatGround::default();
        let times = d.cfg.lights;
        let mut stopped_at_red = 0;
        let mut crossed_lit = 0;
        for _ in 0..(240.0 / DT) as usize {
            let before: BTreeMap<u32, (u32, bool)> = t.cars.iter().map(|c| (c.id, (c.lane, c.granted))).collect();
            t.step(&view(&g, &d, &ground), Vec3::ZERO, DT);
            let now = t.time_s();
            for c in t.cars.iter().filter(|c| c.on_lane() && c.state == CarState::Driving) {
                let Some(&(was, was_granted)) = before.get(&c.id) else { continue };
                let lane = &g.lanes[was as usize];
                let Some(phase) = lane.phase else { continue };
                let shows = light(&times, lane.to_node, phase, now);
                let crossed = c.lane != was && g.lanes[c.lane as usize].kind == LaneKind::Connector;
                let given_now = !was_granted && ((c.lane == was && c.granted) || crossed);
                if given_now {
                    assert_ne!(shows, Light::Red, "car {} given junction {} on red", c.id, lane.to_node);
                }
                crossed_lit += usize::from(crossed);
                let at_line = lane.length_m() - c.s_m < length_of(&d, c) * 0.5 + d.cfg.stop_short_m + 1.0;
                if c.lane == was && !c.granted && shows == Light::Red && at_line && c.speed_m_s < 0.1 {
                    stopped_at_red += 1;
                }
            }
        }
        assert!(stopped_at_red > 0, "cars wait at the line on red");
        assert!(crossed_lit > 20, "cars cross the lit junctions: {crossed_lit}");
    }

    #[test]
    fn with_lights_on_the_arterials_traffic_never_deadlocks() {
        // Ten minutes of the town: junction crossings keep coming every minute, and no car that is not afraid,
        // hailed or behind a wreck stands still longer than three light cycles.
        let (mut t, g, d) = filled(9);
        let ground = FlatGround::default();
        let limit_s = d.cfg.lights.cycle_s() * 3.0;
        let mut still: BTreeMap<u32, f32> = BTreeMap::new();
        let turns = |t: &Traffic| t.cars.iter().map(|c| u64::from(c.turns)).sum::<u64>();
        let mut last = turns(&t);
        for minute in 0..10 {
            for _ in 0..(60.0 / DT) as usize {
                t.step(&view(&g, &d, &ground), Vec3::ZERO, DT);
                for c in &t.cars {
                    let waiting = c.on_lane() && c.state == CarState::Driving && c.fear_s == 0.0 && !c.hailed && c.speed_m_s < 0.1;
                    let s = still.entry(c.id).or_insert(0.0);
                    *s = if waiting { *s + DT } else { 0.0 };
                    assert!(
                        *s < limit_s,
                        "car {} has stood {s:.0} s on lane {} in minute {minute}",
                        c.id,
                        c.lane
                    );
                }
            }
            let now = turns(&t);
            assert!(now > last + 10, "minute {minute}: only {} junctions crossed", now - last);
            last = now;
        }
    }

    /// A car standing on its lane, for tests that set up a junction by hand.
    fn standing(id: u32, lane: u32, s_m: f32, next: Option<u32>, state: CarState) -> Car {
        Car {
            id,
            kind: 0,
            paint: 0,
            state,
            lane,
            s_m,
            next,
            speed_m_s: 0.0,
            cruise_m_s: 12.0,
            fear_s: 0.0,
            hailed: false,
            arrived_s: None,
            granted: false,
            turns: 0,
            pos: Vec3::ZERO,
            heading_rad: 0.0,
            body: None,
            health: 1.0,
            wreck_s: 0.0,
        }
    }

    #[test]
    fn a_car_held_at_red_keeps_its_place_for_the_room_on_the_lane_it_turns_into() {
        // Seed 9 once starved a street car turning onto a crawling arterial for minutes: while its light was red the
        // arterial's own cars took every gap that opened, so on its green there was never room. The room is shared
        // in arrival order, and a car held at red keeps its place in it.
        let (_, _, g) = town();
        let d = data();
        // a lit junction where two lanes of different phases both turn into one lane
        let (first, later, into) = g
            .road_lanes()
            .filter_map(|a| {
                let pa = a.phase?;
                g.road_lanes()
                    .filter(|b| b.to_node == a.to_node && b.phase == Some(1 - pa))
                    .find_map(|b| {
                        a.out.iter().find_map(|&ca| {
                            let o = g.lanes[ca as usize].out[0];
                            b.out
                                .iter()
                                .find(|&&cb| g.lanes[cb as usize].out[0] == o)
                                .map(|&cb| ((a, ca), (b, cb), o))
                        })
                    })
            })
            .next()
            .expect("a lit junction where both phases feed one lane");
        let ((la, ca), (lb, cb), o) = (first, later, into);
        let times = d.cfg.lights;
        // a moment the later car's light is green for a while and the first car's is red
        let now = (0..2000)
            .map(|k| k as f32 * 0.5)
            .find(|&t| {
                (0..20).all(|k| light(&times, lb.to_node, lb.phase.unwrap(), t + k as f32 * 0.25) == Light::Green)
                    && light(&times, la.to_node, la.phase.unwrap(), t) == Light::Red
            })
            .unwrap();
        let mut t = Traffic::new(9);
        t.time_s = now;
        let len = d.kinds[0].size_m[2];
        let near_line = |l: &Lane| l.length_m() - len * 0.5 - d.cfg.stop_short_m;
        let mut a = standing(0, la.id, near_line(la), Some(ca), CarState::Driving);
        a.arrived_s = Some(now - 10.0);
        let b = standing(1, lb.id, near_line(lb), Some(cb), CarState::Driving);
        // a wreck on the lane past the junction leaves room there for one car, not two
        let room = (len + d.cfg.min_gap_m) * 1.5;
        let wreck = standing(2, o, room + len * 0.5, None, CarState::Burnt);
        t.cars = vec![a, b, wreck];
        t.next_id = 3;
        let ground = FlatGround::default();
        t.step(&view(&g, &d, &ground), Vec3::ZERO, DT);
        assert!(!t.get(0).unwrap().granted, "the first car's light is red");
        assert!(
            !t.get(1).unwrap().granted,
            "the room is the first car's: the later car does not take it while the first is held at red"
        );
        for _ in 0..(times.cycle_s() / DT) as usize {
            t.step(&view(&g, &d, &ground), Vec3::ZERO, DT);
            if t.get(0).unwrap().granted || t.get(0).unwrap().lane != la.id {
                break;
            }
            assert!(!t.get(1).unwrap().granted, "the later car goes first");
        }
        assert!(
            t.get(0).unwrap().granted || t.get(0).unwrap().lane != la.id,
            "the first car takes its room on its next green"
        );
    }

    #[test]
    fn seed_11_drives_the_golden_traffic() {
        let (mut t, g, d) = filled(11);
        run(&mut t, &g, &d, 30.0);
        assert_eq!(
            (t.cars.len(), t.fingerprint()),
            GOLDEN_11,
            "the seed-11 traffic changed; if that was intended, update the golden values"
        );
    }

    const GOLDEN_11: (usize, u64) = (455, 8072100176767906781);

    #[test]
    fn a_blast_within_120_m_stops_the_traffic_for_20_to_40_s() {
        let (mut t, g, d) = filled(13);
        run(&mut t, &g, &d, 5.0);
        let at = Vec3::new(0.0, 0.0, 0.0);
        t.blast(&d, at, 4.0, false);
        let near: Vec<u32> = t
            .cars
            .iter()
            .filter(|c| c.state == CarState::Driving && c.pos.distance(at) < 100.0)
            .map(|c| c.id)
            .collect();
        assert!(!near.is_empty());
        run(&mut t, &g, &d, 5.0);
        for id in &near {
            let c = t.get(*id).unwrap();
            assert!(c.speed_m_s < 0.01, "car {id} stopped");
            assert!(c.fear_s >= 15.0 - 5.0 && c.fear_s <= 35.0, "{}", c.fear_s);
        }
        run(&mut t, &g, &d, 40.0);
        assert!(near.iter().filter_map(|id| t.get(*id)).all(|c| c.fear_s == 0.0), "and drives on");
    }

    #[test]
    fn a_blast_sets_cars_alight_within_its_reach_and_the_wreck_stays() {
        let (mut t, g, d) = filled(14);
        let target = t.cars.iter().find(|c| c.state == CarState::Driving).unwrap().clone();
        let n = t.blast(&d, target.pos, 8.0, true);
        assert!(n >= 1);
        let harms = t.take_harms();
        assert!(
            harms
                .iter()
                .any(|h| h.car == target.id && h.by_player && h.occupied && h.cause == HarmCause::Blast)
        );
        let at = t.get(target.id).unwrap().pos;
        run(&mut t, &g, &d, d.cfg.burn_s + 1.0);
        let c = t.get(target.id).unwrap();
        assert_eq!(c.state, CarState::Burnt);
        assert!(c.pos.distance(at) < 0.01, "the wreck stays put");
    }

    #[test]
    fn a_shot_must_beat_5_mm_to_set_a_car_alight() {
        let (mut t, _, d) = filled(15);
        let id = t.cars[0].id;
        assert!(!t.hit(&d, id, 3.0, true));
        assert!(t.take_harms().is_empty());
        assert!(t.hit(&d, id, 20.0, true));
        assert!(matches!(t.get(id).unwrap().state, CarState::Burning { .. }));
        assert_eq!(t.take_harms().len(), 1);
    }

    #[test]
    fn the_foremans_left_foot_crushes_a_sedan_and_it_stays_a_wreck() {
        let (mut t, g, d) = filled(16);
        let sedan = t.cars.iter().find(|c| c.kind == 0 && c.state == CarState::Driving).unwrap().clone();
        let sole = Vec2::new(sedan.pos.x, sedan.pos.z) + forward2(sedan.heading_rad) * 2.0;
        assert!(t.crush(&d, sole, true) >= 1);
        assert_eq!(t.get(sedan.id).unwrap().state, CarState::Crushed);
        let h = t.take_harms();
        assert!(h.iter().any(|h| h.car == sedan.id && h.cause == HarmCause::Crushed && h.by_player));
        run(&mut t, &g, &d, 60.0);
        let c = t.get(sedan.id).unwrap();
        assert_eq!(c.state, CarState::Crushed);
        assert!(c.pos.distance(sedan.pos) < 0.01);
        // a foot that misses by more than its radius crushes nothing
        let far = sole + right_of(forward2(sedan.heading_rad)) * 20.0;
        let before = t.cars.iter().filter(|c| c.state == CarState::Crushed).count();
        t.crush(&d, far, true);
        assert!(t.cars.iter().filter(|c| c.state == CarState::Crushed).count() <= before + 2);
    }

    #[test]
    fn a_moving_mech_frightens_cars_and_shoves_one_in_its_path() {
        let (mut t, g, d) = filled(17);
        let victim = t.cars.iter().find(|c| c.state == CarState::Driving).unwrap().clone();
        let mech = MechNear {
            at: Vec2::new(victim.pos.x, victim.pos.z) + Vec2::new(2.0, 0.0),
            vel: Vec2::new(-3.0, 0.0),
            radius_m: 4.0,
            piloted: true,
        };
        let ground = FlatGround::default();
        let v = TrafficView {
            mechs: &[mech],
            ..view(&g, &d, &ground)
        };
        t.step(&v, Vec3::ZERO, DT);
        let c = t.get(victim.id).unwrap();
        assert!(c.body.is_some(), "shoved off its lane");
        assert!(c.health < 1.0);
        assert!(
            t.take_harms()
                .iter()
                .any(|h| h.car == victim.id && h.cause == HarmCause::Shoved && h.by_player)
        );
        let near = t
            .cars
            .iter()
            .filter(|c| c.state == CarState::Driving && Vec2::new(c.pos.x, c.pos.z).distance(mech.at) < 50.0)
            .count();
        let afraid = t
            .cars
            .iter()
            .filter(|c| c.state == CarState::Driving && Vec2::new(c.pos.x, c.pos.z).distance(mech.at) < 50.0 && c.fear_s > 0.0)
            .count();
        assert_eq!(near, afraid);
    }

    #[test]
    fn taking_a_traffic_car_puts_its_driver_on_foot_running_for_a_door() {
        let (mut t, g, d) = filled(18);
        let id = t.cars.iter().find(|c| c.state == CarState::Driving).unwrap().id;
        let ground = FlatGround::default();
        let refuge = |p: Vec2| p + Vec2::new(30.0, 0.0);
        assert!(!t.take(&d, id, &ground, &refuge), "still moving");
        t.hail(id);
        run(&mut t, &g, &d, 8.0);
        assert!(t.get(id).unwrap().speed_m_s < 0.01, "a hailed car stops");
        assert!(t.take(&d, id, &ground, &refuge));
        assert_eq!(t.get(id).unwrap().state, CarState::Driven);
        assert_eq!(t.runners.len(), 1);
        let start = t.runners[0].at;
        let v = TrafficView {
            driver: Some(Drive {
                throttle: 1.0,
                ..Default::default()
            }),
            ..view(&g, &d, &ground)
        };
        for _ in 0..30 {
            t.step(&v, Vec3::ZERO, DT);
        }
        assert!(t.runners[0].at.distance(start) > 4.0, "the driver runs");
        assert!(t.get(id).unwrap().speed_m_s > 1.0, "the player drives off");
        for _ in 0..(10.0 / DT) as usize {
            t.step(&v, Vec3::ZERO, DT);
        }
        assert!(t.runners.is_empty(), "and is gone through the door");
        t.leave(id);
        assert_eq!(t.get(id).unwrap().state, CarState::Loose);
    }

    #[test]
    fn a_car_crushed_with_the_player_in_it_throws_them_clear() {
        let (mut t, _, d) = filled(19);
        let id = t.cars.iter().find(|c| c.state == CarState::Parked).unwrap().id;
        let ground = FlatGround::default();
        assert!(t.take(&d, id, &ground, &|p| p));
        assert!(t.runners.is_empty(), "a parked car has nobody in it");
        let p = t.get(id).unwrap().pos;
        t.crush(&d, Vec2::new(p.x, p.z), true);
        assert!(t.take_driver_thrown());
        assert!(!t.take_harms()[0].occupied, "the player's own car is not a civilian's");
    }
}
