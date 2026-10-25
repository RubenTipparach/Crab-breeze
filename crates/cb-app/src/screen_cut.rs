//! The screen wall's openings cut into the cabin (openspec change `mech-roster` section 2). An interior from
//! `tools/blender/build_cockpit.py` shows the outside on `SCREEN_allview` quads with a back plate 2 cm behind each
//! and the pod's inner wall behind that, so the cabin is a closed box: hiding the quads from the seat leaves the
//! plates, and the pilot sees black where the world should be. This removes the cabin geometry that lies behind a
//! window inside the pyramid the pilot's eye sees through it, splitting the triangles that cross its edges, so the
//! world camera's picture shows through every screen from the seat. Pure geometry on a loaded mesh; lives in the
//! app because it serves only the cockpit pass.

use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

/// How far behind a window's plane geometry must lie to be cut (metres): negative, so a cabin face lying in the
/// window's own plane goes too (the screen's backing is coplanar with it). The MFD bezels hung on the screen stand
/// 4 mm out and stay.
const BEHIND_M: f32 = -0.002;

/// A cutting region: the points behind a window triangle as seen from the eye (inside all four planes).
struct Region {
    planes: [(Vec3, f32); 4],
}

impl Region {
    fn new(eye: Vec3, w: [Vec3; 3]) -> Option<Self> {
        let n = (w[1] - w[0]).cross(w[2] - w[0]).normalize_or_zero();
        if n == Vec3::ZERO {
            return None;
        }
        // Behind: away from the eye. A plane is (normal, offset) with inside where normal . p > offset.
        let towards_eye = if n.dot(eye - w[0]) > 0.0 { n } else { -n };
        let back = (-towards_eye, -towards_eye.dot(w[0]) + BEHIND_M);
        let centre = (w[0] + w[1] + w[2]) / 3.0;
        let side = |a: Vec3, b: Vec3| {
            let mut s = (a - eye).cross(b - eye).normalize_or_zero();
            if s.dot(centre - eye) < 0.0 {
                s = -s;
            }
            (s, s.dot(eye))
        };
        Some(Self {
            planes: [back, side(w[0], w[1]), side(w[1], w[2]), side(w[2], w[0])],
        })
    }
}

/// One vertex being clipped: its position and every float attribute flattened.
#[derive(Clone)]
struct V {
    p: Vec3,
    a: Vec<f32>,
}

fn lerp(x: &V, y: &V, t: f32) -> V {
    V {
        p: x.p.lerp(y.p, t),
        a: x.a.iter().zip(&y.a).map(|(a, b)| a + (b - a) * t).collect(),
    }
}

/// Splits a convex polygon by a plane into (inside, outside).
fn split(poly: &[V], (n, d): (Vec3, f32)) -> (Vec<V>, Vec<V>) {
    let (mut inside, mut outside) = (Vec::new(), Vec::new());
    for i in 0..poly.len() {
        let (a, b) = (&poly[i], &poly[(i + 1) % poly.len()]);
        let (da, db) = (n.dot(a.p) - d, n.dot(b.p) - d);
        if da >= 0.0 {
            inside.push(a.clone());
        } else {
            outside.push(a.clone());
        }
        if (da >= 0.0) != (db >= 0.0) {
            let m = lerp(a, b, da / (da - db));
            inside.push(m.clone());
            outside.push(m);
        }
    }
    (inside, outside)
}

/// The parts of a convex polygon outside a region (each convex).
fn subtract(poly: Vec<V>, r: &Region, out: &mut Vec<Vec<V>>) {
    // Wholly outside one plane: untouched.
    if r.planes.iter().any(|&(n, d)| poly.iter().all(|v| n.dot(v.p) - d < 0.0)) {
        out.push(poly);
        return;
    }
    let mut rest = poly;
    for &plane in &r.planes {
        let (inside, outside) = split(&rest, plane);
        if outside.len() >= 3 {
            out.push(outside);
        }
        rest = inside;
        if rest.len() < 3 {
            return;
        }
    }
    // What is left is inside every plane: cut away.
}

