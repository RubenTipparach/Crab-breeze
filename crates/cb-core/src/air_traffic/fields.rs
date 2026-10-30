//! The airfields the traffic uses on the ground (openspec change `airfield-traffic`; owner, 2026-10-30: "All traffic
//! on all these sites will be fully dynamic"): every civil airport and every military base as a `Field`, with its
//! runway, its taxi network, its stands and the nodes an aircraft enters and leaves the runway by. A civil airport's
//! network is its page's, exported into its site table as marks and edges (`SiteTemplate::taxi_edges`); a base's is
//! laid out here from its site and runway, the old block-out base having no page of its own. Pure.

use crate::world::bases::{AirBase, Runway, RunwaySide};
use crate::world::sites::{PlacedSite, SiteKind};
use crate::world::{Rect, WorldPlan};
use glam::Vec2;

/// A civil airport's field id is this plus its index among the airports (a base's is its own id).
pub const CIVIL_FIELD_ID: u32 = 100;

/// A taxi network: points (x, z) with names, and the edges between them both ways.
#[derive(Clone, Debug, PartialEq)]
pub struct TaxiGraph {
    /// Every node's place (x, z).
    pub points: Vec<Vec2>,
    /// Every node's name.
    pub names: Vec<String>,
    adj: Vec<Vec<(usize, f32)>>,
}

impl TaxiGraph {
    /// The network of `nodes` (name, place) joined by `edges` (by name); an edge naming no node is left out.
    pub fn new(nodes: Vec<(String, Vec2)>, edges: &[(String, String)]) -> Self {
        let (names, points): (Vec<String>, Vec<Vec2>) = nodes.into_iter().unzip();
        let mut adj = vec![Vec::new(); points.len()];
        for (a, b) in edges {
            let (Some(i), Some(j)) = (names.iter().position(|n| n == a), names.iter().position(|n| n == b)) else {
                continue;
            };
            let d = points[i].distance(points[j]);
            adj[i].push((j, d));
            adj[j].push((i, d));
        }
        Self { points, names, adj }
    }

    /// The node named `name`.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// The shortest way from node `from` to node `to`, both ends included; ties go to the lower index (stable).
    pub fn route(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let n = self.points.len();
        if from >= n || to >= n {
            return None;
        }
        let mut dist = vec![f32::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        dist[from] = 0.0;
        // a few dozen nodes: a plain scan is enough, and its order is the index's
        while let Some(u) = (0..n)
            .filter(|&i| !done[i] && dist[i].is_finite())
            .min_by(|&a, &b| dist[a].total_cmp(&dist[b]).then(a.cmp(&b)))
        {
            if u == to {
                break;
            }
            done[u] = true;
            for &(v, w) in &self.adj[u] {
                if dist[u] + w < dist[v] {
                    dist[v] = dist[u] + w;
                    prev[v] = u;
                }
            }
        }
        if !dist[to].is_finite() {
            return None;
        }
        let mut path = vec![to];
        while let Some(&u) = path.last() {
            if u == from {
                break;
            }
            path.push(prev[u]);
        }
        path.reverse();
        Some(path)
    }

    /// The node among `among` nearest `p`.
    pub fn nearest(&self, p: Vec2, among: &[usize]) -> Option<usize> {
        among
            .iter()
            .copied()
            .min_by(|&a, &b| self.points[a].distance(p).total_cmp(&self.points[b].distance(p)).then(a.cmp(&b)))
    }

    /// The points along a route of nodes.
    pub fn line(&self, route: &[usize]) -> Vec<Vec2> {
        route.iter().map(|&i| self.points[i]).collect()
    }
}

/// A place an aircraft stands between flights.
#[derive(Clone, Debug, PartialEq)]
pub struct Stand {
    /// Its node.
    pub node: usize,
    /// The node it leaves by (pushed back or taxiing out); none for a pad.
    pub out: Option<usize>,
    /// The heading an aircraft stands at, radians.
    pub heading_rad: f32,
    /// The kind the page parks there (the stand's size): `AirTrafficConfig::kinds`' key.
    pub kind: String,
    /// A helicopter pad, left straight up.
    pub pad: bool,
}

/// An airfield.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Its id: a base's own, a civil airport's `CIVIL_FIELD_ID` plus its index.
    pub id: u32,
    /// A civil airport (else a military base).
    pub civil: bool,
    /// Its runway and ground, as the flight code knows a field (a civil airport's own stand-in).
    pub base: AirBase,
    /// Its taxi network.
    pub taxi: TaxiGraph,
    /// Its stands.
    pub stands: Vec<Stand>,
    /// The runway entries: at the `a` end (for a take-off toward `b`) and at the `b` end.
    pub entries: [usize; 2],
    /// The nodes a landed aircraft leaves the runway by (on the runway's centre line).
    pub exits: Vec<usize>,
}

