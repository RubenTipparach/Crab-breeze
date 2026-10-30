//! Civil aircraft shapes (openspec change `civil-air-traffic`, design section 6): the block-out
//! stand-ins as a table of boxes, the hit capsules and the wreck's pieces, all from a kind's measured
//! size. One implementation: the game draws these boxes, `cb-tools aircraft` hands the same table to
//! the mockup page, the shots are swept against these capsules and the crash lays down these pieces.
//!
//! Local frame: x to the right wing, y up, z toward the tail (the nose looks along -z, as every unit
//! does), the origin on the fuselage's centre line at its middle.

use super::config::{AircraftSpec, Role};
use super::flight::{Aircraft, rest_m};
use crate::rng::SimRng;
use glam::{Quat, Vec3};

/// A part that turns while it flies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spin {
    /// About the up axis (a main rotor).
    Up,
    /// About the fore and aft axis (a propeller).
    Ahead,
}

/// One box of a block-out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// What it is ("fuselage", "wing", ...).
    pub name: &'static str,
    /// Centre, local metres.
    pub centre: Vec3,
    /// Size, local metres (x across, y up, z along).
    pub size: Vec3,
    /// Drawn dark (engines, rotors, propellers, skids) rather than in the livery.
    pub dark: bool,
    /// Turns while it flies.
    pub spin: Option<Spin>,
}

fn b(name: &'static str, centre: [f32; 3], size: [f32; 3], dark: bool) -> Block {
    Block {
        name,
        centre: Vec3::from_array(centre),
        size: Vec3::from_array(size),
        dark,
        spin: None,
    }
}

