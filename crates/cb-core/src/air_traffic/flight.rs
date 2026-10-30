//! One civil aircraft's flight (openspec change `civil-air-traffic`, design sections 2-4): the take-off
//! roll and climb-out, cruise, the straight-in final and landing on a base's strip, circling a city,
//! turning clear of a fight, and the fall after it is shot down. A point with a heading, a speed and a
//! climb rate, stepped at the fixed step; pure and deterministic.

use super::config::{AirTrafficConfig, AircraftSpec, RouteConfig};
use super::model::ZoneKind;
use crate::units::{forward, heading_of, turn_toward, wrap};
use crate::world::AirBase;
use glam::{Vec2, Vec3};

const G_M_S2: f32 = 9.81;
/// Speed changes in the air at this rate, metres a second squared.
const AIR_ACCEL_M_S2: f32 = 1.5;
/// The most a plane banks in a full-rate turn, radians (a helicopter: two thirds of it).
const MAX_BANK_RAD: f32 = 0.45;
/// The ground under the next this many seconds of flight sets the cruise height.
const TERRAIN_LOOK_S: f32 = 12.0;
/// Rising ground is looked for this far ahead, seconds of flight, so a climb starts in time.
const TERRAIN_WARN_S: f32 = 120.0;
/// ...every this many seconds of flight.
const TERRAIN_WARN_STEP_S: f32 = 6.0;
/// Climbing steeply it slows to this share above its approach speed (its best climb), metres a second.
const CLIMB_SPEED_OVER_APPROACH: f32 = 1.2;
/// The ground near it is looked at again this often, seconds.
const NEAR_LOOK_S: f32 = 1.0;
/// A route aircraft looks along its way to the next point for its highest ground this often, seconds.
const ROUTE_LOOK_S: f32 = 5.0;
/// ...every this many metres.
const ROUTE_LOOK_STEP_M: f32 = 400.0;
/// Over the fix this far above the glide path it holds, circling down, before turning in, metres.
const HOLD_ABOVE_M: f32 = 150.0;
/// The localizer turns back toward the centre line over this share of a turn's radius.
const LOCALIZER_LEAD: f32 = 0.4;
/// It lands only this near the strip's height at the threshold; higher, it goes round again, metres.
const TOUCHDOWN_WINDOW_M: f32 = 20.0;
/// Extra angle kept between a route bent round a fight and the fight's edge, radians.
const CLEAR_MARGIN_RAD: f32 = 0.15;
/// A way that passes within this of a fight's edge is bent round it too, so an aircraft still turning onto the
/// tangent never cuts inside the circle, metres.
const CLEAR_BUFFER_M: f32 = 100.0;

/// Where a fight is: everywhere within `radius_m` of `centre` (x, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// The fight's centre (x, z).
    pub centre: Vec2,
    /// Its reach, metres.
    pub radius_m: f32,
}

/// True when `p` is inside any zone, grown by `extra_m`.
pub fn in_zone(p: Vec2, zones: &[Zone], extra_m: f32) -> bool {
    zones.iter().any(|z| z.centre.distance(p) < z.radius_m + extra_m)
}

/// The way to fly from `pos` wanting to go along `want` (a unit vector) without entering a zone:
/// straight out of a zone it is inside; along the near tangent of the first zone the way ahead
/// crosses; else `want`. The one rule for keeping clear of a fight.
pub fn steer_clear(pos: Vec2, want: Vec2, zones: &[Zone]) -> Vec2 {
    let inside = zones
        .iter()
        .filter(|z| z.centre.distance(pos) < z.radius_m)
        .min_by(|a, b| (a.centre.distance(pos) / a.radius_m).total_cmp(&(b.centre.distance(pos) / b.radius_m)));
    if let Some(z) = inside {
        return (pos - z.centre).normalize_or(want);
    }
    let mut first: Option<(f32, &Zone)> = None;
    for z in zones {
        let to = z.centre - pos;
        let along = to.dot(want);
        if along <= 0.0 || (to - want * along).length() >= z.radius_m + CLEAR_BUFFER_M {
            continue;
        }
        if first.is_none_or(|(a, _)| along < a) {
            first = Some((along, z));
        }
    }
    let Some((_, z)) = first else { return want };
    let to = z.centre - pos;
    let d = to.length().max(1e-3);
    let ang = ((z.radius_m + CLEAR_BUFFER_M) / d).min(1.0).asin() + CLEAR_MARGIN_RAD;
    let side = if want.perp_dot(to) >= 0.0 { 1.0 } else { -1.0 };
    Vec2::from_angle(-side * ang).rotate(to / d)
}

/// A base's landing strip, flown from `start` (the threshold) toward `end`: its runway, 1.5 to 2 km (survey W45,
/// `world::bases::Runway`), less `strip_margin_m` at each end, flown either way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strip {
    /// The base it belongs to.
    pub base: u32,
    /// Where it begins: the touchdown point landing, the start of the roll taking off (x, z).
    pub start: Vec2,
    /// Where it ends (x, z).
    pub end: Vec2,
    /// The apron's height.
    pub ground_m: f32,
}

impl Strip {
    /// Its direction, a unit vector (x, z).
    pub fn dir(&self) -> Vec2 {
        (self.end - self.start).normalize_or(Vec2::X)
    }

    /// Its length, metres.
    pub fn len(&self) -> f32 {
        self.start.distance(self.end)
    }

    /// The heading along it.
    pub fn heading(&self) -> f32 {
        let d = self.dir();
        heading_of(Vec3::new(d.x, 0.0, d.y))
    }

    /// The final approach fix: `final_m` out along the centre line before the threshold.
    pub fn fix(&self, final_m: f32) -> Vec2 {
        self.start - self.dir() * final_m
    }
}

/// A base's runway as a strip both ways: from its `a` end to its `b` end first, then back.
pub fn strips(base: &AirBase, cfg: &RouteConfig) -> [Strip; 2] {
    let r = base.runway;
    let d = r.dir();
    let w = r.a + d * cfg.strip_margin_m;
    let e = r.b - d * cfg.strip_margin_m;
    let s = |start, end| Strip {
        base: base.id,
        start,
        end,
        ground_m: base.ground_m,
    };
    [s(w, e), s(e, w)]
}

