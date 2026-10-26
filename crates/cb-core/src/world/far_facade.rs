//! The far look of every building type (openspec change `building-type-lods`; owner, 2026-10-25:
//! "for each building type we're gonna need a custom LOD texture/mesh for them, so we dont just yse
//! the generic texture we have now").
//!
//! A building past the dressing radius wears its own type's facade: one bay of its ground storey
//! and one of a typical storey, rendered face-on from the very pieces its near dressing builds
//! (`dressing::dress_variant`), one tile per facade kind and main colour. This file names the
//! kinds and variants, numbers their layers, and renders a band of pieces into texels (material,
//! colour and depth per pixel); `cb-app` turns the texels into colour, normal and
//! occlusion-roughness-metal maps with the same material maps the near skin uses. Pure, no engine.

use crate::world::dressing::{self, DressingConfig, Mat, Piece, Shape};
use glam::Vec3;

/// A building's facade kind: the near dressing's style, with offices split by their grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FacadeKind {
    /// Brick, punched sash windows, shopfronts, a cornice.
    WalkUp,
    /// Concrete spandrels and ribbon windows.
    RibbonOffice,
    /// A deep punched concrete grid.
    PunchedOffice,
    /// A glass curtain wall with mullions.
    Tower,
    /// Brick, steel-sash windows, loading doors.
    Warehouse,
}

impl FacadeKind {
    /// Every kind, in layer order.
    pub const ALL: [FacadeKind; 5] = [
        FacadeKind::WalkUp,
        FacadeKind::RibbonOffice,
        FacadeKind::PunchedOffice,
        FacadeKind::Tower,
        FacadeKind::Warehouse,
    ];
}

/// The far look a building wears: its kind and the index of its main colour (brick for walk-ups
/// and warehouses, concrete for offices, glass for towers) in that kind's palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FarVariant {
    /// Its facade kind.
    pub kind: FacadeKind,
    /// Its main colour's index.
    pub colour: usize,
}

/// Which storey band of a facade a tile shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// The ground storey: shopfronts, doors, loading bays.
    Ground,
    /// Every storey above it.
    Typical,
}

/// Every variant, in layer order: each kind, then each of its colours.
pub fn variants() -> Vec<FarVariant> {
    FacadeKind::ALL
        .iter()
        .flat_map(|&kind| (0..dressing::colours_of(kind)).map(move |colour| FarVariant { kind, colour }))
        .collect()
}

/// The layer of variant `v`'s `band` tile; `None` for a variant not in `variants()`.
pub fn layer_of(v: FarVariant, band: Band) -> Option<u32> {
    let i = variants().iter().position(|x| *x == v)? as u32;
    Some(i * 2 + matches!(band, Band::Typical) as u32)
}

/// The roof's layer, after every facade tile.
pub fn roof_layer() -> u32 {
    variants().len() as u32 * 2
}

/// Layers in all: two per variant and the roof.
pub fn layer_count() -> u32 {
    roof_layer() + 1
}

/// A tile's size, pixels: one bay wide by one storey tall (16 px per metre on the nominal 4 m by
/// 3.5 m bay, about twice what the cockpit resolves at 250 m).
pub const TILE_PX: u32 = 64;

/// The building the tiles are cut from: 4 bays by 3 storeys by 2 bays, so its long face (side 0)
/// has shopfronts, a door in the middle and loading bays like any real one.
pub const SAMPLE_CELLS: [usize; 3] = [4, 3, 2];
/// The nominal cell the tiles are rendered at, m.
pub const SAMPLE_CELL_M: Vec3 = Vec3::new(4.0, 3.5, 4.0);
/// The bay the tiles show along side 0 (a shopfront, not the door; a loading bay on a warehouse).
const SAMPLE_BAY: usize = 1;

/// What one pixel of a facade shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texel {
    /// Its material.
    pub mat: Mat,
    /// Its colour (linear; it multiplies the material's map).
    pub color: [f32; 3],
    /// How far behind the facade plane it is, m (negative stands proud).
    pub depth_m: f32,
    /// Where on the facade it is, m (along the face, up), for sampling the material's map.
    pub at: [f32; 2],
}

/// Renders the pieces facing -z in the window `x0..x1` by `y0..y1` into `w` by `h` texels, top
/// row first: each pixel shows the nearest block whose front is within 0.6 m behind `face_z` or
/// in front of it. A pixel nothing covers is `None`.
#[allow(clippy::too_many_arguments)]
pub fn render(pieces: &[Piece], face_z: f32, x0: f32, x1: f32, y0: f32, y1: f32, w: u32, h: u32) -> Vec<Option<Texel>> {
    let mut out = vec![None; (w * h) as usize];
    for row in 0..h {
        let y = y1 - (row as f32 + 0.5) / h as f32 * (y1 - y0);
        for col in 0..w {
            let x = x0 + (col as f32 + 0.5) / w as f32 * (x1 - x0);
            let mut best: Option<Texel> = None;
            for p in pieces {
                let Shape::Block { min, max } = p.shape else { continue };
                let front = min.z - face_z;
                if x < min.x || x >= max.x || y < min.y || y >= max.y || front > 0.6 || max.z < face_z - 3.0 {
                    continue;
                }
                if best.is_none_or(|b| front < b.depth_m) {
                    best = Some(Texel {
                        mat: p.mat,
                        color: p.color,
                        depth_m: front,
                        at: [x, y],
                    });
                }
            }
            out[(row * w + col) as usize] = best;
        }
    }
    out
}

