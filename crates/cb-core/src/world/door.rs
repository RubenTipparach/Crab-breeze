//! Street doors that swing open as a pilot on foot walks into them and shut behind them (openspec
//! change `building-interiors`, design section 5; survey W30, owner 2026-10-26: "A door that swings
//! open as you walk into it, and shuts behind you").
//!
//! A `StreetDoor` is the leaves of a building's `Entrance`, each hung on a jamb at the middle of the
//! wall's depth. It is shut until a pilot on foot comes within `door_trigger_m` of it on either side
//! of the leaf, then swings away from them (into the building from the street, out of it from the
//! hall), stays open while they are in the doorway or the leaf's arc, and shuts after
//! `door_close_delay_s` once they have gone. A leaf never moves into the pilot's body: a step that
//! would bring it nearer while it touches them is not taken. A shut leaf is a wall to the pilot; an
//! open one stands against its jamb, clear of the way in.
//!
//! The door's state lives with the building's colliders (`interior::Walkable`), which exist only
//! while the pilot is on foot, so the door is an on-foot thing too. Pure and deterministic: stepped
//! once per fixed step, before the pilot moves.

use super::dressing::Anchor;
use super::interior::{DoorKind, Entrance, InteriorConfig, SKIN_M, Site};
use glam::Vec2;

/// A leaf's thickness.
pub const LEAF_T_M: f32 = 0.05;

/// Where a door is in its swing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorPhase {
    /// Shut in its frame.
    Closed,
    /// Swinging open, away from the pilot.
    Opening,
    /// Standing open while the pilot is near, then for `door_close_delay_s`.
    Open,
    /// Swinging shut.
    Closing,
}

/// What a door step did that is heard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorEvent {
    /// It began to swing open from shut.
    Opened,
    /// It came to rest shut.
    Shut,
}

/// The pilot's body as a door sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Feet, world (x, z).
    pub at: Vec2,
    /// Feet height.
    pub feet_y: f32,
    /// Body radius.
    pub radius_m: f32,
}

/// One leaf: its hinge and which way it lies when shut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leaf {
    /// The hinge's axis, world (x, z), at the middle of the wall's depth.
    pub hinge: Vec2,
    /// Unit, from the hinge towards the free edge while shut (along the facade).
    pub shut: Vec2,
    /// Hinge to free edge.
    pub width_m: f32,
}

/// A building's street door and where it is in its swing.
#[derive(Clone, Debug, PartialEq)]
pub struct StreetDoor {
    /// The leaves (a lobby's pair, otherwise one).
    pub leaves: Vec<Leaf>,
    /// Unit, into the building, world (x, z).
    pub inward: Vec2,
    /// Unit, along the facade, world (x, z).
    pub along: Vec2,
    /// The opening's centre on the facade's plane, world (x, z).
    pub centre: Vec2,
    /// Half the clear width.
    pub half_width_m: f32,
    /// The leaves' bottom (the sill) and top, world height.
    pub y0: f32,
    /// See `y0`.
    pub y1: f32,
    /// The wall bay it hangs in: when that falls, the door goes with it.
    pub hold: Anchor,
    /// Widest swing out of the building: `door_open_deg`, or less where the clear distance in front of
    /// the facade is short, so the leaf stays on this building's side of the party line.
    pub out_max_deg: f32,
    /// Where it is in its swing.
    pub phase: DoorPhase,
    /// The leaves' angle from shut, degrees: positive into the building, negative out of it.
    pub angle_deg: f32,
    /// `angle_deg` before the last step, for drawing between steps.
    pub prev_angle_deg: f32,
    /// +1 swinging in, -1 swinging out (meaningful while not shut).
    swing: f32,
    /// Time left standing open before it starts to shut.
    hold_s: f32,
}

/// Least distance from `p` to the segment `a`..`b`, and the nearest point on it.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> (f32, Vec2) {
    let ab = b - a;
    let t = if ab.length_squared() > 1e-12 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let q = a + ab * t;
    (p.distance(q), q)
}

