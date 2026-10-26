//! Marks a mech leaves on the ground (openspec change `ground-marks`; owner, 2026-10-24: "making
//! mech steps really impact. Stamp mech footprint decals like we did with mining mike. Kick up dirt,
//! skates should leave dirt trails. Concrete would get concrete footprints").
//!
//! The rules, with no engine in them: what the ground is under a foot (`mark_surface`), the marks
//! themselves (a footprint, a skate's trail segment, a crater's scorch) lying on the one height
//! query, the fixed ring they live in (the oldest replaced first) and how they fade, the skate
//! trails laid end to end, and the short jolt of the pilot's view on a foot strike. `cb-app::marks`
//! packs the ring into one storage buffer and draws every mark in one draw call.
//!
//! Ported from Mining Mike (`godot-sandbox`, branch `origin/claude/long-haul-playtest`, commit
//! 74e1412c): `godot-sandbox/scripts/fx/ground_dents.gd` (GroundDents: the shapes FOOT, GROOVE,
//! CRATER and TRAIL; the quad 1.55 times the dent's half size; the fade over the last third of a
//! life; the oldest going first past the cap; `add_trail` laying segments end to end) and
//! `godot-sandbox/scripts/rivals/uber_race.gd` (`_lay_trails`: a wheel's line laid from where its
//! last segment ended, every so many metres it glides). Rewritten in Rust for Crab Breeze.

use crate::data::{Validate, finite, positive, within};
use crate::grass::CrushRect;
use crate::world::{Ground, Surface, WorldPlan};
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

/// A mark's shape, as the shader draws it (the numbers are the shader's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkShape {
    /// A mech's sole: a rounded box, wider at the toe, with tread bars.
    Foot = 0,
    /// A short rounded groove (a body's slide).
    Groove = 1,
    /// A ragged round bowl with a scorched middle.
    Crater = 2,
    /// A skate's straight channel filling its quad's whole length, laid end to end.
    Trail = 3,
}

/// The quad a mark is drawn on is this many times its half size across, and along for every shape
/// but a trail (which fills its quad's length so segments butt end to end). Mining Mike's number.
pub const QUAD_MARGIN: f32 = 1.55;

/// A crater's bowl reaches this share of its quad's half size (the shader's crater shape).
pub const CRATER_SHARE: f32 = 0.6;

/// A mark is drawn on a grid of this many vertices a side, each at one sampled ground height.
pub const GRID: usize = 5;

/// The floats one mark takes in the GPU buffer: ten `vec4<f32>` (`Mark` in `ground_marks.wesl`):
/// three of header, then the 25 ground heights, the shape scale (two floats) and a spare zero.
pub const ROW_FLOATS: usize = 40;

/// One mark on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    /// Its middle, on the ground (world metres).
    pub centre: Vec3,
    /// Its long axis's heading: local +z points along (sin yaw, 0, cos yaw).
    pub yaw_rad: f32,
    /// Half the quad's size across and along, metres.
    pub extent_m: Vec2,
    /// Shape.
    pub shape: MarkShape,
    /// Soil or paving.
    pub surface: Surface,
    /// How deep it is pressed, metres.
    pub depth_m: f32,
    /// The lip of pushed-up soil round it, metres (0 on paving).
    pub lip_m: f32,
    /// Seconds it lasts.
    pub life_s: f32,
    /// Seconds since it was laid.
    pub age_s: f32,
    /// A number in 0..1 that varies its cracks and grain.
    pub seed: f32,
    /// The quad's half size over the shape's own (1 when the shape fills the quad as Mining Mike
    /// drew it; more for a paving print, whose cracks and dust reach past the sole).
    pub shape_scale: Vec2,
    /// The ground's height on a `GRID` by `GRID` grid over the quad (across then along, from
    /// -extent to +extent), relative to `centre.y`: the drawn grid's vertices lie on it, so the
    /// mark bends over the slope.
    pub heights_m: [f32; GRID * GRID],
}

impl Mark {
    fn new(centre: Vec3, yaw_rad: f32, extent_m: Vec2, shape: MarkShape, surface: Surface) -> Self {
        Self {
            centre,
            yaw_rad,
            extent_m,
            shape,
            surface,
            depth_m: 0.0,
            lip_m: 0.0,
            life_s: 1.0,
            age_s: 0.0,
            seed: 0.0,
            shape_scale: Vec2::ONE,
            heights_m: [0.0; GRID * GRID],
        }
    }

    /// The unit vectors across (local +x) and along (local +z) the mark, on the ground plane.
    pub fn axes(&self) -> (Vec3, Vec3) {
        let (s, c) = self.yaw_rad.sin_cos();
        (Vec3::new(c, 0.0, -s), Vec3::new(s, 0.0, c))
    }

    /// Where vertex `i` (across) of row `j` (along) of the grid lies on the quad, -1..1 each way.
    pub fn grid_uv(i: usize, j: usize) -> Vec2 {
        let step = 2.0 / (GRID - 1) as f32;
        Vec2::new(i as f32 * step - 1.0, j as f32 * step - 1.0)
    }

    /// The mark laid on the ground: its middle at the ground's height, and the grid of heights it
    /// bends over sampled from the one height query.
    pub fn on_ground(mut self, ground: &impl Ground) -> Self {
        let (across, along) = self.axes();
        let mid = ground.height_at(self.centre.x, self.centre.z);
        self.centre.y = mid;
        for j in 0..GRID {
            for i in 0..GRID {
                let uv = Self::grid_uv(i, j);
                let p = self.centre + across * (uv.x * self.extent_m.x) + along * (uv.y * self.extent_m.y);
                let h = ground.height_at(p.x, p.z) - mid;
                self.heights_m[j * GRID + i] = if h.is_finite() { h } else { 0.0 };
            }
        }
        self
    }

