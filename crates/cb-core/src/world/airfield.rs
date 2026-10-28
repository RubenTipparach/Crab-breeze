//! An air base's airfield laid out round its runway (openspec change `airport-layout`, after the owner's references in
//! `docs/design/references/airports/`): the parallel taxiway, its links to the runway and their holding positions, the
//! apron entries, the terminal with its stands and boarding bridges, the fire station, fuel farm, car park, helicopter
//! stand, windsock, PAPI, approach lights and the fence. Pure and deterministic: the one place every piece's footprint
//! is decided. The game draws it (`cb-app` `air_bases`) and the tests read it.
//!
//! Strip pieces are placed in the runway's frame: `u` along the runway from `a` to `b`, `v` across it toward the site.
//! Site pieces are placed in the site's own frame (x right, z toward the hangars), the way the hangars and the tower
//! already are (`AirBase::hangars`, `AirBase::tower`).

use super::bases::{AirBase, BasesConfig, Runway, RunwaySide};
use super::layout::Rect;
use glam::Vec2;

/// The parallel taxiway's paved width, metres.
pub const TAXIWAY_W_M: f32 = 23.0;
/// The parallel taxiway's edge stands this far off the site's edge, metres.
pub const TAXIWAY_OFF_SITE_M: f32 = 15.0;
/// A holding position stands this far before the runway's edge, metres.
pub const HOLD_BEFORE_M: f32 = 50.0;
/// An apron entry's width, metres.
pub const ENTRY_W_M: f32 = 30.0;
/// How far the strip reaches past the runway's far edge (the PAPI stand in it), metres.
pub const STRIP_OUT_M: f32 = 40.0;
/// The fence stands this far inside the airfield's outline, metres.
pub const FENCE_IN_M: f32 = 3.0;
/// The threshold sits this far in from each end; the paving before it is the blast pad, metres.
pub const BLAST_PAD_M: f32 = 60.0;
/// The aiming point marks stand this far past each threshold, metres.
pub const AIMING_POINT_M: f32 = 400.0;
/// Approach lights reach this far out past each end, a post every `APPROACH_STEP_M`, metres.
pub const APPROACH_M: f32 = 300.0;
/// Between approach light posts, metres.
pub const APPROACH_STEP_M: f32 = 30.0;

/// What a building on the site is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The passenger terminal, its face toward the stands.
    Terminal,
    /// The airfield's fire station, its doors toward the apron.
    FireStation,
    /// One tank of the fuel farm (its footprint is the tank's square).
    FuelTank,
    /// The car park.
    CarPark,
    /// The helicopter stand, a painted circle.
    Helipad,
}

/// A building's footprint and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Building {
    /// What it is.
    pub kind: Kind,
    /// Its footprint (x, z).
    pub rect: Rect,
    /// Its height over the apron, metres (0 for what is painted on it).
    pub height_m: f32,
}

/// An aircraft stand in front of the terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stand {
    /// Its painted square (x, z).
    pub rect: Rect,
    /// The way a parked aircraft's nose points: toward the terminal (x, z), a unit vector.
    pub nose: Vec2,
    /// Its boarding bridge, from the terminal's face toward the stand (x, z).
    pub bridge: Rect,
}

/// A base's airfield.
#[derive(Clone, Debug, PartialEq)]
pub struct Airfield {
    /// The flattened strip between the site and past the runway's far edge, the runway's length.
    pub strip: Rect,
    /// The parallel taxiway.
    pub taxiway: Rect,
    /// The links from the taxiway to the runway: at each end and the middle.
    pub links: Vec<Rect>,
    /// A holding position on each link: a line across it (two ends, x, z).
    pub holds: Vec<[Vec2; 2]>,
    /// The apron entries from the taxiway into the site.
    pub entries: Vec<Rect>,
    /// The buildings on the site.
    pub buildings: Vec<Building>,
    /// The terminal's stands.
    pub stands: Vec<Stand>,
    /// The windsock's mast (x, z).
    pub windsock: Vec2,
    /// The PAPI boxes at each end, nearest the runway first (x, z).
    pub papi: [[Vec2; 4]; 2],
    /// The approach light posts out past each end (x, z), nearest the runway first.
    pub approach: [Vec<Vec2>; 2],
    /// The runway designators read from each end: the heading in tens of degrees, 1 to 36.
    pub designators: [u32; 2],
    /// The fence's outline, corner after corner, closed (x, z).
    pub fence: Vec<Vec2>,
}

