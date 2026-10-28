//! The suburbs' streets, drives and car parks (openspec change `lot-access`), laid on the ground as each ground node
//! draws it. Past 700 m a node draws the ground on coarse cells that cut under the tops of dunes, and paving draped
//! on the full detail hovered over it, by up to 2.7 m at step 2 and 4.3 m at step 4 (owner, 2026-10-28: "suburb
//! roads are hovering"). So each node lays the paving inside it on the ground at its own step
//! (`Heightfield::height_at_step`), built with its mesh and shown and hidden with it (`ground`).

use crate::world::MeshBuilder;
use bevy::prelude::*;
use cb_core::world::small_buildings::paving::Surface;
use cb_core::world::{Rect, WorldPlan};
use std::collections::HashMap;

/// Paving is draped on the ground in cells no longer than these: the terrain's lattice is 8 m, so a street follows
/// it in 4 m pieces, a drive or a car park in 2 m ones.
const STREET_CELL_M: f32 = 4.0;
const SLAB_CELL_M: f32 = 2.0;

/// How the paving's meshes are laid out: its texture tiles and colours (plain data, for the worker threads).
#[derive(Clone, Copy, Debug)]
pub struct PavingStyle {
    /// The suburbs' asphalt tile (Asphalt026C).
    pub asphalt_tile_m: f32,
    /// The drives' concrete tile (the kerbs' paving set).
    pub concrete_tile_m: f32,
    /// The suburbs' asphalt tone.
    pub asphalt: [f32; 4],
}

/// What paving is drawn with, made with the roads (`world::spawn_roads`): its style and a material per surface, in
/// the order of `Surface`.
#[derive(Resource, Clone)]
pub struct PavingLook {
    /// How its meshes are laid out.
    pub style: PavingStyle,
    /// Asphalt, concrete and paint.
    pub materials: [Handle<StandardMaterial>; 3],
}

/// The paving of one ground node: a mesh per surface (asphalt, concrete, paint), `None` where it has none.
pub type NodePaving = [Option<Mesh>; 3];

/// The paving whose pieces lie in `rect` (each piece of a strip goes to the node holding its middle), on the ground
/// as a node drawn at `step` shows it.
pub fn node_paving(plan: &WorldPlan, style: &PavingStyle, rect: Rect, step: usize) -> NodePaving {
    let t = &plan.terrain;
    // Past step 1 the lattice is worked out afresh at every read: keep each corner once.
    let mut corners: HashMap<(i64, i64), f32> = HashMap::new();
    let mut ground = |p: Vec2| {
        if step <= 1 {
            t.height_at(p.x, p.y)
        } else {
            t.height_at_step_with(p.x, p.y, step, &mut |ix, iz| {
                *corners.entry((ix, iz)).or_insert_with(|| t.lattice_at_step(ix, iz, step))
            })
        }
    };
    let mut asphalt = MeshBuilder::tiled(style.asphalt_tile_m, style.asphalt_tile_m);
    let mut concrete = MeshBuilder::tiled(style.concrete_tile_m, style.concrete_tile_m);
    let mut paint = MeshBuilder::default();
    let concrete_tone = [0.74, 0.73, 0.70, 1.0];
    let white = [0.92, 0.92, 0.88, 1.0];
    let paving = &plan.small.paving;
    for st in &paving.streets {
        let n = st.path.len();
        // Each point's side is the mean of its two segments', so the pieces of a bend share their edges.
        let side = |i: usize| {
            let a = st.path[i.saturating_sub(1)];
            let b = st.path[(i + 1).min(n - 1)];
            (b - a).normalize_or(Vec2::X).perp() * st.width_m * 0.5
        };
        for i in 0..n.saturating_sub(1) {
            let (a, b, sa, sb) = (st.path[i], st.path[i + 1], side(i), side(i + 1));
            let c = [a - sa, a + sa, b + sb, b - sb];
            if touches(&c, &rect) {
                drape(&mut asphalt, &mut ground, &rect, c, [st.lift_m; 4], style.asphalt, STREET_CELL_M);
            }
        }
    }
    for slab in &paving.slabs {
        if !touches(&slab.corners, &rect) {
            continue;
        }
        let (into, colour) = match slab.surface {
            Surface::Asphalt => (&mut asphalt, style.asphalt),
            Surface::Concrete => (&mut concrete, concrete_tone),
            Surface::Paint => (&mut paint, white),
        };
        drape(into, &mut ground, &rect, slab.corners, slab.lift_m, colour, SLAB_CELL_M);
    }
    let built = |b: MeshBuilder, tangents: bool| (!b.is_empty()).then(|| if tangents { b.build_with_tangents() } else { b.build() });
    [built(asphalt, true), built(concrete, true), built(paint, false)]
}

