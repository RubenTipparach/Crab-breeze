//! Draws an air base's airfield (openspec change `airport-layout`; owner, 2026-10-28: "We should also improve airports.
//! They don't look much like airports", with the references in `docs/design/references/airports/`). Every footprint
//! comes from `cb_core::world::airfield::layout`; this file only turns it into block-out meshes:
//! - the runway's markings: blast pad chevrons, thresholds, designators, touchdown zone, aiming point;
//! - the parallel taxiway, its links, holding positions and yellow lines;
//! - the stands with their aircraft and boarding bridges;
//! - the terminal, fire station, fuel farm, car park, helicopter stand and glazed tower cab;
//! - the approach lights, PAPI, windsock and fence.
//!
//! Low poly first (owner rule, CLAUDE.md section 1): boxes and drums in vertex colours, lamps emissive and unshadowed
//! (survey N3).

use crate::world::MeshBuilder;
use bevy::prelude::*;
use cb_core::world::airfield::{AIMING_POINT_M, Airfield, BLAST_PAD_M, Kind, TAXIWAY_W_M, layout};
use cb_core::world::bases::{AirBase, BasesConfig, Runway};
use cb_core::world::layout::Rect;

/// White runway paint.
const WHITE: [f32; 4] = [0.92, 0.92, 0.9, 1.0];
/// Yellow taxiway paint.
const YELLOW: [f32; 4] = [0.95, 0.75, 0.15, 1.0];
/// Paint lies this far over the paving it is on, metres (no z-fighting).
const PAINT_M: f32 = 0.02;

/// The meshes an airfield is drawn into, one a material.
pub struct Meshes<'a> {
    /// Concrete: buildings, the tower, posts.
    pub slab: &'a mut MeshBuilder,
    /// Asphalt: the taxiway, links, entries, the car park.
    pub asphalt: &'a mut MeshBuilder,
    /// Paint in vertex colours.
    pub marks: &'a mut MeshBuilder,
    /// Glass: the terminal's windows and the tower's cab.
    pub glass: &'a mut MeshBuilder,
    /// White lamps: the approach lights and PAPI's white halves.
    pub lamps: &'a mut MeshBuilder,
    /// Red lamps: PAPI's red halves, the tower's mast light.
    pub red: &'a mut MeshBuilder,
}

