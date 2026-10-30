//! Street lamps (openspec change `day-night-cycle`; owner, 2026-10-28: "Every stretch of road should
//! have street lamps"): where every lamp on the island stands, placed by one rule from the road network,
//! city streets and arterials and the highways between cities, and the airport's and seaport's masts and
//! floodlights from their pages' layouts (`night-voxel-light` section 2d). With them the glow dots, lights that
//! light nothing: runway lights by one rule for every runway, and the sites' other dots (approach and taxiway
//! lights, ships' anchor and navigation lights). Pure and deterministic: the same plan gives the same lamps and
//! dots in the same order, each lamp with a stable id (its index). `cb-app` draws their poles and heads and
//! makes the nearest of them real lights.

use super::WorldPlan;
use super::layout::{StreetKind, StreetPiece};
use crate::data::{Validate, within};
use glam::{Vec2, Vec3};
use serde::Deserialize;

/// What a lamp lights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LampKind {
    /// A city street.
    Street,
    /// A wide city arterial.
    Arterial,
    /// A highway between cities.
    Highway,
    /// A site's mast or floodlight (an apron, a quay, a car park, a ship's deck).
    Mast,
}

/// The colour of a lamp or a glow dot (each set in `LampConfig`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum LightColour {
    /// A runway's edge and centre line, an anchor light: white.
    White,
    /// A runway's end, a port sidelight, an obstruction lamp.
    Red,
    /// A threshold, a starboard sidelight.
    Green,
    /// A taxiway's edge.
    Blue,
    /// The sites' floodlights: white metal halide.
    Halide,
    /// The street lamps: orange sodium.
    Sodium,
}

impl LightColour {
    /// Every colour, in order.
    pub const ALL: [LightColour; 6] = [Self::White, Self::Red, Self::Green, Self::Blue, Self::Halide, Self::Sodium];

    /// Its colour, linear rgb.
    pub fn rgb(self, cfg: &LampConfig) -> [f32; 3] {
        match self {
            Self::White => cfg.signal_colours.white,
            Self::Red => cfg.signal_colours.red,
            Self::Green => cfg.signal_colours.green,
            Self::Blue => cfg.signal_colours.blue,
            Self::Halide => cfg.halide_colour,
            Self::Sodium => cfg.colour,
        }
    }
}

/// One lamp: a street lamp, or a site's mast or floodlight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lamp {
    /// The foot of its pole, world metres.
    pub base: Vec3,
    /// Its head, where the light is, world metres.
    pub head: Vec3,
    /// Level, from the pole toward what it lights.
    pub toward: Vec2,
    /// What it lights.
    pub kind: LampKind,
    /// Where its light is aimed, world metres.
    pub aim: Vec3,
    /// Its light in street lamps' lumens (`lumens`) for a street lamp, masts' (`mast_lumens`) for a mast; 0 for a
    /// lamp that only glows.
    pub power: f32,
    /// How far its light reaches, metres.
    pub range_m: f32,
    /// Its cone's outer half angle, radians.
    pub cone_rad: f32,
    /// Its colour.
    pub colour: LightColour,
    /// True when it has a pole and a head to draw; false for a floodlight on something built.
    pub pole: bool,
    /// The island's lot it stands on (a ship, a gantry), dark once half of that lot is down.
    pub lot: Option<u32>,
}

/// A light that lights nothing, drawn as a glow (a runway light, a ship's anchor light).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowDot {
    /// Where, world metres.
    pub at: Vec3,
    /// Its colour.
    pub colour: LightColour,
    /// The island's lot it stands on, dark once half of that lot is down.
    pub lot: Option<u32>,
}

/// The signal lights' colours, linear rgb (`signal_colours:` in `lamps:`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SignalColours {
    /// White.
    pub white: [f32; 3],
    /// Red.
    pub red: [f32; 3],
    /// Green.
    pub green: [f32; 3],
    /// Blue.
    pub blue: [f32; 3],
}

