//! The weather (openspec change `cloud-weather`): the sky laid out from the seed by a set of
//! systems, each a rule that places cloud puffs with a kind, a darkness and a layer. Fair cumulus
//! follows the ground's moisture (thick over the wet west, thin over the desert in the lee), storm
//! cells stand towers with anvils blown downwind and rain shafts beneath, a cold front lies off the
//! west coast, a stratocumulus deck sits at sea to the north, cloud banks against the range's
//! windward flank, caps sit over its highest peaks, an altocumulus field lies high to the south-west
//! and cirrus streaks at 8-9 km. A high over the sea to the south-east keeps its sky clear.
//!
//! The island mockup's `weather` block (`docs/mockups/island_gen.js`) is the reference for every
//! rule; this ports it to the game. The numbers a designer would turn are in
//! `data/world/world.ron` under `weather:`; the shape of each system (how a tower narrows, how a
//! cluster spreads) is in the code beside its rule. `clouds::Clouds` indexes the result: it is the
//! one cloud query the view, the rain on the glass and the drawing all read.

use super::clouds::{Cirrus, CloudKind, Puff, RainShaft, StormShade};
use super::terrain::{ValueNoise, smoothstep};
use crate::data::{Validate, finite, positive, positive_range, within};
use crate::rng::SimRng;
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::f32::consts::TAU;

/// What the weather needs to know about the island under it.
pub trait WeatherGround {
    /// Ground height at `p` (x, z), metres above sea level.
    fn ground_m(&self, p: Vec2) -> f32;
    /// Signed distance past the coast at `p`: negative inland, positive out to sea.
    fn past_coast_m(&self, p: Vec2) -> f32;
    /// Moisture at `p`, 0 (desert) to 1 (marsh).
    fn moisture(&self, p: Vec2) -> f32;
}

/// Fair-weather cumulus: clusters on a grid of tiles over a disc round the island.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CumulusConfig {
    /// Cloud base (the condensation level).
    pub base_m: f32,
    /// Side of a tile of the cluster grid.
    pub tile_m: f32,
    /// Cumulus covers a disc this far from the centre...
    pub disc_m: f32,
    /// ...thinning from this far out, so the sky has no edge.
    pub thin_from_m: f32,
    /// How far the disc's edge wanders.
    pub edge_noise_m: f32,
    /// Clusters a tile over land: the first plus the second times the moisture.
    pub land_clusters: [f32; 2],
    /// Clusters a tile at sea.
    pub sea_clusters: f32,
    /// Clusters a tile in the cloud streets.
    pub street_clusters: f32,
    /// The streets lie at sea south of this (z, metres).
    pub streets_south_of_m: f32,
    /// Cloud streets run along the wind, this far apart.
    pub street_spacing_m: f32,
    /// Share of clusters (times the moisture) that tower into congestus.
    pub towering_share: f32,
    /// Share of the rest that stay small and flat (humilis).
    pub humilis_share: f32,
    /// Towering cumulus tops.
    pub towering_top_m: [f32; 2],
    /// No cumulus within this of the front's band.
    pub clear_of_front_m: f32,
    /// Darkness of fair cumulus and of towering cumulus (0 white, 1 storm grey).
    pub dark: [f32; 2],
}

impl Default for CumulusConfig {
    fn default() -> Self {
        Self {
            base_m: 600.0,
            tile_m: 2000.0,
            disc_m: 34000.0,
            thin_from_m: 24000.0,
            edge_noise_m: 2500.0,
            land_clusters: [0.3, 2.8],
            sea_clusters: 0.5,
            street_clusters: 1.6,
            streets_south_of_m: 6000.0,
            street_spacing_m: 2200.0,
            towering_share: 0.28,
            humilis_share: 0.45,
            towering_top_m: [2200.0, 3800.0],
            clear_of_front_m: 1500.0,
            dark: [0.08, 0.3],
        }
    }
}

/// A high: fair cumulus thins to nothing toward its centre.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct HighConfig {
    /// Its centre (x, z).
    pub centre_m: [f32; 2],
    /// Clear sky within this of the centre...
    pub clear_m: f32,
    /// ...and the cumulus back to full past this.
    pub edge_m: f32,
}

impl Default for HighConfig {
    fn default() -> Self {
        Self {
            centre_m: [17000.0, 12000.0],
            clear_m: 6000.0,
            edge_m: 17000.0,
        }
    }
}

/// One storm cell (cumulonimbus).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StormCell {
    /// Its centre (x, z).
    pub centre_m: [f32; 2],
    /// The tower's radius at its base.
    pub radius_m: f32,
    /// The anvil's top.
    pub top_m: f32,
}

/// Storm cells: towers, anvils, scud, rain shafts and the ground darkened under them.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct StormConfig {
    /// The cells.
    pub cells: Vec<StormCell>,
    /// Where the tower starts.
    pub base_m: f32,
    /// The tower's rings of puffs, this far apart in height.
    pub ring_step_m: f32,
    /// Wide flat puffs in each anvil.
    pub anvil_puffs: u32,
    /// The anvil is blown this far downwind of the tower.
    pub anvil_downwind_m: f32,
    /// Ragged puffs of scud under each base.
    pub scud_puffs: u32,
    /// Darkness at the tower's base and at its top.
    pub dark: [f32; 2],
    /// The rain shaft's radius as a share of the tower's, and its top.
    pub shaft_share: f32,
    /// The rain shaft's top (just above the base).
    pub shaft_top_m: f32,
    /// The ground is darkened out to this times the tower's radius...
    pub shade_reach: f32,
    /// ...by this share of the sun at the centre...
    pub shade: f32,
    /// ...its edge wandering this far in and out.
    pub shade_wobble_m: f32,
}