/// The block-out stand-in for a kind, at its true size.
pub fn blockout(spec: &AircraftSpec) -> Vec<Block> {
    let (l, s, h, f) = (spec.length_m, spec.span_m, spec.height_m, spec.fuselage_m);
    let top = h - rest_m(spec);
    // a military kind is drawn as its model; its stand-in is the civil shape nearest it
    let role = match spec.role {
        Role::Fighter => Role::Airliner,
        Role::Transport => Role::Freighter,
        Role::Gunship => Role::Helicopter,
        r => r,
    };
    match role {
        Role::Airliner => {
            let fin_h = top - f * 0.5;
            let pod = f * 0.45;
            vec![
                b("fuselage", [0.0, 0.0, 0.0], [f, f, l], false),
                b("wing", [0.0, -f * 0.3, l * 0.02], [s, f * 0.1, l * 0.16], false),
                b("engine L", [-s * 0.2, -f * 0.3 - pod * 0.75, -l * 0.04], [pod, pod, l * 0.11], true),
                b("engine R", [s * 0.2, -f * 0.3 - pod * 0.75, -l * 0.04], [pod, pod, l * 0.11], true),
                b("tailplane", [0.0, f * 0.1, l * 0.44], [s * 0.36, f * 0.08, l * 0.1], false),
                b("fin", [0.0, f * 0.5 + fin_h * 0.5, l * 0.42], [f * 0.09, fin_h, l * 0.13], false),
            ]
        }
        Role::Freighter => {
            let fin_h = top - f * 0.5;
            let pod = f * 0.3;
            let wing_y = f * 0.42;
            let mut v = vec![
                b("fuselage", [0.0, 0.0, 0.0], [f, f, l], false),
                b("wing", [0.0, wing_y, -l * 0.02], [s, f * 0.1, l * 0.13], false),
                b("fin", [0.0, f * 0.5 + fin_h * 0.5, l * 0.43], [f * 0.06, fin_h, l * 0.12], false),
                b("tailplane", [0.0, top - f * 0.05, l * 0.46], [s * 0.38, f * 0.08, l * 0.08], false),
            ];
            for (name, x) in [
                ("engine 1", -s * 0.32),
                ("engine 2", -s * 0.17),
                ("engine 3", s * 0.17),
                ("engine 4", s * 0.32),
            ] {
                v.push(b(name, [x, wing_y - pod * 0.85, -l * 0.06], [pod, pod, l * 0.08], true));
            }
            v
        }
        Role::LightPlane => {
            let prop = Block {
                spin: Some(Spin::Ahead),
                ..b("propeller", [0.0, 0.0, -l * 0.5], [s * 0.17, f * 0.12, f * 0.06], true)
            };
            vec![
                b("cabin", [0.0, 0.0, -l * 0.18], [f, f * 1.1, l * 0.62], false),
                b("tail cone", [0.0, f * 0.1, l * 0.3], [f * 0.45, f * 0.5, l * 0.38], false),
                b("wing", [0.0, f * 0.62, -l * 0.12], [s, f * 0.12, l * 0.18], false),
                b("tailplane", [0.0, f * 0.15, l * 0.44], [s * 0.3, f * 0.06, l * 0.1], false),
                b(
                    "fin",
                    [0.0, (f * 0.35 + top) * 0.5, l * 0.44],
                    [f * 0.06, top - f * 0.35, l * 0.12],
                    false,
                ),
                b(
                    "gear",
                    [0.0, -(rest_m(spec) + f * 0.5) * 0.5, -l * 0.15],
                    [f * 1.6, rest_m(spec) - f * 0.5, f * 0.15],
                    true,
                ),
                prop,
            ]
        }
        Role::Helicopter => {
            let rest = rest_m(spec);
            let cabin_top = f * 0.6;
            let bar = |name, size: [f32; 3]| Block {
                spin: Some(Spin::Up),
                ..b(name, [0.0, top, -l * 0.08], size, true)
            };
            vec![
                b("cabin", [0.0, 0.0, -l * 0.2], [f, f * 1.2, l * 0.32], false),
                b("tail boom", [0.0, f * 0.15, l * 0.2], [f * 0.22, f * 0.22, l * 0.45], false),
                b("fin", [0.0, f * 0.45, l * 0.42], [f * 0.08, f * 0.9, l * 0.07], false),
                b(
                    "mast",
                    [0.0, (cabin_top + top) * 0.5, -l * 0.08],
                    [f * 0.12, top - cabin_top, f * 0.12],
                    true,
                ),
                bar("rotor", [s, 0.08, f * 0.18]),
                bar("rotor", [f * 0.18, 0.08, s]),
                b("skid L", [-f * 0.55, -rest + 0.06, -l * 0.18], [0.12, 0.12, l * 0.35], true),
                b("skid R", [f * 0.55, -rest + 0.06, -l * 0.18], [0.12, 0.12, l * 0.35], true),
            ]
        }
        Role::Fighter | Role::Transport | Role::Gunship => Vec::new(),
    }
}

/// How it sits in the world: its heading, then its nose up, then its bank.
pub fn attitude(a: &Aircraft) -> Quat {
    Quat::from_rotation_y(a.heading_rad) * Quat::from_rotation_x(a.pitch_rad) * Quat::from_rotation_z(a.bank_rad)
}

/// How it sits in the world, its kind considered: on its wheels a taildragger sits nose up on its tail wheel (the
/// propliner's `rest_pitch_deg`); in the air, `attitude`.
pub fn attitude_for(a: &Aircraft, spec: &AircraftSpec) -> Quat {
    if a.on_ground() {
        let rest = super::flight::rest_pitch_rad(spec);
        if rest != 0.0 {
            return Quat::from_rotation_y(a.heading_rad) * Quat::from_rotation_x(a.pitch_rad + rest) * Quat::from_rotation_z(a.bank_rad);
        }
    }
    attitude(a)
}

/// A hit capsule: from `a` to `b`, `radius_m` round the line, world metres, on part `part`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    /// The part it belongs to: a modelled airliner's damage zone, else 0 (the aircraft as a whole).
    pub part: usize,
    /// One end.
    pub a: Vec3,
    /// The other end.
    pub b: Vec3,
    /// Its radius.
    pub radius_m: f32,
}

