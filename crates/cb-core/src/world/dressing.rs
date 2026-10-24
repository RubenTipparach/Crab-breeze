//! What makes a box read as a building (openspec change `building-generation`): pilasters on the
//! column lines, a string course at every floor, a parapet with a coping round the roof, and roof
//! clutter (timber water tanks on legs, HVAC units, vents, a stair bulkhead, aerial masts), styled
//! by the building's height: brick walk-ups, concrete offices, glass towers.
//!
//! Every piece is tied to the frame element that holds it up (`Anchor`), on the same cell grid as
//! `structure.rs`, so a damaged building drops exactly the pieces whose element fell. A whole
//! building is dressed in long merged pieces instead (`segmented = false`), a fraction of the
//! triangles. Pure and deterministic: the same building and seed give the same pieces.

use crate::rng::SimRng;
use glam::Vec3;

/// What holds a piece up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// The building as a whole (merged pieces of a whole building).
    Whole,
    /// The column on node (a, c) of storey k.
    Column {
        /// Storey.
        k: u32,
        /// Node along x.
        a: u32,
        /// Node along z.
        c: u32,
    },
    /// The floor bay of cell (i, j) at the top of storey k.
    Slab {
        /// Storey.
        k: u32,
        /// Cell along x.
        i: u32,
        /// Cell along z.
        j: u32,
    },
    /// The facade panel of storey k on `side` (0 faces -z, 1 +z, 2 -x, 3 +x) at position `idx`.
    Wall {
        /// Storey.
        k: u32,
        /// Facade side.
        side: u8,
        /// Cell along that side.
        idx: u32,
    },
}

/// A piece's shape, in world metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// An axis-aligned box.
    Block {
        /// Least corner.
        min: Vec3,
        /// Greatest corner.
        max: Vec3,
    },
    /// An upright drum or cone: radius `r0` at `base`, `r1` at `base + height_m`.
    Drum {
        /// Centre of the bottom.
        base: Vec3,
        /// Radius at the bottom.
        r0: f32,
        /// Radius at the top.
        r1: f32,
        /// Height.
        height_m: f32,
    },
}

/// One piece of dressing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    /// Its shape.
    pub shape: Shape,
    /// Its colour (linear, multiplies the trim material).
    pub color: [f32; 3],
    /// What holds it up.
    pub anchor: Anchor,
}

/// The look a building is dressed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Up to 6 storeys: brick piers, stone string courses, a cornice, water tanks and chimneys.
    WalkUp,
    /// 7 to 14 storeys: concrete fins, deep floor bands, HVAC and a dish.
    Office,
    /// 15 storeys and up: thin metal fins, dark spandrel bands, masts and a water tank.
    Tower,
}

/// Dressing tuning, from the `dressing:` section of `data/world/world.ron`.
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DressingConfig {
    /// Parapet height above the roof.
    pub parapet_m: f32,
    /// Parapet thickness.
    pub parapet_wall_m: f32,
    /// Coping thickness and how far it overhangs each side.
    pub coping_m: f32,
    /// String course height.
    pub course_m: f32,
    /// How far a string course stands out from the facade.
    pub course_out_m: f32,
    /// Pilaster width.
    pub pilaster_m: f32,
    /// How far a pilaster stands out from the facade.
    pub pilaster_out_m: f32,
    /// Roof props per 100 square metres of roof.
    pub props_per_100m2: f32,
    /// Most roof props on one roof.
    pub max_props: u32,
    /// Props keep this far inside the roof edge.
    pub prop_edge_m: f32,
    /// Buildings within this distance of the camera are dressed (drawn only; the frame does not care).
    pub dress_radius_m: f32,
}

