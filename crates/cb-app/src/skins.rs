//! Enemy skins (openspec change `blockout-enemies`): a block-out model worn over the stand-in's skeleton. The stand-in
//! still walks, aims, takes hits and loses parts; its boxes are not built and the skin is drawn instead. Each skin has
//! one node per leg and arm at its root joint (`LEG_<pair><L|R>`, `ARM_L`, `ARM_R`): they swing from the stand-in's own
//! gait phase, so the skin walks in step with the mech it rides, and a lost arm or leg throws its limb off whole.
//! Presentation only: the core decides nothing here.

use crate::mech::MechSim;
use crate::sim::PresentSet;
use avian3d::prelude::{AngularVelocity, Collider, LinearVelocity, RigidBody};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use cb_core::mech::damage::PartKind;
use cb_core::skins::SkinSpec;

/// A skin to put on, until the mech's model node is there to hang it on.
#[derive(Component)]
pub struct SkinToWear {
    scene: Handle<bevy::world_serialization::WorldAsset>,
    scale: f32,
}

/// A skin's scene, spawned under the mech's model node, until its limbs are found.
#[derive(Component)]
pub struct PendingSkin(pub Entity);

/// One limb of a skin.
#[derive(Clone, Copy)]
struct Limb {
    entity: Entity,
    leg: bool,
    pair: u8,
    /// +1 the mech's left (+x in glTF space), -1 its right.
    side: f32,
    rest: Quat,
    off: bool,
}

/// A worn skin's limbs.
#[derive(Component)]
pub struct SkinLimbs {
    limbs: Vec<Limb>,
    pairs: f32,
}

/// How far a stepping leg lifts about its own forward axis, and swings about the vertical, radians.
const LIFT_RAD: f32 = 0.22;
const SWING_RAD: f32 = 0.16;
/// How far an arm or claw pitches as it walks, radians.
const ARM_RAD: f32 = 0.18;

/// Hangs and walks the skins.
pub struct SkinsPlugin;

impl Plugin for SkinsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (hang_skins, bind_skins, walk_skins).chain().after(PresentSet::Pose));
    }
}

/// Marks the mech `root` to wear `skin` (its scene is hung on the model node next frame).
pub fn wear(commands: &mut Commands, assets: &AssetServer, root: Entity, skin: &SkinSpec) {
    let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(skin.model.clone()));
    commands.entity(root).insert(SkinToWear { scene, scale: skin.scale });
}

/// Spawns each skin's scene under its mech's model node (glTF space, the stand-in's facing).
fn hang_skins(mut commands: Commands, roots: Query<(Entity, &crate::mech::MechRoot, &SkinToWear)>) {
    for (root, mr, s) in &roots {
        let e = commands
            .spawn((
                Name::new("enemy skin"),
                WorldAssetRoot(s.scene.clone()),
                Transform::from_scale(Vec3::splat(s.scale)),
                Visibility::default(),
                ChildOf(mr.model),
            ))
            .id();
        commands.entity(root).remove::<SkinToWear>().insert(PendingSkin(e));
    }
}

/// A limb node's name read: (is a leg, pair, side), or None for any other node.
fn limb_of(name: &str) -> Option<(bool, u8, f32)> {
    let (leg, rest) = if let Some(r) = name.strip_prefix("LEG_") {
        (true, r)
    } else {
        (false, name.strip_prefix("ARM_")?)
    };
    let side = match rest.chars().last()? {
        'L' => 1.0,
        'R' => -1.0,
        _ => return None,
    };
    let pair = if leg { rest[..rest.len() - 1].parse().ok()? } else { 0 };
    Some((leg, pair, side))
}

fn bind_skins(
    mut commands: Commands,
    pending: Query<(Entity, &PendingSkin)>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
) {
    for (root, p) in &pending {
        let mut any = false;
        let mut limbs = Vec::new();
        for e in children.iter_descendants(p.0) {
            any = true;
            let Some((leg, pair, side)) = names.get(e).ok().and_then(|n| limb_of(n.as_str())) else {
                continue;
            };
            let rest = transforms.get(e).map(|t| t.rotation).unwrap_or_default();
            limbs.push(Limb {
                entity: e,
                leg,
                pair,
                side,
                rest,
                off: false,
            });
        }
        if !any {
            continue; // The scene has not spawned yet.
        }
        let legs = limbs.iter().filter(|l| l.leg).count() as f32;
        info!("enemy skin bound: {} limbs", limbs.len());
        commands.entity(root).remove::<PendingSkin>().insert(SkinLimbs {
            limbs,
            pairs: (legs / 2.0).max(1.0),
        });
    }
}

