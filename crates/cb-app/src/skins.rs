//! Enemy skins (openspec change `blockout-enemies`): a block-out model worn over the stand-in's skeleton. The stand-in
//! still walks, aims, takes hits and loses parts; its boxes are not built and the skin is drawn instead. Each skin has
//! one node per leg and arm at its root joint (`LEG_<pair><L|R>`, `ARM_L`, `ARM_R`): they swing from the stand-in's own
//! gait phase, so the skin walks in step with the mech it rides, and a lost arm or leg throws its limb off whole.
//! Presentation only: the core decides nothing here.

use crate::mech::MechSim;
use crate::sim::PresentSet;
use crate::state::GameApp;
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
    /// For a walking leg: the foot's offset from the hip across the travel (in the skin's frame, metres before the
    /// skin's scale), and the axis it swings about to carry the foot fore and aft. None for an arm or a track.
    foot: Option<(Vec3, Vec3)>,
    /// The node it hangs from, and where in that node's frame.
    parent: Entity,
    pivot: Vec3,
    off: bool,
}

/// A worn skin's limbs.
#[derive(Component)]
pub struct SkinLimbs {
    limbs: Vec<Limb>,
    /// The skin's scene root (dipped as the legs swing, so the planted foot keeps the ground).
    scene: Entity,
    /// The skin's scale.
    scale: f32,
}

/// How far a stepping leg lifts out to the side at the top of its swing, radians.
const LIFT_RAD: f32 = 0.16;
/// The share of a cycle each foot stands on the ground.
const DUTY: f32 = 0.55;
/// The widest a leg swings either side of straight, radians (a stride longer than this allows is clipped).
const SWING_MAX_RAD: f32 = 0.6;
/// How far an arm or claw pitches as it walks, radians.
const ARM_RAD: f32 = 0.18;
/// The skin's forward, in its own (glTF) frame.
const FORWARD: Vec3 = Vec3::Z;

/// Hangs and walks the skins.
pub struct SkinsPlugin;