impl Default for DressingConfig {
    fn default() -> Self {
        Self {
            parapet_m: 1.1,
            parapet_wall_m: 0.3,
            coping_m: 0.08,
            course_m: 0.25,
            course_out_m: 0.12,
            pilaster_m: 0.6,
            pilaster_out_m: 0.2,
            props_per_100m2: 1.0,
            max_props: 10,
            prop_edge_m: 1.2,
            dress_radius_m: 450.0,
        }
    }
}

impl crate::data::Validate for DressingConfig {
    fn validate(&self) -> Result<(), String> {
        use crate::data::positive;
        for (name, v) in [
            ("dressing.parapet_m", self.parapet_m),
            ("dressing.parapet_wall_m", self.parapet_wall_m),
            ("dressing.coping_m", self.coping_m),
            ("dressing.course_m", self.course_m),
            ("dressing.course_out_m", self.course_out_m),
            ("dressing.pilaster_m", self.pilaster_m),
            ("dressing.pilaster_out_m", self.pilaster_out_m),
            ("dressing.props_per_100m2", self.props_per_100m2),
            ("dressing.prop_edge_m", self.prop_edge_m),
            ("dressing.dress_radius_m", self.dress_radius_m),
        ] {
            positive(name, v)?;
        }
        Ok(())
    }
}

/// The style of a building of `storeys`.
pub fn style_of(storeys: usize) -> Style {
    match storeys {
        0..=6 => Style::WalkUp,
        7..=14 => Style::Office,
        _ => Style::Tower,
    }
}

const BRICK_PIER: [f32; 3] = [0.55, 0.30, 0.22];
const STONE: [f32; 3] = [0.80, 0.76, 0.68];
const CONCRETE: [f32; 3] = [0.68, 0.66, 0.62];
const DARK_METAL: [f32; 3] = [0.20, 0.24, 0.29];
const LIGHT_METAL: [f32; 3] = [0.66, 0.69, 0.72];
const TIMBER: [f32; 3] = [0.42, 0.30, 0.20];
const STEEL: [f32; 3] = [0.17, 0.17, 0.18];

fn block(min: Vec3, max: Vec3, color: [f32; 3], anchor: Anchor) -> Piece {
    Piece {
        shape: Shape::Block { min, max },
        color,
        anchor,
    }
}