impl Field {
    /// The entry to take off from along the strip running from `start` toward its far end: the entry nearer `start`.
    pub fn entry_for(&self, start: Vec2) -> usize {
        let [a, b] = self.entries;
        if self.taxi.points[a].distance(start) <= self.taxi.points[b].distance(start) {
            a
        } else {
            b
        }
    }

    /// The exit nearest `p`, ahead of it along `dir` if one is (a landing rolls on to the next exit).
    pub fn exit_for(&self, p: Vec2, dir: Vec2) -> Option<usize> {
        let ahead: Vec<usize> = self
            .exits
            .iter()
            .copied()
            .filter(|&e| (self.taxi.points[e] - p).dot(dir) >= -5.0)
            .collect();
        self.taxi.nearest(p, if ahead.is_empty() { &self.exits } else { &ahead })
    }
}

/// Every airfield of `plan`: the civil airports first (in their order), then the bases.
pub fn fields(plan: &WorldPlan) -> Vec<Field> {
    let airports = plan.sites.iter().filter(|s| s.template.kind == SiteKind::Airport);
    let mut out: Vec<Field> = airports
        .enumerate()
        .filter_map(|(i, s)| site_field(CIVIL_FIELD_ID + i as u32, s, None))
        .collect();
    // a base built as its page's site takes its network from it; a block-out base has one laid out
    out.extend(plan.bases.iter().map(|b| {
        b.site
            .and_then(|i| plan.sites.get(i as usize))
            .and_then(|s| site_field(b.id, s, Some(*b)))
            .unwrap_or_else(|| base_field(b))
    }));
    out
}

/// A civil airport as a field: its page's taxi network and stands, its main runway.
/// A site's field from its page's taxi network and stands, its main runway. `base` is the military base the site is
/// built as (its own record); none for a civil airport, which gets a stand-in.
fn site_field(id: u32, site: &PlacedSite, base: Option<AirBase>) -> Option<Field> {
    let t = &site.template;
    if t.taxi_edges.is_empty() {
        return None;
    }
    let (ra, rb) = (site.mark("rwy_a")?, site.mark("rwy_b")?);
    let (_, _, width_m, surface_m) = site.runways().into_iter().next()?;
    let nodes: Vec<(String, Vec2)> = t.marks.iter().map(|(k, v)| (k.clone(), site.to_world(*v))).collect();
    let taxi = TaxiGraph::new(nodes, &t.taxi_edges);
    let mut stands = Vec::new();
    for (i, p) in t.parked.iter().enumerate() {
        let Some(node) = taxi.index(&format!("stand_{i}")) else { continue };
        stands.push(Stand {
            node,
            out: taxi.index(&format!("out_{i}")),
            heading_rad: site.heading_rad() + p.heading_rad,
            kind: p.kind.clone(),
            pad: taxi.index(&format!("pad_{i}")).is_some(),
        });
    }
    let entries = [taxi.index("entry_w")?, taxi.index("entry_e")?];
    // every link onto the runway, and the airport's rapid exits, in the network's order
    let exits: Vec<usize> = (0..taxi.names.len())
        .filter(|&i| taxi.names[i].starts_with("entry_") || taxi.names[i].ends_with("_rwy"))
        .collect();
    let civil = base.is_none();
    let f = site.footprint();
    let base = base.unwrap_or(AirBase {
        id,
        centre: f.centre(),
        half_m: f.size() * 0.5,
        ground_m: surface_m,
        highway: 0,
        runway: Runway {
            a: ra,
            b: rb,
            width_m,
            side: RunwaySide::Front,
        },
        site: None,
        bays: None,
    });
    Some(Field {
        id,
        civil,
        base,
        taxi,
        stands,
        entries,
        exits,
    })
}

/// The kinds a base's stands hold, nearest the taxiway first: its alert pair and patrol fighters, a transport, and
/// its helicopters on pads (openspec change `military-base`).
pub const BASE_STANDS: [(&str, bool); 6] = [
    ("fighter", false),
    ("fighter", false),
    ("fighter", false),
    ("military_cargo", false),
    ("military_helicopter", true),
    ("military_helicopter", true),
];

