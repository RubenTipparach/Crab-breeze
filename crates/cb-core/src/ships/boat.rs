//! A boat (openspec change `harbours`, design sections 3 and 5): what it is doing, how it steers (pure pursuit
//! along its route, clear of the shallows and of the boat ahead, slowing for its berth), and what the fight does
//! to it (hits by the vehicles' armour rule, sinking to a wreck, fleeing, being shoved or swamped by a mech).
//! Pure: a boat steps from its state, its kind's data and what the caller tells it about the water and the boats
//! round it.

use super::lanes::{End, Route};
use super::{BoatSpec, FightConfig, Sea, SteerConfig, TrafficConfig, ahead, heading_of, wrap};
use crate::units::armour_hit;
use glam::Vec2;

/// A boat sounds the water ahead this often, seconds (each sounding is a few ground samples).
pub const PROBE_EVERY_S: f32 = 0.5;

/// The helm between soundings: what the last sounding of the water ahead decided.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Helm {
    /// Seconds to the next sounding.
    pub next_s: f32,
    /// Turn off the wanted heading toward deeper water, radians.
    pub deflect_rad: f32,
    /// Slow to half for the shoal ahead.
    pub slow: bool,
    /// Shoal water close ahead: no way on this heading.
    pub held: bool,
    /// The heading it was held on.
    pub held_heading: f32,
}

/// What a boat is about.
#[derive(Clone, Debug, PartialEq)]
pub enum Errand {
    /// Running route `route` (an index into the routes the caller passes), `s_m` along it, toward its last point
    /// when `forward`.
    Route {
        /// The route.
        route: u32,
        /// How far along it, metres.
        s_m: f32,
        /// Toward its last point.
        forward: bool,
    },
    /// Rowing or motoring round a loop of points on a lake, steering for `points[next]`.
    Loop {
        /// The loop's points (x, z).
        points: Vec<Vec2>,
        /// The point it steers for.
        next: usize,
    },
    /// Riding at anchor at `at` (it sails back there after fleeing).
    Anchor {
        /// The anchor spot (x, z).
        at: Vec2,
    },
}

/// Where a boat is in its day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Sailing its errand.
    Underway,
    /// Alongside a berth or landing for `left_s` more.
    Moored {
        /// Seconds left.
        left_s: f32,
    },
    /// A fishing boat working its ground in a circle for `left_s` more, at `angle_rad` round it.
    Working {
        /// Seconds left.
        left_s: f32,
        /// Where round the ground's circle it is, radians.
        angle_rad: f32,
    },
    /// Riding at anchor.
    Anchored,
    /// Running from `from` for `left_s` more.
    Fleeing {
        /// Seconds left.
        left_s: f32,
        /// What it runs from (x, z).
        from: Vec2,
    },
    /// Going down, `t_s` since it was destroyed.
    Sinking {
        /// Seconds since destroyed.
        t_s: f32,
    },
    /// A wreck on the bottom, `t_s` since it got there.
    Sunk {
        /// Seconds on the bottom.
        t_s: f32,
    },
    /// Over the horizon at the offing: out of the world.
    Gone,
}

/// A boat.
#[derive(Clone, Debug, PartialEq)]
pub struct Boat {
    /// Stable id.
    pub id: u32,
    /// Its kind's name in `ShipsConfig::kinds`.
    pub kind: String,
    /// What it is about.
    pub errand: Errand,
    /// Where it is (x, z): the middle of its waterline.
    pub pos: Vec2,
    /// Facing (`ships::ahead`).
    pub heading_rad: f32,
    /// Speed through the water, metres a second.
    pub speed_m_s: f32,
    /// A shove's velocity, dying away, metres a second (x, z).
    pub push: Vec2,
    /// Hit points left.
    pub hp: f32,
    /// Where it is in its day.
    pub phase: Phase,
    /// Civil crew aboard.
    pub crew: u32,
    /// Seconds it moors for at a berth or works its ground.
    pub dwell_s: f32,
    /// The pilot has harmed it (reported once, `harm`).
    pub harmed_by_pilot: bool,
    /// Its sinking by the pilot has been reported.
    pub reported_sunk: bool,
    /// The job it is the ward of (an escort's freighter), if any.
    pub ward: Option<u32>,
    /// The helm between soundings.
    pub helm: Helm,
}