impl Default for StormConfig {
    fn default() -> Self {
        Self {
            cells: vec![
                // Over the wet western jungle, 6.5 km west of the spine (the mockup's cell).
                StormCell {
                    centre_m: [-5887.0, -4186.0],
                    radius_m: 2600.0,
                    top_m: 9500.0,
                },
                // At sea to the south-west.
                StormCell {
                    centre_m: [-15500.0, 9500.0],
                    radius_m: 3200.0,
                    top_m: 10500.0,
                },
                // A smaller one at sea to the north-west.
                StormCell {
                    centre_m: [-11000.0, -14000.0],
                    radius_m: 1900.0,
                    top_m: 6500.0,
                },
            ],
            base_m: 700.0,
            ring_step_m: 260.0,
            anvil_puffs: 70,
            anvil_downwind_m: 2600.0,
            scud_puffs: 24,
            dark: [0.75, 0.15],
            shaft_share: 0.75,
            shaft_top_m: 720.0,
            shade_reach: 1.6,
            shade: 0.85,
            shade_wobble_m: 600.0,
        }
    }
}

/// A cold front: a long band of layered grey cloud with towers and rain along it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FrontConfig {
    /// Its middle (x, z).
    pub centre_m: [f32; 2],
    /// The direction it lies along (x, z).
    pub along: [f32; 2],
    /// Its length.
    pub length_m: f32,
    /// Half its width.
    pub half_width_m: f32,
    /// Puffs are laid every this far along it, three abreast.
    pub step_m: f32,
    /// The layered cloud's base and top.
    pub layer_m: [f32; 2],
    /// A tower stands every this far along it...
    pub tower_every_m: f32,
    /// ...up to this.
    pub tower_top_m: f32,
    /// A rain shaft every this far...
    pub shaft_every_m: f32,
    /// ...this wide.
    pub shaft_radius_m: f32,
    /// The ground is darkened every this far along it...
    pub shade_every_m: f32,
    /// ...out to this from the line...
    pub shade_radius_m: f32,
    /// ...by this share of the sun.
    pub shade: f32,
}

impl Default for FrontConfig {
    fn default() -> Self {
        Self {
            centre_m: [-17000.0, -4000.0],
            along: [0.62, -0.78],
            length_m: 52000.0,
            half_width_m: 4200.0,
            step_m: 300.0,
            layer_m: [900.0, 2800.0],
            tower_every_m: 6000.0,
            tower_top_m: 6200.0,
            shaft_every_m: 5000.0,
            shaft_radius_m: 2200.0,
            shade_every_m: 10000.0,
            shade_radius_m: 6000.0,
            shade: 0.6,
        }
    }
}

/// A stratocumulus deck over the sea: wide flat puffs merging into a sheet, with breaks.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DeckConfig {
    /// Puffs tried.
    pub count: u32,
    /// The bearings it covers, degrees (0 north, 90 east; the second may pass 360).
    pub bearing_deg: [f32; 2],
    /// Its distance from the centre.
    pub distance_m: [f32; 2],
    /// Its base and top.
    pub layer_m: [f32; 2],
    /// It keeps this far off the coast.
    pub offshore_m: f32,
    /// Breaks where a noise of this wavelength...
    pub break_wavelength_m: f32,
    /// ...is below this (-1..1).
    pub break_below: f32,
}

impl Default for DeckConfig {
    fn default() -> Self {
        Self {
            count: 900,
            bearing_deg: [330.0, 400.0],
            distance_m: [14500.0, 21500.0],
            layer_m: [700.0, 1000.0],
            offshore_m: 600.0,
            break_wavelength_m: 4000.0,
            break_below: -0.45,
        }
    }
}

/// Cloud banked against the windward flank of high ground: where the land rises downwind.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct OrographicConfig {
    /// Most puffs.
    pub count: u32,
    /// Places tried, over the disc.
    pub tries: u32,
    /// Places are tried within this of the centre.
    pub search_m: f32,
    /// Only over ground at least this high...
    pub min_ground_m: f32,
    /// ...that rises at least `min_rise_m` within `rise_probe_m` downwind (a windward flank).
    pub rise_probe_m: f32,
    /// See `rise_probe_m`.
    pub min_rise_m: f32,
    /// No higher than this.
    pub top_m: f32,
}

impl Default for OrographicConfig {
    fn default() -> Self {
        Self {
            count: 520,
            tries: 12000,
            search_m: 14000.0,
            min_ground_m: 500.0,
            rise_probe_m: 900.0,
            min_rise_m: 60.0,
            top_m: 1900.0,
        }
    }
}

/// Lenticular caps over the highest peaks.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CapConfig {
    /// Only over ground at least this high.
    pub min_peak_m: f32,
    /// Caps at least this far apart.
    pub spacing_m: f32,
    /// At most this many.
    pub most: u32,
    /// Places tried, over the disc.
    pub tries: u32,
    /// Places are tried within this of the centre.
    pub search_m: f32,
}

impl Default for CapConfig {
    fn default() -> Self {
        Self {
            min_peak_m: 1900.0,
            spacing_m: 2500.0,
            most: 5,
            tries: 20000,
            search_m: 12000.0,
        }
    }
}

