//! The exported kits of the houses and small businesses (`assets/models/small_buildings/`, written by
//! `tools/buildings/export_small_buildings.cjs` from `docs/mockups/small_buildings_kit.js`): every
//! type's bounds and its piece table, read as the game's data (CLAUDE.md section 6.5: the real
//! artifact, never a copy). A piece is what breaks off together; the table says what it is made of,
//! where its triangles are, what holds it up, and which pieces are door leaves and furniture.

use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::path::Path;

/// What a type is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A house: suburbs, towns' back streets, villages.
    House,
    /// A business: towns' main streets and roadside stops.
    Business,
    /// A building of a site (the airport, the seaport: openspec changes `airport-layout`, `seaport-layout`).
    Building,
    /// A steel machine of a site (a crane, a gantry, a boom): it hangs further from its support than masonry.
    Machine,
    /// A ship (the seaport's): a hollow hull, holed low it goes down (`seaport-layout` section 2).
    Ship,
}

impl Kind {
    /// True for the kinds a site places (not the houses' and businesses' lots).
    pub fn is_site(self) -> bool {
        matches!(self, Kind::Building | Kind::Machine | Kind::Ship)
    }
}

/// Which node of the glb a piece's triangles are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Node {
    /// The building as drawn within 150 m.
    Lod0,
    /// The block furniture, drawn within 100 m.
    Furn,
    /// A door leaf, in its hinge's frame (the leaf's index).
    Leaf(u16),
}

/// What a broken piece turns into (design section 6).
#[derive(Clone, Debug, PartialEq)]
pub enum Chunking {
    /// A lone box of one surface: chunks of it.
    Box {
        /// Its surface key.
        surface: String,
        /// Its paint, linear.
        tint: [f32; 3],
    },
    /// A roof or floor panel: falls whole, as one tilted chunk.
    Panel {
        /// Its surface key.
        surface: String,
        /// Its paint, linear.
        tint: [f32; 3],
    },
    /// Anything else (a window with its glass, a sofa, a car): falls whole.
    Whole,
}

/// One breakable piece of a type, in the building's frame (front +z, centred on x = 0, ground y = 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// Least corner.
    pub lo: Vec3,
    /// Greatest corner.
    pub hi: Vec3,
    /// Its triangles: (primitive in its node, first triangle, count).
    pub runs: Vec<[u32; 3]>,
    /// The node they are in.
    pub node: Node,
    /// Interior dressing or finish (drawn within LOD0 only).
    pub interior: bool,
    /// Furniture (block furniture; its real model replaces it on foot).
    pub furniture: bool,
    /// What it becomes when it breaks.
    pub chunking: Chunking,
    /// Its share of load-bearing wall, for the collapse rule (0 when it is not one).
    pub wall_area_m2: f32,
    /// Pieces it holds up (indices into `Kit::pieces`).
    pub up: Vec<u32>,
    /// Pieces beside it that can hold it (indices).
    pub side: Vec<u32>,
    /// The boxes it is made of, when it is only boxes; else its bounds stand in. For colliders.
    pub boxes: Vec<(Vec3, Vec3)>,
    /// Every surface it is drawn with.
    pub surfaces: Vec<String>,
}

impl Piece {
    /// The boxes a body meets: its own boxes, or its bounds.
    pub fn solid_boxes(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        let own = if self.boxes.is_empty() {
            None
        } else {
            Some(self.boxes.iter().copied())
        };
        let bounds = own.is_none().then_some((self.lo, self.hi));
        own.into_iter().flatten().chain(bounds)
    }

    /// True when a point in the building's frame is inside one of its boxes.
    pub fn contains(&self, p: Vec3) -> bool {
        self.solid_boxes().any(|(lo, hi)| p.cmpge(lo).all() && p.cmple(hi).all())
    }
}

/// A door leaf: where it hangs and which way it opens, in the building's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct LeafDef {
    /// Its piece (index).
    pub piece: u32,
    /// The hinge, at the middle of the wall's depth, on the sill.
    pub hinge: Vec3,
    /// Unit (x, z): hinge to free edge, shut.
    pub shut: Vec2,
    /// Unit (x, z): into the building.
    pub inward: Vec2,
    /// Hinge to free edge.
    pub width_m: f32,
    /// Sill to head.
    pub height_m: f32,
}