impl Runway {
    /// Unit vector across the runway toward its site (x, z).
    pub fn toward_site(&self) -> Vec2 {
        match self.side {
            RunwaySide::Front => Vec2::Y,
            RunwaySide::Back => Vec2::NEG_Y,
            RunwaySide::West => Vec2::X,
            RunwaySide::East => Vec2::NEG_X,
        }
    }

    /// The point `u` metres along the centre line from `a` and `v` across it toward the site (x, z).
    pub fn at(&self, u: f32, v: f32) -> Vec2 {
        self.a + self.dir() * u + self.toward_site() * v
    }

    /// The rectangle between `u0..u1` along and `v0..v1` across (the runway's frame).
    pub fn span(&self, u0: f32, u1: f32, v0: f32, v1: f32) -> Rect {
        let (p, q) = (self.at(u0, v0), self.at(u1, v1));
        Rect {
            min: p.min(q),
            max: p.max(q),
        }
    }

    /// The airfield strip: from `STRIP_OUT_M` past the runway's far edge to the site's edge, `gap_m` from the runway's
    /// near edge, the runway's length. Flattened with the site and checked dry and clear when the runway is placed.
    pub fn strip(&self) -> Rect {
        let w = self.width_m * 0.5;
        self.span(0.0, self.length_m(), -w - STRIP_OUT_M, w + self.gap_m)
    }

    /// Points over the strip, every `step_m` along and across it.
    pub fn strip_samples(&self, step_m: f32) -> Vec<Vec2> {
        let w = self.width_m * 0.5;
        let (v0, v1) = (-w - STRIP_OUT_M, w + self.gap_m);
        let nu = (self.length_m() / step_m.max(1.0)).ceil().max(1.0) as usize;
        let nv = ((v1 - v0) / step_m.max(1.0)).ceil().max(1.0) as usize;
        (0..=nu)
            .flat_map(|i| (0..=nv).map(move |j| self.at(self.length_m() * i as f32 / nu as f32, v0 + (v1 - v0) * j as f32 / nv as f32)))
            .collect()
    }

    /// The designator read from the `a` end and from the `b` end: the heading flown from each, in tens of degrees,
    /// 1 to 36 (+z is north, x east, so a runway flown from west to east reads 09 at its `a` end).
    pub fn designators(&self) -> [u32; 2] {
        let d = self.dir();
        // Bearing clockwise from north (-z): east (+x) is 90.
        let bearing = d.x.atan2(-d.y).to_degrees().rem_euclid(360.0);
        let read = |deg: f32| {
            let n = (deg / 10.0).round() as u32 % 36;
            if n == 0 { 36 } else { n }
        };
        [read(bearing), read(bearing + 180.0)]
    }
}

