//! The boats round the pilot (openspec change `harbours`, design sections 3 and 8): every boat the island has is a
//! slot in a seeded timetable (ships on the sea lanes, fishing boats, ships at anchor, lake boats, the ferry), whose
//! place at any time is fixed by the seed. A slot becomes a real boat, stepped and fought, when its place comes
//! within `spawn_m` of the pilot, and stops being one past `despawn_m` unless it is sinking, a wreck or a job's
//! ward. A slot whose boat sank stays empty. Pure: the same plan, seed and pilot path give the same boats.

use super::boat::{Boat, BoatStep, BoatView, Errand, Neighbour, Phase};
use super::harbour::{self, Harbour};
use super::lanes::{self, End, Route, SeaGrid};
use super::{FERRY, FISHING, FREIGHTER, LAUNCH, ROWING, Sea, ShipsConfig, TANKER, heading_of};
use crate::jobs::SeaSite;
use crate::rng::SimRng;
use crate::world::small_buildings::Kits;
use crate::world::{Lake, WorldPlan};
use glam::Vec2;
use std::collections::BTreeMap;

/// What a slot's boat does, by the timetable.
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    /// Shuttles a sea lane end to end and back for ever, `offset_m` round the out-and-back loop at time zero.
    Lane {
        /// The route.
        route: u32,
        /// Where round the loop at time zero, metres.
        offset_m: f32,
    },
    /// Out to its ground, works it, home, moors; `phase_s` into that cycle at time zero.
    Fishing {
        /// The route, berth to ground.
        route: u32,
        /// Seconds into the cycle at time zero.
        phase_s: f32,
    },
    /// Rides at anchor.
    Anchor {
        /// The spot (x, z).
        at: Vec2,
        /// Its heading, swung to the shore.
        heading_rad: f32,
    },
    /// Rows or motors a loop on a lake, `phase_m` round it at time zero.
    Loop {
        /// The loop's points (x, z).
        points: Vec<Vec2>,
        /// Where round it at time zero, metres.
        phase_m: f32,
    },
    /// The ferry: out, moor, back, moor; `phase_s` into that cycle at time zero.
    Ferry {
        /// The route, harbour to landing.
        route: u32,
        /// Seconds into the cycle at time zero.
        phase_s: f32,
    },
}

/// A timetabled boat.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    /// Its kind.
    pub kind: String,
    /// What it does.
    pub plan: Plan,
    /// It moors or works this long, seconds.
    pub dwell_s: f32,
    /// On a lake (made real nearer the pilot).
    pub lake: bool,
    /// Its boat sank: the slot stays empty.
    pub sunk: bool,
    /// The real boat's id, while it is real.
    pub live: Option<u32>,
}

/// Where a slot's boat is by the timetable: its place, heading, errand and phase.
struct Placed {
    pos: Vec2,
    heading_rad: f32,
    errand: Errand,
    phase: Phase,
}

/// Every boat on the island: the harbours, the routes, the timetable and the boats real now.
#[derive(Clone, Debug)]
pub struct Traffic {
    /// The harbours.
    pub harbours: Vec<Harbour>,
    /// Every route: the sea lanes first, then the fishing runs, then the ferry's.
    pub routes: Vec<Route>,
    /// How many of `routes` are sea lanes.
    pub lanes: usize,
    /// The timetable.
    pub slots: Vec<Slot>,
    /// The boats real now, in id order.
    pub boats: Vec<Boat>,
    /// Each harbour's first fishing ground (x, z).
    pub grounds: Vec<Vec2>,
    /// The ferry's islet, if the world has a ferry: its centre (x, z) and radius.
    pub islet: Option<(Vec2, f32)>,
    /// Seconds since the world began.
    pub time_s: f64,
    /// The sea's level, metres.
    pub sea_level_m: f32,
    slot_of: BTreeMap<u32, usize>,
    next_id: u32,
}

/// What the island holds for the boats, beyond the water.
pub struct Island<'a> {
    /// The coastal cities, bases and roads harbours keep to.
    pub sites: harbour::Sites<'a>,
    /// The lakes.
    pub lakes: &'a [Lake],
    /// The archipelago's islets: centre, radius, cliffed.
    pub islets: Vec<(Vec2, f32, bool)>,
    /// The world's side, metres.
    pub world_size_m: f32,
}

