//! The airport and the seaport in the world plan (openspec changes `airport-layout` and `seaport-layout`): the
//! approved mockups' layouts, exported as kits and a placement table (`assets/models/sites/<site>/site.json`, written
//! by `tools/buildings/export_sites.cjs` from the very pages the owner approved), placed on the island and turned into
//! lots, so every building breaks, collides and is drawn the way the houses are (`small_buildings`).
//!
//! A site keeps its page's frame, turned by quarter turns: x east and z south as the page draws it, y up from the
//! site's ground (the airfield's paving, the port's still water). The civil airport goes beside the largest city it
//! fits by (survey AP1: one airport, the air bases are the military airfields); the seaport on the largest coastal
//! city's coast, its quays on the shoreline and its basin out to sea (survey SP1). Each site shapes the ground inside
//! its own footprint with its pads (survey SP2). Pure and deterministic.

use super::bases::{AirBase, KeepClear};
use super::lamps::LightColour;
use super::layout::{City, Rect};
use super::small_buildings::kit::Kits;
use super::small_buildings::lots::{Lot, Zone};
use super::terrain::Shape;
use crate::data::{Validate, positive, within};
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// Where the sites go, `sites:` in `data/world/world.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SitesConfig {
    /// The airport's footprint keeps this far from every city's square, metres.
    pub airport_city_clear_m: f32,
    /// ...and this far from every air base's site and runway, metres.
    pub airport_base_clear_m: f32,
    /// The natural ground under the airport may rise at most this much across it (it is flattened), metres.
    pub airport_max_rise_m: f32,
    /// The airport's middle is tried this far apart, out from a city's square and round each ring, metres...
    pub airport_step_m: f32,
    /// ...out to this far past the square's half size, metres.
    pub airport_reach_m: f32,
    /// A port's shore is looked for this far out from its city's square, metres.
    pub port_coast_reach_m: f32,
    /// At least this share of the port's land must be dry land...
    pub port_land_share: f32,
    /// ...and at least this share of its basin open water.
    pub port_sea_share: f32,
    /// The port's land may stand at most this high over the sea before it is cut to the deck, metres.
    pub port_max_land_m: f32,
    /// The port slides along its shore by these, metres.
    pub port_slides_m: Vec<f32>,
    /// The port's footprint keeps this far from every city's square, metres.
    pub port_city_clear_m: f32,
    /// Ground is sampled this far apart when a site is fitted, metres.
    pub sample_m: f32,
    /// The sites' ground surfaces drawn with a PBR texture set instead of their page's flat look: surface name (the
    /// export's `quay`, `asphalt`...) to the set's folder under `assets/` (owner, 2026-10-30: "dock platform should use
    /// concrete textures", "and not textured").
    pub ground_textures: BTreeMap<String, String>,
    /// A ground texture repeats every this many metres.
    pub ground_texture_m: f32,
}

impl Default for SitesConfig {
    fn default() -> Self {
        Self {
            airport_city_clear_m: 300.0,
            airport_base_clear_m: 600.0,
            airport_max_rise_m: 45.0,
            airport_step_m: 500.0,
            airport_reach_m: 9000.0,
            port_coast_reach_m: 3000.0,
            port_land_share: 0.7,
            port_sea_share: 0.6,
            port_max_land_m: 30.0,
            port_slides_m: vec![0.0, -300.0, 300.0, -600.0, 600.0],
            port_city_clear_m: 20.0,
            sample_m: 50.0,
            ground_textures: BTreeMap::new(),
            ground_texture_m: 4.0,
        }
    }
}

impl Validate for SitesConfig {
    fn validate(&self) -> Result<(), String> {
        positive("sites.airport_city_clear_m", self.airport_city_clear_m)?;
        positive("sites.airport_base_clear_m", self.airport_base_clear_m)?;
        positive("sites.airport_max_rise_m", self.airport_max_rise_m)?;
        positive("sites.airport_step_m", self.airport_step_m)?;
        positive("sites.airport_reach_m", self.airport_reach_m)?;
        positive("sites.port_coast_reach_m", self.port_coast_reach_m)?;
        within("sites.port_land_share", self.port_land_share, 0.0, 1.0)?;
        within("sites.port_sea_share", self.port_sea_share, 0.0, 1.0)?;
        positive("sites.port_max_land_m", self.port_max_land_m)?;
        if self.port_slides_m.iter().any(|s| !s.is_finite()) {
            return Err("sites.port_slides_m: finite distances".into());
        }
        positive("sites.port_city_clear_m", self.port_city_clear_m)?;
        positive("sites.sample_m", self.sample_m)?;
        positive("sites.ground_texture_m", self.ground_texture_m)?;
        if self.ground_textures.values().any(|f| f.trim().is_empty()) {
            return Err("sites.ground_textures: every surface names a texture set's folder".into());
        }
        Ok(())
    }
}

/// Which site a template is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteKind {
    /// The civil airport (`airport-layout`).
    Airport,
    /// The seaport (`seaport-layout`).
    Seaport,
}

/// One built thing of a site: a kit at a point of the site's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SitePlacement {
    /// Its kit, an index into `Kits::kits`.
    pub kit: usize,
    /// Where the kit's origin stands (x, z) in the site's frame.
    pub at: Vec2,
    /// The height its ground stands at in the site's frame (the kit's y = 0).
    pub base_m: f32,
    /// Its quarter turns in the site's frame (1: its front, +z, to +x).
    pub turn: u8,
    /// True for a ship lying in the port.
    pub ship: bool,
}

/// A level patch of ground the site shapes: flat over its rect (the site's frame) at its height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SitePad {
    /// Least corner (x, z).
    pub lo: Vec2,
    /// Greatest corner (x, z).
    pub hi: Vec2,
    /// Its height in the site's frame.
    pub y_m: f32,
}