    /// How strongly it shows now, 0..1.
    pub fn fade(&self) -> f32 {
        fade(self.age_s, self.life_s)
    }

    /// Its ten `vec4` rows for the GPU: (centre, yaw), (extent, depth, lip), (shape, surface,
    /// fade, seed), then the 25 heights and three spare zeros.
    pub fn pack(&self) -> [f32; ROW_FLOATS] {
        let surface = match self.surface {
            Surface::Dirt => 0.0,
            Surface::Concrete => 1.0,
        };
        let mut out = [0.0; ROW_FLOATS];
        out[..12].copy_from_slice(&[
            self.centre.x,
            self.centre.y,
            self.centre.z,
            self.yaw_rad,
            self.extent_m.x,
            self.extent_m.y,
            self.depth_m,
            self.lip_m,
            self.shape as u32 as f32,
            surface,
            self.fade(),
            self.seed,
        ]);
        out[12..12 + GRID * GRID].copy_from_slice(&self.heights_m);
        out[12 + GRID * GRID] = self.shape_scale.x;
        out[13 + GRID * GRID] = self.shape_scale.y;
        out
    }

    /// True for a mark that crushes the grass under it: a print or a skate line in soil.
    pub fn crushes(&self) -> bool {
        self.surface == Surface::Dirt && matches!(self.shape, MarkShape::Foot | MarkShape::Trail)
    }

    /// The grass this mark crushes, if it crushes any: the sole (or the trail's width) grown by
    /// `margin_m` each way.
    pub fn crush(&self, margin_m: f32) -> Option<CrushRect> {
        if !self.crushes() {
            return None;
        }
        let (_, along) = self.axes();
        let half = match self.shape {
            MarkShape::Trail => Vec2::new(self.extent_m.x / QUAD_MARGIN, self.extent_m.y),
            _ => self.extent_m / (self.shape_scale * QUAD_MARGIN),
        };
        Some(CrushRect {
            centre: Vec2::new(self.centre.x, self.centre.z),
            along: Vec2::new(along.x, along.z),
            half_m: half + Vec2::splat(margin_m.max(0.0)),
        })
    }
}

/// How strongly a mark `age_s` old with a life of `life_s` shows: fully for the first two thirds,
/// then fading straight to nothing at the end (Mining Mike's `3 (1 - age / life)`, clamped).
pub fn fade(age_s: f32, life_s: f32) -> f32 {
    if !(age_s.is_finite() && life_s > 0.0) {
        return 0.0;
    }
    (3.0 * (1.0 - age_s / life_s)).clamp(0.0, 1.0)
}

/// What a foot or a wheel presses into at `p` (x, z): nothing in water (the water's own ripples
/// and spray show it), else soil or paving by the one cover rule (`WorldPlan::surface_at`).
pub fn mark_surface(plan: &WorldPlan, ground: &impl Ground, p: Vec2) -> Option<Surface> {
    if ground.water_at(p.x, p.y).is_some() {
        return None;
    }
    Some(plan.surface_at(p))
}

/// Every mark alive, in a ring of fixed size: a new mark takes the slot after the last one laid,
/// so past the capacity the oldest goes first, and the GPU buffer never changes size.
#[derive(Clone, Debug)]
pub struct MarkRing {
    slots: Vec<Option<Mark>>,
    next: usize,
    laid: u64,
    crush_version: u64,
}

impl MarkRing {
    /// A ring of `capacity` slots (at least one).
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity.max(1)],
            next: 0,
            laid: 0,
            crush_version: 0,
        }
    }

    /// Slots.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Lays `mark` in the next slot (replacing the oldest when the ring is full) and returns the
    /// slot. A mark with a non-finite position is refused (`None`) and changes nothing.
    pub fn add(&mut self, mut mark: Mark) -> Option<usize> {
        if !(mark.centre.is_finite() && mark.yaw_rad.is_finite() && mark.extent_m.is_finite()) {
            return None;
        }
        let slot = self.next;
        mark.age_s = 0.0;
        // A golden-ratio walk: neighbouring prints get different cracks.
        mark.seed = (self.laid as f32 * 0.618_034).fract();
        if mark.crushes() || self.slots[slot].is_some_and(|m| m.crushes()) {
            self.crush_version += 1;
        }
        self.slots[slot] = Some(mark);
        self.next = (slot + 1) % self.slots.len();
        self.laid += 1;
        Some(slot)
    }

    /// The mark in `slot`, if one lives there.
    pub fn get(&self, slot: usize) -> Option<&Mark> {
        self.slots.get(slot).and_then(Option::as_ref)
    }

    /// Ages every mark by `dt_s` and clears the ones whose life is out. True if any mark lives.
    pub fn advance(&mut self, dt_s: f32) -> bool {
        let dt = if dt_s.is_finite() { dt_s.max(0.0) } else { 0.0 };
        let mut any = false;
        for slot in &mut self.slots {
            if let Some(m) = slot {
                m.age_s += dt;
                if m.age_s >= m.life_s {
                    if m.crushes() {
                        self.crush_version += 1;
                    }
                    *slot = None;
                } else {
                    any = true;
                }
            }
        }
        any
    }

    /// Marks alive.
    pub fn live(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// The whole ring as GPU rows, slot by slot; an empty slot is all zeros (fade 0: not drawn).
    pub fn pack_into(&self, out: &mut [[f32; 4]]) {
        for (i, slot) in self.slots.iter().enumerate() {
            let row = slot.map_or([0.0; ROW_FLOATS], |m| m.pack());
            for k in 0..ROW_FLOATS / 4 {
                if let Some(o) = out.get_mut(i * ROW_FLOATS / 4 + k) {
                    *o = [row[k * 4], row[k * 4 + 1], row[k * 4 + 2], row[k * 4 + 3]];
                }
            }
        }
    }

    /// Moves on every time a mark that crushes grass is laid, replaced or fades out: the grass
    /// repacks its tiles only when it changes.
    pub fn crush_version(&self) -> u64 {
        self.crush_version
    }

    /// The grass every live mark crushes, slot by slot.
    pub fn crushes(&self, margin_m: f32) -> Vec<CrushRect> {
        self.slots.iter().flatten().filter_map(|m| m.crush(margin_m)).collect()
    }
}

