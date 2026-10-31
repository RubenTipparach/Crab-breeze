//! The Stratostacean, the crab air fortress (openspec change `crab-airship`): how it stays up, flies, sinks, lists,
//! falls and crashes, how each of its parts takes a hit, how its turrets aim and fire, and when it launches a
//! fighter. Pure and deterministic: `cb-app::fortress` feeds it the fixed step, where its parts' nodes are and
//! where the pilot's mech is, and acts on what it says. The cockpit's lift readout reads `lift`, the rule the
//! fortress flies by (CLAUDE.md 6.1).
//!
//! Frame: metres, +Y up, the fortress faces +Z, +X is its own left (port), origin at the hull's middle.

mod guns;
mod hits;
mod lift;

pub use guns::{Beam, Fire, Mount, Target, hull_hides};
pub use hits::{HitOutcome, Weak};
pub use lift::{Lift, lift};

use crate::data::{Validate, finite, positive, within};
use crate::mission::RaiderSpec;
use crate::rng::SimRng;
use crate::weapons::WeaponSpec;
use crate::world::{Slab, Structures, WorldPlan};
use glam::{Affine3A, Quat, Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

const G_M_S2: f32 = 9.81;

/// A part's hit capsule in its own frame: along its Y axis, centred `centre_y_m` up it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapsuleSpec {
    /// Radius.
    pub radius_m: f32,
    /// Half the length of its axis, end to end of the straight part.
    pub half_m: f32,
    /// Where its middle sits up the part's Y axis.
    #[serde(default)]
    pub centre_y_m: f32,
}

/// A beam turret's cycle (design section 4): it charges where the pilot can see it, then burns.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BeamSpec {
    /// The lens brightens this long before the beam lights.
    pub charge_s: f32,
    /// The beam burns this long.
    pub on_s: f32,
    /// Damage a second to the part it touches.
    pub damage_per_s: f32,
    /// It sweeps across where the target stood when it began to charge, from this far to one side to this far to the
    /// other, metres, so a pilot who moves off that line is missed; 0 holds it on the target (owner, 2026-10-29:
    /// "lasers sweep").
    #[serde(default)]
    pub sweep_m: f32,
    /// A sweeping beam burns what is within this of its line, metres.
    #[serde(default = "six")]
    pub hit_m: f32,
}

fn six() -> f32 {
    6.0
}

/// A defence's gun.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GunSpec {
    /// Its round: speed, penetration, damage, blast, a missile's seeker; `interval_s` is the time between rounds
    /// in a burst.
    pub weapon: WeaponSpec,
    /// It fires at a target this close or closer.
    pub range_m: f32,
    /// Rounds in a burst.
    pub burst: u32,
    /// Pause after a burst (a beam: between one burn and the next charge).
    pub pause_s: f32,
    /// Each round's direction is off by up to this, degrees (doubled with the bridge lost).
    pub aim_error_deg: f32,
    /// How far it elevates, [lowest, highest], degrees above the fortress's horizontal.
    pub elev_deg: [f32; 2],
    /// It turns on a yaw ring and a pitch cradle (the game drives `YAW_<id>` and `PITCH_<id>`); a missile
    /// battery's cells do not turn.
    #[serde(default = "yes")]
    pub turns: bool,
    /// A round bursts this close to the target (flak), 0 for none.
    #[serde(default)]
    pub proximity_m: f32,
    /// Rounds aboard, 0 for no limit.
    #[serde(default)]
    pub magazine: u32,
    /// A beam instead of rounds.
    #[serde(default)]
    pub beam: Option<BeamSpec>,
    /// It holds on the target this long before a burst, the HUD warning the pilot, seconds (owner, 2026-10-29: "have
    /// some way of telegraphing shots", "railgun warning so player can dodge").
    #[serde(default)]
    pub warn_s: f32,
}

fn yes() -> bool {
    true
}

/// One kind of part (design section 3).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KindSpec {
    /// What the pilot calls it.
    pub name: String,
    /// Hit points.
    pub hp: f32,
    /// Armour a round must beat, millimetres.
    pub armour_mm: f32,
    /// Its weak point, if any.
    #[serde(default)]
    pub weak: Weak,
    /// The weak point's armour, millimetres.
    #[serde(default = "forty")]
    pub weak_mm: f32,
    /// Damage through the weak point is multiplied by this.
    #[serde(default = "two")]
    pub weak_damage: f32,
    /// Its hit capsule.
    pub capsule: CapsuleSpec,
    /// Its gun, if it is a defence.
    #[serde(default)]
    pub gun: Option<GunSpec>,
    /// Boxed and lockable all the time; any other part only while its gun fires or warns (owner, 2026-10-29: "There are
    /// way too many targets to even see it properly"). Every part can be hit.
    #[serde(default)]
    pub always_target: bool,
    /// One in this many parts of the kind carries its gun, the first of each run of them (owner, 2026-10-29: "1/2
    /// weapons"); the rest are silent mounts.
    #[serde(default = "one")]
    pub armed_every: u32,
}

fn one() -> u32 {
    1
}

fn forty() -> f32 {
    40.0
}

fn two() -> f32 {
    2.0
}

fn four() -> f32 {
    4.0
}

fn three() -> u32 {
    3
}

/// Fighters off the catapults (design section 4).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaunchSpec {
    /// The unit launched (`data/world/units.ron`).
    pub unit: String,
    /// A launch every this long, from the live catapults in turn.
    pub interval_s: f32,
    /// The first launch this long after the mission begins.
    pub first_s: f32,
    /// No launch while this many of its fighters fly.
    pub max_airborne: u32,
    /// Fighters in the hangar.
    pub hangar: u32,
    /// Where a fighter leaves its catapult, in the catapult's frame.
    pub off_m: Vec3,
}

/// Under a live lift thruster (design section 5).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DownwashSpec {
    /// The column's radius at the nozzle.
    pub radius_top_m: f32,
    /// Its radius `length_m` below.
    pub radius_bottom_m: f32,
    /// How far down it reaches.
    pub length_m: f32,
    /// It pushes a mech down this hard.
    pub push_m_s2: f32,
    /// And burns its torso this much a second.
    pub burn_per_s: f32,
}

/// A thruster's leg breaking off and its fall (owner, 2026-10-25: "when a pylon is destroyed it should fall off
/// and fall onto the ground and explode").
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LegSpec {
    /// The explosion where it lands, megajoules.
    pub blast_mj: f32,
    /// Building cells within this of where it lands break.
    pub break_radius_m: f32,
    /// Tumbling, degrees a second.
    pub tumble_deg_s: f32,
}

/// A post of the deck guard: which crab stands where, in the fortress's frame (x, z) on one of its tops.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GuardPost {
    /// The crab, a key of `GuardSpec::crabs`.
    pub crab: String,
    /// Where it stands (x, z), metres.
    pub at_m: Vec2,
}

/// The deck guard (design section 4; owner, 2026-10-26, survey H6: 2 Shrikes and 4 Mites on deck, a Mite up each
/// hangar lift every 60 s while the lifts work).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GuardSpec {
    /// Aboard from the start.
    pub start: Vec<GuardPost>,
    /// The crab each hangar lift brings up.
    pub lift_crab: String,
    /// Each working lift brings one up this often, seconds.
    pub lift_interval_s: f32,
    /// No lift brings one up while this many stand.
    pub max_on_deck: u32,
    /// Crabs below decks for the lifts.
    pub hangar: u32,
    /// Each crab: its mech, skin, guns and pilot, as a defence raider's.
    pub crabs: BTreeMap<String, RaiderSpec>,
}

