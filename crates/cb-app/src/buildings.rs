//! Building drawing and timing (`building-generation`, `structural-destruction`):
//! - a building far from the player is one plain box (no generated detail);
//! - within `dress_radius_m` of the player it is dressed instead: the mockup's facade skin per bay
//!   per storey, roof, parapet and roof props, in a trim mesh and a glass mesh, and its box hidden;
//! - a damaged one also draws its frame's floors and columns and the rooms behind them in bands of
//!   twelve storeys (facade panels too, where no skin hangs).
//!
//! Meshes are rebuilt in place (`Assets::insert` on the same handle), never despawned and spawned:
//! a fresh entity is drawn a frame late, and a mech ramming a building rebuilt it every few frames,
//! which strobed the whole view (desktop playtest, 2026-10-24). Each fixed step advances the
//! frames' clock so overloaded members fail when their creak runs out and falling parts land.

use crate::sim::{PresentSet, SimSet};
use crate::world::{Live, MeshBuilder};
use bevy::prelude::*;
use cb_core::rng::SimRng;
use cb_core::world::destruct::Cells;
use cb_core::world::dressing::{self, Anchor, DressingConfig, Mat, Shape};
use cb_core::world::interior::{self, Rest};
use cb_core::world::structure::{Frame, GROUND, Kind};
use std::collections::{BTreeMap, HashMap};

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
    /// Dressing materials: brick, concrete, roof and wood textured, trim plain, glass see-through.
    pub skin: BTreeMap<Mat, Handle<StandardMaterial>>,
}

/// A drawn mesh: its entity and the mesh it shows (rebuilt in place).
#[derive(Clone)]
struct Drawn {
    entity: Entity,
    mesh: Handle<Mesh>,
}

/// A dressed building's meshes, one per material.
struct Dressing(BTreeMap<Mat, Drawn>);

/// Dressed buildings near the player.
#[derive(Resource, Default)]
struct Dressed(HashMap<u32, Dressing>);

/// Set when every building near the player is dressed (none waits for its turn), so a capture
/// can wait for the street to finish dressing on a slow renderer.
#[derive(Resource, Default)]
pub struct DressingSettled(pub bool);

/// Most buildings dressed in one frame (a 60-storey tower's skin and rooms are a few milliseconds).
const DRESS_PER_FRAME: usize = 1;

/// One drawn band of a damaged building: its meshes, and what it was drawn from.
struct Band {
    facade: Drawn,
    bare: Drawn,
    count: usize,
    walls: bool,
}

/// A damaged building's drawn bands.
#[derive(Resource, Default)]
struct DrawnBands(HashMap<u32, Vec<Option<Band>>>);

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
            .init_resource::<DressingSettled>()
            .add_systems(FixedUpdate, step_structures.in_set(SimSet::World))
            .add_systems(Update, (dress_near_player, redraw_buildings).chain().in_set(PresentSet::Pose));
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
                crate::fx::Fx::new("collapse_dust", at).scaled((mass_kg / 2.0e5).cbrt().clamp(0.5, 2.0))
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