impl StreetDoor {
    /// The door of entrance `e` on `site`, shut.
    pub fn new(site: &Site, e: &Entrance, cfg: &InteriorConfig) -> Self {
        let inward = -Site::out_of(e.side);
        let along = if e.side < 2 { Vec2::X } else { Vec2::Y };
        let half = e.width_m * 0.5;
        let mid = e.centre + inward * (SKIN_M * 0.5);
        let jamb = half - LEAF_T_M * 0.5;
        let leaves = match e.kind {
            DoorKind::Lobby => vec![
                Leaf {
                    hinge: mid - along * jamb,
                    shut: along,
                    width_m: jamb,
                },
                Leaf {
                    hinge: mid + along * jamb,
                    shut: -along,
                    width_m: jamb,
                },
            ],
            _ => vec![Leaf {
                hinge: mid - along * jamb,
                shut: along,
                width_m: e.width_m - LEAF_T_M,
            }],
        };
        // Out of the building the leaf reaches `width * sin(angle) - SKIN_M / 2` past the facade, and
        // no farther than this building's half of the gap in front of it.
        let room = (site.fronts[e.side as usize] * 0.5 - 0.05 + SKIN_M * 0.5).max(0.0);
        let widest = leaves.iter().map(|l| l.width_m).fold(0.0f32, f32::max).max(1e-3);
        let ratio = room / widest;
        let out_max_deg = if ratio >= 1.0 {
            cfg.door_open_deg
        } else {
            ratio.asin().to_degrees().min(cfg.door_open_deg)
        };
        let y0 = site.origin.y + e.sill_m;
        Self {
            leaves,
            inward,
            along,
            centre: e.centre,
            half_width_m: half,
            y0,
            y1: y0 + e.clear_m,
            hold: Anchor::Wall {
                k: 0,
                side: e.side,
                idx: e.bay,
            },
            out_max_deg,
            phase: DoorPhase::Closed,
            angle_deg: 0.0,
            prev_angle_deg: 0.0,
            swing: 1.0,
            hold_s: 0.0,
        }
    }

    /// A single leaf hung at `hinge` (x, z), shut along `shut` for `width_m`, opening into `inward`, its sill
    /// at `y0` and head at `y1`, standing while `hold` stands; it may swing out of the building by up to
    /// `out_max_deg`. Shut. (A house's doors, `small_buildings::walk`, in the house's frame.)
    pub fn hung(hinge: Vec2, shut: Vec2, width_m: f32, inward: Vec2, y0: f32, y1: f32, hold: Anchor, out_max_deg: f32) -> Self {
        Self {
            leaves: vec![Leaf { hinge, shut, width_m }],
            inward,
            along: shut,
            centre: hinge + shut * (width_m * 0.5),
            half_width_m: width_m * 0.5,
            y0,
            y1,
            hold,
            out_max_deg,
            phase: DoorPhase::Closed,
            angle_deg: 0.0,
            prev_angle_deg: 0.0,
            swing: 1.0,
            hold_s: 0.0,
        }
    }

    /// Unit, from leaf `i`'s hinge to its free edge at `angle_deg`.
    pub fn leaf_dir(&self, i: usize, angle_deg: f32) -> Vec2 {
        let (s, c) = angle_deg.to_radians().sin_cos();
        self.leaves[i].shut * c + self.inward * s
    }

    /// Leaf `i` as a segment, hinge to free edge, at `angle_deg`.
    pub fn leaf_at(&self, i: usize, angle_deg: f32) -> (Vec2, Vec2) {
        let l = &self.leaves[i];
        (l.hinge, l.hinge + self.leaf_dir(i, angle_deg) * l.width_m)
    }

    /// True when a body with its feet at `feet_y` meets the leaves' height (as `Walkable` decides
    /// what is in the way: over a step, under a head).
    pub fn at_level(&self, feet_y: f32, cfg: &InteriorConfig) -> bool {
        feet_y.is_finite() && feet_y + cfg.step_m < self.y1 && feet_y + cfg.head_m > self.y0
    }

    /// True when `b` is in the door's trigger zones: within `door_trigger_m` of the leaf's line on
    /// either side, across the opening's width, or in the opening itself.
    pub fn in_zone(&self, b: &Body, cfg: &InteriorConfig) -> bool {
        let r = b.at - self.centre;
        let (a, d) = (r.dot(self.along), r.dot(self.inward));
        a.abs() <= self.half_width_m + b.radius_m && d >= -cfg.door_trigger_m && d <= SKIN_M + cfg.door_trigger_m
    }

    /// True when `b` stands in the arc an open leaf sweeps on its way shut.
    pub fn in_arc(&self, b: &Body) -> bool {
        if self.angle_deg == 0.0 {
            return false;
        }
        self.leaves.iter().any(|l| {
            let r = b.at - l.hinge;
            r.length() <= l.width_m + b.radius_m && r.dot(l.shut) >= -b.radius_m && r.dot(self.inward) * self.swing >= -b.radius_m
        })
    }

    /// Nearest any leaf comes to `p` at `angle_deg`.
    fn nearest(&self, p: Vec2, angle_deg: f32) -> f32 {
        (0..self.leaves.len())
            .map(|i| {
                let (a, b) = self.leaf_at(i, angle_deg);
                to_segment(p, a, b).0
            })
            .fold(f32::INFINITY, f32::min)
    }

