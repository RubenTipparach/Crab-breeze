//! The building's skin and roof (openspec change `building-generation`): the mockup's facade
//! modules in the game, one panel per bay per storey per side, of the building's style. Walk-ups
//! are brick with punched sash windows, stone sills and lintels; offices are board-formed concrete
//! with ribbon windows, deep spandrels and fins on the column lines; towers are glass curtain walls
//! with mullions and dark spandrels; warehouses are brick with steel-sash windows and loading doors.
//! The ground storey of a walk-up or office is shopfronts with sign bands and awnings on its long
//! faces. Round the roof a parapet and coping, on it the clutter (timber water tanks on legs, HVAC
//! units, vents, ducts, chimneys, aerials, masts, a stair bulkhead). Only buildings near the player
//! are dressed (`dress_radius_m`); far ones stay plain boxes.
//!
//! Every piece is tied to the frame element that holds it up (`Anchor`), on the same cell grid as
//! `structure.rs`, so a damaged building drops exactly the pieces whose element fell. A whole
//! building is dressed in long merged pieces instead (`segmented = false`), a fraction of the
//! triangles. Pure and deterministic: the same building and seed give the same pieces.

use crate::rng::SimRng;
use crate::world::far_facade::{FacadeKind, FarVariant};
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
    /// What it is made of (which material draws it).
    pub mat: Mat,
}

/// A piece's material. Brick, concrete, roof and wood are textured (tinted by the piece's colour);
/// glass is see-through; trim is plain, coloured per vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mat {
    /// Painted metal, stone, frames, props: colour only.
    Trim,
    /// Brick, coursed (a 2.4 m tile).
    Brick,
    /// Board-formed concrete (a 2.4 m tile).
    Concrete,
    /// Window glass.
    Glass,
    /// Roofing felt and gravel.
    Roof,
    /// Timber: doors, water tanks.
    Wood,
}

/// The look a building is dressed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Up to 6 storeys: brick with punched sash windows, stone sills and lintels, shopfronts on the
    /// ground, a cornice, water tanks and chimneys.
    WalkUp,
    /// 7 to 14 storeys: board-formed concrete, ribbon windows over deep spandrels, fins on the column
    /// lines, shopfronts on the ground, HVAC and a dish.
    Office,
    /// 15 storeys and up: glass curtain walls with mullions and dark spandrels, masts and a water
    /// tank.
    Tower,
    /// Up to 4 storeys on a footprint over 900 square metres: brick, steel-sash windows, loading
    /// doors.
    Warehouse,
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
    /// Roof props per 100 square metres of roof.
    pub props_per_100m2: f32,
    /// Most roof props on one roof.
    pub max_props: u32,
    /// Props keep this far inside the roof edge.
    pub prop_edge_m: f32,
    /// Buildings within this distance of the camera are dressed (drawn only; the frame does not care).
    pub dress_radius_m: f32,
    /// Rooms and furniture are drawn within this distance of the pilot's eye (`building-lod`; owner,
    /// 2026-10-24: "I would prefer 250 m for furniture").
    pub rooms_radius_m: f32,
    /// ...and hidden beyond the radius plus this margin (in between they keep their state).
    pub rooms_margin_m: f32,
    /// Beyond `dress_radius_m` and within this, a building shows its massing (parapet, coping,
    /// cornice and roof props from its own dressing) over walls wearing its far facade
    /// (`building-type-lods`); beyond it, the walls alone. Metres.
    pub massing_radius_m: f32,
    /// At most this many massing pieces per building, largest first.
    pub massing_pieces: u32,
    /// At most this many buildings get their massing built in one frame, nearest first.
    pub massing_per_frame: u32,
}