impl Default for SignalColours {
    fn default() -> Self {
        Self {
            white: [1.0, 0.93, 0.82],
            red: [1.0, 0.05, 0.03],
            green: [0.05, 0.75, 0.2],
            blue: [0.08, 0.2, 1.0],
        }
    }
}

/// The runway lights' rule (`runway:` in `lamps:`; ICAO's pattern, cut to what reads): edge lights both sides, a
/// centre line between the touchdown ends, a green bar across each threshold and a red one just past it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RunwayLights {
    /// Edge lights this far apart along each edge, metres...
    pub edge_spacing_m: f32,
    /// ...this far outside the paved edge, metres...
    pub edge_out_m: f32,
    /// ...and this high over the surface, metres.
    pub edge_y_m: f32,
    /// Centre line lights this far apart, metres...
    pub centre_spacing_m: f32,
    /// ...from this far in from each end to as far from the other, metres...
    pub centre_from_m: f32,
    /// ...this high, metres.
    pub centre_y_m: f32,
    /// The threshold this far in from each end, metres.
    pub threshold_in_m: f32,
    /// The red end bar this far toward the end from the threshold, metres.
    pub end_past_m: f32,
    /// The bars' lights this far apart across the runway, metres...
    pub bar_spacing_m: f32,
    /// ...this high, metres.
    pub bar_y_m: f32,
}

impl Default for RunwayLights {
    fn default() -> Self {
        Self {
            edge_spacing_m: 60.0,
            edge_out_m: 3.0,
            edge_y_m: 0.35,
            centre_spacing_m: 30.0,
            centre_from_m: 130.0,
            centre_y_m: 0.1,
            threshold_in_m: 60.0,
            end_past_m: 2.0,
            bar_spacing_m: 3.0,
            bar_y_m: 0.2,
        }
    }
}

/// `lamps:` in `data/world/day_night.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LampConfig {
    /// Along a city street, metres between lamps, alternating sides.
    pub street_spacing_m: f32,
    /// Along an arterial, metres between lamps on each side.
    pub arterial_spacing_m: f32,
    /// Along a highway, metres between lamps, alternating sides.
    pub highway_spacing_m: f32,
    /// Head height over the ground: streets, arterials, highways, metres.
    pub head_m: [f32; 3],
    /// How far the arm reaches out over the road, metres.
    pub arm_m: f32,
    /// The pole stands this far outside the paved edge, metres.
    pub kerb_m: f32,
    /// A street piece shorter than this along its length gets no lamp (the squares where streets
    /// cross), metres.
    pub min_piece_m: f32,
    /// The lamps come on as the sun sinks between these elevations, degrees (fully on at the first).
    pub on_deg: [f32; 2],
    /// A real light's output, lumens (a 1980s 150 W high-pressure sodium lamp).
    pub lumens: f32,
    /// A real light's reach, metres.
    pub range_m: f32,
    /// Its cone: inner and outer half angles, degrees.
    pub cone_deg: [f32; 2],
    /// The light's colour, sRGB 0..1 (sodium orange, survey N5).
    pub colour: [f32; 3],
    /// How many of the nearest lamps are real lights.
    pub real_lamps: u32,
    /// Real lights only within this distance of the eye, metres; they fade out over the last
    /// `fade_m`.
    pub real_m: f32,
    /// Over this distance, metres.
    pub fade_m: f32,
    /// A lit head's brightness, cd/m2 (times the exposure on screen).
    pub head_cd_m2: f32,
    /// A far lamp's glow at its heart, on screen (against white, 1), so a city reads from the air.
    pub glow_brightness: f32,
    /// The glow's least size on screen, pixels.
    pub glow_px: f32,
    /// Its size in the world, metres (it grows past `glow_px` nearer).
    pub glow_m: f32,
    /// The glow is fully in from this far, metres (from half that it fades in).
    pub glow_from_m: f32,
    /// The sites' floodlights' colour, linear rgb (white metal halide, survey N5).
    pub halide_colour: [f32; 3],
    /// A mast's light per unit of its power, lumens.
    pub mast_lumens: f32,
    /// A mast is made a real light within this of the eye, metres (`real_m` is a street lamp's): a 35 m yard mast
    /// lights stacks seen from across the basin.
    pub mast_real_m: f32,
    /// A glow dot's heart, times its colour, on screen.
    pub dot_brightness: f32,
    /// A glow dot's size in the world, metres (at least `glow_px` on screen).
    pub dot_m: f32,
    /// The signal lights' colours.
    pub signal_colours: SignalColours,
    /// The runway lights' rule.
    pub runway: RunwayLights,
}