/// Dresses building `id`: `n` = [cells along x, storeys, cells along z], `origin` its least corner
/// on the ground, `size` one cell (as `destruct::Cells`). `segmented` cuts pieces per storey and
/// per bay so each can follow its element; otherwise long pieces are merged.
pub fn dress(id: u32, n: [usize; 3], origin: Vec3, size: Vec3, seed: &SimRng, segmented: bool, cfg: &DressingConfig) -> Vec<Piece> {
    let [nx, ns, nz] = n;
    let mut out = Vec::new();
    if nx == 0 || ns == 0 || nz == 0 || !size.is_finite() {
        return out;
    }
    let style = style_of(ns);
    let top_k = ns as u32 - 1;
    let roof_y = origin.y + size.y * ns as f32;
    let far = origin + Vec3::new(size.x * nx as f32, 0.0, size.z * nz as f32);
    let (pier, band) = match style {
        Style::WalkUp => (BRICK_PIER, STONE),
        Style::Office => (CONCRETE, CONCRETE),
        Style::Tower => (LIGHT_METAL, DARK_METAL),
    };
    let (p_half, p_out) = match style {
        Style::Tower => (0.06, cfg.pilaster_out_m * 0.6),
        Style::Office => (0.15, cfg.pilaster_out_m * 2.5),
        Style::WalkUp => (cfg.pilaster_m * 0.5, cfg.pilaster_out_m),
    };
    // The outward offset of a point on facade `side`, for a piece standing `out` proud.
    let side_box = |side: u8, along0: f32, along1: f32, y0: f32, y1: f32, out: f32, inn: f32| -> (Vec3, Vec3) {
        match side {
            0 => (Vec3::new(along0, y0, origin.z - out), Vec3::new(along1, y1, origin.z + inn)),
            1 => (Vec3::new(along0, y0, far.z - inn), Vec3::new(along1, y1, far.z + out)),
            2 => (Vec3::new(origin.x - out, y0, along0), Vec3::new(origin.x + inn, y1, along1)),
            _ => (Vec3::new(far.x - inn, y0, along0), Vec3::new(far.x + out, y1, along1)),
        }
    };
    let along = |side: u8, idx: usize| -> f32 {
        if side < 2 {
            origin.x + size.x * idx as f32
        } else {
            origin.z + size.z * idx as f32
        }
    };
    let count = |side: u8| if side < 2 { nx } else { nz };
    // Pilasters on every perimeter node (corners once, on the x faces). A tower's fins also stand
    // at each half bay.
    for side in 0..4u8 {
        for idx in 0..=count(side) {
            if side >= 2 && (idx == 0 || idx == count(side)) {
                continue;
            }
            let s = along(side, idx);
            let node = match side {
                0 => (idx as u32, 0),
                1 => (idx as u32, nz as u32),
                2 => (0, idx as u32),
                _ => (nx as u32, idx as u32),
            };
            let mut place = |s: f32| {
                if segmented {
                    for k in 0..ns {
                        let (y0, y1) = (origin.y + size.y * k as f32, origin.y + size.y * (k + 1) as f32);
                        let (lo, hi) = side_box(side, s - p_half, s + p_half, y0, y1, p_out, 0.05);
                        out.push(block(
                            lo,
                            hi,
                            pier,
                            Anchor::Column {
                                k: k as u32,
                                a: node.0,
                                c: node.1,
                            },
                        ));
                    }
                } else {
                    let (lo, hi) = side_box(side, s - p_half, s + p_half, origin.y, roof_y, p_out, 0.05);
                    out.push(block(lo, hi, pier, Anchor::Whole));
                }
            };
            place(s);
            if style == Style::Tower && idx < count(side) {
                let half = if side < 2 { size.x } else { size.z } * 0.5;
                place(s + half);
            }
        }
    }
    // A string course (or floor band) at every floor above the ground.
    let (c_h, c_out) = match style {
        Style::Tower => (0.9, cfg.course_out_m * 0.5),
        Style::Office => (1.0, cfg.course_out_m),
        Style::WalkUp => (cfg.course_m, cfg.course_out_m),
    };
    for k in 1..ns {
        let y = origin.y + size.y * k as f32;
        for side in 0..4u8 {
            if segmented {
                for idx in 0..count(side) {
                    let (lo, hi) = side_box(
                        side,
                        along(side, idx),
                        along(side, idx + 1),
                        y - c_h * 0.5,
                        y + c_h * 0.5,
                        c_out,
                        0.04,
                    );
                    out.push(block(
                        lo,
                        hi,
                        band,
                        Anchor::Wall {
                            k: k as u32,
                            side,
                            idx: idx as u32,
                        },
                    ));
                }
            } else {
                let (lo, hi) = side_box(
                    side,
                    along(side, 0),
                    along(side, count(side)),
                    y - c_h * 0.5,
                    y + c_h * 0.5,
                    c_out,
                    0.04,
                );
                out.push(block(lo, hi, band, Anchor::Whole));
            }
        }
    }
    // Parapet and coping. Side walls (2, 3) stop short of the corners so no faces share a plane.
    let t = cfg.parapet_wall_m;
    let cap = cfg.coping_m;
    let parapet_color = if style == Style::Tower { DARK_METAL } else { pier };
    for side in 0..4u8 {
        let pieces: Vec<(f32, f32, Anchor)> = if segmented {
            (0..count(side))
                .map(|idx| {
                    let (i, j) = match side {
                        0 => (idx, 0),
                        1 => (idx, nz - 1),
                        2 => (0, idx),
                        _ => (nx - 1, idx),
                    };
                    (
                        along(side, idx),
                        along(side, idx + 1),
                        Anchor::Slab {
                            k: top_k,
                            i: i as u32,
                            j: j as u32,
                        },
                    )
                })
                .collect()
        } else {
            vec![(along(side, 0), along(side, count(side)), Anchor::Whole)]
        };
        let last = along(side, count(side));
        for (mut a0, mut a1, anchor) in pieces {
            let (mut c0, mut c1) = (a0 - cap, a1 + cap);
            if side >= 2 {
                if a0 <= along(side, 0) + 1e-3 {
                    a0 += t;
                    c0 = a0 + cap;
                }
                if a1 >= last - 1e-3 {
                    a1 -= t;
                    c1 = a1 - cap;
                }
            }
            let (lo, hi) = side_box(side, a0, a1, roof_y, roof_y + cfg.parapet_m, 0.0, t);
            out.push(block(lo, hi, parapet_color, anchor));
            let (lo, hi) = side_box(side, c0, c1, roof_y + cfg.parapet_m, roof_y + cfg.parapet_m + cap, cap, t + cap);
            out.push(block(lo, hi, if style == Style::Tower { LIGHT_METAL } else { STONE }, anchor));
        }
    }
    roof_props(&mut out, id, n, origin, size, roof_y, style, seed, cfg);
    out
}