/// The look of one of the ground's surfaces (the page's stand-in, until its Material Maker set).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GroundMaterial {
    /// Base colour, linear.
    pub colour: [f32; 3],
    /// Roughness.
    pub roughness: f32,
    /// Metallic.
    pub metallic: f32,
    /// Emitted light, linear.
    pub emissive: [f32; 3],
}

/// A site's paving, paint, quays and breakwaters: one static glb, its surfaces and the solid boxes in it.
#[derive(Clone, Debug, PartialEq)]
pub struct SiteGround {
    /// Its glb under `assets/models/`.
    pub glb: String,
    /// Its primitives' surfaces, in order.
    pub surfaces: Vec<String>,
    /// Boxes standing out of the ground (quays, plinths) in the site's frame: walked on and collided with.
    pub solids: Vec<(Vec3, Vec3)>,
    /// Each surface's look.
    pub materials: BTreeMap<String, GroundMaterial>,
}

/// One of a site's floodlights (openspec change `night-voxel-light`, design section 2d), in its frame: a mast, or a
/// floodlight on something built (a gantry's boom, a ship's house).
#[derive(Clone, Debug, PartialEq)]
pub struct SiteMast {
    /// Its foot (x, z).
    pub at: Vec2,
    /// The height its foot stands at.
    pub base_m: f32,
    /// Its head over its foot, metres.
    pub h_m: f32,
    /// Where its light is aimed (x, y, z).
    pub aim: Vec3,
    /// Its light against a street lamp's per unit (`lamps.mast_lumens`); 0 lights nothing and only glows.
    pub power: f32,
    /// How far its light reaches, metres.
    pub range_m: f32,
    /// Its cone's half angle, radians.
    pub cone_rad: f32,
    /// Its light's colour.
    pub colour: LightColour,
    /// True for a mast with a pole and a head; false for a floodlight on something built, drawn by nothing.
    pub pole: bool,
    /// The placement it stands on (an index into `placements`): it goes out when that is damaged.
    pub lot: Option<u32>,
}

/// One of a site's glow dots (a runway light, a ship's anchor light), in its frame: a glow that lights nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct SiteDot {
    /// Where (x, y, z).
    pub at: Vec3,
    /// Its colour.
    pub colour: LightColour,
    /// The placement it stands on: it goes out when that is damaged.
    pub lot: Option<u32>,
}

/// A runway of a site, lit by the runway lights' rule (`lamps::runway_dots`), in its frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteRunway {
    /// One end of its centre line (x, z).
    pub a: Vec2,
    /// The other end (x, z).
    pub b: Vec2,
    /// Its paved width, metres.
    pub width_m: f32,
    /// Its surface's height.
    pub y_m: f32,
}

/// A site's night lights, as its page lays them out (its `nightLights`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SiteLights {
    /// Masts and floodlights.
    pub masts: Vec<SiteMast>,
    /// Glow dots.
    pub dots: Vec<SiteDot>,
    /// Runways.
    pub runways: Vec<SiteRunway>,
}

/// An aircraft its page parks at a stand: the game parks the civil traffic's model of its kind there.
#[derive(Clone, Debug, PartialEq)]
pub struct ParkedAircraft {
    /// Its kind, a key of `air_traffic.ron`'s `kinds`.
    pub kind: String,
    /// Where its wheels stand (x, y, z) in the site's frame.
    pub at: Vec3,
    /// Its nose's heading about the vertical, radians (`Quat::from_rotation_y`), in the site's frame.
    pub heading_rad: f32,
}

/// One site as its page laid it out.
#[derive(Clone, Debug, PartialEq)]
pub struct SiteTemplate {
    /// Which site.
    pub kind: SiteKind,
    /// Its folder under `assets/models/`.
    pub folder: String,
    /// What is built on it.
    pub placements: Vec<SitePlacement>,
    /// The ground it shapes.
    pub pads: Vec<SitePad>,
    /// Its ground.
    pub ground: SiteGround,
    /// Points the game reads off it, in its frame (x, z): the seaport's `terminal`, `berth` and `mouth`.
    pub marks: BTreeMap<String, Vec2>,
    /// Its night lights.
    pub lights: SiteLights,
    /// The aircraft parked at its stands.
    pub parked: Vec<ParkedAircraft>,
}

impl SiteTemplate {
    /// The pads' bounds in the site's frame (x, z): least and greatest.
    pub fn footprint(&self) -> (Vec2, Vec2) {
        self.pads
            .iter()
            .fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| {
                (lo.min(p.lo), hi.max(p.hi))
            })
    }
}

/// The templates of both sites, read with the kits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SiteTemplates {
    /// The airport, when shipped.
    pub airport: Option<Arc<SiteTemplate>>,
    /// The seaport, when shipped.
    pub seaport: Option<Arc<SiteTemplate>>,
}

