//! The lane graph traffic drives on (openspec change `car-traffic`, design section 1): every city
//! street and every highway cut into one-way lanes, joined at junctions by short connector lanes that
//! go straight on, left and right. Pure and built once from the world plan: the same cities and
//! highways always give the same graph, ids in plan order. It lives with the world because it is the
//! roads' own shape; `crate::traffic` drives cars along it.
//!
//! Traffic keeps to the right. A lane runs along its road's centre line at a fixed offset to its
//! right, from one junction to the next, stopping short of each junction by the junction's half
//! width; the connectors span the junction box between the lanes.

use super::layout::{City, Highway, Rect};
use crate::data::{Validate, positive};
use glam::Vec2;
use serde::Deserialize;

/// How the lanes sit on their roads, loaded with the traffic data (`data/world/traffic.ron`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LaneLayout {
    /// Each lane of an ordinary street, by its offset to the right of the centre line, metres
    /// (one lane each way).
    pub street_offsets_m: Vec<f32>,
    /// The same for an arterial (two lanes each way).
    pub arterial_offsets_m: Vec<f32>,
    /// The same for a highway (two lanes each way).
    pub highway_offsets_m: Vec<f32>,
    /// Two connectors across one junction conflict when they pass within this of each other, metres:
    /// two cars may not be on them at once.
    pub conflict_m: f32,
    /// Parked cars stand this far in from the road's edge, metres (their middle).
    pub parked_in_from_edge_m: f32,
}

impl Default for LaneLayout {
    fn default() -> Self {
        Self {
            street_offsets_m: vec![3.5],
            arterial_offsets_m: vec![3.5, 7.0],
            highway_offsets_m: vec![3.5, 7.0],
            conflict_m: 2.5,
            parked_in_from_edge_m: 2.5,
        }
    }
}

impl Validate for LaneLayout {
    fn validate(&self) -> Result<(), String> {
        for (name, list) in [
            ("street_offsets_m", &self.street_offsets_m),
            ("arterial_offsets_m", &self.arterial_offsets_m),
            ("highway_offsets_m", &self.highway_offsets_m),
        ] {
            if list.is_empty() {
                return Err(format!("`{name}` needs at least one lane"));
            }
            for &o in list {
                positive(name, o)?;
            }
            if list.windows(2).any(|w| w[1] <= w[0]) {
                return Err(format!("`{name}` must run from the centre line outward, got {list:?}"));
            }
        }
        positive("conflict_m", self.conflict_m)?;
        positive("parked_in_from_edge_m", self.parked_in_from_edge_m)
    }
}

/// What a lane is part of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LaneKind {
    /// An ordinary city street.
    Street,
    /// A wide city arterial.
    Arterial,
    /// A highway between cities.
    Highway,
    /// A path across a junction, from one road's lane to another's.
    Connector,
}

impl LaneKind {
    /// True for a city street or arterial.
    pub fn in_city(self) -> bool {
        matches!(self, Self::Street | Self::Arterial)
    }
}

/// A polyline with the distance along it to each point.
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline {
    /// Points (x, z), in order.
    pub points: Vec<Vec2>,
    /// Distance from the first point to each point, metres.
    pub along_m: Vec<f32>,
}