#[allow(clippy::too_many_arguments)]
fn roof_props(
    out: &mut Vec<Piece>,
    id: u32,
    n: [usize; 3],
    origin: Vec3,
    size: Vec3,
    roof_y: f32,
    style: Style,
    seed: &SimRng,
    cfg: &DressingConfig,
) {
    let [nx, ns, nz] = n;
    let top_k = ns as u32 - 1;
    let mut rng = seed.fork(&format!("building/{id}/roof"));
    let w = size.x * nx as f32;
    let d = size.z * nz as f32;
    let anchor_at = |p: Vec3| Anchor::Slab {
        k: top_k,
        i: (((p.x - origin.x) / size.x) as u32).min(nx as u32 - 1),
        j: (((p.z - origin.z) / size.z) as u32).min(nz as u32 - 1),
    };
    let mut placed: Vec<(Vec3, f32)> = Vec::new();
    // A stair and lift bulkhead over the middle on anything with a core.
    if nx >= 3 && nz >= 3 {
        let c = origin + Vec3::new(w * 0.5, 0.0, d * 0.5);
        let half = Vec3::new(2.6, 0.0, 2.6);
        let tone = if style == Style::WalkUp { BRICK_PIER } else { CONCRETE };
        out.push(block(
            Vec3::new(c.x - half.x, roof_y, c.z - half.z),
            Vec3::new(c.x + half.x, roof_y + 3.6, c.z + half.z),
            tone,
            anchor_at(c),
        ));
        out.push(block(
            Vec3::new(c.x - half.x - 0.15, roof_y + 3.6, c.z - half.z - 0.15),
            Vec3::new(c.x + half.x + 0.15, roof_y + 3.75, c.z + half.z + 0.15),
            STONE,
            anchor_at(c),
        ));
        placed.push((c, 4.0));
    }
    let wants = ((w * d / 100.0 * cfg.props_per_100m2) as u32).clamp(2, cfg.max_props);
    let menu: &[&str] = match style {
        Style::WalkUp => &["tank", "chimney", "chimney", "vent", "vent", "aerial"],
        Style::Office => &["hvac", "hvac", "hvac", "vent", "duct", "mast"],
        Style::Tower => &["tank", "hvac", "hvac", "mast", "mast", "vent"],
    };
    let edge = cfg.prop_edge_m;
    for p in 0..wants {
        let kind = menu[p as usize % menu.len()];
        let r = match kind {
            "tank" => 3.0,
            "hvac" => 1.8,
            "duct" => 2.4,
            "mast" => 1.0,
            "chimney" => 0.8,
            _ => 0.5,
        };
        for _ in 0..30 {
            let x = origin.x + rng.range_f32(edge + r, (w - edge - r).max(edge + r));
            let z = origin.z + rng.range_f32(edge + r, (d - edge - r).max(edge + r));
            let c = Vec3::new(x, roof_y, z);
            if x + r > origin.x + w - edge || z + r > origin.z + d - edge || placed.iter().any(|(q, qr)| q.distance(c) < qr + r) {
                continue;
            }
            placed.push((c, r));
            let a = anchor_at(c);
            let b = |dx0: f32, y0: f32, dz0: f32, dx1: f32, y1: f32, dz1: f32, col: [f32; 3]| {
                block(
                    Vec3::new(x + dx0, roof_y + y0, z + dz0),
                    Vec3::new(x + dx1, roof_y + y1, z + dz1),
                    col,
                    a,
                )
            };
            match kind {
                "tank" => {
                    // A timber tank on a steel stand, New York style.
                    for (sx, sz) in [(-1.5, -1.5), (1.5, -1.5), (-1.5, 1.5), (1.5, 1.5)] {
                        out.push(b(sx - 0.1, 0.0, sz - 0.1, sx + 0.1, 3.0, sz + 0.1, STEEL));
                    }
                    out.push(b(-2.0, 3.0, -2.0, 2.0, 3.12, 2.0, STEEL));
                    out.push(Piece {
                        shape: Shape::Drum {
                            base: c + Vec3::Y * 3.12,
                            r0: 1.9,
                            r1: 1.9,
                            height_m: 3.6,
                        },
                        color: TIMBER,
                        anchor: a,
                    });
                    out.push(Piece {
                        shape: Shape::Drum {
                            base: c + Vec3::Y * 6.72,
                            r0: 2.05,
                            r1: 0.12,
                            height_m: 1.3,
                        },
                        color: [0.33, 0.25, 0.19],
                        anchor: a,
                    });
                }
                "hvac" => {
                    out.push(b(-1.3, 0.0, -0.75, 1.3, 0.16, 0.75, STEEL));
                    out.push(b(-1.2, 0.16, -0.7, 1.2, 1.66, 0.7, [0.72, 0.74, 0.71]));
                    for fx in [-0.6, 0.6] {
                        out.push(Piece {
                            shape: Shape::Drum {
                                base: Vec3::new(x + fx, roof_y + 1.66, z),
                                r0: 0.5,
                                r1: 0.5,
                                height_m: 0.08,
                            },
                            color: STEEL,
                            anchor: a,
                        });
                    }
                }
                "duct" => {
                    out.push(b(-2.2, 0.4, -0.35, 2.2, 1.1, 0.35, LIGHT_METAL));
                    for dx in [-1.8, 0.0, 1.8] {
                        out.push(b(dx - 0.05, 0.0, -0.4, dx + 0.05, 0.4, 0.4, STEEL));
                    }
                }
                "mast" => {
                    let h = rng.range_f32(8.0, 14.0);
                    out.push(b(-0.12, 0.0, -0.12, 0.12, h, 0.12, [0.70, 0.25, 0.18]));
                    out.push(b(-0.18, h, -0.18, 0.18, h + 0.3, 0.18, [0.9, 0.15, 0.1]));
                }
                "chimney" => {
                    out.push(b(-0.45, 0.0, -0.3, 0.45, 2.2, 0.3, BRICK_PIER));
                    out.push(b(-0.5, 2.2, -0.35, 0.5, 2.3, 0.35, STONE));
                }
                "aerial" => {
                    out.push(b(-0.025, 0.0, -0.025, 0.025, 3.2, 0.025, LIGHT_METAL));
                    for e in 0..5 {
                        let half = 0.7 - e as f32 * 0.1;
                        let zz = -0.6 + e as f32 * 0.3;
                        out.push(b(-half, 3.0, zz - 0.012, half, 3.025, zz + 0.012, LIGHT_METAL));
                    }
                }
                _ => {
                    out.push(Piece {
                        shape: Shape::Drum {
                            base: c,
                            r0: 0.22,
                            r1: 0.22,
                            height_m: 1.3,
                        },
                        color: LIGHT_METAL,
                        anchor: a,
                    });
                    out.push(Piece {
                        shape: Shape::Drum {
                            base: c + Vec3::Y * 1.3,
                            r0: 0.4,
                            r1: 0.05,
                            height_m: 0.3,
                        },
                        color: [0.42, 0.44, 0.46],
                        anchor: a,
                    });
                }
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(storeys: usize, segmented: bool) -> Vec<Piece> {
        dress(
            5,
            [5, storeys, 4],
            Vec3::new(100.0, 20.0, 50.0),
            Vec3::new(6.0, 3.5, 6.0),
            &SimRng::new(42),
            segmented,
            &DressingConfig::default(),
        )
    }

    fn top(p: &Piece) -> f32 {
        match p.shape {
            Shape::Block { max, .. } => max.y,
            Shape::Drum { base, height_m, .. } => base.y + height_m,
        }
    }

    #[test]
    fn the_same_building_is_dressed_the_same() {
        assert_eq!(go(9, true), go(9, true));
    }

    #[test]
    fn height_picks_the_style() {
        assert_eq!(style_of(4), Style::WalkUp);
        assert_eq!(style_of(10), Style::Office);
        assert_eq!(style_of(30), Style::Tower);
    }

    #[test]
    fn roof_props_keep_clear_of_the_roof_edge() {
        let cfg = DressingConfig::default();
        let roof = 20.0 + 3.5 * 9.0;
        for p in go(9, true) {
            if let Anchor::Slab { .. } = p.anchor {
                let (lo, hi) = match p.shape {
                    Shape::Block { min, max } => (min, max),
                    Shape::Drum { base, r0, r1, .. } => (base - Vec3::splat(r0.max(r1)), base + Vec3::splat(r0.max(r1))),
                };
                if lo.y >= roof + cfg.parapet_m - 1e-3
                    || top(&p) <= roof + cfg.parapet_m + cfg.coping_m + 1e-3 && (lo.x < 100.5 || hi.x > 129.5 || lo.z < 50.5 || hi.z > 73.5)
                {
                    continue; // parapets and copings sit on the edge by design
                }
                assert!(
                    lo.x >= 100.0 + cfg.prop_edge_m - 0.01 && hi.x <= 130.0 - cfg.prop_edge_m + 0.01,
                    "prop at x {lo:?}..{hi:?}"
                );
                assert!(
                    lo.z >= 50.0 + cfg.prop_edge_m - 0.01 && hi.z <= 74.0 - cfg.prop_edge_m + 0.01,
                    "prop at z {lo:?}..{hi:?}"
                );
            }
        }
    }

    #[test]
    fn a_whole_building_takes_far_fewer_pieces_than_a_cut_one() {
        let (whole, cut) = (go(30, false).len(), go(30, true).len());
        assert!(whole * 5 < cut, "whole {whole}, cut {cut}");
        assert!(
            go(30, false)
                .iter()
                .filter(|p| !matches!(p.anchor, Anchor::Slab { .. }))
                .all(|p| p.anchor == Anchor::Whole)
        );
    }

    #[test]
    fn every_cut_piece_names_an_element_on_the_grid() {
        for p in go(9, true) {
            match p.anchor {
                Anchor::Whole => panic!("a cut building has no whole pieces"),
                Anchor::Column { k, a, c } => assert!(k < 9 && a <= 5 && c <= 4),
                Anchor::Slab { k, i, j } => assert!(k == 8 && i < 5 && j < 4),
                Anchor::Wall { k, side, idx } => assert!(k < 9 && side < 4 && idx < if side < 2 { 5 } else { 4 }),
            }
        }
    }
}