impl Traffic {
    /// The boats of `plan`: `road_m` is how far a highway's graded band reaches from its centre line, `world_size_m`
    /// the world's side; `seed` the world's.
    pub fn from_plan(plan: &WorldPlan, cfg: &ShipsConfig, road_m: f32, world_size_m: f32, seed: u64) -> Self {
        let road_dist = |p: Vec2| plan.highways.iter().map(|h| h.project(p).0).fold(f32::INFINITY, f32::min);
        let island = Island {
            sites: harbour::Sites {
                cities: plan.cities.iter().map(|c| (c.id, c.rect())).collect(),
                bases: plan.bases.iter().map(|b| b.rect()).collect(),
                road_dist: &road_dist,
                road_m,
                ports: harbour::ports_of(plan, &cfg.harbours),
            },
            lakes: &plan.waters.lakes.list,
            islets: plan.terrain.shape().island().islets().collect(),
            world_size_m,
        };
        let sea = Sea {
            ground: plan,
            sea_level_m: plan.waters.sea_level_m,
        };
        Self::new(&island, &sea, cfg, seed)
    }

    /// Lays out the harbours, lanes and timetable of an island.
    pub fn new(island: &Island, sea: &Sea, cfg: &ShipsConfig, seed: u64) -> Self {
        let root = SimRng::new(seed).fork("ships");
        let harbours = harbour::place(&island.sites, sea, &cfg.harbours);
        let grid = SeaGrid::new(sea, island.world_size_m, &cfg.lanes);
        let mut routes = lanes::sea_lanes(&harbours, &grid, &cfg.lanes);
        let n_lanes = routes.len();
        let mut slots = Vec::new();
        let t = &cfg.traffic;
        let dwell = |rng: &mut SimRng, r: (f32, f32)| rng.range_f32(r.0, r.1);
        // Ships on the lanes, a headway apart round each out-and-back loop.
        for (i, r) in routes.iter().enumerate() {
            let mut rng = root.fork(&format!("lane/{i}"));
            let headway = rng.range_f32(t.headway_m.0, t.headway_m.1);
            let loop_m = r.length_m() * 2.0;
            let n = ((loop_m / headway).floor() as usize).max(1);
            for k in 0..n {
                let kind = if rng.chance(t.tanker_share) { TANKER } else { FREIGHTER };
                let offset_m = (k as f32 + rng.range_f32(0.0, 0.5)) * loop_m / n as f32;
                slots.push(Slot {
                    kind: kind.into(),
                    plan: Plan::Lane { route: i as u32, offset_m },
                    dwell_s: dwell(&mut rng, t.moor_s),
                    lake: false,
                    sunk: false,
                    live: None,
                });
            }
        }
        // Each harbour's fishing boats, each to a ground of its own off the mouth; and its ships at anchor.
        let mut grounds = Vec::new();
        for h in &harbours {
            let mut rng = root.fork(&format!("harbour/{}", h.id));
            for k in 0..t.fishing_per_harbour {
                let Some(ground) = (0..8).find_map(|_| {
                    let a = rng.range_f32(-0.7, 0.7);
                    let (s, c) = a.sin_cos();
                    let dir = Vec2::new(h.out.x * c - h.out.y * s, h.out.x * s + h.out.y * c);
                    let p = h.mouth + dir * rng.range_f32(cfg.harbours.ground_m.0, cfg.harbours.ground_m.1);
                    grid.open_at(p).then_some(p)
                }) else {
                    continue;
                };
                if k == 0 {
                    grounds.push(ground);
                }
                let mut pts = vec![h.berth, h.mouth];
                match grid.clear_line(h.mouth, ground) {
                    true => pts.push(ground),
                    false => match grid.path(h.mouth, ground) {
                        Some(mid) => {
                            pts.extend(mid);
                            pts.push(ground);
                        }
                        None => continue,
                    },
                }
                let route = routes.len() as u32;
                routes.push(Route::new(pts, [End::Berth(h.id), End::Ground]));
                let cycle = fishing_cycle(&routes[route as usize], cfg, t.work_s.1, t.moor_s.1);
                slots.push(Slot {
                    kind: FISHING.into(),
                    plan: Plan::Fishing {
                        route,
                        phase_s: rng.range_f32(0.0, cycle),
                    },
                    dwell_s: dwell(&mut rng, t.work_s),
                    lake: false,
                    sunk: false,
                    live: None,
                });
            }
            if grounds.len() <= h.id as usize {
                grounds.push(h.mouth);
            }
            let n = cfg.harbours.at_anchor;
            for k in 0..n {
                let kind = if rng.chance(t.tanker_share) { TANKER } else { FREIGHTER };
                slots.push(Slot {
                    kind: kind.into(),
                    plan: Plan::Anchor {
                        at: h.anchor_spot(k, n),
                        heading_rad: heading_of(h.side()),
                    },
                    dwell_s: 0.0,
                    lake: false,
                    sunk: false,
                    live: None,
                });
            }
        }
        // The lakes' boats.
        for (i, l) in island.lakes.iter().enumerate().filter(|(_, l)| !l.plunge) {
            let mut rng = root.fork(&format!("lake/{i}"));
            let n = ((l.radius_m / t.lake_boat_radius_m).floor() as usize).clamp(1, t.lake_boats_most.max(1));
            for _ in 0..n {
                let kind = if rng.chance(t.launch_share) { LAUNCH } else { ROWING };
                let draft = cfg.kinds.get(kind).map_or(1.0, |k| k.draft_m);
                let points = lake_loop(l, draft, sea, &mut rng);
                let perimeter = loop_length(&points);
                slots.push(Slot {
                    kind: kind.into(),
                    plan: Plan::Loop {
                        points,
                        phase_m: rng.range_f32(0.0, perimeter.max(1.0)),
                    },
                    dwell_s: 0.0,
                    lake: true,
                    sunk: false,
                    live: None,
                });
            }
        }
        // The ferry: from the harbour nearest the largest islet with a shore to land on, to a landing on it.
        let mut islet = None;
        if t.ferry
            && let Some(ferry) = cfg.kinds.get(FERRY)
            && let Some((route, isle)) = ferry_route(&harbours, &island.islets, &grid, sea, ferry.draft_m)
        {
            let mut rng = root.fork("ferry");
            islet = Some(isle);
            let r = routes.len() as u32;
            routes.push(route);
            let cycle = ferry_cycle(&routes[r as usize], ferry.speed_m_s, t.moor_s.1);
            slots.push(Slot {
                kind: FERRY.into(),
                plan: Plan::Ferry {
                    route: r,
                    phase_s: rng.range_f32(0.0, cycle),
                },
                dwell_s: t.moor_s.1,
                lake: false,
                sunk: false,
                live: None,
            });
        }
        Self {
            harbours,
            routes,
            lanes: n_lanes,
            slots,
            boats: Vec::new(),
            grounds,
            islet,
            time_s: 0.0,
            sea_level_m: sea.sea_level_m,
            slot_of: BTreeMap::new(),
            next_id: 0,
        }
    }