/// A piece of block furniture and where it stands, so a modelled piece can take its place.
#[derive(Clone, Debug, PartialEq)]
pub struct FurnDef {
    /// Its piece (index).
    pub piece: u32,
    /// Its kind (`sofa`, `bed`, ...: the kit's `FURN` keys).
    pub item: String,
    /// Its origin, in the building's frame.
    pub at: Vec3,
    /// Quarter turns of its local frame: local (x, z) lies at (-z, x) for one turn.
    pub quarter_turns: u8,
    /// The paint of its tinted surface, linear.
    pub tint: [f32; 3],
}

/// One exported type (or a variant of one).
#[derive(Clone, Debug, PartialEq)]
pub struct Kit {
    /// Its id (`ranch`, `ranch_b`, ...): its glb is `<folder>/<id>.glb`.
    pub id: String,
    /// The folder of its glb under `assets/models/` (`small_buildings`, `sites/airport`, ...).
    pub folder: String,
    /// Its name.
    pub name: String,
    /// House or business.
    pub kind: Kind,
    /// Where it stands (the manifest's words: `temperate`, `any roadside`, `towns`...).
    pub biomes: Vec<String>,
    /// The type it is a variant of, if it is one.
    pub variant_of: Option<String>,
    /// Least corner of everything it draws.
    pub lo: Vec3,
    /// Greatest corner.
    pub hi: Vec3,
    /// LOD0's primitives' surfaces, in order.
    pub primitives: Vec<String>,
    /// FURN's primitives' surfaces, in order.
    pub furn_primitives: Vec<String>,
    /// The pieces.
    pub pieces: Vec<Piece>,
    /// The door leaves.
    pub leaves: Vec<LeafDef>,
    /// The block furniture.
    pub furniture: Vec<FurnDef>,
    /// Most side links between a piece and one on something (the standing rule).
    pub max_side_steps: u32,
    /// The share of wall area below which it collapses.
    pub collapse_wall_share: f32,
    /// Its whole wall area.
    pub wall_area_m2: f32,
    /// How it is reached from the road (openspec change `lot-access`).
    pub access: Access,
    /// A ship's waterline over its keel (its frame's y = 0), metres; `None` for anything that does not float.
    pub waterline_m: Option<f32>,
}

/// How a kit is reached from the road (openspec change `lot-access`), in its frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Access {
    /// A house: its drive's slab runs to the kit's front edge (`hi.z`) centred on `x_m`, `width_m` wide; the
    /// game paves it on from there to the street.
    Drive {
        /// The drive's middle across the kit.
        x_m: f32,
        /// Its width.
        width_m: f32,
    },
    /// A business: the game's car park starts at `z_m` (the building's front, or the end of the kit's own
    /// apron) and runs out toward the road; striped with stalls unless `stalls` is false (a gas station).
    Lot {
        /// Where the car park starts.
        z_m: f32,
        /// Whether it is striped with stalls.
        stalls: bool,
    },
    /// A site's: its site lays out its own paving.
    Site,
}

impl Kit {
    /// The footprint (x, z) in its frame: least and greatest.
    pub fn footprint(&self) -> (Vec2, Vec2) {
        (Vec2::new(self.lo.x, self.lo.z), Vec2::new(self.hi.x, self.hi.z))
    }

    /// True when the manifest says it stands in a place called `word`.
    pub fn stands_in(&self, word: &str) -> bool {
        self.biomes.iter().any(|b| b == word)
    }
}

/// One modelled piece of furniture (`assets/models/furniture.json`, `tools/blender/build_furniture.py`).
#[derive(Clone, Debug, PartialEq)]
pub struct FurnItem {
    /// Its glb, under `assets/`.
    pub file: String,
    /// The block it replaces, least corner (its local frame).
    pub box_lo: Vec3,
    /// Greatest corner.
    pub box_hi: Vec3,
    /// What it models fills, least corner.
    pub lo: Vec3,
    /// Greatest corner.
    pub hi: Vec3,
    /// The surface the placement's tint colours, if any.
    pub tint_surface: Option<String>,
    /// Its triangles.
    pub triangles: u32,
}