/// A field of small flat altocumulus with gaps.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct AltocumulusConfig {
    /// The field's centre (x, z).
    pub centre_m: [f32; 2],
    /// Its radius.
    pub radius_m: f32,
    /// A puff every this far.
    pub spacing_m: f32,
    /// Its base and top.
    pub layer_m: [f32; 2],
}

impl Default for AltocumulusConfig {
    fn default() -> Self {
        Self {
            centre_m: [-6000.0, 14000.0],
            radius_m: 11000.0,
            spacing_m: 450.0,
            layer_m: [3300.0, 3700.0],
        }
    }
}

/// Cirrus: long thin streaks high up, along the wind.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CirrusConfig {
    /// How many streaks.
    pub count: u32,
    /// They lie on a disc this wide (radius).
    pub disc_m: f32,
    /// Their height.
    pub height_m: [f32; 2],
    /// Their length.
    pub length_m: [f32; 2],
    /// Their width.
    pub width_m: [f32; 2],
}

impl Default for CirrusConfig {
    fn default() -> Self {
        Self {
            count: 90,
            disc_m: 32000.0,
            height_m: [8200.0, 9400.0],
            length_m: [5000.0, 14000.0],
            width_m: [500.0, 1400.0],
        }
    }
}

/// The weather systems' tuning (`data/world/world.ron`, `weather:`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WeatherConfig {
    /// Fair-weather cumulus.
    pub cumulus: CumulusConfig,
    /// The high with clear sky under it.
    pub high: HighConfig,
    /// Storm cells.
    pub storms: StormConfig,
    /// The cold front.
    pub front: FrontConfig,
    /// The stratocumulus deck.
    pub deck: DeckConfig,
    /// Cloud on the range's windward flank.
    pub orographic: OrographicConfig,
    /// Caps over the peaks.
    pub caps: CapConfig,
    /// The altocumulus field.
    pub altocumulus: AltocumulusConfig,
    /// Cirrus.
    pub cirrus: CirrusConfig,
    /// How opaque each kind is, in `CloudKind` order.
    pub opacity: [f32; 7],
    /// Rain on the glass inside a rain shaft below its top, 0..1.
    pub shaft_rain: f32,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            cumulus: CumulusConfig::default(),
            high: HighConfig::default(),
            storms: StormConfig::default(),
            front: FrontConfig::default(),
            deck: DeckConfig::default(),
            orographic: OrographicConfig::default(),
            caps: CapConfig::default(),
            altocumulus: AltocumulusConfig::default(),
            cirrus: CirrusConfig::default(),
            opacity: [0.95, 1.0, 0.8, 0.8, 0.6, 0.9, 0.6],
            shaft_rain: 0.7,
        }
    }
}

fn layer(field: &str, r: [f32; 2]) -> Result<(), String> {
    positive_range(field, r)
}

