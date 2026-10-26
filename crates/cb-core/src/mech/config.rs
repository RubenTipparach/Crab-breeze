//! A mech's data: its stand-in skeleton and boxes, an optional glb, and its locomotion and gait
//! tuning (`data/mechs/<name>.ron`, openspec change `mech-rig`).

use super::damage::{PartSpec, default_parts, validate_parts};
use super::rig::{BONES, SOCKETS, contract_parent};
use crate::data::{Validate, finite, finite3, positive, within};
use serde::Deserialize;

/// One node of the stand-in: a contract bone or socket, placed relative to its parent.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    /// Contract name.
    pub name: String,
    /// Parent's contract name; empty for `root`.
    #[serde(default)]
    pub parent: String,
    /// Offset from the parent at rest, in glTF space (+y up, +z forward, +x the mech's left).
    pub offset_m: [f32; 3],
}

/// A box of the stand-in's body, carried by a bone.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoxSpec {
    /// The bone that carries it.
    pub bone: String,
    /// Centre, relative to the bone.
    pub centre_m: [f32; 3],
    /// Full size. With `along_m`, the y size is replaced by the segment's length.
    pub size_m: [f32; 3],
    /// When set, the box runs from the bone's origin along this vector (its local y axis turned
    /// onto it), and `centre_m` is ignored: a limb segment.
    #[serde(default)]
    pub along_m: Option<[f32; 3]>,
    /// Paint tone index (0 hull, 1 dark, 2 joint, 3 cockpit frame, 4 glass, 5 accent).
    #[serde(default)]
    pub tone: u32,
}

/// Locomotion tuning (mech-rig design section 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LocomotionConfig {
    /// Ground speed at full throttle.
    pub walk_speed_max_m_s: f32,
    /// Lowest throttle lever setting (reverse), -1..0.
    pub throttle_min: f32,
    /// Throttle lever travel per second with the key held.
    pub throttle_rate_per_s: f32,
    /// Acceleration toward a higher speed.
    pub accel_m_s2: f32,
    /// Deceleration toward a lower speed.
    pub brake_m_s2: f32,
    /// Leg turn rate at full turn input.
    pub turn_rate_deg_s: f32,
    /// How fast the leg turn rate changes, so a 40 t mech eases into and out of a turn instead of
    /// snapping (owner, 2026-10-25: "mech should turn more smoothly").
    pub turn_accel_deg_s2: f32,
    /// Torso twist limit either side of the legs.
    pub twist_max_deg: f32,
    /// Torso twist rate.
    pub twist_rate_deg_s: f32,
    /// How fast the torso's twist speeds up and slows down: it eases into a turn and brakes onto
    /// the aim instead of starting and stopping dead (owner, 2026-10-25: "mech isnt accelerating
    /// to match to the rotation", "rotates abruptly").
    pub twist_accel_deg_s2: f32,
    /// Torso pitch limits, [down, up] (down negative).
    pub pitch_limits_deg: [f32; 2],
    /// Torso pitch rate.
    pub pitch_rate_deg_s: f32,
    /// Radius of the body for building collision, and the legs' reach against tree trunks.
    pub body_radius_m: f32,
    /// Reach of the hull and arms through tree crowns (vegetation design section 5).
    pub brush_radius_m: f32,
    /// Height from the feet to the top of the mech (survey M1: 12 m).
    pub body_height_m: f32,
    /// Below this ground speed the mech counts as stopped (boarding allowed).
    pub stopped_speed_m_s: f32,
    /// Hip height, for wading: water this deep reaches the hips.
    pub hip_height_m: f32,
    /// Deepest water the mech walks into (the cockpit floor).
    pub max_wade_depth_m: f32,
    /// Share of top speed lost with water at the hips (terrain-water design section 2).
    pub wade_slowdown: f32,
    /// Mass, for knocking things over.
    pub mass_kg: f32,
    /// Share of walking speed left with one leg (mech-damage design section 2).
    pub limp_speed_ratio: f32,
    /// The root within this far under the water surface counts as at the surface (water-skimming).
    pub float_catch_m: f32,
    /// Sinking speed through water.
    pub water_sink_m_s: f32,
    /// Fastest rise through water on the rockets.
    pub water_climb_m_s: f32,
    /// How hard water brakes the mech's motion through it (sinking, entering at speed).
    pub water_drag_m_s2: f32,
    /// The skates ride water only deeper than this share of the mech's height; shallower water
    /// is waded or walked, skates or not (owner, 2026-10-24: "The skis only kicking if you're above
    /// water deeper than 50% of the mech").
    pub skim_depth_share: f32,
    /// How fast a wading mech with its skates on rises to the surface once the water is deep enough.
    pub skim_rise_m_s: f32,
}