/// Draws `base`'s airfield at its ground height `g` plus `lift` (the apron's top), the approach lights standing on
/// the natural ground (`height_at`). Returns the layout for whoever parks aircraft on its stands.
pub fn draw(base: &AirBase, cfg: &BasesConfig, lift: f32, height_at: impl Fn(Vec2) -> f32, mut m: Meshes) -> Airfield {
    let a = layout(base, cfg);
    let rw = &base.runway;
    let g = base.ground_m;
    let top = g + lift;
    let paint = top + PAINT_M;

    // Paving: the taxiway, its links and the apron entries, flush with the runway.
    for f in std::iter::once(&a.taxiway).chain(&a.links).chain(&a.entries) {
        m.asphalt.block(
            Vec3::new(f.min.x, g - 0.5, f.min.y),
            Vec3::new(f.max.x, top, f.max.y),
            [0.15, 0.15, 0.15, 1.0],
        );
    }

    runway_marks(rw, paint, &a, m.marks);

    // Yellow centre lines: down the taxiway, across each link and through each entry 60 m onto the apron.
    let len = rw.length_m();
    let w = rw.width_m * 0.5;
    let t_mid = w + rw.gap_m - 15.0 - TAXIWAY_W_M * 0.5;
    line(m.marks, rw.at(0.0, t_mid), rw.at(len, t_mid), 0.3, paint, YELLOW);
    for l in &a.links {
        let u = (l.centre() - rw.a).dot(rw.dir());
        line(m.marks, rw.at(u, w), rw.at(u, t_mid), 0.3, paint, YELLOW);
    }
    for e in &a.entries {
        let u = (e.centre() - rw.a).dot(rw.dir());
        line(m.marks, rw.at(u, t_mid), rw.at(u, w + rw.gap_m + 60.0), 0.3, paint, YELLOW);
    }
    // Holding positions: two solid lines on the taxiway side, two dashed toward the runway.
    for [p, q] in &a.holds {
        let toward = -rw.toward_site();
        for k in 0..2 {
            let o = rw.toward_site() * (1.0 + k as f32 * 0.9);
            line(m.marks, *p + o, *q + o, 0.15, paint, YELLOW);
        }
        let n = 8;
        for k in 0..2 {
            let o = toward * (0.6 + k as f32 * 0.9);
            for i in 0..n {
                if i % 2 == 0 {
                    let (s, e) = (p.lerp(*q, i as f32 / n as f32), p.lerp(*q, (i + 1) as f32 / n as f32));
                    line(m.marks, s + o, e + o, 0.15, paint, YELLOW);
                }
            }
        }
    }

    // Stands: yellow outlines, a lead-in line to the nose and a stop bar; boarding bridges on legs.
    for s in &a.stands {
        outline(m.marks, &s.rect, 0.3, paint, YELLOW);
        let c = s.rect.centre();
        let half = s.rect.size().x * 0.5;
        line(m.marks, c - s.nose * half, c + s.nose * (half - 4.0), 0.3, paint, YELLOW);
        let stop = c + s.nose * (half - 6.0);
        let across = s.nose.perp() * 4.0;
        line(m.marks, stop - across, stop + across, 0.4, paint, YELLOW);
        let b = &s.bridge;
        m.slab.block(
            Vec3::new(b.min.x, top + 4.0, b.min.y),
            Vec3::new(b.max.x, top + 7.0, b.max.y),
            [0.62, 0.64, 0.66, 1.0],
        );
        let foot = Vec2::new(b.max.x - 2.0, b.centre().y);
        m.slab.block(
            Vec3::new(foot.x - 0.4, top, foot.y - 1.2),
            Vec3::new(foot.x + 0.4, top + 4.0, foot.y + 1.2),
            [0.3, 0.3, 0.32, 1.0],
        );
    }

    for b in &a.buildings {
        building(b.kind, &b.rect, b.height_m, top, paint, &mut m);
    }

    tower(base.tower().centre(), g, &mut m);

    // The windsock: a mast and an orange and white sleeve along the runway.
    let ws = a.windsock;
    m.slab.drum(Vec3::new(ws.x, g, ws.y), 0.15, 0.1, 8.0, 6, [0.8, 0.8, 0.8, 1.0]);
    for k in 0..5 {
        let p = ws + rw.dir() * (0.4 + k as f32 * 0.6);
        let r = 0.5 - k as f32 * 0.06;
        let tone = if k % 2 == 0 { [0.95, 0.4, 0.05, 1.0] } else { WHITE };
        m.slab.block(
            Vec3::new(p.x - r, g + 7.4 - r, p.y - r),
            Vec3::new(p.x + r, g + 7.4 + r, p.y + r),
            tone,
        );
    }

    // PAPI: four boxes beside each aiming point, a white lamp over a red one on each.
    for end in &a.papi {
        for p in end {
            m.slab.block(
                Vec3::new(p.x - 0.8, top, p.y - 0.8),
                Vec3::new(p.x + 0.8, top + 1.0, p.y + 0.8),
                [0.2, 0.2, 0.2, 1.0],
            );
            m.lamps.block(
                Vec3::new(p.x - 0.5, top + 1.0, p.y - 0.5),
                Vec3::new(p.x + 0.5, top + 1.3, p.y + 0.5),
                WHITE,
            );
            m.red.block(
                Vec3::new(p.x - 0.5, top + 0.4, p.y - 0.85),
                Vec3::new(p.x + 0.5, top + 0.8, p.y - 0.8),
                WHITE,
            );
        }
    }

    // Approach lights: a post every `APPROACH_STEP_M` on the natural ground, a crossbar of five lamps at runway height.
    for posts in &a.approach {
        for (i, p) in posts.iter().enumerate() {
            let ground = height_at(*p);
            let lamp_y = top.max(ground) + 1.2;
            m.slab.block(
                Vec3::new(p.x - 0.15, ground.min(top) - 0.5, p.y - 0.15),
                Vec3::new(p.x + 0.15, lamp_y, p.y + 0.15),
                [0.85, 0.85, 0.85, 1.0],
            );
            // The crossbar widens toward the runway every third post.
            let half = if i % 3 == 2 { 9.0 } else { 6.0 };
            let across = rw.toward_site() * half;
            let (l, r) = (*p - across, *p + across);
            let (lo, hi) = (l.min(r) - Vec2::splat(0.1), l.max(r) + Vec2::splat(0.1));
            m.slab.block(
                Vec3::new(lo.x, lamp_y - 0.2, lo.y),
                Vec3::new(hi.x, lamp_y, hi.y),
                [0.85, 0.85, 0.85, 1.0],
            );
            for k in 0..5 {
                let q = l.lerp(r, k as f32 / 4.0);
                m.lamps.block(
                    Vec3::new(q.x - 0.3, lamp_y, q.y - 0.3),
                    Vec3::new(q.x + 0.3, lamp_y + 0.4, q.y + 0.3),
                    WHITE,
                );
            }
        }
    }

    fence(&a.fence, g, m.slab);
    a
}