/// The one mesh every mark is drawn from: for each of `capacity` slots a `GRID` by `GRID` grid of
/// vertices, each carrying only (slot, grid x, grid z); the shader places it. Two triangles a cell.
pub fn grid_mesh(capacity: usize) -> (Vec<[f32; 3]>, Vec<u32>) {
    let per = GRID * GRID;
    let mut vertices = Vec::with_capacity(capacity * per);
    let mut indices = Vec::with_capacity(capacity * (GRID - 1) * (GRID - 1) * 6);
    for slot in 0..capacity {
        let base = (slot * per) as u32;
        for j in 0..GRID {
            for i in 0..GRID {
                vertices.push([slot as f32, i as f32, j as f32]);
            }
        }
        for j in 0..GRID - 1 {
            for i in 0..GRID - 1 {
                let v = base + (j * GRID + i) as u32;
                let g = GRID as u32;
                indices.extend_from_slice(&[v, v + g, v + 1, v + 1, v + g, v + g + 1]);
            }
        }
    }
    (vertices, indices)
}

/// Lays each skate wheel's trail as straight segments end to end (Mining Mike's `_lay_trails` and
/// `add_trail`): a wheel's line runs from where its last segment ended to where the wheel is, each
/// time it has glided `segment_m`. A jump farther than `break_m` (a hop, a respawn) starts a new
/// line, and a wheel not fed for a while (off its skates, in the air) is forgotten.
#[derive(Clone, Debug, Default)]
pub struct TrailLayer {
    /// Each wheel's line: where its last segment ended and when it was last fed (seconds).
    ends: BTreeMap<(u32, u8), (Vec2, f32)>,
}

impl TrailLayer {
    /// Wheel `key` (mech id, wheel) is at `at` at `now_s`: the segment to lay, if one is due.
    pub fn feed(&mut self, key: (u32, u8), at: Vec2, now_s: f32, segment_m: f32, break_m: f32) -> Option<(Vec2, Vec2)> {
        if !at.is_finite() {
            return None;
        }
        let Some((from, seen)) = self.ends.get_mut(&key) else {
            self.ends.insert(key, (at, now_s));
            return None;
        };
        *seen = now_s;
        let d = at.distance(*from);
        if d > break_m {
            *from = at;
            None
        } else if d >= segment_m {
            let seg = (*from, at);
            *from = at;
            Some(seg)
        } else {
            None
        }
    }

    /// Forgets every wheel not fed since `now_s - gap_s`: its next contact starts a new line.
    pub fn forget(&mut self, now_s: f32, gap_s: f32) {
        self.ends.retain(|_, (_, seen)| now_s - *seen <= gap_s);
    }

    /// Wheels with a line in progress.
    pub fn lines(&self) -> usize {
        self.ends.len()
    }
}

/// How deep and how long each mark is on one surface.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SurfaceMarks {
    /// A footprint's depth, metres.
    pub foot_depth_m: f32,
    /// The lip of soil round a footprint, metres.
    pub foot_lip_m: f32,
    /// A skate trail's depth, metres.
    pub trail_depth_m: f32,
    /// The lip either side of a skate trail, metres.
    pub trail_lip_m: f32,
    /// A crater mark's depth (the dug crater is the terrain's; this is the scorch's relief), metres.
    pub crater_depth_m: f32,
}

impl Default for SurfaceMarks {
    fn default() -> Self {
        Self {
            foot_depth_m: 0.32,
            foot_lip_m: 0.14,
            trail_depth_m: 0.22,
            trail_lip_m: 0.08,
            crater_depth_m: 0.15,
        }
    }
}

impl SurfaceMarks {
    fn validate(&self, at: &str) -> Result<(), String> {
        let f = |k: &str| format!("{at}.{k}");
        within(&f("foot_depth_m"), self.foot_depth_m, 0.0, 2.0)?;
        within(&f("foot_lip_m"), self.foot_lip_m, 0.0, 1.0)?;
        within(&f("trail_depth_m"), self.trail_depth_m, 0.0, 2.0)?;
        within(&f("trail_lip_m"), self.trail_lip_m, 0.0, 1.0)?;
        within(&f("crater_depth_m"), self.crater_depth_m, 0.0, 5.0)
    }
}