    /// The boat with this id.
    pub fn boat(&self, id: u32) -> Option<&Boat> {
        self.boats.iter().find(|b| b.id == id)
    }

    /// The boat with this id.
    pub fn boat_mut(&mut self, id: u32) -> Option<&mut Boat> {
        self.boats.iter_mut().find(|b| b.id == id)
    }

    /// Where slot `i`'s boat is by the timetable at `t_s`.
    fn place(&self, i: usize, cfg: &ShipsConfig, t_s: f64) -> Option<Placed> {
        let slot = &self.slots[i];
        let spec = cfg.kinds.get(&slot.kind)?;
        let v = spec.speed_m_s;
        let route_at = |route: u32, s: f32, forward: bool| {
            let r = &self.routes[route as usize];
            let (p, dir) = r.at(s);
            let travel = if forward { dir } else { -dir };
            (p + travel.perp() * cfg.lanes.keep_right_m, heading_of(travel))
        };
        let underway = |route: u32, s: f32, forward: bool| {
            let (pos, heading_rad) = route_at(route, s, forward);
            Placed {
                pos,
                heading_rad,
                errand: Errand::Route { route, s_m: s, forward },
                phase: Phase::Underway,
            }
        };
        Some(match &slot.plan {
            Plan::Lane { route, offset_m } => {
                let len = self.routes[*route as usize].length_m();
                let u = ((f64::from(*offset_m) + f64::from(v) * t_s) % f64::from(2.0 * len)) as f32;
                if u < len {
                    underway(*route, u, true)
                } else {
                    underway(*route, 2.0 * len - u, false)
                }
            }
            Plan::Fishing { route, phase_s } => {
                let r = &self.routes[*route as usize];
                let len = r.length_m();
                let run = len / v;
                let (work, moor) = (slot.dwell_s, cfg.traffic.moor_s.0);
                let cycle = 2.0 * run + work + moor;
                let u = ((f64::from(*phase_s) + t_s) % f64::from(cycle)) as f32;
                if u < run {
                    underway(*route, u * v, true)
                } else if u < run + work {
                    let ground = *r.points.last()?;
                    let a = (u - run) * cfg.traffic.work_m_s / cfg.traffic.work_radius_m;
                    Placed {
                        pos: ground + Vec2::new(a.cos(), a.sin()) * cfg.traffic.work_radius_m,
                        heading_rad: heading_of(Vec2::new(-a.sin(), a.cos())),
                        errand: Errand::Route {
                            route: *route,
                            s_m: len,
                            forward: true,
                        },
                        phase: Phase::Working {
                            left_s: run + work - u,
                            angle_rad: a,
                        },
                    }
                } else if u < 2.0 * run + work {
                    underway(*route, len - (u - run - work) * v, false)
                } else {
                    moored(r, *route, false, spec.beam_m(), cycle - u)
                }
            }
            Plan::Ferry { route, phase_s } => {
                let r = &self.routes[*route as usize];
                let len = r.length_m();
                let run = len / v;
                let moor = slot.dwell_s;
                let cycle = 2.0 * (run + moor);
                let u = ((f64::from(*phase_s) + t_s) % f64::from(cycle)) as f32;
                if u < run {
                    underway(*route, u * v, true)
                } else if u < run + moor {
                    moored(r, *route, true, spec.beam_m(), run + moor - u)
                } else if u < 2.0 * run + moor {
                    underway(*route, len - (u - run - moor) * v, false)
                } else {
                    moored(r, *route, false, spec.beam_m(), cycle - u)
                }
            }
            Plan::Anchor { at, heading_rad } => Placed {
                pos: *at,
                heading_rad: *heading_rad,
                errand: Errand::Anchor { at: *at },
                phase: Phase::Anchored,
            },
            Plan::Loop { points, phase_m } => {
                let perimeter = loop_length(points);
                let u = ((f64::from(*phase_m) + f64::from(v) * t_s) % f64::from(perimeter.max(1.0))) as f32;
                let (pos, dir, next) = along_loop(points, u);
                Placed {
                    pos,
                    heading_rad: heading_of(dir),
                    errand: Errand::Loop {
                        points: points.clone(),
                        next,
                    },
                    phase: Phase::Underway,
                }
            }
        })
    }

