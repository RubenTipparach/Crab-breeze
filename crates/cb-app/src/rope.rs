//! The boarding rope (openspec change `mech-boarding`, design section 8): with the hatch open, a rope
//! drops from the sill of the knelt mech's chest to the ground, and reels back up when the hatch
//! closes. The pilot climbs it (`cb_core::pilot`'s boarding path runs up the rope's line); this
//! module only draws it, from the same published access the climb uses (owner, 2026-10-24: "there
//! should be a rope dangling down for me the player to climb up").

use crate::mech::{MechAccessNow, ROPE_OUT_M};
use crate::sim::PresentSet;
use bevy::prelude::*;

/// Rope thickness, metres.
const RADIUS_M: f32 = 0.04;
/// The rope drops, and reels up, at this many metres a second.
const DROP_M_S: f32 = 9.0;
/// The rope's top sits this far above the sill, on its winch.
const TOP_ABOVE_SILL_M: f32 = 0.4;

/// The rope, and how much of it is paid out.
#[derive(Component, Default)]
struct Rope {
    out_m: f32,
}

/// Draws the boarding rope.
pub struct RopePlugin;

impl Plugin for RopePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(Update, hang.in_set(PresentSet::Pose));
    }
}

fn spawn(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Name::new("boarding rope"),
        Rope::default(),
        // A unit-tall rope, stretched to its length each frame.
        Mesh3d(meshes.add(Cylinder::new(RADIUS_M, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.46, 0.38, 0.24),
            perceptual_roughness: 0.92,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
    ));
}

/// Pays the rope out while the hatch is open and reels it in when it closes, hanging straight down
/// from just in front of the sill, with a slow sway.
fn hang(access: Res<MechAccessNow>, time: Res<Time>, rope: Single<(&mut Rope, &mut Transform, &mut Visibility)>) {
    let (mut rope, mut t, mut vis) = rope.into_inner();
    let Some(a) = access.0.as_ref() else {
        *vis = Visibility::Hidden;
        return;
    };
    let out = Vec2::new(a.ladder_base.x - a.hatch_entry.x, a.ladder_base.z - a.hatch_entry.z).normalize_or(Vec2::Y);
    let top = Vec3::new(
        a.hatch_entry.x + out.x * ROPE_OUT_M,
        a.hatch_entry.y + TOP_ABOVE_SILL_M,
        a.hatch_entry.z + out.y * ROPE_OUT_M,
    );
    let full = (top.y - a.ladder_base.y).max(0.0);
    let want = if a.hatch_open { full } else { 0.0 };
    let step = DROP_M_S * time.delta_secs();
    rope.out_m = (rope.out_m + (want - rope.out_m).clamp(-step, step)).min(full);
    if rope.out_m < 0.05 {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    // A slow sway about the top, at most about half a degree.
    let s = time.elapsed_secs();
    let sway = Quat::from_rotation_x(0.008 * (s * 0.7).sin()) * Quat::from_rotation_z(0.006 * (s * 0.53).sin());
    t.rotation = sway;
    t.scale = Vec3::new(1.0, rope.out_m, 1.0);
    t.translation = top + sway * (Vec3::NEG_Y * rope.out_m * 0.5);
}