/// The runway's paint: blast pad chevrons, thresholds, designators, touchdown zone, aiming point.
fn runway_marks(rw: &Runway, y: f32, a: &Airfield, marks: &mut MeshBuilder) {
    let len = rw.length_m();
    let w = rw.width_m * 0.5;
    // From each end, `along(end, d)` is d metres in from that end, and `side` is its right-hand way across.
    for end in 0..2 {
        let (from, d, right) = if end == 0 {
            (rw.a, rw.dir(), rw.dir().perp())
        } else {
            (rw.b, -rw.dir(), -rw.dir().perp())
        };
        let at = |along: f32, across: f32| from + d * along + right * across;
        // Blast pad chevrons, yellow, pointing to the threshold.
        for k in 0..3 {
            let tip = 15.0 + k as f32 * 15.0;
            line(marks, at(tip, 0.0), at(tip - 12.0, w - 2.0), 0.9, y, YELLOW);
            line(marks, at(tip, 0.0), at(tip - 12.0, -(w - 2.0)), 0.9, y, YELLOW);
        }
        // Threshold bars, eight across.
        for k in 0..8 {
            let off = (k as f32 - 3.5) * 4.6;
            line(marks, at(BLAST_PAD_M + 3.0, off), at(BLAST_PAD_M + 33.0, off), 0.9, y, WHITE);
        }
        // Designator: two digits 9 m tall, upright for a pilot landing from this end.
        let n = a.designators[end];
        let digits = [n / 10, n % 10];
        for (i, dg) in digits.iter().enumerate() {
            let left = if i == 0 { -6.5 } else { 1.5 };
            digit(*dg, |x0, z0, x1, z1| {
                line(
                    marks,
                    at(BLAST_PAD_M + 45.0 + z0, left + x0),
                    at(BLAST_PAD_M + 45.0 + z1, left + x1),
                    0.6,
                    y,
                    WHITE,
                )
            });
        }
        // Touchdown zone: 3, 3, 2, 1 bar pairs at 150, 300, 600 and 750 m past the threshold.
        for (dist, bars) in [(150.0f32, 3), (300.0, 3), (600.0, 2), (750.0, 1)] {
            if BLAST_PAD_M + dist + 22.5 > len * 0.5 {
                continue;
            }
            for k in 0..bars {
                let off = 4.5 + k as f32 * 3.0;
                for s in [-1.0f32, 1.0] {
                    line(
                        marks,
                        at(BLAST_PAD_M + dist, s * off),
                        at(BLAST_PAD_M + dist + 22.5, s * off),
                        0.9,
                        y,
                        WHITE,
                    );
                }
            }
        }
        // Aiming point: a broad bar either side of the centre line.
        for s in [-1.0f32, 1.0] {
            let off = s * 10.5;
            line(
                marks,
                at(BLAST_PAD_M + AIMING_POINT_M, off),
                at(BLAST_PAD_M + AIMING_POINT_M + 50.0, off),
                4.5,
                y,
                WHITE,
            );
        }
    }
    // Edge lines down both sides, and the dashed centre line between the designators.
    line(marks, rw.at(0.0, w - 1.0), rw.at(len, w - 1.0), 0.45, y, WHITE);
    line(marks, rw.at(0.0, -(w - 1.0)), rw.at(len, -(w - 1.0)), 0.45, y, WHITE);
    let mut at = BLAST_PAD_M + 70.0;
    while at + 30.0 <= len - BLAST_PAD_M - 70.0 {
        line(marks, rw.at(at, 0.0), rw.at(at + 30.0, 0.0), 0.45, y, WHITE);
        at += 50.0;
    }
}