impl Plugin for SkinsPlugin {
    fn build(&self, app: &mut App) {
        app.add_game_systems(Update, (hang_skins, bind_skins, walk_skins).chain().after(PresentSet::Pose));
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
    parents: Query<&ChildOf>,
) {
    for (root, p) in &pending {
        let mut any = false;
        let mut limbs = Vec::new();
        for e in children.iter_descendants(p.0) {
            any = true;
            let Some((leg, pair, side)) = names.get(e).ok().and_then(|n| limb_of(n.as_str())) else {
                continue;
            };
            let Ok(t) = transforms.get(e) else { continue };
            // The leg's `<name>_foot` child: where it meets the ground (none on a track, which does not walk).
            let foot = children
                .get(e)
                .into_iter()
                .flatten()
                .find(|c| names.get(**c).is_ok_and(|n| n.as_str().ends_with("_foot")))
                .and_then(|c| transforms.get(*c).ok())
                .and_then(|f| swing_axis(t.rotation * f.translation));
            limbs.push(Limb {
                entity: e,
                leg,
                pair,
                side,
                rest: t.rotation,
                foot,
                parent: parents.get(e).map_or(p.0, |c| c.parent()),
                pivot: t.translation,
                off: false,
            });
        }
        if !any {
            continue; // The scene has not spawned yet.
        }
        let scale = transforms.get(p.0).map(|t| t.scale.y).unwrap_or(1.0);
        info!("enemy skin bound: {} limbs", limbs.len());
        commands
            .entity(root)
            .remove::<PendingSkin>()
            .insert(SkinLimbs { limbs, scene: p.0, scale });
    }
}

/// The offset of a hip to its foot `d` read for the walk: the part across the travel, and the axis a swing turns it
/// about to carry the foot forward (a leg hanging down swings about its side axis, a crab's leg splayed out to the
/// side about the vertical). None when the foot lies along the travel (no swing would move it fore and aft).
fn swing_axis(d: Vec3) -> Option<(Vec3, Vec3)> {
    let across = d - FORWARD * d.dot(FORWARD);
    let axis = across.cross(FORWARD).try_normalize()?;
    Some((across, axis))
}

/// A leg's swing at phase `u` of its cycle (0 its foot is put down) for a half sweep of `a` radians: the angle about
/// its swing axis (+ carries the foot forward) and how far up its step (0 planted .. 1 at the top). On the ground the
/// angle falls evenly from +a to -a while the body passes over the foot; in the air it comes back, eased.
pub fn leg_phase(u: f32, a: f32) -> (f32, f32) {
    let u = u.rem_euclid(1.0);
    if u < DUTY {
        (a * (1.0 - 2.0 * u / DUTY), 0.0)
    } else {
        let w = (u - DUTY) / (1.0 - DUTY);
        let e = w * w * (3.0 - 2.0 * w);
        (-a + 2.0 * a * e, (std::f32::consts::PI * w).sin())
    }
}

/// The half sweep, radians, that keeps a foot `reach_m` from its hip's swing axis planted while the body travels
/// `stride_m` a cycle: on the ground for `DUTY` of it, the foot passes from `+a` to `-a`, `2 r sin a` metres.
pub fn half_sweep(stride_m: f32, reach_m: f32) -> f32 {
    if reach_m <= 0.0 || !stride_m.is_finite() {
        return 0.0;
    }
    (stride_m.max(0.0) * DUTY / (2.0 * reach_m)).clamp(0.0, SWING_MAX_RAD.sin()).asin()
}

/// The phase of a limb's group in a cycle: the two sides half a cycle apart, each pair a quarter on from the pair
/// before, so a crab's legs ripple; the arms walk with the first pair's opposite side.
fn group(l: &Limb) -> f32 {
    let pair = if l.leg { f32::from(l.pair) } else { 0.0 };
    let side = if l.side > 0.0 { 0.0 } else { 0.5 };
    side + pair * 0.25 + if l.leg { 0.0 } else { 0.5 }
}

fn walk_skins(skins: Query<(&MechSim, &SkinLimbs)>, mut transforms: Query<&mut Transform>) {
    for (sim, skin) in &skins {
        let cycles = sim.curr.gait_cycles.rem_euclid(1.0) as f32;
        let amp = if sim.dead { 0.0 } else { sim.curr.gait_swing.clamp(0.0, 1.0) };
        let mut dip = 0.0_f32;
        for l in skin.limbs.iter().filter(|l| !l.off) {
            let Ok(mut t) = transforms.get_mut(l.entity) else { continue };
            let u = cycles + group(l);
            if !l.leg {
                let lift = (u * std::f32::consts::TAU).sin().max(0.0) * amp;
                t.rotation = l.rest * Quat::from_rotation_x(-lift * ARM_RAD);
                continue;
            }
            let Some((across, axis)) = l.foot else { continue };
            let a = half_sweep(sim.curr.stride_m, across.length() * skin.scale) * amp;
            let (angle, up) = leg_phase(u, a);
            let turn = Quat::from_rotation_z(l.side * up * amp * LIFT_RAD) * Quat::from_axis_angle(axis, angle);
            t.rotation = turn * l.rest;
            if up == 0.0 {
                // A planted foot swung away from straight rises off the ground: the body comes down by as much.
                dip = dip.max((turn * across - across).y * skin.scale);
            }
        }
        if let Ok(mut t) = transforms.get_mut(skin.scene) {
            t.translation.y = -dip;
        }
    }
}

/// A limb thrown off a skin: the limb's entity, the node it hung from and where (that node's frame), and the skin's
/// scale.
pub struct Thrown {
    /// The limb, now a free body.
    pub limb: Entity,
    /// The node it hung from.
    pub parent: Entity,
    /// Where it hung, in `parent`'s frame.
    pub pivot: Vec3,
    /// The skin's scale.
    pub scale: f32,
}

impl SkinLimbs {
    /// The skin node a part of `kind` is drawn by: its arm, the lowest leg left on its side, or the skin's root for the
    /// torso and head.
    pub fn node_for(&self, kind: PartKind) -> Entity {
        let (leg, side) = match kind {
            PartKind::ArmL => (false, 1.0),
            PartKind::ArmR => (false, -1.0),
            PartKind::LegL => (true, 1.0),
            PartKind::LegR => (true, -1.0),
            _ => return self.scene,
        };
        self.limbs
            .iter()
            .filter(|l| !l.off && l.leg == leg && l.side == side)
            .min_by_key(|l| l.pair)
            .map_or(self.scene, |l| l.entity)
    }

    /// The skin's scene root.
    pub fn scene(&self) -> Entity {
        self.scene
    }