/// The strip to land on at `base` arriving from `from`: the one whose final approach fix is nearer.
pub fn landing_strip(base: &AirBase, cfg: &RouteConfig, from: Vec2) -> Strip {
    let [a, b] = strips(base, cfg);
    if a.fix(cfg.final_m).distance(from) <= b.fix(cfg.final_m).distance(from) {
        a
    } else {
        b
    }
}

/// The strip to land on at `base` arriving from `from`, knowing the ground: of the two, one whose final
/// approach clears the ground under its glide path by `clear_m`, the nearer if both do; the one with
/// more room under it if neither does.
pub fn best_strip(base: &AirBase, cfg: &RouteConfig, from: Vec2, ground: &dyn Fn(f32, f32) -> f32, clear_m: f32) -> Strip {
    let room = |s: &Strip| {
        let n = (cfg.final_m / ROUTE_LOOK_STEP_M).ceil().max(1.0) as usize;
        (0..=n)
            .map(|i| {
                let out = cfg.final_m * i as f32 / n as f32;
                let p = s.start - s.dir() * out;
                s.ground_m + out * cfg.glide_deg.to_radians().tan() - ground(p.x, p.y)
            })
            .fold(f32::INFINITY, f32::min)
    };
    let [a, b] = strips(base, cfg);
    let (ra, rb) = (room(&a), room(&b));
    let near_a = a.fix(cfg.final_m).distance(from) <= b.fix(cfg.final_m).distance(from);
    match (ra >= clear_m, rb >= clear_m) {
        (true, true) => {
            if near_a {
                a
            } else {
                b
            }
        }
        (true, false) => a,
        (false, true) => b,
        (false, false) => {
            if ra >= rb {
                a
            } else {
                b
            }
        }
    }
}

/// The strip to take off along at `base` heading for `toward`: the one pointing more toward it.
pub fn departure_strip(base: &AirBase, cfg: &RouteConfig, toward: Vec2) -> Strip {
    let [a, b] = strips(base, cfg);
    if a.dir().dot(toward - a.start) >= b.dir().dot(toward - b.start) {
        a
    } else {
        b
    }
}

/// What an aircraft is doing with its flight.
#[derive(Clone, Debug, PartialEq)]
pub enum Flight {
    /// Flying to a base: over each `via` point in order (none for a scheduled route), then the final
    /// approach to `strip`. The career loop's air drop is a route with its drop point as a `via`.
    Route {
        /// The destination base.
        to: u32,
        /// The strip it lands on there.
        strip: Strip,
        /// Points to overfly at cruise first (x, z).
        via: Vec<Vec2>,
        /// The next of them.
        next_via: usize,
    },
    /// A military patrol (airfield-traffic): circling over its base for `left_s` more, then home to field `home`
    /// (a helicopter to its pad at `pad`).
    Patrol {
        /// Its home field.
        home: u32,
        /// The circle's centre (x, z).
        centre: Vec2,
        /// The circle's radius.
        radius_m: f32,
        /// +1 circles left, -1 right.
        dir: f32,
        /// Seconds of patrol left.
        left_s: f32,
        /// A helicopter's pad (x, z): where it settles back.
        pad: Option<Vec2>,
    },
    /// A helicopter flying to a pad at `at` (x, z) on field `field`, to settle on it.
    ToPad {
        /// The field.
        field: u32,
        /// The pad.
        at: Vec2,
    },
    /// Circling a city.
    Orbit {
        /// The city.
        city: u32,
        /// The circle's centre (x, z).
        centre: Vec2,
        /// The circle's radius.
        radius_m: f32,
        /// +1 circles left, -1 right.
        dir: f32,
    },
}

/// Where an aircraft is in its flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// The take-off roll along a strip, speeding up at `accel_m_s2`.
    Roll {
        /// Its acceleration, set so it lifts off at the strip's end.
        accel_m_s2: f32,
    },
    /// Climbing straight out after lift-off.
    Climb,
    /// Flying its route at cruise (to its `via` points, then the approach's entry).
    Enroute,
    /// Joining the strip's extended centre line between the approach's entry and the final fix.
    Intercept,
    /// On the final approach: lined up on the strip, down the glide path.
    Final,
    /// Braking along the strip after touchdown at `decel_m_s2`.
    Rollout {
        /// Its deceleration, set so it stops before the strip's end.
        decel_m_s2: f32,
    },
    /// At a stand or pad between flights for `t_s` (airfield-traffic); the traffic sends it off.
    Stand {
        /// Time at the stand.
        t_s: f32,
    },
    /// Taxiing along its taxi line (`Aircraft::taxi`), `s_m` along it; the first `push_m` it is pushed back,
    /// tail first; at the end it does `then`.
    Taxi {
        /// Metres along the line.
        s_m: f32,
        /// The push-back's length at its start, metres.
        push_m: f32,
        /// What it does at the line's end.
        then: Then,
    },
    /// Waiting at the holding point for `t_s`; the traffic clears it onto the runway.
    Hold {
        /// Time waiting.
        t_s: f32,
    },
    /// A helicopter lifting straight up off its pad.
    Lift,
    /// A helicopter settling onto its pad.
    Settle,
    /// Flown by the pilot, who stole it (`fly-stolen-aircraft`): the pilot's flight model moves it, not the traffic.
    /// `wheels` when it stands or rolls on them.
    Piloted {
        /// On its wheels or skids.
        wheels: bool,
    },
    /// Stopped on the apron for `t_s`.
    Parked {
        /// Time stopped.
        t_s: f32,
    },
    /// Circling its city.
    Orbit,
    /// Leaving for good (its city came under attack): straight on, clear of fights.
    Leaving,
    /// Shot down and falling.
    Falling,
    /// On the ground, wrecked.
    Wrecked,
}

/// What a taxiing aircraft does at the end of its taxi line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    /// Waits at the holding point.
    Hold,
    /// Turns onto its departure strip and rolls.
    Roll,
    /// Stops at its stand, facing `Aircraft::stand_heading`.
    Stand,
}