    /// One fixed step of `dt_s` with the pilot at `pilot` (x, z): slots near the pilot become real boats, real ones
    /// far off stop being so, wrecks long on the bottom go, and every real boat steps. Returns each boat's step
    /// where something happened.
    pub fn step(&mut self, cfg: &ShipsConfig, sea: &Sea, pilot: Vec2, dt_s: f32) -> Vec<(u32, BoatStep)> {
        self.time_s += f64::from(dt_s);
        self.keep_round(cfg, sea, pilot);
        let others: Vec<Neighbour> = self
            .boats
            .iter()
            .filter(|b| b.afloat())
            .filter_map(|b| {
                Some(Neighbour {
                    id: b.id,
                    pos: b.pos,
                    vel: b.vel(),
                    length_m: cfg.kinds.get(&b.kind)?.length_m(),
                })
            })
            .collect();
        let mut out = Vec::new();
        for b in &mut self.boats {
            let Some(spec) = cfg.kinds.get(&b.kind) else { continue };
            // Only the boats within reach of each other are neighbours worth asking.
            let reach = spec.length_m() * cfg.steer.gap_lengths + 200.0;
            let near: Vec<Neighbour> = others.iter().copied().filter(|o| o.pos.distance(b.pos) <= reach).collect();
            let view = BoatView {
                spec,
                sea,
                routes: &self.routes,
                others: &near,
                steer: &cfg.steer,
                traffic: &cfg.traffic,
                keep_right_m: cfg.lanes.keep_right_m,
                approach_m: cfg.harbours.approach_m,
            };
            let st = b.step(&view, dt_s);
            if st != BoatStep::default() {
                out.push((b.id, st));
            }
        }
        out
    }

    /// One fixed step of every boat drawn from a ship kit that is going down (`Boat::step_kit`), over the seabed
    /// `seabed` gives at a point (x, z).
    pub fn step_kits(&mut self, kits: &Kits, seabed: &dyn Fn(Vec2) -> f32, dt_s: f32) {
        for b in self
            .boats
            .iter_mut()
            .filter(|b| matches!(b.phase, Phase::Sinking { .. } | Phase::Sunk { .. }))
        {
            let Some(k) = kits.ship(&b.kind) else { continue };
            let floor = seabed(b.pos);
            b.step_kit(&kits.kits[k], floor, dt_s);
        }
    }