impl Validate for WeatherConfig {
    fn validate(&self) -> Result<(), String> {
        let c = &self.cumulus;
        positive("weather.cumulus.base_m", c.base_m)?;
        positive("weather.cumulus.tile_m", c.tile_m)?;
        positive("weather.cumulus.disc_m", c.disc_m)?;
        positive("weather.cumulus.thin_from_m", c.thin_from_m)?;
        if c.thin_from_m >= c.disc_m {
            return Err("`weather.cumulus.thin_from_m` must be inside `disc_m`".into());
        }
        finite("weather.cumulus.edge_noise_m", c.edge_noise_m)?;
        for v in c.land_clusters.iter().chain([&c.sea_clusters, &c.street_clusters]) {
            within("weather.cumulus clusters", *v, 0.0, 20.0)?;
        }
        finite("weather.cumulus.streets_south_of_m", c.streets_south_of_m)?;
        positive("weather.cumulus.street_spacing_m", c.street_spacing_m)?;
        within("weather.cumulus.towering_share", c.towering_share, 0.0, 1.0)?;
        within("weather.cumulus.humilis_share", c.humilis_share, 0.0, 1.0)?;
        layer("weather.cumulus.towering_top_m", c.towering_top_m)?;
        finite("weather.cumulus.clear_of_front_m", c.clear_of_front_m)?;
        for d in c.dark {
            within("weather.cumulus.dark", d, 0.0, 1.0)?;
        }
        let h = &self.high;
        finite("weather.high.centre_m", h.centre_m[0])?;
        finite("weather.high.centre_m", h.centre_m[1])?;
        positive_range("weather.high (clear_m, edge_m)", [h.clear_m, h.edge_m])?;
        let s = &self.storms;
        for cell in &s.cells {
            finite("weather.storms.cells.centre_m", cell.centre_m[0])?;
            finite("weather.storms.cells.centre_m", cell.centre_m[1])?;
            positive("weather.storms.cells.radius_m", cell.radius_m)?;
            if cell.top_m <= s.base_m * 2.0 {
                return Err("a storm cell's `top_m` must be well above `weather.storms.base_m`".into());
            }
        }
        positive("weather.storms.base_m", s.base_m)?;
        positive("weather.storms.ring_step_m", s.ring_step_m)?;
        finite("weather.storms.anvil_downwind_m", s.anvil_downwind_m)?;
        for d in s.dark {
            within("weather.storms.dark", d, 0.0, 1.0)?;
        }
        within("weather.storms.shaft_share", s.shaft_share, 0.0, 2.0)?;
        positive("weather.storms.shaft_top_m", s.shaft_top_m)?;
        positive("weather.storms.shade_reach", s.shade_reach)?;
        within("weather.storms.shade", s.shade, 0.0, 1.0)?;
        finite("weather.storms.shade_wobble_m", s.shade_wobble_m)?;
        let f = &self.front;
        finite("weather.front.centre_m", f.centre_m[0])?;
        finite("weather.front.centre_m", f.centre_m[1])?;
        if Vec2::from(f.along).length().is_nan() || Vec2::from(f.along).length() <= 1e-3 {
            return Err("`weather.front.along` must be a direction".into());
        }
        positive("weather.front.length_m", f.length_m)?;
        positive("weather.front.half_width_m", f.half_width_m)?;
        positive("weather.front.step_m", f.step_m)?;
        layer("weather.front.layer_m", f.layer_m)?;
        positive("weather.front.tower_every_m", f.tower_every_m)?;
        positive("weather.front.tower_top_m", f.tower_top_m)?;
        positive("weather.front.shaft_every_m", f.shaft_every_m)?;
        positive("weather.front.shaft_radius_m", f.shaft_radius_m)?;
        positive("weather.front.shade_every_m", f.shade_every_m)?;
        positive("weather.front.shade_radius_m", f.shade_radius_m)?;
        within("weather.front.shade", f.shade, 0.0, 1.0)?;
        let d = &self.deck;
        finite("weather.deck.bearing_deg", d.bearing_deg[0])?;
        finite("weather.deck.bearing_deg", d.bearing_deg[1])?;
        positive_range("weather.deck.distance_m", d.distance_m)?;
        layer("weather.deck.layer_m", d.layer_m)?;
        finite("weather.deck.offshore_m", d.offshore_m)?;
        positive("weather.deck.break_wavelength_m", d.break_wavelength_m)?;
        within("weather.deck.break_below", d.break_below, -1.0, 1.0)?;
        let o = &self.orographic;
        positive("weather.orographic.search_m", o.search_m)?;
        finite("weather.orographic.min_ground_m", o.min_ground_m)?;
        positive("weather.orographic.rise_probe_m", o.rise_probe_m)?;
        finite("weather.orographic.min_rise_m", o.min_rise_m)?;
        positive("weather.orographic.top_m", o.top_m)?;
        positive("weather.caps.min_peak_m", self.caps.min_peak_m)?;
        positive("weather.caps.spacing_m", self.caps.spacing_m)?;
        positive("weather.caps.search_m", self.caps.search_m)?;
        let a = &self.altocumulus;
        finite("weather.altocumulus.centre_m", a.centre_m[0])?;
        finite("weather.altocumulus.centre_m", a.centre_m[1])?;
        positive("weather.altocumulus.radius_m", a.radius_m)?;
        positive("weather.altocumulus.spacing_m", a.spacing_m)?;
        layer("weather.altocumulus.layer_m", a.layer_m)?;
        let ci = &self.cirrus;
        positive("weather.cirrus.disc_m", ci.disc_m)?;
        layer("weather.cirrus.height_m", ci.height_m)?;
        layer("weather.cirrus.length_m", ci.length_m)?;
        layer("weather.cirrus.width_m", ci.width_m)?;
        for v in self.opacity {
            within("weather.opacity", v, 0.0, 1.0)?;
        }
        within("weather.shaft_rain", self.shaft_rain, 0.0, 1.0)
    }
}

/// What the weather systems lay out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sky {
    /// Every puff, system by system.
    pub puffs: Vec<Puff>,
    /// Rain shafts.
    pub shafts: Vec<RainShaft>,
    /// Where the ground is darkened.
    pub shades: Vec<StormShade>,
    /// Cirrus streaks.
    pub cirrus: Vec<Cirrus>,
}

/// Puffs as they are added: one random stream, the opacity table, the kind of each.
struct Builder<'a> {
    r: SimRng,
    opacity: &'a [f32; 7],
    sky: Sky,
}

impl Builder<'_> {
    fn f(&mut self) -> f32 {
        self.r.next_f32()
    }

    /// `lo..hi`.
    fn range(&mut self, r: [f32; 2]) -> f32 {
        r[0] + (r[1] - r[0]) * self.f()
    }

    /// A puff at `c`, `rx` across and `ry` up, lit through the layer `[base, top]`.
    fn puff(&mut self, c: Vec3, rx: f32, ry: f32, kind: CloudKind, dark: f32, layer_m: [f32; 2]) {
        let seed = self.f() * 100.0;
        let top = layer_m[1].max(layer_m[0] + 1.0);
        self.sky.puffs.push(Puff {
            centre: c,
            radii: Vec3::new(rx, ry, rx),
            kind,
            dark: dark.clamp(0.0, 1.0),
            opacity: self.opacity[kind as usize],
            layer_m: [layer_m[0], top],
            seed,
        });
    }
}

/// The point at `bearing` degrees and `distance` from `c`.
fn at_bearing(c: Vec2, bearing: f32, distance: f32) -> Vec2 {
    let b = bearing.to_radians();
    c + Vec2::new(b.sin(), -b.cos()) * distance
}