/// Its hit shapes: a modelled airliner's damage zones' capsules (measured off the model by its build), each on its
/// zone, the zones that have broken away left out; otherwise the fuselage, and the wing for a plane (a rotor is not
/// hit).
pub fn capsules(a: &Aircraft, spec: &AircraftSpec) -> Vec<Capsule> {
    let q = attitude_for(a, spec);
    let zones = spec.zones();
    if !zones.is_empty() {
        return zones
            .iter()
            .enumerate()
            .filter(|(i, _)| !a.broken.get(*i).copied().unwrap_or(false))
            .flat_map(|(i, z)| {
                z.capsules.iter().map(move |&(ca, cb, r)| Capsule {
                    part: i,
                    a: a.pos + q * ca,
                    b: a.pos + q * cb,
                    radius_m: r,
                })
            })
            .collect();
    }
    let r = spec.fuselage_m * 0.5;
    let half = (spec.length_m * 0.5 - r).max(0.0);
    let mut out = vec![Capsule {
        part: 0,
        a: a.pos + q * Vec3::new(0.0, 0.0, -half),
        b: a.pos + q * Vec3::new(0.0, 0.0, half),
        radius_m: if spec.role == Role::Helicopter { r * 1.2 } else { r },
    }];
    if spec.role != Role::Helicopter {
        let wing = blockout(spec).into_iter().find(|b| b.name == "wing");
        if let Some(w) = wing {
            let rw = (w.size.z * 0.3).max(0.3);
            let half_span = (w.size.x * 0.5 - rw).max(0.0);
            out.push(Capsule {
                part: 0,
                a: a.pos + q * (w.centre - Vec3::X * half_span),
                b: a.pos + q * (w.centre + Vec3::X * half_span),
                radius_m: rw,
            });
        }
    }
    out
}

/// The wreck a crash at `at` heading `heading_rad` lays down: at most `pieces` boxes (centre, size,
/// world metres), the fuselage broken into lengths and the wing into halves, strewn along the way it
/// was going. Seeded by the aircraft's id, so a replay strews them the same.
pub fn wreck(spec: &AircraftSpec, id: u32, at: Vec3, heading_rad: f32, pieces: u32) -> Vec<(Vec3, Vec3)> {
    let pieces = pieces.max(1) as usize;
    let fwd = crate::units::forward(heading_rad);
    let side = Vec3::new(-fwd.z, 0.0, fwd.x);
    // Boxes are laid along whichever world axis is nearer the heading.
    let along_x = fwd.x.abs() >= fwd.z.abs();
    let orient = |across: f32, up: f32, along: f32| {
        if along_x {
            Vec3::new(along, up, across)
        } else {
            Vec3::new(across, up, along)
        }
    };
    let mut rng = SimRng::new(u64::from(id)).fork("wreck");
    let wing_pieces = if spec.role == Role::Helicopter || pieces < 3 { 0 } else { 2 };
    let body = (pieces - wing_pieces).max(1);
    let (l, f) = (spec.length_m, spec.fuselage_m);
    let mut out = Vec::with_capacity(pieces);
    for i in 0..body {
        let t = (i as f32 + 0.5) / body as f32 - 0.5;
        let spread = 1.0 + rng.range_f32(0.2, 0.8);
        let c = at + fwd * (-t * l * spread) + side * rng.range_f32(-f, f) + Vec3::Y * f * 0.5;
        out.push((c, orient(f, f * 0.8, l / body as f32 * 0.9)));
    }
    for s in [-1.0f32, 1.0].into_iter().take(wing_pieces) {
        let half = spec.span_m * 0.4;
        let c = at + side * s * half * rng.range_f32(0.6, 1.2) + fwd * rng.range_f32(-l * 0.2, l * 0.2) + Vec3::Y * 0.5;
        out.push((c, orient(half, (f * 0.12).max(0.3), l * 0.12)));
    }
    out
}