/// The modelled furniture and its one atlas: each item by the kit's `FURN` key, and the atlas's flat
/// swatches of each surface for the block furniture (UV rect u0, v0, u1, v1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Furniture {
    /// The items.
    pub items: std::collections::BTreeMap<String, FurnItem>,
    /// The swatches, by surface key.
    pub swatches: std::collections::BTreeMap<String, [f32; 4]>,
}

#[derive(Deserialize)]
struct FurnBoundsJson {
    lo: [f32; 3],
    hi: [f32; 3],
}

#[derive(Deserialize)]
struct FurnItemJson {
    file: String,
    box_m: FurnBoundsJson,
    bounds_m: FurnBoundsJson,
    #[serde(default)]
    tint_surface: Option<String>,
    triangles: u32,
}

#[derive(Deserialize)]
struct FurnitureJson {
    items: std::collections::BTreeMap<String, FurnItemJson>,
    swatches: std::collections::BTreeMap<String, [f32; 4]>,
}

impl Furniture {
    /// `furniture.json` at `path`. An error names it.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let j: FurnitureJson = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let items = j
            .items
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    FurnItem {
                        file: v.file,
                        box_lo: Vec3::from(v.box_m.lo),
                        box_hi: Vec3::from(v.box_m.hi),
                        lo: Vec3::from(v.bounds_m.lo),
                        hi: Vec3::from(v.bounds_m.hi),
                        tint_surface: v.tint_surface.filter(|t| t != "none" && !t.is_empty()),
                        triangles: v.triangles,
                    },
                )
            })
            .collect();
        Ok(Self {
            items,
            swatches: j.swatches,
        })
    }
}

/// Every kit, in the manifest's order, and the modelled furniture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Kits {
    /// The kits.
    pub kits: Vec<Kit>,
    /// The modelled furniture that stands in for the kits' block furniture.
    pub furniture: Furniture,
}

#[derive(Deserialize)]
struct ManifestJson {
    types: Vec<TypeJson>,
}

#[derive(Deserialize)]
struct BoundsJson {
    lo: [f32; 3],
    hi: [f32; 3],
}

