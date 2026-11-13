//! Draws the roads over roads (openspec change `city-districts`, design section 5): each viaduct's slab under the
//! highway's own surface (its fascia, its underside, its barriers), each exit's ramps (concrete where they stand over the
//! ground, the streets' own asphalt where they land, a strip of the concrete fading out over the asphalt between, so
//! the two meet with no line), and the piers. Every structure is cut into 512 m tiles at three levels of detail, chosen
//! each frame by the eye's distance to the tile ([`DeckLod`]): barriers and pier caps near, the slab's sides and plain
//! piers further, nothing past that but the road's surface (drawn with the roads). The tiles keep their vertices in
//! world space (the crater cut and the seamless mapping need them there), so Bevy's own visibility ranges, which
//! measure from the entity's origin, cannot choose them. Surfaces are mapped in world space, as every road is, so a ramp's
//! asphalt runs on into its street and its concrete onto the viaduct's without a seam. The decks themselves are
//! `cb_core::world::decks`.

use crate::world::MeshBuilder;
use bevy::prelude::*;
use cb_core::world::WorldPlan;
use cb_core::world::decks::{DeckKind, RoadDeck};
use std::collections::BTreeMap;

/// The tiles the structures are cut into, metres.
const TILE_M: f32 = 512.0;
/// The detail (barriers, pier caps) draws while the eye is within this of the tile's nearest point, metres.
const NEAR_M: f32 = 1300.0;
/// The slab's sides and plain piers draw to here.
const FAR_M: f32 = 4000.0;
/// A barrier: its height and its width, metres.
const BARRIER_M: Vec2 = Vec2::new(1.1, 0.45);
/// The ramps' concrete fades out over the asphalt over this length, metres.
const BLEND_M: f32 = 10.0;

/// The meshes of one tile: detail (near), plain (middle), and the blend strips (near and middle), each mapped in world
/// space at the concrete's tile.
struct Tile {
    near: MeshBuilder,
    mid: MeshBuilder,
    blend: MeshBuilder,
}

/// The tiles, by key, made on first use with the concrete's tile size.
struct Tiles {
    tile_m: f32,
    by_key: BTreeMap<(i32, i32), Tile>,
}

impl Tiles {
    fn at(&mut self, p: Vec3) -> &mut Tile {
        let t = self.tile_m;
        self.by_key.entry(tile_of(p)).or_insert_with(|| Tile {
            near: MeshBuilder::tiled(t, t),
            mid: MeshBuilder::tiled(t, t),
            blend: MeshBuilder::tiled(t, t),
        })
    }
}

fn tile_of(p: Vec3) -> (i32, i32) {
    ((p.x / TILE_M).floor() as i32, (p.z / TILE_M).floor() as i32)
}

/// A point along `pts` (x, top, z) at arc length `s`, and the direction there (x, z).
fn along(pts: &[Vec3], s: f32) -> (Vec3, Vec2) {
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let d = Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z);
        let len = d.length();
        if len > 1e-6 && s <= acc + len {
            let t = ((s - acc) / len).clamp(0.0, 1.0);
            return (w[0].lerp(w[1], t), d / len);
        }
        acc += len;
    }
    let n = pts.len();
    let d = Vec2::new(pts[n - 1].x - pts[n - 2].x, pts[n - 1].z - pts[n - 2].z).normalize_or(Vec2::X);
    (pts[n - 1], d)
}

fn length(pts: &[Vec3]) -> f32 {
    pts.windows(2).map(|w| Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length()).sum()
}

/// A quad whose outward normal points the way of `out`: corners turned round if they face away.
fn face(b: &mut MeshBuilder, mut q: [Vec3; 4], out: Vec3, colour: [f32; 4]) {
    if (q[1] - q[0]).cross(q[3] - q[0]).dot(out) < 0.0 {
        q.reverse();
    }
    b.quad(q, colour);
}