/// A civil aircraft.
#[derive(Clone, Debug, PartialEq)]
pub struct Aircraft {
    /// Stable id.
    pub id: u32,
    /// Its kind's name in `AirTrafficConfig::kinds`.
    pub kind: String,
    /// The fuselage's centre line at its middle.
    pub pos: Vec3,
    /// Velocity.
    pub vel: Vec3,
    /// Facing (`units::forward`).
    pub heading_rad: f32,
    /// Nose up, radians (drawn).
    pub pitch_rad: f32,
    /// Bank, left wing down positive, radians (drawn).
    pub bank_rad: f32,
    /// Speed along the heading.
    pub speed_m_s: f32,
    /// Structure points left (a kind with no damage zones).
    pub hp: f32,
    /// A modelled airliner's damage zones' structure points left, in its zones' order (airliner-models).
    pub zone_hp: Vec<f32>,
    /// Which of its zones have broken away.
    pub broken: Vec<bool>,
    /// Where it is in its flight.
    pub phase: Phase,
    /// Its flight.
    pub flight: Flight,
    /// The highest ground on the way to where it flies next, plus its clearance, and how far off it
    /// is (metres, metres): looked again every `ROUTE_LOOK_S`.
    pub route_floor: (f32, f32),
    /// Time until it looks along its way again.
    pub look_due_s: f32,
    /// What it last read of the ground near it (every `NEAR_LOOK_S`: the ground is noise evaluated
    /// point by point, about 3 microseconds a read, so it is read sparingly).
    pub sight: Sight,
    /// The base whose approach it has chosen by the ground under it (`best_strip`).
    pub approach_for: Option<u32>,
    /// Its taxi line on the ground (x, z), while taxiing.
    pub taxi: Vec<Vec2>,
    /// The field and stand it holds (standing at it, or taxiing to or from it).
    pub stand: Option<(u32, usize)>,
    /// The heading it stands at, at its stand.
    pub stand_heading: f32,
    /// The field it is on (on the ground), or flies from.
    pub field: Option<u32>,
    /// The strip it takes off along, once cleared to roll.
    pub departure: Option<Strip>,
    /// Stolen by the pilot (`fly-stolen-aircraft`): the traffic neither flies it nor sends it anywhere again.
    pub stolen: bool,
    /// Scrambled against the pilot and armed: once over them it is handed to the invasion's units to attack
    /// (`Traffic::take_attackers`).
    pub hostile: bool,
    /// Scrambled against a hostile pilot (airfield-traffic): it leaves its stand at once and patrols over this point
    /// (x, z), not its base.
    pub scramble: Option<Vec2>,
}

/// What an aircraft last read of the ground near it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sight {
    /// Under it.
    pub here_m: f32,
    /// The higher of under it and `TERRAIN_LOOK_S` of flight ahead.
    pub ahead_m: f32,
    /// The highest ground on its next 1.5 looks of flight: what it must never sink into.
    pub under_m: f32,
    /// The least height the ground in the next `TERRAIN_WARN_S` allows, climbing (`terrain_floor`).
    pub floor_m: f32,
    /// Time until the next look.
    pub due_s: f32,
}

impl Default for Sight {
    fn default() -> Self {
        Self {
            here_m: f32::NEG_INFINITY,
            ahead_m: f32::NEG_INFINITY,
            under_m: f32::NEG_INFINITY,
            floor_m: f32::NEG_INFINITY,
            due_s: 0.0,
        }
    }
}

/// The world as an aircraft sees it each step.
pub struct Air<'a> {
    /// Ground height under a point (x, z).
    pub ground: &'a dyn Fn(f32, f32) -> f32,
    /// Fights to keep clear of.
    pub zones: &'a [Zone],
    /// The air bases.
    pub bases: &'a [AirBase],
    /// Tuning.
    pub cfg: &'a AirTrafficConfig,
}

/// What a hit on one part did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitOutcome {
    /// It brought the aircraft down.
    pub downed: bool,
    /// The damage zones it broke away (the hit zone and every zone hanging from it), in the zones' order.
    pub broke: Vec<usize>,
}

/// What happened to an aircraft this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlightStep {
    /// It hit the ground falling (or was downed on it): a crash at `pos`.
    pub crashed: bool,
    /// It passed over its route's `via` point with this index.
    pub passed: Option<usize>,
    /// It touched down.
    pub landed: bool,
    /// Its destination came under attack and it turned for another base.
    pub diverted: bool,
    /// It has finished (parked long enough) and may leave the world.
    pub done: bool,
}

/// How high the fuselage's centre line sits over the ground on its wheels or skids: its model's own, measured by its
/// build, when it has one that says (the airliners), else from its size.
pub fn rest_m(spec: &AircraftSpec) -> f32 {
    if let Some(r) = spec.model.as_ref().and_then(|m| m.loaded.as_ref()).and_then(|l| l.manifest.rest_m) {
        return r;
    }
    spec.fuselage_m * 0.5 + (spec.height_m - spec.fuselage_m) * 0.15
}

/// Nose up standing on its gear, radians (a taildragger; zero for the rest).
pub fn rest_pitch_rad(spec: &AircraftSpec) -> f32 {
    spec.model
        .as_ref()
        .and_then(|m| m.loaded.as_ref())
        .map_or(0.0, |l| l.manifest.rest_pitch_deg.to_radians())
}

impl Aircraft {
    /// A new aircraft at `pos`, facing `heading_rad`, at `speed_m_s`.
    pub fn new(
        id: u32,
        kind: &str,
        spec: &AircraftSpec,
        pos: Vec3,
        heading_rad: f32,
        speed_m_s: f32,
        phase: Phase,
        flight: Flight,
    ) -> Self {
        Self {
            id,
            kind: kind.into(),
            pos,
            vel: forward(heading_rad) * speed_m_s,
            heading_rad,
            pitch_rad: 0.0,
            bank_rad: 0.0,
            speed_m_s,
            hp: spec.hp,
            zone_hp: spec.zones().iter().map(|z| z.share * spec.hp).collect(),
            broken: vec![false; spec.zones().len()],
            phase,
            flight,
            route_floor: (f32::NEG_INFINITY, 0.0),
            // Looks are spread over the steps by id, so no two aircraft read the ground in one step.
            look_due_s: (id % 37) as f32 / 37.0 * ROUTE_LOOK_S,
            sight: Sight::default(),
            approach_for: None,
            taxi: Vec::new(),
            stand: None,
            stand_heading: heading_rad,
            field: None,
            departure: None,
            stolen: false,
            hostile: false,
            scramble: None,
        }
    }