/// How the marks look (the shader's numbers).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct MarkLook {
    /// What the bottom of a print in soil is multiplied by, linear rgb.
    pub soil_tint: [f32; 3],
    /// The bottom of a print in soil darkens by this share more.
    pub pit_dark: f32,
    /// What a crack in paving is multiplied by, linear rgb.
    pub crack_colour: [f32; 3],
    /// A crack's width at its root, metres.
    pub crack_width_m: f32,
    /// The longest crack runs this many sole half lengths from the middle; a paving print's quad
    /// reaches this far every way.
    pub crack_reach: f32,
    /// Inside a print on paving the ground is multiplied by this (the crushed sole).
    pub scuff: f32,
    /// The broken slabs inside a paving print are about this wide, metres.
    pub slab_m: f32,
    /// The dust round a print on paving brightens the ground by this factor.
    pub dust_brighten: f32,
    /// The ambient share in a mark's own relief lighting (Mining Mike's 0.7).
    pub ambient: f32,
    /// A mark is pulled toward the eye by this share of its distance (at most 0.5 m), so it never
    /// shares the ground's depth.
    pub pull_per_m: f32,
}

impl Default for MarkLook {
    fn default() -> Self {
        Self {
            soil_tint: [0.8, 0.42, 0.36],
            pit_dark: 0.45,
            crack_colour: [0.22, 0.21, 0.2],
            crack_width_m: 0.08,
            crack_reach: 1.7,
            scuff: 0.5,
            slab_m: 0.35,
            dust_brighten: 1.35,
            ambient: 0.7,
            pull_per_m: 0.002,
        }
    }
}

impl MarkLook {
    fn validate(&self) -> Result<(), String> {
        for (k, c) in [("look.soil_tint", self.soil_tint), ("look.crack_colour", self.crack_colour)] {
            for v in c {
                within(k, v, 0.0, 1.0)?;
            }
        }
        within("look.pit_dark", self.pit_dark, 0.0, 1.0)?;
        within("look.crack_width_m", self.crack_width_m, 0.0, 1.0)?;
        within("look.crack_reach", self.crack_reach, 0.0, 5.0)?;
        within("look.scuff", self.scuff, 0.0, 1.0)?;
        positive("look.slab_m", self.slab_m)?;
        within("look.dust_brighten", self.dust_brighten, 1.0, 2.0)?;
        within("look.ambient", self.ambient, 0.0, 4.0)?;
        within("look.pull_per_m", self.pull_per_m, 0.0, 0.05)
    }
}

/// The pilot's view jolting on the piloted mech's foot strikes: a short drop and recovery of the
/// eye's POSITION, never its direction (CLAUDE.md section 8).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct JoltConfig {
    /// A step's drop standing still, centimetres.
    pub step_cm: f32,
    /// Added per metre a second of speed, centimetres.
    pub per_m_s_cm: f32,
    /// The most a step drops, centimetres.
    pub max_cm: f32,
    /// A landing's drop, centimetres.
    pub landing_cm: f32,
    /// How long a jolt lasts, seconds.
    pub duration_s: f32,
    /// The sideways shove toward the striking foot, as a share of the drop.
    pub side_share: f32,
}

impl Default for JoltConfig {
    fn default() -> Self {
        Self {
            step_cm: 1.0,
            per_m_s_cm: 0.2,
            max_cm: 4.0,
            landing_cm: 7.0,
            duration_s: 0.15,
            side_share: 0.25,
        }
    }
}

impl JoltConfig {
    /// A strike's drop, metres: a step's grows with speed up to the cap; a landing's is its own.
    pub fn amplitude_m(&self, speed_m_s: f32, landed: bool) -> f32 {
        let cm = if landed {
            self.landing_cm
        } else {
            (self.step_cm + self.per_m_s_cm * speed_m_s.abs()).min(self.max_cm)
        };
        if cm.is_finite() { cm.max(0.0) * 0.01 } else { 0.0 }
    }

    fn validate(&self) -> Result<(), String> {
        within("jolt.step_cm", self.step_cm, 0.0, 20.0)?;
        within("jolt.per_m_s_cm", self.per_m_s_cm, 0.0, 5.0)?;
        within("jolt.max_cm", self.max_cm, 0.0, 30.0)?;
        within("jolt.landing_cm", self.landing_cm, 0.0, 30.0)?;
        within("jolt.duration_s", self.duration_s, 0.01, 1.0)?;
        within("jolt.side_share", self.side_share, 0.0, 1.0)
    }
}

/// The jolt's shape over its life `k` (0..1): a sharp drop to its deepest a quarter of the way in,
/// then an easy recovery to nothing at the end.
pub fn jolt_profile(k: f32) -> f32 {
    if !(0.0..1.0).contains(&k) {
        return 0.0;
    }
    (std::f32::consts::PI * k.sqrt()).sin()
}

/// One strike's jolt.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Kick {
    age_s: f32,
    amp_m: f32,
    /// Unit vector toward the striking foot (zero for a landing).
    side: Vec3,
}

/// The jolts playing on the pilot's eye now (a few can overlap).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Jolt {
    kicks: Vec<Kick>,
}

impl Jolt {
    /// The most jolts that play at once; a new one past it replaces the oldest.
    pub const MAX_KICKS: usize = 4;

    /// A foot strikes: a jolt of `amp_m`, shoved toward `side` (a unit vector on the ground).
    pub fn kick(&mut self, amp_m: f32, side: Vec3) {
        if !(amp_m.is_finite() && amp_m > 0.0) {
            return;
        }
        if self.kicks.len() >= Self::MAX_KICKS {
            self.kicks.remove(0);
        }
        let side = if side.is_finite() { side.normalize_or_zero() } else { Vec3::ZERO };
        self.kicks.push(Kick { age_s: 0.0, amp_m, side });
    }

