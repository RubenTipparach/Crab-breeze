//! The boarding crane and rope (openspec change `mech-boarding`, design section 4): with the hatch
//! open, a crane arm extends from above the knelt mech's sill out past its hips and legs, then
//! lowers a rope with a stirrup to the ground. The pilot steps into the stirrup and is winched up
//! the rope, and the arm draws in to bring them to the sill; climbing out runs it backwards. When
//! the hatch closes the rope reels in and the arm retracts. The climb itself is the pilot's
//! boarding path (`cb_core::pilot`), along the same published access this module draws from, so
//! the rope is always where the eye rides (owner, 2026-10-24: "there should be a rope dangling down
//! for me the player to climb up", "extend a crane out then lower the rope", "the rope needs a little
//! triangle at the end where my foot would presumablyt go").

use crate::mech::MechAccessNow;
use crate::pilot::{PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet};
use bevy::prelude::*;
use cb_core::pilot::PilotMode;

/// Rope thickness, metres.
const ROPE_RADIUS_M: f32 = 0.04;
/// The rope pays out and reels in at this many metres a second.
const ROPE_M_S: f32 = 9.0;
/// The arm extends and retracts over this many seconds.
const BOOM_S: f32 = 1.0;
/// The arm runs this far above the eye of a pilot standing on the sill, so a hanging pilot's head
/// clears it.
const BOOM_ABOVE_EYE_M: f32 = 0.7;
/// The arm's square section, metres.
const BOOM_SECTION_M: f32 = 0.32;
/// The stirrup: a triangle this wide, hanging point up from the rope's end.
const STIRRUP_SIDE_M: f32 = 0.45;
/// The stirrup's bars, metres thick.
const STIRRUP_BAR_M: f32 = 0.025;

/// What a piece of the crane is.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Piece {
    /// The arm, from its root above the sill to its tip.
    Boom,
    /// The pulley at the arm's tip.
    Head,
    /// The rope, from the tip down.
    Rope,
    /// The stirrup at the rope's end.
    Stirrup,
}

/// How far the arm is out (0..1) and how much rope is paid out.
#[derive(Resource, Default)]
struct Crane {
    boom: f32,
    rope_m: f32,
}

/// Draws the boarding crane.
pub struct RopePlugin;

impl Plugin for RopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Crane>()
            .add_systems(Startup, spawn)
            .add_systems(Update, hang.in_set(PresentSet::Pose));
    }
}

fn spawn(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let paint = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.64, 0.12),
        metallic: 0.5,
        perceptual_roughness: 0.5,
        ..default()
    });
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.33, 0.35),
        metallic: 0.9,
        perceptual_roughness: 0.35,
        ..default()
    });
    let hemp = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.38, 0.24),
        perceptual_roughness: 0.92,
        ..default()
    });
    let hidden = (Transform::default(), Visibility::Hidden);
    // A unit-long arm along x and a unit-tall rope, stretched each frame.
    commands.spawn((
        Name::new("boarding crane arm"),
        Piece::Boom,
        Mesh3d(meshes.add(Cuboid::new(1.0, BOOM_SECTION_M, BOOM_SECTION_M))),
        MeshMaterial3d(paint),
        hidden,
    ));
    commands.spawn((
        Name::new("boarding crane pulley"),
        Piece::Head,
        Mesh3d(meshes.add(Cylinder::new(0.22, 0.16))),
        MeshMaterial3d(steel.clone()),
        hidden,
    ));
    commands.spawn((
        Name::new("boarding rope"),
        Piece::Rope,
        Mesh3d(meshes.add(Cylinder::new(ROPE_RADIUS_M, 1.0))),
        MeshMaterial3d(hemp),
        hidden,
    ));
    // The stirrup: its apex at its origin, its foot bar below, in its own x-y plane.
    let bar = meshes.add(Cylinder::new(STIRRUP_BAR_M, STIRRUP_SIDE_M));
    let h = STIRRUP_SIDE_M * 3f32.sqrt() * 0.5;
    let corners = [
        Vec3::ZERO,
        Vec3::new(-STIRRUP_SIDE_M * 0.5, -h, 0.0),
        Vec3::new(STIRRUP_SIDE_M * 0.5, -h, 0.0),
    ];
    commands
        .spawn((Name::new("boarding stirrup"), Piece::Stirrup, hidden))
        .with_children(|s| {
            for i in 0..3 {
                let (a, b) = (corners[i], corners[(i + 1) % 3]);
                s.spawn((
                    Mesh3d(bar.clone()),
                    MeshMaterial3d(steel.clone()),
                    Transform::from_translation((a + b) * 0.5).with_rotation(Quat::from_rotation_arc(Vec3::Y, (b - a).normalize())),
                ));
            }
        });
}