impl Polyline {
    /// A polyline through `points` (at least two).
    pub fn new(points: Vec<Vec2>) -> Self {
        let mut along_m = Vec::with_capacity(points.len());
        let mut d = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                d += points[i - 1].distance(*p);
            }
            along_m.push(d);
        }
        Self { points, along_m }
    }

    /// Length, metres.
    pub fn length_m(&self) -> f32 {
        self.along_m.last().copied().unwrap_or(0.0)
    }

    /// The segment `s_m` along the line lies on, and the share of it (clamped to the line's ends).
    pub fn locate(&self, s_m: f32) -> (usize, f32) {
        let n = self.points.len();
        if n < 2 {
            return (0, 0.0);
        }
        let s = s_m.clamp(0.0, self.length_m());
        let i = self.along_m.partition_point(|&a| a <= s).saturating_sub(1).min(n - 2);
        let len = (self.along_m[i + 1] - self.along_m[i]).max(1e-6);
        (i, ((s - self.along_m[i]) / len).clamp(0.0, 1.0))
    }

    /// The point and the direction `s_m` along the line (clamped to its ends).
    pub fn at(&self, s_m: f32) -> (Vec2, Vec2) {
        let n = self.points.len();
        if n < 2 {
            return (self.points.first().copied().unwrap_or(Vec2::ZERO), Vec2::NEG_Y);
        }
        let s = s_m.clamp(0.0, self.length_m());
        let i = self.along_m.partition_point(|&a| a <= s).saturating_sub(1).min(n - 2);
        let len = (self.along_m[i + 1] - self.along_m[i]).max(1e-6);
        let u = ((s - self.along_m[i]) / len).clamp(0.0, 1.0);
        let dir = (self.points[i + 1] - self.points[i]).normalize_or(Vec2::NEG_Y);
        (self.points[i].lerp(self.points[i + 1], u), dir)
    }

    /// The part of the line from `s0_m` to `s1_m` along it.
    pub fn cut(&self, s0_m: f32, s1_m: f32) -> Self {
        let mut pts = vec![self.at(s0_m).0];
        for (p, a) in self.points.iter().zip(&self.along_m) {
            if *a > s0_m && *a < s1_m {
                pts.push(*p);
            }
        }
        pts.push(self.at(s1_m).0);
        Self::new(pts)
    }

    /// The line moved `offset_m` to its right (positive) at every point.
    pub fn offset(&self, offset_m: f32) -> Self {
        let n = self.points.len();
        let pts = (0..n)
            .map(|i| {
                let a = self.points[i.saturating_sub(1)];
                let b = self.points[(i + 1).min(n - 1)];
                let d = (b - a).normalize_or(Vec2::NEG_Y);
                self.points[i] + right_of(d) * offset_m
            })
            .collect();
        Self::new(pts)
    }

    /// The same line, run the other way.
    pub fn reversed(&self) -> Self {
        Self::new(self.points.iter().rev().copied().collect())
    }

    /// The least distance between this line's points and `other`'s segments, metres.
    fn nearest_to(&self, other: &Polyline) -> f32 {
        let mut best = f32::INFINITY;
        for p in &self.points {
            for w in other.points.windows(2) {
                best = best.min(point_segment(*p, w[0], w[1]));
            }
        }
        best
    }
}

/// The right of a direction (x, z) seen from above: forward -z has +x on its right.
pub fn right_of(d: Vec2) -> Vec2 {
    Vec2::new(-d.y, d.x)
}

fn point_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(p)
}

/// One one-way lane.
#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    /// Stable id: its index in `LaneGraph::lanes`.
    pub id: u32,
    /// What it is part of.
    pub kind: LaneKind,
    /// Its centre line, in the direction of travel.
    pub line: Polyline,
    /// The junction it leaves.
    pub from_node: u32,
    /// The junction it runs into.
    pub to_node: u32,
    /// The road it belongs to (`LaneGraph::roads`); a connector's is its node's first road.
    pub road: u32,
    /// Which lane of its road and direction, 0 nearest the centre line.
    pub slot: u8,
    /// Where it can go at its end: connectors across the junction for a road lane, the one road
    /// lane a connector leads into.
    pub out: Vec<u32>,
    /// A connector: how far it turns, radians (left positive); 0 for a road lane.
    pub turn_rad: f32,
    /// A connector: the other connectors across the same junction a car on this one would meet.
    pub conflicts: Vec<u32>,
    /// A parked car's sideways offset from this lane, right positive, metres; `None` where nobody
    /// parks (highways, the inner lanes of an arterial, connectors).
    pub parking_m: Option<f32>,
    /// A road lane running into a junction with traffic lights: which of the junction's two phases its light shows
    /// green in (survey W39); `None` where junctions take turns by arrival, and for connectors.
    pub phase: Option<u8>,
}

impl Lane {
    /// Length, metres.
    pub fn length_m(&self) -> f32 {
        self.line.length_m()
    }
}

/// A junction: where roads meet.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Stable id: its index in `LaneGraph::nodes`.
    pub id: u32,
    /// Where (x, z).
    pub at: Vec2,
    /// The city it is in.
    pub city: u32,
    /// The connectors across it.
    pub connectors: Vec<u32>,
    /// Lanes stop this far short of its middle, metres.
    pub radius_m: f32,
    /// 1980s traffic lights stand here (survey W39, owner 2026-10-26: "Lights on the arterials"): where an arterial
    /// meets at least two other roads. Every other junction takes turns by arrival.
    pub signal: bool,
}