    /// Ages the jolts by `dt_s`; a finished one is gone.
    pub fn advance(&mut self, dt_s: f32, cfg: &JoltConfig) {
        let dt = if dt_s.is_finite() { dt_s.max(0.0) } else { 0.0 };
        for k in &mut self.kicks {
            k.age_s += dt;
        }
        self.kicks.retain(|k| k.age_s < cfg.duration_s);
    }

    /// The eye's offset now, world metres: down, and a little toward the striking foot.
    pub fn offset(&self, cfg: &JoltConfig) -> Vec3 {
        self.kicks
            .iter()
            .map(|k| {
                let p = jolt_profile(k.age_s / cfg.duration_s) * k.amp_m;
                Vec3::NEG_Y * p + k.side * (p * cfg.side_share)
            })
            .sum()
    }

    /// True while a jolt plays.
    pub fn active(&self) -> bool {
        !self.kicks.is_empty()
    }
}

/// `data/fx/ground_marks.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct GroundMarksConfig {
    /// Most marks alive at once (the GPU buffer's size); a new one past it replaces the oldest.
    pub max_marks: u32,
    /// Marks are laid only within this distance of the pilot's eye, metres.
    pub lay_within_m: f32,
    /// Marks fade out toward this distance from the eye and are not drawn past it, metres.
    pub draw_within_m: f32,
    /// Quads lie this far above the ground, metres (plus a pull toward the eye that grows with
    /// distance, so a mark never shares the ground's depth).
    pub lift_m: f32,
    /// Half a footprint's width, metres (the stand-in's sole is 1.4 m wide).
    pub foot_half_width_m: f32,
    /// Half a footprint's length, metres (the stand-in's sole is 2.6 m long).
    pub foot_half_length_m: f32,
    /// The sole's middle lies this far ahead of the ankle, metres.
    pub sole_ahead_m: f32,
    /// A footprint lasts this long, seconds.
    pub foot_life_s: f32,
    /// A landing presses this many times deeper.
    pub landing_depth_scale: f32,
    /// Half a skate trail's width, metres.
    pub trail_half_width_m: f32,
    /// A trail segment is laid each time a wheel glides this far, metres.
    pub trail_segment_m: f32,
    /// A wheel that jumps farther than this starts a new line, metres.
    pub trail_break_m: f32,
    /// A trail lasts this long, seconds.
    pub trail_life_s: f32,
    /// A wheel lays a trail only above this speed, m/s.
    pub trail_min_speed_m_s: f32,
    /// A wheel throws dirt or sparks only above this speed, m/s.
    pub skate_fx_min_m_s: f32,
    /// A crater's scorch lasts this long, seconds.
    pub crater_life_s: f32,
    /// A soil print or skate line clears the grass this far past its sole or its width, metres.
    pub crush_margin_m: f32,
    /// Marks in soil.
    pub dirt: SurfaceMarks,
    /// Marks on paving.
    pub concrete: SurfaceMarks,
    /// The pilot's view on a foot strike.
    pub jolt: JoltConfig,
    /// How the marks look.
    pub look: MarkLook,
}

impl Default for GroundMarksConfig {
    fn default() -> Self {
        Self {
            max_marks: 1024,
            lay_within_m: 800.0,
            draw_within_m: 450.0,
            lift_m: 0.04,
            foot_half_width_m: 0.8,
            foot_half_length_m: 1.3,
            sole_ahead_m: 0.4,
            foot_life_s: 90.0,
            landing_depth_scale: 1.6,
            trail_half_width_m: 0.45,
            trail_segment_m: 4.0,
            trail_break_m: 14.0,
            trail_life_s: 40.0,
            trail_min_speed_m_s: 1.0,
            skate_fx_min_m_s: 5.0,
            crater_life_s: 240.0,
            crush_margin_m: 0.3,
            dirt: SurfaceMarks::default(),
            concrete: SurfaceMarks {
                foot_depth_m: 0.1,
                foot_lip_m: 0.0,
                trail_depth_m: 0.01,
                trail_lip_m: 0.0,
                crater_depth_m: 0.05,
            },
            jolt: JoltConfig::default(),
            look: MarkLook::default(),
        }
    }
}

impl GroundMarksConfig {
    /// The middle of a footprint's sole: `lead_m` plus `sole_ahead_m` ahead of the ankle's ground point
    /// `foot`, for a mech heading `heading_rad`. The one place a foot strike is turned into where the sole
    /// lands: the footprint is stamped there and a car under it is crushed there (car-traffic).
    pub fn sole_centre(&self, foot: Vec3, heading_rad: f32, lead_m: f32) -> Vec3 {
        foot + crate::mech::forward(heading_rad) * (lead_m + self.sole_ahead_m)
    }

    /// The depths for a surface.
    pub fn on(&self, surface: Surface) -> &SurfaceMarks {
        match surface {
            Surface::Dirt => &self.dirt,
            Surface::Concrete => &self.concrete,
        }
    }