#[derive(Deserialize)]
struct TypeJson {
    id: String,
    name: String,
    kind: String,
    #[serde(default)]
    biomes: Vec<String>,
    pieces: String,
    bounds_m: BoundsJson,
    #[serde(default)]
    variant_of: Option<String>,
    #[serde(default)]
    access: Option<AccessJson>,
    #[serde(default)]
    waterline_m: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessJson {
    #[serde(default)]
    drive: Option<[f32; 2]>,
    #[serde(default)]
    lot: Option<f32>,
    #[serde(default = "yes")]
    stalls: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct PiecesJson {
    max_side_steps: u32,
    collapse_wall_share: f32,
    primitives: Vec<String>,
    #[serde(default)]
    furn_primitives: Vec<String>,
    #[serde(default)]
    leaves: Vec<LeafJson>,
    #[serde(default)]
    furniture: Vec<FurnJson>,
    pieces: Vec<PieceJson>,
}

#[derive(Deserialize)]
struct LeafJson {
    piece: u32,
    hinge: [f32; 3],
    shut: [f32; 2],
    inward: [f32; 2],
    width_m: f32,
    height_m: f32,
}

#[derive(Deserialize)]
struct FurnJson {
    piece: u32,
    item: String,
    at: [f32; 3],
    rot: u8,
    tint: [f32; 3],
}

#[derive(Deserialize)]
struct MatJson {
    mat: String,
    tint: [f32; 3],
}

#[derive(Deserialize)]
struct PieceJson {
    id: u32,
    lo: [f32; 3],
    hi: [f32; 3],
    runs: Vec<[u32; 3]>,
    #[serde(default)]
    node: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    interior: bool,
    #[serde(default, rename = "box")]
    box_: Option<MatJson>,
    #[serde(default)]
    panel: Option<MatJson>,
    #[serde(default)]
    wall_area_m2: Option<f32>,
    up: Vec<u32>,
    side: Vec<u32>,
    #[serde(default)]
    boxes: Option<Vec<[f32; 6]>>,
}

fn finite3(v: [f32; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// One kit from its manifest entry and its piece table's text (`what` names it in errors).
fn kit_of(t: TypeJson, table: &str, what: &str, folder: &str) -> Result<Kit, String> {
    let table: PiecesJson = serde_json::from_str(table).map_err(|e| format!("{what}: {e}"))?;
    let index: std::collections::HashMap<u32, u32> = table.pieces.iter().enumerate().map(|(i, p)| (p.id, i as u32)).collect();
    let at = |id: u32, field: &str| {
        index
            .get(&id)
            .copied()
            .ok_or_else(|| format!("{what}: {field} names piece {id}, which is not in the table"))
    };
    let kind = match t.kind.as_str() {
        "house" => Kind::House,
        "business" => Kind::Business,
        "building" => Kind::Building,
        "machine" => Kind::Machine,
        "ship" => Kind::Ship,
        k => return Err(format!("{what}: kind `{k}` is not house, business, building, machine or ship")),
    };
    let leaf_of: std::collections::HashMap<u32, u16> = table.leaves.iter().enumerate().map(|(i, l)| (l.piece, i as u16)).collect();
    let mut pieces = Vec::with_capacity(table.pieces.len());
    for p in &table.pieces {
        if !finite3(p.lo) || !finite3(p.hi) {
            return Err(format!("{what}: piece {} has non-finite bounds", p.id));
        }
        let node = match p.node.as_deref() {
            None | Some("LOD0") => Node::Lod0,
            Some("FURN") => Node::Furn,
            Some(n) if n.starts_with("LEAF") => Node::Leaf(
                *leaf_of
                    .get(&p.id)
                    .ok_or_else(|| format!("{what}: piece {} is in {n} but no leaf names it", p.id))?,
            ),
            Some(n) => return Err(format!("{what}: piece {} is in an unknown node `{n}`", p.id)),
        };
        let prims = match node {
            Node::Furn => &table.furn_primitives,
            _ => &table.primitives,
        };
        let mut surfaces: Vec<String> = Vec::new();
        for r in &p.runs {
            let s = prims
                .get(r[0] as usize)
                .ok_or_else(|| format!("{what}: piece {} runs into primitive {} of {}", p.id, r[0], prims.len()))?;
            if !surfaces.contains(s) {
                surfaces.push(s.clone());
            }
        }
        let chunking = match (&p.box_, &p.panel) {
            (Some(b), _) => Chunking::Box {
                surface: b.mat.clone(),
                tint: b.tint,
            },
            (None, Some(b)) => Chunking::Panel {
                surface: b.mat.clone(),
                tint: b.tint,
            },
            _ => Chunking::Whole,
        };
        let boxes = p
            .boxes
            .iter()
            .flatten()
            .map(|b| (Vec3::new(b[0], b[1], b[2]), Vec3::new(b[3], b[4], b[5])))
            .collect();
        pieces.push(Piece {
            lo: Vec3::from(p.lo),
            hi: Vec3::from(p.hi),
            runs: p.runs.clone(),
            node,
            interior: p.interior,
            furniture: p.tag.as_deref() == Some("furniture"),
            chunking,
            wall_area_m2: p.wall_area_m2.unwrap_or(0.0).max(0.0),
            up: p.up.iter().map(|&u| at(u, "up")).collect::<Result<_, _>>()?,
            side: p.side.iter().map(|&u| at(u, "side")).collect::<Result<_, _>>()?,
            boxes,
            surfaces,
        });
    }
    let leaves = table
        .leaves
        .iter()
        .map(|l| {
            let ok = finite3(l.hinge) && l.width_m > 0.3 && l.height_m > 1.5;
            if !ok {
                return Err(format!("{what}: leaf of piece {} is not a door", l.piece));
            }
            Ok(LeafDef {
                piece: at(l.piece, "leaves")?,
                hinge: Vec3::from(l.hinge),
                shut: Vec2::from(l.shut),
                inward: Vec2::from(l.inward),
                width_m: l.width_m,
                height_m: l.height_m,
            })
        })
        .collect::<Result<_, _>>()?;
    let furniture = table
        .furniture
        .iter()
        .map(|f| {
            Ok(FurnDef {
                piece: at(f.piece, "furniture")?,
                item: f.item.clone(),
                at: Vec3::from(f.at),
                quarter_turns: f.rot % 4,
                tint: f.tint,
            })
        })
        .collect::<Result<_, String>>()?;
    let access = match (kind, t.access.as_ref()) {
        (Kind::House, Some(AccessJson { drive: Some([x, w]), .. })) if x.is_finite() && *w >= 2.0 => Access::Drive { x_m: *x, width_m: *w },
        (Kind::Business, Some(AccessJson { lot: Some(z), stalls, .. })) if z.is_finite() => Access::Lot { z_m: *z, stalls: *stalls },
        (k, None) if k.is_site() => Access::Site,
        _ => return Err(format!("{what}: a house needs access.drive [x, width >= 2], a business access.lot")),
    };
    if !(0.0..=1.0).contains(&table.collapse_wall_share) {
        return Err(format!("{what}: collapse_wall_share must be within 0..1"));
    }
    hang_from_above(&mut pieces, table.max_side_steps);
    let wall_area_m2 = pieces.iter().map(|p| p.wall_area_m2).sum();
    Ok(Kit {
        id: t.id,
        folder: folder.to_string(),
        name: t.name,
        kind,
        biomes: t.biomes,
        variant_of: t.variant_of,
        lo: Vec3::from(t.bounds_m.lo),
        hi: Vec3::from(t.bounds_m.hi),
        primitives: table.primitives,
        furn_primitives: table.furn_primitives,
        pieces,
        leaves,
        furniture,
        max_side_steps: table.max_side_steps,
        collapse_wall_share: table.collapse_wall_share,
        wall_area_m2,
        access,
        waterline_m: t.waterline_m.filter(|w| w.is_finite() && *w >= 0.0 && kind == Kind::Ship),
    })
}

/// Hangs what a whole building holds from above (a ceiling lamp under its ceiling, which the kit's support
/// links, made from below and beside, leave unheld): a piece that would not stand in the whole building is
/// held by the standing pieces it touches (or the nearest one), so it stands while they do.
fn hang_from_above(pieces: &mut [Piece], max_side_steps: u32) {
    let all = vec![true; pieces.len()];
    let stands = super::damage::standing_pieces(pieces, max_side_steps, &all);
    let gap = |a: &Piece, b: &Piece| (a.lo - b.hi).max(b.lo - a.hi).max(Vec3::ZERO).length();
    for i in (0..pieces.len()).filter(|&i| !stands[i]) {
        // Held by every standing piece it touches, or else by the nearest one (an umbrella's canopy by its pole).
        let touching: Vec<usize> = (0..pieces.len())
            .filter(|&j| j != i && stands[j] && gap(&pieces[i], &pieces[j]) <= 0.06)
            .collect();
        let holders = if touching.is_empty() {
            (0..pieces.len())
                .filter(|&j| stands[j])
                .min_by(|&a, &b| gap(&pieces[i], &pieces[a]).total_cmp(&gap(&pieces[i], &pieces[b])).then(a.cmp(&b)))
                .into_iter()
                .collect()
        } else {
            touching
        };
        for j in holders {
            pieces[j].up.push(i as u32);
        }
    }
}

impl Kits {
    /// Every kit in `dir` (`manifest.json` and each type's piece table). An error names the file.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let manifest_path = dir.join("manifest.json");
        let text = std::fs::read_to_string(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
        let manifest: ManifestJson = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
        let mut kits = Vec::with_capacity(manifest.types.len());
        for t in manifest.types {
            let path = dir.join(&t.pieces);
            let table = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            kits.push(kit_of(t, &table, &path.display().to_string(), "small_buildings")?);
        }
        if kits.is_empty() {
            return Err(format!("{}: no types", manifest_path.display()));
        }
        let furniture = Furniture::load(&dir.join("../furniture.json"))?;
        let me = Self { kits, furniture };
        me.check_furniture(&manifest_path)?;
        Ok(me)
    }

    /// The kit called `id`, by index (the houses' and businesses' ids; a site's kit is `<folder>/<id>`'s, found
    /// through its site).
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.kits.iter().position(|k| k.id == id && !k.kind.is_site())
    }

    /// Adds the kits a site's table lists (`site.json`'s `kits`, each with its piece table beside it), their glbs in
    /// `folder` under `assets/models/`. Returns the index of each, in the table's order. An error names the file.
    pub fn add_site(&mut self, dir: &Path, folder: &str, entries: Vec<serde_json::Value>) -> Result<Vec<usize>, String> {
        let mut out = Vec::with_capacity(entries.len());
        for e in entries {
            let t: TypeJson = serde_json::from_value(e).map_err(|err| format!("{}: {err}", dir.join("site.json").display()))?;
            let path = dir.join(&t.pieces);
            let table = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let kit = kit_of(t, &table, &path.display().to_string(), folder)?;
            if !kit.kind.is_site() {
                return Err(format!("{}: a site's kit is a building, a machine or a ship", path.display()));
            }
            out.push(self.kits.len());
            self.kits.push(kit);
        }
        self.check_furniture(&dir.join("site.json"))?;
        Ok(out)
    }

    /// Every block a kit places has its modelled piece, and every surface the blocks use has its swatch.
    fn check_furniture(&self, what: &Path) -> Result<(), String> {
        for k in &self.kits {
            for f in &k.furniture {
                if !self.furniture.items.contains_key(&f.item) {
                    return Err(format!(
                        "{}: {} places a `{}`, which furniture.json does not model",
                        what.display(),
                        k.id,
                        f.item
                    ));
                }
            }
            for s in &k.furn_primitives {
                if !self.furniture.swatches.contains_key(s) {
                    return Err(format!(
                        "{}: {}'s block furniture is `{s}`, which has no swatch in furniture.json",
                        what.display(),
                        k.id
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn shipped() -> Kits {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/small_buildings");
    Kits::load(&dir).expect("the shipped kits load")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_loads_with_its_leaves_and_furniture() {
        let k = shipped();
        assert!(k.kits.len() >= 24, "16 types and 8 variants, got {}", k.kits.len());
        for id in ["ranch", "ranch_b", "ranch_c", "colonial_c", "bungalow_b", "split_c"] {
            assert!(k.index_of(id).is_some(), "{id}");
        }
        for kit in &k.kits {
            assert!(!kit.leaves.is_empty(), "{} has a door", kit.id);
            for l in &kit.leaves {
                assert_eq!(
                    kit.pieces[l.piece as usize].node,
                    Node::Leaf(kit.leaves.iter().position(|x| x == l).unwrap() as u16)
                );
            }
            for f in &kit.furniture {
                assert!(kit.pieces[f.piece as usize].furniture, "{} {}", kit.id, f.item);
            }
        }
    }

    #[test]
    fn every_kit_loads_and_its_furniture_is_real() {
        let k = shipped();
        assert!(k.furniture.items.len() >= 17);
        for (name, it) in &k.furniture.items {
            // The model fills the block it replaces, within 5 cm.
            assert!(
                it.lo.cmpge(it.box_lo - Vec3::splat(0.051)).all() && it.hi.cmple(it.box_hi + Vec3::splat(0.051)).all(),
                "{name} outside its block"
            );
            assert!((it.hi - it.lo).cmpgt((it.box_hi - it.box_lo) * 0.6).all(), "{name} fills its block");
            assert!(it.triangles > 200 && it.triangles <= 6000, "{name}: {} triangles", it.triangles);
        }
    }

    #[test]
    fn a_door_frame_is_boxes_round_a_hole() {
        let k = shipped();
        let ranch = &k.kits[k.index_of("ranch").unwrap()];
        let front = &ranch.leaves[0];
        // The middle of the doorway, a metre up, is in no piece but the leaf.
        let mid = front.hinge + Vec3::new(front.shut.x, 0.0, front.shut.y) * front.width_m * 0.5 + Vec3::Y;
        let holders: Vec<usize> = ranch
            .pieces
            .iter()
            .enumerate()
            .filter(|(_, p)| p.contains(mid))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(holders, vec![front.piece as usize], "only the leaf fills the doorway");
    }
}