/// Extends the arm and pays out the rope while the hatch is open, carries the climbing pilot, and
/// reels in and retracts once it closes.
fn hang(
    access: Res<MechAccessNow>,
    data: Res<Data>,
    pilot: Res<PilotSim>,
    eye: Query<&GlobalTransform, With<PilotCamera>>,
    time: Res<Time>,
    mut crane: ResMut<Crane>,
    mut pieces: Query<(&Piece, &mut Transform, &mut Visibility)>,
) {
    let Some(a) = access.0.as_ref() else {
        for (_, _, mut v) in &mut pieces {
            *v = Visibility::Hidden;
        }
        return;
    };
    let eye_h = data.0.pilot.eye_height_m;
    let flat = Vec2::new(a.ladder_base.x - a.hatch_entry.x, a.ladder_base.z - a.hatch_entry.z);
    let reach = flat.length().max(0.1);
    let out = Vec3::new(flat.x / reach, 0.0, flat.y / reach);
    let root = a.hatch_entry + Vec3::Y * (eye_h + BOOM_ABOVE_EYE_M);
    let ground = a.ladder_base.y;
    let stirrup_h = STIRRUP_SIDE_M * 3f32.sqrt() * 0.5;
    let full_rope = (root.y - ground - stirrup_h - 0.05).max(0.0);
    let dt = time.delta_secs();

    // A climbing pilot rides the rope's line: the rope ends at their feet and the arm's tip is over
    // them, so the winch hauls them up and the arm draws them in to the sill.
    let climbing = matches!(pilot.curr.mode, PilotMode::Boarding(_) | PilotMode::Disembarking(_));
    let carried = eye.iter().next().map(|g| g.translation()).filter(|e| {
        let rel = *e - a.hatch_entry;
        let along = rel.x * out.x + rel.z * out.z;
        let across = (Vec2::new(rel.x, rel.z) - Vec2::new(out.x, out.z) * along).length();
        climbing && across < 0.6 && along > 0.2 && e.y > ground + eye_h + 0.3
    });
    if let Some(e) = carried {
        let rel = e - a.hatch_entry;
        let along = (rel.x * out.x + rel.z * out.z).clamp(0.2, reach);
        crane.boom = along / reach;
        crane.rope_m = (root.y - (e.y - eye_h) - stirrup_h).clamp(0.0, full_rope);
    } else {
        // The arm goes out once the hatch is open; the rope drops once the arm is out. Closing, the
        // rope comes up first, then the arm goes in.
        let boom_target = if a.hatch_open {
            1.0
        } else if crane.rope_m > 0.05 {
            crane.boom
        } else {
            0.0
        };
        let step = dt / BOOM_S;
        crane.boom += (boom_target - crane.boom).clamp(-step, step);
        let rope_target = if a.hatch_open && crane.boom > 0.999 { full_rope } else { 0.0 };
        let step = ROPE_M_S * dt;
        crane.rope_m = (crane.rope_m + (rope_target - crane.rope_m).clamp(-step, step)).min(full_rope);
    }

    let shown = crane.boom > 0.01;
    let tip = root + out * reach * crane.boom;
    let roped = shown && crane.rope_m > 0.05;
    // A slow sway about the tip while it hangs free, under half a degree.
    let s = time.elapsed_secs();
    let sway = if carried.is_some() {
        Quat::IDENTITY
    } else {
        Quat::from_rotation_x(0.008 * (s * 0.7).sin()) * Quat::from_rotation_z(0.006 * (s * 0.53).sin())
    };
    let rope_top = tip - Vec3::Y * 0.2;
    let rope_end = rope_top + sway * (Vec3::NEG_Y * crane.rope_m);
    for (piece, mut t, mut v) in &mut pieces {
        let (show, transform) = match piece {
            Piece::Boom => {
                let len = reach * crane.boom + BOOM_SECTION_M;
                let mid = root + out * (len * 0.5 - BOOM_SECTION_M * 0.5);
                (
                    shown,
                    Transform::from_translation(mid)
                        .with_rotation(Quat::from_rotation_arc(Vec3::X, out))
                        .with_scale(Vec3::new(len, 1.0, 1.0)),
                )
            }
            Piece::Head => (
                shown,
                Transform::from_translation(tip).with_rotation(Quat::from_rotation_arc(Vec3::Y, out.cross(Vec3::Y).normalize_or(Vec3::X))),
            ),
            Piece::Rope => (
                roped,
                Transform::from_translation((rope_top + rope_end) * 0.5)
                    .with_rotation(sway)
                    .with_scale(Vec3::new(1.0, crane.rope_m, 1.0)),
            ),
            Piece::Stirrup => (
                shown,
                Transform::from_translation(if roped { rope_end } else { rope_top })
                    .with_rotation(Quat::from_rotation_arc(Vec3::Z, out) * sway),
            ),
        };
        *t = transform;
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
    }
}