    /// A footprint at the ground under `at` (the sole's middle), its toe toward `heading_rad` (the
    /// mech's: 0 faces -z), pressed deeper by a landing.
    pub fn footprint(&self, surface: Surface, at: Vec3, heading_rad: f32, landed: bool) -> Mark {
        let s = self.on(surface);
        let deeper = if landed { self.landing_depth_scale } else { 1.0 };
        let sole = Vec2::new(self.foot_half_width_m, self.foot_half_length_m) * QUAD_MARGIN;
        // On paving the cracks and the dust reach past the sole: the quad grows, the sole does not.
        let extent = match surface {
            Surface::Dirt => sole,
            Surface::Concrete => sole.max(Vec2::splat(self.foot_half_length_m * self.look.crack_reach * 1.1)),
        };
        let mut m = Mark::new(at, heading_rad + std::f32::consts::PI, extent, MarkShape::Foot, surface);
        m.shape_scale = extent / sole;
        m.depth_m = s.foot_depth_m * deeper;
        m.lip_m = s.foot_lip_m * deeper;
        m.life_s = self.foot_life_s;
        m
    }

    /// A skate trail's straight segment from `a` to `b` (on the ground), filling its quad's length
    /// so the next segment from `b` carries the line on with no seam.
    pub fn trail(&self, surface: Surface, a: Vec3, b: Vec3) -> Mark {
        let s = self.on(surface);
        let d = Vec2::new(b.x - a.x, b.z - a.z);
        let mut m = Mark::new(
            (a + b) * 0.5,
            d.x.atan2(d.y),
            Vec2::new(self.trail_half_width_m * QUAD_MARGIN, d.length() * 0.5),
            MarkShape::Trail,
            surface,
        );
        m.depth_m = s.trail_depth_m;
        m.lip_m = s.trail_lip_m;
        m.life_s = self.trail_life_s;
        m
    }

    /// A crater's scorch of radius `radius_m` at `at`, turned by `yaw_rad`.
    pub fn crater(&self, surface: Surface, at: Vec3, radius_m: f32, yaw_rad: f32) -> Mark {
        let s = self.on(surface);
        let mut m = Mark::new(at, yaw_rad, Vec2::splat(radius_m / CRATER_SHARE), MarkShape::Crater, surface);
        m.depth_m = s.crater_depth_m;
        m.lip_m = s.crater_depth_m * 0.5;
        m.life_s = self.crater_life_s;
        m
    }
}

impl Validate for GroundMarksConfig {
    fn validate(&self) -> Result<(), String> {
        if !(16..=16_384).contains(&self.max_marks) {
            return Err(format!("`max_marks` must be within 16 and 16384, got {}", self.max_marks));
        }
        positive("lay_within_m", self.lay_within_m)?;
        positive("draw_within_m", self.draw_within_m)?;
        within("lift_m", self.lift_m, 0.0, 1.0)?;
        positive("foot_half_width_m", self.foot_half_width_m)?;
        positive("foot_half_length_m", self.foot_half_length_m)?;
        finite("sole_ahead_m", self.sole_ahead_m)?;
        positive("foot_life_s", self.foot_life_s)?;
        within("landing_depth_scale", self.landing_depth_scale, 1.0, 5.0)?;
        positive("trail_half_width_m", self.trail_half_width_m)?;
        positive("trail_segment_m", self.trail_segment_m)?;
        if !(self.trail_break_m.is_finite() && self.trail_break_m > self.trail_segment_m) {
            return Err(format!(
                "`trail_break_m` must be finite and longer than `trail_segment_m` ({}), got {}",
                self.trail_segment_m, self.trail_break_m
            ));
        }
        positive("trail_life_s", self.trail_life_s)?;
        within("trail_min_speed_m_s", self.trail_min_speed_m_s, 0.0, 100.0)?;
        within("skate_fx_min_m_s", self.skate_fx_min_m_s, 0.0, 100.0)?;
        positive("crater_life_s", self.crater_life_s)?;
        within("crush_margin_m", self.crush_margin_m, 0.0, 5.0)?;
        self.dirt.validate("dirt")?;
        self.concrete.validate("concrete")?;
        self.look.validate()?;
        self.jolt.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{load_ron, parse_ron};
    use crate::world::{FlatGround, WorldConfig, generate};
    use std::path::Path;
    use std::sync::OnceLock;

    fn plan() -> &'static WorldPlan {
        static PLAN: OnceLock<WorldPlan> = OnceLock::new();
        PLAN.get_or_init(|| generate(42, &WorldConfig::default()))
    }

    fn cfg() -> GroundMarksConfig {
        GroundMarksConfig::default()
    }

    fn print_at(x: f32) -> Mark {
        cfg().footprint(Surface::Dirt, Vec3::new(x, 0.0, 0.0), 0.0, false)
    }

    #[test]
    fn a_road_gives_concrete_a_meadow_gives_dirt_and_water_gives_nothing() {
        let p = plan();
        // A highway's paved middle, on dry land.
        let road = p
            .highways
            .iter()
            .flat_map(|h| (1..20).map(move |i| h.point_at(i as f32 / 20.0)))
            .find(|q| p.water_at(q.x, q.y).is_none())
            .expect("a highway on dry land");
        assert_eq!(mark_surface(p, p, road), Some(Surface::Concrete), "a highway is paved");
        let street = p.cities[0].streets[0].rect.centre();
        assert_eq!(mark_surface(p, p, street), Some(Surface::Concrete), "a city street is paved");
        let meadow = crate::grass::meadow_start(p, &crate::grass::GrassConfig::default(), 40.0, 6000.0)
            .expect("a meadow near the start")
            .mech_xz;
        assert_eq!(mark_surface(p, p, meadow), Some(Surface::Dirt), "a meadow is soil");
        let sea = Vec2::splat(p.terrain.origin_m + 50.0);
        assert!(p.water_at(sea.x, sea.y).is_some(), "the world's corner is sea");
        assert_eq!(mark_surface(p, p, sea), None, "water takes no mark");
    }