/// A box with its centre `c`, half sizes `h` (across, up, along) turned so `along` (x, z) is its length.
fn obox(b: &mut MeshBuilder, c: Vec3, h: Vec3, along: Vec2, colour: [f32; 4]) {
    let ax = Vec3::new(-along.y, 0.0, along.x) * h.x;
    let ay = Vec3::Y * h.y;
    let az = Vec3::new(along.x, 0.0, along.y) * h.z;
    let p = |sx: f32, sy: f32, sz: f32| c + ax * sx + ay * sy + az * sz;
    for (n, q) in [
        (ay, [p(-1., 1., -1.), p(1., 1., -1.), p(1., 1., 1.), p(-1., 1., 1.)]),
        (-ay, [p(-1., -1., -1.), p(1., -1., -1.), p(1., -1., 1.), p(-1., -1., 1.)]),
        (ax, [p(1., -1., -1.), p(1., 1., -1.), p(1., 1., 1.), p(1., -1., 1.)]),
        (-ax, [p(-1., -1., -1.), p(-1., 1., -1.), p(-1., 1., 1.), p(-1., -1., 1.)]),
        (az, [p(-1., -1., 1.), p(1., -1., 1.), p(1., 1., 1.), p(-1., 1., 1.)]),
        (-az, [p(-1., -1., -1.), p(1., -1., -1.), p(1., 1., -1.), p(-1., 1., -1.)]),
    ] {
        face(b, q, n, colour);
    }
}

/// The slab under a deck's top, both sides and its underside (to the ground where it stands on fill), and its
/// barriers where the edge stands over the ground and no ramp leaves it, every `step_m`, into the tiles.
fn slab(tiles: &mut Tiles, d: &RoadDeck, ground: &dyn Fn(Vec2) -> f32, open: &dyn Fn(Vec3) -> bool, colour: [f32; 4]) {
    let total = length(&d.pts);
    let step = 4.0f32;
    let n = (total / step).ceil().max(1.0) as usize;
    let half = d.width_m * 0.5;
    for i in 0..n {
        let (s0, s1) = (total * i as f32 / n as f32, total * (i + 1) as f32 / n as f32);
        let ((p0, d0), (p1, d1)) = (along(&d.pts, s0), along(&d.pts, s1));
        let (r0, r1) = (Vec3::new(-d0.y, 0.0, d0.x), Vec3::new(-d1.y, 0.0, d1.x));
        let tile = tiles.at(p0);
        let g = |p: Vec3| ground(Vec2::new(p.x, p.z));
        for side in [-1.0f32, 1.0] {
            let (e0, e1) = (p0 + r0 * half * side, p1 + r1 * half * side);
            let (b0, b1) = (d.underside_m(e0.y, g(e0)), d.underside_m(e1.y, g(e1)));
            // the slab's side: its fascia, or a wall down to the ground on fill
            let q = [Vec3::new(e0.x, b0, e0.z), Vec3::new(e1.x, b1, e1.z), e1, e0];
            face(&mut tile.mid, q, r0 * side, colour);
            // a barrier on the edge, where it stands a metre over the ground and no ramp leaves here
            if e0.y - g(e0) > 1.0 && !open(e0) && !open(e1) {
                // swept along the edge (outer face, top, inner face), each corner shared with the next piece, so the
                // barrier follows the deck's grade with no step and no gap
                let up = Vec3::Y * BARRIER_M.x;
                let (i0, i1) = (e0 - r0 * side * BARRIER_M.y, e1 - r1 * side * BARRIER_M.y);
                face(&mut tile.near, [e0, e1, e1 + up, e0 + up], r0 * side, colour);
                face(&mut tile.near, [e0 + up, e1 + up, i1 + up, i0 + up], Vec3::Y, colour);
                face(&mut tile.near, [i0, i1, i1 + up, i0 + up], -r0 * side, colour);
            }
        }
        // the underside, where the slab stands on piers
        let (a0, a1) = (p0 - r0 * half, p1 - r1 * half);
        let (c0, c1) = (p0 + r0 * half, p1 + r1 * half);
        let (u0, u1) = (d.underside_m(p0.y, g(p0)), d.underside_m(p1.y, g(p1)));
        if u0 > g(p0) + 0.5 {
            let y = |p: Vec3, u: f32| Vec3::new(p.x, u, p.z);
            face(&mut tile.mid, [y(a0, u0), y(a1, u1), y(c1, u1), y(c0, u0)], Vec3::NEG_Y, colour);
        }
    }
}