/// Lays out `base`'s airfield.
pub fn layout(base: &AirBase, cfg: &BasesConfig) -> Airfield {
    let rw = &base.runway;
    let site = base.rect();
    let (len, w, gap) = (rw.length_m(), rw.width_m * 0.5, rw.gap_m);
    let edge_v = w + gap;

    // The parallel taxiway: `TAXIWAY_OFF_SITE_M` off the site's edge, the runway's length.
    let tv1 = edge_v - TAXIWAY_OFF_SITE_M;
    let tv0 = tv1 - TAXIWAY_W_M;
    let taxiway = rw.span(0.0, len, tv0, tv1);
    // Links at each end and the middle, from the taxiway's runway edge to the runway's edge.
    let link_at = [TAXIWAY_W_M * 0.5, len * 0.5, len - TAXIWAY_W_M * 0.5];
    let half_link = TAXIWAY_W_M * 0.5;
    let links: Vec<Rect> = link_at.iter().map(|&u| rw.span(u - half_link, u + half_link, w, tv0)).collect();
    let holds = link_at
        .iter()
        .map(|&u| [rw.at(u - half_link, w + HOLD_BEFORE_M), rw.at(u + half_link, w + HOLD_BEFORE_M)])
        .collect();

    // Site buildings, in the site's frame (x right, z toward the hangars).
    let c = base.centre;
    let local = |x0: f32, z0: f32, x1: f32, z1: f32| Rect {
        min: c + Vec2::new(x0, z0),
        max: c + Vec2::new(x1, z1),
    };
    let (hx, hz) = (base.half_m.x, base.half_m.y);
    let mut buildings = vec![
        Building {
            kind: Kind::Terminal,
            rect: local(-hx + 5.0, -130.0, -hx + 35.0, 130.0),
            height_m: 14.0,
        },
        Building {
            kind: Kind::FireStation,
            rect: local(-hx + 5.0, -hz + 10.0, -hx + 35.0, -hz + 35.0),
            height_m: 9.0,
        },
        Building {
            kind: Kind::CarPark,
            rect: local(-hx + 20.0, hz - 100.0, -hx + 140.0, hz - 30.0),
            height_m: 0.0,
        },
    ];
    for k in [-1.0f32, 0.0, 1.0] {
        buildings.push(Building {
            kind: Kind::FuelTank,
            rect: Rect::centred(c + Vec2::new(hx - 30.0, 30.0 + k * 25.0), Vec2::splat(7.0)),
            height_m: 10.0,
        });
    }
    let tower = base.tower().centre();
    buildings.push(Building {
        kind: Kind::Helipad,
        rect: Rect::centred(tower - Vec2::new(55.0, 0.0), Vec2::splat(12.0)),
        height_m: 0.0,
    });

    // Three stands in front of the terminal, 80 m square, their aircraft nose in toward it.
    let face = -hx + 35.0;
    let stands = [-85.0f32, 0.0, 85.0]
        .into_iter()
        .map(|z| Stand {
            rect: local(face + 25.0, z - 40.0, face + 105.0, z + 40.0),
            nose: Vec2::NEG_X,
            bridge: local(face, z - 1.5, face + 25.0, z + 1.5),
        })
        .collect::<Vec<_>>();

    // The apron entries: the first two lanes along the site's runway-side edge whose way 60 m into the site is clear.
    let (su0, su1) = site_span_u(rw, &site);
    let mut entries = Vec::new();
    for f in [1.0 / 3.0, 2.0 / 3.0, 1.0 / 6.0, 5.0 / 6.0, 0.5] {
        if entries.len() == 2 {
            break;
        }
        let u = su0 + (su1 - su0) * f;
        let lane = rw.span(u - ENTRY_W_M * 0.5, u + ENTRY_W_M * 0.5, tv1, edge_v + 60.0);
        let blocked = buildings.iter().any(|b| b.rect.overlaps(&lane))
            || stands.iter().any(|s| s.rect.overlaps(&lane))
            || base.structures(cfg).iter().any(|s| s.overlaps(&lane));
        if !blocked {
            entries.push(rw.span(u - ENTRY_W_M * 0.5, u + ENTRY_W_M * 0.5, tv1, edge_v));
        }
    }

    let windsock = rw.at(200.0, w + 25.0);
    let papi = [0usize, 1].map(|end| {
        let u = if end == 0 {
            BLAST_PAD_M + AIMING_POINT_M
        } else {
            len - BLAST_PAD_M - AIMING_POINT_M
        };
        [0.0f32, 1.0, 2.0, 3.0].map(|k| rw.at(u, -w - 15.0 - k * 9.0))
    });
    let posts = |sign: f32, from: f32| -> Vec<Vec2> {
        let n = (APPROACH_M / APPROACH_STEP_M) as usize;
        (1..=n).map(|i| rw.at(from + sign * i as f32 * APPROACH_STEP_M, 0.0)).collect()
    };
    let approach = [posts(-1.0, 0.0), posts(1.0, len)];

    // The fence round the site and the strip, `FENCE_IN_M` inside their joint outline.
    let fence = inset_polygon(&outline(rw, &site, gap), FENCE_IN_M);

    Airfield {
        strip: rw.strip(),
        taxiway,
        links,
        holds,
        entries,
        buildings,
        stands,
        windsock,
        papi,
        approach,
        designators: rw.designators(),
        fence,
    }
}