    /// Swings towards `target_deg` at `rate_deg_s` for `dt`, unless that would bring a leaf into `body`
    /// (then it waits). True once there.
    fn swing_to(&mut self, target_deg: f32, rate_deg_s: f32, dt: f32, body: Option<&Body>) -> bool {
        let step = rate_deg_s * dt;
        let next = if (target_deg - self.angle_deg).abs() <= step {
            target_deg
        } else {
            self.angle_deg + step * (target_deg - self.angle_deg).signum()
        };
        if let Some(b) = body {
            let touch = b.radius_m + LEAF_T_M * 0.5 + 0.01;
            let (now, then) = (self.nearest(b.at, self.angle_deg), self.nearest(b.at, next));
            if then < touch && then < now - 1e-5 {
                return false;
            }
        }
        self.angle_deg = next;
        next == target_deg
    }

    /// One fixed step of `dt` seconds with the pilot's body, when they are on foot (`None` for no one).
    pub fn step(&mut self, pilot: Option<Body>, cfg: &InteriorConfig, dt: f32) -> Option<DoorEvent> {
        self.prev_angle_deg = self.angle_deg;
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        let body = pilot.filter(|b| b.at.is_finite() && b.radius_m.is_finite() && self.at_level(b.feet_y, cfg));
        let wants = body.as_ref().is_some_and(|b| self.in_zone(b, cfg) || self.in_arc(b));
        let open_to = |s: f32| if s > 0.0 { cfg.door_open_deg } else { -self.out_max_deg };
        match self.phase {
            DoorPhase::Closed => {
                let b = body.filter(|_| wants)?;
                // Away from them: in from the street, out from the hall.
                let d = (b.at - self.centre).dot(self.inward);
                self.swing = if d < SKIN_M * 0.5 { 1.0 } else { -1.0 };
                self.phase = DoorPhase::Opening;
                self.swing_to(open_to(self.swing), cfg.door_open_deg_s, dt, body.as_ref());
                Some(DoorEvent::Opened)
            }
            DoorPhase::Opening => {
                if self.swing_to(open_to(self.swing), cfg.door_open_deg_s, dt, body.as_ref()) {
                    self.phase = DoorPhase::Open;
                    self.hold_s = cfg.door_close_delay_s;
                }
                None
            }
            DoorPhase::Open => {
                if wants {
                    self.hold_s = cfg.door_close_delay_s;
                } else {
                    self.hold_s -= dt;
                    if self.hold_s <= 0.0 {
                        self.phase = DoorPhase::Closing;
                    }
                }
                None
            }
            DoorPhase::Closing => {
                if wants {
                    self.phase = DoorPhase::Opening;
                    return None;
                }
                if self.swing_to(0.0, cfg.door_close_deg_s, dt, body.as_ref()) {
                    self.phase = DoorPhase::Closed;
                    return Some(DoorEvent::Shut);
                }
                None
            }
        }
    }