impl Default for LocomotionConfig {
    fn default() -> Self {
        Self {
            walk_speed_max_m_s: 7.0,
            throttle_min: -0.4,
            throttle_rate_per_s: 0.6,
            accel_m_s2: 2.5,
            brake_m_s2: 4.0,
            turn_rate_deg_s: 30.0,
            turn_accel_deg_s2: 120.0,
            twist_max_deg: 100.0,
            twist_rate_deg_s: 70.0,
            twist_accel_deg_s2: 600.0,
            pitch_limits_deg: [-20.0, 25.0],
            pitch_rate_deg_s: 40.0,
            body_radius_m: 3.0,
            brush_radius_m: 5.5,
            body_height_m: 12.0,
            stopped_speed_m_s: 0.2,
            hip_height_m: 5.9,
            max_wade_depth_m: 8.0,
            wade_slowdown: 0.6,
            mass_kg: 50_000.0,
            limp_speed_ratio: 0.22,
            float_catch_m: 1.0,
            water_sink_m_s: 4.0,
            water_climb_m_s: 10.0,
            water_drag_m_s2: 30.0,
            skim_depth_share: 0.5,
            skim_rise_m_s: 6.0,
        }
    }
}

/// Walk tuning (mech-rig design section 3).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct GaitConfig {
    /// Distance covered per full gait cycle (two steps) at top speed.
    pub stride_max_m: f32,
    /// Shortest stride, used when slow or turning in place.
    pub stride_min_m: f32,
    /// Foot lift at the top of a swing.
    pub step_height_m: f32,
    /// Pelvis drop at mid-stance.
    pub bob_m: f32,
    /// Pelvis sway toward the stance foot.
    pub sway_m: f32,
    /// Turning in place steps as if walking at turn rate times this radius.
    pub turn_step_radius_m: f32,
    /// Speed by which the stride reaches its full swing; below it the feet settle together.
    pub settle_speed_m_s: f32,
}

impl Default for GaitConfig {
    fn default() -> Self {
        Self {
            stride_max_m: 9.0,
            stride_min_m: 2.0,
            step_height_m: 0.9,
            bob_m: 0.25,
            sway_m: 0.15,
            turn_step_radius_m: 2.5,
            settle_speed_m_s: 0.6,
        }
    }
}

/// Booster tuning (openspec change `boost-and-skate`; survey M3: heat-limited, fuel ignored).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct JetConfig {
    /// Upward acceleration at full lift, before gravity.
    pub thrust_m_s2: f32,
    /// Fastest climb.
    pub climb_max_m_s: f32,
    /// Fastest fall while lifting at all.
    pub sink_max_m_s: f32,
    /// Fastest fall without the rockets.
    pub fall_max_m_s: f32,
    /// Extra downward acceleration while diving (Ctrl in the air).
    pub dive_m_s2: f32,
    /// Fastest fall while diving.
    pub dive_max_m_s: f32,
    /// Air drag on horizontal speed without thrust, per second.
    pub drag_per_s: f32,
    /// Seconds of burn from full.
    pub burn_s: f32,
    /// Seconds to recharge from empty.
    pub recharge_s: f32,
    /// Pause after burning before recharging starts.
    pub recharge_delay_s: f32,
    /// Emptied, they relight once this share is back.
    pub relight_fraction: f32,
    /// WASD swivels the rockets up to this far from straight down, toward the way asked, so they push the mech that
    /// way a little (owner, 2026-10-25: "I should move forward slightly when using my rockets, wasd shoulsd also
    /// swivel the rocket in that general direction").
    pub vector_max_deg: f32,
    /// How fast the rockets swivel, degrees a second.
    pub vector_rate_deg_s: f32,
}