/// Whether the box round corners `c` meets `rect`.
fn touches(c: &[Vec2; 4], rect: &Rect) -> bool {
    let lo = c.iter().fold(Vec2::splat(f32::MAX), |a, p| a.min(*p));
    let hi = c.iter().fold(Vec2::splat(f32::MIN), |a, p| a.max(*p));
    lo.x < rect.max.x && hi.x >= rect.min.x && lo.y < rect.max.y && hi.y >= rect.min.y
}

/// A four-cornered piece of paving draped on the ground (openspec change `lot-access`): cut into cells no longer than
/// `cell_m` so it follows `ground` between its corners, each point the ground plus its corners' lift, blended. Only
/// the cells whose middle lies in `keep` (min inclusive, max exclusive) are laid, so a piece crossing nodes is laid
/// once.
fn drape(
    b: &mut MeshBuilder,
    ground: &mut dyn FnMut(Vec2) -> f32,
    keep: &Rect,
    c: [Vec2; 4],
    lift: [f32; 4],
    colour: [f32; 4],
    cell_m: f32,
) {
    let cells = |a: f32, b: f32| (a.max(b) / cell_m).ceil().clamp(1.0, 256.0) as usize;
    let nu = cells((c[1] - c[0]).length(), (c[2] - c[3]).length());
    let nv = cells((c[3] - c[0]).length(), (c[2] - c[1]).length());
    let xz = |i: usize, j: usize| {
        let (u, v) = (i as f32 / nu as f32, j as f32 / nv as f32);
        c[0].lerp(c[1], u).lerp(c[3].lerp(c[2], u), v)
    };
    let inside = |p: Vec2| p.x >= keep.min.x && p.x < keep.max.x && p.y >= keep.min.y && p.y < keep.max.y;
    // Heights only for the points of cells kept.
    let mut points: HashMap<(usize, usize), Vec3> = HashMap::new();
    let mut at = |i: usize, j: usize| {
        *points.entry((i, j)).or_insert_with(|| {
            let (u, v) = (i as f32 / nu as f32, j as f32 / nv as f32);
            let p = xz(i, j);
            let l = (lift[0] + (lift[1] - lift[0]) * u) * (1.0 - v) + (lift[3] + (lift[2] - lift[3]) * u) * v;
            Vec3::new(p.x, ground(p) + l, p.y)
        })
    };
    for i in 0..nu {
        for j in 0..nv {
            if !inside((xz(i, j) + xz(i + 1, j + 1)) * 0.5) {
                continue;
            }
            let q = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)];
            // `quad` faces its normal (c1 - c0) x (c3 - c0): turn the corners so it looks up.
            if (q[1] - q[0]).cross(q[3] - q[0]).y >= 0.0 {
                b.quad(q, colour);
            } else {
                b.quad([q[0], q[3], q[2], q[1]], colour);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_crossing_two_nodes_is_laid_once_between_them() {
        let piece = [
            Vec2::new(-6.0, -2.0),
            Vec2::new(6.0, -2.0),
            Vec2::new(6.0, 2.0),
            Vec2::new(-6.0, 2.0),
        ];
        let mut flat = |_: Vec2| 0.0;
        let halves = [
            Rect {
                min: Vec2::new(-100.0, -100.0),
                max: Vec2::new(0.0, 100.0),
            },
            Rect {
                min: Vec2::new(0.0, -100.0),
                max: Vec2::new(100.0, 100.0),
            },
        ];
        let laid: Vec<usize> = halves
            .iter()
            .map(|r| {
                let mut b = MeshBuilder::default();
                drape(&mut b, &mut flat, r, piece, [0.1; 4], [1.0; 4], 2.0);
                b.build().indices().map_or(0, |i| i.len()) / 6
            })
            .collect();
        assert_eq!(laid, vec![3 * 2, 3 * 2], "six 2 m cells by two, half on each side");
    }

    #[test]
    fn paving_rides_on_the_ground_it_is_given() {
        let piece = [Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0), Vec2::new(4.0, 4.0), Vec2::new(0.0, 4.0)];
        let mut slope = |p: Vec2| p.x * 0.5;
        let keep = Rect {
            min: Vec2::splat(-10.0),
            max: Vec2::splat(10.0),
        };
        let mut b = MeshBuilder::default();
        drape(&mut b, &mut slope, &keep, piece, [0.2; 4], [1.0; 4], 2.0);
        let mesh = b.build();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("positions");
        };
        assert!(!positions.is_empty());
        for p in positions {
            assert!((p[1] - (p[0] * 0.5 + 0.2)).abs() < 1e-5, "{p:?} on the ground plus its lift");
        }
    }
}