/// Lays out the sky (cloud-weather design section 2). Pure: the same seed, config, wind and
/// ground give the same sky, in the same order.
pub fn make_sky(rng: &SimRng, cfg: &WeatherConfig, wind_m_s: Vec2, ground: &dyn WeatherGround) -> Sky {
    let wind = wind_m_s.normalize_or(Vec2::X);
    let across = wind.perp();
    let mut b = Builder {
        r: rng.fork("systems"),
        opacity: &cfg.opacity,
        sky: Sky::default(),
    };
    let noise = ValueNoise::new(&rng.fork("noise"));
    storms(&mut b, &cfg.storms, wind);
    front(&mut b, &cfg.front, cfg.storms.shade_wobble_m);
    cumulus(&mut b, cfg, wind, across, ground, &noise);
    altocumulus(&mut b, &cfg.altocumulus, &noise);
    deck(&mut b, &cfg.deck, ground, &noise);
    orographic(&mut b, &cfg.orographic, ground, wind);
    caps(&mut b, &cfg.caps, ground, wind);
    cirrus(&mut b, &cfg.cirrus, wind);
    b.sky
}

/// Storm cells: a tower of rings of puffs, widest low and narrowing, leaning downwind; an anvil of
/// wide flat puffs at 86-94 % of the top, blown downwind; ragged scud under the base; a rain shaft
/// and darkened ground beneath.
fn storms(b: &mut Builder, s: &StormConfig, wind: Vec2) {
    for cell in &s.cells {
        let c = Vec2::from(cell.centre_m);
        let (r, top) = (cell.radius_m, cell.top_m);
        let layer = [s.base_m, top];
        b.sky.shades.push(StormShade {
            centre: c,
            radius_m: r * s.shade_reach,
            shade: s.shade,
            wobble_m: s.shade_wobble_m,
        });
        let mut y = s.base_m + 50.0;
        while y < top * 0.86 {
            let t = y / top;
            let ring = r * (0.95 - 0.35 * t) * (0.85 + 0.3 * b.f());
            let n = ((ring / 260.0).round() as u32).max(3);
            for _ in 0..n {
                let a = b.f() * TAU;
                let d = b.f().sqrt() * ring;
                let rr = 380.0 + b.f() * 420.0 + 260.0 * t;
                let at = c + Vec2::new(a.cos(), a.sin()) * d + wind * (t * 900.0);
                let h = y + b.f() * 120.0;
                let dark = s.dark[0] + (s.dark[1] - s.dark[0]) * t;
                b.puff(Vec3::new(at.x, h, at.y), rr, rr * 0.85, CloudKind::Tower, dark, layer);
            }
            y += s.ring_step_m;
        }
        for _ in 0..s.anvil_puffs {
            let a = b.f() * TAU;
            let d = b.f().sqrt() * r * 2.4;
            let off = Vec2::new(a.cos() * d, a.sin() * d * 0.7);
            // The anvil spreads across the wind more than along it.
            let at = c + wind * (s.anvil_downwind_m + off.x) + wind.perp() * off.y;
            let h = top * (0.86 + 0.08 * b.f());
            let (rx, ry) = (1300.0 + b.f() * 900.0, 260.0 + b.f() * 160.0);
            b.puff(Vec3::new(at.x, h, at.y), rx, ry, CloudKind::Anvil, s.dark[1], layer);
        }
        for _ in 0..s.scud_puffs {
            let a = b.f() * TAU;
            let d = b.f().sqrt() * r;
            let at = c + Vec2::new(a.cos(), a.sin()) * d;
            let h = s.base_m - 180.0 + b.f() * 160.0;
            let (rx, ry) = (260.0 + b.f() * 260.0, 90.0 + b.f() * 60.0);
            b.puff(
                Vec3::new(at.x, h, at.y),
                rx,
                ry,
                CloudKind::Layer,
                0.85,
                [s.base_m - 250.0, s.base_m + 200.0],
            );
        }
        b.sky.shafts.push(RainShaft {
            centre: c + wind * 300.0,
            radius_m: r * s.shaft_share,
            top_m: s.shaft_top_m,
        });
    }
}

/// The front: layered grey cloud along a band, thickest and darkest along its middle, towers every
/// few kilometres, rain shafts and darkened ground along it (its edge wandering `wobble_m`, as a
/// storm's does).
fn front(b: &mut Builder, f: &FrontConfig, wobble_m: f32) {
    let c = Vec2::from(f.centre_m);
    let along = Vec2::from(f.along).normalize_or(Vec2::X);
    let side = along.perp();
    let half = f.length_m * 0.5;
    let n = (f.length_m / f.step_m) as i32;
    let near = |x: f32, every: f32| (x.rem_euclid(every)).min(every - x.rem_euclid(every)) < f.step_m * 0.5;
    for i in 0..n {
        let s = -half + i as f32 * f.step_m;
        let line = c + along * s;
        for _ in 0..3 {
            let off = (b.f() - 0.5) * 2.0 * f.half_width_m;
            let core = 1.0 - off.abs() / f.half_width_m;
            let at = line + side * off;
            let span = f.layer_m[1] - f.layer_m[0];
            let y = f.layer_m[0] + b.f() * (0.25 + 0.75 * core) * span * 0.75;
            let dark = 0.35 + 0.3 * core * (1.0 - (y - f.layer_m[0]) / span);
            let (rx, ry) = (700.0 + b.f() * 700.0, 200.0 + b.f() * 180.0);
            b.puff(
                Vec3::new(at.x, y, at.y),
                rx,
                ry,
                CloudKind::Layer,
                dark,
                [f.layer_m[0] - 100.0, f.layer_m[1]],
            );
        }
        if near(s, f.tower_every_m) {
            let mut y = f.layer_m[0] + 300.0;
            while y < f.tower_top_m {
                let t = y / f.tower_top_m;
                let rr = 500.0 + b.f() * 300.0;
                let at = line + Vec2::new((b.f() - 0.5) * 800.0 + t * 700.0, (b.f() - 0.5) * 800.0);
                b.puff(
                    Vec3::new(at.x, y, at.y),
                    rr,
                    rr * 0.85,
                    CloudKind::Tower,
                    0.7 - 0.55 * t,
                    [f.layer_m[0], f.tower_top_m + 300.0],
                );
                y += 300.0;
            }
        }
        if near(s, f.shaft_every_m) {
            b.sky.shafts.push(RainShaft {
                centre: line,
                radius_m: f.shaft_radius_m,
                top_m: f.layer_m[0],
            });
        }
        if near(s, f.shade_every_m) {
            b.sky.shades.push(StormShade {
                centre: line,
                radius_m: f.shade_radius_m,
                shade: f.shade,
                wobble_m,
            });
        }
    }
}