/// One road between two junctions: a city street's block-long piece, or a whole highway.
#[derive(Clone, Debug, PartialEq)]
pub struct Road {
    /// Its centre line, from `nodes[0]` to `nodes[1]`.
    pub line: Polyline,
    /// The junctions at each end.
    pub nodes: [u32; 2],
    /// Paved width, metres.
    pub width_m: f32,
    /// What kind of road.
    pub kind: LaneKind,
}

/// Every lane on the island.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LaneGraph {
    /// Lanes, by id.
    pub lanes: Vec<Lane>,
    /// Junctions, by id.
    pub nodes: Vec<Node>,
    /// Roads, in plan order: each city's streets, then the highways.
    pub roads: Vec<Road>,
}

/// Centres closer than this are the same street line.
const SAME_LINE_M: f32 = 0.5;

/// A city's street lines: centre and width, in order along the axis.
fn street_axes(city: &City) -> (Vec<(f32, f32)>, Vec<(f32, f32)>) {
    let r: Rect = city.rect();
    let mut xs: Vec<(f32, f32)> = Vec::new();
    let mut zs: Vec<(f32, f32)> = Vec::new();
    let put = |list: &mut Vec<(f32, f32)>, c: f32, w: f32| {
        if !list.iter().any(|(x, _)| (x - c).abs() < SAME_LINE_M) {
            list.push((c, w));
        }
    };
    for s in &city.streets {
        let full_width = (s.rect.min.x - r.min.x).abs() < SAME_LINE_M && (s.rect.max.x - r.max.x).abs() < SAME_LINE_M;
        if full_width {
            // runs along x: a line at its centre z
            put(&mut zs, s.rect.centre().y, s.width_m);
        } else {
            put(&mut xs, s.rect.centre().x, s.width_m);
        }
    }
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    zs.sort_by(|a, b| a.0.total_cmp(&b.0));
    (xs, zs)
}

impl LaneGraph {
    /// The graph for these cities and highways: each city's street grid, junctions where its street
    /// lines cross, every highway joined at each end to the nearest junction of its city (a site's access road at its
    /// far end to a gate node of its own).
    pub fn build(cities: &[City], highways: &[Highway], layout: &LaneLayout, arterial_width_m: f32) -> Self {
        let mut g = LaneGraph::default();
        // the junction nearest each point, per city, for joining highways
        let mut city_nodes: Vec<(u32, Vec<u32>)> = Vec::new();
        for city in cities {
            let (xs, zs) = street_axes(city);
            let first = g.nodes.len() as u32;
            let nx = xs.len();
            let mut ids = Vec::new();
            for (j, &(z, wz)) in zs.iter().enumerate() {
                for (i, &(x, wx)) in xs.iter().enumerate() {
                    let id = first + (j * nx + i) as u32;
                    ids.push(id);
                    g.nodes.push(Node {
                        id,
                        at: Vec2::new(x, z),
                        city: city.id,
                        connectors: Vec::new(),
                        radius_m: wx.max(wz) * 0.5,
                        signal: false,
                    });
                }
            }
            let kind = |w: f32| {
                if w >= arterial_width_m - 1e-3 {
                    LaneKind::Arterial
                } else {
                    LaneKind::Street
                }
            };
            // along x: each z line, from junction to junction
            for (j, &(_, w)) in zs.iter().enumerate() {
                for i in 0..nx.saturating_sub(1) {
                    let a = first + (j * nx + i) as u32;
                    g.add_road(a, a + 1, None, w, kind(w));
                }
            }
            // along z: each x line
            for (i, &(_, w)) in xs.iter().enumerate() {
                for j in 0..zs.len().saturating_sub(1) {
                    let a = first + (j * nx + i) as u32;
                    g.add_road(a, a + nx as u32, None, w, kind(w));
                }
            }
            city_nodes.push((city.id, ids));
        }
        for h in highways {
            let nearest = |city: u32, p: Vec2| {
                city_nodes.iter().find(|(c, _)| *c == city).and_then(|(_, ids)| {
                    ids.iter()
                        .copied()
                        .min_by(|&a, &b| g.nodes[a as usize].at.distance(p).total_cmp(&g.nodes[b as usize].at.distance(p)))
                })
            };
            let near = nearest(h.cities[0], h.ends[0]);
            // A site's access road ends at a gate of its own (`roads::SITE_END`): a dead end cars turn round at.
            let far = if h.cities[1] == super::roads::SITE_END {
                near.map(|_| {
                    let id = g.nodes.len() as u32;
                    g.nodes.push(Node {
                        id,
                        at: h.ends[1],
                        city: h.cities[0],
                        connectors: Vec::new(),
                        radius_m: h.width_m * 0.5,
                        signal: false,
                    });
                    id
                })
            } else {
                nearest(h.cities[1], h.ends[1])
            };
            let (Some(a), Some(b)) = (near, far) else {
                continue;
            };
            let mut line = vec![g.nodes[a as usize].at];
            line.extend(h.path.iter().copied());
            if h.cities[1] != super::roads::SITE_END {
                line.push(g.nodes[b as usize].at);
            }
            g.add_road(a, b, Some(line), h.width_m, LaneKind::Highway);
        }
        g.lay_lanes(layout);
        g.join(layout);
        g.signal();
        g
    }