    #[test]
    fn a_park_in_a_city_is_soil() {
        let p = plan();
        let park = p
            .cities
            .iter()
            .flat_map(|c| c.blocks.iter())
            .find(|b| b.park)
            .map(|b| b.rect.centre());
        if let Some(c) = park {
            assert_eq!(p.surface_at(c), Surface::Dirt);
        }
    }

    #[test]
    fn past_its_capacity_the_ring_replaces_the_oldest_mark_first() {
        let mut ring = MarkRing::new(3);
        for i in 0..3 {
            assert_eq!(ring.add(print_at(i as f32)), Some(i));
        }
        assert_eq!(ring.add(print_at(3.0)), Some(0), "the fourth takes the first's slot");
        assert_eq!(ring.get(0).unwrap().centre.x, 3.0);
        assert_eq!(ring.get(1).unwrap().centre.x, 1.0, "the second is still there");
        assert_eq!(ring.add(print_at(4.0)), Some(1), "then the second goes");
        assert_eq!(ring.live(), 3);
    }

    #[test]
    fn a_mark_with_a_broken_position_is_refused_and_changes_nothing() {
        let mut ring = MarkRing::new(2);
        assert_eq!(ring.add(print_at(f32::NAN)), None);
        assert_eq!(ring.live(), 0);
        assert_eq!(ring.add(print_at(1.0)), Some(0), "the slot was not used");
    }

    #[test]
    fn a_mark_shows_fully_for_two_thirds_of_its_life_then_fades_to_nothing() {
        assert_eq!(fade(0.0, 90.0), 1.0);
        assert_eq!(fade(59.0, 90.0), 1.0, "two thirds in, still whole");
        assert!((fade(75.0, 90.0) - 0.5).abs() < 1e-5, "half way through the last third");
        assert_eq!(fade(90.0, 90.0), 0.0);
        assert_eq!(fade(f32::NAN, 90.0), 0.0);
        let mut ring = MarkRing::new(4);
        ring.add(print_at(0.0));
        assert!(ring.advance(89.0), "alive at 89 s");
        assert!(ring.get(0).unwrap().fade() < 0.04);
        assert!(!ring.advance(1.5), "gone past its life");
        assert_eq!(ring.live(), 0);
    }

    #[test]
    fn an_empty_slot_packs_as_zeros_so_it_is_not_drawn() {
        let mut ring = MarkRing::new(2);
        ring.add(print_at(5.0));
        let mut rows = vec![[9.0f32; 4]; 2 * ROW_FLOATS / 4];
        ring.pack_into(&mut rows);
        assert_eq!(rows[0][0], 5.0);
        assert_eq!(rows[2][2], 1.0, "fade, fresh");
        assert!(rows[ROW_FLOATS / 4..].iter().all(|r| *r == [0.0; 4]), "the empty slot");
    }

    #[test]
    fn a_footprint_lies_on_the_slope_and_points_where_the_mech_walks() {
        struct Slope;
        impl Ground for Slope {
            fn height_at(&self, x: f32, _z: f32) -> f32 {
                10.0 + 0.5 * x
            }
            fn push_out(&self, p: Vec2, _r: f32) -> Vec2 {
                p
            }
        }
        let c = cfg();
        let m = c.footprint(Surface::Dirt, Vec3::new(2.0, 0.0, 0.0), 0.0, false).on_ground(&Slope);
        assert_eq!(m.centre.y, 11.0, "the middle on the ground");
        let (across, along) = m.axes();
        assert!(along.distance(crate::mech::forward(0.0)) < 1e-6, "toe toward the heading");
        // The grid's corners rise and fall with the slope across x.
        let ext = m.extent_m.x * across.x;
        assert!((m.heights_m[GRID - 1] - 0.5 * ext).abs() < 1e-4, "{:?}", m.heights_m);
        assert!((m.heights_m[0] + 0.5 * ext).abs() < 1e-4);
        assert_eq!(m.heights_m[GRID * GRID / 2], 0.0, "the middle is the middle");
        assert!((m.extent_m.y - c.foot_half_length_m * QUAD_MARGIN).abs() < 1e-6);
        let landing = c.footprint(Surface::Dirt, Vec3::ZERO, 0.0, true);
        assert!(landing.depth_m > m.depth_m, "a landing presses deeper");
        let paved = c
            .footprint(Surface::Concrete, Vec3::ZERO, 0.0, false)
            .on_ground(&FlatGround::default());
        assert_eq!(paved.lip_m, 0.0, "no lip on paving");
    }

    #[test]
    fn the_mesh_is_a_five_by_five_grid_a_slot_and_nothing_else() {
        let (v, i) = grid_mesh(1024);
        assert_eq!(v.len(), 25_600, "25 vertices a slot");
        assert_eq!(i.len() / 3, 32_768, "32 triangles a slot");
        assert_eq!(v[25], [1.0, 0.0, 0.0], "the second slot starts at its own grid's corner");
        assert!(i.iter().all(|k| (*k as usize) < v.len()));
    }