/// Cuts from `mesh` (whose vertices `to_space` carries into the interior's space) everything behind the window
/// triangles `windows` as seen from `eye`. Returns the cut mesh, or `None` when nothing is cut or the mesh is not
/// one this can rebuild (not a triangle list, or an attribute that is not float).
pub fn cut(mesh: &Mesh, to_space: Mat4, eye: Vec3, windows: &[[Vec3; 3]]) -> Option<Mesh> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    let regions: Vec<Region> = windows.iter().filter_map(|w| Region::new(eye, *w)).collect();
    let attrs: Vec<(bevy::mesh::MeshVertexAttribute, &VertexAttributeValues)> = mesh.try_attributes().ok()?.map(|(a, v)| (*a, v)).collect();
    let pos = match mesh.try_attribute(Mesh::ATTRIBUTE_POSITION).ok()? {
        VertexAttributeValues::Float32x3(p) => p,
        _ => return None,
    };
    let widths: Vec<usize> = attrs.iter().map(|(_, v)| width(v)).collect::<Option<_>>()?;
    let vertex = |i: usize| V {
        p: to_space.transform_point3(Vec3::from(pos[i])),
        a: attrs.iter().flat_map(|(_, v)| floats(v, i)).collect(),
    };
    let index: Vec<usize> = match mesh.indices() {
        Some(Indices::U16(i)) => i.iter().map(|&i| i as usize).collect(),
        Some(Indices::U32(i)) => i.iter().map(|&i| i as usize).collect(),
        None => (0..pos.len()).collect(),
    };
    let mut changed = false;
    let mut polys: Vec<Vec<V>> = Vec::new();
    let mut kept: Vec<[usize; 3]> = Vec::new();
    for t in index.as_chunks::<3>().0 {
        let tri = vec![vertex(t[0]), vertex(t[1]), vertex(t[2])];
        let mut parts = vec![tri];
        for r in &regions {
            let mut next = Vec::new();
            for p in parts {
                subtract(p, r, &mut next);
            }
            parts = next;
        }
        if parts.len() == 1 && parts[0].len() == 3 && parts[0].iter().zip(t).all(|(v, &i)| v.p == vertex(i).p) {
            kept.push(*t);
        } else {
            changed = true;
            polys.extend(parts);
        }
    }
    if !changed {
        return None;
    }
    // The untouched triangles keep their vertices; every cut piece appends new ones.
    let back = to_space.inverse();
    let mut verts: Vec<V> = (0..pos.len()).map(vertex).collect();
    let mut tris: Vec<u32> = kept.iter().flatten().map(|&i| i as u32).collect();
    for p in polys {
        let base = verts.len() as u32;
        let n = p.len() as u32;
        verts.extend(p);
        for k in 1..n - 1 {
            tris.extend([base, base + k, base + k + 1]);
        }
    }
    let mut out = Mesh::new(PrimitiveTopology::TriangleList, mesh.asset_usage);
    let mut offset = 0;
    for ((attr, values), w) in attrs.iter().zip(&widths) {
        let column: Vec<Vec<f32>> = verts
            .iter()
            .map(|v| {
                if attr.id == Mesh::ATTRIBUTE_POSITION.id {
                    back.transform_point3(v.p).to_array().to_vec()
                } else {
                    v.a[offset..offset + w].to_vec()
                }
            })
            .collect();
        out.insert_attribute(*attr, rebuild(values, column, attr.id == Mesh::ATTRIBUTE_NORMAL.id));
        offset += w;
    }
    out.insert_indices(Indices::U32(tris));
    Some(out)
}

/// A triangle-list mesh's triangles carried by `to_space` (for the window quads).
pub fn triangles(mesh: &Mesh, to_space: Mat4) -> Vec<[Vec3; 3]> {
    let Ok(VertexAttributeValues::Float32x3(pos)) = mesh.try_attribute(Mesh::ATTRIBUTE_POSITION) else {
        return Vec::new();
    };
    let at = |i: usize| to_space.transform_point3(Vec3::from(pos[i]));
    let index: Vec<usize> = match mesh.indices() {
        Some(Indices::U16(i)) => i.iter().map(|&i| i as usize).collect(),
        Some(Indices::U32(i)) => i.iter().map(|&i| i as usize).collect(),
        None => (0..pos.len()).collect(),
    };
    index.as_chunks::<3>().0.iter().map(|t| t.map(at)).collect()
}

fn width(v: &VertexAttributeValues) -> Option<usize> {
    match v {
        VertexAttributeValues::Float32(_) => Some(1),
        VertexAttributeValues::Float32x2(_) => Some(2),
        VertexAttributeValues::Float32x3(_) => Some(3),
        VertexAttributeValues::Float32x4(_) => Some(4),
        _ => None,
    }
}