    /// Puts traffic lights where an arterial meets at least two other roads (survey W39), and gives each road lane
    /// into such a junction its phase: 0 for the roads running within 45 degrees of the junction's first arterial
    /// (both ways), 1 for the rest. The two phases take the junction in turn (`traffic::light`).
    fn signal(&mut self) {
        for n in 0..self.nodes.len() {
            // every road at this junction, and the way it leaves it
            let mut here: Vec<(u32, Vec2, LaneKind)> = Vec::new();
            for (ri, r) in self.roads.iter().enumerate() {
                let pts = &r.line.points;
                let away = if r.nodes[0] == n as u32 {
                    pts[1] - pts[0]
                } else if r.nodes[1] == n as u32 {
                    pts[pts.len() - 2] - pts[pts.len() - 1]
                } else {
                    continue;
                };
                here.push((ri as u32, away.normalize_or_zero(), r.kind));
            }
            let Some(&(_, axis, _)) = here.iter().find(|h| h.2 == LaneKind::Arterial) else {
                continue;
            };
            if here.len() < 3 {
                continue;
            }
            self.nodes[n].signal = true;
            let cos45 = std::f32::consts::FRAC_1_SQRT_2;
            for lane in self.lanes.iter_mut() {
                if lane.kind == LaneKind::Connector || lane.to_node != n as u32 {
                    continue;
                }
                let Some(h) = here.iter().find(|h| h.0 == lane.road) else {
                    continue;
                };
                lane.phase = Some(if h.1.dot(axis).abs() >= cos45 { 0 } else { 1 });
            }
        }
    }

    fn add_road(&mut self, a: u32, b: u32, line: Option<Vec<Vec2>>, width_m: f32, kind: LaneKind) {
        let pts = line.unwrap_or_else(|| vec![self.nodes[a as usize].at, self.nodes[b as usize].at]);
        self.roads.push(Road {
            line: Polyline::new(pts),
            nodes: [a, b],
            width_m,
            kind,
        });
    }

    /// Cuts every road into its lanes, each way, short of the junctions at both ends.
    fn lay_lanes(&mut self, layout: &LaneLayout) {
        // A highway's junction is as wide as the widest road into it.
        for r in &self.roads {
            for n in r.nodes {
                let node = &mut self.nodes[n as usize];
                node.radius_m = node.radius_m.max(r.width_m * 0.5);
            }
        }
        for (ri, road) in self.roads.iter().enumerate() {
            let offsets = match road.kind {
                LaneKind::Street => &layout.street_offsets_m,
                LaneKind::Arterial => &layout.arterial_offsets_m,
                _ => &layout.highway_offsets_m,
            };
            let [a, b] = road.nodes;
            let (ta, tb) = (self.nodes[a as usize].radius_m, self.nodes[b as usize].radius_m);
            let len = road.line.length_m();
            if len <= ta + tb + 1.0 {
                continue;
            }
            let trimmed = road.line.cut(ta, len - tb);
            for (dir, line, from, to) in [(0, trimmed.clone(), a, b), (1, trimmed.reversed(), b, a)] {
                let _ = dir;
                for (slot, &o) in offsets.iter().enumerate() {
                    let outermost = slot + 1 == offsets.len();
                    let parking_m = (road.kind.in_city() && outermost).then(|| {
                        let edge = road.width_m * 0.5 - layout.parked_in_from_edge_m;
                        (edge - o).max(0.0)
                    });
                    let id = self.lanes.len() as u32;
                    self.lanes.push(Lane {
                        id,
                        kind: road.kind,
                        line: line.offset(o),
                        from_node: from,
                        to_node: to,
                        road: ri as u32,
                        slot: slot as u8,
                        out: Vec::new(),
                        turn_rad: 0.0,
                        conflicts: Vec::new(),
                        parking_m,
                        phase: None,
                    });
                }
            }
        }
    }