    /// Makes the timetable's boats near the pilot real and lets far ones go.
    fn keep_round(&mut self, cfg: &ShipsConfig, sea: &Sea, pilot: Vec2) {
        let t = &cfg.traffic;
        let lake_out = t.lake_spawn_m * t.despawn_m / t.spawn_m;
        // Let go: far boats (not sinking, wrecks or wards), boats gone over the horizon, wrecks long down.
        let mut keep = Vec::with_capacity(self.boats.len());
        for b in std::mem::take(&mut self.boats) {
            let slot = self.slot_of.get(&b.id).copied();
            let lake = slot.is_some_and(|s| self.slots[s].lake);
            let out_m = if lake { lake_out } else { t.despawn_m };
            let far = b.pos.distance(pilot) > out_m;
            let fighting = matches!(b.phase, Phase::Sinking { .. }) || b.ward.is_some();
            let drop = match b.phase {
                Phase::Gone => true,
                Phase::Sunk { t_s } => t_s > cfg.fight.wreck_s || far,
                _ => far && !fighting,
            };
            if !b.afloat()
                && let Some(s) = slot
                && !matches!(b.phase, Phase::Gone)
            {
                self.slots[s].sunk = true;
            }
            if drop {
                if let Some(s) = slot {
                    self.slots[s].live = None;
                }
                self.slot_of.remove(&b.id);
            } else {
                keep.push(b);
            }
        }
        self.boats = keep;
        // Make real: slots whose place is near, in slot order (stable ids).
        for i in 0..self.slots.len() {
            let s = &self.slots[i];
            if s.sunk || s.live.is_some() {
                continue;
            }
            let Some(p) = self.place(i, cfg, self.time_s) else { continue };
            let reach = if s.lake { t.lake_spawn_m } else { t.spawn_m };
            if p.phase == Phase::Gone || p.pos.distance(pilot) > reach {
                continue;
            }
            let Some(spec) = cfg.kinds.get(&s.kind) else { continue };
            let id = self.next_id;
            self.next_id += 1;
            let mut b = Boat::new(id, &s.kind, spec, p.errand, p.pos, p.heading_rad);
            b.phase = p.phase;
            b.dwell_s = s.dwell_s;
            if matches!(b.phase, Phase::Moored { .. } | Phase::Anchored) {
                b.speed_m_s = 0.0;
            }
            if let Phase::Working { .. } = b.phase {
                b.speed_m_s = cfg.traffic.work_m_s;
            }
            b.surface_m = sea.water(b.pos).map_or(sea.sea_level_m, |w| w.0);
            self.slots[i].live = Some(id);
            self.slot_of.insert(id, i);
            self.boats.push(b);
        }
        self.boats.sort_by_key(|b| b.id);
    }

    /// Puts a boat of `kind` into the world outside the timetable (an escort's freighter, a raider's sinking hull):
    /// returns its id.
    /// A boat added outside the timetable floats at sea level until its first sounding.
    pub fn add(&mut self, kind: &str, cfg: &ShipsConfig, errand: Errand, pos: Vec2, heading_rad: f32) -> Option<u32> {
        let spec = cfg.kinds.get(kind)?;
        let id = self.next_id;
        self.next_id += 1;
        let mut b = Boat::new(id, kind, spec, errand, pos, heading_rad);
        b.surface_m = self.sea_level_m;
        self.boats.push(b);
        Some(id)
    }

    /// The sea lane out to the offing from harbour `h`, and where on it an escort's freighter starts (`out_m` past
    /// the mouth): (route, metres along it, the point).
    pub fn escort_start(&self, h: u32, out_m: f32, approach_m: f32) -> Option<(u32, f32, Vec2)> {
        let i = self.routes[..self.lanes]
            .iter()
            .position(|r| r.ends == [End::Berth(h), End::Offing])
            .or_else(|| self.routes[..self.lanes].iter().position(|r| r.ends[0] == End::Berth(h)))?;
        let r = &self.routes[i];
        let s = (approach_m + out_m).min(r.length_m() - 1.0).max(0.0);
        Some((i as u32, s, r.at(s).0))
    }

    /// The harbours as shipping jobs see them (`jobs::SeaSite`).
    pub fn sea_sites(&self, escort_out_m: f32, approach_m: f32) -> Vec<SeaSite> {
        self.harbours
            .iter()
            .map(|h| SeaSite {
                city: h.city,
                harbour: h.id,
                mouth: h.mouth,
                ground: self.grounds.get(h.id as usize).copied().unwrap_or(h.mouth),
                escort_from: self.escort_start(h.id, escort_out_m, approach_m).map_or(h.mouth, |e| e.2),
            })
            .collect()
    }
}