fn floats(v: &VertexAttributeValues, i: usize) -> Vec<f32> {
    match v {
        VertexAttributeValues::Float32(x) => vec![x[i]],
        VertexAttributeValues::Float32x2(x) => x[i].to_vec(),
        VertexAttributeValues::Float32x3(x) => x[i].to_vec(),
        VertexAttributeValues::Float32x4(x) => x[i].to_vec(),
        _ => Vec::new(),
    }
}

fn rebuild(like: &VertexAttributeValues, column: Vec<Vec<f32>>, unit: bool) -> VertexAttributeValues {
    let fix = |c: Vec<f32>| -> Vec<f32> {
        if unit && c.len() == 3 {
            Vec3::from_slice(&c).normalize_or_zero().to_array().to_vec()
        } else {
            c
        }
    };
    match like {
        VertexAttributeValues::Float32(_) => VertexAttributeValues::Float32(column.into_iter().map(|c| c[0]).collect()),
        VertexAttributeValues::Float32x2(_) => VertexAttributeValues::Float32x2(column.into_iter().map(|c| [c[0], c[1]]).collect()),
        VertexAttributeValues::Float32x3(_) => {
            VertexAttributeValues::Float32x3(column.into_iter().map(fix).map(|c| [c[0], c[1], c[2]]).collect())
        }
        _ => VertexAttributeValues::Float32x4(column.into_iter().map(|c| [c[0], c[1], c[2], c[3]]).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;

    /// A square of the given size at depth z, facing +z, as two triangles.
    fn square(half: f32, z: f32) -> Mesh {
        let p = vec![[-half, -half, z], [half, -half, z], [half, half, z], [-half, half, z]];
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    }

    fn area(m: &Mesh) -> f32 {
        let VertexAttributeValues::Float32x3(p) = m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() else {
            panic!()
        };
        let Some(Indices::U32(i)) = m.indices() else { panic!() };
        i.chunks(3)
            .map(|t| {
                let (a, b, c) = (
                    Vec3::from(p[t[0] as usize]),
                    Vec3::from(p[t[1] as usize]),
                    Vec3::from(p[t[2] as usize]),
                );
                (b - a).cross(c - a).length() / 2.0
            })
            .sum()
    }

    /// A 1 m window at z = 1 (two triangles), seen from the eye at the origin.
    fn window() -> Vec<[Vec3; 3]> {
        let (a, b, c, d) = (
            Vec3::new(-0.5, -0.5, 1.0),
            Vec3::new(0.5, -0.5, 1.0),
            Vec3::new(0.5, 0.5, 1.0),
            Vec3::new(-0.5, 0.5, 1.0),
        );
        vec![[a, b, c], [a, c, d]]
    }

    #[test]
    fn a_back_plate_just_behind_a_window_is_cut_away_entirely() {
        let cut = cut(&square(0.5, 1.02), Mat4::IDENTITY, Vec3::ZERO, &window()).expect("cut");
        assert!(area(&cut) < 1e-4, "left {} m2", area(&cut));
    }

    #[test]
    fn a_wall_behind_a_window_keeps_everything_outside_the_view_through_it() {
        // A 4 m wall 2 m out: the window's 1 m at 1 m covers 2 m x 2 m of it, so 16 - 4 = 12 m2 stays.
        let cut = cut(&square(2.0, 2.0), Mat4::IDENTITY, Vec3::ZERO, &window()).expect("cut");
        assert!((area(&cut) - 12.0).abs() < 1e-3, "kept {} m2", area(&cut));
    }

    #[test]
    fn a_face_in_the_windows_own_plane_is_cut_too() {
        let cut = cut(&square(0.5, 1.0), Mat4::IDENTITY, Vec3::ZERO, &window()).expect("cut");
        assert!(area(&cut) < 1e-4, "left {} m2", area(&cut));
    }

    #[test]
    fn geometry_in_front_of_a_window_or_off_to_the_side_is_untouched() {
        assert!(cut(&square(0.4, 0.8), Mat4::IDENTITY, Vec3::ZERO, &window()).is_none());
        let aside = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        assert!(cut(&square(0.5, 1.5), aside, Vec3::ZERO, &window()).is_none());
    }
}