    /// Joins the lanes at every junction: from each lane into a lane of every other road there, the
    /// same slot or the nearest there is, by a curve across the junction.
    fn join(&mut self, layout: &LaneLayout) {
        let road_lanes = self.lanes.len();
        let mut into: Vec<Vec<u32>> = vec![Vec::new(); self.nodes.len()];
        let mut from: Vec<Vec<u32>> = vec![Vec::new(); self.nodes.len()];
        for l in &self.lanes {
            into[l.to_node as usize].push(l.id);
            from[l.from_node as usize].push(l.id);
        }
        for n in 0..self.nodes.len() {
            let roads_here: std::collections::BTreeSet<u32> = from[n].iter().map(|&l| self.lanes[l as usize].road).collect();
            let mut made = Vec::new();
            for &li in &into[n] {
                let l = self.lanes[li as usize].clone();
                // the other roads' outgoing lanes, by road; back the way it came only at a dead end
                for &road in &roads_here {
                    if road == l.road && roads_here.len() > 1 {
                        continue;
                    }
                    let outs: Vec<&Lane> = from[n]
                        .iter()
                        .map(|&o| &self.lanes[o as usize])
                        .filter(|o| o.road == road)
                        .collect();
                    let slots = outs.iter().map(|o| o.slot).max().unwrap_or(0);
                    let Some(target) = outs.iter().find(|o| o.slot == l.slot.min(slots)) else {
                        continue;
                    };
                    let (p0, d0) = l.line.at(l.length_m());
                    let (p1, d1) = target.line.at(0.0);
                    let turn = d0.perp_dot(d1).atan2(d0.dot(d1));
                    // a cubic from the lane's end to the next lane's start, leaving and arriving along each
                    let h = p0.distance(p1) * 0.4;
                    let (c0, c1) = (p0 + d0 * h, p1 - d1 * h);
                    let pts: Vec<Vec2> = (0..=8)
                        .map(|k| {
                            let t = k as f32 / 8.0;
                            let u = 1.0 - t;
                            p0 * (u * u * u) + c0 * (3.0 * u * u * t) + c1 * (3.0 * u * t * t) + p1 * (t * t * t)
                        })
                        .collect();
                    made.push((li, target.id, turn, Polyline::new(pts)));
                }
            }
            for (li, target, turn, line) in made {
                let id = self.lanes.len() as u32;
                self.lanes[li as usize].out.push(id);
                self.nodes[n].connectors.push(id);
                self.lanes.push(Lane {
                    id,
                    kind: LaneKind::Connector,
                    line,
                    from_node: n as u32,
                    to_node: n as u32,
                    road: self.lanes[li as usize].road,
                    slot: 0,
                    out: vec![target],
                    turn_rad: turn,
                    conflicts: Vec::new(),
                    parking_m: None,
                    phase: None,
                });
            }
        }
        // Two connectors at one junction conflict when they come within `conflict_m` or lead into
        // the same lane.
        for n in 0..self.nodes.len() {
            let cs = self.nodes[n].connectors.clone();
            for (i, &a) in cs.iter().enumerate() {
                for &b in &cs[i + 1..] {
                    let (la, lb) = (&self.lanes[a as usize], &self.lanes[b as usize]);
                    let close = la.line.nearest_to(&lb.line).min(lb.line.nearest_to(&la.line)) < layout.conflict_m;
                    if close || la.out == lb.out {
                        self.lanes[a as usize].conflicts.push(b);
                        self.lanes[b as usize].conflicts.push(a);
                    }
                }
            }
        }
        debug_assert!(self.lanes[..road_lanes].iter().all(|l| l.kind != LaneKind::Connector));
    }

