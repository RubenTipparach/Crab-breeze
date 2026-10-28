//! The cockpit drawn in the mech's own frame (openspec change `cockpit-local-space` section 1). The cabin is the mech's
//! own meshes, thousands of metres out in the world, where a 32-bit float steps 0.5 to 1 mm: drawn there, the eye and
//! the cabin rounded apart and the cabin shimmered a pixel or two (owner, 2026-10-28: "is cockpit drawn in local camera
//! space?", "fix it"). While the pilot sits sealed in, each cabin mesh and lamp (on `CABIN_SOURCE_LAYER`, which no
//! camera draws) has a stand-in on the cockpit layer sharing its mesh and material, placed by the chain of local
//! transforms from the part up to the mech's root (`mech::chain_steady`), never through a world position; the cockpit
//! camera sits in the same frame, at the seat's eye, turned as the view is. Every number the cockpit pass draws with is
//! a few metres. The parts themselves stay where they are, for clicks, hit tests and everything else that reads them.
//!
//! A stand-in carries only what draws (its name, mesh, `StandardMaterial`, visibility, the no-shadow flag, or its
//! light), so no game rule finds it; its mesh, material and visibility follow its part each frame. The scope's window,
//! placed in the world, is carried into the frame too, and the cockpit's environment light (the world-aligned cube of
//! the surroundings) is turned by the mech's heading. `CB_COCKPIT_WORLD=1` draws the cabin in the world as before (an
//! instrument, to compare).

use crate::mech::{MechRig, MechSim, Piloted};
use crate::pilot::{COCKPIT_LAYER, CockpitCamera, PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use bevy::camera::visibility::RenderLayers;
use bevy::light::{GeneratedEnvironmentMapLight, NotShadowCaster};
use bevy::prelude::*;
use cb_core::pilot::PilotMode;
use std::collections::BTreeMap;

/// The layer the cabin's parts and lamps are on while they feed the stand-ins: drawn by no camera.
pub const CABIN_SOURCE_LAYER: usize = 29;

/// Whether the cockpit is drawn in the mech's frame (not `CB_COCKPIT_WORLD=1`).
pub fn on() -> bool {
    !crate::cli::switched_off("CB_COCKPIT_WORLD")
}

/// The layer `mech::cockpit_layer` puts the cabin on: the stand-ins' source, or the cockpit pass itself when the
/// cockpit is drawn in the world.
pub fn cabin_layer() -> usize {
    if on() { CABIN_SOURCE_LAYER } else { COCKPIT_LAYER }
}

/// A stand-in drawing a cabin part in the mech's frame (`StandIns` maps the part to it).
#[derive(Component)]
struct StandIn;

/// Each cabin part's stand-in, by part (ordered, so the work is in a stable order).
#[derive(Resource, Default)]
struct StandIns(BTreeMap<Entity, Entity>);

/// The mech's drawn root this frame, while the cockpit is drawn in its frame: the scope carries its window by it.
#[derive(Resource, Default)]
pub struct CockpitFrame(pub Option<Transform>);

impl CockpitFrame {
    /// A world transform in the mech's frame (the scope's window, placed in the world).
    pub fn local(&self, world: Transform) -> Option<Transform> {
        let root = self.0?;
        let inv = Transform::from_matrix(root.to_matrix().inverse());
        Some(inv.mul_transform(world))
    }
}

/// The cockpit in the mech's frame.
pub struct CockpitSpacePlugin;

impl Plugin for CockpitSpacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StandIns>().init_resource::<CockpitFrame>();
        if !on() {
            return;
        }
        app.add_game_systems(
            Update,
            (keep_stand_ins, place_stand_ins, place_cockpit_camera)
                .chain()
                .after(PresentSet::Camera)
                .after(crate::mech::cockpit_layer),
        );
    }
}

/// The piloted mech, if the pilot sits sealed in it.
fn sealed_in<'a>(
    pilot: &PilotSim,
    data: &Data,
    mech: &'a Query<(Entity, &MechSim, &MechRig, &Transform), (With<Piloted>, Without<StandIn>, Without<CockpitCamera>)>,
) -> Option<(Entity, &'a MechRig, Transform)> {
    let (root, sim, rig, at) = mech.iter().next()?;
    (pilot.curr.mode == PilotMode::Seated && sim.hatch.is_closed(&data.0.boarding)).then_some((root, rig, *at))
}

/// Gives each cabin part on the source layer its stand-in, and takes away the stand-ins of parts gone from it.
#[allow(clippy::type_complexity)]
fn keep_stand_ins(
    mut commands: Commands,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mech: Query<(Entity, &MechSim, &MechRig, &Transform), (With<Piloted>, Without<StandIn>, Without<CockpitCamera>)>,
    children: Query<&Children>,
    layers: Query<&RenderLayers, Or<(With<Mesh3d>, With<PointLight>, With<SpotLight>)>>,
    mut ins: ResMut<StandIns>,
    mut frame: ResMut<CockpitFrame>,
) {
    let source = RenderLayers::layer(CABIN_SOURCE_LAYER);
    let mut want = Vec::new();
    if let Some((root, _, at)) = sealed_in(&pilot, &data, &mech) {
        frame.0 = Some(at);
        want = children
            .iter_descendants(root)
            .filter(|e| layers.get(*e).is_ok_and(|l| *l == source))
            .collect();
    } else {
        frame.0 = None;
    }
    want.sort();
    let gone: Vec<Entity> = ins.0.keys().filter(|e| want.binary_search(e).is_err()).copied().collect();
    for part in gone {
        if let Some(s) = ins.0.remove(&part) {
            commands.entity(s).try_despawn();
        }
    }
    for part in want {
        if ins.0.contains_key(&part) {
            continue;
        }
        let s = commands
            .entity(part)
            .clone_and_spawn_with_opt_in(|b| {
                b.allow::<(Name, Mesh3d, MeshMaterial3d<StandardMaterial>, Visibility, NotShadowCaster)>()
                    .allow::<(PointLight, SpotLight)>();
            })
            .insert((StandIn, RenderLayers::layer(COCKPIT_LAYER), Transform::IDENTITY))
            .id();
        ins.0.insert(part, s);
    }
}