impl Default for LampConfig {
    fn default() -> Self {
        Self {
            street_spacing_m: 32.0,
            arterial_spacing_m: 30.0,
            highway_spacing_m: 50.0,
            head_m: [9.0, 12.0, 11.0],
            arm_m: 1.6,
            kerb_m: 0.8,
            min_piece_m: 12.0,
            on_deg: [-3.0, 1.0],
            lumens: 15000.0,
            range_m: 35.0,
            cone_deg: [40.0, 65.0],
            colour: [1.0, 0.62, 0.25],
            real_lamps: 96,
            real_m: 250.0,
            fade_m: 50.0,
            head_cd_m2: 8.0,
            glow_brightness: 1.2,
            glow_px: 3.0,
            glow_m: 2.5,
            glow_from_m: 300.0,
            halide_colour: [0.86, 0.92, 1.0],
            mast_lumens: 60000.0,
            mast_real_m: 700.0,
            dot_brightness: 2.0,
            dot_m: 0.8,
            signal_colours: SignalColours::default(),
            runway: RunwayLights::default(),
        }
    }
}

impl Validate for LampConfig {
    fn validate(&self) -> Result<(), String> {
        within("lamps.street_spacing_m", self.street_spacing_m, 5.0, 500.0)?;
        within("lamps.arterial_spacing_m", self.arterial_spacing_m, 5.0, 500.0)?;
        within("lamps.highway_spacing_m", self.highway_spacing_m, 5.0, 1000.0)?;
        for h in self.head_m {
            within("lamps.head_m", h, 2.0, 40.0)?;
        }
        within("lamps.arm_m", self.arm_m, 0.0, 10.0)?;
        within("lamps.kerb_m", self.kerb_m, 0.0, 10.0)?;
        within("lamps.min_piece_m", self.min_piece_m, 0.0, 200.0)?;
        within("lamps.on_deg", self.on_deg[0], -30.0, 30.0)?;
        within("lamps.on_deg", self.on_deg[1], self.on_deg[0] + 0.01, 30.0)?;
        within("lamps.lumens", self.lumens, 0.0, 1.0e7)?;
        within("lamps.range_m", self.range_m, 1.0, 1000.0)?;
        within("lamps.cone_deg", self.cone_deg[0], 0.0, 89.0)?;
        within("lamps.cone_deg", self.cone_deg[1], self.cone_deg[0], 89.9)?;
        for c in self.colour {
            within("lamps.colour", c, 0.0, 1.0)?;
        }
        within("lamps.real_lamps", self.real_lamps as f32, 0.0, 512.0)?;
        within("lamps.real_m", self.real_m, 0.0, 5000.0)?;
        within("lamps.fade_m", self.fade_m, 0.01, 5000.0)?;
        within("lamps.head_cd_m2", self.head_cd_m2, 0.0, 1.0e6)?;
        within("lamps.glow_brightness", self.glow_brightness, 0.0, 100.0)?;
        within("lamps.glow_px", self.glow_px, 0.0, 64.0)?;
        within("lamps.glow_m", self.glow_m, 0.0, 100.0)?;
        within("lamps.glow_from_m", self.glow_from_m, 0.0, 10000.0)?;
        let s = &self.signal_colours;
        for c in self
            .halide_colour
            .iter()
            .chain(&s.white)
            .chain(&s.red)
            .chain(&s.green)
            .chain(&s.blue)
        {
            within("lamps colours", *c, 0.0, 1.0)?;
        }
        within("lamps.mast_lumens", self.mast_lumens, 0.0, 1.0e8)?;
        within("lamps.mast_real_m", self.mast_real_m, 0.0, 5000.0)?;
        within("lamps.dot_brightness", self.dot_brightness, 0.0, 100.0)?;
        within("lamps.dot_m", self.dot_m, 0.0, 100.0)?;
        let r = &self.runway;
        within("lamps.runway.edge_spacing_m", r.edge_spacing_m, 1.0, 1000.0)?;
        within("lamps.runway.edge_out_m", r.edge_out_m, 0.0, 100.0)?;
        within("lamps.runway.edge_y_m", r.edge_y_m, 0.0, 10.0)?;
        within("lamps.runway.centre_spacing_m", r.centre_spacing_m, 1.0, 1000.0)?;
        within("lamps.runway.centre_from_m", r.centre_from_m, 0.0, 5000.0)?;
        within("lamps.runway.centre_y_m", r.centre_y_m, 0.0, 10.0)?;
        within("lamps.runway.threshold_in_m", r.threshold_in_m, 0.0, 5000.0)?;
        within("lamps.runway.end_past_m", r.end_past_m, 0.0, 100.0)?;
        within("lamps.runway.bar_spacing_m", r.bar_spacing_m, 0.5, 100.0)?;
        within("lamps.runway.bar_y_m", r.bar_y_m, 0.0, 10.0)
    }
}