/// A military base as a field: stands in a row along the site's edge facing its runway, an apron lane in front of
/// them, the taxiway across the gap to the runway's middle, and the runway back-tracked to either end.
pub fn base_field(b: &AirBase) -> Field {
    let r = b.runway;
    let site = Rect::centred(b.centre, b.half_m);
    let mid = (r.a + r.b) * 0.5;
    let gate = mid.clamp(site.min, site.max);
    let inward = (b.centre - gate).normalize_or(Vec2::Y);
    let along = inward.perp();
    let edge = mid; // the runway's middle, on its centre line
    let mut nodes = vec![
        ("entry_a".to_string(), r.a + r.dir() * 60.0),
        ("entry_b".to_string(), r.b - r.dir() * 60.0),
        ("runway_mid".to_string(), edge),
        ("apron".to_string(), gate + inward * 25.0),
    ];
    let mut edges = vec![
        ("entry_a".to_string(), "runway_mid".to_string()),
        ("runway_mid".to_string(), "entry_b".to_string()),
        ("runway_mid".to_string(), "apron".to_string()),
    ];
    let mut stands = Vec::new();
    let n = BASE_STANDS.len();
    for (i, (kind, pad)) in BASE_STANDS.iter().enumerate() {
        let k = i as f32 - (n as f32 - 1.0) * 0.5;
        let lane = gate + inward * 25.0 + along * k * 70.0;
        let at = lane + inward * 45.0;
        nodes.push((format!("lane_{i}"), lane));
        nodes.push((format!("stand_{i}"), at));
        edges.push((format!("stand_{i}"), format!("lane_{i}")));
        stands.push((i, *kind, *pad));
    }
    // the apron lane, in order along the edge, joined at its middle to the taxiway
    let mut lane: Vec<(f32, String)> = (0..n)
        .map(|i| ((i as f32 - (n as f32 - 1.0) * 0.5) * 70.0, format!("lane_{i}")))
        .collect();
    lane.push((0.0, "apron".to_string()));
    lane.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in lane.windows(2) {
        edges.push((w[0].1.clone(), w[1].1.clone()));
    }
    let taxi = TaxiGraph::new(nodes, &edges);
    let facing = crate::units::heading_of(glam::Vec3::new(-inward.x, 0.0, -inward.y));
    let stands = stands
        .into_iter()
        .filter_map(|(i, kind, pad)| {
            Some(Stand {
                node: taxi.index(&format!("stand_{i}"))?,
                out: if pad { None } else { taxi.index(&format!("lane_{i}")) },
                heading_rad: facing,
                kind: kind.to_string(),
                pad,
            })
        })
        .collect();
    let entries = [taxi.index("entry_a").unwrap_or(0), taxi.index("entry_b").unwrap_or(1)];
    let exits = ["runway_mid", "entry_a", "entry_b"].iter().filter_map(|k| taxi.index(k)).collect();
    Field {
        id: b.id,
        civil: false,
        base: *b,
        taxi,
        stands,
        entries,
        exits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> TaxiGraph {
        let n = |s: &str, x: f32, z: f32| (s.to_string(), Vec2::new(x, z));
        let e = |a: &str, b: &str| (a.to_string(), b.to_string());
        TaxiGraph::new(
            vec![n("a", 0.0, 0.0), n("b", 100.0, 0.0), n("c", 100.0, 100.0), n("d", 0.0, 100.0)],
            &[e("a", "b"), e("b", "c"), e("a", "d"), e("d", "c"), e("a", "nowhere")],
        )
    }

    #[test]
    fn a_route_is_the_shortest_way_and_ties_keep_the_lower_index() {
        let g = graph();
        assert_eq!(
            g.route(0, 2),
            Some(vec![0, 1, 2]),
            "both ways are 200 m: through b, the lower index"
        );
        assert_eq!(g.route(3, 1), Some(vec![3, 0, 1]));
        assert_eq!(g.route(2, 2), Some(vec![2]));
    }

    #[test]
    fn an_edge_naming_no_node_is_left_out() {
        let g = graph();
        assert_eq!(g.index("nowhere"), None);
        assert_eq!(g.route(0, 9), None);
    }

    #[test]
    fn a_base_has_stands_a_lane_and_a_way_onto_both_ends_of_its_runway() {
        let cfg = crate::world::BasesConfig::default();
        let c = Vec2::new(1000.0, 2000.0);
        let b = AirBase {
            id: 3,
            centre: c,
            half_m: cfg.half_m,
            ground_m: 40.0,
            highway: 0,
            runway: Runway::beside(c, cfg.half_m, RunwaySide::Front, 2000.0, 0.0, &cfg),
            site: None,
            bays: None,
        };
        let f = base_field(&b);
        assert_eq!(f.stands.len(), BASE_STANDS.len());
        for s in &f.stands {
            assert!(
                Rect::centred(c, cfg.half_m).distance(f.taxi.points[s.node]) <= 0.0,
                "the stand is on the base"
            );
            for e in f.entries {
                assert!(f.taxi.route(s.node, e).is_some(), "a way from every stand to both ends");
            }
        }
        assert!(f.stands.iter().filter(|s| s.pad).count() >= 2);
    }
}