/// The crash goes nuclear (owner, 2026-10-30: "Crab carrier when it impacts the ground goes nuclear in a big mushroom
/// cloud and leave a giant Crater"; crab-airship section 11).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NukeSpec {
    /// The crater: its bowl's radius...
    pub crater_radius_m: f32,
    /// ...its depth at the middle...
    pub crater_depth_m: f32,
    /// ...and its rim's height.
    pub rim_m: f32,
    /// Every building and house within this of the impact comes down.
    pub flatten_radius_m: f32,
    /// The fireball's radius as it swells, the flash's size (`nuke_flash`).
    pub fireball_radius_m: f32,
    /// The mushroom cloud's cap climbs to this height over the ground (Mining Mike's cloud scaled to it, `nuke`)...
    pub cap_height_m: f32,
    /// ...his cloud stretched up this many times over its width to reach it (owner, 2026-10-30: "make it 1.5x
    /// taller")...
    pub stretch_up: f32,
    /// ...over this long, the explosion: the fireball bursts, swells, cools and climbs on its column...
    pub cloud_s: f32,
    /// ...then its smoke hangs this long after, slowly spreading...
    pub smoke_linger_s: f32,
    /// ...to this many times the size it reached, fading away over the last fifth (owner, 2026-10-30: "The smoke
    /// should last for a good minute though as it slowly grows to 1.5x the final size").
    pub smoke_grow: f32,
}

/// The crash (design section 6).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CrashSpec {
    /// The hull's middle is this high over the ground when it hits.
    pub clearance_m: f32,
    /// The explosion at its middle, megajoules.
    pub blast_mj: f32,
    /// Building cells under its footprint and this far round it break.
    pub break_margin_m: f32,
    /// Its footprint on the ground: half its width (x) and half its length (z), an ellipse in its own frame.
    pub half_size_m: Vec2,
    /// It goes nuclear when it hits (`None`: an ordinary crash).
    #[serde(default)]
    pub nuke: Option<NukeSpec>,
}

/// `data/missions/fortress.ron`: every number the fortress uses (design sections 1 to 6 and 9).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FortressConfig {
    /// What the pilot calls it, on the HUD, the TARGET page and in the log (owner, 2026-10-26, survey H2: "the
    /// Stratostacean").
    pub name: String,
    /// Mass, tonnes (its weight W = mass x g).
    pub mass_t: f32,
    /// The hull's middle over the attacked city's ground.
    pub altitude_m: f32,
    /// It circles the city's centre at this radius...
    pub circle_radius_m: f32,
    /// ...at this speed with both drive thrusters.
    pub speed_m_s: f32,
    /// With both drives lost its drift dies away over this long.
    pub drive_stop_s: f32,
    /// A lift thruster carries at most this share of W.
    pub lift_max_share: f32,
    /// A hull thruster carries at most this share of W.
    pub hull_max_share: f32,
    /// With every lift thruster lost, its hull thrusters hold it until this many of them are lost, on either side;
    /// then it has no thrust at all (owner, 2026-10-31: "all crab legs have to be destroyed, then half the internal
    /// thrusters would have to be destroyed").
    #[serde(default = "two_kills")]
    pub hull_kills_to_fall: u32,
    /// On its circle it weaves in and out by this much (owner, 2026-10-30: "have crab carrier move around a bit too so
    /// it's not so easy to shoot down"), metres either side of `circle_radius_m`...
    #[serde(default)]
    pub weave_m: f32,
    /// ...this many times a turn.
    #[serde(default)]
    pub weave_per_turn: f32,
    /// Sink drag, per metre (a = ... - k v |v|).
    pub sink_drag_per_m: f32,
    /// List, degrees per weight of imbalance between the sides.
    pub list_deg_per_weight: f32,
    /// The list moves toward its target at this rate, degrees a second.
    pub list_rate_deg_s: f32,
    /// Holding again, any sink dies away with this time constant.
    pub hold_time_s: f32,
    /// The hull doors take this long to open.
    pub doors_open_s: f32,
    /// The hull thrusters light this long after the doors start opening.
    pub hull_light_s: f32,
    /// Turrets turn at this rate, degrees a second.
    pub turret_turn_deg_s: f32,
    /// A turret fires only within this of its aim, degrees.
    pub fire_cone_deg: f32,
    /// Its detail (the spikes, the model's `DETAIL_` meshes) draws only this close to the eye, metres (owner,
    /// 2026-10-25: "spikes should not render until we are up close").
    pub detail_range_m: f32,
    /// A part that is not `always_target` is boxed for this long after its gun fires, seconds.
    #[serde(default = "four")]
    pub fire_box_s: f32,
    /// At most this many guns warn, fire or burn at the pilot at once (owner, 2026-10-29: "weapons should not all target
    /// you at once, maybe 3 at a time").
    #[serde(default = "three")]
    pub max_firing: u32,
    /// Every kind of part by name.
    pub kinds: BTreeMap<String, KindSpec>,
    /// Fighters.
    pub launch: LaunchSpec,
    /// Downwash.
    pub downwash: DownwashSpec,
    /// The deck guard.
    pub guard: GuardSpec,
    /// A leg breaking off.
    pub leg: LegSpec,
    /// The crash.
    pub crash: CrashSpec,
}

impl FortressConfig {
    /// Its weight, newtons.
    pub fn weight_n(&self) -> f32 {
        self.mass_t * 1000.0 * G_M_S2
    }
}

fn two_kills() -> u32 {
    2
}