/// Another boat, as a boat steering sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neighbour {
    /// Its id.
    pub id: u32,
    /// Where it is (x, z).
    pub pos: Vec2,
    /// Its velocity (x, z).
    pub vel: Vec2,
    /// Its length, metres.
    pub length_m: f32,
}

/// What a boat needs to know to step.
pub struct BoatView<'a> {
    /// Its kind.
    pub spec: &'a BoatSpec,
    /// The water.
    pub sea: &'a Sea<'a>,
    /// Every route, by index.
    pub routes: &'a [Route],
    /// The boats round it (itself may be among them: it is skipped by id).
    pub others: &'a [Neighbour],
    /// Steering.
    pub steer: &'a SteerConfig,
    /// The timetable's numbers (a fishing boat's work).
    pub traffic: &'a TrafficConfig,
    /// Ships keep this far right of a lane's line, metres.
    pub keep_right_m: f32,
    /// A boat slows from this far out from its berth, metres.
    pub approach_m: f32,
}

/// What happened to a boat this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoatStep {
    /// It came alongside its berth or landing.
    pub moored: bool,
    /// It reached the offing and left the world.
    pub gone: bool,
    /// It reached the bottom.
    pub sank: bool,
}

impl Boat {
    /// A boat of `spec` on `errand` at `pos`, facing `heading_rad`, under way at its cruising speed.
    pub fn new(id: u32, kind: &str, spec: &BoatSpec, errand: Errand, pos: Vec2, heading_rad: f32) -> Self {
        Self {
            id,
            kind: kind.into(),
            errand,
            pos,
            heading_rad,
            speed_m_s: spec.speed_m_s,
            push: Vec2::ZERO,
            hp: spec.hp,
            phase: Phase::Underway,
            crew: spec.crew,
            dwell_s: 180.0,
            harmed_by_pilot: false,
            reported_sunk: false,
            ward: None,
            helm: Helm {
                next_s: (id % 5) as f32 * PROBE_EVERY_S / 5.0,
                ..Helm::default()
            },
        }
    }

    /// Afloat: not sinking, sunk or gone (a target, and in the way).
    pub fn afloat(&self) -> bool {
        !matches!(self.phase, Phase::Sinking { .. } | Phase::Sunk { .. } | Phase::Gone)
    }

    /// Crewed now: afloat with civil crew aboard (a wreck is not).
    pub fn crewed(&self) -> bool {
        self.afloat() && self.crew > 0
    }

    /// Its velocity (x, z).
    pub fn vel(&self) -> Vec2 {
        ahead(self.heading_rad) * self.speed_m_s + self.push
    }

    /// Its hull as a segment (bow and stern centres, a beam's half in from each end) and its radius.
    pub fn hull(&self, spec: &BoatSpec) -> (Vec2, Vec2, f32) {
        let half = (spec.length_m() - spec.beam_m()).max(0.0) * 0.5;
        let fwd = ahead(self.heading_rad);
        (self.pos + fwd * half, self.pos - fwd * half, spec.beam_m() * 0.5)
    }

    /// Takes a hit by the vehicles' rule (`units::armour_hit`); true when it destroyed the boat, which starts to
    /// sink.
    pub fn hit(&mut self, penetration_mm: f32, damage: f32, spec: &BoatSpec) -> bool {
        if !self.afloat() {
            return false;
        }
        if armour_hit(&mut self.hp, penetration_mm, damage, spec.armour_mm) {
            self.phase = Phase::Sinking { t_s: 0.0 };
            return true;
        }
        false
    }

    /// Swamped or holed past saving: it sinks now. False when it was not afloat.
    pub fn founder(&mut self) -> bool {
        if !self.afloat() {
            return false;
        }
        self.hp = 0.0;
        self.phase = Phase::Sinking { t_s: 0.0 };
        true
    }

    /// Something to run from at `from`: a boat under way, working or at anchor flees for `flee_s`; a moored one
    /// stays moored; a sinking one has other cares.
    pub fn alarm(&mut self, from: Vec2, flee_s: f32) {
        if matches!(
            self.phase,
            Phase::Underway | Phase::Working { .. } | Phase::Anchored | Phase::Fleeing { .. }
        ) {
            self.phase = Phase::Fleeing { left_s: flee_s, from };
        }
    }