    /// A body of `radius_m` at `p` with its feet at `feet_y`, moved out of the leaves as they stand.
    pub fn push_out(&self, p: Vec2, radius_m: f32, feet_y: f32, cfg: &InteriorConfig) -> Vec2 {
        if !p.is_finite() || !self.at_level(feet_y, cfg) {
            return p;
        }
        let reach = radius_m + LEAF_T_M * 0.5;
        let mut p = p;
        for i in 0..self.leaves.len() {
            let (a, b) = self.leaf_at(i, self.angle_deg);
            let (d, q) = to_segment(p, a, b);
            if d >= reach {
                continue;
            }
            let away = if d > 1e-6 {
                (p - q) / d
            } else {
                // On the leaf's line: back to the side of it the door's centre is not on.
                let dir = (b - a).normalize_or_zero();
                let n = Vec2::new(-dir.y, dir.x);
                if (self.centre - self.inward - q).dot(n) >= 0.0 { n } else { -n }
            };
            p = q + away * reach;
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn site(fronts: f32) -> Site {
        Site {
            id: 1,
            n: [3, 2, 3],
            origin: Vec3::new(0.0, 0.0, 0.0),
            size: Vec3::new(6.0, 3.5, 6.0),
            street: 0,
            fronts: [fronts; 4],
        }
    }

    fn door(kind: DoorKind, fronts: f32) -> (StreetDoor, InteriorConfig) {
        let cfg = InteriorConfig::default();
        let s = site(fronts);
        let mut e = super::super::interior::shell(&s, &cfg, &crate::rng::SimRng::new(42)).entrance;
        e.kind = kind;
        e.width_m = if kind == DoorKind::Lobby {
            cfg.lobby_door_w_m
        } else {
            cfg.front_door_w_m
        };
        (StreetDoor::new(&s, &e, &cfg), cfg)
    }

    fn body(at: Vec2) -> Body {
        Body {
            at,
            feet_y: 0.0,
            radius_m: 0.35,
        }
    }

    fn run(d: &mut StreetDoor, cfg: &InteriorConfig, at: Option<Vec2>, s: f32) {
        for _ in 0..(s * 60.0) as usize {
            d.step(at.map(body), cfg, 1.0 / 60.0);
        }
    }

    #[test]
    fn a_door_is_shut_until_someone_comes_near_it() {
        let (mut d, cfg) = door(DoorKind::Front, 40.0);
        run(&mut d, &cfg, None, 2.0);
        assert_eq!((d.phase, d.angle_deg), (DoorPhase::Closed, 0.0));
        // Far down the pavement: still shut.
        let at = d.centre - d.inward * 5.0;
        run(&mut d, &cfg, Some(at), 2.0);
        assert_eq!(d.phase, DoorPhase::Closed);
    }

    #[test]
    fn it_opens_away_from_the_pilot_on_either_side() {
        let (mut d, cfg) = door(DoorKind::Front, 40.0);
        let outside = d.centre - d.inward * 1.0;
        run(&mut d, &cfg, Some(outside), 1.0);
        assert_eq!(d.phase, DoorPhase::Open);
        assert!(d.angle_deg > 0.0, "from the street it swings in: {}", d.angle_deg);
        let (mut d, cfg) = door(DoorKind::Front, 40.0);
        let inside = d.centre + d.inward * 1.2;
        run(&mut d, &cfg, Some(inside), 1.0);
        assert!(d.angle_deg < 0.0, "from the hall it swings out: {}", d.angle_deg);
        // Away: the swung leaf is farther from them than the shut one was.
        for (from, ang) in [(outside, cfg.door_open_deg), (inside, -d.out_max_deg)] {
            assert!(d.nearest(from, ang) > d.nearest(from, 0.0) - 1e-4);
        }
    }

    #[test]
    fn it_shuts_after_the_delay_once_the_pilot_has_gone() {
        let (mut d, cfg) = door(DoorKind::Lobby, 40.0);
        let at = d.centre - d.inward;
        run(&mut d, &cfg, Some(at), 1.0);
        assert_eq!(d.phase, DoorPhase::Open);
        // Gone: still open within the delay, shut after it and the swing.
        run(&mut d, &cfg, None, cfg.door_close_delay_s * 0.5);
        assert_eq!(d.phase, DoorPhase::Open);
        let mut shut = false;
        for _ in 0..600 {
            shut |= d.step(None, &cfg, 1.0 / 60.0) == Some(DoorEvent::Shut);
        }
        assert!(shut && d.phase == DoorPhase::Closed && d.angle_deg == 0.0);
    }

    #[test]
    fn it_never_closes_on_the_pilot() {
        let (mut d, cfg) = door(DoorKind::Front, 40.0);
        let at = d.centre - d.inward;
        run(&mut d, &cfg, Some(at), 1.0);
        // Standing in the leaf's arc inside: it stays open as long as they stand there.
        let l = d.leaves[0];
        let arc = l.hinge + (l.shut + d.inward).normalize() * (l.width_m * 0.6);
        assert!(d.in_arc(&body(arc)));
        run(&mut d, &cfg, Some(arc), 10.0);
        assert_eq!(d.phase, DoorPhase::Open, "open while they stand in its arc");
        // Forced to shut with them there: it stops short of them.
        d.phase = DoorPhase::Closing;
        for _ in 0..600 {
            d.swing_to(0.0, cfg.door_close_deg_s, 1.0 / 60.0, Some(&body(arc)));
        }
        assert!(d.nearest(arc, d.angle_deg) >= 0.35, "the leaf stopped at them");
    }

    #[test]
    fn a_shut_leaf_is_a_wall_and_an_open_one_is_not() {
        let (mut d, cfg) = door(DoorKind::Personnel, 40.0);
        let p = d.centre - d.inward * 0.1;
        let pushed = d.push_out(p, 0.35, 0.0, &cfg);
        assert!(
            (pushed - d.centre).dot(d.inward) <= SKIN_M * 0.5 - 0.35 + 1e-4,
            "kept outside: {pushed:?}"
        );
        let at = d.centre - d.inward;
        run(&mut d, &cfg, Some(at), 1.0);
        assert_eq!(d.push_out(d.centre, 0.35, 0.0, &cfg), d.centre, "the way through is clear");
        // A floor up, the door is not in the way.
        d.angle_deg = 0.0;
        assert_eq!(d.push_out(p, 0.35, 3.5, &cfg), p);
    }

    #[test]
    fn a_short_front_keeps_the_leaf_on_its_side_of_the_party_line() {
        let (d, _) = door(DoorKind::Front, 1.0);
        assert!(d.out_max_deg < 90.0);
        let (a, b) = d.leaf_at(0, -d.out_max_deg);
        for p in [a, b] {
            assert!((p - d.centre).dot(-d.inward) <= 0.5 - 0.05 + 1e-4);
        }
    }
}