impl Validate for FortressConfig {
    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("`name` needs text".into());
        }
        for (k, v) in [
            ("mass_t", self.mass_t),
            ("altitude_m", self.altitude_m),
            ("circle_radius_m", self.circle_radius_m),
            ("speed_m_s", self.speed_m_s),
            ("drive_stop_s", self.drive_stop_s),
            ("sink_drag_per_m", self.sink_drag_per_m),
            ("list_deg_per_weight", self.list_deg_per_weight),
            ("list_rate_deg_s", self.list_rate_deg_s),
            ("hold_time_s", self.hold_time_s),
            ("doors_open_s", self.doors_open_s),
            ("hull_light_s", self.hull_light_s),
            ("turret_turn_deg_s", self.turret_turn_deg_s),
            ("fire_cone_deg", self.fire_cone_deg),
            ("detail_range_m", self.detail_range_m),
            ("launch.interval_s", self.launch.interval_s),
            ("downwash.length_m", self.downwash.length_m),
            ("crash.clearance_m", self.crash.clearance_m),
            ("guard.lift_interval_s", self.guard.lift_interval_s),
            ("crash.half_size_m.x", self.crash.half_size_m.x),
            ("crash.half_size_m.y", self.crash.half_size_m.y),
        ] {
            positive(k, v)?;
        }
        within("lift_max_share", self.lift_max_share, 1e-3, 0.5)?;
        let g = &self.guard;
        for c in g.start.iter().map(|p| &p.crab).chain(std::iter::once(&g.lift_crab)) {
            if !g.crabs.contains_key(c) {
                return Err(format!("guard: no crab `{c}` in guard.crabs"));
            }
        }
        for (name, c) in &g.crabs {
            positive(&format!("guard.crabs.{name}.armour_scale"), c.armour_scale)?;
        }
        within("hull_max_share", self.hull_max_share, 1e-3, 0.5)?;
        within("weave_m", self.weave_m, 0.0, self.circle_radius_m * 0.8)?;
        within("weave_per_turn", self.weave_per_turn, 0.0, 20.0)?;
        if let Some(n) = &self.crash.nuke {
            for (k, v) in [
                ("crash.nuke.crater_radius_m", n.crater_radius_m),
                ("crash.nuke.crater_depth_m", n.crater_depth_m),
                ("crash.nuke.flatten_radius_m", n.flatten_radius_m),
                ("crash.nuke.fireball_radius_m", n.fireball_radius_m),
                ("crash.nuke.cap_height_m", n.cap_height_m),
                ("crash.nuke.cloud_s", n.cloud_s),
                ("crash.nuke.smoke_linger_s", n.smoke_linger_s),
            ] {
                positive(k, v)?;
            }
            within("crash.nuke.rim_m", n.rim_m, 0.0, n.crater_depth_m)?;
            within("crash.nuke.smoke_grow", n.smoke_grow, 1.0, 4.0)?;
            within("crash.nuke.stretch_up", n.stretch_up, 0.5, 3.0)?;
        }
        finite("launch.first_s", self.launch.first_s)?;
        for (name, k) in &self.kinds {
            let f = |x: &str| format!("kinds.{name}.{x}");
            positive(&f("hp"), k.hp)?;
            within(&f("armed_every"), k.armed_every as f32, 1.0, 100.0)?;
            within(&f("armour_mm"), k.armour_mm, 0.0, 10_000.0)?;
            positive(&f("capsule.radius_m"), k.capsule.radius_m)?;
            within(&f("capsule.half_m"), k.capsule.half_m, 0.0, 1000.0)?;
            if let Some(g) = &k.gun {
                g.weapon.validate()?;
                positive(&f("gun.range_m"), g.range_m)?;
                within(&f("gun.warn_s"), g.warn_s, 0.0, 30.0)?;
                within(&f("gun.aim_error_deg"), g.aim_error_deg, 0.0, 45.0)?;
                if g.burst == 0 {
                    return Err(format!("`{}` must be above zero", f("gun.burst")));
                }
                if g.elev_deg[0] > g.elev_deg[1] {
                    return Err(format!("`{}`: lowest above highest", f("gun.elev_deg")));
                }
                if let Some(b) = &g.beam {
                    positive(&f("gun.beam.charge_s"), b.charge_s)?;
                    positive(&f("gun.beam.on_s"), b.on_s)?;
                    within(&f("gun.beam.sweep_m"), b.sweep_m, 0.0, 1000.0)?;
                    positive(&f("gun.beam.hit_m"), b.hit_m)?;
                }
            }
        }
        Ok(())
    }
}

/// A part in the shape file.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartRef {
    /// Its id: the glb's `PART_<id>` node.
    pub id: String,
    /// Its kind in `FortressConfig::kinds`.
    pub kind: String,
    /// A thruster's side: +1 port (+X), -1 starboard, 0 for none.
    pub side: i8,
}

/// A capsule of hull that stops rounds and lines of sight, in the fortress's frame.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HullCapsule {
    /// One end.
    pub a: Vec3,
    /// The other.
    pub b: Vec3,
    /// Radius.
    pub radius_m: f32,
}

/// `data/missions/fortress_shape.ron`, measured off the model by `tools/mockups/export_fortress_glb.mjs`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FortressShape {
    /// The model, relative to `assets/`.
    pub model: String,
    /// Every part, in a stable order (the order everything about the parts is kept in).
    pub parts: Vec<PartRef>,
    /// The tops a mech stands on.
    pub platforms: Vec<Slab>,
    /// The hull.
    pub hull: Vec<HullCapsule>,
}

impl Validate for FortressShape {
    fn validate(&self) -> Result<(), String> {
        if self.parts.is_empty() {
            return Err("parts: none".into());
        }
        let mut ids: Vec<&str> = self.parts.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
            return Err(format!("parts: `{}` twice", w[0]));
        }
        for p in &self.parts {
            if !(-1..=1).contains(&p.side) {
                return Err(format!("parts.{}.side: -1, 0 or 1", p.id));
            }
        }
        Ok(())
    }
}

/// The fortress's data, both files.
#[derive(Clone, Debug, PartialEq)]
pub struct FortressData {
    /// The numbers.
    pub cfg: FortressConfig,
    /// The shape.
    pub shape: FortressShape,
}

impl FortressData {
    /// Every part's kind is in the config's kinds.
    pub fn check(&self) -> Result<(), String> {
        for p in &self.shape.parts {
            if !self.cfg.kinds.contains_key(&p.kind) {
                return Err(format!("fortress_shape.ron part `{}`: no kind `{}` in fortress.ron", p.id, p.kind));
            }
        }
        for p in &self.cfg.guard.start {
            if self.shape.platforms.iter().all(|s| s.top_at(p.at_m).is_none()) {
                return Err(format!("fortress.ron guard post {} at {}: not on any of its tops", p.crab, p.at_m));
            }
        }
        for k in ["lift", "hull"] {
            if !self.shape.parts.iter().any(|p| p.kind == k) {
                return Err(format!("fortress_shape.ron: no `{k}` parts"));
            }
        }
        Ok(())
    }

    /// A part's kind.
    pub fn kind(&self, part: usize) -> &KindSpec {
        &self.cfg.kinds[&self.shape.parts[part].kind]
    }

    /// The index of the part `id`.
    pub fn part(&self, id: &str) -> Option<usize> {
        self.shape.parts.iter().position(|p| p.id == id)
    }
}

/// A beam turret's phase.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BeamPhase {
    /// Waiting, this long left.
    #[default]
    Idle,
    /// The lens brightens.
    Charging,
    /// It burns.
    On,
}

/// A defence's aim and clocks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GunState {
    /// Yaw on its ring, radians (0 along the fortress's +Z, toward +X positive).
    pub yaw_rad: f32,
    /// Elevation, radians above the fortress's horizontal.
    pub elev_rad: f32,
    /// Until the next round (or the next phase of a beam).
    pub cool_s: f32,
    /// Rounds left in this burst.
    pub left: u32,
    /// Rounds fired.
    pub fired: u32,
    /// A beam's phase.
    pub beam: BeamPhase,
    /// Its warning before a burst, seconds left (`GunSpec::warn_s`).
    pub warn_s: f32,
    /// Warned and firing its burst.
    pub firing: bool,
    /// A sweeping beam's line: the middle (where the target stood) and the way it sweeps, world.
    pub sweep_at: Vec3,
    /// ...
    pub sweep_side: Vec3,
}

impl GunState {
    /// Warning, firing or burning at the pilot: holding one of `max_firing`'s places.
    pub fn engaged(&self) -> bool {
        self.firing || self.warn_s > 0.0 || self.beam != BeamPhase::Idle
    }

    /// Fired, warning or burning just now: a round left since `fired_before`, or engaged.
    pub fn busy(&self, fired_before: u32) -> bool {
        self.fired != fired_before || self.engaged()
    }

    /// A launcher or gun is in the middle of a burst (its tube faces open: `Weak::WhileFiring`).
    pub fn firing(&self, spec: &GunSpec) -> bool {
        self.left < spec.burst && self.left > 0
    }
}

/// A part as it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartState {
    /// Hit points left.
    pub hp: f32,
    /// Not yet destroyed.
    pub alive: bool,
    /// Its gun.
    pub gun: GunState,
}

/// What happened this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A side's outer thrusters can no longer hold it: the hull doors open.
    DoorsOpening,
    /// The hull thrusters lit.
    HullLit,
    /// It can no longer hold its weight: it falls.
    Falling,
    /// It hit the ground.
    Crashed,
}