impl LampConfig {
    /// How far from the eye a lamp of `kind` is made a real light, metres.
    pub fn real_m_of(&self, kind: LampKind) -> f32 {
        if kind == LampKind::Mast { self.mast_real_m } else { self.real_m }
    }

    /// How far on the lamps are with the sun at `sun_deg`: 0 by day, 1 from dusk.
    pub fn on_share(&self, sun_deg: f32) -> f32 {
        1.0 - super::terrain::smoothstep(self.on_deg[0], self.on_deg[1], sun_deg)
    }
}

/// Every lamp on the island, in a stable order: the cities' streets city by city, then the
/// highways. None stands in a building or in water.
pub fn place(plan: &WorldPlan, cfg: &LampConfig) -> Vec<Lamp> {
    let mut lamps = Vec::new();
    let clear =
        |p: Vec2, ground: f32| plan.collision.push_out_at(p, 0.4, ground).distance(p) < 1e-3 && plan.waters.water_at(p, ground).is_none();
    for city in &plan.cities {
        for piece in &city.streets {
            street_lamps(piece, cfg, &mut |p, toward, kind| {
                let ground = plan.terrain.height_at(p.x, p.y);
                if clear(p, ground) {
                    lamps.push(lamp(p, ground, toward, kind, cfg));
                }
            });
        }
    }
    let in_city = |p: Vec2| {
        plan.cities
            .iter()
            .any(|c| (p - c.centre).abs().max_element() < c.half_size_m + 20.0)
    };
    for road in &plan.highways {
        let path = &road.path;
        let mut along = 0.0f32;
        let mut next = cfg.highway_spacing_m * 0.5;
        let mut side = 1.0f32;
        for (i, w) in path.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let len = a.distance(b);
            if len < 1e-3 {
                continue;
            }
            let dir = (b - a) / len;
            while next <= along + len {
                let t = (next - along) / len;
                let centre = a.lerp(b, t);
                let out = Vec2::new(-dir.y, dir.x) * side;
                let p = centre + out * (road.width_m * 0.5 + cfg.kerb_m);
                // The pole stands at the road's own height (on a bridge, its deck).
                let grade = road.grade_m[i] + (road.grade_m[i + 1] - road.grade_m[i]) * t;
                if !in_city(p) && plan.collision.push_out_at(p, 0.4, grade).distance(p) < 1e-3 {
                    lamps.push(lamp(p, grade, -out, LampKind::Highway, cfg));
                }
                side = -side;
                next += cfg.highway_spacing_m;
            }
            along += len;
        }
    }
    // The sites' masts and floodlights, as their pages lay them out.
    for site in &plan.sites {
        // a lamp on a port's ship sails with it (ship-traffic): the boats light their own
        let on_ship = |lot: Option<u32>| lot.is_some_and(|p| site.template.placements.get(p as usize).is_some_and(|q| q.ship));
        for m in site.template.lights.masts.iter().filter(|m| !on_ship(m.lot)) {
            let base = site.point_to_world(Vec3::new(m.at.x, m.base_m, m.at.y));
            let aim = site.point_to_world(m.aim);
            let toward = Vec2::new(aim.x - base.x, aim.z - base.z).normalize_or(Vec2::X);
            lamps.push(Lamp {
                base,
                head: base + Vec3::Y * m.h_m,
                toward,
                kind: LampKind::Mast,
                aim,
                power: m.power,
                range_m: m.range_m,
                cone_rad: m.cone_rad,
                colour: m.colour,
                pole: m.pole,
                lot: m.lot.and_then(|p| site.lot_of(p)),
            });
        }
    }
    lamps
}

