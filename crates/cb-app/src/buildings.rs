//! Building drawing and timing for `structural-destruction`: a whole building is one box; a
//! damaged one is drawn from its structural frame (facade panels in the building's facade
//! material, floors and columns in bare concrete) in bands of four storeys, and only a band whose
//! standing elements changed is rebuilt. Each fixed step advances the frames' clock so overloaded
//! members fail when their creak runs out and falling parts land.

use crate::sim::{PresentSet, SimSet};
use crate::world::{Live, MeshBuilder};
use bevy::prelude::*;
use cb_core::world::destruct::Cells;
use cb_core::world::structure::{Frame, Kind};
use std::collections::HashMap;

/// Storeys per drawn band of a damaged building.
const BAND_STOREYS: u32 = 4;

/// Materials buildings are drawn with: the facade tones (by id) and bare structure.
#[derive(Resource)]
pub struct BuildingLooks {
    /// Facade materials, picked by building id modulo the count.
    pub tones: Vec<Handle<StandardMaterial>>,
    /// Exposed floors and columns.
    pub structure: Handle<StandardMaterial>,
}

/// A damaged building's drawn bands: per band, its entities and the standing count it was drawn with.
#[derive(Resource, Default)]
struct DrawnBands(HashMap<u32, Vec<Option<(Vec<Entity>, usize)>>>);

/// Building entities by id.
#[derive(Resource, Default)]
pub struct BuildingEntities(pub Vec<Entity>);

/// Redraws damaged buildings.
pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingEntities>()
            .init_resource::<DrawnBands>()
            .add_systems(FixedUpdate, step_structures.in_set(SimSet::World))
            .add_systems(Update, redraw_buildings.in_set(PresentSet::Pose));
    }
}

/// The mesh of a building's standing cells, in world space.
fn cells_mesh(c: &Cells) -> Mesh {
    let mut b = MeshBuilder::tiled(4.0, 3.5);
    let white = [1.0, 1.0, 1.0, 1.0];
    let solid = |i: isize, k: isize, j: isize| {
        i >= 0
            && k >= 0
            && j >= 0
            && (i as usize) < c.n[0]
            && (k as usize) < c.n[1]
            && (j as usize) < c.n[2]
            && c.at(i as usize, k as usize, j as usize)
    };
    for k in 0..c.n[1] {
        for j in 0..c.n[2] {
            for i in 0..c.n[0] {
                if !c.at(i, k, j) {
                    continue;
                }
                let lo = c.origin + c.size * Vec3::new(i as f32, k as f32, j as f32);
                let hi = lo + c.size;
                let v = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
                let (ii, kk, jj) = (i as isize, k as isize, j as isize);
                // Each face listed so its outward normal is (c1 - c0) x (c3 - c0).
                if !solid(ii, kk + 1, jj) {
                    b.quad(
                        [v(lo.x, hi.y, lo.z), v(lo.x, hi.y, hi.z), v(hi.x, hi.y, hi.z), v(hi.x, hi.y, lo.z)],
                        white,
                    );
                }
                if !solid(ii, kk - 1, jj) {
                    b.quad(
                        [v(lo.x, lo.y, lo.z), v(hi.x, lo.y, lo.z), v(hi.x, lo.y, hi.z), v(lo.x, lo.y, hi.z)],
                        white,
                    );
                }
                if !solid(ii, kk, jj + 1) {
                    b.quad(
                        [v(lo.x, lo.y, hi.z), v(hi.x, lo.y, hi.z), v(hi.x, hi.y, hi.z), v(lo.x, hi.y, hi.z)],
                        white,
                    );
                }
                if !solid(ii, kk, jj - 1) {
                    b.quad(
                        [v(hi.x, lo.y, lo.z), v(lo.x, lo.y, lo.z), v(lo.x, hi.y, lo.z), v(hi.x, hi.y, lo.z)],
                        white,
                    );
                }
                if !solid(ii + 1, kk, jj) {
                    b.quad(
                        [v(hi.x, lo.y, hi.z), v(hi.x, lo.y, lo.z), v(hi.x, hi.y, lo.z), v(hi.x, hi.y, hi.z)],
                        white,
                    );
                }
                if !solid(ii - 1, kk, jj) {
                    b.quad(
                        [v(lo.x, lo.y, lo.z), v(lo.x, lo.y, hi.z), v(lo.x, hi.y, hi.z), v(lo.x, hi.y, lo.z)],
                        white,
                    );
                }
            }
        }
    }
    b.build()
}

/// Creaking members fail on time and falling parts land; what falls goes to the debris like any
/// other chunk, and what reaches the ground is raised as a heap in the crater layer (`rubble`), so
/// the pilot, the mech and the debris stand on it.
fn step_structures(time: Res<Time<Fixed>>, mut live: ResMut<Live>) {
    let (changed, chunks) = live.structures.step(time.delta_secs());
    live.dirty_buildings.extend(changed);
    live.new_chunks.extend(chunks);
    let cfg = live.structures.cfg.clone();
    for heap in live.structures.take_landings() {
        let touched = live.dug.pile(
            heap.lo,
            heap.hi,
            heap.volume_m3,
            cfg.rubble_spread_m,
            cfg.repose_slope,
            cfg.max_rubble_m,
        );
        live.dirty_tiles.extend(touched);
    }
}