impl Default for JetConfig {
    fn default() -> Self {
        Self {
            // Thrust to weight 5.7 (owner, 2026-10-24: "make rocket TWR 2x stronger").
            thrust_m_s2: 56.0,
            climb_max_m_s: 60.0,
            sink_max_m_s: 25.0,
            fall_max_m_s: 60.0,
            dive_m_s2: 30.0,
            dive_max_m_s: 90.0,
            // A 40 t mech keeps its momentum in the air; the skates are its side thrusters for moving
            // across (owner, 2026-10-25: "when flying if skate is off I should inherit inertia, think of
            // skate as lateral thrusters"). Air drag on 100 m2 at 30 m/s is about 1.3 m/s2: 4 % a second.
            drag_per_s: 0.04,
            burn_s: 20.0,
            recharge_s: 8.0,
            recharge_delay_s: 1.5,
            relight_fraction: 0.3,
            // sin(20 degrees) of 56 m/s2: about 19 m/s2 across at full lift, a third of the climb's push.
            vector_max_deg: 20.0,
            vector_rate_deg_s: 60.0,
        }
    }
}

/// Skate tuning (openspec change `boost-and-skate`; survey M4: foot wheels).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SkateConfig {
    /// Top speed at full throttle.
    pub speed_max_m_s: f32,
    /// Wheel drive from a stand; it fades as `1 - (v / speed_max)^2` toward top speed.
    pub accel_m_s2: f32,
    /// Braking when the throttle asks for less.
    pub brake_m_s2: f32,
    /// How fast the velocity swings onto the heading, per second (low: drifts).
    pub grip_per_s: f32,
    /// Turn rate.
    pub turn_rate_deg_s: f32,
    /// Seconds of use from full.
    pub burn_s: f32,
    /// Seconds to recharge from empty.
    pub recharge_s: f32,
    /// Pause after skating before recharging starts.
    pub recharge_delay_s: f32,
    /// Emptied, they work again once this share is back.
    pub relight_fraction: f32,
}

impl Default for SkateConfig {
    fn default() -> Self {
        Self {
            // Owner, 2026-10-24: "skate speed 4x faster, good acceleration curve, just takes time
            // to build speed": 55 m/s at 5 s, 109 m/s at 15 s.
            speed_max_m_s: 120.0,
            accel_m_s2: 12.0,
            brake_m_s2: 15.0,
            grip_per_s: 1.2,
            turn_rate_deg_s: 90.0,
            burn_s: 40.0,
            recharge_s: 10.0,
            recharge_delay_s: 1.0,
            relight_fraction: 0.2,
        }
    }
}

/// Ramming a building (openspec change `mech-ramming`; owner, 2026-10-24: "the mech has to hit a
/// biulding with a weapon or ram into it at a significant speed to actually caause damage").
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RamConfig {
    /// Least speed that damages a building; a slower mech is only stopped by it.
    pub speed_m_s: f32,
    /// The building must take the speed below this fraction of itself in one step.
    pub stopped_fraction: f32,
    /// The hole's radius at the ram speed.
    pub radius_m: f32,
    /// The biggest hole's radius.
    pub radius_max_m: f32,
    /// The fraction of its speed the mech keeps after smashing through.
    pub kept_fraction: f32,
    /// The hole's centre ahead of the body's front.
    pub reach_m: f32,
    /// The hole's centre over the ground.
    pub height_m: f32,
}