/// A digit's strokes in a 5 x 9 m cell: `stroke(x0, z0, x1, z1)`, x across to the right, z along toward the far end.
fn digit(n: u32, mut stroke: impl FnMut(f32, f32, f32, f32)) {
    // Seven segments: top, top right, bottom right, bottom, bottom left, top left, middle (z 0 is the near end).
    let seg = [
        (0.0, 9.0, 5.0, 9.0),
        (5.0, 4.5, 5.0, 9.0),
        (5.0, 0.0, 5.0, 4.5),
        (0.0, 0.0, 5.0, 0.0),
        (0.0, 0.0, 0.0, 4.5),
        (0.0, 4.5, 0.0, 9.0),
        (0.0, 4.5, 5.0, 4.5),
    ];
    let on: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];
    for (i, s) in seg.iter().enumerate() {
        if on[n as usize % 10] & (1 << i) != 0 {
            stroke(s.0, s.1, s.2, s.3);
        }
    }
}

/// A flat painted stroke from `p` to `q`, `w` metres wide, at height `y`.
fn line(marks: &mut MeshBuilder, p: Vec2, q: Vec2, w: f32, y: f32, tone: [f32; 4]) {
    let d = (q - p).normalize_or(Vec2::X);
    // Square the ends off by half the width so strokes meet at corners.
    let (p, q) = (p - d * w * 0.5, q + d * w * 0.5);
    let s = d.perp() * w * 0.5;
    marks.quad(
        [
            Vec3::new(p.x + s.x, y, p.y + s.y),
            Vec3::new(q.x + s.x, y, q.y + s.y),
            Vec3::new(q.x - s.x, y, q.y - s.y),
            Vec3::new(p.x - s.x, y, p.y - s.y),
        ],
        tone,
    );
}

/// A rectangle's outline in paint, inside it.
fn outline(marks: &mut MeshBuilder, r: &Rect, w: f32, y: f32, tone: [f32; 4]) {
    let i = w * 0.5;
    let (a, b) = (r.min + Vec2::splat(i), r.max - Vec2::splat(i));
    let c = [a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)];
    for k in 0..4 {
        line(marks, c[k], c[(k + 1) % 4], w, y, tone);
    }
}

/// One building as a block-out.
fn building(kind: Kind, r: &Rect, h: f32, top: f32, paint: f32, m: &mut Meshes) {
    let (lo, hi) = (r.min, r.max);
    let block =
        |mb: &mut MeshBuilder, y0: f32, y1: f32, tone: [f32; 4]| mb.block(Vec3::new(lo.x, y0, lo.y), Vec3::new(hi.x, y1, hi.y), tone);
    match kind {
        Kind::Terminal => {
            let slab = &mut *m.slab;
            block(slab, top - 0.5, top + h, [0.78, 0.77, 0.74, 1.0]);
            // A roof that overhangs the face toward the stands, and a glazed band along that face.
            slab.block(
                Vec3::new(hi.x - 2.0, top + h, lo.y - 2.0),
                Vec3::new(hi.x + 4.0, top + h + 1.0, hi.y + 2.0),
                [0.5, 0.52, 0.54, 1.0],
            );
            let glass = &mut *m.glass;
            glass.quad(
                [
                    Vec3::new(hi.x + 0.05, top + 3.0, hi.y - 4.0),
                    Vec3::new(hi.x + 0.05, top + 3.0, lo.y + 4.0),
                    Vec3::new(hi.x + 0.05, top + h - 2.0, lo.y + 4.0),
                    Vec3::new(hi.x + 0.05, top + h - 2.0, hi.y - 4.0),
                ],
                WHITE,
            );
        }
        Kind::FireStation => {
            let slab = &mut *m.slab;
            block(slab, top - 0.5, top + h, [0.8, 0.78, 0.74, 1.0]);
            // Three red doors facing the apron (+x).
            let n = 3;
            for i in 0..n {
                let z0 = lo.y + 1.5 + i as f32 * (hi.y - lo.y - 3.0) / n as f32;
                let z1 = z0 + (hi.y - lo.y - 3.0) / n as f32 - 1.5;
                slab.quad(
                    [
                        Vec3::new(hi.x + 0.05, top, z1),
                        Vec3::new(hi.x + 0.05, top, z0),
                        Vec3::new(hi.x + 0.05, top + 6.0, z0),
                        Vec3::new(hi.x + 0.05, top + 6.0, z1),
                    ],
                    [0.75, 0.08, 0.06, 1.0],
                );
            }
        }
        Kind::FuelTank => {
            let c = r.centre();
            &mut *m.slab.drum(
                Vec3::new(c.x, top - 0.5, c.y),
                r.size().x * 0.5,
                r.size().x * 0.5,
                h + 0.5,
                20,
                [0.86, 0.86, 0.82, 1.0],
            );
        }
        Kind::CarPark => {
            let asphalt = &mut *m.asphalt;
            asphalt.block(
                Vec3::new(lo.x, top - 0.5, lo.y),
                Vec3::new(hi.x, top + 0.04, hi.y),
                [0.16, 0.16, 0.16, 1.0],
            );
            // Two rows of bays, 2.5 m apart, 5 m deep, either side of an aisle.
            let marks = &mut *m.marks;
            let y = paint + 0.04;
            let mid = (lo.y + hi.y) * 0.5;
            let mut x = lo.x + 2.0;
            while x <= hi.x - 2.0 {
                for (z0, z1) in [
                    (lo.y + 2.0, lo.y + 7.0),
                    (mid + 4.0, mid + 9.0),
                    (mid - 9.0, mid - 4.0),
                    (hi.y - 7.0, hi.y - 2.0),
                ] {
                    line(marks, Vec2::new(x, z0), Vec2::new(x, z1), 0.12, y, WHITE);
                }
                x += 2.5;
            }
        }
        Kind::Helipad => {
            let marks = &mut *m.marks;
            let c = r.centre();
            let rad = r.size().x * 0.5 - 0.5;
            let seg = 32;
            for i in 0..seg {
                let a0 = i as f32 / seg as f32 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / seg as f32 * std::f32::consts::TAU;
                let p = c + Vec2::new(a0.cos(), a0.sin()) * rad;
                let q = c + Vec2::new(a1.cos(), a1.sin()) * rad;
                line(marks, p, q, 0.6, paint, YELLOW);
            }
            // The H.
            for (x0, z0, x1, z1) in [(-3.0, -4.5, -3.0, 4.5), (3.0, -4.5, 3.0, 4.5), (-3.0, 0.0, 3.0, 0.0)] {
                line(marks, c + Vec2::new(x0, z0), c + Vec2::new(x1, z1), 1.0, paint, WHITE);
            }
        }
    }
}