    /// A mech's body (a circle of `radius_m` at `at`, moving at `vel`) against this boat: `None` when they do not
    /// touch or the boat is too big to shove; otherwise the boat is pushed out of the way, and `Some(true)` when the
    /// two met faster than `swamp_m_s` and the boat is swamped (it sinks).
    pub fn shove(&mut self, spec: &BoatSpec, at: Vec2, vel: Vec2, radius_m: f32, fight: &FightConfig) -> Option<bool> {
        if !self.afloat() || spec.length_m() > fight.shove_max_m {
            return None;
        }
        let (a, b, r) = self.hull(spec);
        let ab = b - a;
        let u = if ab.length_squared() > 0.0 {
            ((at - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let near = a + ab * u;
        let gap = near.distance(at) - radius_m - r;
        if gap >= 0.0 {
            return None;
        }
        let n = (near - at).normalize_or(ahead(self.heading_rad).perp());
        let closing = (vel - self.vel()).dot(n);
        if closing > fight.swamp_m_s {
            self.founder();
            return Some(true);
        }
        self.pos += n * -gap;
        self.push = n * vel.dot(n).max(0.5);
        self.speed_m_s *= 0.5;
        Some(false)
    }

    /// One fixed step of `dt_s`.
    pub fn step(&mut self, view: &BoatView, dt_s: f32) -> BoatStep {
        let mut out = BoatStep::default();
        let spec = view.spec;
        match self.phase {
            Phase::Gone => return out,
            Phase::Sunk { t_s } => {
                self.phase = Phase::Sunk { t_s: t_s + dt_s };
                return out;
            }
            Phase::Sinking { t_s } => {
                let t = t_s + dt_s;
                self.speed_m_s = (self.speed_m_s - spec.accel_m_s2 * 3.0 * dt_s).max(0.0);
                self.pos += ahead(self.heading_rad) * self.speed_m_s * dt_s;
                if t >= spec.sink_s {
                    self.phase = Phase::Sunk { t_s: 0.0 };
                    out.sank = true;
                } else {
                    self.phase = Phase::Sinking { t_s: t };
                }
                return out;
            }
            Phase::Moored { left_s } => {
                self.speed_m_s = 0.0;
                if left_s - dt_s > 0.0 {
                    self.phase = Phase::Moored { left_s: left_s - dt_s };
                    return out;
                }
                // Cast off: back along the route.
                if let Errand::Route { forward, .. } = &mut self.errand {
                    *forward = !*forward;
                }
                self.phase = Phase::Underway;
                return out;
            }
            Phase::Anchored => {
                self.speed_m_s = 0.0;
                self.drift(dt_s);
                return out;
            }
            _ => {}
        }
        let (want, mut speed, near_end) = match self.phase {
            Phase::Fleeing { left_s, from } => {
                if left_s - dt_s <= 0.0 {
                    self.rejoin(view);
                } else {
                    self.phase = Phase::Fleeing {
                        left_s: left_s - dt_s,
                        from,
                    };
                }
                let away = (self.pos - from).normalize_or(ahead(self.heading_rad));
                (self.pos + away * 100.0, spec.speed_m_s * view.steer.flee_speed_share, false)
            }
            Phase::Working { left_s, angle_rad } => {
                let r = view.traffic.work_radius_m;
                let ground = self.route_end(view).unwrap_or(self.pos);
                let a = angle_rad + view.traffic.work_m_s / r * dt_s;
                if left_s - dt_s <= 0.0 {
                    if let Errand::Route { forward, .. } = &mut self.errand {
                        *forward = !*forward;
                    }
                    self.phase = Phase::Underway;
                } else {
                    self.phase = Phase::Working {
                        left_s: left_s - dt_s,
                        angle_rad: a,
                    };
                }
                (ground + Vec2::new(a.cos(), a.sin()) * r, view.traffic.work_m_s, false)
            }
            _ => match self.errand_target(view, &mut out) {
                Some(t) => t,
                None => return out,
            },
        };
        // Keep clear of the shallows ahead (not on the last run in to a berth, which is dredged): sounded every
        // `PROBE_EVERY_S`, the helm held between soundings.
        let mut want_heading = heading_of(want - self.pos);
        self.helm.next_s -= dt_s;
        if self.helm.next_s <= 0.0 {
            self.helm = self.sound(view, want_heading, near_end);
        }
        want_heading += self.helm.deflect_rad;
        if self.helm.slow {
            speed *= 0.5;
        }
        // Keep clear of the boat ahead: match its speed, and stop short of it unless this boat has way (lower id).
        let fwd = ahead(self.heading_rad);
        for o in view.others.iter().filter(|o| o.id != self.id) {
            let rel = o.pos - self.pos;
            let d = rel.length();
            if d > spec.length_m() * view.steer.gap_lengths || d < 1e-3 {
                continue;
            }
            if fwd.dot(rel / d) < 0.866 {
                continue;
            }
            speed = speed.min(o.vel.dot(fwd).max(0.0));
            if d < (spec.length_m() + o.length_m) * 0.5 + 5.0 && self.id > o.id {
                speed = 0.0;
            }
        }
        let grip = (self.speed_m_s / (spec.speed_m_s * 0.25)).clamp(0.3, 1.0);
        let turn = spec.turn_deg_s.to_radians() * dt_s * grip;
        self.heading_rad += wrap(want_heading - self.heading_rad).clamp(-turn, turn);
        let rate = if speed < self.speed_m_s {
            spec.accel_m_s2 * 2.0
        } else {
            spec.accel_m_s2
        };
        self.speed_m_s += (speed - self.speed_m_s).clamp(-rate * dt_s, rate * dt_s);
        // Never on to ground shallower than its keel: held where the last sounding found shoal water close ahead.
        if self.helm.held && wrap(self.heading_rad - self.helm.held_heading).abs() < 0.5 {
            self.speed_m_s = 0.0;
        } else {
            self.pos += ahead(self.heading_rad) * self.speed_m_s * dt_s;
        }
        self.drift(dt_s);
        out
    }

    /// Sounds the water ahead: whether to turn off toward deeper water, slow, or hold where it is.
    fn sound(&self, view: &BoatView, want_heading: f32, near_end: bool) -> Helm {
        let spec = view.spec;
        let mut helm = Helm {
            next_s: PROBE_EVERY_S,
            ..Helm::default()
        };
        let depth = |p: Vec2| view.sea.depth(p);
        let here = depth(self.pos);
        // Close ahead: shoal water inside two soundings' travel holds it (on the dredged run in, half its keel).
        let floor = if near_end { spec.draft_m * 0.5 } else { spec.draft_m + 0.5 };
        let close = (self.speed_m_s * PROBE_EVERY_S * 2.0).max(spec.beam_m());
        let there = depth(self.pos + ahead(self.heading_rad) * (close + spec.length_m() * 0.5));
        if there < floor && there < here {
            helm.held = true;
            helm.held_heading = self.heading_rad;
        }
        if near_end {
            return helm;
        }
        let need = spec.draft_m + view.steer.keel_clear_m;
        let reach = (self.speed_m_s.max(spec.speed_m_s * 0.3) * view.steer.probe_s).max(spec.length_m() * 3.0);
        let probe = |h: f32| depth(self.pos + ahead(h) * reach);
        if probe(want_heading) < need {
            let left = probe(want_heading + 0.52);
            let right = probe(want_heading - 0.52);
            // Toward the deeper side; hard over when both sides are shallow too.
            let hard = if left.max(right) < need { 1.6 } else { 0.8 };
            helm.deflect_rad = if left >= right { hard } else { -hard };
            helm.slow = true;
        }
        helm
    }

    /// A shove's push carries it and dies away.
    fn drift(&mut self, dt_s: f32) {
        self.pos += self.push * dt_s;
        self.push *= 1.0 - (1.5 * dt_s).min(1.0);
    }

    /// The point at the end it is running toward, for a route errand.
    fn route_end(&self, view: &BoatView) -> Option<Vec2> {
        let Errand::Route { route, forward, .. } = &self.errand else {
            return None;
        };
        let r = view.routes.get(*route as usize)?;
        Some(if *forward { *r.points.last()? } else { r.points[0] })
    }

    /// After fleeing: back onto its route at the point nearest it.
    fn rejoin(&mut self, view: &BoatView) {
        if let Errand::Route { route, s_m, .. } = &mut self.errand
            && let Some(r) = view.routes.get(*route as usize)
        {
            *s_m = r.project(self.pos, 0.0, f32::INFINITY).0;
        }
        self.phase = Phase::Underway;
    }

    /// Where the errand steers it, the speed it wants and whether it is on its last run in to a berth; `None`
    /// once it has moored, gone or anchored this step.
    fn errand_target(&mut self, view: &BoatView, out: &mut BoatStep) -> Option<(Vec2, f32, bool)> {
        let spec = view.spec;
        match &mut self.errand {
            Errand::Anchor { at } => {
                if self.pos.distance(*at) < spec.length_m().max(30.0) {
                    self.phase = Phase::Anchored;
                    self.speed_m_s = 0.0;
                    return None;
                }
                Some((*at, spec.speed_m_s * 0.5, false))
            }
            Errand::Loop { points, next } => {
                if points.is_empty() {
                    return Some((self.pos, 0.0, false));
                }
                let p = points[*next % points.len()];
                if self.pos.distance(p) < (spec.length_m() * 2.0).max(10.0) {
                    *next = (*next + 1) % points.len();
                }
                Some((points[*next % points.len()], spec.speed_m_s, false))
            }
            Errand::Route { route, s_m, forward } => {
                let r = view.routes.get(*route as usize)?;
                let len = r.length_m();
                *s_m = r.project(self.pos, *s_m, 400.0).0;
                let remaining = if *forward { len - *s_m } else { *s_m };
                let end = r.ends[usize::from(*forward)];
                let lead = spec.length_m() + view.steer.lookahead_m;
                let s_t = if *forward { *s_m + lead } else { *s_m - lead };
                let (p, dir) = r.at(s_t);
                let travel = if *forward { dir } else { -dir };
                let from_start = if *forward { *s_m } else { len - *s_m };
                let fade = (remaining / view.approach_m).min(from_start / view.approach_m).clamp(0.0, 1.0);
                let target = p + travel.perp() * view.keep_right_m * fade;
                let berthing = matches!(end, End::Berth(_) | End::Landing);
                let mut speed = spec.speed_m_s;
                if berthing && remaining < view.approach_m {
                    let k = (remaining / view.approach_m).clamp(0.0, 1.0);
                    speed = view.steer.berth_m_s + (spec.speed_m_s - view.steer.berth_m_s) * k;
                }
                let alongside = spec.beam_m() * 0.5 + 2.0 + spec.length_m() * 0.1;
                match end {
                    End::Berth(_) | End::Landing if remaining <= alongside => {
                        // Alongside: lying along the berth face, its stand-off from it.
                        let (end_p, end_dir) = r.at(if *forward { len } else { 0.0 });
                        let in_dir = if *forward { end_dir } else { -end_dir };
                        self.pos = end_p - in_dir * (spec.beam_m() * 0.5 + 2.0);
                        self.heading_rad = heading_of(in_dir.perp());
                        self.speed_m_s = 0.0;
                        self.phase = Phase::Moored { left_s: self.dwell_s };
                        out.moored = true;
                        None
                    }
                    End::Offing if remaining < 50.0 => {
                        self.phase = Phase::Gone;
                        out.gone = true;
                        None
                    }
                    End::Ground if remaining < 30.0 => {
                        let ground = if *forward { *r.points.last()? } else { r.points[0] };
                        let d = self.pos - ground;
                        self.phase = Phase::Working {
                            left_s: self.dwell_s,
                            angle_rad: d.y.atan2(d.x),
                        };
                        Some((ground, view.traffic.work_m_s, false))
                    }
                    _ => Some((target, speed, berthing && remaining < view.approach_m)),
                }
            }
        }
    }

    /// Where it sits now: its waterline's height and its pitch (bow up) and roll (starboard up), radians, from the
    /// water's `surface_m` and the `bed_m` under it, the swell's `ride`, and how far it lists when sinking.
    pub fn pose(&self, spec: &BoatSpec, surface_m: f32, bed_m: f32, ride: super::float::Ride, list_deg: f32) -> (f32, f32, f32) {
        let wreck_y = (bed_m + spec.draft_m).min(surface_m - 0.3);
        let list = list_deg.to_radians();
        match self.phase {
            Phase::Sinking { t_s } => {
                let f = (t_s / spec.sink_s).clamp(0.0, 1.0);
                let y = surface_m + ride.heave_m * (1.0 - f) + (wreck_y - surface_m) * f.powf(1.5);
                (y, ride.pitch_rad * (1.0 - f) - 0.15 * f, ride.roll_rad * (1.0 - f) + list * f)
            }
            Phase::Sunk { .. } => (wreck_y, -0.15, list),
            _ => (surface_m + ride.heave_m, ride.pitch_rad, ride.roll_rad),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_world::{sea, shipped};
    use super::super::{FISHING, FREIGHTER, ROWING, ShipsConfig};
    use super::*;

    const DT: f32 = 0.1;

    struct Bench {
        cfg: ShipsConfig,
        routes: Vec<Route>,
    }

    impl Bench {
        fn new(routes: Vec<Route>) -> Self {
            Self { cfg: shipped(), routes }
        }

        fn run(&self, b: &mut Boat, others: &[Neighbour], seconds: f32) -> Vec<BoatStep> {
            let s = sea();
            let spec = &self.cfg.kinds[&b.kind];
            let view = BoatView {
                spec,
                sea: &s,
                routes: &self.routes,
                others,
                steer: &self.cfg.steer,
                traffic: &self.cfg.traffic,
                keep_right_m: self.cfg.lanes.keep_right_m,
                approach_m: self.cfg.harbours.approach_m,
            };
            (0..(seconds / DT) as usize).map(|_| b.step(&view, DT)).collect()
        }
    }

    /// A lane due north along x = 6 km, a berth at its far end.
    fn north() -> Route {
        Route::new(
            vec![Vec2::new(6000.0, 6000.0), Vec2::new(6000.0, -6000.0)],
            [End::Offing, End::Berth(0)],
        )
    }

    fn freighter(b: &Bench, at: Vec2, s: f32) -> Boat {
        let spec = &b.cfg.kinds[FREIGHTER];
        Boat::new(
            1,
            FREIGHTER,
            spec,
            Errand::Route {
                route: 0,
                s_m: s,
                forward: true,
            },
            at,
            0.0,
        )
    }

    #[test]
    fn a_ship_follows_its_lane_keeping_right() {
        let bench = Bench::new(vec![north()]);
        let mut b = freighter(&bench, Vec2::new(6000.0, 5000.0), 1000.0);
        bench.run(&mut b, &[], 400.0);
        // Running north (-z), the right is east (+x): it keeps 80 m right of the line.
        assert!((b.pos.x - 6080.0).abs() < 15.0, "{:?}", b.pos);
        assert!(b.pos.y < 3000.0, "made way: {:?}", b.pos);
    }

    #[test]
    fn a_ship_slows_for_its_berth_and_moors_alongside() {
        let bench = Bench::new(vec![north()]);
        let mut b = freighter(&bench, Vec2::new(6000.0, -3000.0), 9000.0);
        let mut slowest_near = f32::MAX;
        let mut moored = false;
        for _ in 0..12_000 {
            let st = bench.run(&mut b, &[], DT);
            moored |= st.iter().any(|s| s.moored);
            if b.pos.y < -5500.0 && b.afloat() && !moored {
                slowest_near = slowest_near.min(b.speed_m_s);
            }
            if moored {
                break;
            }
        }
        assert!(moored, "{:?} {:?}", b.pos, b.phase);
        assert!(slowest_near < 2.0, "slow in to the berth: {slowest_near}");
        assert!(matches!(b.phase, Phase::Moored { .. }));
        // Alongside: lying across the lane's last leg, its beam off the berth face.
        assert!(b.pos.distance(Vec2::new(6000.0, -6000.0)) < 20.0, "{:?}", b.pos);
        assert!(ahead(b.heading_rad).y.abs() < 0.01, "{}", b.heading_rad);
    }

    #[test]
    fn a_moored_ship_casts_off_back_along_its_route() {
        let bench = Bench::new(vec![north()]);
        let mut b = freighter(&bench, Vec2::new(6000.0, -6000.0), 12000.0);
        b.phase = Phase::Moored { left_s: 1.0 };
        bench.run(&mut b, &[], 200.0);
        assert!(matches!(b.errand, Errand::Route { forward: false, .. }));
        assert!(b.pos.y > -5900.0, "under way again: {:?}", b.pos);
    }

    #[test]
    fn a_ship_steers_off_the_shallows_ahead() {
        // A lane straight at the island: the ship turns aside rather than run aground.
        let r = Route::new(vec![Vec2::new(9000.0, 0.0), Vec2::new(-9000.0, 0.0)], [End::Offing, End::Offing]);
        let bench = Bench::new(vec![r]);
        let mut b = freighter(&bench, Vec2::new(7000.0, 0.0), 2000.0);
        b.heading_rad = heading_of(Vec2::new(-1.0, 0.0));
        let need = bench.cfg.kinds[FREIGHTER].draft_m;
        for _ in 0..600 {
            bench.run(&mut b, &[], 1.0);
            assert!(sea().depth(b.pos) >= need, "aground at {:?}", b.pos);
        }
    }

    #[test]
    fn a_ship_slows_behind_the_one_ahead_and_never_runs_into_it() {
        let bench = Bench::new(vec![north()]);
        let mut b = freighter(&bench, Vec2::new(6080.0, 0.0), 6000.0);
        b.id = 2;
        // A stopped ship 700 m ahead on the same lane (a freighter needs 245 m to stop from its cruise).
        let ahead_of = Neighbour {
            id: 1,
            pos: Vec2::new(6080.0, -700.0),
            vel: Vec2::ZERO,
            length_m: 120.0,
        };
        bench.run(&mut b, &[ahead_of], 300.0);
        assert!(b.pos.distance(ahead_of.pos) > 120.0, "{:?}", b.pos);
        assert!(b.speed_m_s < 0.5, "{}", b.speed_m_s);
    }

    #[test]
    fn a_fishing_boat_works_its_ground_and_turns_for_home() {
        let r = Route::new(vec![Vec2::new(5000.0, 0.0), Vec2::new(8000.0, 0.0)], [End::Berth(0), End::Ground]);
        let bench = Bench::new(vec![r]);
        let spec = &bench.cfg.kinds[FISHING];
        let mut b = Boat::new(
            3,
            FISHING,
            spec,
            Errand::Route {
                route: 0,
                s_m: 0.0,
                forward: true,
            },
            Vec2::new(5000.0, 0.0),
            heading_of(Vec2::X),
        );
        b.dwell_s = 100.0;
        bench.run(&mut b, &[], 900.0);
        let worked = matches!(b.phase, Phase::Working { .. }) || matches!(b.errand, Errand::Route { forward: false, .. });
        assert!(worked, "{:?} {:?}", b.phase, b.pos);
        bench.run(&mut b, &[], 200.0);
        assert!(matches!(b.errand, Errand::Route { forward: false, .. }), "turned for home");
    }

    #[test]
    fn a_rowing_boat_keeps_to_its_lake() {
        let bench = Bench::new(vec![]);
        let spec = &bench.cfg.kinds[ROWING];
        let c = Vec2::new(1000.0, 0.0);
        let points: Vec<Vec2> = (0..5)
            .map(|i| {
                let a = i as f32 / 5.0 * std::f32::consts::TAU;
                c + Vec2::new(a.cos(), a.sin()) * 180.0
            })
            .collect();
        let mut b = Boat::new(4, ROWING, spec, Errand::Loop { points, next: 0 }, c, 0.0);
        for _ in 0..600 {
            bench.run(&mut b, &[], 1.0);
            assert!(sea().depth(b.pos) >= spec.draft_m, "{:?}", b.pos);
            assert!(b.pos.distance(c) < 300.0);
        }
        assert!(b.pos.distance(c) > 50.0, "it rows its loop");
    }

    #[test]
    fn a_hit_that_does_not_beat_the_armour_does_nothing_and_one_that_kills_sinks_it() {
        let cfg = shipped();
        let spec = &cfg.kinds[FREIGHTER];
        let mut b = Boat::new(1, FREIGHTER, spec, Errand::Anchor { at: Vec2::ZERO }, Vec2::ZERO, 0.0);
        assert!(!b.hit(spec.armour_mm - 1.0, 1e6, spec));
        assert_eq!(b.hp, spec.hp);
        assert!(b.hit(spec.armour_mm, spec.hp, spec));
        assert!(matches!(b.phase, Phase::Sinking { .. }));
        assert!(!b.crewed() && !b.afloat());
    }

    #[test]
    fn a_sunk_boat_goes_down_over_its_sinking_time_to_a_wreck_on_the_bottom() {
        let bench = Bench::new(vec![]);
        let spec = &bench.cfg.kinds[FREIGHTER];
        let mut b = Boat::new(1, FREIGHTER, spec, Errand::Anchor { at: Vec2::ZERO }, Vec2::new(5000.0, 0.0), 0.0);
        b.founder();
        let ride = super::super::float::Ride::default();
        let list = bench.cfg.fight.list_deg;
        let start = b.pose(spec, 0.0, -60.0, ride, list).0;
        let steps = bench.run(&mut b, &[], spec.sink_s * 0.5);
        assert!(steps.iter().all(|s| !s.sank));
        let half = b.pose(spec, 0.0, -60.0, ride, list);
        assert!(half.0 < start && half.2 > 0.0, "settling and listing: {half:?}");
        let steps = bench.run(&mut b, &[], spec.sink_s);
        assert!(steps.iter().any(|s| s.sank));
        assert!(matches!(b.phase, Phase::Sunk { .. }));
        let (y, _, roll) = b.pose(spec, 0.0, -60.0, ride, list);
        assert!((y - (-60.0 + spec.draft_m)).abs() < 1e-4, "keel on the bed: {y}");
        assert!((roll - list.to_radians()).abs() < 1e-5);
        // In shallow water the wreck shows above the surface.
        let (y, ..) = b.pose(spec, 0.0, -3.0, ride, list);
        assert!(y + spec.size_m[1] - spec.draft_m > 0.0, "masts out of the water");
    }

    #[test]
    fn a_blast_sets_a_boat_running_away_and_it_rejoins_its_lane_after() {
        let bench = Bench::new(vec![north()]);
        let spec = &bench.cfg.kinds[FISHING];
        let errand = Errand::Route {
            route: 0,
            s_m: 6000.0,
            forward: true,
        };
        let mut b = Boat::new(1, FISHING, spec, errand, Vec2::new(6080.0, 0.0), 0.0);
        let blast = Vec2::new(6080.0, -400.0);
        b.alarm(blast, bench.cfg.fight.flee_s);
        bench.run(&mut b, &[], 50.0);
        assert!(b.pos.distance(blast) > 500.0, "ran from the blast to the south: {:?}", b.pos);
        assert!(matches!(b.phase, Phase::Fleeing { .. }));
        bench.run(&mut b, &[], 20.0);
        assert_eq!(b.phase, Phase::Underway, "back on its lane");
    }

    #[test]
    fn a_moored_boat_stays_moored_when_alarmed() {
        let cfg = shipped();
        let spec = &cfg.kinds[FREIGHTER];
        let mut b = Boat::new(1, FREIGHTER, spec, Errand::Anchor { at: Vec2::ZERO }, Vec2::ZERO, 0.0);
        b.phase = Phase::Moored { left_s: 10.0 };
        b.alarm(Vec2::X, 60.0);
        assert!(matches!(b.phase, Phase::Moored { .. }));
    }

    #[test]
    fn a_mech_shoves_a_rowing_boat_aside_and_swamps_one_it_runs_into() {
        let cfg = shipped();
        let spec = &cfg.kinds[ROWING];
        let mut b = Boat::new(1, ROWING, spec, Errand::Anchor { at: Vec2::ZERO }, Vec2::ZERO, 0.0);
        b.speed_m_s = 0.0;
        // A wading mech, 4 m of body, coming at it slowly from the east.
        let shoved = b.shove(spec, Vec2::new(4.0, 0.0), Vec2::new(-2.0, 0.0), 4.0, &cfg.fight);
        assert_eq!(shoved, Some(false));
        assert!(b.pos.x < 0.0 && b.push.x < 0.0, "pushed west: {:?}", b.pos);
        assert!(b.afloat());
        // Skimming into it at 12 m/s.
        let swamped = b.shove(spec, b.pos + Vec2::new(4.0, 0.0), Vec2::new(-12.0, 0.0), 4.0, &cfg.fight);
        assert_eq!(swamped, Some(true));
        assert!(!b.afloat());
    }

    #[test]
    fn a_mech_cannot_shove_a_freighter() {
        let cfg = shipped();
        let spec = &cfg.kinds[FREIGHTER];
        let mut b = Boat::new(1, FREIGHTER, spec, Errand::Anchor { at: Vec2::ZERO }, Vec2::ZERO, 0.0);
        assert_eq!(b.shove(spec, Vec2::new(5.0, 0.0), Vec2::new(-20.0, 0.0), 4.0, &cfg.fight), None);
    }
}