impl Default for RamConfig {
    fn default() -> Self {
        Self {
            // Above the placeholder's 12 m/s walk: reached in under 2 s of skating, or on jets.
            speed_m_s: 20.0,
            stopped_fraction: 0.6,
            radius_m: 4.5,
            radius_max_m: 9.0,
            kept_fraction: 0.7,
            reach_m: 2.5,
            height_m: 5.0,
        }
    }
}

/// The sizes the game reads that the rig does not carry, per mech (openspec change `mech-roster` section 3). The
/// defaults are the 12 m stand-in's, so a mech file that leaves them out flies as it always did.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BodyConfig {
    /// Height of the pilot's eye over the feet at rest: where the AI and the targeting look from.
    pub eye_height_m: f32,
    /// The torso's hit capsule, from and to this far over the `torso` bone.
    pub torso_hit_m: [f32; 2],
    /// The torso's hit capsule radius.
    pub torso_hit_radius_m: f32,
    /// An arm's hit capsule radius, shoulder to hand.
    pub arm_hit_radius_m: f32,
    /// A leg's hit capsule radius, thigh to foot.
    pub leg_hit_radius_m: f32,
    /// The head's hit capsule radius.
    pub head_hit_radius_m: f32,
    /// Height of the centre of mass over the feet, for the wreck's topple.
    pub wreck_centre_m: f32,
    /// Height over the feet of the blast when the torso goes.
    pub blast_height_m: f32,
    /// Height over the feet of the point that breaks a wall the mech strides into.
    pub smash_height_m: f32,
    /// Half extents of the physics box a shot-off part falls as.
    pub cut_box_m: [f32; 3],
}

impl Default for BodyConfig {
    fn default() -> Self {
        Self {
            eye_height_m: 9.6,
            torso_hit_m: [0.5, 3.9],
            torso_hit_radius_m: 2.4,
            arm_hit_radius_m: 1.0,
            leg_hit_radius_m: 1.1,
            head_hit_radius_m: 1.1,
            wreck_centre_m: 10.0,
            blast_height_m: 9.0,
            smash_height_m: 5.0,
            cut_box_m: [1.6, 3.0, 1.6],
        }
    }
}

impl RamConfig {
    /// The radius of the hole a mech breaks when its speed went from `before_m_s` to `after_m_s`
    /// in one step against a building: none unless it was ramming (at or above the ram speed) and
    /// the building stopped it. The hole grows as (speed / ram speed)^(2/3), the cube root of
    /// the impact energy over the threshold's, up to `radius_max_m`.
    pub fn hole_radius_m(&self, before_m_s: f32, after_m_s: f32) -> Option<f32> {
        let (before, after) = (before_m_s.abs(), after_m_s.abs());
        if !before.is_finite() || !after.is_finite() || before < self.speed_m_s || after >= before * self.stopped_fraction {
            return None;
        }
        Some((self.radius_m * (before / self.speed_m_s).powf(2.0 / 3.0)).min(self.radius_max_m))
    }
}

/// An interior part that opens on a chest joint: the two open as one body (`mech_boarding.pod`, mode "inner").
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RideSpec {
    /// The interior's node, moved with everything under it.
    pub part: String,
    /// The mech's chest joint it rides.
    pub joint: String,
}

/// The cockpit interior a mech carries in its chest (openspec change `mech-roster` section 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InteriorConfig {
    /// A glTF under `assets/`, hung on the `cockpit` bone at its origin.
    pub model: String,
    /// The interior parts that ride chest joints.
    #[serde(default)]
    pub rides: Vec<RideSpec>,
}

