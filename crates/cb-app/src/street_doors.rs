//! Street door leaves, drawn swinging (openspec change `building-interiors`; survey W30, owner
//! 2026-10-26: "A door that swings open as you walk into it, and shuts behind you").
//!
//! The swing is `cb_core::world::door::StreetDoor`, stepped by `walk_in` while the pilot is on foot.
//! Here every dressed building gets its leaves as entities of their own (the dressing draws the
//! doorway, not the leaves), hung on their hinges in the building's own materials, and each frame
//! they are turned to the door's angle, between its last two fixed steps. A building whose colliders
//! are not built (the pilot is in the mech, or far away) has its door shut. A leaf whose wall bay has
//! been blown out is hidden with it. Presentation only: the angle is the simulation's.
//!
//! It also keeps `RefugeDoors`, every building's street door as people and car drivers running from
//! danger make for it (`cb_core::civilians::StreetDoors`, from the same `interior::shell` as the leaves).

use crate::buildings::{Dressed, LeafLooks};
use crate::sim::PresentSet;
use crate::state::GameApp;
use crate::walk_in::InteriorColliders;
use crate::world::{Live, Plan, Seed};
use bevy::prelude::*;
use cb_core::civilians::StreetDoors;
use cb_core::world::door::{LEAF_T_M, StreetDoor};
use cb_core::world::dressing;
use cb_core::world::interior::{self, Site, Walkable};
use std::collections::BTreeMap;

/// A building's drawn street door: its geometry and its leaves' entities.
struct Drawn {
    door: StreetDoor,
    leaves: Vec<(Entity, Handle<Mesh>)>,
}

/// The street doors drawn now, by building id.
#[derive(Resource, Default)]
struct DrawnDoors(BTreeMap<u32, Drawn>);

/// Every building's street door, for people and drivers running from danger (civilians, car-traffic).
#[derive(Resource, Default)]
pub struct RefugeDoors(pub StreetDoors);

/// Draws the street door leaves of dressed buildings, posed from the simulation.
pub struct StreetDoorsPlugin;

impl Plugin for StreetDoorsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawnDoors>()
            .init_resource::<RefugeDoors>()
            .add_world_spawn(index_refuges)
            .add_game_systems(Update, (hang_leaves, swing_leaves).chain().in_set(PresentSet::Pose));
    }
}

/// Finds every building's street door once the world is spawned.
fn index_refuges(plan: Res<Plan>, live: Res<Live>, seed: Res<Seed>, data: Res<crate::sim::Data>, mut out: ResMut<RefugeDoors>) {
    let started = std::time::Instant::now();
    let rng = crate::buildings::building_rng(seed.0);
    out.0 = StreetDoors::of_world(&plan.0, &live.structures, &data.0.world.interiors, &rng);
    info!(
        "street doors: {} to run for, found in {:.0} ms",
        out.0.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
}

/// Hangs leaves on buildings newly dressed and takes them off those no longer dressed.
#[allow(clippy::too_many_arguments)]
fn hang_leaves(
    mut commands: Commands,
    dressed: Res<Dressed>,
    plan: Res<Plan>,
    live: Res<Live>,
    seed: Res<Seed>,
    data: Res<crate::sim::Data>,
    mut drawn: ResMut<DrawnDoors>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut looks: LeafLooks,
) {
    let ids = dressed.ids();
    let gone: Vec<u32> = drawn.0.keys().copied().filter(|b| ids.binary_search(b).is_err()).collect();
    for b in gone {
        if let Some(d) = drawn.0.remove(&b) {
            for (e, mesh) in d.leaves {
                commands.entity(e).despawn();
                meshes.remove(&mesh);
            }
        }
    }
    let icfg = &data.0.world.interiors;
    let rng = crate::buildings::building_rng(seed.0);
    for b in ids {
        if drawn.0.contains_key(&b) {
            continue;
        }
        let c = &live.structures.buildings[b as usize];
        let site = Site::of(&plan.0, b, c.n, c.origin, c.size);
        let e = interior::shell(&site, icfg, &rng).entrance;
        let door = StreetDoor::new(&site, &e, icfg);
        let (mat, tint) = dressing::leaf_finish(b, c.n, c.size, &rng, e.kind);
        let mut leaves = Vec::new();
        for (i, l) in door.leaves.iter().enumerate() {
            let size = Vec3::new(l.width_m, door.y1 - door.y0, LEAF_T_M);
            let Some(leaf) = looks.spawn(&mut commands, &mut meshes, mat, tint, size, format!("building {b} door leaf {i}")) else {
                // The building looks are not loaded yet: try again next frame.
                for (e, mesh) in leaves {
                    commands.entity(e).despawn();
                    meshes.remove(&mesh);
                }
                return;
            };
            leaves.push(leaf);
        }
        drawn.0.insert(b, Drawn { door, leaves });
    }
}

/// The rotation about +y that turns local +x to `dir` (world x, z).
fn yaw_to(dir: Vec2) -> Quat {
    Quat::from_rotation_y((-dir.y).atan2(dir.x))
}

/// Turns each drawn leaf to its door's angle now, and hides those whose bay has fallen.
fn swing_leaves(
    drawn: Res<DrawnDoors>,
    colliders: Res<InteriorColliders>,
    live: Res<Live>,
    fixed: Res<Time<Fixed>>,
    mut leaves: Query<(&mut Transform, &mut Visibility)>,
) {
    let alpha = fixed.overstep_fraction();
    for (b, d) in &drawn.0 {
        let built = colliders.0.buildings.get(b).map(|w| &w.door);
        let angle = built.map_or(0.0, |s| s.prev_angle_deg + (s.angle_deg - s.prev_angle_deg) * alpha);
        let frame = live.structures.buildings[*b as usize].frame.as_deref();
        let hangs = live.structures.buildings[*b as usize].standing > 0 && Walkable::holds(d.door.hold, false, frame);
        for (i, (e, _)) in d.leaves.iter().enumerate() {
            let Ok((mut t, mut v)) = leaves.get_mut(*e) else { continue };
            let hinge = d.door.leaves[i].hinge;
            let want = Transform::from_translation(Vec3::new(hinge.x, d.door.y0, hinge.y)).with_rotation(yaw_to(d.door.leaf_dir(i, angle)));
            if *t != want {
                *t = want;
            }
            let show = if hangs { Visibility::Inherited } else { Visibility::Hidden };
            if *v != show {
                *v = show;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leaf_turns_to_lie_along_its_direction() {
        for dir in [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y, Vec2::new(0.6, 0.8)] {
            let x = yaw_to(dir) * Vec3::X;
            assert!((Vec2::new(x.x, x.z) - dir).length() < 1e-5, "{dir:?} -> {x:?}");
            assert!(x.y.abs() < 1e-6, "stays upright");
        }
    }
}