/// The fortress as it flies.
#[derive(Clone, Debug, PartialEq)]
pub struct Fortress {
    /// The centre of the circle it flies (x, z).
    pub centre: Vec2,
    /// The attacked city's ground.
    pub ground_m: f32,
    /// Where it is on its circle, radians.
    pub angle_rad: f32,
    /// How far out from the centre it is: 0 where it appears, over the city's centre, growing at its speed until
    /// it reaches its circle (`circle_radius_m`), then held there (section 10).
    pub radius_m: f32,
    /// It has reached its circle and flies round it (weaving, `weave_m`).
    pub circling: bool,
    /// Its drift speed now.
    pub speed_m_s: f32,
    /// The hull's middle.
    pub pos: Vec3,
    /// Yaw: its +Z turned this far about +Y.
    pub yaw_rad: f32,
    /// List, radians, port side down positive.
    pub list_rad: f32,
    /// Vertical speed, up positive.
    pub v_m_s: f32,
    /// Seconds since the hull doors began to open, once they have.
    pub doors_s: Option<f32>,
    /// Every part, in the shape's order.
    pub parts: Vec<PartState>,
    /// It cannot hold its weight.
    pub falling: bool,
    /// It is down.
    pub crashed: bool,
    /// Until the next launch.
    pub launch_in_s: f32,
    /// The catapult that launches next, by turn.
    pub launch_turn: usize,
    /// Fighters left in the hangar.
    pub hangar: u32,
    /// Until the hangar lifts next bring a crab up.
    pub guard_in_s: f32,
    /// Crabs left below decks for the lifts.
    pub guard_hangar: u32,
    rng: SimRng,
}

impl Fortress {
    /// A fortress over `centre`, `cfg.altitude_m` over `ground_m`, every part whole, about to fly out onto its circle
    /// round that centre (section 10: it appears over the city's centre).
    pub fn new(d: &FortressData, centre: Vec2, ground_m: f32, seed: u64) -> Self {
        let parts = (0..d.shape.parts.len())
            .map(|i| {
                let k = d.kind(i);
                PartState {
                    hp: k.hp,
                    alive: true,
                    gun: GunState {
                        left: k.gun.as_ref().map_or(0, |g| g.burst),
                        ..GunState::default()
                    },
                }
            })
            .collect();
        let mut f = Self {
            centre,
            ground_m,
            angle_rad: 0.0,
            radius_m: 0.0,
            circling: false,
            speed_m_s: d.cfg.speed_m_s,
            pos: Vec3::ZERO,
            yaw_rad: 0.0,
            list_rad: 0.0,
            v_m_s: 0.0,
            doors_s: None,
            parts,
            falling: false,
            crashed: false,
            launch_in_s: d.cfg.launch.first_s,
            launch_turn: 0,
            hangar: d.cfg.launch.hangar,
            guard_in_s: d.cfg.guard.lift_interval_s,
            guard_hangar: d.cfg.guard.hangar,
            rng: SimRng::new(seed).fork("fortress"),
        };
        f.place(&d.cfg, ground_m + d.cfg.altitude_m);
        f
    }

    fn place(&mut self, cfg: &FortressConfig, y: f32) {
        let (s, c) = self.angle_rad.sin_cos();
        let xz = self.centre + Vec2::new(c, s) * self.radius_m;
        self.pos = Vec3::new(xz.x, y, xz.y);
        // nose the way it flies: out along the radius, then round (angle increasing: +x -> +z), in and out as it weaves
        self.yaw_rad = if self.circling {
            let k = cfg.weave_per_turn;
            let dr = cfg.weave_m * k * (k * self.angle_rad).cos();
            let along = Vec2::new(c, s) * dr + Vec2::new(-s, c) * self.radius_m;
            along.x.atan2(along.y)
        } else {
            c.atan2(s)
        };
    }