    /// Standing at `stand` of field `field` at `at` (its wheels' ground under it), facing `heading_rad`, `t_s` into its
    /// turnaround (airfield-traffic).
    #[allow(clippy::too_many_arguments)]
    pub fn standing(id: u32, kind: &str, spec: &AircraftSpec, field: u32, stand: usize, at: Vec3, heading_rad: f32, t_s: f32) -> Self {
        let mut a = Self::new(
            id,
            kind,
            spec,
            at + Vec3::Y * rest_m(spec),
            heading_rad,
            0.0,
            Phase::Stand { t_s },
            Flight::Orbit {
                city: 0,
                centre: Vec2::new(at.x, at.z),
                radius_m: 1.0,
                dir: 1.0,
            },
        );
        a.stand = Some((field, stand));
        a.stand_heading = heading_rad;
        a.field = Some(field);
        a
    }

    /// Ready to roll from the start of `from` (a departure strip) for base `to`.
    pub fn departing(id: u32, kind: &str, spec: &AircraftSpec, from: Strip, to: &AirBase, cfg: &RouteConfig) -> Self {
        let accel = spec.approach_m_s * spec.approach_m_s / (2.0 * (from.len() * 0.9).max(1.0));
        let at = Vec3::new(from.start.x, from.ground_m + rest_m(spec), from.start.y);
        let strip = landing_strip(to, cfg, from.end);
        Self::new(
            id,
            kind,
            spec,
            at,
            from.heading(),
            0.0,
            Phase::Roll { accel_m_s2: accel },
            Flight::Route {
                to: to.id,
                strip,
                via: Vec::new(),
                next_via: 0,
            },
        )
    }

    /// True while it is in the world and not wrecked (a target, a thing to draw flying).
    pub fn flying(&self) -> bool {
        !matches!(self.phase, Phase::Wrecked)
    }

    /// True once it has been shot down.
    pub fn downed(&self) -> bool {
        matches!(self.phase, Phase::Falling | Phase::Wrecked)
    }

    /// True while it is on its wheels.
    pub fn on_ground(&self) -> bool {
        matches!(
            self.phase,
            Phase::Roll { .. }
                | Phase::Rollout { .. }
                | Phase::Parked { .. }
                | Phase::Stand { .. }
                | Phase::Taxi { .. }
                | Phase::Hold { .. }
                | Phase::Piloted { wheels: true }
        )
    }

    /// Where along its taxi line `s_m` is: the point and the line's direction there.
    pub fn taxi_at(&self, s_m: f32) -> (Vec2, Vec2) {
        let mut left = s_m.max(0.0);
        for w in self.taxi.windows(2) {
            let d = w[0].distance(w[1]);
            if left <= d && d > 1e-3 {
                return (w[0].lerp(w[1], left / d), (w[1] - w[0]) / d);
            }
            left -= d;
        }
        let n = self.taxi.len();
        let dir = if n >= 2 {
            (self.taxi[n - 1] - self.taxi[n - 2]).normalize_or(Vec2::X)
        } else {
            self.fwd2()
        };
        (self.taxi.last().copied().unwrap_or_else(|| self.xz()), dir)
    }