/// A moored boat by the timetable: alongside its route's end (`at_end`: the last point, else the first).
fn moored(r: &Route, route: u32, at_end: bool, beam_m: f32, left_s: f32) -> Placed {
    let (p, dir) = r.at(if at_end { r.length_m() } else { 0.0 });
    let in_dir = if at_end { dir } else { -dir };
    Placed {
        pos: p - in_dir * (beam_m * 0.5 + 2.0),
        heading_rad: heading_of(in_dir.perp()),
        errand: Errand::Route {
            route,
            s_m: if at_end { r.length_m() } else { 0.0 },
            forward: at_end,
        },
        phase: Phase::Moored { left_s },
    }
}

fn fishing_cycle(r: &Route, cfg: &ShipsConfig, work_s: f32, moor_s: f32) -> f32 {
    let v = cfg.kinds.get(FISHING).map_or(1.0, |k| k.speed_m_s);
    2.0 * r.length_m() / v + work_s + moor_s
}

fn ferry_cycle(r: &Route, speed_m_s: f32, moor_s: f32) -> f32 {
    2.0 * (r.length_m() / speed_m_s + moor_s)
}

/// The length round a closed loop of points.
fn loop_length(points: &[Vec2]) -> f32 {
    (0..points.len()).map(|i| points[i].distance(points[(i + 1) % points.len()])).sum()
}

/// The point `u` metres round a closed loop, the way it runs there, and the index of the point it runs toward.
fn along_loop(points: &[Vec2], mut u: f32) -> (Vec2, Vec2, usize) {
    let n = points.len();
    if n == 0 {
        return (Vec2::ZERO, Vec2::X, 0);
    }
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        let l = a.distance(b);
        if u <= l || i == n - 1 {
            let dir = (b - a).normalize_or(Vec2::X);
            return (a + dir * u.min(l), dir, (i + 1) % n);
        }
        u -= l;
    }
    (points[0], Vec2::X, 0)
}

/// A loop of five points round a lake, inside 70 % of its radius, each over water deeper than `draft_m` plus a
/// metre (pulled in toward the middle until it is).
fn lake_loop(l: &Lake, draft_m: f32, sea: &Sea, rng: &mut SimRng) -> Vec<Vec2> {
    let base = rng.range_f32(0.0, std::f32::consts::TAU);
    (0..5)
        .map(|j| {
            let a = base + j as f32 / 5.0 * std::f32::consts::TAU + rng.range_f32(-0.3, 0.3);
            let mut r = l.radius_m * rng.range_f32(0.35, 0.7);
            let dir = Vec2::new(a.cos(), a.sin());
            for _ in 0..6 {
                if sea.depth(l.centre + dir * r) >= draft_m + 1.0 {
                    break;
                }
                r *= 0.7;
            }
            l.centre + dir * r
        })
        .collect()
}

/// The ferry's route: from the harbour nearest the largest islet that is not cliffed all round, out through the
/// harbour's mouth over open water to a landing on the islet's side facing it. `None` without both.
fn ferry_route(
    harbours: &[Harbour],
    islets: &[(Vec2, f32, bool)],
    grid: &SeaGrid,
    sea: &Sea,
    draft_m: f32,
) -> Option<(Route, (Vec2, f32))> {
    let (centre, radius, _) = islets
        .iter()
        .filter(|i| !i.2)
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.x.total_cmp(&a.0.x)))
        .copied()?;
    let h = harbours
        .iter()
        .min_by(|a, b| a.berth.distance(centre).total_cmp(&b.berth.distance(centre)))?;
    // The landing: out from the islet's centre toward the harbour until the water floats the ferry with room.
    let toward = (h.berth - centre).normalize_or(Vec2::X);
    let mut d = 0.0;
    let mut landing = None;
    let mut ashore = false;
    while d < radius * 3.0 + 2000.0 {
        let p = centre + toward * d;
        let depth = sea.sea_depth(p);
        ashore |= depth <= 0.0;
        if ashore && depth >= draft_m + 2.0 {
            landing = Some(p);
            break;
        }
        d += 10.0;
    }
    let landing = landing?;
    let mid = grid.path(h.mouth, landing)?;
    let mut pts = vec![h.berth, h.mouth];
    pts.extend(mid);
    pts.push(landing);
    Some((Route::new(pts, [End::Berth(h.id), End::Landing]), (centre, radius)))
}