    /// Its rotation: yaw, then the list about its own forward axis (port down for a positive list).
    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw_rad) * Quat::from_rotation_z(-self.list_rad)
    }

    /// Fortress frame to world.
    pub fn pose(&self) -> Affine3A {
        Affine3A::from_rotation_translation(self.rotation(), self.pos)
    }

    /// Its velocity now (drift along the circle and the sink).
    pub fn velocity(&self) -> Vec3 {
        if self.crashed {
            return Vec3::ZERO;
        }
        // out along the radius until on the circle, then along it: the way its nose points (`place`)
        let along = Vec3::new(self.yaw_rad.sin(), 0.0, self.yaw_rad.cos());
        along * self.speed_m_s + Vec3::Y * self.v_m_s
    }

    /// The hull doors' opening, 0 shut to 1 open.
    pub fn doors(&self, cfg: &FortressConfig) -> f32 {
        self.doors_s.map_or(0.0, |t| (t / cfg.doors_open_s).clamp(0.0, 1.0))
    }

    /// The hull thrusters are lit.
    pub fn hull_lit(&self, cfg: &FortressConfig) -> bool {
        self.doors_s.is_some_and(|t| t >= cfg.hull_light_s)
    }

    /// Its lift now, by the rule it flies by.
    pub fn lift(&self, d: &FortressData) -> Lift {
        lift(d, &self.parts, self.hull_lit(&d.cfg))
    }

    /// The part a pilot shoots next to bring it down (section 12): every lift thruster, port's first, each side in the
    /// shape's order; then its hull thrusters in the shape's order until it falls. `None` once it is falling or down.
    pub fn next_to_bring_down(&self, d: &FortressData) -> Option<usize> {
        if self.falling || self.crashed {
            return None;
        }
        let live = |kind: &'static str, side: i8| {
            (0..d.shape.parts.len()).filter(move |&i| d.shape.parts[i].kind == kind && d.shape.parts[i].side == side && self.parts[i].alive)
        };
        live("lift", 1)
            .chain(live("lift", -1))
            .chain(live("hull", 1))
            .chain(live("hull", -1))
            .next()
    }

    /// The bridge stands (losing it doubles every aim error).
    pub fn bridge_alive(&self, d: &FortressData) -> bool {
        d.shape
            .parts
            .iter()
            .zip(&self.parts)
            .filter(|(r, _)| r.kind == "bridge")
            .all(|(_, p)| p.alive)
    }

    /// One fixed step of `dt_s`: the doors, the lift, the sink and the list, the drift, the crash. `ground`
    /// gives the ground's height under a point.
    pub fn step(&mut self, d: &FortressData, ground: &dyn Fn(f32, f32) -> f32, dt_s: f32) -> Vec<Event> {
        let cfg = &d.cfg;
        let mut events = Vec::new();
        if self.crashed || dt_s.is_nan() || dt_s <= 0.0 {
            return events;
        }
        // The doors open when the last lift thruster is lost (section 12).
        let outer = lift(d, &self.parts, false);
        if self.doors_s.is_none() && outer.outer == [0, 0] {
            self.doors_s = Some(0.0);
            events.push(Event::DoorsOpening);
        }
        if let Some(t) = self.doors_s.as_mut() {
            let was_lit = *t >= cfg.hull_light_s;
            *t += dt_s;
            if !was_lit && *t >= cfg.hull_light_s {
                events.push(Event::HullLit);
            }
        }
        let l = self.lift(d);
        // Falling: half its hull thrusters lost with no leg left, and with them all its thrust.
        let could = lift(d, &self.parts, true);
        if !could.holds && !self.falling {
            self.falling = true;
            events.push(Event::Falling);
        }
        // Vertical: a = g (L / W) - g - k v |v|; holding, the sink dies away.
        let total = l.port + l.stbd;
        if l.holds {
            self.v_m_s -= self.v_m_s * (dt_s / cfg.hold_time_s).min(1.0);
        } else {
            let a = G_M_S2 * total - G_M_S2 - cfg.sink_drag_per_m * self.v_m_s * self.v_m_s.abs();
            self.v_m_s += a * dt_s;
        }
        // List toward the weaker side (port down positive).
        let want = (cfg.list_deg_per_weight * (l.stbd - l.port)).to_radians();
        let step = cfg.list_rate_deg_s.to_radians() * dt_s;
        self.list_rad += (want - self.list_rad).clamp(-step, step);
        // Drift: half speed per drive lost; both lost, it dies away.
        let drives = d
            .shape
            .parts
            .iter()
            .zip(&self.parts)
            .filter(|(r, p)| r.kind == "drive" && p.alive)
            .count() as f32;
        let all = d.shape.parts.iter().filter(|r| r.kind == "drive").count().max(1) as f32;
        let want_speed = cfg.speed_m_s * drives / all;
        let rate = cfg.speed_m_s / cfg.drive_stop_s * dt_s;
        self.speed_m_s += (want_speed - self.speed_m_s).clamp(-rate, rate);
        if self.circling {
            self.angle_rad += self.speed_m_s / cfg.circle_radius_m * dt_s;
            self.radius_m = cfg.circle_radius_m + cfg.weave_m * (cfg.weave_per_turn * self.angle_rad).sin();
        } else {
            self.radius_m = (self.radius_m + self.speed_m_s * dt_s).min(cfg.circle_radius_m);
            self.circling = self.radius_m >= cfg.circle_radius_m;
        }
        let y = self.pos.y + self.v_m_s * dt_s;
        self.place(cfg, y);
        let floor = ground(self.pos.x, self.pos.z) + cfg.crash.clearance_m;
        if self.pos.y <= floor {
            self.pos.y = floor;
            self.v_m_s = 0.0;
            self.speed_m_s = 0.0;
            self.crashed = true;
            events.push(Event::Crashed);
        }
        events
    }

    /// The next launch, if one is due: the index of the catapult part it leaves from. `airborne` is how many of
    /// its fighters fly now.
    pub fn launch(&mut self, d: &FortressData, airborne: u32, dt_s: f32) -> Option<usize> {
        if self.falling || self.crashed || self.hangar == 0 {
            return None;
        }
        self.launch_in_s -= dt_s;
        if self.launch_in_s > 0.0 || airborne >= d.cfg.launch.max_airborne {
            return None;
        }
        let cats: Vec<usize> = (0..d.shape.parts.len())
            .filter(|&i| d.shape.parts[i].kind == "cat" && self.parts[i].alive)
            .collect();
        if cats.is_empty() {
            return None;
        }
        let cat = cats[self.launch_turn % cats.len()];
        self.launch_turn += 1;
        self.hangar -= 1;
        // From the catapults left: a lost catapult halves the launches.
        let all = d.shape.parts.iter().filter(|r| r.kind == "cat").count().max(1);
        self.launch_in_s = d.cfg.launch.interval_s * all as f32 / cats.len() as f32;
        Some(cat)
    }

    /// The hangar lifts this step (survey H6): every `lift_interval_s` each working lift brings a crab up, while
    /// fewer than `max_on_deck` stand (`on_deck`, alive), crabs are left below and it is not falling. The indices of
    /// the lift parts that bring one, in the shape's order.
    pub fn guard_lift(&mut self, d: &FortressData, on_deck: u32, dt_s: f32) -> Vec<usize> {
        if self.falling || self.crashed {
            return Vec::new();
        }
        self.guard_in_s -= dt_s;
        if self.guard_in_s > 0.0 {
            return Vec::new();
        }
        self.guard_in_s += d.cfg.guard.lift_interval_s;
        let mut up = Vec::new();
        for i in (0..d.shape.parts.len()).filter(|&i| d.shape.parts[i].kind == "hlift" && self.parts[i].alive) {
            if on_deck + up.len() as u32 >= d.cfg.guard.max_on_deck || self.guard_hangar == 0 {
                break;
            }
            self.guard_hangar -= 1;
            up.push(i);
        }
        up
    }

    /// Where a body stands on the fortress over its own point `(x, z)`: the highest of its tops there, in the world.
    pub fn stand_at(&self, d: &FortressData, at: Vec2) -> Option<Vec3> {
        let top = d.shape.platforms.iter().filter_map(|s| s.top_at(at)).reduce(f32::max)?;
        Some(self.pose().transform_point3(Vec3::new(at.x, top, at.y)))
    }

    /// The push and burn on a mech at world point `p` from the live lift thrusters' downwash, given each
    /// thruster's nozzle (world) by part: (push down, m/s2; burn, damage a second), if it is in a column.
    pub fn downwash(&self, d: &FortressData, nozzles: &[(usize, Vec3)], p: Vec3) -> Option<(f32, f32)> {
        if self.crashed {
            return None;
        }
        let w = &d.cfg.downwash;
        let down = self.rotation() * Vec3::NEG_Y;
        nozzles
            .iter()
            .filter(|(i, _)| self.parts.get(*i).is_some_and(|s| s.alive) && d.shape.parts[*i].kind == "lift")
            .find(|(_, n)| {
                let rel = p - *n;
                let along = rel.dot(down);
                let r = w.radius_top_m + (w.radius_bottom_m - w.radius_top_m) * (along / w.length_m);
                (0.0..=w.length_m).contains(&along) && (rel - down * along).length() <= r
            })
            .map(|_| (w.push_m_s2, w.burn_per_s))
    }

    /// What its wreck breaks where it came down (design section 6): every building whose middle lies under its
    /// footprint or within `crash.break_margin_m` of it is broken from its ground floor up, so the structure falls
    /// (`structural-destruction`). Each is (building, the middle of the sphere, its radius) for
    /// `Structures::break_sphere`, in the building index's order. The cells it breaks are the attacked city's own,
    /// so its damage counts as city damage (survey H5, owner 2026-10-26): `mission::city_damage` measures it with
    /// everything else that broke. Empty until it has crashed.
    pub fn wreck_breaks(&self, d: &FortressData, plan: &WorldPlan, structures: &Structures) -> Vec<(u32, Vec3, f32)> {
        if !self.crashed {
            return Vec::new();
        }
        let c = &d.cfg.crash;
        let reach = Vec2::new(c.half_size_m.x + c.break_margin_m, c.half_size_m.y + c.break_margin_m);
        let inv = self.pose().inverse();
        let mut out = Vec::new();
        for b in plan.collision.candidates(Vec2::new(self.pos.x, self.pos.z), reach.max_element()) {
            let mid = plan.buildings[b as usize].footprint.centre();
            let local = inv.transform_point3(Vec3::new(mid.x, self.pos.y, mid.y));
            if (local.x / reach.x).powi(2) + (local.z / reach.y).powi(2) > 1.0 {
                continue;
            }
            let Some(st) = structures.buildings.get(b as usize) else { continue };
            let half = (st.size.x * st.n[0] as f32).max(st.size.z * st.n[2] as f32) * 0.75;
            out.push((b, Vec3::new(mid.x, st.origin.y + 2.0, mid.y), half.max(6.0)));
        }
        out
    }

    /// The rng for a defence's spread, in the step's stable order of parts.
    fn rng(&mut self) -> &mut SimRng {
        &mut self.rng
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::data::load_ron;
    use std::path::Path;

    pub fn data() -> FortressData {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/missions");
        let d = FortressData {
            cfg: load_ron(&root.join("fortress.ron")).unwrap(),
            shape: load_ron(&root.join("fortress_shape.ron")).unwrap(),
        };
        d.check().unwrap();
        d
    }

    pub const DT: f32 = 1.0 / 60.0;
    pub fn flat(_: f32, _: f32) -> f32 {
        0.0
    }

    /// A fortress already out on its circle (the tests of how it flies, holds and falls start there).
    pub fn fortress() -> (FortressData, Fortress) {
        let mut d = data();
        d.cfg.weave_m = 0.0;
        let mut f = Fortress::new(&d, Vec2::ZERO, 0.0, 42);
        f.radius_m = d.cfg.circle_radius_m;
        f.circling = true;
        let y = f.pos.y;
        f.place(&d.cfg, y);
        (d, f)
    }

    #[test]
    fn the_hull_lights_only_when_every_leg_is_gone() {
        let (d, mut f) = fortress();
        let y = f.pos.y;
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3"]);
        let e = run(&d, &mut f, 10.0);
        assert!(e.is_empty(), "one leg left: nothing opens, nothing sinks: {e:?}");
        assert!((f.pos.y - y).abs() < 1e-3);
        kill(&d, &mut f, &["R4"]);
        let e = run(&d, &mut f, 10.0);
        assert!(e.contains(&Event::DoorsOpening) && e.contains(&Event::HullLit), "{e:?}");
        assert!(!f.falling);
        assert!((f.pos.y - y).abs() < 1e-3, "it holds its height while the doors open: {}", f.pos.y);
    }

    #[test]
    fn it_weaves_in_and_out_on_its_circle() {
        let d = data();
        assert!(d.cfg.weave_m > 0.0 && d.cfg.weave_per_turn > 0.0, "the data weaves it");
        let mut f = Fortress::new(&d, Vec2::ZERO, 0.0, 42);
        let (mut lo, mut hi) = (f32::MAX, 0.0f32);
        run(&d, &mut f, d.cfg.circle_radius_m / d.cfg.speed_m_s + 1.0);
        for _ in 0..400 {
            run(&d, &mut f, 1.0);
            let r = Vec2::new(f.pos.x, f.pos.z).length();
            (lo, hi) = (lo.min(r), hi.max(r));
        }
        let (r, w) = (d.cfg.circle_radius_m, d.cfg.weave_m);
        assert!(
            lo < r - w * 0.9 && hi > r + w * 0.9 && lo > r - w - 1.0 && hi < r + w + 1.0,
            "{lo}..{hi}"
        );
    }

    #[test]
    fn shooting_what_next_to_bring_down_names_brings_it_down_in_eleven_kills() {
        let (d, mut f) = fortress();
        let mut killed = Vec::new();
        while let Some(i) = f.next_to_bring_down(&d) {
            assert!(killed.len() < 14, "it should be falling by now: {killed:?}");
            f.parts[i].alive = false;
            f.parts[i].hp = 0.0;
            killed.push(d.shape.parts[i].id.clone());
            // the doors open and the hull thrusters light before the next is chosen
            run(&d, &mut f, 6.0);
        }
        assert!(f.falling, "falling after {killed:?}");
        assert_eq!(killed, ["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4", "HL1", "HL2", "HL3"]);
    }

    #[test]
    fn it_appears_over_the_city_centre_and_flies_out_onto_its_circle() {
        let mut d = data();
        d.cfg.weave_m = 0.0; // the weave is `it_weaves_in_and_out_on_its_circle`'s
        let centre = Vec2::new(1200.0, -3400.0);
        let mut f = Fortress::new(&d, centre, 35.0, 42);
        assert!(Vec2::new(f.pos.x, f.pos.z).distance(centre) < 1e-3, "over the centre: {:?}", f.pos);
        assert!((f.pos.y - (35.0 + d.cfg.altitude_m)).abs() < 1e-3);
        // straight out at its speed, nose first, then round
        let out_s = d.cfg.circle_radius_m / d.cfg.speed_m_s;
        run(&d, &mut f, out_s * 0.5);
        let half = Vec2::new(f.pos.x, f.pos.z).distance(centre);
        assert!((half - d.cfg.circle_radius_m * 0.5).abs() < 1.0, "half way out: {half}");
        assert!(f.angle_rad.abs() < 1e-6, "no turning on the way out");
        let v = f.velocity();
        let out = (Vec2::new(f.pos.x, f.pos.z) - centre).normalize();
        assert!(Vec2::new(v.x, v.z).normalize().dot(out) > 0.999, "flying straight out: {v:?}");
        run(&d, &mut f, out_s * 0.5 + 30.0);
        let r = Vec2::new(f.pos.x, f.pos.z).distance(centre);
        assert!((r - d.cfg.circle_radius_m).abs() < 0.01, "on its circle: {r}");
        assert!(f.angle_rad > 0.5, "circling: {}", f.angle_rad);
    }

    pub fn kill(d: &FortressData, f: &mut Fortress, ids: &[&str]) {
        for id in ids {
            let i = d.part(id).unwrap_or_else(|| panic!("no part {id}"));
            f.parts[i].alive = false;
            f.parts[i].hp = 0.0;
        }
    }

    pub fn run(d: &FortressData, f: &mut Fortress, s: f32) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..(s / DT).round() as usize {
            all.extend(f.step(d, &flat, DT));
        }
        all
    }

    #[test]
    fn whole_it_holds_its_height_on_its_circle() {
        let (d, mut f) = fortress();
        let y0 = f.pos.y;
        assert!((y0 - d.cfg.altitude_m).abs() < 1e-3);
        run(&d, &mut f, 30.0);
        assert!((f.pos.y - y0).abs() < 1e-3, "{}", f.pos.y);
        assert!((Vec2::new(f.pos.x, f.pos.z).length() - d.cfg.circle_radius_m).abs() < 0.01);
        let flown = f.angle_rad * d.cfg.circle_radius_m;
        assert!((flown - 30.0 * d.cfg.speed_m_s).abs() < 0.5, "{flown}");
    }

    #[test]
    fn half_its_hull_thrusters_lost_it_falls_level_and_crashes() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4"]);
        run(&d, &mut f, 10.0);
        kill(&d, &mut f, &["HL1", "HR1"]);
        assert!(run(&d, &mut f, 5.0).is_empty(), "two of six: it holds");
        kill(&d, &mut f, &["HR2"]);
        let e = run(&d, &mut f, 1.0);
        assert!(e.contains(&Event::Falling));
        let e = run(&d, &mut f, 120.0);
        assert!(e.contains(&Event::Crashed));
        assert!(f.list_rad.abs() < 1e-4, "no thrust on either side: it falls level, {}", f.list_rad);
    }

    #[test]
    fn two_kills_a_side_hold_at_full_throttle() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "R3", "R4"]);
        let e = run(&d, &mut f, 30.0);
        assert!(e.is_empty(), "{e:?}");
        let l = f.lift(&d);
        assert!(l.holds);
        assert!((l.throttle_outer[0] - 1.0).abs() < 1e-4 && (l.throttle_outer[1] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn with_no_thrust_it_falls_level_at_its_terminal_speed() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4"]);
        kill(&d, &mut f, &["HL1", "HL2", "HL3"]);
        let mut fastest: f32 = 0.0;
        for _ in 0..(40.0 / DT) as usize {
            f.step(&d, &flat, DT);
            fastest = fastest.max(-f.v_m_s);
        }
        // no lift at all: a = -g - k v |v|, terminal sqrt(g / k)
        let terminal = (G_M_S2 / d.cfg.sink_drag_per_m).sqrt();
        assert!(fastest <= terminal + 1e-3, "{fastest} > {terminal}");
        assert!(fastest > terminal * 0.9, "{fastest}");
    }

    #[test]
    fn both_drives_lost_it_hangs_where_it_is() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["drive1", "drive2"]);
        run(&d, &mut f, d.cfg.drive_stop_s + 1.0);
        let a = f.angle_rad;
        run(&d, &mut f, 10.0);
        assert_eq!(f.angle_rad, a);
    }

    #[test]
    fn a_lost_catapult_halves_the_launches() {
        let (d, mut f) = fortress();
        let count = |f: &mut Fortress| (0..(200.0 / DT) as usize).filter(|_| f.launch(&d, 0, DT).is_some()).count();
        let both = count(&mut f.clone());
        kill(&d, &mut f, &["cat1"]);
        let one = count(&mut f);
        assert!(both >= 9, "{both}");
        assert!((one as f32 - both as f32 / 2.0).abs() <= 1.0, "{one} against {both}");
    }

    #[test]
    fn nothing_launches_while_falling() {
        let (d, mut f) = fortress();
        f.falling = true;
        assert!((0..(100.0 / DT) as usize).all(|_| f.launch(&d, 0, DT).is_none()));
    }

    #[test]
    fn a_mech_under_a_live_thruster_is_pushed_down_and_burned() {
        let (d, f) = fortress();
        let i = d.part("L2").unwrap();
        let nozzle = Vec3::new(150.0, 540.0, 45.0);
        let under = nozzle - Vec3::Y * 60.0;
        assert_eq!(
            f.downwash(&d, &[(i, nozzle)], under),
            Some((d.cfg.downwash.push_m_s2, d.cfg.downwash.burn_per_s))
        );
        assert_eq!(f.downwash(&d, &[(i, nozzle)], under + Vec3::X * 80.0), None, "beside the column");
        assert_eq!(f.downwash(&d, &[(i, nozzle)], nozzle + Vec3::Y * 10.0), None, "above the nozzle");
        let mut dead = f.clone();
        dead.parts[i].alive = false;
        assert_eq!(dead.downwash(&d, &[(i, nozzle)], under), None, "a dead thruster blows nothing");
    }

    #[test]
    fn the_same_seed_flies_the_same_fortress() {
        let (d, mut a) = fortress();
        let mut b = a.clone();
        kill(&d, &mut a, &["R1", "R2", "R3"]);
        kill(&d, &mut b, &["R1", "R2", "R3"]);
        run(&d, &mut a, 30.0);
        run(&d, &mut b, 30.0);
        assert_eq!(a, b);
    }

    /// The open air with the fortress's platforms in it (and flat ground under them): what a mech flying to it and
    /// landing on it stands on.
    struct Aboard<'a>(&'a [crate::world::Platform]);

    impl crate::world::Ground for Aboard<'_> {
        fn height_at(&self, _x: f32, _z: f32) -> f32 {
            0.0
        }
        fn push_out(&self, p: Vec2, _radius_m: f32) -> Vec2 {
            p
        }
        fn support_at(&self, x: f32, z: f32, feet_y: f32) -> f32 {
            crate::world::platform::top_under(self.0, x, z, feet_y).map_or(0.0, |t| t.max(0.0))
        }
    }

    fn roster_mech(name: &str) -> crate::mech::MechConfig {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/mechs/{name}.ron"));
        load_ron(&p).unwrap()
    }

    #[test]
    fn every_mech_reaches_the_deck_on_its_jets_as_they_are() {
        // Survey H4 (owner, 2026-10-26): the fortress is reached on the jets as they are. Straight up from the city's
        // ground at full lift, every mech on the roster passes the deck's height with burn to spare.
        let (d, f) = fortress();
        let deck = d.shape.platforms.iter().find(|s| s.name == "deck").unwrap();
        let deck_y = f.pose().transform_point3(Vec3::new(0.0, deck.top_m, 0.0)).y;
        for name in [
            "driller",
            "foreman",
            "harrow",
            "ironhorse",
            "mule",
            "picket",
            "roustabout",
            "sapper",
        ] {
            let cfg = roster_mech(name);
            let mut m = crate::mech::MechState::standing(Vec3::new(2000.0, 0.0, 0.0), 0.0, &cfg.gait);
            let up = crate::mech::MechControls {
                lift_axis: 1.0,
                ..Default::default()
            };
            let mut t = 0.0;
            while m.pos.y < deck_y && t < 60.0 {
                m.step(&up, &cfg, &Aboard(&[]), DT);
                t += DT;
            }
            assert!(m.pos.y >= deck_y, "{name} stalls at {:.0} m of {deck_y:.0} m", m.pos.y);
            let left_s = (1.0 - m.boost.heat) * cfg.jet.burn_s;
            eprintln!("{name}: the deck's {deck_y:.0} m in {t:.1} s, {left_s:.1} s of burn left");
            assert!(!m.boost.locked && left_s > 1.0, "{name}: {left_s:.1} s left");
        }
    }

    #[test]
    fn a_mech_lands_on_the_deck_a_shoulder_and_a_walkway_and_its_jets_recharge_there() {
        // Survey H4: land on the deck, a shoulder or a pylon walkway to recharge and fight there. A mech flying
        // alongside at the speed of the spot below it, let go 10 m over each, its jets nearly spent from the climb, lands,
        // then stands where it landed while the fortress flies on, carried with it, and its jets come back to full.
        let cfg = roster_mech("foreman");
        for name in ["deck", "shoulder", "walkway_L2", "walkway_R3"] {
            let (d, mut f) = fortress();
            let slab = d.shape.platforms.iter().find(|s| s.name == name).unwrap().clone();
            let spot = (slab.min + slab.max) * 0.5;
            let top = slab.top_at(spot).unwrap();
            let start = f.pose().transform_point3(Vec3::new(spot.x, top + 10.0, spot.y));
            let mut m = crate::mech::MechState::standing(start, 0.0, &cfg.gait);
            m.in_air = true;
            // alongside the spot it lands on, which turns with the fortress on its circle
            let mut ahead = f.clone();
            ahead.step(&d, &flat, DT);
            let v = crate::world::Platform {
                slabs: Vec::new(),
                pose: ahead.pose(),
                was: f.pose(),
            }
            .velocity_at(start, DT);
            m.vel = Vec2::new(v.x, v.z);
            m.boost.heat = 0.9;
            let mut was = f.pose();
            let mut landed: Option<Vec3> = None;
            let mut aboard = false;
            for _ in 0..(20.0 / DT) as usize {
                f.step(&d, &flat, DT);
                let platform = crate::world::Platform {
                    slabs: d.shape.platforms.clone(),
                    pose: f.pose(),
                    was,
                };
                was = f.pose();
                // the game's order: the fortress flies and carries what stood on it, then the mechs step
                let r = platform.ride(m.pos, Vec3::new(m.vel.x, m.vel_y_m_s, m.vel.y), aboard, DT);
                (m.pos, m.vel, m.vel_y_m_s, aboard) = (r.feet, Vec2::new(r.vel.x, r.vel.z), r.vel.y, r.aboard);
                m.step(&Default::default(), &cfg, &Aboard(std::slice::from_ref(&platform)), DT);
                if landed.is_none() && !m.in_air {
                    landed = Some(f.pose().inverse().transform_point3(m.pos));
                }
            }
            let local = f.pose().inverse().transform_point3(m.pos);
            assert!(!m.in_air, "{name}: still in the air at {}", m.pos);
            let here = slab.top_at(Vec2::new(local.x, local.z));
            assert!(
                here.is_some_and(|t| (local.y - t).abs() < 0.05),
                "{name}: off it at {local} ({here:?})"
            );
            let landed = landed.unwrap_or_else(|| panic!("{name}: never landed"));
            assert!(local.distance(landed) < 0.5, "{name}: slid from {landed} to {local}");
            assert_eq!(m.boost.heat, 0.0, "{name}: the jets recharged");
            assert!(f.angle_rad > 0.5, "the fortress flew on meanwhile");
        }
    }

    fn island() -> &'static WorldPlan {
        static P: std::sync::OnceLock<WorldPlan> = std::sync::OnceLock::new();
        P.get_or_init(|| {
            let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
            crate::world::generate(42, &load_ron::<crate::world::WorldConfig>(&p).unwrap())
        })
    }

    /// Runs until it is down on the island's ground; the crash's place (x, z).
    fn fall(d: &FortressData, f: &mut Fortress, plan: &WorldPlan) -> Vec2 {
        let ground = |x: f32, z: f32| plan.terrain.height_at(x, z);
        for _ in 0..(300.0 / DT) as usize {
            if f.step(d, &ground, DT).contains(&Event::Crashed) {
                return Vec2::new(f.pos.x, f.pos.z);
            }
        }
        panic!("it never came down: {}", f.pos);
    }

    #[test]
    fn stopping_its_drives_drops_it_where_it_hangs() {
        // Survey H5 (owner, 2026-10-26): it drifts at 15 m/s; stop its drive thrusters and it falls where it is.
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["drive1", "drive2"]);
        run(&d, &mut f, d.cfg.drive_stop_s + 1.0);
        assert_eq!(f.speed_m_s, 0.0);
        let hung = Vec2::new(f.pos.x, f.pos.z);
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4", "HL1", "HL2", "HL3"]);
        let mut down = None;
        for _ in 0..(120.0 / DT) as usize {
            if f.step(&d, &flat, DT).contains(&Event::Crashed) {
                down = Some(Vec2::new(f.pos.x, f.pos.z));
                break;
            }
        }
        let down = down.expect("it came down");
        assert!(down.distance(hung) < 0.01, "it fell straight down: {hung} -> {down}");
    }

    #[test]
    fn with_its_drives_it_drifts_on_at_15_m_s_while_it_falls() {
        let (d, mut f) = fortress();
        assert_eq!(d.cfg.speed_m_s, 15.0);
        let from = f.angle_rad;
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4", "HL1", "HL2", "HL3"]);
        let mut t = 0.0;
        while !f.crashed {
            f.step(&d, &flat, DT);
            t += DT;
        }
        let flown = (f.angle_rad - from) * d.cfg.circle_radius_m;
        eprintln!("with its drives it flew {flown:.0} m on in a {t:.1} s fall");
        assert!((flown - 15.0 * t).abs() < 1.0, "{flown:.0} m in {t:.1} s");
        assert!(flown > 200.0, "the drift carries it {flown:.0} m on before it lands");
    }

    #[test]
    fn its_crash_on_the_city_counts_as_the_citys_damage() {
        // Survey H5 (owner, 2026-10-26): the fortress coming down on the city it attacks is that city's damage, the
        // one number the defence loses on (`mission::city_damage`). Its drives stopped, it falls on the circle over
        // the city; what its wreck breaks is the city's.
        let plan = island();
        let d = data();
        let city = plan
            .cities
            .iter()
            .max_by_key(|c| plan.buildings.iter().filter(|b| b.city == c.id).count())
            .unwrap();
        let mut f = Fortress::new(&d, city.centre, city.ground_m, 42);
        kill(&d, &mut f, &["drive1", "drive2"]);
        run(&d, &mut f, d.cfg.drive_stop_s + 1.0);
        let mut s = Structures::new(plan);
        assert!(f.wreck_breaks(&d, plan, &s).is_empty(), "nothing breaks before it is down");
        let before = crate::mission::city_damage(plan, &s, city.id);
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4", "HR1", "HR2", "HR3"]);
        fall(&d, &mut f, plan);
        let breaks = f.wreck_breaks(&d, plan, &s);
        assert!(!breaks.is_empty(), "it came down on buildings");
        let mut broke = 0;
        for (b, at, r) in &breaks {
            assert_eq!(
                plan.buildings[*b as usize].city, city.id,
                "only the city's own buildings are under it"
            );
            broke += s.break_sphere(*b, *at, *r).0;
        }
        let after = crate::mission::city_damage(plan, &s, city.id);
        eprintln!(
            "the wreck broke {} buildings, {broke} cells: city {} damage {:.1} % -> {:.1} %",
            breaks.len(),
            city.id + 1,
            before * 100.0,
            after * 100.0
        );
        assert!(broke > 0 && after > before, "{before} -> {after}");
    }

    /// Lifts in the first `s` seconds with `on_deck` standing throughout: (seconds, lift part) each time one comes up.
    fn lifts(d: &FortressData, f: &mut Fortress, on_deck: u32, s: f32) -> Vec<(f32, usize)> {
        let mut out = Vec::new();
        for k in 1..=(s / DT).round() as usize {
            for i in f.guard_lift(d, on_deck, DT) {
                out.push((k as f32 * DT, i));
            }
        }
        out
    }

    #[test]
    fn the_deck_guard_is_two_shrikes_and_four_mites_standing_on_its_tops() {
        // Survey H6 (owner, 2026-10-26): 2 Shrikes and 4 Mites on deck from the start.
        let (d, f) = fortress();
        let g = &d.cfg.guard;
        let count = |c: &str| g.start.iter().filter(|p| p.crab == c).count();
        assert_eq!((count("shrike"), count("mite"), g.start.len()), (2, 4, 6));
        assert_eq!(g.lift_crab, "mite");
        for p in &g.start {
            let at = f
                .stand_at(&d, p.at_m)
                .unwrap_or_else(|| panic!("{} at {} stands on nothing", p.crab, p.at_m));
            assert!(at.y > d.cfg.altitude_m + 40.0, "{} stands on its top, {at}", p.crab);
        }
    }

    #[test]
    fn each_working_lift_brings_a_mite_up_every_60_s() {
        let (d, mut f) = fortress();
        let up = lifts(&d, &mut f, 2, 125.0);
        let ids: Vec<(u32, &str)> = up.iter().map(|(t, i)| (t.round() as u32, d.shape.parts[*i].id.as_str())).collect();
        assert_eq!(ids, [(60, "lift1"), (60, "lift2"), (120, "lift1"), (120, "lift2")]);
    }

    #[test]
    fn a_lost_lift_brings_no_more() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["lift1"]);
        let up = lifts(&d, &mut f, 0, 125.0);
        assert!(up.iter().all(|(_, i)| d.shape.parts[*i].id == "lift2") && up.len() == 2, "{up:?}");
        kill(&d, &mut f, &["lift2"]);
        assert!(lifts(&d, &mut f, 0, 300.0).is_empty(), "both lost: no more deck guards");
    }

    #[test]
    fn no_more_come_up_than_six_on_deck() {
        let (d, mut f) = fortress();
        assert!(lifts(&d, &mut f, 6, 300.0).is_empty(), "a full guard brings none");
        let (d, mut f) = fortress();
        assert_eq!(lifts(&d, &mut f, 5, 61.0).len(), 1, "one short: one comes up");
    }

    #[test]
    fn the_lifts_stop_when_the_hangar_is_empty_or_it_falls() {
        let (d, mut f) = fortress();
        assert_eq!(lifts(&d, &mut f, 0, 1000.0).len(), d.cfg.guard.hangar as usize);
        let (d, mut f) = fortress();
        f.falling = true;
        assert!(lifts(&d, &mut f, 0, 300.0).is_empty());
    }
}