/// The control tower: a shaft, a cab floor, a glazed cab, an overhanging roof and a mast with a red light.
fn tower(c: Vec2, g: f32, m: &mut Meshes) {
    let slab = &mut *m.slab;
    let tone = [0.72, 0.72, 0.7, 1.0];
    let roof = [0.5, 0.52, 0.54, 1.0];
    slab.block(Vec3::new(c.x - 6.0, g, c.y - 6.0), Vec3::new(c.x + 6.0, g + 36.0, c.y + 6.0), tone);
    slab.block(
        Vec3::new(c.x - 9.0, g + 36.0, c.y - 9.0),
        Vec3::new(c.x + 9.0, g + 37.0, c.y + 9.0),
        roof,
    );
    &mut *m.glass.block(
        Vec3::new(c.x - 8.0, g + 37.0, c.y - 8.0),
        Vec3::new(c.x + 8.0, g + 43.0, c.y + 8.0),
        WHITE,
    );
    slab.block(
        Vec3::new(c.x - 9.5, g + 43.0, c.y - 9.5),
        Vec3::new(c.x + 9.5, g + 44.0, c.y + 9.5),
        roof,
    );
    slab.block(
        Vec3::new(c.x - 0.2, g + 44.0, c.y - 0.2),
        Vec3::new(c.x + 0.2, g + 50.0, c.y + 0.2),
        tone,
    );
    &mut *m.red.block(
        Vec3::new(c.x - 0.4, g + 50.0, c.y - 0.4),
        Vec3::new(c.x + 0.4, g + 50.8, c.y + 0.4),
        WHITE,
    );
}

/// The fence: posts every 10 m and two rails along each side.
fn fence(poly: &[Vec2], g: f32, slab: &mut MeshBuilder) {
    let tone = [0.55, 0.57, 0.58, 1.0];
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let len = p.distance(q);
        let posts = (len / 10.0).ceil().max(1.0) as usize;
        for k in 0..posts {
            let s = p.lerp(q, k as f32 / posts as f32);
            slab.block(
                Vec3::new(s.x - 0.06, g, s.y - 0.06),
                Vec3::new(s.x + 0.06, g + 2.4, s.y + 0.06),
                tone,
            );
        }
        let (lo, hi) = (p.min(q) - Vec2::splat(0.03), p.max(q) + Vec2::splat(0.03));
        for y in [1.0, 2.3] {
            slab.block(Vec3::new(lo.x, g + y, lo.y), Vec3::new(hi.x, g + y + 0.06, hi.y), tone);
        }
    }
}
