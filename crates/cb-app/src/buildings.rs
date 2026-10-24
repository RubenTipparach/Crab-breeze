//! Building drawing and timing for `structural-destruction`: a whole building is one box; a
//! damaged one is rebuilt from its standing cells (a face only where the cell beside it is gone),
//! and a building with nothing standing is hidden. Each fixed step advances the frames' clock so
//! overloaded members fail when their creak runs out.

use crate::sim::{PresentSet, SimSet};
use crate::world::{Live, MeshBuilder};
use bevy::prelude::*;
use cb_core::world::destruct::Cells;

/// Building entities by id.
#[derive(Resource, Default)]
pub struct BuildingEntities(pub Vec<Entity>);

/// Redraws damaged buildings.
pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingEntities>()
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

/// Creaking members fail on time; what falls goes to the debris like any other chunk.
fn step_structures(time: Res<Time<Fixed>>, mut live: ResMut<Live>) {
    let (changed, chunks) = live.structures.step(time.delta_secs());
    live.dirty_buildings.extend(changed);
    live.new_chunks.extend(chunks);
}

fn redraw_buildings(mut commands: Commands, mut live: ResMut<Live>, ents: Res<BuildingEntities>, mut meshes: ResMut<Assets<Mesh>>) {
    if live.dirty_buildings.is_empty() || ents.0.is_empty() {
        return;
    }
    let mut ids = std::mem::take(&mut live.dirty_buildings);
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        let c = &live.structures.buildings[id as usize];
        let e = ents.0[id as usize];
        if c.standing == 0 {
            commands.entity(e).insert(Visibility::Hidden);
            continue;
        }
        commands.entity(e).insert((Mesh3d(meshes.add(cells_mesh(c))), Transform::IDENTITY));
    }
}