/// Places each stand-in by its part's chain of local transforms to the mech's root, and keeps its mesh, material and
/// visibility its part's.
#[allow(clippy::type_complexity)]
fn place_stand_ins(
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mech: Query<(Entity, &MechSim, &MechRig, &Transform), (With<Piloted>, Without<StandIn>, Without<CockpitCamera>)>,
    transforms: Query<&Transform, (Without<PilotCamera>, Without<StandIn>, Without<CockpitCamera>)>,
    parents: Query<&ChildOf>,
    parts: Query<(&Visibility, Option<&Mesh3d>, Option<&MeshMaterial3d<StandardMaterial>>), Without<StandIn>>,
    ins: Res<StandIns>,
    mut drawn: Query<
        (
            &mut Transform,
            &mut Visibility,
            Option<&mut Mesh3d>,
            Option<&mut MeshMaterial3d<StandardMaterial>>,
        ),
        (With<StandIn>, Without<PilotCamera>, Without<CockpitCamera>),
    >,
) {
    let Some((root, rig, _)) = sealed_in(&pilot, &data, &mech) else {
        return;
    };
    for (&part, &s) in &ins.0 {
        let (Some(at), Ok((vis, mesh, material)), Ok((mut t, mut v, m, mat))) = (
            crate::mech::chain_steady(rig, part, Some(root), &transforms, &parents),
            parts.get(part),
            drawn.get_mut(s),
        ) else {
            continue;
        };
        // Written only when they change, so a still part is sent to the GPU once.
        t.set_if_neq(at);
        v.set_if_neq(*vis);
        if let (Some(mut m), Some(mesh)) = (m, mesh) {
            m.set_if_neq(mesh.clone());
        }
        if let (Some(mut mat), Some(material)) = (mat, material) {
            mat.set_if_neq(material.clone());
        }
    }
}

/// The cockpit camera in the mech's frame while sealed in: at the seat's eye, turned as the view is with the mech's
/// heading taken off; the environment light turned by the heading, so the world-aligned cube lights the cabin from
/// the world's sides. Otherwise back on the eye, as before.
#[allow(clippy::type_complexity)]
fn place_cockpit_camera(
    mut commands: Commands,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mech: Query<(Entity, &MechSim, &MechRig, &Transform), (With<Piloted>, Without<StandIn>, Without<CockpitCamera>)>,
    transforms: Query<&Transform, (Without<PilotCamera>, Without<StandIn>, Without<CockpitCamera>)>,
    parents: Query<&ChildOf>,
    eye: Single<(Entity, &Transform), With<PilotCamera>>,
    mut cockpit: Single<
        (Entity, &mut Transform, Option<&ChildOf>, Option<&mut GeneratedEnvironmentMapLight>),
        (With<CockpitCamera>, Without<PilotCamera>, Without<StandIn>),
    >,
) {
    let (eye_e, eye_at) = *eye;
    let (cam, ref mut at, child_of, ref mut env) = *cockpit;
    let seat = sealed_in(&pilot, &data, &mech).and_then(|(root, rig, root_at)| {
        let s = rig.socket("EYE_pilot")?;
        let local = crate::mech::chain_steady(rig, s, Some(root), &transforms, &parents)?;
        Some((local.translation, root_at.rotation))
    });
    match seat {
        Some((eye_local, heading)) => {
            if child_of.is_some() {
                commands.entity(cam).remove::<ChildOf>();
            }
            at.set_if_neq(Transform::from_translation(eye_local).with_rotation(heading.inverse() * eye_at.rotation));
            if let Some(env) = env.as_deref_mut() {
                env.rotation = heading.inverse();
            }
        }
        None => {
            if child_of.is_none_or(|c| c.parent() != eye_e) {
                commands.entity(cam).insert(ChildOf(eye_e));
            }
            at.set_if_neq(Transform::IDENTITY);
            if let Some(env) = env.as_deref_mut()
                && env.rotation != Quat::IDENTITY
            {
                env.rotation = Quat::IDENTITY;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each system's queries are disjoint (Bevy refuses overlapping mutable access when a system first runs; the first
    /// build stopped at the game's start on one).
    #[test]
    fn the_systems_queries_do_not_overlap() {
        let mut world = World::new();
        let mut a = IntoSystem::into_system(keep_stand_ins);
        a.initialize(&mut world);
        let mut b = IntoSystem::into_system(place_stand_ins);
        b.initialize(&mut world);
        let mut c = IntoSystem::into_system(place_cockpit_camera);
        c.initialize(&mut world);
    }

    #[test]
    fn a_world_transform_comes_into_the_mech_frame() {
        let root = Transform::from_xyz(-7000.0, 120.0, 3500.0).with_rotation(Quat::from_rotation_y(0.7));
        let frame = CockpitFrame(Some(root));
        let part = Transform::from_xyz(0.4, 9.0, 1.2);
        let world = root.mul_transform(part);
        let back = frame.local(world).unwrap();
        assert!(back.translation.distance(part.translation) < 2e-3, "{:?}", back.translation);
        assert_eq!(CockpitFrame(None).local(world), None);
    }
}