    /// The skin's scale.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Its hips' height in the skin's frame (the highest leg pivot), metres before the scale.
    pub fn hip_height(&self) -> f32 {
        self.limbs.iter().filter(|l| l.leg).map(|l| l.pivot.y).fold(0.0, f32::max)
    }
}

/// Throws off the skin limb a lost part takes (an arm its arm; a leg the lowest numbered leg left on its side) as a
/// dynamic body with the cut's box, moving out and up from `centre`. Returns what was thrown, if anything.
pub fn throw_limb(
    commands: &mut Commands,
    skin: &mut SkinLimbs,
    kind: PartKind,
    globals: &Query<&GlobalTransform>,
    centre: Vec3,
    cut_box_m: [f32; 3],
) -> Option<Thrown> {
    let (leg, side) = match kind {
        PartKind::ArmL => (false, 1.0),
        PartKind::ArmR => (false, -1.0),
        PartKind::LegL => (true, 1.0),
        PartKind::LegR => (true, -1.0),
        _ => return None,
    };
    let l = skin
        .limbs
        .iter_mut()
        .filter(|l| !l.off && l.leg == leg && l.side == side)
        .min_by_key(|l| l.pair)?;
    l.off = true;
    let g = globals.get(l.entity).ok()?;
    let at = g.translation();
    let out = (at - centre).with_y(0.0).normalize_or(Vec3::X);
    commands.entity(l.entity).remove::<ChildOf>().insert((
        g.compute_transform(),
        RigidBody::Dynamic,
        Collider::cuboid(cut_box_m[0], cut_box_m[1], cut_box_m[2]),
        LinearVelocity(out * 6.0 + Vec3::Y * 5.0),
        AngularVelocity(Vec3::new(1.5, 0.7, -1.1)),
    ));
    Some(Thrown {
        limb: l.entity,
        parent: l.parent,
        pivot: l.pivot,
        scale: skin.scale,
    })
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
    fn the_two_sides_of_a_pair_step_half_a_cycle_apart_and_pairs_ripple() {
        let l = |pair, side| Limb {
            entity: Entity::PLACEHOLDER,
            leg: true,
            pair,
            side,
            rest: Quat::IDENTITY,
            foot: None,
            parent: Entity::PLACEHOLDER,
            pivot: Vec3::ZERO,
            off: false,
        };
        assert_eq!(group(&l(0, -1.0)) - group(&l(0, 1.0)), 0.5);
        assert_eq!(group(&l(1, 1.0)) - group(&l(0, 1.0)), 0.25);
    }

    #[test]
    fn a_hanging_leg_swings_about_its_side_axis_and_a_splayed_leg_about_the_vertical() {
        let (_, hang) = swing_axis(Vec3::new(0.0, -10.0, 0.5)).unwrap();
        assert!(hang.abs().abs_diff_eq(Vec3::X, 1e-5), "{hang}");
        let (_, splay) = swing_axis(Vec3::new(8.0, -2.0, 0.0)).unwrap();
        assert!(splay.y.abs() > 0.9, "{splay}");
        // A positive swing carries the foot forward.
        for d in [Vec3::new(0.0, -10.0, 0.0), Vec3::new(8.0, -2.0, 0.0), Vec3::new(-8.0, -2.0, 0.0)] {
            let (across, axis) = swing_axis(d).unwrap();
            assert!((Quat::from_axis_angle(axis, 0.3) * across).z > 0.0, "{d}");
        }
        assert!(swing_axis(Vec3::new(0.0, 0.0, 5.0)).is_none());
    }

    #[test]
    fn a_planted_foot_travels_back_exactly_as_far_as_the_body_goes_forward() {
        let (stride, reach) = (13.0, 12.0);
        let a = half_sweep(stride, reach);
        // Foot fore-aft offset over the stance, against the body's travel over the same share of the cycle.
        let fore = |u: f32| reach * leg_phase(u, a).0.sin();
        let slid = fore(0.0) - fore(DUTY - 1e-6);
        assert!((slid - stride * DUTY).abs() < 1e-3, "{slid}");
        assert_eq!(leg_phase(0.2, a).1, 0.0, "planted feet stay down");
        assert!(leg_phase(DUTY + (1.0 - DUTY) / 2.0, a).1 > 0.99, "a stepping foot is up mid-swing");
        assert_eq!(half_sweep(stride, 0.0), 0.0);
        assert!(half_sweep(1000.0, 1.0) <= SWING_MAX_RAD + 1e-6);
    }
}