/// Spawns every deck's structure and piers, by tile, with their levels of detail, and adds each ramp's road surface to
/// the roads' meshes: `concrete` (the highways') where it stands over the ground, `asphalt` (the streets') where it
/// lands, a strip of concrete fading out over the asphalt between (drawn into the tiles' blend meshes).
#[allow(clippy::too_many_arguments)]
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    plan: &WorldPlan,
    concrete: &mut MeshBuilder,
    asphalt: &mut MeshBuilder,
    tones: ([f32; 3], [f32; 3]),
    structure_mat: Handle<StandardMaterial>,
    blend_mat: Handle<StandardMaterial>,
    tile_m: f32,
) {
    let ground = |p: Vec2| plan.terrain.height_at(p.x, p.y);
    let grey = [0.78, 0.77, 0.74, 1.0];
    let mut tiles = Tiles {
        tile_m,
        by_key: BTreeMap::new(),
    };
    // where a ramp leaves or joins a viaduct, the viaduct's barrier opens for it
    let open = |p: Vec3| {
        plan.ramps.iter().any(|r| {
            r.line.windows(2).any(|w| {
                let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
                let ab = b - a;
                let t = ((Vec2::new(p.x, p.z) - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
                (a + ab * t).distance(Vec2::new(p.x, p.z)) < r.width_m * 0.5 + 1.5 && (w[0].y + (w[1].y - w[0].y) * t - p.y).abs() < 1.0
            })
        })
    };
    for d in &plan.decks.decks {
        match d.kind {
            DeckKind::Viaduct { .. } => slab(&mut tiles, d, &ground, &open, grey),
            DeckKind::Ramp { ramp } => {
                // the ramp's own barriers stay closed (it leaves the viaduct, the viaduct's barrier opens)
                slab(&mut tiles, d, &ground, &|_| false, grey);
                let r = &plan.ramps[ramp as usize];
                ramp_surface(
                    &mut tiles,
                    d,
                    r,
                    plan.cities[r.city as usize].ground_m + plan.road_lift_m,
                    concrete,
                    asphalt,
                    tones,
                );
            }
        }
        for p in &d.piers {
            let g = ground(p.at);
            let dir = Vec2::new(p.heading_rad.sin(), p.heading_rad.cos());
            let along = Vec2::new(-dir.y, dir.x);
            let tile = tiles.at(Vec3::new(p.at.x, 0.0, p.at.y));
            let h = p.top_m - g;
            // near: a column and a cap beam under the slab
            let cap = 1.2f32.min(h * 0.3);
            let column = Vec3::new(p.size_m.x * 0.32, (h - cap) * 0.5, p.size_m.y * 0.45);
            obox(&mut tile.near, Vec3::new(p.at.x, g + (h - cap) * 0.5, p.at.y), column, along, grey);
            obox(
                &mut tile.near,
                Vec3::new(p.at.x, p.top_m - cap * 0.5, p.at.y),
                Vec3::new(p.size_m.x * 0.5, cap * 0.5, p.size_m.y * 0.5),
                along,
                grey,
            );
            // further: one plain box
            obox(
                &mut tile.mid,
                Vec3::new(p.at.x, g + h * 0.5, p.at.y),
                Vec3::new(p.size_m.x * 0.32, h * 0.5, p.size_m.y * 0.45),
                along,
                grey,
            );
        }
    }
    let (mut tris, mut count) = (0, 0);
    for (key, t) in tiles.by_key {
        let lo = Vec2::new(key.0 as f32, key.1 as f32) * TILE_M;
        for (name, b, to_m, mat) in [
            ("deck detail", t.near, NEAR_M, structure_mat.clone()),
            ("deck structure", t.mid, FAR_M, structure_mat.clone()),
            ("ramp blend", t.blend, FAR_M, blend_mat.clone()),
        ] {
            if b.is_empty() {
                continue;
            }
            count += 1;
            let mesh = b.build_with_tangents();
            tris += mesh.indices().map_or(0, |i| i.len() / 3);
            commands.spawn((
                Name::new(name),
                Mesh3d(meshes.add(crate::world::kept_for_cutting(mesh))),
                MeshMaterial3d(mat),
                crate::world::CutByCraters,
                DeckLod {
                    min: lo,
                    max: lo + Vec2::splat(TILE_M),
                    to_m,
                },
                Visibility::Hidden,
            ));
        }
    }
    info!(
        "decks: {} decks, {} ramps, {count} tile meshes, {tris} triangles",
        plan.decks.decks.len(),
        plan.ramps.len()
    );
}

/// One level of a tile: drawn while the eye is within `to_m` of the tile's square (`min`, `max`, x and z), metres.
#[derive(Component, Clone, Copy, Debug)]
pub struct DeckLod {
    min: Vec2,
    max: Vec2,
    to_m: f32,
}

impl DeckLod {
    /// Whether the level draws with the eye at `eye` (x and z, metres): within `to_m` of the tile's square.
    fn shows(&self, eye: Vec2) -> bool {
        eye.clamp(self.min, self.max).distance_squared(eye) <= self.to_m * self.to_m
    }
}

/// Chooses the decks' levels of detail from the pilot's eye each frame.
pub struct DeckLodPlugin;

impl Plugin for DeckLodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, choose_levels);
    }
}