/// Every glow dot on the island, in a stable order: each site's runways' lights and its own dots, then the air
/// bases' runways' lights.
pub fn place_dots(plan: &WorldPlan, cfg: &LampConfig) -> Vec<GlowDot> {
    let mut dots = Vec::new();
    for site in &plan.sites {
        let lights = &site.template.lights;
        for r in &lights.runways {
            runway_dots(r.a, r.b, r.width_m, &cfg.runway, &mut |p, h, colour| {
                dots.push(GlowDot {
                    at: site.point_to_world(Vec3::new(p.x, r.y_m + h, p.y)),
                    colour,
                    lot: None,
                });
            });
        }
        let on_ship = |lot: Option<u32>| lot.is_some_and(|p| site.template.placements.get(p as usize).is_some_and(|q| q.ship));
        dots.extend(lights.dots.iter().filter(|d| !on_ship(d.lot)).map(|d| GlowDot {
            at: site.point_to_world(d.at),
            colour: d.colour,
            lot: d.lot.and_then(|p| site.lot_of(p)),
        }));
    }
    for base in &plan.bases {
        let r = &base.runway;
        runway_dots(r.a, r.b, r.width_m, &cfg.runway, &mut |p, h, colour| {
            dots.push(GlowDot {
                at: Vec3::new(p.x, base.ground_m + h, p.y),
                colour,
                lot: None,
            });
        });
    }
    dots
}

/// The lights of a runway whose centre line runs from `a` to `b`, `width_m` wide, by the one rule (`RunwayLights`):
/// `put` takes (where, its height over the surface, its colour). The airport page's `runwayDots` draws the same.
pub fn runway_dots(a: Vec2, b: Vec2, width_m: f32, r: &RunwayLights, put: &mut impl FnMut(Vec2, f32, LightColour)) {
    let len = a.distance(b);
    if !(len.is_finite() && len > 1.0) {
        return;
    }
    let d = (b - a) / len;
    let right = Vec2::new(-d.y, d.x);
    let half = width_m * 0.5;
    let at = |u: f32, v: f32| a + d * u + right * v;
    let mut u = 0.0;
    while u <= len + 0.1 {
        for v in [-(half + r.edge_out_m), half + r.edge_out_m] {
            put(at(u, v), r.edge_y_m, LightColour::White);
        }
        u += r.edge_spacing_m;
    }
    let mut u = r.centre_from_m;
    while u <= len - r.centre_from_m {
        put(at(u, 0.0), r.centre_y_m, LightColour::White);
        u += r.centre_spacing_m;
    }
    let bars = [
        (r.threshold_in_m, LightColour::Green),
        (len - r.threshold_in_m, LightColour::Green),
        (r.threshold_in_m - r.end_past_m, LightColour::Red),
        (len - r.threshold_in_m + r.end_past_m, LightColour::Red),
    ];
    for (u, colour) in bars {
        let mut v = -half;
        while v <= half + 0.1 {
            put(at(u, v), r.bar_y_m, colour);
            v += r.bar_spacing_m;
        }
    }
}