/// A mech: its model, its stand-in, and its tuning.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct MechConfig {
    /// Display name.
    pub name: String,
    /// Its risk tier, 1 to 4: the riskiest jobs owning it opens (openspec change `career-progression`).
    pub tier: u8,
    /// A glb under `assets/` following the rig contract (`models/mech_<name>.glb`). When set it
    /// replaces the stand-in's boxes; the stand-in skeleton is then unused.
    pub model: Option<String>,
    /// The cockpit interior hung in its chest, for a glb whose bay is empty.
    pub interior: Option<InteriorConfig>,
    /// Sizes the game reads that the rig does not carry.
    pub body: BodyConfig,
    /// Locomotion tuning.
    pub locomotion: LocomotionConfig,
    /// Walk tuning.
    pub gait: GaitConfig,
    /// Booster tuning.
    pub jet: JetConfig,
    /// Skate tuning.
    pub skate: SkateConfig,
    /// Ramming buildings.
    pub ram: RamConfig,
    /// Parts that can be shot off.
    pub parts: Vec<PartSpec>,
    /// Guns.
    pub weapons: Vec<crate::weapons::WeaponSpec>,
    /// Close combat: melee range and the arms' strikes (openspec change `close-combat`).
    pub melee: super::melee::MeleeConfig,
    /// The stand-in's bones and sockets.
    pub nodes: Vec<NodeSpec>,
    /// The stand-in's boxes.
    pub boxes: Vec<BoxSpec>,
}

impl MechConfig {
    /// Where a mech is aimed at, metres above its feet: its chest, a metre under the pilot's eyes,
    /// inside the torso's hit capsule (owner, 2026-10-25: "I'm 24 m tall, why not aim at my
    /// center?", "sure, chest is fine"). A fixed 9.8 m, set for the 12 m mechs, put the aim between
    /// the 24 m mechs' thighs, and rounds flew through.
    pub fn aim_height_m(&self) -> f32 {
        (self.body.eye_height_m - 1.0).max(0.0)
    }
}

impl Default for MechConfig {
    fn default() -> Self {
        Self {
            name: "stand-in".into(),
            tier: 1,
            model: None,
            interior: None,
            body: BodyConfig::default(),
            locomotion: LocomotionConfig::default(),
            gait: GaitConfig::default(),
            jet: JetConfig::default(),
            skate: SkateConfig::default(),
            ram: RamConfig::default(),
            parts: default_parts(),
            weapons: crate::weapons::default_weapons(),
            melee: super::melee::MeleeConfig::default(),
            nodes: Vec::new(),
            boxes: Vec::new(),
        }
    }
}