/// Shows each tile's level while the eye is within its range, hides it past that; touches only the ones that change.
fn choose_levels(eye: Option<Single<&GlobalTransform, With<crate::pilot::PilotCamera>>>, mut tiles: Query<(&DeckLod, &mut Visibility)>) {
    let Some(eye) = eye else { return };
    let at = eye.translation();
    let at = Vec2::new(at.x, at.z);
    for (lod, mut vis) in &mut tiles {
        let want = if lod.shows(at) { Visibility::Inherited } else { Visibility::Hidden };
        vis.set_if_neq(want);
    }
}

/// A ramp's road surface: concrete into `concrete` over the ground, asphalt into `asphalt` from `BLEND_M` before it
/// lands (its top at `street_top`), and a strip of the concrete fading out over the asphalt between, a centimetre over
/// it (into the tile's blend mesh, drawn with an alpha-blended concrete).
#[allow(clippy::too_many_arguments)]
fn ramp_surface(
    tiles: &mut Tiles,
    d: &RoadDeck,
    r: &cb_core::world::trunk::Ramp,
    street_top: f32,
    concrete: &mut MeshBuilder,
    asphalt: &mut MeshBuilder,
    (concrete_tone, asphalt_tone): ([f32; 3], [f32; 3]),
) {
    let total = length(&d.pts);
    let step = 2.0f32;
    let n = (total / step).ceil().max(1.0) as usize;
    let half = d.width_m * 0.5;
    // where it reaches the street's level, along its deck from the viaduct end
    let mut land = total;
    let from_high = r.off;
    for i in 0..=n {
        let s = if from_high {
            total * i as f32 / n as f32
        } else {
            total * (n - i) as f32 / n as f32
        };
        if along(&d.pts, s).0.y <= street_top + 0.05 {
            land = s;
            break;
        }
    }
    let rgba = |t: [f32; 3], a: f32| [t[0], t[1], t[2], a];
    for i in 0..n {
        let (s0, s1) = (total * i as f32 / n as f32, total * (i + 1) as f32 / n as f32);
        let ((p0, d0), (p1, d1)) = (along(&d.pts, s0), along(&d.pts, s1));
        let (r0, r1) = (Vec3::new(-d0.y, 0.0, d0.x), Vec3::new(-d1.y, 0.0, d1.x));
        let q = [p0 - r0 * half, p1 - r1 * half, p1 + r1 * half, p0 + r0 * half];
        // distance of this piece from the landing, toward the high end (positive) or the street (negative)
        let mid = (s0 + s1) * 0.5;
        let up = if from_high { land - mid } else { mid - land };
        if up > BLEND_M * 0.5 {
            face(concrete, q, Vec3::Y, rgba(concrete_tone, 1.0));
        } else {
            face(asphalt, q, Vec3::Y, rgba(asphalt_tone, 1.0));
            if up > -BLEND_M * 0.5 {
                // the concrete fading out over the asphalt: full where it meets the concrete, gone toward the street
                let a = |s: f32| {
                    let u = if from_high { land - s } else { s - land };
                    ((u + BLEND_M * 0.5) / BLEND_M).clamp(0.0, 1.0)
                };
                let lift = Vec3::Y * 0.012;
                let tile = tiles.at(p0);
                let mut q2 = [q[0] + lift, q[1] + lift, q[2] + lift, q[3] + lift];
                let mut cols = [
                    rgba(concrete_tone, a(s0)),
                    rgba(concrete_tone, a(s1)),
                    rgba(concrete_tone, a(s1)),
                    rgba(concrete_tone, a(s0)),
                ];
                if (q2[1] - q2[0]).cross(q2[3] - q2[0]).y < 0.0 {
                    q2.reverse();
                    cols.reverse();
                }
                tile.blend.quad_shaded(q2, cols);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_shows_within_its_range_of_the_tiles_nearest_point_and_hides_past_it() {
        let lod = DeckLod {
            min: Vec2::ZERO,
            max: Vec2::splat(TILE_M),
            to_m: NEAR_M,
        };
        assert!(lod.shows(Vec2::splat(100.0)), "inside the tile");
        assert!(
            lod.shows(Vec2::new(TILE_M + NEAR_M - 1.0, 200.0)),
            "just inside the range beside it"
        );
        assert!(!lod.shows(Vec2::new(TILE_M + NEAR_M + 1.0, 200.0)), "just past the range");
        assert!(!lod.shows(Vec2::new(-NEAR_M, -NEAR_M)), "past the range off a corner");
    }
}