/// The outline of the site and the strip together, counterclockwise in (x, z) when +x is right and +z up the page.
fn outline(rw: &Runway, site: &Rect, gap: f32) -> Vec<Vec2> {
    let w = rw.width_m * 0.5;
    let len = rw.length_m();
    let (v_far, v_edge) = (-w - STRIP_OUT_M, w + gap);
    let (su0, su1) = site_span_u(rw, site);
    let (su0, su1) = (su0.max(0.0), su1.min(len));
    let sv1 = corners_uv(rw, site).iter().map(|c| c.y).fold(f32::NEG_INFINITY, f32::max);
    let uv = [
        Vec2::new(0.0, v_far),
        Vec2::new(len, v_far),
        Vec2::new(len, v_edge),
        Vec2::new(su1, v_edge),
        Vec2::new(su1, sv1),
        Vec2::new(su0, sv1),
        Vec2::new(su0, v_edge),
        Vec2::new(0.0, v_edge),
    ];
    uv.iter().map(|p| rw.at(p.x, p.y)).collect()
}

/// The site's corners in the runway's frame (u along, v across toward the site).
fn corners_uv(rw: &Runway, site: &Rect) -> [Vec2; 4] {
    let to_uv = |p: Vec2| Vec2::new((p - rw.a).dot(rw.dir()), (p - rw.a).dot(rw.toward_site()));
    [
        site.min,
        site.max,
        Vec2::new(site.min.x, site.max.y),
        Vec2::new(site.max.x, site.min.y),
    ]
    .map(to_uv)
}

/// Where the site runs along the runway, `u0..u1` from `a`.
fn site_span_u(rw: &Runway, site: &Rect) -> (f32, f32) {
    let c = corners_uv(rw, site);
    (
        c.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
        c.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
    )
}

