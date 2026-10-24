//! Building drawing and timing for `structural-destruction`: a whole building is one box; a
//! damaged one is drawn from its structural frame (facade panels in the building's facade
//! material, floors and columns in bare concrete) in bands of twelve storeys, and only a band whose
//! standing elements changed is rebuilt. Each fixed step advances the frames' clock so overloaded
//! members fail when their creak runs out and falling parts land.

use crate::sim::{PresentSet, SimSet};
use crate::world::{Live, MeshBuilder};
use bevy::prelude::*;
use cb_core::rng::SimRng;
use cb_core::world::destruct::Cells;
use cb_core::world::dressing::{self, Anchor, DressingConfig, Shape};
use cb_core::world::structure::{Frame, GROUND, Kind};
use std::collections::HashMap;

/// Storeys per drawn band of a damaged building. Bands keep a rebuild small; few of them keep the
/// mesh entity count low (the game is CPU-bound on entities per view, measured on the RTX 3070,
/// openspec change `gpu-instancing`): a 60-storey tower is 5 bands, 10 entities.
const BAND_STOREYS: u32 = 12;

/// Materials buildings are drawn with: the facade tones (by id) and bare structure.
#[derive(Resource)]
pub struct BuildingLooks {
    /// Facade materials, picked by building id modulo the count.
    pub tones: Vec<Handle<StandardMaterial>>,
    /// Exposed floors and columns.
    pub structure: Handle<StandardMaterial>,
    /// Dressing: pilasters, courses, parapets, roof props (coloured per vertex).
    pub trim: Handle<StandardMaterial>,
}

/// Dressed buildings near the camera: the dressing's entity and whether it is cut per element.
#[derive(Resource, Default)]
struct Dressed(HashMap<u32, (Entity, bool)>);

