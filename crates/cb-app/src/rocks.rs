//! Rock formations drawn (openspec change `rock-formations`; owner, 2026-10-25: "add stacks and
//! arches?"): the block-out stage of the rock kit, before the Blender kit with LODs and PBR maps
//! (the owner's rule: low poly first). Three kit pieces, built here as the island mockup builds
//! them (`buildRocks` in `docs/mockups/island.html`): a sea stack (a tapered column), a sea arch (a
//! half ring) and a hoodoo (a capped spire), each pushed in and out by the mockup's noise and
//! flat-shaded, banded with strata in vertex colours. Every rock of `cb_core::world::rocks` is an
//! entity sharing its kind's mesh and one of three shades of its material, so Bevy draws each kind
//! in a few instanced batches. Bodies walk round them by the core's rule (`rocks::push_out`).

use crate::state::GameApp;
use crate::world::Plan;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use cb_core::world::island::perlin;
use cb_core::world::rocks::{Rock, RockKind};
use std::f32::consts::PI;

/// Draws the rocks.
pub struct RocksPlugin;

impl Plugin for RocksPlugin {
    fn build(&self, app: &mut App) {
        app.add_world_spawn(spawn_rocks);
    }
}

/// The rock surfaces: limestone stacks and arches, red sandstone hoodoos (the mockup's colours,
/// sRGB), each banded with two other tones of strata.
fn strata_of(kind: RockKind) -> [[f32; 3]; 3] {
    match kind {
        RockKind::Stack | RockKind::Arch => [[0.74, 0.71, 0.64], [0.66, 0.63, 0.56], [0.80, 0.77, 0.70]],
        RockKind::Hoodoo => [[0.70, 0.38, 0.23], [0.62, 0.30, 0.18], [0.78, 0.52, 0.32]],
    }
}

/// Pushes each vertex in or out across the vertical by the mockup's noise (`displace`).
fn displace(points: &mut [Vec3], amount: f32, seed: f32) {
    for v in points {
        let n = perlin(f64::from(v.x * 2.1 + seed), f64::from(v.z * 2.1 + v.y * 1.3), 77) as f32 * amount;
        let r = Vec2::new(v.x, v.z).length().max(1e-4);
        v.x += v.x / r * n;
        v.z += v.z / r * n;
    }
}

/// A grid of `rows` x `cols` points (`at(row, col)`) as triangles, wrapping round the columns.
fn grid_triangles(rows: usize, cols: usize, closed: bool) -> Vec<[usize; 3]> {
    let mut tris = Vec::new();
    let span = if closed { cols } else { cols - 1 };
    for r in 0..rows - 1 {
        for c in 0..span {
            let c1 = (c + 1) % cols;
            let (a, b, d, e) = (r * cols + c, r * cols + c1, (r + 1) * cols + c, (r + 1) * cols + c1);
            tris.push([a, d, b]);
            tris.push([b, d, e]);
        }
    }
    tris
}

/// A sea stack: a column of radius 1 at the foot and 0.62 at the top, from y 0 to 1.
fn stack_points() -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let (rows, cols) = (9, 10);
    let mut pts = Vec::new();
    for r in 0..rows {
        let y = r as f32 / (rows - 1) as f32;
        let rad = 1.0 + (0.62 - 1.0) * y;
        for c in 0..cols {
            let a = c as f32 / cols as f32 * 2.0 * PI;
            pts.push(Vec3::new(a.cos() * rad, y, a.sin() * rad));
        }
    }
    let mut tris = grid_triangles(rows, cols, true);
    // The flat top.
    let top = pts.len();
    pts.push(Vec3::Y);
    for c in 0..cols {
        tris.push([top, (rows - 1) * cols + (c + 1) % cols, (rows - 1) * cols + c]);
    }
    (pts, tris)
}

/// A sea arch: half a ring of radius 1 and tube 0.3 standing in its x-y plane, its feet at x = +-1.
fn arch_points() -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let (rows, cols) = (19, 8);
    let mut pts = Vec::new();
    for r in 0..rows {
        let t = r as f32 / (rows - 1) as f32 * PI;
        for c in 0..cols {
            let p = c as f32 / cols as f32 * 2.0 * PI;
            let k = 1.0 + 0.3 * p.cos();
            pts.push(Vec3::new(t.cos() * k, t.sin() * k, 0.3 * p.sin()));
        }
    }
    (pts, grid_triangles(rows, cols, true))
}