impl Default for DressingConfig {
    fn default() -> Self {
        Self {
            parapet_m: 1.1,
            parapet_wall_m: 0.3,
            coping_m: 0.08,
            props_per_100m2: 1.0,
            max_props: 10,
            prop_edge_m: 1.2,
            dress_radius_m: 250.0,
            rooms_radius_m: 250.0,
            rooms_margin_m: 20.0,
            massing_radius_m: 1500.0,
            massing_pieces: 16,
            massing_per_frame: 12,
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
            ("dressing.props_per_100m2", self.props_per_100m2),
            ("dressing.prop_edge_m", self.prop_edge_m),
            ("dressing.dress_radius_m", self.dress_radius_m),
            ("dressing.rooms_radius_m", self.rooms_radius_m),
            ("dressing.rooms_margin_m", self.rooms_margin_m),
            ("dressing.massing_radius_m", self.massing_radius_m),
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
const LIGHT_METAL: [f32; 3] = [0.66, 0.69, 0.72];
const STEEL: [f32; 3] = [0.17, 0.17, 0.18];

fn block(min: Vec3, max: Vec3, color: [f32; 3], anchor: Anchor) -> Piece {
    Piece {
        shape: Shape::Block { min, max },
        color,
        anchor,
        mat: Mat::Trim,
    }
}

const FRAME_WHITE: [f32; 3] = [0.88, 0.86, 0.80];
const FRAME_GREEN: [f32; 3] = [0.14, 0.26, 0.19];
const FRAME_BLACK: [f32; 3] = [0.09, 0.09, 0.10];
const KICK: [f32; 3] = [0.18, 0.18, 0.19];
const SIGNS: [[f32; 3]; 6] = [
    [0.70, 0.16, 0.12],
    [0.14, 0.36, 0.62],
    [0.82, 0.58, 0.14],
    [0.16, 0.48, 0.30],
    [0.86, 0.82, 0.74],
    [0.45, 0.18, 0.45],
];
// Tints for the near-white brick texture: red, brown, dark red, buff, orange, grey.
const BRICKS: [[f32; 3]; 6] = [
    [0.95, 0.55, 0.42],
    [0.80, 0.50, 0.38],
    [0.72, 0.38, 0.30],
    [0.95, 0.80, 0.60],
    [1.0, 0.62, 0.40],
    [0.70, 0.66, 0.62],
];
const CONCRETES: [[f32; 3]; 4] = [[0.92, 0.90, 0.86], [0.80, 0.78, 0.74], [0.95, 0.88, 0.78], [0.72, 0.74, 0.76]];
const GLASSES: [[f32; 3]; 4] = [[0.30, 0.42, 0.55], [0.30, 0.48, 0.42], [0.50, 0.40, 0.28], [0.36, 0.38, 0.42]];
const MULLIONS: [[f32; 3]; 3] = [[0.70, 0.72, 0.75], [0.10, 0.10, 0.11], [0.45, 0.34, 0.22]];

/// One building's look, drawn from its seed: materials, colours and window rhythm, so two
/// buildings of a style are never twins.
#[derive(Clone, Copy, Debug)]
struct Look {
    style: Style,
    /// An office as a deep punched concrete grid instead of ribbon windows.
    punched_grid: bool,
    brick: [f32; 3],
    concrete: [f32; 3],
    glass: [f32; 3],
    frame: [f32; 3],
    mullion: [f32; 3],
    /// Walk-up windows per bay, and their width.
    windows: u32,
    window_w: f32,
    /// Lintels in brick (a soldier course) rather than stone.
    brick_lintels: bool,
}

impl Look {
    /// The far variant this look wears: its facade kind and the index of its main colour.
    fn far_variant(&self) -> FarVariant {
        let index = |of: &[[f32; 3]], c: [f32; 3]| of.iter().position(|x| *x == c).unwrap_or(0);
        let (kind, colour) = match self.style {
            Style::WalkUp => (FacadeKind::WalkUp, index(&BRICKS, self.brick)),
            Style::Warehouse => (FacadeKind::Warehouse, index(&BRICKS, self.brick)),
            Style::Office if self.punched_grid => (FacadeKind::PunchedOffice, index(&CONCRETES, self.concrete)),
            Style::Office => (FacadeKind::RibbonOffice, index(&CONCRETES, self.concrete)),
            Style::Tower => (FacadeKind::Tower, index(&GLASSES, self.glass)),
        };
        FarVariant { kind, colour }
    }

    /// A plain look of variant `v`: its kind and main colour, everything else the first choice.
    fn of_variant(v: FarVariant) -> Look {
        let c = v.colour;
        let style = match v.kind {
            FacadeKind::WalkUp => Style::WalkUp,
            FacadeKind::Warehouse => Style::Warehouse,
            FacadeKind::RibbonOffice | FacadeKind::PunchedOffice => Style::Office,
            FacadeKind::Tower => Style::Tower,
        };
        let walls_brick = matches!(v.kind, FacadeKind::WalkUp | FacadeKind::Warehouse);
        let walls_concrete = matches!(v.kind, FacadeKind::RibbonOffice | FacadeKind::PunchedOffice);
        Look {
            style,
            punched_grid: v.kind == FacadeKind::PunchedOffice,
            brick: BRICKS[if walls_brick { c % BRICKS.len() } else { 0 }],
            concrete: CONCRETES[if walls_concrete { c % CONCRETES.len() } else { 0 }],
            glass: GLASSES[if v.kind == FacadeKind::Tower { c % GLASSES.len() } else { 0 }],
            frame: FRAME_WHITE,
            mullion: MULLIONS[0],
            windows: 2,
            window_w: 1.1,
            brick_lintels: false,
        }
    }
}

/// How many main colours a facade kind comes in (the palette its look picks from).
pub fn colours_of(kind: FacadeKind) -> usize {
    match kind {
        FacadeKind::WalkUp | FacadeKind::Warehouse => BRICKS.len(),
        FacadeKind::RibbonOffice | FacadeKind::PunchedOffice => CONCRETES.len(),
        FacadeKind::Tower => GLASSES.len(),
    }
}

fn pick<T: Copy>(rng: &mut SimRng, of: &[T]) -> T {
    of[(rng.next_u64() % of.len() as u64) as usize]
}

/// The style and look of a building of `storeys` on `area_m2`, from its seed.
fn look_of(storeys: usize, area_m2: f32, rng: &mut SimRng) -> Look {
    let r = rng.next_f32();
    let style = match storeys {
        _ if storeys <= 4 && area_m2 > 900.0 => Style::Warehouse,
        0..=6 => {
            if r < 0.85 {
                Style::WalkUp
            } else {
                Style::Office
            }
        }
        7..=14 => {
            if r < 0.7 {
                Style::Office
            } else {
                Style::Tower
            }
        }
        _ => {
            if r < 0.85 {
                Style::Tower
            } else {
                Style::Office
            }
        }
    };
    Look {
        style,
        punched_grid: rng.chance(0.4),
        brick: pick(rng, &BRICKS),
        concrete: pick(rng, &CONCRETES),
        glass: pick(rng, &GLASSES),
        frame: pick(rng, &[FRAME_WHITE, FRAME_WHITE, FRAME_GREEN, FRAME_BLACK]),
        mullion: pick(rng, &MULLIONS),
        windows: if rng.chance(0.3) { 3 } else { 2 },
        window_w: rng.range_f32(0.9, 1.25),
        brick_lintels: rng.chance(0.4),
    }
}

/// A facade panel's frame: `side` of the building, along-face start `s0`, the outer plane and its
/// outward sign. `d` runs inward from the outer plane (negative stands proud of it).
struct Panel {
    side: u8,
    s0: f32,
    plane: f32,
    out: f32,
    anchor: Anchor,
}

impl Panel {
    #[allow(clippy::too_many_arguments)]
    fn made(&self, mat: Mat, s0: f32, s1: f32, y0: f32, y1: f32, d0: f32, d1: f32, color: [f32; 3]) -> Piece {
        let (a, b) = (self.plane - self.out * d0, self.plane - self.out * d1);
        let (lo, hi) = (a.min(b), a.max(b));
        let (sa, sb) = (self.s0 + s0, self.s0 + s1);
        let (min, max) = if self.side < 2 {
            (Vec3::new(sa, y0, lo), Vec3::new(sb, y1, hi))
        } else {
            (Vec3::new(lo, y0, sa), Vec3::new(hi, y1, sb))
        };
        Piece {
            mat,
            ..block(min, max, color, self.anchor)
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn piece(&self, s0: f32, s1: f32, y0: f32, y1: f32, d0: f32, d1: f32, color: [f32; 3]) -> Piece {
        self.made(Mat::Trim, s0, s1, y0, y1, d0, d1, color)
    }
}

/// A sash window `w` wide centred at `c`, from `ya` to `yb`, set `d` into the wall: see-through
/// glass in a frame with a meeting rail.
#[allow(clippy::too_many_arguments)]
fn sash(out: &mut Vec<Piece>, p: &Panel, c: f32, w: f32, ya: f32, yb: f32, look: &Look, d: f32) {
    let (f, frame) = (0.06, look.frame);
    out.push(p.made(Mat::Glass, c - w * 0.5, c + w * 0.5, ya, yb, d + 0.04, d + 0.06, look.glass));
    out.push(p.piece(c - w * 0.5, c + w * 0.5, ya, ya + f, d, d + 0.08, frame));
    out.push(p.piece(c - w * 0.5, c + w * 0.5, yb - f, yb, d, d + 0.08, frame));
    out.push(p.piece(c - w * 0.5, c - w * 0.5 + f, ya + f, yb - f, d, d + 0.08, frame));
    out.push(p.piece(c + w * 0.5 - f, c + w * 0.5, ya + f, yb - f, d, d + 0.08, frame));
    let mid = (ya + yb) * 0.5;
    out.push(p.piece(
        c - w * 0.5 + f,
        c + w * 0.5 - f,
        mid - 0.025,
        mid + 0.025,
        d + 0.01,
        d + 0.07,
        frame,
    ));
}

/// What a ground-floor bay is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ground {
    /// Like the storeys above.
    Plain,
    /// A shopfront.
    Shop,
    /// The street door (a walk-up's stoop, an office's lobby doors).
    Door,
    /// A warehouse loading door.
    Loading,
}

/// One panel for a bay `g` wide on a storey from `y0` (its floor) to `y1`. Trims stand 2 cm off
/// the faces they sit on, so no faces share a plane (CLAUDE.md section 10).
#[allow(clippy::too_many_arguments)]
fn skin(out: &mut Vec<Piece>, p: &Panel, look: &Look, g: f32, y0: f32, y1: f32, ground: Ground, sign: [f32; 3]) {
    let t = 0.3;
    let wall_mat = if matches!(look.style, Style::WalkUp | Style::Warehouse) {
        Mat::Brick
    } else {
        Mat::Concrete
    };
    let wall = if wall_mat == Mat::Brick { look.brick } else { look.concrete };
    match ground {
        Ground::Shop => return shopfront(out, p, g, y0, y1, wall_mat, wall, sign, false),
        Ground::Door if look.style == Style::Office || look.style == Style::Tower => {
            return shopfront(out, p, g, y0, y1, wall_mat, wall, sign, true);
        }
        Ground::Door => {
            // A walk-up's street door: a timber door under a glazed transom, stone stoop steps.
            let (c, dw) = (g * 0.5, 1.2);
            let top = y0 + 2.6;
            out.push(p.made(wall_mat, 0.0, c - dw * 0.5 - 0.15, y0, y1, 0.0, t, wall));
            out.push(p.made(wall_mat, c + dw * 0.5 + 0.15, g, y0, y1, 0.0, t, wall));
            out.push(p.made(wall_mat, c - dw * 0.5 - 0.15, c + dw * 0.5 + 0.15, top + 0.5, y1, 0.0, t, wall));
            out.push(p.made(
                Mat::Wood,
                c - dw * 0.5,
                c + dw * 0.5,
                y0 + 0.36,
                top,
                0.18,
                0.24,
                [0.55, 0.35, 0.22],
            ));
            out.push(p.made(
                Mat::Glass,
                c - dw * 0.5,
                c + dw * 0.5,
                top + 0.06,
                top + 0.44,
                0.16,
                0.18,
                look.glass,
            ));
            out.push(p.piece(c - dw * 0.5 - 0.15, c + dw * 0.5 + 0.15, top, top + 0.06, -0.04, 0.2, look.frame));
            out.push(p.piece(c - dw * 0.5 - 0.15, c - dw * 0.5, y0, top + 0.5, -0.04, 0.2, look.frame));
            out.push(p.piece(c + dw * 0.5, c + dw * 0.5 + 0.15, y0, top + 0.5, -0.04, 0.2, look.frame));
            for step in 0..3 {
                let (h, depth) = (0.12 * (step + 1) as f32, 0.35 * (3 - step) as f32);
                out.push(p.made(
                    Mat::Concrete,
                    c - dw * 0.5 - 0.3,
                    c + dw * 0.5 + 0.3,
                    y0 + h - 0.12,
                    y0 + h,
                    -depth,
                    0.0,
                    STONE,
                ));
            }
            return;
        }
        _ => {}
    }
    match look.style {
        Style::WalkUp => {
            let n = look.windows as usize;
            let w = if n == 3 { look.window_w.min(1.0) } else { look.window_w };
            let sill = y0 + 0.85;
            let head = (sill + 1.75).min(y1 - 0.45);
            out.push(p.made(Mat::Brick, 0.0, g, y0, sill, 0.0, t, wall));
            out.push(p.made(Mat::Brick, 0.0, g, head, y1, 0.0, t, wall));
            let cs: Vec<f32> = (0..n).map(|i| g * (i as f32 + 0.5) / n as f32).collect();
            let mut s = 0.0;
            for &c in &cs {
                out.push(p.made(Mat::Brick, s, c - w * 0.5, sill, head, 0.0, t, wall));
                s = c + w * 0.5;
            }
            out.push(p.made(Mat::Brick, s, g, sill, head, 0.0, t, wall));
            for &c in &cs {
                out.push(p.made(
                    Mat::Concrete,
                    c - w * 0.5 - 0.13,
                    c + w * 0.5 + 0.13,
                    sill - 0.08,
                    sill + 0.02,
                    -0.07,
                    0.12,
                    STONE,
                ));
                if look.brick_lintels {
                    out.push(p.made(
                        Mat::Brick,
                        c - w * 0.5 - 0.12,
                        c + w * 0.5 + 0.12,
                        head - 0.02,
                        head + 0.24,
                        -0.03,
                        0.1,
                        [wall[0] * 0.75, wall[1] * 0.75, wall[2] * 0.75],
                    ));
                } else {
                    out.push(p.made(
                        Mat::Concrete,
                        c - w * 0.5 - 0.17,
                        c + w * 0.5 + 0.17,
                        head - 0.02,
                        head + 0.24,
                        -0.04,
                        0.1,
                        STONE,
                    ));
                }
                sash(out, p, c, w, sill + 0.02, head - 0.02, look, 0.12);
            }
        }
        Style::Office if look.punched_grid => {
            // A deep concrete grid: one recessed window per half bay.
            let (sill, head) = (y0 + 0.9, y1 - 0.55);
            out.push(p.made(Mat::Concrete, 0.0, g, y0, sill, -0.1, t, wall));
            out.push(p.made(Mat::Concrete, 0.0, g, head, y1, -0.1, t, wall));
            let n = 2;
            let w = g / n as f32 - 0.7;
            let mut s = 0.0;
            for i in 0..n {
                let c = g * (i as f32 + 0.5) / n as f32;
                out.push(p.made(Mat::Concrete, s, c - w * 0.5, sill, head, -0.1, t, wall));
                s = c + w * 0.5;
                out.push(p.made(Mat::Glass, c - w * 0.5, c + w * 0.5, sill, head, 0.34, 0.36, look.glass));
                out.push(p.piece(c - 0.03, c + 0.03, sill, head, 0.3, 0.37, look.mullion));
            }
            out.push(p.made(Mat::Concrete, s, g, sill, head, -0.1, t, wall));
        }
        Style::Office => {
            let (sill, head) = (y0 + 1.0, y1 - 0.5);
            out.push(p.made(Mat::Concrete, 0.0, g, y0, sill, -0.18, t, wall));
            out.push(p.made(
                Mat::Concrete,
                0.0,
                g,
                head,
                y1,
                0.0,
                t,
                [wall[0] * 0.85, wall[1] * 0.85, wall[2] * 0.85],
            ));
            out.push(p.made(Mat::Glass, 0.15, g - 0.15, sill, head, 0.2, 0.22, look.glass));
            let step = (g - 0.3) / 4.0;
            for m in 0..=4 {
                let x = 0.15 + step * m as f32;
                out.push(p.piece(x - 0.04, x + 0.04, sill, head, 0.14, 0.24, look.mullion));
            }
            out.push(p.made(Mat::Concrete, -0.17, 0.17, y0, y1, -0.8, 0.02, wall));
        }
        Style::Tower => {
            let (vis0, vis1) = (y0 + 0.1, y1 - 0.75);
            let spandrel = [look.glass[0] * 0.5, look.glass[1] * 0.5, look.glass[2] * 0.5];
            out.push(p.piece(0.0, g, vis1, y1, 0.02, 0.1, spandrel));
            out.push(p.piece(0.0, g, y0, vis0, 0.02, 0.1, spandrel));
            out.push(p.made(Mat::Glass, 0.0, g, vis0, vis1, 0.04, 0.06, look.glass));
            let n = (g / 1.5).round().max(2.0) as usize;
            for m in 0..n {
                let x = g * m as f32 / n as f32;
                out.push(p.piece(x - 0.04, x + 0.04, y0, y1, -0.14, 0.04, look.mullion));
            }
            out.push(p.piece(0.0, g, vis1 - 0.03, vis1 + 0.03, -0.08, 0.04, look.mullion));
        }
        Style::Warehouse => {
            if ground == Ground::Loading {
                let top = y0 + 4.2_f32.min(y1 - y0 - 0.4);
                out.push(p.made(Mat::Brick, 0.0, 0.6, y0, y1, 0.0, t, wall));
                out.push(p.made(Mat::Brick, g - 0.6, g, y0, y1, 0.0, t, wall));
                out.push(p.made(Mat::Brick, 0.6, g - 0.6, top, y1, 0.0, t, wall));
                for i in 0..8 {
                    let (a, b) = (y0 + (top - y0) * i as f32 / 8.0, y0 + (top - y0) * (i + 1) as f32 / 8.0);
                    let tone = if i % 2 == 0 { [0.44, 0.48, 0.36] } else { [0.40, 0.44, 0.33] };
                    out.push(p.piece(0.6, g - 0.6, a, b, 0.14 + (i % 2) as f32 * 0.03, 0.22, tone));
                }
                return;
            }
            let (sill, head) = (y0 + 1.1, y1 - 0.9);
            out.push(p.made(Mat::Brick, 0.0, g, y0, sill, 0.0, t, wall));
            out.push(p.made(Mat::Brick, 0.0, g, head, y1, 0.0, t, wall));
            out.push(p.made(Mat::Brick, 0.0, 0.9, sill, head, 0.0, t, wall));
            out.push(p.made(Mat::Brick, g - 0.9, g, sill, head, 0.0, t, wall));
            out.push(p.made(
                Mat::Brick,
                0.8,
                g - 0.8,
                head - 0.02,
                head + 0.32,
                -0.04,
                0.08,
                [wall[0] * 0.8, wall[1] * 0.8, wall[2] * 0.8],
            ));
            out.push(p.made(Mat::Concrete, 0.8, g - 0.8, sill - 0.1, sill + 0.02, -0.06, 0.1, STONE));
            out.push(p.made(Mat::Glass, 0.9, g - 0.9, sill + 0.02, head - 0.02, 0.18, 0.2, look.glass));
            for m in 0..=7 {
                let x = 0.9 + (g - 1.8) * m as f32 / 7.0;
                out.push(p.piece(x - 0.025, x + 0.025, sill, head, 0.15, 0.22, look.frame));
            }
            for m in 0..=5 {
                let y = sill + (head - sill) * m as f32 / 5.0;
                out.push(p.piece(0.9, g - 0.9, y - 0.025, y + 0.025, 0.16, 0.21, look.frame));
            }
        }
    }
}

/// A shopfront (or, with `doors`, a lobby front with a pair of glass doors): kickplate, plate
/// glass with mullions, a sign band and an awning stepped out in three slats.
#[allow(clippy::too_many_arguments)]
fn shopfront(out: &mut Vec<Piece>, p: &Panel, g: f32, y0: f32, y1: f32, wall_mat: Mat, wall: [f32; 3], sign: [f32; 3], doors: bool) {
    let top = (y0 + 3.0).min(y1 - 0.8);
    let glass = [0.32, 0.40, 0.46];
    out.push(p.made(Mat::Concrete, 0.0, g, y0, y0 + 0.45, -0.04, 0.3, KICK));
    out.push(p.made(wall_mat, 0.0, 0.3, y0 + 0.45, top, 0.0, 0.3, wall));
    out.push(p.made(wall_mat, g - 0.3, g, y0 + 0.45, top, 0.0, 0.3, wall));
    out.push(p.made(wall_mat, 0.0, g, top, y1, 0.0, 0.3, wall));
    out.push(p.made(Mat::Glass, 0.3, g - 0.3, y0 + 0.45, top, 0.12, 0.14, glass));
    for x in [0.3, g * 0.5, g - 0.3] {
        out.push(p.piece(x - 0.04, x + 0.04, y0 + 0.45, top, 0.08, 0.18, FRAME_BLACK));
    }
    if doors {
        // Two glass doors in steel frames, down to the pavement.
        let c = g * 0.5;
        out.push(p.piece(c - 1.0, c + 1.0, y0, y0 + 0.45, -0.05, 0.08, [0.55, 0.56, 0.58]));
        for (a, b) in [(c - 0.95, c - 0.05), (c + 0.05, c + 0.95)] {
            out.push(p.piece(a, b, y0 + 2.2, y0 + 2.26, 0.06, 0.16, FRAME_BLACK));
            out.push(p.piece(
                (a + b) * 0.5 - 0.02,
                (a + b) * 0.5 + 0.02,
                y0 + 0.9,
                y0 + 1.05,
                0.04,
                0.08,
                [0.75, 0.74, 0.70],
            ));
        }
    }
    out.push(p.piece(0.2, g - 0.2, top + 0.15, top + 0.7, -0.14, -0.01, sign));
    for i in 0..3 {
        let (d0, d1) = (-0.02 - 0.45 * i as f32, -0.47 - 0.45 * i as f32);
        let y = top - 0.05 - 0.16 * i as f32;
        out.push(p.piece(
            0.3,
            g - 0.3,
            y - 0.05,
            y,
            d1,
            d0,
            if i % 2 == 0 { sign } else { [0.86, 0.82, 0.74] },
        ));
    }
}

/// Dresses building `id`: `n` = [cells along x, storeys, cells along z], `origin` its least corner
/// on the ground, `size` one cell (as `destruct::Cells`). `segmented` cuts pieces per storey and
/// per bay so each can follow its element; otherwise long pieces are merged.
pub fn dress(id: u32, n: [usize; 3], origin: Vec3, size: Vec3, seed: &SimRng, segmented: bool, cfg: &DressingConfig) -> Vec<Piece> {
    let [nx, ns, nz] = n;
    if nx == 0 || ns == 0 || nz == 0 || !size.is_finite() {
        return Vec::new();
    }
    let mut srng = seed.fork(&format!("building/{id}/skin"));
    let look = look_of(ns, size.x * nx as f32 * size.z * nz as f32, &mut srng);
    dress_look(id, n, origin, size, look, srng, seed, segmented, cfg)
}

/// Which far look building `id` wears (openspec change `building-type-lods`): the same pick its
/// dressing makes, from the same stream, so near and far agree on its kind and main colour.
pub fn far_variant(id: u32, n: [usize; 3], size: Vec3, seed: &SimRng) -> FarVariant {
    let mut srng = seed.fork(&format!("building/{id}/skin"));
    look_of(n[1], size.x * n[0] as f32 * size.z * n[2] as f32, &mut srng).far_variant()
}

/// The dressing of a building wearing far variant `v` (its kind and main colour, the rest of its
/// look plain), for rendering that variant's facade once at startup.
pub fn dress_variant(v: FarVariant, n: [usize; 3], origin: Vec3, size: Vec3, cfg: &DressingConfig) -> Vec<Piece> {
    if n.contains(&0) || !size.is_finite() {
        return Vec::new();
    }
    let seed = SimRng::new(0);
    let srng = seed.fork("far facade");
    dress_look(0, n, origin, size, Look::of_variant(v), srng, &seed, false, cfg)
}

/// A building's massing at mid distance (`building-type-lods`): what its own dressing puts on and
/// above the roof line (parapet, coping, cornice, roof props), the `max` largest by volume. Its
/// walls are drawn flat with the far facade, so the skin itself is left out.
pub fn massing(id: u32, n: [usize; 3], origin: Vec3, size: Vec3, seed: &SimRng, cfg: &DressingConfig, max: usize) -> Vec<Piece> {
    let roof_y = origin.y + size.y * n[1] as f32;
    let mut kept: Vec<(usize, f32, Piece)> = dress(id, n, origin, size, seed, false, cfg)
        .into_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let (min_y, vol) = match p.shape {
                Shape::Block { min, max } => (min.y, (max - min).abs().element_product()),
                Shape::Drum { base, r0, r1, height_m } => (base.y, std::f32::consts::PI * r0.max(r1).powi(2) * height_m),
            };
            (min_y >= roof_y - 0.01).then_some((i, vol, p))
        })
        .collect();
    // Largest first, ties by order (never hash order).
    kept.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    kept.into_iter().take(max).map(|(_, _, p)| p).collect()
}

#[allow(clippy::too_many_arguments)]
fn dress_look(
    id: u32,
    n: [usize; 3],
    origin: Vec3,
    size: Vec3,
    look: Look,
    mut srng: SimRng,
    seed: &SimRng,
    segmented: bool,
    cfg: &DressingConfig,
) -> Vec<Piece> {
    let [nx, ns, nz] = n;
    let mut out = Vec::new();
    let style = look.style;
    let top_k = ns as u32 - 1;
    let roof_y = origin.y + size.y * ns as f32;
    let far = origin + Vec3::new(size.x * nx as f32, 0.0, size.z * nz as f32);
    let (pier, pier_mat) = match style {
        Style::WalkUp | Style::Warehouse => (look.brick, Mat::Brick),
        Style::Office => (look.concrete, Mat::Concrete),
        Style::Tower => ([look.glass[0] * 0.5, look.glass[1] * 0.5, look.glass[2] * 0.5], Mat::Trim),
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
    // The skin: one panel per bay per storey per side, hung on that storey's facade panel.
    let long_x = nx >= nz;
    for side in 0..4u8 {
        for idx in 0..count(side) {
            let bay = if side < 2 { size.x } else { size.z };
            let sign = pick(&mut srng, &SIGNS);
            let (plane, out_sign) = match side {
                0 => (origin.z, -1.0),
                1 => (far.z, 1.0),
                2 => (origin.x, -1.0),
                _ => (far.x, 1.0),
            };
            let long_face = (side < 2) == long_x;
            let ground = match style {
                Style::WalkUp | Style::Office if long_face && idx == count(side) / 2 => Ground::Door,
                Style::Tower if long_face && idx == count(side) / 2 => Ground::Door,
                Style::WalkUp | Style::Office if long_face => Ground::Shop,
                Style::Warehouse if long_face && idx % 3 == 1 => Ground::Loading,
                _ => Ground::Plain,
            };
            for k in 0..ns {
                let (y0, y1) = (origin.y + size.y * k as f32, origin.y + size.y * (k + 1) as f32);
                let p = Panel {
                    side,
                    s0: along(side, idx),
                    plane,
                    out: out_sign,
                    anchor: Anchor::Wall {
                        k: k as u32,
                        side,
                        idx: idx as u32,
                    },
                };
                skin(&mut out, &p, &look, bay, y0, y1, if k == 0 { ground } else { Ground::Plain }, sign);
            }
        }
    }
    // Floors, and the roof. A whole building near the player shows its rooms through the glass
    // (`interior_pieces`); a damaged one draws them in its bands instead.
    let t_roof = 0.25;
    let roof_tone = [0.9, 0.9, 0.9];
    if segmented {
        for i in 0..nx {
            for j in 0..nz {
                let lo = origin + Vec3::new(size.x * i as f32, size.y * ns as f32 - t_roof, size.z * j as f32);
                out.push(Piece {
                    mat: Mat::Roof,
                    ..block(
                        lo,
                        lo + Vec3::new(size.x, t_roof, size.z),
                        roof_tone,
                        Anchor::Slab {
                            k: top_k,
                            i: i as u32,
                            j: j as u32,
                        },
                    )
                });
            }
        }
    } else {
        out.push(Piece {
            mat: Mat::Roof,
            ..block(
                Vec3::new(origin.x, roof_y - t_roof, origin.z),
                Vec3::new(far.x, roof_y, far.z),
                roof_tone,
                Anchor::Whole,
            )
        });
        let inset = 0.3;
        for k in 1..ns {
            let y = origin.y + size.y * k as f32;
            out.push(Piece {
                mat: Mat::Concrete,
                ..block(
                    Vec3::new(origin.x + inset, y - t_roof, origin.z + inset),
                    Vec3::new(far.x - inset, y, far.z - inset),
                    [0.62, 0.60, 0.56],
                    Anchor::Whole,
                )
            });
        }
    }
    // Parapet and coping. Side walls (2, 3) stop short of the corners so no faces share a plane.
    let t = cfg.parapet_wall_m;
    let cap = cfg.coping_m;
    let parapet_color = pier;
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
            out.push(Piece {
                mat: pier_mat,
                ..block(lo, hi, parapet_color, anchor)
            });
            let (lo, hi) = side_box(side, c0, c1, roof_y + cfg.parapet_m, roof_y + cfg.parapet_m + cap, cap, t + cap);
            out.push(if style == Style::Tower {
                block(lo, hi, look.mullion, anchor)
            } else {
                Piece {
                    mat: Mat::Concrete,
                    ..block(lo, hi, STONE, anchor)
                }
            });
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
        let tone = if matches!(style, Style::WalkUp | Style::Warehouse) {
            BRICK_PIER
        } else {
            CONCRETE
        };
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
        Style::Warehouse => &["vent", "vent", "hvac", "tank", "duct"],
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
                        color: [0.80, 0.60, 0.42],
                        anchor: a,
                        mat: Mat::Wood,
                    });
                    out.push(Piece {
                        shape: Shape::Drum {
                            base: c + Vec3::Y * 6.72,
                            r0: 2.05,
                            r1: 0.12,
                            height_m: 1.3,
                        },
                        color: [0.55, 0.42, 0.32],
                        anchor: a,
                        mat: Mat::Wood,
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
                            mat: Mat::Trim,
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
                        mat: Mat::Trim,
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
                        mat: Mat::Trim,
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
    fn every_bay_of_every_storey_has_a_skin() {
        let pieces = go(9, true);
        for k in 0..9u32 {
            for side in 0..4u8 {
                for idx in 0..if side < 2 { 5 } else { 4 } {
                    let a = Anchor::Wall { k, side, idx };
                    assert!(pieces.iter().any(|p| p.anchor == a), "storey {k} side {side} bay {idx} has a panel");
                }
            }
        }
        assert!(pieces.iter().any(|p| p.mat == Mat::Glass), "and windows");
    }

    #[test]
    fn a_whole_building_is_backed_and_a_cut_one_is_not() {
        assert!(go(9, false).iter().any(|p| p.anchor == Anchor::Whole));
        assert!(go(9, true).iter().all(|p| p.anchor != Anchor::Whole));
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