/// The phase offset of a limb's group: legs alternate by pair and side; the arms walk as the pair before the first.
fn group(l: &Limb) -> f32 {
    let pair = if l.leg { f32::from(l.pair) + 1.0 } else { 0.0 };
    pair + if l.side > 0.0 { 0.0 } else { 1.0 }
}

fn walk_skins(skins: Query<(&MechSim, &SkinLimbs)>, mut transforms: Query<&mut Transform>) {
    for (sim, skin) in &skins {
        let cycles = sim.curr.gait_cycles as f32;
        let amp = if sim.dead { 0.0 } else { sim.curr.gait_swing.clamp(0.0, 1.0) };
        for l in skin.limbs.iter().filter(|l| !l.off) {
            let Ok(mut t) = transforms.get_mut(l.entity) else { continue };
            let ph = cycles * std::f32::consts::TAU + group(l) * std::f32::consts::PI;
            let lift = ph.sin().max(0.0) * amp;
            t.rotation = if l.leg {
                l.rest
                    * Quat::from_rotation_z(l.side * lift * LIFT_RAD)
                    * Quat::from_rotation_y(-l.side * ph.cos() * amp * SWING_RAD / skin.pairs.sqrt())
            } else {
                l.rest * Quat::from_rotation_x(-lift * ARM_RAD)
            };
        }
    }
}

/// Throws off the skin limb a lost part takes (an arm its arm; a leg the lowest numbered leg left on its side) as a
/// dynamic body with the cut's box, moving out and up from `centre`. Returns whether a limb was thrown.
pub fn throw_limb(
    commands: &mut Commands,
    skin: &mut SkinLimbs,
    kind: PartKind,
    globals: &Query<&GlobalTransform>,
    centre: Vec3,
    cut_box_m: [f32; 3],
) -> bool {
    let (leg, side) = match kind {
        PartKind::ArmL => (false, 1.0),
        PartKind::ArmR => (false, -1.0),
        PartKind::LegL => (true, 1.0),
        PartKind::LegR => (true, -1.0),
        _ => return false,
    };
    let Some(l) = skin
        .limbs
        .iter_mut()
        .filter(|l| !l.off && l.leg == leg && l.side == side)
        .min_by_key(|l| l.pair)
    else {
        return false;
    };
    l.off = true;
    let Ok(g) = globals.get(l.entity) else { return false };
    let at = g.translation();
    let out = (at - centre).with_y(0.0).normalize_or(Vec3::X);
    commands.entity(l.entity).remove::<ChildOf>().insert((
        g.compute_transform(),
        RigidBody::Dynamic,
        Collider::cuboid(cut_box_m[0], cut_box_m[1], cut_box_m[2]),
        LinearVelocity(out * 6.0 + Vec3::Y * 5.0),
        AngularVelocity(Vec3::new(1.5, 0.7, -1.1)),
    ));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limb_names_read_as_leg_or_arm_with_pair_and_side() {
        assert_eq!(limb_of("LEG_0L"), Some((true, 0, 1.0)));
        assert_eq!(limb_of("LEG_2R"), Some((true, 2, -1.0)));
        assert_eq!(limb_of("ARM_R"), Some((false, 0, -1.0)));
        assert_eq!(limb_of("ARM_X"), None);
        assert_eq!(limb_of("legs"), None);
        assert_eq!(limb_of("LEG_L"), None);
    }

    #[test]
    fn the_two_sides_of_a_pair_step_half_a_cycle_apart() {
        let l = |pair, side| Limb {
            entity: Entity::PLACEHOLDER,
            leg: true,
            pair,
            side,
            rest: Quat::IDENTITY,
            off: false,
        };
        assert_eq!(group(&l(0, -1.0)) - group(&l(0, 1.0)), 1.0);
        assert_eq!(group(&l(1, 1.0)) - group(&l(0, 1.0)), 1.0);
    }
}