/// A hoodoo: a spire turned on a lathe, waisted, with a wider cap stone, y 0 to 1.
fn hoodoo_points() -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let profile = [
        (0.001, 0.0),
        (0.42, 0.0),
        (0.34, 0.25),
        (0.25, 0.5),
        (0.3, 0.62),
        (0.2, 0.8),
        (0.55, 0.84),
        (0.6, 0.92),
        (0.3, 1.0),
        (0.001, 1.0),
    ];
    let cols = 9;
    let mut pts = Vec::new();
    for (rad, y) in profile {
        for c in 0..cols {
            let a = c as f32 / cols as f32 * 2.0 * PI;
            pts.push(Vec3::new(a.cos() * rad, y, a.sin() * rad));
        }
    }
    (pts, grid_triangles(profile.len(), cols, true))
}

/// A kit piece's mesh: displaced, flat-shaded (each triangle its own corners), banded with strata
/// by height in the piece's own frame.
fn kit_mesh(kind: RockKind) -> Mesh {
    let ((mut pts, tris), amount, seed) = match kind {
        RockKind::Stack => (stack_points(), 0.22, 1.0),
        RockKind::Arch => (arch_points(), 0.15, 2.0),
        RockKind::Hoodoo => (hoodoo_points(), 0.06, 3.0),
    };
    displace(&mut pts, amount, seed);
    let bands = strata_of(kind);
    let (mut pos, mut nor, mut col) = (Vec::new(), Vec::new(), Vec::new());
    for t in &tris {
        let [a, b, c] = t.map(|i| pts[i]);
        let n = (b - a).cross(c - a).normalize_or(Vec3::Y);
        for v in [a, b, c] {
            pos.push(v.to_array());
            nor.push(n.to_array());
            // Twelve bands up the piece, three tones, chosen by a stable hash of the band.
            let band = (v.y * 12.0).floor() as i32;
            let k = (band.wrapping_mul(2_654_435_761u32 as i32) as u32 >> 28) % 3;
            let s = bands[k as usize];
            col.push([s[0].powf(2.2), s[1].powf(2.2), s[2].powf(2.2), 1.0]);
        }
    }
    let n = pos.len() as u32;
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32((0..n).collect()))
}

/// The three shades a rock may take (its `shade` rounded to the nearest).
const SHADES: [f32; 3] = [0.9, 1.0, 1.1];

fn spawn_rocks(mut commands: Commands, plan: Res<Plan>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let kinds = [RockKind::Stack, RockKind::Arch, RockKind::Hoodoo];
    let kit: Vec<Handle<Mesh>> = kinds.iter().map(|k| meshes.add(kit_mesh(*k))).collect();
    let mats: Vec<Handle<StandardMaterial>> = SHADES
        .iter()
        .map(|s| {
            materials.add(StandardMaterial {
                base_color: Color::linear_rgb(*s, *s, *s),
                perceptual_roughness: 0.92,
                ..default()
            })
        })
        .collect();
    for (i, r) in plan.rocks.iter().enumerate() {
        let k = kinds.iter().position(|k| *k == r.kind).unwrap_or(0);
        let shade = SHADES
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - r.shade).abs().total_cmp(&(b.1 - r.shade).abs()))
            .map_or(1, |(j, _)| j);
        commands.spawn((
            Name::new(format!("rock {i} {:?}", r.kind)),
            Mesh3d(kit[k].clone()),
            MeshMaterial3d(mats[shade].clone()),
            transform_of(r),
        ));
    }
    info!("rocks: {} stacks, arches and hoodoos", plan.rocks.len());
}

/// Where a rock's kit piece stands: at its base, turned about the vertical, scaled.
fn transform_of(r: &Rock) -> Transform {
    Transform::from_translation(r.base)
        .with_rotation(Quat::from_rotation_y(r.yaw_rad))
        .with_scale(r.scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_piece_is_closed_flat_shaded_and_about_a_unit_across() {
        for kind in [RockKind::Stack, RockKind::Arch, RockKind::Hoodoo] {
            let m = kit_mesh(kind);
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else {
                panic!("positions")
            };
            assert_eq!(pos.len() % 3, 0, "{kind:?}: whole triangles");
            assert!(pos.len() > 100, "{kind:?}");
            let (lo, hi) = pos.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                (lo.min(Vec3::from(*p)), hi.max(Vec3::from(*p)))
            });
            assert!(lo.y > -0.4 && hi.y < 1.4 && hi.x < 1.6 && lo.x > -1.6, "{kind:?}: {lo} {hi}");
        }
    }

    #[test]
    fn an_arch_feet_stand_where_the_core_puts_them() {
        // The kit's arch has its feet at local x = +-1; the core's `feet` turns them by the same yaw.
        let r = Rock {
            kind: RockKind::Arch,
            base: Vec3::new(10.0, -6.0, 20.0),
            scale: Vec3::new(50.0, 40.0, 20.0),
            yaw_rad: 0.7,
            shade: 1.0,
        };
        let foot = transform_of(&r).transform_point(Vec3::X);
        let core = r.feet()[0].0;
        assert!(Vec2::new(foot.x, foot.z).distance(core) < 1e-3, "{foot} vs {core}");
    }
}