    /// The road lanes (not connectors), in id order.
    pub fn road_lanes(&self) -> impl Iterator<Item = &Lane> {
        self.lanes.iter().filter(|l| l.kind != LaneKind::Connector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SimRng;
    use crate::world::WorldConfig;
    use crate::world::layout::lay_out_city;

    /// Two small cities 3 km apart with a straight highway between them.
    pub(crate) fn two_towns() -> (Vec<City>, Vec<Highway>, WorldConfig) {
        let cfg = WorldConfig::default();
        let mut rng = SimRng::new(7);
        let a = lay_out_city(&mut rng, 0, Vec2::new(0.0, 0.0), 300.0, 10.0, &cfg);
        let b = lay_out_city(&mut rng, 1, Vec2::new(3000.0, 0.0), 250.0, 10.0, &cfg);
        let h = Highway::straight(
            [0, 1],
            Vec2::new(300.0, 0.0),
            Vec2::new(2750.0, 0.0),
            [10.0, 10.0],
            cfg.highway_width_m,
            50.0,
            cfg.world_size_m,
        );
        (vec![a, b], vec![h], cfg)
    }

    fn graph() -> LaneGraph {
        let (c, h, cfg) = two_towns();
        LaneGraph::build(&c, &h, &LaneLayout::default(), cfg.arterial_width_m)
    }

    #[test]
    fn every_lane_has_a_way_out() {
        let g = graph();
        assert!(g.lanes.len() > 100, "{} lanes", g.lanes.len());
        for l in &g.lanes {
            assert!(!l.out.is_empty(), "lane {} ({:?}) dead-ends", l.id, l.kind);
            for &o in &l.out {
                assert!((o as usize) < g.lanes.len());
            }
        }
    }

    #[test]
    fn a_connector_leads_from_a_road_lane_into_another_roads_lane() {
        let g = graph();
        for c in g.lanes.iter().filter(|l| l.kind == LaneKind::Connector) {
            assert_eq!(c.out.len(), 1);
            let next = &g.lanes[c.out[0] as usize];
            assert_ne!(next.kind, LaneKind::Connector);
            let from = g.lanes.iter().find(|l| l.out.contains(&c.id)).expect("a lane feeds it");
            assert_ne!(from.road, next.road, "no U-turns where another road leads on");
            assert!(from.line.at(from.length_m()).0.distance(c.line.points[0]) < 1e-3);
            assert!(next.line.points[0].distance(*c.line.points.last().unwrap()) < 1e-3);
        }
    }

    #[test]
    fn lanes_lie_on_the_road_surface() {
        let (cities, highways, cfg) = two_towns();
        let g = LaneGraph::build(&cities, &highways, &LaneLayout::default(), cfg.arterial_width_m);
        for l in g.road_lanes() {
            for p in &l.line.points {
                let ok = if l.kind.in_city() {
                    cities.iter().flat_map(|c| &c.streets).any(|s| s.rect.distance(*p) < 0.01)
                } else {
                    // the highway, plus the short stub from its end on the city's edge to the junction
                    let road = &g.roads[l.road as usize];
                    let d = road
                        .line
                        .points
                        .windows(2)
                        .map(|w| point_segment(*p, w[0], w[1]))
                        .fold(f32::INFINITY, f32::min);
                    d <= road.width_m * 0.5 && highways.iter().any(|h| h.path.iter().all(|q| road.line.points.contains(q)))
                };
                assert!(ok, "lane {} point {p} is off the road", l.id);
            }
        }
    }

    #[test]
    fn traffic_keeps_to_the_right() {
        let g = graph();
        for l in g.road_lanes().filter(|l| l.kind == LaneKind::Street) {
            let road = &g.roads[l.road as usize];
            let (p, d) = l.line.at(l.length_m() * 0.5);
            let (rp, _) = road.line.at(road.line.length_m() * 0.5);
            assert!((p - rp).dot(right_of(d)) > 3.0, "lane {} sits right of its centre line", l.id);
        }
    }

    #[test]
    fn crossing_connectors_conflict_and_the_relation_is_mutual() {
        let g = graph();
        let mut any = false;
        for c in g.lanes.iter().filter(|l| l.kind == LaneKind::Connector) {
            for &o in &c.conflicts {
                assert!(g.lanes[o as usize].conflicts.contains(&c.id));
                any = true;
            }
        }
        assert!(any);
    }

    #[test]
    fn the_highway_joins_both_towns() {
        let g = graph();
        let hw: Vec<&Lane> = g.road_lanes().filter(|l| l.kind == LaneKind::Highway).collect();
        assert_eq!(hw.len(), 4, "two lanes each way");
        for l in hw {
            assert!(l.length_m() > 2000.0);
            assert_ne!(g.nodes[l.from_node as usize].city, g.nodes[l.to_node as usize].city);
        }
    }

    #[test]
    fn the_same_plan_gives_the_same_graph() {
        assert_eq!(graph(), graph());
    }
}