/// The meshes of band `band` of a frame: (facade panels, floors and columns). Columns are drawn
/// 1 cm thinner than they are and side walls stop short of the corner walls, so no two faces share
/// a plane (CLAUDE.md section 10).
fn band_meshes(f: &Frame, band: u32, wall_m: f32) -> (Mesh, Mesh, usize) {
    let mut facade = MeshBuilder::tiled(4.0, 3.5);
    let mut bare = MeshBuilder::tiled(4.0, 4.0);
    let white = [1.0; 4];
    let (k0, k1) = (band * BAND_STOREYS, (band + 1) * BAND_STOREYS);
    let mut standing = 0;
    for e in f.elements.iter().filter(|e| e.alive && e.storey >= k0 && e.storey < k1) {
        standing += 1;
        match e.kind {
            Kind::Wall => {
                let (mut lo, mut hi) = (e.min, e.max);
                if e.side >= 2 {
                    if e.at[1] == 0 {
                        lo.z += wall_m;
                    }
                    if e.at[1] as usize + 1 == f.n[2] {
                        hi.z -= wall_m;
                    }
                }
                facade.block(lo, hi, white);
            }
            Kind::Column => bare.block(e.min + Vec3::new(0.01, 0.0, 0.01), e.max - Vec3::new(0.01, 0.0, 0.01), white),
            Kind::Slab => bare.block(e.min, e.max, [0.85, 0.85, 0.85, 1.0]),
        }
    }
    (facade.build(), bare.build(), standing)
}

fn redraw_buildings(
    mut commands: Commands,
    mut live: ResMut<Live>,
    ents: Res<BuildingEntities>,
    looks: Option<Res<BuildingLooks>>,
    mut drawn: ResMut<DrawnBands>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if live.dirty_buildings.is_empty() || ents.0.is_empty() {
        return;
    }
    let Some(looks) = looks else { return };
    let mut ids = std::mem::take(&mut live.dirty_buildings);
    ids.sort_unstable();
    ids.dedup();
    let wall_m = live.structures.cfg.wall_m;
    for id in ids {
        let c = &live.structures.buildings[id as usize];
        let e = ents.0[id as usize];
        let Some(f) = c.frame.as_ref() else {
            // No frame (an old save, a test): the cell view.
            if c.standing == 0 {
                commands.entity(e).insert(Visibility::Hidden);
            } else {
                commands.entity(e).insert((Mesh3d(meshes.add(cells_mesh(c))), Transform::IDENTITY));
            }
            continue;
        };
        // The whole-building box gives way to the frame's bands.
        commands.entity(e).insert(Visibility::Hidden);
        let bands = (f.n[1] as u32).div_ceil(BAND_STOREYS);
        let slots = drawn.0.entry(id).or_insert_with(|| vec![None; bands as usize]);
        for band in 0..bands {
            let (k0, k1) = (band * BAND_STOREYS, (band + 1) * BAND_STOREYS);
            let now = f.elements.iter().filter(|e| e.alive && e.storey >= k0 && e.storey < k1).count();
            let slot = &mut slots[band as usize];
            if slot.as_ref().is_some_and(|(_, was)| *was == now) {
                continue;
            }
            if let Some((old, _)) = slot.take() {
                for o in old {
                    commands.entity(o).despawn();
                }
            }
            if now == 0 {
                *slot = Some((Vec::new(), 0));
                continue;
            }
            let (facade, bare, _) = band_meshes(f, band, wall_m);
            let tone = looks.tones[id as usize % looks.tones.len()].clone();
            let a = commands
                .spawn((
                    Name::new(format!("building {id} band {band} facade")),
                    Mesh3d(meshes.add(facade)),
                    MeshMaterial3d(tone),
                    Transform::IDENTITY,
                ))
                .id();
            let b = commands
                .spawn((
                    Name::new(format!("building {id} band {band} structure")),
                    Mesh3d(meshes.add(bare)),
                    MeshMaterial3d(looks.structure.clone()),
                    Transform::IDENTITY,
                ))
                .id();
            *slot = Some((vec![a, b], now));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_core::world::StructureConfig;

    fn frame() -> Frame {
        Frame::new(3, [4, 9, 3], Vec3::ZERO, Vec3::new(6.0, 3.5, 6.0), &StructureConfig::default())
    }

    #[test]
    fn a_band_draws_every_standing_element_of_its_storeys() {
        let f = frame();
        let per_storey = f.elements.iter().filter(|e| e.storey == 0).count();
        let (_, _, n0) = band_meshes(&f, 0, 0.3);
        let (_, _, n2) = band_meshes(&f, 2, 0.3);
        assert_eq!(n0, per_storey * 4);
        assert_eq!(n2, per_storey, "the last band holds the ninth storey only");
    }

    #[test]
    fn a_broken_element_leaves_its_band() {
        let mut f = frame();
        let id = f.elements.iter().position(|e| e.kind == Kind::Wall && e.storey == 5).unwrap();
        let before = band_meshes(&f, 1, 0.3).2;
        f.elements[id].alive = false;
        assert_eq!(band_meshes(&f, 1, 0.3).2, before - 1);
        assert_eq!(
            band_meshes(&f, 0, 0.3).2,
            band_meshes(&frame(), 0, 0.3).2,
            "other bands are untouched"
        );
    }
}