#[derive(Deserialize)]
struct SiteJson {
    #[serde(default)]
    marks: BTreeMap<String, [f32; 2]>,
    pads: Vec<PadJson>,
    kits: Vec<serde_json::Value>,
    placements: Vec<PlacementJson>,
    ground: GroundJson,
    #[serde(default)]
    lights: LightsJson,
    #[serde(default)]
    parked: Vec<ParkedJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParkedJson {
    kind: String,
    at: [f32; 3],
    heading: f32,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LightsJson {
    masts: Vec<MastJson>,
    dots: Vec<DotJson>,
    runways: Vec<RunwayJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MastJson {
    x: f32,
    z: f32,
    base: f32,
    h: f32,
    aim: [f32; 3],
    power: f32,
    range: f32,
    cone: f32,
    colour: LightColour,
    pole: bool,
    #[serde(default)]
    lot: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DotJson {
    at: [f32; 3],
    colour: LightColour,
    #[serde(default)]
    lot: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunwayJson {
    a: [f32; 2],
    b: [f32; 2],
    w: f32,
    y: f32,
}

#[derive(Deserialize)]
struct PadJson {
    lo: [f32; 2],
    hi: [f32; 2],
    y: f32,
}

#[derive(Deserialize)]
struct PlacementJson {
    kit: String,
    x: f32,
    z: f32,
    base: f32,
    turn: u8,
    #[serde(default)]
    ship: bool,
    #[serde(default)]
    #[allow(dead_code)]
    anchor_built: bool,
}

#[derive(Deserialize)]
struct SolidJson {
    lo: [f32; 3],
    hi: [f32; 3],
}

#[derive(Deserialize)]
struct GroundJson {
    glb: String,
    surfaces: Vec<String>,
    solids: Vec<SolidJson>,
    materials: BTreeMap<String, GroundMaterial>,
}

impl SiteTemplates {
    /// Both sites under `dir` (`assets/models/sites`), their kits added to `kits`. An error names the file.
    pub fn load(dir: &Path, kits: &mut Kits) -> Result<Self, String> {
        Ok(Self {
            airport: Some(Arc::new(load_site(dir, "airport", SiteKind::Airport, kits)?)),
            seaport: Some(Arc::new(load_site(dir, "seaport", SiteKind::Seaport, kits)?)),
        })
    }
}

fn load_site(dir: &Path, name: &str, kind: SiteKind, kits: &mut Kits) -> Result<SiteTemplate, String> {
    let site_dir = dir.join(name);
    let path = site_dir.join("site.json");
    let what = path.display().to_string();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{what}: {e}"))?;
    let j: SiteJson = serde_json::from_str(&text).map_err(|e| format!("{what}: {e}"))?;
    let folder = format!("sites/{name}");
    let ids: Vec<String> = j
        .kits
        .iter()
        .map(|k| k.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string())
        .collect();
    let index = kits.add_site(&site_dir, &folder, j.kits)?;
    let kit_of = |id: &str| {
        ids.iter()
            .position(|k| k == id)
            .map(|i| index[i])
            .ok_or_else(|| format!("{what}: a placement names kit `{id}`, which the table does not list"))
    };
    let finite2 = |v: [f32; 2]| v.iter().all(|x| x.is_finite());
    let mut placements = Vec::with_capacity(j.placements.len());
    for p in &j.placements {
        if !(p.x.is_finite() && p.z.is_finite() && p.base.is_finite()) || p.turn > 3 {
            return Err(format!("{what}: placement of `{}` is not a finite point and a quarter turn", p.kit));
        }
        placements.push(SitePlacement {
            kit: kit_of(&p.kit)?,
            at: Vec2::new(p.x, p.z),
            base_m: p.base,
            turn: p.turn,
            ship: p.ship,
        });
    }
    if j.pads.is_empty() {
        return Err(format!("{what}: no pads: a site shapes its ground"));
    }
    let mut pads = Vec::with_capacity(j.pads.len());
    for p in &j.pads {
        if !finite2(p.lo) || !finite2(p.hi) || !p.y.is_finite() || p.lo[0] >= p.hi[0] || p.lo[1] >= p.hi[1] {
            return Err(format!("{what}: a pad is not a finite rect with its least corner first"));
        }
        pads.push(SitePad {
            lo: Vec2::from(p.lo),
            hi: Vec2::from(p.hi),
            y_m: p.y,
        });
    }
    let solids = j
        .ground
        .solids
        .iter()
        .map(|s| (Vec3::from(s.lo), Vec3::from(s.hi)))
        .collect::<Vec<_>>();
    if solids
        .iter()
        .any(|(lo, hi)| !lo.is_finite() || !hi.is_finite() || lo.cmpgt(*hi).any())
    {
        return Err(format!("{what}: a solid is not a finite box"));
    }
    for s in &j.ground.surfaces {
        if !j.ground.materials.contains_key(s) {
            return Err(format!("{what}: ground surface `{s}` has no material"));
        }
    }
    let marks: BTreeMap<String, Vec2> = j.marks.iter().map(|(k, v)| (k.clone(), Vec2::from(*v))).collect();
    if marks.values().any(|v| !v.is_finite()) {
        return Err(format!("{what}: a mark is not a finite point"));
    }
    let lights = lights_of(&j.lights, placements.len()).map_err(|e| format!("{what}: {e}"))?;
    let mut parked = Vec::with_capacity(j.parked.len());
    for a in &j.parked {
        if !a.at.iter().all(|v| v.is_finite()) || !a.heading.is_finite() || a.kind.is_empty() {
            return Err(format!("{what}: a parked `{}` is not a kind at a finite point and heading", a.kind));
        }
        parked.push(ParkedAircraft {
            kind: a.kind.clone(),
            at: Vec3::from(a.at),
            heading_rad: a.heading,
        });
    }
    if kind == SiteKind::Seaport {
        for m in ["terminal", "berth", "mouth"] {
            if !marks.contains_key(m) {
                return Err(format!("{what}: the seaport has no `{m}` mark"));
            }
        }
    }
    Ok(SiteTemplate {
        kind,
        folder: folder.clone(),
        placements,
        pads,
        ground: SiteGround {
            glb: format!("{folder}/{}", j.ground.glb),
            surfaces: j.ground.surfaces,
            solids,
            materials: j.ground.materials,
        },
        marks,
        lights,
        parked,
    })
}

/// A site table's lights, checked: every number finite, a mast's height and reach not negative, its cone under a
/// right angle, a runway's ends apart, and every `lot` one of the site's `placements`.
fn lights_of(j: &LightsJson, placements: usize) -> Result<SiteLights, String> {
    let lot = |l: Option<u32>| match l {
        Some(i) if i as usize >= placements => Err(format!("a light names placement {i}, past the {placements} there are")),
        l => Ok(l),
    };
    let mut out = SiteLights::default();
    for m in &j.masts {
        let finite = [m.x, m.z, m.base, m.h, m.power, m.range, m.cone]
            .iter()
            .chain(&m.aim)
            .all(|v| v.is_finite());
        if !finite || m.h < 0.0 || m.power < 0.0 || m.range < 0.0 || !(0.0..1.56).contains(&m.cone) {
            return Err(format!(
                "the mast at ({}, {}) is not a finite mast with a cone under a right angle",
                m.x, m.z
            ));
        }
        out.masts.push(SiteMast {
            at: Vec2::new(m.x, m.z),
            base_m: m.base,
            h_m: m.h,
            aim: Vec3::from(m.aim),
            power: m.power,
            range_m: m.range,
            cone_rad: m.cone,
            colour: m.colour,
            pole: m.pole,
            lot: lot(m.lot)?,
        });
    }
    for d in &j.dots {
        if !d.at.iter().all(|v| v.is_finite()) {
            return Err("a light dot is not a finite point".into());
        }
        out.dots.push(SiteDot {
            at: Vec3::from(d.at),
            colour: d.colour,
            lot: lot(d.lot)?,
        });
    }
    for r in &j.runways {
        let (a, b) = (Vec2::from(r.a), Vec2::from(r.b));
        if !(a.is_finite() && b.is_finite() && r.w.is_finite() && r.y.is_finite()) || r.w <= 0.0 || a.distance(b) < 1.0 {
            return Err("a runway is not two finite ends apart with a width".into());
        }
        out.runways.push(SiteRunway {
            a,
            b,
            width_m: r.w,
            y_m: r.y,
        });
    }
    Ok(out)
}

/// A site placed on the island.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedSite {
    /// Its template.
    pub template: Arc<SiteTemplate>,
    /// Where its frame's origin stands (x, z).
    pub origin: Vec2,
    /// Its quarter turns (1: the frame's +z to the world's +x).
    pub turn: u8,
    /// The world height of its frame's y = 0.
    pub ground_m: f32,
    /// The city it serves.
    pub city: u32,
    /// Its ground's solids in the world (least and greatest corners), worked out once it is placed.
    pub solids: Vec<(Vec3, Vec3)>,
    /// The island's lot of its first placement: its placements are the lots from here, in order.
    pub first_lot: u32,
}

impl PlacedSite {
    fn new(template: Arc<SiteTemplate>, origin: Vec2, turn: u8, ground_m: f32, city: u32) -> Self {
        let mut s = Self {
            template,
            origin,
            turn,
            ground_m,
            city,
            solids: Vec::new(),
            first_lot: 0,
        };
        s.solids = s.solids_in_world();
        s
    }

    /// Its turn about the vertical (Bevy's `Quat::from_rotation_y`), as a lot's heading.
    pub fn heading_rad(&self) -> f32 {
        f32::from(self.turn) * std::f32::consts::FRAC_PI_2
    }

    /// A point of its frame (x, z) in the world.
    pub fn to_world(&self, local: Vec2) -> Vec2 {
        self.origin + turn_by(local, self.turn)
    }

    /// A point of its frame (x, y, z) in the world.
    pub fn point_to_world(&self, local: Vec3) -> Vec3 {
        let p = self.to_world(Vec2::new(local.x, local.z));
        Vec3::new(p.x, self.ground_m + local.y, p.y)
    }

    /// The island's lot of its placement `placement`.
    pub fn lot_of(&self, placement: u32) -> u32 {
        self.first_lot + placement
    }

    /// A world point (x, z) in its frame.
    pub fn to_local(&self, world: Vec2) -> Vec2 {
        turn_by(world - self.origin, (4 - self.turn) % 4)
    }

    /// A rect of its frame in the world.
    pub fn rect_to_world(&self, lo: Vec2, hi: Vec2) -> Rect {
        let a = self.to_world(lo);
        let b = self.to_world(hi);
        Rect {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// A mark of its template in the world (x, z), if it has one.
    pub fn mark(&self, name: &str) -> Option<Vec2> {
        self.template.marks.get(name).map(|m| self.to_world(*m))
    }

    /// Its frame's +z (out to sea, for the seaport) in the world.
    pub fn out(&self) -> Vec2 {
        turn_by(Vec2::Y, self.turn)
    }

    /// Its pads in the world: each rect and its height.
    pub fn pads(&self) -> Vec<(Rect, f32)> {
        self.template
            .pads
            .iter()
            .map(|p| (self.rect_to_world(p.lo, p.hi), self.ground_m + p.y_m))
            .collect()
    }

    /// Its footprint in the world.
    pub fn footprint(&self) -> Rect {
        let (lo, hi) = self.template.footprint();
        self.rect_to_world(lo, hi)
    }

    fn solids_in_world(&self) -> Vec<(Vec3, Vec3)> {
        self.template
            .ground
            .solids
            .iter()
            .map(|(lo, hi)| {
                let r = self.rect_to_world(Vec2::new(lo.x, lo.z), Vec2::new(hi.x, hi.z));
                (
                    Vec3::new(r.min.x, self.ground_m + lo.y, r.min.y),
                    Vec3::new(r.max.x, self.ground_m + hi.y, r.max.y),
                )
            })
            .collect()
    }

    /// Its built things as lots on the island, in the template's order.
    pub fn lots(&self, kits: &Kits) -> Vec<Lot> {
        let zone = match self.template.kind {
            SiteKind::Airport => Zone::Airport,
            SiteKind::Seaport => Zone::Seaport,
        };
        self.template
            .placements
            .iter()
            .map(|p| {
                let k = &kits.kits[p.kit];
                let heading_rad = self.heading_rad() + f32::from(p.turn) * std::f32::consts::FRAC_PI_2;
                let centre = self.to_world(p.at);
                Lot {
                    kit: p.kit as u16,
                    centre,
                    ground_m: self.ground_m + p.base_m,
                    heading_rad,
                    rect: super::small_buildings::lots::footprint(
                        Vec2::new(k.lo.x, k.lo.z),
                        Vec2::new(k.hi.x, k.hi.z),
                        centre,
                        heading_rad,
                        0.0,
                    ),
                    zone,
                }
            })
            .collect()
    }
}

/// `v` turned by `turn` quarter turns the way a lot is (one turn takes +z to +x).
fn turn_by(v: Vec2, turn: u8) -> Vec2 {
    match turn % 4 {
        0 => v,
        1 => Vec2::new(v.y, -v.x),
        2 => -v,
        _ => Vec2::new(-v.y, v.x),
    }
}

/// What the sites keep clear of.
pub struct Clear<'a> {
    /// The cities.
    pub cities: &'a [City],
    /// The air bases.
    pub bases: &'a [AirBase],
    /// The highways and rivers.
    pub keep: &'a KeepClear<'a>,
    /// True when a lake comes near the box from the first corner to the second.
    pub water_near: &'a dyn Fn(Vec2, Vec2) -> bool,
    /// Built ground is at least this high (above the sea's freeboard).
    pub min_ground_m: f32,
    /// The sea's level.
    pub sea_level_m: f32,
}

/// Sample points `step_m` apart over `r`, edges included.
fn samples(r: &Rect, step_m: f32) -> Vec<Vec2> {
    let size = r.max - r.min;
    let (nx, nz) = (
        (size.x / step_m).ceil().max(1.0) as usize,
        (size.y / step_m).ceil().max(1.0) as usize,
    );
    let mut out = Vec::with_capacity((nx + 1) * (nz + 1));
    for i in 0..=nx {
        for j in 0..=nz {
            out.push(r.min + Vec2::new(size.x * i as f32 / nx as f32, size.y * j as f32 / nz as f32));
        }
    }
    out
}

/// True when `r` keeps `m` from every base's site and runway.
fn clear_of_bases(r: &Rect, bases: &[AirBase], m: f32) -> bool {
    bases.iter().all(|b| {
        let grow = |x: Rect| Rect {
            min: x.min - Vec2::splat(m),
            max: x.max + Vec2::splat(m),
        };
        !grow(b.rect()).overlaps(r) && !grow(b.runway.rect()).overlaps(r)
    })
}

/// The civil airport: beside the largest city it fits by, its middle tried on rings round each city in turn, out from
/// the square in `airport_step_m` steps to `airport_reach_m`, a step apart round each ring, every quarter turn; on dry
/// land flat enough to level, clear of the cities, the bases, the highways and the lakes (`Clear::water_near`). Its ground is the mean of the land under it.
pub fn place_airport(shape: &Shape, tpl: &Arc<SiteTemplate>, clear: &Clear, cfg: &SitesConfig) -> Option<PlacedSite> {
    place_airport_counted(shape, tpl, clear, cfg).0
}

/// Why airport candidates were turned down: too near a city, a base, off dry land, too steep, by water, by a highway.
pub type Rejections = [u32; 6];

/// `place_airport`, with how many candidates each rule turned down before the one taken (a measurement instrument for
/// `cb-tools sites`).
pub fn place_airport_counted(shape: &Shape, tpl: &Arc<SiteTemplate>, clear: &Clear, cfg: &SitesConfig) -> (Option<PlacedSite>, Rejections) {
    let mut why: Rejections = [0; 6];
    let (lo, hi) = tpl.footprint();
    let half_diag = (hi - lo).length() * 0.5;
    let mid = (lo + hi) * 0.5;
    let roads = KeepClear {
        courses: &[],
        ..*clear.keep
    };
    for city in clear.cities {
        let half = city.rect().size().max_element() * 0.5;
        let mut ring = half + cfg.airport_city_clear_m + half_diag * 0.5;
        while ring <= half + cfg.airport_reach_m {
            let around = ((ring * std::f32::consts::TAU / cfg.airport_step_m).ceil() as usize).max(8);
            for b in 0..around {
                let ang = b as f32 / around as f32 * std::f32::consts::TAU;
                let c = city.centre + Vec2::new(ang.cos(), ang.sin()) * ring;
                for turn in 0..4u8 {
                    let site = PlacedSite::new(tpl.clone(), c - turn_by(mid, turn), turn, 0.0, city.id);
                    let r = site.footprint();
                    let grow = |x: Rect, m: f32| Rect {
                        min: x.min - Vec2::splat(m),
                        max: x.max + Vec2::splat(m),
                    };
                    if clear.cities.iter().any(|k| grow(k.rect(), cfg.airport_city_clear_m).overlaps(&r)) {
                        why[0] += 1;
                        continue;
                    }
                    if !clear_of_bases(&r, clear.bases, cfg.airport_base_clear_m) {
                        why[1] += 1;
                        continue;
                    }
                    let pts = samples(&r, cfg.sample_m * 2.0);
                    let mut hs = Vec::with_capacity(pts.len());
                    for p in &pts {
                        let h = shape.height(*p);
                        if shape.past_land_edge_m(*p) >= 0.0 || h < clear.min_ground_m {
                            break;
                        }
                        hs.push(h);
                    }
                    if hs.len() < pts.len() {
                        why[2] += 1;
                        continue;
                    }
                    let (a, z) = hs
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), h| (a.min(*h), b.max(*h)));
                    if z - a > cfg.airport_max_rise_m {
                        why[3] += 1;
                        continue;
                    }
                    if (clear.water_near)(r.min, r.max) {
                        why[4] += 1;
                        continue;
                    }
                    if !super::bases::keeps_clear(&r, &roads) {
                        why[5] += 1;
                        continue;
                    }
                    let mean = hs.iter().sum::<f32>() / hs.len() as f32;
                    // The pad levels the ground to its height in the frame, so y = 0 sits that much over the mean.
                    let pad_y = tpl.pads.first().map_or(0.0, |p| p.y_m);
                    let placed = PlacedSite::new(tpl.clone(), site.origin, turn, mean.max(clear.min_ground_m) - pad_y, city.id);
                    return (Some(placed), why);
                }
            }
            ring += cfg.airport_step_m;
        }
    }
    (None, why)
}

/// The seaport: on the coast of the largest city with a fitting shore (cities in their order), its frame's z = 0 on
/// the shoreline and its +z out to sea, turned the quarter turn nearest the way out, slid along the shore; the one whose
/// land is most land and whose basin most sea. Its ground (y = 0) is the sea's level. `airport` is kept clear of.
pub fn place_seaport(
    shape: &Shape,
    tpl: &Arc<SiteTemplate>,
    clear: &Clear,
    airport: Option<&Rect>,
    cfg: &SitesConfig,
) -> Option<PlacedSite> {
    let sea = clear.sea_level_m;
    for city in clear.cities {
        let mut best: Option<(f32, PlacedSite)> = None;
        let half = city.rect().size().max_element() * 0.5;
        for b in 0..32 {
            let ang = b as f32 / 32.0 * std::f32::consts::TAU;
            let dir = Vec2::new(ang.cos(), ang.sin());
            // Walk out to the first water: the shore is the last dry step.
            let mut shore = None;
            let mut t = half;
            while t <= half + cfg.port_coast_reach_m {
                let p = city.centre + dir * t;
                if shape.height(p) < sea {
                    shore = Some(city.centre + dir * (t - 20.0));
                    break;
                }
                t += 20.0;
            }
            let Some(shore) = shore else { continue };
            // The quarter turn whose +z points nearest the way out.
            let turn = (0..4u8)
                .max_by(|&a, &b| turn_by(Vec2::Y, a).dot(dir).total_cmp(&turn_by(Vec2::Y, b).dot(dir)))
                .unwrap_or(0);
            let side = turn_by(Vec2::X, turn);
            for &slide in &cfg.port_slides_m {
                let site = PlacedSite::new(tpl.clone(), shore + side * slide, turn, sea, city.id);
                let r = site.footprint();
                let grow = |x: Rect, m: f32| Rect {
                    min: x.min - Vec2::splat(m),
                    max: x.max + Vec2::splat(m),
                };
                if clear.cities.iter().any(|k| grow(k.rect(), cfg.port_city_clear_m).overlaps(&r))
                    || !clear_of_bases(&r, clear.bases, cfg.airport_base_clear_m)
                    || airport.is_some_and(|a| grow(*a, 200.0).overlaps(&r))
                {
                    continue;
                }
                let (mut land, mut land_n, mut wet, mut wet_n, mut ok) = (0usize, 0usize, 0usize, 0usize, true);
                for (rect, y) in site.pads() {
                    let pts = samples(&rect, cfg.sample_m);
                    if y > sea {
                        // Land: dry, not too high to cut to the deck, and clear of the highways and rivers.
                        if !super::bases::keeps_clear(&rect, clear.keep) {
                            ok = false;
                            break;
                        }
                        for p in &pts {
                            let h = shape.height(*p);
                            land_n += 1;
                            if h >= sea && shape.past_land_edge_m(*p) < 0.0 {
                                land += 1;
                            }
                            if h > sea + cfg.port_max_land_m {
                                ok = false;
                            }
                        }
                    } else if rect.size().x * rect.size().y > 1.0e5 {
                        // The basin: mostly open water.
                        for p in &pts {
                            wet_n += 1;
                            if shape.height(*p) < sea - 1.0 {
                                wet += 1;
                            }
                        }
                    }
                }
                if !ok || land_n == 0 || wet_n == 0 {
                    continue;
                }
                let (ls, ws) = (land as f32 / land_n as f32, wet as f32 / wet_n as f32);
                if ls < cfg.port_land_share || ws < cfg.port_sea_share {
                    continue;
                }
                let score = ls + ws - slide.abs() * 1e-5;
                if best.as_ref().is_none_or(|(s, _)| score > *s) {
                    best = Some((score, site));
                }
            }
        }
        if let Some((_, s)) = best {
            return Some(s);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> (Kits, SiteTemplates) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
        let mut kits = Kits::load(&root.join("small_buildings")).expect("the house kits load");
        let sites = SiteTemplates::load(&root.join("sites"), &mut kits).expect("the site kits load");
        (kits, sites)
    }

    #[test]
    fn both_sites_load_with_every_kit_they_place() {
        let (kits, sites) = shipped();
        for t in [sites.airport.unwrap(), sites.seaport.unwrap()] {
            assert!(!t.placements.is_empty());
            for p in &t.placements {
                let k = &kits.kits[p.kit];
                assert!(k.kind.is_site(), "{} is a site's kit", k.id);
                assert!(k.folder.starts_with("sites/"), "{} under sites/", k.id);
                assert!(!k.pieces.is_empty(), "{} has pieces", k.id);
            }
            assert!(!t.ground.surfaces.is_empty());
        }
    }

    #[test]
    fn both_sites_carry_their_pages_night_lights() {
        let (kits, sites) = shipped();
        let (airport, seaport) = (sites.airport.unwrap(), sites.seaport.unwrap());
        assert_eq!(airport.lights.runways.len(), 2, "the main and the cross runway");
        assert!(
            airport.lights.masts.iter().filter(|m| m.h_m >= 25.0).count() >= 10,
            "a mast at every gate"
        );
        assert!(airport.lights.dots.iter().any(|d| d.colour == LightColour::Blue), "taxiway lights");
        assert!(seaport.lights.runways.is_empty());
        // Every ship's lights stand on that ship.
        for (i, p) in seaport.placements.iter().enumerate().filter(|(_, p)| p.ship) {
            let on = seaport.lights.dots.iter().filter(|d| d.lot == Some(i as u32)).count();
            assert!(on > 0, "{} carries its lights", kits.kits[p.kit].id);
        }
        for d in &seaport.lights.dots {
            if let Some(lot) = d.lot {
                let p = &seaport.placements[lot as usize];
                let k = &kits.kits[p.kit];
                let reach = (k.hi - k.lo).length() + 5.0;
                assert!(Vec2::new(d.at.x, d.at.z).distance(p.at) < reach, "a light of {} stands on it", k.id);
            }
        }
    }

    #[test]
    fn a_light_naming_a_placement_past_the_last_is_refused() {
        let j: LightsJson =
            serde_json::from_str(r#"{"masts": [], "dots": [{"at": [0, 1, 0], "colour": "red", "lot": 3}], "runways": []}"#).unwrap();
        assert!(lights_of(&j, 3).is_err());
        assert!(lights_of(&j, 4).is_ok());
    }

    #[test]
    fn an_unknown_light_colour_is_refused() {
        let j = serde_json::from_str::<LightsJson>(r#"{"masts": [], "dots": [{"at": [0, 1, 0], "colour": "mauve"}], "runways": []}"#);
        assert!(j.is_err());
    }

    #[test]
    fn every_site_building_stands_as_built() {
        let (kits, sites) = shipped();
        for t in [sites.airport.unwrap(), sites.seaport.unwrap()] {
            let mut seen = std::collections::BTreeSet::new();
            for p in &t.placements {
                if !seen.insert(p.kit) {
                    continue;
                }
                let k = &kits.kits[p.kit];
                let stands = super::super::small_buildings::damage::standing(k, &vec![true; k.pieces.len()]);
                let down = stands.iter().filter(|s| !**s).count();
                assert_eq!(down, 0, "{}: {down} of {} pieces do not stand as built", k.id, k.pieces.len());
            }
        }
    }

    #[test]
    fn the_seaport_has_its_ships_and_the_airport_its_terminals() {
        let (kits, sites) = shipped();
        let sea = sites.seaport.unwrap();
        let ships: Vec<&str> = sea
            .placements
            .iter()
            .filter(|p| p.ship)
            .map(|p| kits.kits[p.kit].id.as_str())
            .collect();
        for s in ["freighter", "tanker", "ferry", "tug", "gunboat"] {
            assert!(ships.contains(&s), "the port has a {s}: {ships:?}");
        }
        let air = sites.airport.unwrap();
        let ids: Vec<&str> = air.placements.iter().map(|p| kits.kits[p.kit].id.as_str()).collect();
        for s in ["terminal_1", "terminal_2", "control_tower", "fire_station"] {
            assert!(ids.contains(&s), "the airport has {s}");
        }
    }

    #[test]
    fn a_freighter_holed_at_its_waterline_sinks_whole_breaks_on_the_way_down_and_is_gone_under_the_seabed() {
        use super::super::small_buildings::{Houses, Lots, Placed};
        let (kits, sites) = shipped();
        let sea = sites.seaport.unwrap();
        let p = sea
            .placements
            .iter()
            .find(|p| p.ship && kits.kits[p.kit].id == "freighter")
            .unwrap();
        let k = kits.kits[p.kit].clone();
        let draft = k.waterline_m.expect("a ship has its waterline");
        // The freighter afloat on still water at y = 0 over a seabed 14 m down.
        let lot = Lot {
            kit: p.kit as u16,
            centre: Vec2::ZERO,
            ground_m: -draft,
            heading_rad: 0.0,
            rect: super::super::small_buildings::lots::footprint(
                Vec2::new(k.lo.x, k.lo.z),
                Vec2::new(k.hi.x, k.hi.z),
                Vec2::ZERO,
                0.0,
                0.0,
            ),
            zone: Zone::Seaport,
        };
        let lots = Lots::new(
            Arc::new(kits),
            Placed {
                lots: vec![lot],
                ..Default::default()
            },
            4096.0,
        );
        let mut h = Houses::default();
        // A shell into its side, at the waterline.
        let side = Vec3::new(k.hi.x, 0.0, (k.lo.z + k.hi.z) * 0.5);
        let first = h.break_sphere(&lots, 0, side, 2.0);
        assert!(!first.pieces.is_empty());
        assert!(h.states[&0].sinking.is_some(), "holed at the waterline, it goes down");
        let seabed = |_: Vec2| -14.0;
        for _ in 0..600 {
            h.step_sinking(&lots, 1.0 / 60.0, &seabed);
        }
        let s = h.states[&0].sinking.unwrap();
        assert!(s.drop_m > 1.0, "ten seconds on it has gone {} m down", s.drop_m);
        let alive = h.alive(0).unwrap().iter().filter(|a| **a).count();
        assert!(alive > first.pieces.len(), "it goes down whole, not in pieces");
        // Shot as it goes down: the hit lands on the ship where it is now.
        let deck = Vec3::new((k.lo.x + k.hi.x) * 0.5, k.hi.y * 0.5 - draft - s.drop_m, (k.lo.z + k.hi.z) * 0.5);
        let again = h.break_sphere(&lots, 0, deck, 3.0);
        assert!(!again.pieces.is_empty(), "a sinking ship still breaks piece by piece");
        let mut gone = false;
        for _ in 0..(60 * 60 * 10) {
            let (_, g) = h.step_sinking(&lots, 1.0 / 60.0, &seabed);
            if !g.is_empty() {
                gone = true;
                break;
            }
        }
        assert!(gone, "under the seabed it is gone");
        assert!(h.alive(0).unwrap().iter().all(|a| !a));
    }

    #[test]
    fn a_freighter_under_way_breaks_where_it_is_founders_when_holed_and_goes_down_as_the_quay_s_do() {
        use crate::ships::{Boat, BoatSpec, Errand, Phase};
        let (kits, _) = shipped();
        let k = kits
            .ship("freighter")
            .map(|i| kits.kits[i].clone())
            .expect("the seaport's freighter kit");
        let w = k.waterline_m.unwrap();
        let spec = BoatSpec {
            size_m: [18.0, 26.0, 120.0],
            draft_m: w,
            speed_m_s: 7.0,
            accel_m_s2: 0.05,
            turn_deg_s: 2.0,
            hp: 4000.0,
            armour_mm: 10.0,
            crew: 20,
            sink_s: 90.0,
            model: String::new(),
            tint: [0.2, 0.26, 0.34],
            burns: false,
            sink_hull_share: 0.25,
        };
        let mut b = Boat::new(
            7,
            "freighter",
            &spec,
            Errand::Anchor { at: Vec2::ZERO },
            Vec2::new(500.0, 300.0),
            1.0,
        );
        let mid = (k.lo.z + k.hi.z) * 0.5;
        // Its highest piece (the bridge's top), well over its waterline.
        let high = k
            .pieces
            .iter()
            .max_by(|a, b| a.hi.y.total_cmp(&b.hi.y))
            .map(|p| (p.lo + p.hi) * 0.5)
            .unwrap();
        // It has sailed on 60 m: a shot where it was misses, a shot into its upperworks where it is breaks pieces and
        // leaves it afloat.
        let was = b.kit_pose(&k).transform_point3(high);
        b.pos += crate::ships::ahead(b.heading_rad) * 60.0;
        assert!(b.break_sphere(&k, was, 1.5).pieces.is_empty(), "a shot where it was misses");
        let top = b.kit_pose(&k).transform_point3(high);
        assert!(!b.break_sphere(&k, top, 1.5).pieces.is_empty(), "a shot where it is breaks it");
        assert!(b.afloat(), "broken above its waterline it sails on");
        // A cargo ship takes a quarter of its hull at and under its waterline gone before it goes down (ships.ron's
        // `sink_hull_share`): one hole and it sails on; holed along its side until a quarter is gone, it founders and
        // goes down whole, as a ship at the quays does.
        let mut k = k;
        k.sink_share = 0.25;
        let side = |b: &Boat, z: f32| b.kit_pose(&k).transform_point3(Vec3::new(k.hi.x, w * 0.5, z));
        assert!(!b.break_sphere(&k, side(&b, mid), 2.0).pieces.is_empty());
        assert!(b.afloat(), "one hole in a cargo ship and it sails on");
        let mut z = k.lo.z + 2.0;
        let mut x = k.hi.x;
        while b.afloat() {
            let at = b.kit_pose(&k).transform_point3(Vec3::new(x, w * 0.5, z));
            b.break_sphere(&k, at, 3.0);
            z += 4.0;
            if z > k.hi.z {
                z = k.lo.z + 2.0;
                x -= 4.0;
                assert!(x > k.lo.x - 4.0, "holed through and through it never sank");
            }
        }
        let share = super::super::small_buildings::damage::holed_share(&k, &b.pieces.as_ref().unwrap().alive).unwrap();
        assert!(
            (0.25..0.5).contains(&share),
            "it went down at a quarter of its hull gone, not {share}"
        );
        assert!(matches!(b.phase, Phase::Sinking { .. }), "holed enough it founders");
        let mut steps = 0;
        while b.phase != Phase::Gone && steps < 60 * 60 * 20 {
            b.step_kit(&k, -20.0, 1.0 / 60.0);
            steps += 1;
        }
        assert_eq!(b.phase, Phase::Gone, "under the seabed it is gone");
        assert!(b.pieces.as_ref().unwrap().alive.iter().all(|a| !a));
        // One sunk by its hit points (no hole) still goes down through its kit.
        let mut c = Boat::new(8, "freighter", &spec, Errand::Anchor { at: Vec2::ZERO }, Vec2::ZERO, 0.0);
        assert!(c.founder());
        c.step_kit(&k, -20.0, 1.0 / 60.0);
        assert!(c.pieces.as_ref().is_some_and(|s| s.sinking.is_some()));
    }

    #[test]
    fn a_quarter_turn_takes_the_frame_s_z_to_the_world_s_x() {
        assert_eq!(turn_by(Vec2::Y, 1), Vec2::X);
        for t in 0..4 {
            let v = Vec2::new(3.0, -7.0);
            assert!((turn_by(turn_by(v, t), (4 - t) % 4) - v).length() < 1e-5);
        }
    }
}