fn lamp(p: Vec2, ground: f32, toward: Vec2, kind: LampKind, cfg: &LampConfig) -> Lamp {
    let h = cfg.head_m[match kind {
        LampKind::Street => 0,
        LampKind::Arterial => 1,
        // A mast is placed from its site's table with its own height, never here.
        LampKind::Highway | LampKind::Mast => 2,
    }];
    let reach = toward * cfg.arm_m;
    let head = Vec3::new(p.x + reach.x, ground + h, p.y + reach.y);
    Lamp {
        base: Vec3::new(p.x, ground, p.y),
        head,
        toward,
        kind,
        // Down, leaning a little over the road.
        aim: head + Vec3::new(toward.x, 0.0, toward.y) * 2.0 - Vec3::Y * 6.0,
        power: 1.0,
        range_m: cfg.range_m,
        cone_rad: cfg.cone_deg[1].to_radians(),
        colour: LightColour::Sodium,
        pole: true,
        lot: None,
    }
}

/// The lamps along one piece of city street: streets every `street_spacing_m` alternating sides,
/// arterials every `arterial_spacing_m` on both; each just outside the paved edge, its arm over the
/// road. `put` takes (where, toward the road, kind).
fn street_lamps(piece: &StreetPiece, cfg: &LampConfig, put: &mut impl FnMut(Vec2, Vec2, LampKind)) {
    let r = piece.rect;
    let size = r.max - r.min;
    let (along, across) = if size.x >= size.y { (Vec2::X, Vec2::Y) } else { (Vec2::Y, Vec2::X) };
    let length = size.dot(along);
    let width = size.dot(across);
    if length < cfg.min_piece_m {
        return;
    }
    let (kind, spacing, both) = match piece.kind {
        StreetKind::Street => (LampKind::Street, cfg.street_spacing_m, false),
        StreetKind::Arterial => (LampKind::Arterial, cfg.arterial_spacing_m, true),
    };
    let count = (length / spacing).floor().max(1.0) as u32;
    let step = length / count as f32;
    let start = r.centre() - along * (length * 0.5);
    for k in 0..count {
        let at = start + along * (step * (k as f32 + 0.5));
        let sides: &[f32] = if both {
            &[1.0, -1.0]
        } else if k % 2 == 0 {
            &[1.0]
        } else {
            &[-1.0]
        };
        for side in sides {
            let out = across * *side;
            put(at + out * (width * 0.5 + cfg.kerb_m), -out, kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::Rect;

    fn piece(kind: StreetKind, len: f32, width: f32) -> StreetPiece {
        StreetPiece {
            rect: Rect::centred(Vec2::ZERO, Vec2::new(len * 0.5, width * 0.5)),
            kind,
            width_m: width,
        }
    }

    fn lamps_of(p: &StreetPiece) -> Vec<(Vec2, Vec2, LampKind)> {
        let mut out = Vec::new();
        street_lamps(p, &LampConfig::default(), &mut |a, t, k| out.push((a, t, k)));
        out
    }

    #[test]
    fn a_street_is_lit_every_32_m_on_alternating_sides_from_just_outside_its_edge() {
        let lamps = lamps_of(&piece(StreetKind::Street, 320.0, 12.0));
        assert_eq!(lamps.len(), 10);
        for (i, (p, toward, kind)) in lamps.iter().enumerate() {
            assert_eq!(*kind, LampKind::Street);
            assert!((p.y.abs() - (6.0 + 0.8)).abs() < 1e-4, "outside the edge: {p}");
            assert!(toward.dot(-*p).is_sign_positive(), "the arm reaches over the road");
            if i > 0 {
                assert!(p.y.signum() != lamps[i - 1].0.y.signum(), "alternating sides");
                assert!((p.x - lamps[i - 1].0.x - 32.0).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn an_arterial_is_lit_on_both_sides() {
        let lamps = lamps_of(&piece(StreetKind::Arterial, 300.0, 20.0));
        assert_eq!(lamps.len(), 20);
        assert_eq!(lamps.iter().filter(|l| l.0.y > 0.0).count(), 10);
    }

    #[test]
    fn the_square_where_streets_cross_has_no_lamp() {
        assert!(lamps_of(&piece(StreetKind::Street, 10.0, 10.0)).is_empty());
    }

    fn runway_of(len: f32, width: f32) -> Vec<(Vec2, f32, LightColour)> {
        let mut out = Vec::new();
        runway_dots(Vec2::ZERO, Vec2::new(len, 0.0), width, &RunwayLights::default(), &mut |p, h, c| {
            out.push((p, h, c))
        });
        out
    }

    #[test]
    fn a_runways_edges_are_lit_every_60_m_3_m_outside_the_paving() {
        let dots = runway_of(2000.0, 45.0);
        let edges: Vec<_> = dots.iter().filter(|d| d.0.y.abs() > 22.6).collect();
        assert_eq!(edges.len(), 2 * 34, "0 to 1,980 m every 60 m, both sides");
        assert!(edges.iter().all(|d| (d.0.y.abs() - 25.5).abs() < 1e-3 && d.2 == LightColour::White));
    }

    #[test]
    fn a_runways_thresholds_are_green_and_its_ends_red_across_its_width() {
        let dots = runway_of(2000.0, 45.0);
        let at = |x: f32, c| dots.iter().filter(|d| (d.0.x - x).abs() < 1e-3 && d.2 == c).count();
        assert_eq!(at(60.0, LightColour::Green), 16, "every 3 m across 45 m");
        assert_eq!(at(1940.0, LightColour::Green), 16);
        assert_eq!(at(58.0, LightColour::Red), 16);
        assert_eq!(at(1942.0, LightColour::Red), 16);
    }

    #[test]
    fn a_runways_centre_line_runs_between_its_touchdown_ends() {
        let centre: Vec<_> = runway_of(2000.0, 45.0).into_iter().filter(|d| d.0.y == 0.0).collect();
        assert!(
            centre
                .iter()
                .all(|d| d.0.x >= 130.0 && d.0.x <= 1870.0 && d.2 == LightColour::White)
        );
        assert_eq!(centre.len(), 59);
    }

    #[test]
    fn a_runway_with_its_ends_together_has_no_lights() {
        assert!(runway_of(0.5, 45.0).is_empty());
    }

    #[test]
    fn every_light_colour_reads_its_colour_from_the_config() {
        let cfg = LampConfig::default();
        assert_eq!(LightColour::Sodium.rgb(&cfg), cfg.colour);
        assert_eq!(LightColour::Halide.rgb(&cfg), cfg.halide_colour);
        assert_eq!(LightColour::Red.rgb(&cfg), cfg.signal_colours.red);
    }

    #[test]
    fn lamps_are_on_from_dusk_and_off_by_day() {
        let cfg = LampConfig::default();
        assert_eq!(cfg.on_share(30.0), 0.0);
        assert_eq!(cfg.on_share(-10.0), 1.0);
        assert!(cfg.on_share(-1.0) > 0.0 && cfg.on_share(-1.0) < 1.0);
    }
}