/// A rectilinear polygon moved `d` metres inward, whichever way it winds.
fn inset_polygon(poly: &[Vec2], d: f32) -> Vec<Vec2> {
    let n = poly.len();
    // Signed area: positive when counterclockwise.
    let area: f32 = (0..n).map(|i| poly[i].perp_dot(poly[(i + 1) % n])).sum::<f32>() * 0.5;
    let inward = |e: Vec2| if area > 0.0 { e.perp() } else { -e.perp() };
    (0..n)
        .map(|i| {
            let (prev, p, next) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            let (e0, e1) = ((p - prev).normalize_or_zero(), (next - p).normalize_or_zero());
            p + (inward(e0) + inward(e1)) * d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(side: RunwaySide) -> (AirBase, BasesConfig) {
        let cfg = BasesConfig {
            runway_gap_m: 100.0,
            ..BasesConfig::default()
        };
        let centre = Vec2::new(500.0, -800.0);
        let runway = Runway::beside(centre, cfg.half_m, side, 1800.0, 0.0, &cfg);
        let b = AirBase {
            id: 1,
            centre,
            half_m: cfg.half_m,
            ground_m: 20.0,
            highway: 0,
            runway,
        };
        (b, cfg)
    }

    const SIDES: [RunwaySide; 4] = [RunwaySide::Front, RunwaySide::Back, RunwaySide::West, RunwaySide::East];

    #[test]
    fn the_taxiway_runs_the_runways_length_in_the_strip_clear_of_the_runway() {
        for side in SIDES {
            let (b, cfg) = base(side);
            let a = layout(&b, &cfg);
            let rw = b.runway.rect();
            assert!(!a.taxiway.overlaps(&rw), "{side:?}: the taxiway is off the runway");
            assert!(!a.taxiway.overlaps(&b.rect()), "{side:?}: and off the site");
            for corner in [a.taxiway.min, a.taxiway.max] {
                assert_eq!(a.strip.distance(corner), 0.0, "{side:?}: inside the strip");
            }
            let gap = [a.taxiway.min, a.taxiway.max]
                .iter()
                .map(|p| rw.distance(*p))
                .fold(f32::INFINITY, f32::min);
            assert!(gap >= 50.0, "{side:?}: {gap} m from the runway");
            assert!((a.taxiway.size().max_element() - b.runway.length_m()).abs() < 0.5);
            assert_eq!(a.links.len(), 3);
            for l in &a.links {
                assert!(rw.distance(l.centre()) > 0.0 && a.strip.distance(l.centre()) == 0.0);
            }
        }
    }

    #[test]
    fn no_two_buildings_overlap_and_every_stand_is_clear() {
        for side in SIDES {
            let (b, cfg) = base(side);
            let a = layout(&b, &cfg);
            let site = b.rect();
            let mut solid: Vec<Rect> = a.buildings.iter().map(|x| x.rect).collect();
            solid.extend(b.structures(&cfg));
            // The landing pads across the front of the apron.
            for i in 0..3 {
                let c = Vec2::new(b.centre.x + (i as f32 - 1.0) * 80.0, site.min.y + 40.0);
                solid.push(Rect::centred(c, Vec2::splat(20.0)));
            }
            for (i, x) in solid.iter().enumerate() {
                for (j, y) in solid.iter().enumerate().skip(i + 1) {
                    assert!(!x.overlaps(y), "{side:?}: {i} and {j} overlap");
                }
                assert!(
                    site.distance(x.min) == 0.0 && site.distance(x.max) == 0.0,
                    "{side:?}: {i} is on the site"
                );
            }
            for s in &a.stands {
                assert!(site.distance(s.rect.min) == 0.0 && site.distance(s.rect.max) == 0.0);
                assert!(solid.iter().all(|x| !x.overlaps(&s.rect)), "{side:?}: a stand is clear");
            }
            assert_eq!(a.entries.len(), 2, "{side:?}: two ways onto the apron");
        }
    }

    #[test]
    fn designators_read_the_heading_flown_from_each_end() {
        let (b, _) = base(RunwaySide::Front);
        assert_eq!(b.runway.designators(), [9, 27], "west to east");
        let (b, _) = base(RunwaySide::West);
        // `a` is the low-z end, north: flown from there it heads south.
        assert_eq!(b.runway.designators(), [18, 36]);
    }

    #[test]
    fn the_fence_encloses_the_site_and_the_strip() {
        for side in SIDES {
            let (b, cfg) = base(side);
            let a = layout(&b, &cfg);
            assert_eq!(a.fence.len(), 8);
            let inside = |p: Vec2| {
                let n = a.fence.len();
                let mut wind = false;
                for i in 0..n {
                    let (u, v) = (a.fence[i], a.fence[(i + 1) % n]);
                    if (u.y > p.y) != (v.y > p.y) && p.x < u.x + (p.y - u.y) / (v.y - u.y) * (v.x - u.x) {
                        wind = !wind;
                    }
                }
                wind
            };
            for p in [b.centre, b.runway.at(b.runway.length_m() * 0.5, 0.0), a.taxiway.centre()] {
                assert!(inside(p), "{side:?}: {p} is inside the fence");
            }
            for x in &a.buildings {
                assert!(inside(x.rect.centre()), "{side:?}: {:?} is inside the fence", x.kind);
            }
        }
    }
}