    /// Its taxi line's length, metres.
    pub fn taxi_len(&self) -> f32 {
        self.taxi.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// Takes a hit through the one armour rule (`units::armour_hit`). True when this hit downed it.
    pub fn hit(&mut self, penetration_mm: f32, damage: f32, armour_mm: f32) -> bool {
        if self.downed() {
            return false;
        }
        if crate::units::armour_hit(&mut self.hp, penetration_mm, damage, armour_mm) {
            self.phase = Phase::Falling;
            return true;
        }
        false
    }

    /// Takes a hit on part `part` (a damage zone's index; a kind with no zones takes it as a whole) through the one
    /// armour rule. A zone at zero breaks away with every zone hanging from it (design section 5); losing any zone but
    /// an engine, or every engine, brings it down.
    pub fn hit_part(&mut self, spec: &AircraftSpec, part: usize, penetration_mm: f32, damage: f32) -> HitOutcome {
        let zones = spec.zones();
        if zones.is_empty() || part >= zones.len() || self.zone_hp.len() != zones.len() {
            return HitOutcome {
                downed: self.hit(penetration_mm, damage, spec.armour_mm),
                broke: Vec::new(),
            };
        }
        let mut out = HitOutcome::default();
        if self.downed() || self.broken[part] {
            return out;
        }
        if !crate::units::armour_hit(&mut self.zone_hp[part], penetration_mm, damage, spec.armour_mm) {
            return out;
        }
        for z in super::model::falls_with(zones, part) {
            if !self.broken[z] {
                self.broken[z] = true;
                out.broke.push(z);
            }
        }
        let engines = zones.iter().filter(|z| z.kind == ZoneKind::Engine).count();
        let all_engines = engines > 0 && self.engines_lost(spec) >= engines;
        if out.broke.iter().any(|&z| zones[z].kind.downs()) || all_engines {
            self.phase = Phase::Falling;
            out.downed = true;
        }
        out
    }

    /// How many of its engines it has lost.
    pub fn engines_lost(&self, spec: &AircraftSpec) -> usize {
        spec.zones()
            .iter()
            .zip(&self.broken)
            .filter(|(z, b)| **b && z.kind == ZoneKind::Engine)
            .count()
    }

    /// The share of its engines' thrust it has lost (0 to 1).
    pub fn thrust_lost(&self, spec: &AircraftSpec) -> f32 {
        let engines = spec.zones().iter().filter(|z| z.kind == ZoneKind::Engine).count();
        if engines == 0 {
            return 0.0;
        }
        self.engines_lost(spec) as f32 / engines as f32
    }

    /// One fixed step of `dt_s`.
    pub fn step(&mut self, spec: &AircraftSpec, air: &Air, dt_s: f32) -> FlightStep {
        // an engine shot away: it flies on, slower, climbing less, on the ones it has left
        let lost = self.thrust_lost(spec);
        let slowed;
        let spec = if lost > 0.0 {
            slowed = spec.with_thrust_lost(lost);
            &slowed
        } else {
            spec
        };
        let mut out = FlightStep::default();
        if !(dt_s > 0.0 && dt_s.is_finite()) {
            return out;
        }
        let rest = rest_m(spec);
        self.sight.due_s -= dt_s;
        let r = &air.cfg.routes;
        let turn_max = spec.turn_deg_s.to_radians() * dt_s;
        let mut vy = self.vel.y;
        let mut want_heading = self.heading_rad;
        match self.phase {
            Phase::Wrecked | Phase::Piloted { .. } => return out,
            Phase::Falling => {
                self.fall(spec, air, dt_s, &mut out);
                return out;
            }
            Phase::Parked { t_s } => {
                let t = t_s + dt_s;
                self.phase = Phase::Parked { t_s: t };
                self.speed_m_s = 0.0;
                self.vel = Vec3::ZERO;
                out.done = t >= r.park_s;
                return out;
            }
            Phase::Stand { t_s } | Phase::Hold { t_s } => {
                let t = t_s + dt_s;
                self.phase = if matches!(self.phase, Phase::Stand { .. }) {
                    Phase::Stand { t_s: t }
                } else {
                    Phase::Hold { t_s: t }
                };
                self.speed_m_s = 0.0;
                self.vel = Vec3::ZERO;
                return out;
            }
            Phase::Taxi { s_m, push_m, then } => {
                let pushing = s_m < push_m;
                let v = if pushing { r.pushback_m_s } else { r.taxi_m_s };
                let s = s_m + v * dt_s;
                let len = self.taxi_len();
                let (p, dir) = self.taxi_at(s.min(len));
                let g = (air.ground)(p.x, p.y);
                self.pos = Vec3::new(p.x, g + rest, p.y);
                self.speed_m_s = v;
                self.vel = Vec3::new(dir.x, 0.0, dir.y) * if pushing { -v } else { v };
                // pushed back it keeps facing the stand's way; taxiing it faces where it goes
                if !pushing && s < len {
                    self.heading_rad = wrap(turn_toward(
                        self.heading_rad,
                        heading_of(Vec3::new(dir.x, 0.0, dir.y)),
                        turn_max * 3.0,
                    ));
                }
                self.phase = Phase::Taxi { s_m: s, push_m, then };
                // at the line's end it turns where it stands onto the runway, or onto its stand, before it stops
                let face = match then {
                    Then::Roll => self.departure.map(|d| d.heading()),
                    Then::Stand => Some(self.stand_heading),
                    Then::Hold => None,
                };
                if s >= len
                    && let Some(face) = face
                    && wrap(self.heading_rad - face).abs() > 0.02
                {
                    self.heading_rad = wrap(turn_toward(self.heading_rad, face, turn_max * 3.0));
                    self.speed_m_s = 0.0;
                    self.vel = Vec3::ZERO;
                    self.bank_rad = 0.0;
                    self.pitch_rad = 0.0;
                    return out;
                }
                if s >= len {
                    self.speed_m_s = 0.0;
                    self.vel = Vec3::ZERO;
                    self.phase = match then {
                        Then::Hold => Phase::Hold { t_s: 0.0 },
                        Then::Stand => {
                            self.heading_rad = self.stand_heading;
                            Phase::Stand { t_s: 0.0 }
                        }
                        Then::Roll => match self.departure {
                            Some(strip) => {
                                self.heading_rad = strip.heading();
                                let left = (strip.len() - (self.xz() - strip.start).dot(strip.dir())).max(300.0);
                                Phase::Roll {
                                    accel_m_s2: spec.approach_m_s * spec.approach_m_s / (2.0 * left * 0.9),
                                }
                            }
                            None => Phase::Hold { t_s: 0.0 },
                        },
                    };
                }
                self.bank_rad = 0.0;
                self.pitch_rad = 0.0;
                return out;
            }
            Phase::Lift => {
                self.speed_m_s = 0.0;
                let here = (air.ground)(self.pos.x, self.pos.z);
                self.pos.y += r.lift_m_s * dt_s;
                self.vel = Vec3::Y * r.lift_m_s;
                if self.pos.y - here - rest >= r.lift_m {
                    self.phase = if matches!(self.flight, Flight::Route { .. }) {
                        Phase::Climb
                    } else {
                        Phase::Orbit
                    };
                }
                return out;
            }
            Phase::Settle => {
                let Flight::ToPad { at, .. } = self.flight else {
                    self.phase = Phase::Orbit;
                    return out;
                };
                let here = self.xz();
                let to = at - here;
                let d = to.length();
                let v = d.min(8.0);
                let step = if d > 1e-3 { to / d * (v * dt_s).min(d) } else { Vec2::ZERO };
                self.speed_m_s = v;
                if d > 1.0 {
                    self.heading_rad = wrap(turn_toward(self.heading_rad, heading_of(Vec3::new(to.x, 0.0, to.y)), turn_max));
                }
                let ground = (air.ground)(at.x, at.y) + rest;
                let down = if d < 20.0 { r.lift_m_s } else { 0.0 };
                self.pos = Vec3::new(here.x + step.x, (self.pos.y - down * dt_s).max(ground), here.y + step.y);
                self.vel = Vec3::new(step.x / dt_s, -down, step.y / dt_s);
                self.bank_rad = 0.0;
                self.pitch_rad = 0.0;
                if self.pos.y <= ground + 0.05 && d < 1.0 {
                    self.pos = Vec3::new(at.x, ground, at.y);
                    self.speed_m_s = 0.0;
                    self.vel = Vec3::ZERO;
                    self.heading_rad = self.stand_heading;
                    self.phase = Phase::Stand { t_s: 0.0 };
                    out.landed = true;
                }
                return out;
            }
            Phase::Roll { accel_m_s2 } => {
                // Along the flat runway: its height stays the strip's.
                self.speed_m_s += accel_m_s2 * dt_s;
                vy = 0.0;
                if self.speed_m_s >= spec.approach_m_s {
                    self.phase = Phase::Climb;
                }
            }
            Phase::Climb => {
                self.speed_m_s = ease(self.speed_m_s, spec.cruise_m_s, AIR_ACCEL_M_S2 * dt_s);
                vy = spec.climb_m_s;
                let fwd = self.fwd2();
                let sight = self.look(air, [fwd, fwd], spec, r);
                if self.pos.y - sight.here_m >= r.climb_out_m {
                    self.phase = if matches!(self.flight, Flight::Patrol { .. }) {
                        Phase::Orbit
                    } else {
                        Phase::Enroute
                    };
                }
            }
            Phase::Enroute => {
                if let Flight::ToPad { at, .. } = self.flight
                    && self.xz().distance(at) < 150.0
                {
                    self.phase = Phase::Settle;
                    return out;
                }
                self.divert_if_needed(air, &mut out);
                let (target, via) = self.route_target(r, spec);
                let pos2 = self.xz();
                let capture = turn_radius(spec.cruise_m_s, spec).max(300.0);
                let d = pos2.distance(target);
                let high = via.is_none() && self.high_for_approach(r, spec, rest);
                match via {
                    // A point to overfly is passed when over it, or when it falls behind inside a turn's
                    // reach (a turn could not have brought it nearer).
                    Some(i) if d < 150.0 || (d < capture && (target - pos2).dot(self.fwd2()) <= 0.0) => {
                        out.passed = Some(i);
                        if let Flight::Route { next_via, .. } = &mut self.flight {
                            *next_via += 1;
                        }
                    }
                    None if d < capture && !high && matches!(self.flight, Flight::Route { .. }) => self.phase = Phase::Intercept,
                    None if matches!(self.flight, Flight::Patrol { .. }) && d < capture => self.phase = Phase::Orbit,
                    _ => {}
                }
                let mut want = steer_clear(pos2, (target - pos2).normalize_or(self.fwd2()), air.zones);
                self.look_due_s -= dt_s;
                if self.look_due_s <= 0.0 {
                    self.look_due_s = ROUTE_LOOK_S;
                    self.route_floor = route_floor(air, pos2, target, r.min_clear_m);
                    // The approach to fly is the one with the ground well under its glide path, chosen
                    // once for each base it heads for.
                    if let (Flight::Route { to, strip, .. }, None) = (&mut self.flight, via)
                        && self.approach_for != Some(*to)
                        && let Some(dest) = air.bases.iter().find(|b| b.id == *to)
                    {
                        *strip = best_strip(dest, r, pos2, air.ground, r.min_clear_m * 0.5);
                        self.approach_for = Some(*to);
                    }
                }
                // Ground on the way it could not climb over even at its best climb: it circles where it
                // is, climbing, until it can (a spiral climb out of a base under the island's peak).
                let (floor_m, floor_at_m) = self.route_floor;
                let best = spec.cruise_m_s.min(spec.approach_m_s * CLIMB_SPEED_OVER_APPROACH);
                let reach_m = (floor_m - self.pos.y).max(0.0) / spec.climb_m_s * best;
                // At the approach's entry far above the glide path: it circles there, coming down.
                let holding = high && d < capture * 1.5;
                // (It decides a circle's width early, so the circle never swings into the slope.)
                let circle_m = 2.0 * turn_radius(best, spec);
                let circling = (floor_m > self.pos.y && reach_m + circle_m > floor_at_m) || holding;
                if circling {
                    want = Vec2::from_angle(std::f32::consts::FRAC_PI_2).rotate(self.fwd2());
                }
                want_heading = heading_of(Vec3::new(want.x, 0.0, want.y));
                let sight = self.look(air, [self.fwd2(), want], spec, r);
                let mut tgt = sight.ahead_m + spec.cruise_agl_m;
                if let (Flight::Route { strip, .. }, None) = (&self.flight, via) {
                    let out_m = approach_m(r, spec);
                    let path = pos2.distance(target) + out_m;
                    tgt = tgt.min(strip.ground_m + rest + path * r.glide_deg.to_radians().tan());
                }
                tgt = tgt.max(sight.floor_m).max(floor_m);
                vy = (tgt - self.pos.y).clamp(-spec.descent_m_s, spec.climb_m_s);
                // A long climb is flown slower, at its best climb speed: a steeper climb over a ridge.
                let speed = if circling || tgt - self.pos.y > 100.0 {
                    best
                } else {
                    spec.cruise_m_s
                };
                self.speed_m_s = ease(self.speed_m_s, speed, AIR_ACCEL_M_S2 * dt_s);
            }
            Phase::Intercept | Phase::Final => {
                self.divert_if_needed(air, &mut out);
                if out.diverted {
                    self.phase = Phase::Enroute;
                } else if let Flight::Route { strip, .. } = self.flight {
                    let d = strip.dir();
                    let along = (self.xz() - strip.start).dot(d);
                    let want = localizer(&strip, self.xz(), (turn_radius(self.speed_m_s, spec) * LOCALIZER_LEAD).max(300.0));
                    want_heading = heading_of(Vec3::new(want.x, 0.0, want.y));
                    self.speed_m_s = ease(self.speed_m_s, spec.approach_m_s, AIR_ACCEL_M_S2 * dt_s);
                    let glide = strip.ground_m + rest + (-along).max(0.0) * r.glide_deg.to_radians().tan();
                    let mut tgt = glide;
                    if self.phase == Phase::Intercept {
                        // Still outside the fix: down toward the glide path, clear of the ground.
                        tgt = tgt.max(self.look(air, [self.fwd2(), want], spec, r).floor_m);
                        if along >= -r.final_m {
                            self.phase = Phase::Final;
                        }
                    }
                    vy = (tgt - self.pos.y).clamp(-spec.descent_m_s, spec.climb_m_s);
                    let lateral = (self.xz() - strip.start).perp_dot(d).abs();
                    let lined_up = lateral < TOUCHDOWN_WINDOW_M * 2.0 && wrap(self.heading_rad - strip.heading()).abs() < 0.35;
                    if along >= 0.0 && (self.pos.y - (strip.ground_m + rest) > TOUCHDOWN_WINDOW_M || !lined_up) {
                        // Too high over the threshold (a ridge held it up), or not lined up: round again.
                        self.phase = Phase::Enroute;
                    } else if along >= 0.0 {
                        let left = (strip.len() - along).max(50.0);
                        self.phase = Phase::Rollout {
                            decel_m_s2: self.speed_m_s * self.speed_m_s / (2.0 * left),
                        };
                        self.pos.y = strip.ground_m + rest;
                        vy = 0.0;
                        out.landed = true;
                    }
                }
            }
            Phase::Rollout { decel_m_s2 } => {
                self.speed_m_s = (self.speed_m_s - decel_m_s2 * dt_s).max(0.0);
                if let Flight::Route { strip, .. } = self.flight {
                    want_heading = strip.heading();
                }
                vy = 0.0;
                if let Flight::Route { strip, .. } = self.flight {
                    self.pos.y = strip.ground_m + rest;
                }
                if self.speed_m_s <= 0.5 {
                    self.speed_m_s = 0.0;
                    self.phase = Phase::Parked { t_s: 0.0 };
                }
            }
            Phase::Orbit | Phase::Leaving => {
                let pos2 = self.xz();
                let mut want = self.fwd2();
                // a patrol's time over its base runs out: home, a helicopter to its pad, an aircraft to the runway
                if let Flight::Patrol { home, left_s, pad, .. } = &mut self.flight {
                    *left_s -= dt_s;
                    if *left_s <= 0.0 {
                        let (home, pad) = (*home, *pad);
                        if let Some(at) = pad {
                            self.flight = Flight::ToPad { field: home, at };
                        } else if let Some(b) = air.bases.iter().find(|b| b.id == home) {
                            self.flight = Flight::Route {
                                to: home,
                                strip: landing_strip(b, r, pos2),
                                via: Vec::new(),
                                next_via: 0,
                            };
                        }
                        self.phase = Phase::Enroute;
                    }
                }
                let circling = match &self.flight {
                    Flight::Orbit { centre, radius_m, dir, .. } | Flight::Patrol { centre, radius_m, dir, .. } => {
                        Some((*centre, *radius_m, *dir))
                    }
                    _ => None,
                };
                if let (Phase::Orbit, Some((centre, radius_m, dir))) = (self.phase, circling) {
                    let (centre, radius_m, dir) = (&centre, &radius_m, &dir);
                    if in_zone(*centre, air.zones, *radius_m) {
                        // Its city is in a fight: it leaves, away from the nearest one.
                        self.phase = Phase::Leaving;
                        if let Some(z) = air
                            .zones
                            .iter()
                            .min_by(|a, b| a.centre.distance(pos2).total_cmp(&b.centre.distance(pos2)))
                        {
                            want = (pos2 - z.centre).normalize_or(want);
                        }
                    } else {
                        let rel = pos2 - *centre;
                        let d = rel.length().max(1.0);
                        let radial = rel / d;
                        let tangent = radial.perp() * *dir;
                        let correct = ((radius_m - d) / radius_m).clamp(-3.0, 1.0);
                        want = (tangent + radial * correct * 2.0).normalize_or(tangent);
                    }
                }
                let want = steer_clear(pos2, want, air.zones);
                want_heading = heading_of(Vec3::new(want.x, 0.0, want.y));
                self.speed_m_s = ease(self.speed_m_s, spec.cruise_m_s, AIR_ACCEL_M_S2 * dt_s);
                let sight = self.look(air, [self.fwd2(), want], spec, r);
                let tgt = (sight.ahead_m + spec.cruise_agl_m).max(sight.floor_m);
                vy = (tgt - self.pos.y).clamp(-spec.descent_m_s, spec.climb_m_s);
            }
        }
        let before = self.heading_rad;
        self.heading_rad = wrap(turn_toward(self.heading_rad, want_heading, turn_max));
        let rate = wrap(self.heading_rad - before) / (spec.turn_deg_s.to_radians() * dt_s);
        let max_bank = if spec.role.rotorcraft() {
            MAX_BANK_RAD * 0.66
        } else {
            MAX_BANK_RAD
        };
        let bank = if self.on_ground() { 0.0 } else { rate.clamp(-1.0, 1.0) * max_bank };
        self.bank_rad = ease(self.bank_rad, bank, 0.6 * dt_s);
        self.vel = forward(self.heading_rad) * self.speed_m_s + Vec3::Y * vy;
        self.pitch_rad = if self.on_ground() { 0.0 } else { vy.atan2(self.speed_m_s.max(1.0)) };
        self.pos += self.vel * dt_s;
        if !self.on_ground() {
            // Never through the ground: a climb that could not keep up holds the aircraft on it.
            self.pos.y = self.pos.y.max(self.sight.under_m + rest);
        }
        out
    }

    fn fall(&mut self, spec: &AircraftSpec, air: &Air, dt_s: f32, out: &mut FlightStep) {
        let f = &air.cfg.fall;
        let keep = (-f.drag_per_s * dt_s).exp();
        let spin = if spec.role.rotorcraft() { 3.0 } else { 1.0 } * f.spin_deg_s.to_radians();
        self.vel.x *= keep;
        self.vel.z *= keep;
        self.vel.y -= G_M_S2 * dt_s;
        self.heading_rad = wrap(self.heading_rad + spin * dt_s);
        self.bank_rad = wrap(self.bank_rad + spin * 0.5 * dt_s);
        let h = Vec2::new(self.vel.x, self.vel.z).length();
        self.speed_m_s = h;
        self.pitch_rad = self.vel.y.atan2(h.max(1.0));
        self.pos += self.vel * dt_s;
        let ground = (air.ground)(self.pos.x, self.pos.z);
        if self.pos.y <= ground + rest_m(spec) * 0.5 {
            self.pos.y = ground;
            self.vel = Vec3::ZERO;
            self.speed_m_s = 0.0;
            self.phase = Phase::Wrecked;
            out.crashed = true;
        }
    }

    /// True when it is too high at the approach's entry to make the glide path down to the strip.
    fn high_for_approach(&self, r: &RouteConfig, spec: &AircraftSpec, rest: f32) -> bool {
        let Flight::Route { strip, .. } = &self.flight else { return false };
        let at_entry = strip.ground_m + rest + approach_m(r, spec) * r.glide_deg.to_radians().tan();
        self.pos.y > at_entry + HOLD_ABOVE_M
    }

    /// Its reading of the ground near it, looked at again when due: under it, ahead, the highest it
    /// will be over next, and the least height it may fly at now to clear the ground by `min_clear_m`
    /// along either way it may go (`dirs`, unit vectors) for the next `TERRAIN_WARN_S`, climbing at
    /// its best from here (a ridge two minutes ahead starts the climb now).
    fn look(&mut self, air: &Air, dirs: [Vec2; 2], spec: &AircraftSpec, r: &RouteConfig) -> Sight {
        let first = !self.sight.here_m.is_finite();
        if self.sight.due_s > 0.0 && !first {
            return self.sight;
        }
        let ground = |p: Vec2| (air.ground)(p.x, p.y);
        let here = self.xz();
        let speed = self.speed_m_s.max(1.0);
        let fwd = self.fwd2();
        let here_m = ground(here);
        let next = ground(here + fwd * speed * NEAR_LOOK_S * 1.5);
        let ahead_m = here_m.max(ground(here + fwd * speed * TERRAIN_LOOK_S));
        let mut floor_m = f32::NEG_INFINITY;
        for d in dirs {
            let mut t = TERRAIN_WARN_STEP_S;
            while t <= TERRAIN_WARN_S {
                let p = here + d * speed * t;
                floor_m = floor_m.max(ground(p) + r.min_clear_m - spec.climb_m_s * t);
                t += TERRAIN_WARN_STEP_S;
            }
        }
        self.sight = Sight {
            here_m,
            ahead_m,
            under_m: here_m.max(next),
            floor_m: floor_m.max(here_m + r.min_clear_m),
            // The first look's wait is spread by id, so no two aircraft read the ground in one step.
            due_s: if first {
                (1 + self.id % 13) as f32 / 13.0 * NEAR_LOOK_S
            } else {
                NEAR_LOOK_S
            },
        };
        self.sight
    }

    /// The point it flies for next on its route: its next `via` point (with its index), or the
    /// approach's entry on the strip's extended centre line. A city flight has none (its own centre).
    fn route_target(&self, r: &RouteConfig, spec: &AircraftSpec) -> (Vec2, Option<usize>) {
        match &self.flight {
            Flight::Route { strip, via, next_via, .. } => match via.get(*next_via) {
                Some(p) => (*p, Some(*next_via)),
                None => (strip.fix(approach_m(r, spec)), None),
            },
            Flight::Orbit { centre, .. } | Flight::Patrol { centre, .. } => (*centre, None),
            Flight::ToPad { at, .. } => (*at, None),
        }
    }

    /// A route whose destination is in a fight turns for the nearest base that is not.
    fn divert_if_needed(&mut self, air: &Air, out: &mut FlightStep) {
        let Flight::Route { to, strip, .. } = &mut self.flight else {
            return;
        };
        let Some(dest) = air.bases.iter().find(|b| b.id == *to) else {
            return;
        };
        if !in_zone(dest.centre, air.zones, 0.0) {
            return;
        }
        let here = Vec2::new(self.pos.x, self.pos.z);
        let next = air
            .bases
            .iter()
            .filter(|b| b.id != dest.id && !in_zone(b.centre, air.zones, 0.0))
            .min_by(|a, b| a.centre.distance(here).total_cmp(&b.centre.distance(here)).then(a.id.cmp(&b.id)));
        if let Some(b) = next {
            *to = b.id;
            *strip = landing_strip(b, &air.cfg.routes, here);
            out.diverted = true;
        }
    }

    /// Where a pilot on foot climbs in (on the ground beside it) and where their eye ends up in its seat, in the world,
    /// as it stands (`fly-stolen-aircraft`): its model's `DOOR_*` (the first by name) and `EYE_pilot`; a kind without
    /// them (a block-out) has a door on its left a third of the way back and the eye ahead of it, by its size.
    pub fn boarding(&self, spec: &AircraftSpec) -> (Vec3, Vec3) {
        let rest = rest_m(spec);
        let marks = spec.model.as_ref().and_then(|m| m.loaded.as_ref()).map(|l| &l.manifest.marks);
        let door = marks
            .and_then(|m| m.iter().find(|(k, _)| k.starts_with("DOOR_")))
            .map(|(_, v)| Vec3::from_array(*v))
            .unwrap_or(Vec3::new(-(spec.fuselage_m * 0.5 + 1.2), -rest, -spec.length_m * 0.2));
        let eye = marks
            .and_then(|m| m.get("EYE_pilot"))
            .map(|v| Vec3::from_array(*v))
            .unwrap_or(Vec3::new(-spec.fuselage_m * 0.2, spec.fuselage_m * 0.25, -spec.length_m * 0.35));
        let rot = glam::Quat::from_rotation_y(self.heading_rad);
        // the door's foot on the ground, a little out from the hull so the pilot stands clear of it
        let mut door_w = self.pos + rot * door;
        door_w.y = self.pos.y - rest;
        let side = rot * Vec3::new(door.x.signum(), 0.0, 0.0);
        (door_w + side * 0.8, self.pos + rot * eye)
    }

    /// Its position (x, z).
    pub fn xz(&self) -> Vec2 {
        Vec2::new(self.pos.x, self.pos.z)
    }

    /// Its heading as a unit vector (x, z).
    pub fn fwd2(&self) -> Vec2 {
        let f = forward(self.heading_rad);
        Vec2::new(f.x, f.z)
    }
}

/// The radius it turns at full rate at `speed_m_s`, metres.
fn turn_radius(speed_m_s: f32, spec: &AircraftSpec) -> f32 {
    speed_m_s / spec.turn_deg_s.to_radians()
}

/// How far out along the centre line the approach begins: the final approach fix, plus two turns'
/// radius at cruise to line up in before it.
fn approach_m(r: &RouteConfig, spec: &AircraftSpec) -> f32 {
    r.final_m + 2.0 * turn_radius(spec.cruise_m_s, spec)
}

/// The way to fly to join and hold the strip's extended centre line from `pos`: along it, turned back
/// toward it by the offset over `lead_m` (a localizer), at most about 56 degrees off it.
fn localizer(strip: &Strip, pos: Vec2, lead_m: f32) -> Vec2 {
    let d = strip.dir();
    let n = d.perp();
    let off = (pos - strip.start).dot(n);
    (d - n * (off / lead_m.max(1.0)).clamp(-1.5, 1.5)).normalize_or(d)
}

/// The highest ground on the straight way from `from` to `to` plus `clear_m`, and how far along it is.
fn route_floor(air: &Air, from: Vec2, to: Vec2, clear_m: f32) -> (f32, f32) {
    let len = from.distance(to);
    let n = (len / ROUTE_LOOK_STEP_M).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let p = from.lerp(to, t);
            ((air.ground)(p.x, p.y) + clear_m, t * len)
        })
        .fold((f32::NEG_INFINITY, 0.0), |a, b| if b.0 > a.0 { b } else { a })
}

fn ease(v: f32, to: f32, step: f32) -> f32 {
    v + (to - v).clamp(-step, step)
}