/// Most buildings dressed in one frame (each costs well under a millisecond).
const DRESS_PER_FRAME: usize = 6;

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
            .init_resource::<Dressed>()
            .add_systems(FixedUpdate, step_structures.in_set(SimSet::World))
            .add_systems(Update, (redraw_buildings, dress_near_camera).chain().in_set(PresentSet::Pose));
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
fn step_structures(time: Res<Time<Fixed>>, mut live: ResMut<Live>, mut fx: MessageWriter<crate::fx::Fx>) {
    let (changed, chunks) = live.structures.step(time.delta_secs());
    for m in live.structures.take_moments() {
        fx.write(match m {
            cb_core::world::Moment::Creak(at) => crate::fx::Fx::new("creak_dust", at),
            cb_core::world::Moment::Fail(at) => crate::fx::Fx::new("member_fail", at),
            cb_core::world::Moment::Impact { at, mass_kg } => {
                crate::fx::Fx::new("collapse_dust", at).scaled((mass_kg / 2.0e5).cbrt().clamp(0.5, 3.0))
            }
        });
    }
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

/// True while the element a dressing piece hangs on stands (a whole building: always).
fn anchored(f: Option<&Frame>, a: Anchor) -> bool {
    let Some(f) = f else { return true };
    let id = match a {
        Anchor::Whole => return true,
        Anchor::Column { k, a, c } => f.column(k as usize, a as usize, c as usize),
        Anchor::Slab { k, i, j } => f.slab(k as usize, i as usize, j as usize),
        Anchor::Wall { k, side, idx } => f.wall(k as usize, side as usize, idx as usize),
    };
    id != GROUND && f.elements[id as usize].alive
}

/// The dressing mesh of building `id`: cut per element (and filtered by what stands) once it has
/// a frame, merged while it is whole.
fn dressing_mesh(c: &Cells, id: u32, seed: &SimRng, cfg: &DressingConfig) -> Mesh {
    let f = c.frame.as_deref();
    let mut mb = MeshBuilder::tiled(4.0, 4.0);
    for p in dressing::dress(id, c.n, c.origin, c.size, seed, f.is_some(), cfg) {
        if !anchored(f, p.anchor) {
            continue;
        }
        let col = [p.color[0], p.color[1], p.color[2], 1.0];
        match p.shape {
            Shape::Block { min, max } => mb.block(min, max, col),
            Shape::Drum { base, r0, r1, height_m } => mb.drum(base, r0, r1, height_m, 14, col),
        }
    }
    mb.build()
}

/// Dresses the buildings near the camera, a few a frame, and drops the dressing of those left
/// behind.
#[allow(clippy::too_many_arguments)]
fn dress_near_camera(
    mut commands: Commands,
    live: Res<Live>,
    plan: Res<crate::world::Plan>,
    seed: Res<crate::world::Seed>,
    looks: Option<Res<BuildingLooks>>,
    cfg: Res<crate::sim::Data>,
    camera: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    mut dressed: ResMut<Dressed>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(looks), Ok(eye)) = (looks, camera.single()) else { return };
    let cfg = &cfg.0.world.dressing;
    let at = eye.translation();
    let here = Vec2::new(at.x, at.z);
    let rng = SimRng::new(seed.0).fork("dressing");
    let near = plan.0.collision.candidates(here, cfg.dress_radius_m);
    let mut added = 0;
    for b in near {
        if added >= DRESS_PER_FRAME {
            break;
        }
        if dressed.0.contains_key(&b) || plan.0.buildings[b as usize].footprint.centre().distance(here) > cfg.dress_radius_m {
            continue;
        }
        let c = &live.structures.buildings[b as usize];
        if c.standing == 0 {
            continue;
        }
        let e = commands
            .spawn((
                Name::new(format!("building {b} dressing")),
                Mesh3d(meshes.add(dressing_mesh(c, b, &rng, cfg))),
                MeshMaterial3d(looks.trim.clone()),
                Transform::IDENTITY,
            ))
            .id();
        dressed.0.insert(b, (e, c.frame.is_some()));
        added += 1;
    }
    let keep = cfg.dress_radius_m * 1.15;
    let gone: Vec<u32> = dressed
        .0
        .keys()
        .copied()
        .filter(|&b| plan.0.buildings[b as usize].footprint.centre().distance(here) > keep)
        .collect();
    for b in gone {
        if let Some((e, _)) = dressed.0.remove(&b) {
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn redraw_buildings(
    mut commands: Commands,
    mut live: ResMut<Live>,
    ents: Res<BuildingEntities>,
    looks: Option<Res<BuildingLooks>>,
    mut drawn: ResMut<DrawnBands>,
    mut dressed: ResMut<Dressed>,
    seed: Res<crate::world::Seed>,
    cfg: Res<crate::sim::Data>,
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
    let rng = SimRng::new(seed.0).fork("dressing");
    for id in ids {
        let c = &live.structures.buildings[id as usize];
        let e = ents.0[id as usize];
        // A dressed building's dressing follows its elements.
        if let Some((d, _)) = dressed.0.get(&id).copied() {
            if c.standing == 0 {
                commands.entity(d).despawn();
                dressed.0.remove(&id);
            } else {
                commands
                    .entity(d)
                    .insert(Mesh3d(meshes.add(dressing_mesh(c, id, &rng, &cfg.0.world.dressing))));
                dressed.0.insert(id, (d, c.frame.is_some()));
            }
        }
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
        let f = Frame::new(3, [4, 13, 3], Vec3::ZERO, Vec3::new(6.0, 3.5, 6.0), &StructureConfig::default());
        let (_, _, n0) = band_meshes(&f, 0, 0.3);
        let (_, _, n1) = band_meshes(&f, 1, 0.3);
        assert_eq!(n0, per_storey * 12);
        assert_eq!(n1, per_storey, "the last band holds the thirteenth storey only");
    }

    #[test]
    fn dressing_drops_the_pieces_whose_element_fell() {
        let f = frame();
        let cfg = DressingConfig::default();
        let rng = SimRng::new(1);
        let all = dressing::dress(3, f.n, f.origin, f.size, &rng, true, &cfg);
        let standing = |f: &Frame| all.iter().filter(|p| anchored(Some(f), p.anchor)).count();
        assert_eq!(standing(&f), all.len(), "a frame that stands keeps every piece");
        let mut broken = f.clone();
        let roof = broken.slab(f.n[1] - 1, 1, 1) as usize;
        broken.elements[roof].alive = false;
        let col = broken.column(4, 0, 0) as usize;
        broken.elements[col].alive = false;
        assert!(standing(&broken) < all.len(), "the pieces on the broken roof bay and column go");
        assert!(anchored(None, Anchor::Whole));
    }

    #[test]
    fn a_broken_element_leaves_its_band() {
        let mut f = Frame::new(3, [4, 13, 3], Vec3::ZERO, Vec3::new(6.0, 3.5, 6.0), &StructureConfig::default());
        let id = f.elements.iter().position(|e| e.kind == Kind::Wall && e.storey == 12).unwrap();
        let (b0, b1) = (band_meshes(&f, 0, 0.3).2, band_meshes(&f, 1, 0.3).2);
        f.elements[id].alive = false;
        assert_eq!(band_meshes(&f, 1, 0.3).2, b1 - 1);
        assert_eq!(band_meshes(&f, 0, 0.3).2, b0, "other bands are untouched");
    }
}