    #[test]
    fn a_paving_print_is_deep_and_its_quad_reaches_past_the_sole() {
        let c = cfg();
        let paved = c.footprint(Surface::Concrete, Vec3::ZERO, 0.0, false);
        let soil = c.footprint(Surface::Dirt, Vec3::ZERO, 0.0, false);
        assert!((0.08..=0.12).contains(&paved.depth_m), "{}", paved.depth_m);
        assert!(paved.extent_m.min_element() >= 1.5 * c.foot_half_length_m, "{}", paved.extent_m);
        let sole = |m: &Mark| m.extent_m / m.shape_scale;
        assert!(sole(&paved).distance(sole(&soil)) < 1e-5, "the same sole");
        assert_eq!(soil.shape_scale, Vec2::ONE);
        let row = paved.pack();
        assert_eq!((row[37], row[38]), (paved.shape_scale.x, paved.shape_scale.y));
    }

    #[test]
    fn the_crush_version_moves_when_a_soil_mark_comes_or_goes() {
        let c = cfg();
        let mut ring = MarkRing::new(8);
        let v0 = ring.crush_version();
        ring.add(c.footprint(Surface::Dirt, Vec3::new(10.0, 0.0, 0.0), 0.0, false));
        let v1 = ring.crush_version();
        assert!(v1 > v0, "a soil print crushes");
        ring.add(c.footprint(Surface::Concrete, Vec3::ZERO, 0.0, false));
        assert_eq!(ring.crush_version(), v1, "a paving print does not");
        let rects = ring.crushes(c.crush_margin_m);
        assert_eq!(rects.len(), 1);
        let r = rects[0];
        assert!(
            r.contains(Vec2::new(10.0, 0.0)) && r.contains(Vec2::new(10.0, -1.4)),
            "the sole's length"
        );
        assert!(!r.contains(Vec2::new(10.0, 2.5)) && !r.contains(Vec2::new(11.5, 0.0)));
        ring.advance(c.foot_life_s + 1.0);
        assert!(ring.crush_version() > v1, "it faded out");
        assert!(ring.crushes(c.crush_margin_m).is_empty());
    }

    #[test]
    fn trail_segments_butt_end_to_end() {
        let c = cfg();
        let mut layer = TrailLayer::default();
        let key = (0, 0);
        assert_eq!(
            layer.feed(key, Vec2::ZERO, 0.0, c.trail_segment_m, c.trail_break_m),
            None,
            "a line starts"
        );
        assert_eq!(layer.feed(key, Vec2::new(0.0, 2.0), 0.1, c.trail_segment_m, c.trail_break_m), None);
        let (a, b) = layer
            .feed(key, Vec2::new(0.0, 4.5), 0.2, c.trail_segment_m, c.trail_break_m)
            .unwrap();
        assert_eq!((a, b), (Vec2::ZERO, Vec2::new(0.0, 4.5)));
        let (a2, _) = layer
            .feed(key, Vec2::new(0.0, 9.0), 0.3, c.trail_segment_m, c.trail_break_m)
            .unwrap();
        assert_eq!(a2, b, "the next segment starts where the last ended");
        let seg = c.trail(Surface::Dirt, Vec3::new(a.x, 0.0, a.y), Vec3::new(b.x, 0.0, b.y));
        assert!((seg.extent_m.y * 2.0 - 4.5).abs() < 1e-5, "a trail fills its quad's length");
        assert_eq!(seg.centre, Vec3::new(0.0, 0.0, 2.25));
        assert_eq!(
            layer.feed(key, Vec2::new(0.0, 60.0), 0.4, c.trail_segment_m, c.trail_break_m),
            None,
            "a jump"
        );
        layer.forget(2.0, 0.5);
        assert_eq!(layer.lines(), 0, "a wheel not fed is forgotten");
    }

    #[test]
    fn the_jolt_drops_fast_recovers_and_ends() {
        let j = JoltConfig::default();
        assert!((j.amplitude_m(0.0, false) - 0.01).abs() < 1e-6);
        assert!((j.amplitude_m(100.0, false) - 0.04).abs() < 1e-6, "capped");
        assert!((j.amplitude_m(0.0, true) - 0.07).abs() < 1e-6, "a landing");
        assert_eq!(jolt_profile(0.0), 0.0);
        assert!((jolt_profile(0.25) - 1.0).abs() < 1e-6, "deepest a quarter in");
        assert_eq!(jolt_profile(1.0), 0.0);
        let mut jolt = Jolt::default();
        jolt.kick(0.04, Vec3::X);
        jolt.advance(0.25 * j.duration_s, &j);
        let o = jolt.offset(&j);
        assert!((o.y + 0.04).abs() < 1e-4 && o.x > 0.0, "{o}");
        jolt.advance(j.duration_s, &j);
        assert!(!jolt.active());
        assert_eq!(jolt.offset(&j), Vec3::ZERO);
        jolt.kick(f32::NAN, Vec3::X);
        assert!(!jolt.active(), "a broken kick is refused");
    }

    #[test]
    fn the_shipped_file_loads_and_matches_the_defaults_it_documents() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fx/ground_marks.ron");
        let c: GroundMarksConfig = load_ron(&p).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.max_marks, 1024);
        GroundMarksConfig::default().validate().unwrap();
    }

    #[test]
    fn a_break_shorter_than_a_segment_and_an_unknown_key_are_refused() {
        let e = parse_ron::<GroundMarksConfig>("(trail_break_m: 2.0)", Path::new("data/fx/ground_marks.ron"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("trail_break_m") && e.contains("ground_marks.ron"), "{e}");
        let e = parse_ron::<GroundMarksConfig>("(jolt: (step_mm: 3.0))", Path::new("x.ron"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("step_mm"), "{e}");
    }
}