/// The meshes of band `band` of a frame: (facade panels when `walls`, else none; floors, columns
/// and the interiors behind them). Columns are drawn 1 cm thinner than they are and side walls stop
/// short of the corner walls, so no two faces share a plane (CLAUDE.md section 10). Partitions and
/// furniture (`building-interiors`) are drawn while the floor bay under them stands. Returns the
/// standing element count with the meshes.
fn band_meshes(f: &Frame, band: u32, wall_m: f32, seed: &SimRng, walls: bool) -> (Mesh, Mesh, usize) {
    let mut facade = MeshBuilder::tiled(4.0, 3.5);
    let mut bare = MeshBuilder::tiled(4.0, 4.0);
    let white = [1.0; 4];
    let concrete = [0.66, 0.64, 0.6, 1.0];
    let (k0, k1) = (band * BAND_STOREYS, (band + 1) * BAND_STOREYS);
    let mut standing = 0;
    for e in f.elements.iter().filter(|e| e.alive && e.storey >= k0 && e.storey < k1) {
        standing += 1;
        match e.kind {
            Kind::Wall if walls => {
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
            Kind::Wall => {}
            Kind::Column => bare.block(e.min + Vec3::new(0.01, 0.0, 0.01), e.max - Vec3::new(0.01, 0.0, 0.01), concrete),
            Kind::Slab => bare.block(e.min, e.max, [0.56, 0.55, 0.52, 1.0]),
        }
    }
    let holds = |r: Rest| match r {
        Rest::Ground => true,
        Rest::Slab { k, i, j } => f.elements[f.slab(k as usize, i as usize, j as usize) as usize].alive,
    };
    for k in k0..k1.min(f.n[1] as u32) {
        let plan = interior::plan_storey(f.building, k as usize, f.n, f.origin, f.size, seed);
        for w in plan.walls.iter().filter(|w| holds(w.rest)) {
            bare.block(w.min, w.max, if w.core { concrete } else { [0.86, 0.84, 0.78, 1.0] });
        }
        for it in plan.items.iter().filter(|it| holds(it.rest)) {
            for (lo, hi, c) in &it.boxes {
                bare.block(*lo, *hi, [c[0], c[1], c[2], 1.0]);
            }
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

/// Metres per texture tile for a material (its texture's real size).
fn tile_m(mat: Mat) -> f32 {
    match mat {
        Mat::Brick | Mat::Concrete => 2.4,
        Mat::Roof => 2.0,
        Mat::Wood => 1.0,
        Mat::Trim | Mat::Glass => 4.0,
    }
}

/// The dressing meshes of building `id`, one per material: cut per element and filtered by what
/// stands once it has a frame; whole, with its floors, rooms and furniture behind the glass while
/// it is whole (a damaged building draws its rooms in its bands).
fn dressing_meshes(c: &Cells, id: u32, seed: &SimRng, cfg: &DressingConfig) -> Vec<(Mat, Mesh)> {
    let f = c.frame.as_deref();
    let mut builders: BTreeMap<Mat, MeshBuilder> = BTreeMap::new();
    for p in dressing::dress(id, c.n, c.origin, c.size, seed, f.is_some(), cfg) {
        if !anchored(f, p.anchor) {
            continue;
        }
        let mb = builders
            .entry(p.mat)
            .or_insert_with(|| MeshBuilder::tiled(tile_m(p.mat), tile_m(p.mat)));
        let col = [p.color[0], p.color[1], p.color[2], 1.0];
        match p.shape {
            Shape::Block { min, max } => mb.block(min, max, col),
            Shape::Drum { base, r0, r1, height_m } => mb.drum(base, r0, r1, height_m, 14, col),
        }
    }
    if f.is_none() {
        let trim = builders.entry(Mat::Trim).or_insert_with(|| MeshBuilder::tiled(4.0, 4.0));
        for k in 0..c.n[1] {
            let plan = interior::plan_storey(id, k, c.n, c.origin, c.size, seed);
            for w in &plan.walls {
                trim.block(w.min, w.max, if w.core { [0.62, 0.60, 0.56, 1.0] } else { [0.86, 0.84, 0.78, 1.0] });
            }
            for it in &plan.items {
                for (lo, hi, col) in &it.boxes {
                    trim.block(*lo, *hi, [col[0], col[1], col[2], 1.0]);
                }
            }
        }
    }
    builders
        .into_iter()
        .map(|(mat, mb)| {
            let mut mesh = mb.build();
            if matches!(mat, Mat::Brick | Mat::Concrete | Mat::Roof | Mat::Wood) {
                mesh.generate_tangents().ok();
            }
            (mat, mesh)
        })
        .collect()
}

/// Spawns an entity showing `mesh` in `material`, or rebuilds the one it has in place.
fn show(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    slot: Option<&Drawn>,
    mesh: Mesh,
    material: &Handle<StandardMaterial>,
    name: String,
) -> Drawn {
    match slot {
        Some(d) => {
            meshes.insert(&d.mesh, mesh).ok();
            d.clone()
        }
        None => {
            let handle = meshes.add(mesh);
            let entity = commands
                .spawn((
                    Name::new(name),
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::IDENTITY,
                ))
                .id();
            Drawn { entity, mesh: handle }
        }
    }
}

/// Where the player is: the pilot's eyes, on foot or in the cockpit.
fn player_xz(camera: &Query<&GlobalTransform, With<crate::pilot::PilotCamera>>) -> Option<Vec2> {
    camera.single().ok().map(|t| Vec2::new(t.translation().x, t.translation().z))
}

/// Dresses the buildings near the player, a few a frame, and undresses those left behind (their
/// plain box shows again). Far buildings never get generated detail (owner, 2026-10-24: "far away
/// buildings would not HAVE these generated elements").
#[allow(clippy::too_many_arguments)]
fn dress_near_player(
    mut commands: Commands,
    mut live: ResMut<Live>,
    plan: Res<crate::world::Plan>,
    seed: Res<crate::world::Seed>,
    looks: Option<Res<BuildingLooks>>,
    cfg: Res<crate::sim::Data>,
    ents: Res<BuildingEntities>,
    camera: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    mut dressed: ResMut<Dressed>,
    mut settled: ResMut<DressingSettled>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    settled.0 = false;
    let (Some(looks), Some(here)) = (looks, player_xz(&camera)) else {
        return;
    };
    if ents.0.is_empty() {
        return;
    }
    let cfg = &cfg.0.world.dressing;
    let rng = SimRng::new(seed.0).fork("dressing");
    let mut near: Vec<u32> = plan
        .0
        .collision
        .candidates(here, cfg.dress_radius_m)
        .into_iter()
        .filter(|&b| !dressed.0.contains_key(&b) && plan.0.buildings[b as usize].footprint.centre().distance(here) <= cfg.dress_radius_m)
        .collect();
    // Nearest first, ties by id (never hash order).
    near.sort_by(|&a, &b| {
        let (da, db) = (
            plan.0.buildings[a as usize].footprint.centre().distance(here),
            plan.0.buildings[b as usize].footprint.centre().distance(here),
        );
        da.total_cmp(&db).then(a.cmp(&b))
    });
    settled.0 = near.len() <= DRESS_PER_FRAME;
    for b in near.into_iter().take(DRESS_PER_FRAME) {
        let c = &live.structures.buildings[b as usize];
        if c.standing == 0 {
            continue;
        }
        let parts = dressing_meshes(c, b, &rng, cfg)
            .into_iter()
            .map(|(mat, mesh)| {
                (
                    mat,
                    show(
                        &mut commands,
                        &mut meshes,
                        None,
                        mesh,
                        &looks.skin[&mat],
                        format!("building {b} {mat:?}"),
                    ),
                )
            })
            .collect();
        dressed.0.insert(b, Dressing(parts));
        commands.entity(ents.0[b as usize]).insert(Visibility::Hidden);
        if c.frame.is_some() {
            live.dirty_buildings.push(b); // its bands drop the facade panels the skin now covers
        }
    }
    let keep = cfg.dress_radius_m * 1.15;
    let mut gone: Vec<u32> = dressed
        .0
        .keys()
        .copied()
        .filter(|&b| plan.0.buildings[b as usize].footprint.centre().distance(here) > keep)
        .collect();
    gone.sort_unstable();
    for b in gone {
        if let Some(d) = dressed.0.remove(&b) {
            for part in d.0.values() {
                commands.entity(part.entity).despawn();
                meshes.remove(&part.mesh);
            }
        }
        let c = &live.structures.buildings[b as usize];
        if c.frame.is_some() {
            live.dirty_buildings.push(b); // its bands draw the facade panels again
        } else {
            commands.entity(ents.0[b as usize]).insert(Visibility::Visible);
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
    let _t = crate::perf_hud::slow("redraw_buildings");
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
        // A dressed building's skin follows its elements, rebuilt in place.
        if let Some(d) = dressed.0.get_mut(&id) {
            let mut parts = dressing_meshes(c, id, &rng, &cfg.0.world.dressing);
            // A material that no longer appears keeps its entity with an empty mesh.
            for mat in d.0.keys() {
                if !parts.iter().any(|(m, _)| m == mat) {
                    parts.push((*mat, MeshBuilder::tiled(4.0, 4.0).build()));
                }
            }
            for (mat, mesh) in parts {
                let slot = d.0.get(&mat).cloned();
                let drawn = show(
                    &mut commands,
                    &mut meshes,
                    slot.as_ref(),
                    mesh,
                    &looks.skin[&mat],
                    format!("building {id} {mat:?}"),
                );
                d.0.insert(mat, drawn);
            }
        }
        let skinned = dressed.0.contains_key(&id);
        let Some(f) = c.frame.as_ref() else {
            // No frame (an old save, a test): the cell view.
            if c.standing == 0 {
                commands.entity(e).insert(Visibility::Hidden);
            } else {
                commands.entity(e).insert((Mesh3d(meshes.add(cells_mesh(c))), Transform::IDENTITY));
            }
            continue;
        };
        // The whole-building box gives way to the frame's bands (and the skin, if dressed).
        commands.entity(e).insert(Visibility::Hidden);
        let bands = (f.n[1] as u32).div_ceil(BAND_STOREYS);
        let slots = drawn.0.entry(id).or_insert_with(|| (0..bands).map(|_| None).collect());
        for band in 0..bands {
            let (k0, k1) = (band * BAND_STOREYS, (band + 1) * BAND_STOREYS);
            let now = f.elements.iter().filter(|e| e.alive && e.storey >= k0 && e.storey < k1).count();
            let walls = !skinned;
            let slot = &mut slots[band as usize];
            if slot.as_ref().is_some_and(|b| b.count == now && b.walls == walls) {
                continue;
            }
            let (facade, bare, _) = band_meshes(f, band, wall_m, &rng, walls);
            let tone = &looks.tones[id as usize % looks.tones.len()];
            let facade = show(
                &mut commands,
                &mut meshes,
                slot.as_ref().map(|b| &b.facade),
                facade,
                tone,
                format!("building {id} band {band} facade"),
            );
            let bare = show(
                &mut commands,
                &mut meshes,
                slot.as_ref().map(|b| &b.bare),
                bare,
                &looks.structure,
                format!("building {id} band {band} structure"),
            );
            *slot = Some(Band {
                facade,
                bare,
                count: now,
                walls,
            });
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
        let (_, _, n0) = band_meshes(&f, 0, 0.3, &SimRng::new(1), true);
        let (_, _, n1) = band_meshes(&f, 1, 0.3, &SimRng::new(1), true);
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
        let wall = broken.wall(4, 0, 0) as usize;
        broken.elements[wall].alive = false;
        assert!(
            standing(&broken) < all.len(),
            "the pieces on the broken roof bay and facade panel go"
        );
        assert!(anchored(None, Anchor::Whole));
    }

    #[test]
    fn a_broken_element_leaves_its_band() {
        let mut f = Frame::new(3, [4, 13, 3], Vec3::ZERO, Vec3::new(6.0, 3.5, 6.0), &StructureConfig::default());
        let id = f.elements.iter().position(|e| e.kind == Kind::Wall && e.storey == 12).unwrap();
        let (b0, b1) = (
            band_meshes(&f, 0, 0.3, &SimRng::new(1), true).2,
            band_meshes(&f, 1, 0.3, &SimRng::new(1), true).2,
        );
        f.elements[id].alive = false;
        assert_eq!(band_meshes(&f, 1, 0.3, &SimRng::new(1), true).2, b1 - 1);
        assert_eq!(band_meshes(&f, 0, 0.3, &SimRng::new(1), true).2, b0, "other bands are untouched");
    }
}