/// Distance across the front's line.
fn across_front(f: &FrontConfig, p: Vec2) -> f32 {
    let along = Vec2::from(f.along).normalize_or(Vec2::X);
    (p - Vec2::from(f.centre_m)).dot(along.perp())
}

/// Fair-weather cumulus on a grid of tiles over a disc: more over wet ground, few over the desert,
/// none under a storm, in the front or in the high; in streets along the wind at sea to the south;
/// small flat humilis, medium, and towering congestus.
fn cumulus(b: &mut Builder, cfg: &WeatherConfig, wind: Vec2, across: Vec2, ground: &dyn WeatherGround, noise: &ValueNoise) {
    let c = &cfg.cumulus;
    let high = Vec2::from(cfg.high.centre_m);
    let shades = b.sky.shades.clone();
    let storm_at = |p: Vec2| shades.iter().any(|s| s.centre.distance(p) < s.radius_m);
    let n = (c.disc_m / c.tile_m).ceil() as i32 + 1;
    for tz in -n..n {
        for tx in -n..n {
            let corner = Vec2::new(tx as f32, tz as f32) * c.tile_m;
            let mid = corner + Vec2::splat(c.tile_m * 0.5);
            let edge = mid.length() + c.edge_noise_m * noise.unit(mid.x, mid.y, 9000.0, 2, 0.5);
            let far = 1.0 - smoothstep(c.thin_from_m, c.disc_m, edge);
            if far <= 0.0 {
                continue;
            }
            let land = ground.past_coast_m(mid) < 0.0;
            let m = if land { ground.moisture(mid) } else { 0.45 };
            let streets = !land && mid.y > c.streets_south_of_m;
            let base = if land {
                c.land_clusters[0] + c.land_clusters[1] * m
            } else if streets {
                c.street_clusters
            } else {
                c.sea_clusters
            };
            let mut count = base + 0.35 * noise.unit(mid.x + 4100.0, mid.y - 900.0, 9000.0, 2, 0.5);
            count *= smoothstep(cfg.high.clear_m, cfg.high.edge_m, mid.distance(high)) * far;
            if storm_at(mid) || across_front(&cfg.front, mid).abs() < cfg.front.half_width_m + c.clear_of_front_m {
                count = 0.0;
            }
            let clusters = (count.max(0.0) + b.f()).floor() as u32;
            for _ in 0..clusters {
                let mut at = corner + Vec2::new(b.f(), b.f()) * c.tile_m;
                if streets {
                    // Rows along the wind, `street_spacing_m` apart across it.
                    let a = at.dot(across);
                    let snapped = (a / c.street_spacing_m).round() * c.street_spacing_m + (b.f() - 0.5) * 260.0;
                    at += across * (snapped - a);
                }
                let big = b.f() < c.towering_share * if land { m } else { 0.3 };
                let small = !big && b.f() < c.humilis_share;
                let puffs = if big {
                    14 + (b.f() * 10.0) as u32
                } else if small {
                    3 + (b.f() * 4.0) as u32
                } else {
                    5 + (b.f() * 7.0) as u32
                };
                let spread = if big {
                    600.0
                } else if small {
                    160.0 + b.f() * 140.0
                } else {
                    260.0 + b.f() * 260.0
                };
                let tops = if big {
                    b.range(c.towering_top_m)
                } else if small {
                    c.base_m + 200.0
                } else {
                    c.base_m + 500.0
                };
                let dark = if big { c.dark[1] } else { c.dark[0] };
                for _ in 0..puffs {
                    let r = if small {
                        60.0 + b.f() * 110.0
                    } else {
                        90.0 + b.f() * 200.0 + if big { 120.0 } else { 0.0 }
                    };
                    let lift = b.f() * ((tops - c.base_m) * if big { 0.75 } else { 0.35 }).max(10.0);
                    let y = c.base_m + r * 0.7 + lift;
                    let stretch = if streets { 3.0 } else { 2.0 };
                    let off = wind * ((b.f() - 0.5) * spread * stretch) + across * ((b.f() - 0.5) * spread * 1.4);
                    let ry = if small { r * 0.65 } else { r };
                    b.puff(
                        Vec3::new(at.x + off.x, y, at.y + off.y),
                        r * 1.15,
                        ry,
                        CloudKind::Cumulus,
                        dark,
                        [c.base_m, tops],
                    );
                }
            }
        }
    }
}