/// Variant `v`'s `band` tile, `TILE_PX` square, rendered from its own dressing.
pub fn tile(v: FarVariant, band: Band, cfg: &DressingConfig) -> Vec<Option<Texel>> {
    let s = SAMPLE_CELL_M;
    let pieces = dressing::dress_variant(v, SAMPLE_CELLS, Vec3::ZERO, s, cfg);
    let k = match band {
        Band::Ground => 0.0,
        Band::Typical => 1.0,
    };
    let (x0, y0) = (s.x * SAMPLE_BAY as f32, s.y * k);
    render(&pieces, 0.0, x0, x0 + s.x, y0, y0 + s.y, TILE_PX, TILE_PX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_at_least_one_colour_and_its_own_layers() {
        let all = variants();
        for kind in FacadeKind::ALL {
            assert!(all.iter().any(|v| v.kind == kind), "{kind:?} has no variant");
        }
        let mut layers: Vec<u32> = all
            .iter()
            .flat_map(|&v| [layer_of(v, Band::Ground).unwrap(), layer_of(v, Band::Typical).unwrap()])
            .collect();
        layers.push(roof_layer());
        layers.sort();
        layers.dedup();
        assert_eq!(layers.len() as u32, layer_count(), "every tile has a layer of its own");
    }

    #[test]
    fn every_building_type_has_its_far_look() {
        let cfg = DressingConfig::default();
        for v in variants() {
            for band in [Band::Ground, Band::Typical] {
                let t = tile(v, band, &cfg);
                let covered = t.iter().filter(|x| x.is_some()).count();
                assert!(
                    covered as f32 > 0.97 * t.len() as f32,
                    "{v:?} {band:?}: only {covered} of {} pixels covered",
                    t.len()
                );
            }
        }
    }

    #[test]
    fn a_tower_is_mostly_glass_and_a_walk_up_mostly_brick() {
        let cfg = DressingConfig::default();
        let share = |kind, mat| {
            let t = tile(FarVariant { kind, colour: 0 }, Band::Typical, &cfg);
            t.iter().filter(|x| x.is_some_and(|x| x.mat == mat)).count() as f32 / t.len() as f32
        };
        assert!(share(FacadeKind::Tower, Mat::Glass) > 0.4);
        assert!(share(FacadeKind::WalkUp, Mat::Brick) > 0.3);
    }

    #[test]
    fn a_tile_is_the_same_every_time() {
        let cfg = DressingConfig::default();
        let v = FarVariant {
            kind: FacadeKind::Warehouse,
            colour: 2,
        };
        assert_eq!(tile(v, Band::Ground, &cfg), tile(v, Band::Ground, &cfg));
    }

    #[test]
    fn the_far_look_and_the_dressing_pick_the_same_kind_and_colour() {
        // the variant a building wears is the dressing's own pick: dressing the variant again
        // gives the same main colour on its piers as the real dressing
        let seed = crate::rng::SimRng::new(42);
        let cfg = DressingConfig::default();
        for id in 0..200 {
            let n = [3 + id as usize % 5, 2 + id as usize % 30, 3];
            let size = SAMPLE_CELL_M;
            let v = dressing::far_variant(id, n, size, &seed);
            let real = dressing::dress(id, n, Vec3::ZERO, size, &seed, false, &cfg, [crate::world::FRONT_LIMIT_M; 4], None);
            let plain = dressing::dress_variant(v, n, Vec3::ZERO, size, &cfg);
            let main = |ps: &[Piece]| {
                ps.iter()
                    .find(|p| matches!(p.mat, Mat::Brick | Mat::Concrete) && matches!(p.anchor, dressing::Anchor::Whole))
                    .map(|p| p.color)
            };
            // the parapet carries the main wall colour in both
            if v.kind != FacadeKind::Tower {
                assert_eq!(main(&real), main(&plain), "building {id} {v:?}");
            }
        }
    }

    #[test]
    fn massing_keeps_only_what_stands_on_the_roof() {
        let seed = crate::rng::SimRng::new(7);
        let cfg = DressingConfig::default();
        let (n, s) = ([5, 8, 4], SAMPLE_CELL_M);
        let roof_y = s.y * 8.0;
        let m = dressing::massing(3, n, Vec3::ZERO, s, &seed, &cfg, [crate::world::FRONT_LIMIT_M; 4], 16);
        assert!(!m.is_empty() && m.len() <= 16);
        for p in &m {
            let y = match p.shape {
                Shape::Block { min, .. } => min.y,
                Shape::Drum { base, .. } => base.y,
            };
            assert!(y >= roof_y - 0.01);
        }
    }
}