#[cfg(test)]
mod tests {
    use super::super::test_world::{self, shipped};
    use super::*;
    use crate::world::layout::Rect;

    fn island() -> Island<'static> {
        static LAKES: std::sync::OnceLock<Vec<Lake>> = std::sync::OnceLock::new();
        let lakes = LAKES.get_or_init(|| {
            vec![Lake {
                centre: Vec2::new(1000.0, 0.0),
                radius_m: 300.0,
                level_m: 40.0,
                depth_m: 6.0,
                plunge: false,
            }]
        });
        Island {
            sites: harbour::Sites {
                cities: vec![
                    (0, Rect::centred(Vec2::new(2700.0, 0.0), Vec2::splat(250.0))),
                    (1, Rect::centred(Vec2::new(-2700.0, 0.0), Vec2::splat(250.0))),
                ],
                bases: vec![],
                road_dist: &|_| f32::INFINITY,
                road_m: 0.0,
                ports: Vec::new(),
            },
            lakes,
            // A beach islet to the south, 6 km out.
            islets: vec![(Vec2::new(0.0, 7000.0), 200.0, false)],
            world_size_m: 16000.0,
        }
    }

    /// The test island has no islet in its ground; this one has the islet in the sea too.
    struct WithIslet;
    impl crate::world::Ground for WithIslet {
        fn height_at(&self, x: f32, z: f32) -> f32 {
            let p = Vec2::new(x, z);
            let isle = p.distance(Vec2::new(0.0, 7000.0));
            if isle < 400.0 {
                return 10.0 - isle / 400.0 * 70.0;
            }
            test_world::Island::ground_at(p)
        }
        fn push_out(&self, p: Vec2, _: f32) -> Vec2 {
            p
        }
        fn water_at(&self, x: f32, z: f32) -> Option<crate::world::WaterSample> {
            let g = self.height_at(x, z);
            if Vec2::new(x, z).distance(Vec2::new(0.0, 7000.0)) < 400.0 {
                return (g < 0.0).then_some(crate::world::WaterSample {
                    surface_m: 0.0,
                    depth_m: -g,
                    flow_m_s: Vec2::ZERO,
                });
            }
            test_world::Island.water_at(x, z)
        }
    }

    fn traffic() -> (Traffic, ShipsConfig) {
        let cfg = shipped();
        let sea = Sea {
            ground: &WithIslet,
            sea_level_m: 0.0,
        };
        (Traffic::new(&island(), &sea, &cfg, 42), cfg)
    }

    fn sea() -> Sea<'static> {
        Sea {
            ground: &WithIslet,
            sea_level_m: 0.0,
        }
    }

    #[test]
    fn the_timetable_has_ships_fishing_boats_anchored_ships_lake_boats_and_the_ferry() {
        let (t, cfg) = traffic();
        assert_eq!(t.harbours.len(), 2, "both coastal cities");
        assert!(t.lanes >= 3, "harbour to harbour and each out to sea: {}", t.lanes);
        let has = |k: &str| t.slots.iter().any(|s| s.kind == k);
        assert!(has(FREIGHTER) || has(TANKER));
        assert_eq!(
            t.slots.iter().filter(|s| s.kind == FISHING).count(),
            2 * cfg.traffic.fishing_per_harbour
        );
        assert_eq!(
            t.slots.iter().filter(|s| matches!(s.plan, Plan::Anchor { .. })).count(),
            2 * cfg.harbours.at_anchor
        );
        assert!(t.slots.iter().any(|s| s.lake), "boats on the lake");
        assert!(has(FERRY) && t.islet.is_some(), "the ferry to the islet");
    }

    #[test]
    fn boats_are_real_only_near_the_pilot() {
        let (mut t, cfg) = traffic();
        let s = sea();
        // Far out at sea, past every harbour.
        t.step(&cfg, &s, Vec2::new(60_000.0, 60_000.0), 0.1);
        assert!(t.boats.is_empty(), "{} boats", t.boats.len());
        // At the first harbour.
        let at = t.harbours[0].berth;
        t.step(&cfg, &s, at, 0.1);
        assert!(!t.boats.is_empty());
        for b in &t.boats {
            let reach = if t.slots[t.slot_of[&b.id]].lake {
                cfg.traffic.lake_spawn_m
            } else {
                cfg.traffic.spawn_m
            };
            assert!(b.pos.distance(at) <= reach + 1.0, "{:?}", b.pos);
        }
        // Gone again once the pilot leaves.
        t.step(&cfg, &s, Vec2::new(60_000.0, 60_000.0), 0.1);
        assert!(t.boats.is_empty());
    }

    #[test]
    fn the_same_seed_and_pilot_path_give_the_same_boats() {
        let run = || {
            let (mut t, cfg) = traffic();
            let s = sea();
            let path = [Vec2::new(4000.0, 0.0), Vec2::new(0.0, 4500.0), Vec2::new(-4000.0, 0.0)];
            for p in path {
                for _ in 0..300 {
                    t.step(&cfg, &s, p, 0.1);
                }
            }
            t.boats
                .iter()
                .map(|b| (b.id, b.kind.clone(), b.pos.x.to_bits(), b.pos.y.to_bits()))
                .collect::<Vec<_>>()
        };
        let a = run();
        assert!(!a.is_empty());
        assert_eq!(a, run());
    }

    #[test]
    fn a_sunk_slot_stays_empty() {
        let (mut t, cfg) = traffic();
        let s = sea();
        let at = t.harbours[0].anchorage;
        t.step(&cfg, &s, at, 0.1);
        let id = t
            .boats
            .iter()
            .find(|b| matches!(b.errand, Errand::Anchor { .. }))
            .expect("an anchored ship")
            .id;
        let slot = t.slot_of[&id];
        t.boat_mut(id).unwrap().founder();
        for _ in 0..200 {
            t.step(&cfg, &s, at, 1.0);
        }
        // Long gone from the bottom; leave and come back.
        t.step(&cfg, &s, Vec2::new(60_000.0, 60_000.0), cfg.fight.wreck_s + 1.0);
        t.step(&cfg, &s, at, 0.1);
        assert!(t.slots[slot].sunk && t.slots[slot].live.is_none());
    }

    #[test]
    fn a_ward_is_not_let_go_when_the_pilot_is_far() {
        let (mut t, cfg) = traffic();
        let s = sea();
        let (route, s_m, p) = t
            .escort_start(0, cfg.lanes.escort_out_m, cfg.harbours.approach_m)
            .expect("a lane out");
        let id = t
            .add(
                FREIGHTER,
                &cfg,
                Errand::Route {
                    route,
                    s_m,
                    forward: false,
                },
                p,
                0.0,
            )
            .unwrap();
        t.boat_mut(id).unwrap().ward = Some(5);
        t.step(&cfg, &s, Vec2::new(60_000.0, 60_000.0), 0.1);
        assert!(t.boat(id).is_some());
    }

    #[test]
    fn an_escorted_freighter_sails_in_and_moors() {
        let (mut t, cfg) = traffic();
        let s = sea();
        let (route, s_m, p) = t.escort_start(0, cfg.lanes.escort_out_m, cfg.harbours.approach_m).unwrap();
        let heading = heading_of(-t.routes[route as usize].at(s_m).1);
        let id = t
            .add(
                FREIGHTER,
                &cfg,
                Errand::Route {
                    route,
                    s_m,
                    forward: false,
                },
                p,
                heading,
            )
            .unwrap();
        t.boat_mut(id).unwrap().ward = Some(1);
        let far = Vec2::new(60_000.0, 60_000.0);
        let mut moored = false;
        for _ in 0..12_000 {
            moored |= t.step(&cfg, &s, far, 0.1).iter().any(|(b, st)| *b == id && st.moored);
            if moored {
                break;
            }
        }
        assert!(moored, "{:?}", t.boat(id));
    }

    #[test]
    fn lake_boats_keep_to_their_lake() {
        let (mut t, cfg) = traffic();
        let s = sea();
        let lake = Vec2::new(1000.0, 0.0);
        for _ in 0..3000 {
            t.step(&cfg, &s, lake, 0.2);
        }
        let on_lake: Vec<&Boat> = t.boats.iter().filter(|b| t.slots[t.slot_of[&b.id]].lake).collect();
        assert!(!on_lake.is_empty());
        for b in on_lake {
            assert!(b.pos.distance(lake) < 300.0 && s.depth(b.pos) > 0.0, "{:?}", b.pos);
        }
    }

    #[test]
    fn sea_sites_name_each_harbour() {
        let (t, cfg) = traffic();
        let sites = t.sea_sites(cfg.lanes.escort_out_m, cfg.harbours.approach_m);
        assert_eq!(sites.len(), t.harbours.len());
        for (s, h) in sites.iter().zip(&t.harbours) {
            assert_eq!(s.harbour, h.id);
            assert!(s.escort_from.distance(h.mouth) > 1000.0, "out along a lane");
        }
    }
}