/// A field of small flat puffs high up, with gaps.
fn altocumulus(b: &mut Builder, a: &AltocumulusConfig, noise: &ValueNoise) {
    let c = Vec2::from(a.centre_m);
    let n = (a.radius_m / a.spacing_m).ceil() as i32;
    for iz in -n..=n {
        for ix in -n..=n {
            let p = c + Vec2::new(ix as f32, iz as f32) * a.spacing_m;
            if p.distance(c) > a.radius_m || noise.unit(p.x - 7700.0, p.y + 3100.0, 3000.0, 2, 0.5) < 0.04 {
                continue;
            }
            let at = p + (Vec2::new(b.f(), b.f()) - 0.5) * 300.0;
            let y = b.range(a.layer_m);
            let (rx, ry) = (150.0 + b.f() * 110.0, 45.0 + b.f() * 25.0);
            b.puff(Vec3::new(at.x, y, at.y), rx, ry, CloudKind::Altocumulus, 0.05, a.layer_m);
        }
    }
}

/// The stratocumulus deck over the sea: wide flat puffs that merge into a sheet, with breaks.
fn deck(b: &mut Builder, d: &DeckConfig, ground: &dyn WeatherGround, noise: &ValueNoise) {
    for _ in 0..d.count {
        let bearing = b.range(d.bearing_deg);
        let r = b.range(d.distance_m);
        let p = at_bearing(Vec2::ZERO, bearing, r);
        let y = d.layer_m[0] + 80.0 + b.f() * (d.layer_m[1] - d.layer_m[0]) * 0.55;
        let (rx, ry) = (520.0 + b.f() * 420.0, 120.0 + b.f() * 70.0);
        let dark = 0.25 + 0.2 * b.f();
        if ground.past_coast_m(p) < d.offshore_m || noise.unit(p.x, p.y, d.break_wavelength_m, 2, 0.5) < d.break_below {
            continue;
        }
        b.puff(Vec3::new(p.x, y, p.y), rx, ry, CloudKind::Layer, dark, d.layer_m);
    }
}

/// A point tried uniformly over the disc of radius `r`.
fn in_disc(b: &mut Builder, r: f32) -> Vec2 {
    let a = b.f() * TAU;
    let d = b.f().sqrt() * r;
    Vec2::new(a.cos(), a.sin()) * d
}

/// Cloud banked on a windward flank (high ground rising downwind), hugging the ground but clear
/// of it.
fn orographic(b: &mut Builder, o: &OrographicConfig, ground: &dyn WeatherGround, wind: Vec2) {
    let mut placed = 0;
    for _ in 0..o.tries {
        if placed >= o.count {
            break;
        }
        let p = in_disc(b, o.search_m);
        let (rx, ry) = (260.0 + b.f() * 260.0, 120.0 + b.f() * 90.0);
        let lift = 80.0 + b.f() * 220.0;
        let g = ground.ground_m(p);
        if g < o.min_ground_m || ground.ground_m(p + wind * o.rise_probe_m) < g + o.min_rise_m {
            continue;
        }
        let y = (g + ry * 1.4 + lift).min(o.top_m);
        if y - ry <= g {
            continue;
        }
        placed += 1;
        b.puff(
            Vec3::new(p.x, y, p.y),
            rx,
            ry,
            CloudKind::Orographic,
            0.2,
            [o.min_ground_m + 300.0, o.top_m + 100.0],
        );
    }
}

/// Lenticular caps: three stacked smooth lenses over each of the highest peaks.
fn caps(b: &mut Builder, c: &CapConfig, ground: &dyn WeatherGround, wind: Vec2) {
    let mut placed: Vec<Vec2> = Vec::new();
    for _ in 0..c.tries {
        if placed.len() >= c.most as usize {
            break;
        }
        let p = in_disc(b, c.search_m);
        let g = ground.ground_m(p);
        if g < c.min_peak_m || placed.iter().any(|q| q.distance(p) < c.spacing_m) {
            continue;
        }
        placed.push(p);
        let at = p + wind * 300.0;
        for l in 0..3 {
            let y = g + 280.0 + l as f32 * 110.0;
            b.puff(
                Vec3::new(at.x, y, at.y),
                900.0 - l as f32 * 180.0,
                70.0,
                CloudKind::Cap,
                0.05,
                [g, g + 600.0],
            );
        }
    }
}