impl Validate for MechConfig {
    fn validate(&self) -> Result<(), String> {
        if !(1..=4).contains(&self.tier) {
            return Err(format!("`tier` must be 1 to 4, got {}", self.tier));
        }
        let r = &self.ram;
        positive("ram.speed_m_s", r.speed_m_s)?;
        within("ram.stopped_fraction", r.stopped_fraction, 0.0, 1.0)?;
        positive("ram.radius_m", r.radius_m)?;
        positive("ram.radius_max_m", r.radius_max_m)?;
        if r.radius_max_m < r.radius_m {
            return Err("`ram.radius_max_m` must not be less than `ram.radius_m`".into());
        }
        within("ram.kept_fraction", r.kept_fraction, 0.0, 1.0)?;
        within("ram.reach_m", r.reach_m, 0.0, 50.0)?;
        within("ram.height_m", r.height_m, 0.0, 100.0)?;
        let l = &self.locomotion;
        positive("locomotion.walk_speed_max_m_s", l.walk_speed_max_m_s)?;
        within("locomotion.throttle_min", l.throttle_min, -1.0, 0.0)?;
        positive("locomotion.throttle_rate_per_s", l.throttle_rate_per_s)?;
        positive("locomotion.accel_m_s2", l.accel_m_s2)?;
        positive("locomotion.brake_m_s2", l.brake_m_s2)?;
        positive("locomotion.turn_rate_deg_s", l.turn_rate_deg_s)?;
        positive("locomotion.turn_accel_deg_s2", l.turn_accel_deg_s2)?;
        within("locomotion.twist_max_deg", l.twist_max_deg, 0.0, 180.0)?;
        positive("locomotion.twist_rate_deg_s", l.twist_rate_deg_s)?;
        positive("locomotion.twist_accel_deg_s2", l.twist_accel_deg_s2)?;
        within("locomotion.pitch_limits_deg[0]", l.pitch_limits_deg[0], -89.0, 0.0)?;
        within("locomotion.pitch_limits_deg[1]", l.pitch_limits_deg[1], 0.0, 89.0)?;
        positive("locomotion.pitch_rate_deg_s", l.pitch_rate_deg_s)?;
        positive("locomotion.body_radius_m", l.body_radius_m)?;
        positive("locomotion.brush_radius_m", l.brush_radius_m)?;
        positive("locomotion.body_height_m", l.body_height_m)?;
        positive("locomotion.stopped_speed_m_s", l.stopped_speed_m_s)?;
        positive("locomotion.hip_height_m", l.hip_height_m)?;
        positive("locomotion.max_wade_depth_m", l.max_wade_depth_m)?;
        within("locomotion.wade_slowdown", l.wade_slowdown, 0.0, 0.95)?;
        positive("locomotion.mass_kg", l.mass_kg)?;
        within("locomotion.limp_speed_ratio", l.limp_speed_ratio, 0.0, 1.0)?;
        positive("locomotion.float_catch_m", l.float_catch_m)?;
        positive("locomotion.water_sink_m_s", l.water_sink_m_s)?;
        positive("locomotion.water_climb_m_s", l.water_climb_m_s)?;
        positive("locomotion.water_drag_m_s2", l.water_drag_m_s2)?;
        self.melee.validate()?;
        within("locomotion.skim_depth_share", l.skim_depth_share, 0.0, 2.0)?;
        positive("locomotion.skim_rise_m_s", l.skim_rise_m_s)?;
        let g = &self.gait;
        positive("gait.stride_max_m", g.stride_max_m)?;
        positive("gait.stride_min_m", g.stride_min_m)?;
        finite("gait.step_height_m", g.step_height_m)?;
        finite("gait.bob_m", g.bob_m)?;
        finite("gait.sway_m", g.sway_m)?;
        finite("gait.turn_step_radius_m", g.turn_step_radius_m)?;
        positive("gait.settle_speed_m_s", g.settle_speed_m_s)?;
        let j = &self.jet;
        positive("jet.thrust_m_s2", j.thrust_m_s2)?;
        positive("jet.climb_max_m_s", j.climb_max_m_s)?;
        positive("jet.sink_max_m_s", j.sink_max_m_s)?;
        positive("jet.fall_max_m_s", j.fall_max_m_s)?;
        positive("jet.dive_m_s2", j.dive_m_s2)?;
        positive("jet.dive_max_m_s", j.dive_max_m_s)?;
        crate::data::finite("jet.drag_per_s", j.drag_per_s)?;
        positive("jet.burn_s", j.burn_s)?;
        positive("jet.recharge_s", j.recharge_s)?;
        crate::data::finite("jet.recharge_delay_s", j.recharge_delay_s)?;
        within("jet.relight_fraction", j.relight_fraction, 0.0, 1.0)?;
        within("jet.vector_max_deg", j.vector_max_deg, 0.0, 60.0)?;
        positive("jet.vector_rate_deg_s", j.vector_rate_deg_s)?;
        let k = &self.skate;
        positive("skate.speed_max_m_s", k.speed_max_m_s)?;
        positive("skate.accel_m_s2", k.accel_m_s2)?;
        positive("skate.brake_m_s2", k.brake_m_s2)?;
        positive("skate.grip_per_s", k.grip_per_s)?;
        positive("skate.turn_rate_deg_s", k.turn_rate_deg_s)?;
        positive("skate.burn_s", k.burn_s)?;
        positive("skate.recharge_s", k.recharge_s)?;
        crate::data::finite("skate.recharge_delay_s", k.recharge_delay_s)?;
        within("skate.relight_fraction", k.relight_fraction, 0.0, 1.0)?;
        let b = &self.body;
        positive("body.eye_height_m", b.eye_height_m)?;
        finite("body.torso_hit_m[0]", b.torso_hit_m[0])?;
        positive("body.torso_hit_m[1]", b.torso_hit_m[1])?;
        if b.torso_hit_m[1] <= b.torso_hit_m[0] {
            return Err("body.torso_hit_m: the capsule must end above where it starts".into());
        }
        positive("body.torso_hit_radius_m", b.torso_hit_radius_m)?;
        positive("body.arm_hit_radius_m", b.arm_hit_radius_m)?;
        positive("body.leg_hit_radius_m", b.leg_hit_radius_m)?;
        positive("body.head_hit_radius_m", b.head_hit_radius_m)?;
        positive("body.wreck_centre_m", b.wreck_centre_m)?;
        finite("body.blast_height_m", b.blast_height_m)?;
        finite("body.smash_height_m", b.smash_height_m)?;
        for (i, v) in b.cut_box_m.iter().enumerate() {
            positive(&format!("body.cut_box_m[{i}]"), *v)?;
        }
        if let Some(i) = &self.interior {
            if self.model.is_none() {
                return Err("interior: only a glb mech carries an interior".into());
            }
            if !(i.model.ends_with(".glb") || i.model.ends_with(".gltf")) {
                return Err(format!("interior.model: `{}` must be a .glb or .gltf under assets/", i.model));
            }
            for r in &i.rides {
                if r.part.is_empty() || r.joint.is_empty() {
                    return Err("interior.rides: a ride needs a part and a joint".into());
                }
            }
        }
        validate_parts(&self.parts)?;
        for w in &self.weapons {
            w.validate()?;
        }
        for n in &self.nodes {
            finite3(&format!("nodes[{}].offset_m", n.name), n.offset_m)?;
            let parent = (!n.parent.is_empty()).then_some(n.parent.as_str());
            match contract_parent(&n.name).or_else(|| super::rig::optional_parent(&n.name)) {
                None => return Err(format!("nodes: `{}` is not a contract bone or socket", n.name)),
                Some(p) if p != parent => {
                    return Err(format!(
                        "nodes: `{}` must hang from `{}`, not `{}`",
                        n.name,
                        p.unwrap_or(""),
                        n.parent
                    ));
                }
                Some(_) => {}
            }
        }
        if !self.nodes.is_empty() {
            for b in BONES.iter().chain(SOCKETS.iter()) {
                if !self.nodes.iter().any(|n| n.name == b.name) {
                    return Err(format!("nodes: the stand-in is missing contract node `{}`", b.name));
                }
            }
        }
        for b in &self.boxes {
            finite3("boxes.centre_m", b.centre_m)?;
            finite3("boxes.size_m", b.size_m)?;
            if let Some(v) = b.along_m {
                finite3("boxes.along_m", v)?;
            }
            if !self.nodes.iter().any(|n| n.name == b.bone) {
                return Err(format!("boxes: bone `{}` is not a stand-in node", b.bone));
            }
        }
        if let Some(m) = &self.model
            && !(m.ends_with(".glb") || m.ends_with(".gltf"))
        {
            return Err(format!("model: `{m}` must be a .glb or .gltf under assets/"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{load_ron, parse_ron};
    use std::path::{Path, PathBuf};

    #[test]
    fn every_shipped_mech_is_aimed_at_its_chest_above_the_hips() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/mechs");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).expect("data/mechs reads") {
            let path = entry.expect("entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            if !name.ends_with(".ron") || name.contains("boarding") {
                continue;
            }
            let Ok(m) = load_ron::<MechConfig>(&path) else { continue };
            let aim = m.aim_height_m();
            let (hip, top) = (m.locomotion.hip_height_m, m.locomotion.body_height_m);
            assert!(aim > hip && aim < top, "{name}: aimed at {aim} m, hips {hip} m, top {top} m");
            checked += 1;
        }
        assert!(checked >= 5, "{checked} mechs checked");
    }

    fn shipped() -> MechConfig {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/mechs/placeholder.ron");
        load_ron(&p).expect("placeholder mech data loads")
    }

    #[test]
    fn walking_into_a_building_at_full_walk_breaks_nothing() {
        let m = shipped();
        let walk = m.locomotion.walk_speed_max_m_s;
        assert!(walk < m.ram.speed_m_s, "the walk must be slower than a ram");
        assert_eq!(m.ram.hole_radius_m(walk, 0.0), None);
    }

    #[test]
    fn a_ram_that_the_wall_stops_breaks_a_hole_that_grows_with_speed() {
        let r = RamConfig::default();
        assert_eq!(r.hole_radius_m(20.0, 0.0), Some(4.5));
        let at_55 = r.hole_radius_m(55.0, 10.0).unwrap();
        assert!((at_55 - 8.83).abs() < 0.02, "{at_55}");
        assert_eq!(r.hole_radius_m(120.0, 0.0), Some(9.0), "capped");
    }

    #[test]
    fn a_ram_the_wall_does_not_stop_breaks_nothing() {
        let r = RamConfig::default();
        assert_eq!(r.hole_radius_m(30.0, 25.0), None);
        assert_eq!(r.hole_radius_m(f32::NAN, 0.0), None);
    }

    #[test]
    fn a_ram_hole_smaller_than_its_cap_is_refused() {
        let e = parse_ron::<MechConfig>("(ram: (radius_m: 10.0, radius_max_m: 9.0))", Path::new("m.ron")).unwrap_err();
        assert!(e.to_string().contains("ram.radius_max_m"), "{e}");
    }

    #[test]
    fn the_placeholder_follows_the_contract() {
        let m = shipped();
        for b in BONES.iter().chain(SOCKETS.iter()) {
            let node = m
                .nodes
                .iter()
                .find(|n| n.name == b.name)
                .unwrap_or_else(|| panic!("{} missing", b.name));
            assert_eq!(node.parent, b.parent.unwrap_or(""), "{}", b.name);
        }
    }

    #[test]
    fn the_placeholder_is_twelve_metres_with_eyes_at_nine_point_six() {
        let m = shipped();
        let world_y = |name: &str| {
            let mut y = 0.0;
            let mut at = name.to_string();
            while let Some(n) = m.nodes.iter().find(|n| n.name == at) {
                y += n.offset_m[1];
                at = n.parent.clone();
            }
            y
        };
        assert!((world_y("EYE_pilot") - 9.6).abs() < 1e-3);
        let torso_top = world_y("torso") + 2.2 + 2.2;
        assert!((torso_top - 12.0).abs() < 1e-3, "{torso_top}");
    }

    #[test]
    fn a_mech_without_a_body_section_keeps_the_stand_ins_sizes() {
        let m = parse_ron::<MechConfig>(r#"(name: "x")"#, Path::new("m.ron")).unwrap();
        assert_eq!(m.body.eye_height_m, 9.6);
        assert_eq!(m.body.torso_hit_m, [0.5, 3.9]);
    }

    #[test]
    fn an_interior_without_a_model_is_refused() {
        let e = parse_ron::<MechConfig>(r#"(interior: Some((model: "models/c.gltf")))"#, Path::new("m.ron"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("interior"), "{e}");
    }

    #[test]
    fn a_node_off_the_contract_is_refused_by_name() {
        let e = parse_ron::<MechConfig>(
            r#"(nodes: [(name: "knee_L", parent: "thigh_L", offset_m: (0.0, 0.0, 0.0))])"#,
            Path::new("m.ron"),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("knee_L"), "{e}");
    }
}