/// Cirrus streaks on a disc, along the wind.
fn cirrus(b: &mut Builder, c: &CirrusConfig, wind: Vec2) {
    let heading = wind.y.atan2(wind.x);
    for _ in 0..c.count {
        let a = b.f() * TAU;
        let r = c.disc_m * b.f().sqrt();
        let y = b.range(c.height_m);
        let length_m = b.range(c.length_m);
        let width_m = b.range(c.width_m);
        let angle_rad = heading + (b.f() - 0.5) * 0.5;
        let seed = b.f() * 100.0;
        b.sky.cirrus.push(Cirrus {
            centre: Vec3::new(a.cos() * r, y, a.sin() * r),
            length_m,
            width_m,
            angle_rad,
            seed,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A round island 13 km across, wet in the west and dry in the east, with a ring of mountains
    /// 2 km high 5.5 km from the centre.
    /// 5.5 km from the centre.
    struct TestIsland;

    impl WeatherGround for TestIsland {
        fn ground_m(&self, p: Vec2) -> f32 {
            let rim = 2000.0 * (1.0 - ((p.length() - 5500.0).abs() / 3000.0).min(1.0));
            if p.length() < 13000.0 { 50.0 + rim } else { -30.0 }
        }
        fn past_coast_m(&self, p: Vec2) -> f32 {
            p.length() - 13000.0
        }
        fn moisture(&self, p: Vec2) -> f32 {
            (0.5 - p.x / 20000.0).clamp(0.0, 1.0)
        }
    }

    fn sky(seed: u64) -> Sky {
        make_sky(&SimRng::new(seed), &WeatherConfig::default(), Vec2::new(6.0, 0.0), &TestIsland)
    }

    fn count(s: &Sky, kind: CloudKind) -> usize {
        s.puffs.iter().filter(|p| p.kind == kind).count()
    }

    #[test]
    fn the_same_seed_gives_the_same_sky() {
        assert_eq!(sky(42), sky(42));
        assert_ne!(sky(42).puffs, sky(43).puffs);
    }

    #[test]
    fn every_puff_lies_inside_its_layer() {
        for p in &sky(42).puffs {
            assert!(
                p.layer_m[0] <= p.centre.y && p.centre.y <= p.layer_m[1],
                "{:?} at {} outside {:?}",
                p.kind,
                p.centre.y,
                p.layer_m
            );
            assert!(p.radii.min_element() > 0.0 && p.centre.is_finite());
            assert!((0.0..=1.0).contains(&p.dark) && (0.0..=1.0).contains(&p.opacity));
        }
    }

    #[test]
    fn every_system_places_cloud() {
        let s = sky(42);
        for kind in [
            CloudKind::Cumulus,
            CloudKind::Tower,
            CloudKind::Anvil,
            CloudKind::Layer,
            CloudKind::Orographic,
            CloudKind::Cap,
            CloudKind::Altocumulus,
        ] {
            assert!(count(&s, kind) > 0, "no {kind:?}");
        }
        assert!(count(&s, CloudKind::Cap) <= 15, "at most five caps of three lenses");
        let cfg = WeatherConfig::default();
        // One shaft and one shade per cell, and the front's.
        assert!(s.shafts.len() > cfg.storms.cells.len());
        assert!(s.shades.len() > cfg.storms.cells.len());
        assert_eq!(s.cirrus.len(), cfg.cirrus.count as usize);
    }

    #[test]
    fn storm_towers_climb_to_their_anvils() {
        let s = sky(42);
        let cfg = WeatherConfig::default();
        let top = cfg.storms.cells.iter().map(|c| c.top_m).fold(0.0, f32::max);
        let highest = s
            .puffs
            .iter()
            .filter(|p| p.kind == CloudKind::Anvil)
            .map(|p| p.centre.y)
            .fold(0.0, f32::max);
        assert!(highest > 0.85 * top && highest <= top, "{highest}");
        let tower_base = s
            .puffs
            .iter()
            .filter(|p| p.kind == CloudKind::Tower)
            .map(|p| p.centre.y)
            .fold(f32::MAX, f32::min);
        assert!(tower_base < 1200.0, "{tower_base}");
    }

    #[test]
    fn the_desert_is_clearer_than_the_wet_west() {
        let s = sky(42);
        let c = |lo: f32, hi: f32| {
            s.puffs
                .iter()
                .filter(|p| p.kind == CloudKind::Cumulus && (lo..hi).contains(&p.centre.x) && p.centre.z.abs() < 4000.0)
                .count()
        };
        // Wet west of the rim (moisture near 0.9) against the dry east (near 0.1), the same area.
        let (west, east) = (c(-12500.0, -8500.0), c(8500.0, 12500.0));
        assert!(west > east * 2, "west {west} east {east}");
    }

    #[test]
    fn the_high_keeps_its_sky_clear() {
        let s = sky(42);
        let h = Vec2::from(WeatherConfig::default().high.centre_m);
        let near = s
            .puffs
            .iter()
            .filter(|p| p.kind == CloudKind::Cumulus && Vec2::new(p.centre.x, p.centre.z).distance(h) < 4000.0)
            .count();
        assert_eq!(near, 0);
    }

    #[test]
    fn orographic_cloud_banks_on_windward_flanks_only() {
        let s = sky(42);
        let oro: Vec<&Puff> = s.puffs.iter().filter(|p| p.kind == CloudKind::Orographic).collect();
        assert!(oro.len() > 50);
        for p in oro {
            let at = Vec2::new(p.centre.x, p.centre.z);
            let g = TestIsland.ground_m(at);
            assert!(TestIsland.ground_m(at + Vec2::X * 900.0) >= g + 60.0, "the ground rises downwind");
            assert!(p.centre.y - p.radii.y > g, "clear of the ground");
        }
    }

    #[test]
    fn caps_sit_over_high_peaks() {
        let s = sky(42);
        for p in s.puffs.iter().filter(|p| p.kind == CloudKind::Cap) {
            assert!(p.layer_m[0] >= WeatherConfig::default().caps.min_peak_m);
        }
    }

    #[test]
    fn cumulus_runs_past_the_world_over_the_sea_with_no_square_edge() {
        let s = sky(42);
        let far = |lo: f32, hi: f32| {
            s.puffs
                .iter()
                .filter(|p| p.kind == CloudKind::Cumulus && (lo..hi).contains(&Vec2::new(p.centre.x, p.centre.z).length()))
                .count()
        };
        assert!(far(21000.0, 30000.0) > 0, "past the 20.5 km half-world");
        assert_eq!(far(37000.0, 1e9), 0, "nothing past the disc");
    }

    #[test]
    fn the_shipped_weather_is_valid() {
        assert!(WeatherConfig::default().validate().is_ok());
        let mut bad = WeatherConfig::default();
        bad.cumulus.thin_from_m = bad.cumulus.disc_m;
        assert!(bad.validate().is_err());
        let mut bad = WeatherConfig::default();
        bad.opacity[0] = f32::NAN;
        assert!(bad.validate().is_err());
    }
}
