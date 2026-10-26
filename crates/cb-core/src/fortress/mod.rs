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
use crate::rng::SimRng;
use crate::weapons::WeaponSpec;
use crate::world::Slab;
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
}

fn forty() -> f32 {
    40.0
}

fn two() -> f32 {
    2.0
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
    /// Every kind of part by name.
    pub kinds: BTreeMap<String, KindSpec>,
    /// Fighters.
    pub launch: LaunchSpec,
    /// Downwash.
    pub downwash: DownwashSpec,
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
        ] {
            positive(k, v)?;
        }
        within("lift_max_share", self.lift_max_share, 1e-3, 0.5)?;
        within("hull_max_share", self.hull_max_share, 1e-3, 0.5)?;
        finite("launch.first_s", self.launch.first_s)?;
        for (name, k) in &self.kinds {
            let f = |x: &str| format!("kinds.{name}.{x}");
            positive(&f("hp"), k.hp)?;
            within(&f("armour_mm"), k.armour_mm, 0.0, 10_000.0)?;
            positive(&f("capsule.radius_m"), k.capsule.radius_m)?;
            within(&f("capsule.half_m"), k.capsule.half_m, 0.0, 1000.0)?;
            if let Some(g) = &k.gun {
                g.weapon.validate()?;
                positive(&f("gun.range_m"), g.range_m)?;
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
}

impl GunState {
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
    rng: SimRng,
}

impl Fortress {
    /// A fortress on its circle round `centre`, `cfg.altitude_m` over `ground_m`, every part whole.
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
            rng: SimRng::new(seed).fork("fortress"),
        };
        f.place(&d.cfg, ground_m + d.cfg.altitude_m);
        f
    }

    fn place(&mut self, cfg: &FortressConfig, y: f32) {
        let (s, c) = self.angle_rad.sin_cos();
        let xz = self.centre + Vec2::new(c, s) * cfg.circle_radius_m;
        self.pos = Vec3::new(xz.x, y, xz.y);
        // nose along the circle, the way it flies (angle increasing: +x -> +z)
        self.yaw_rad = (-s).atan2(c);
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
        let (s, c) = self.angle_rad.sin_cos();
        Vec3::new(-s, 0.0, c) * self.speed_m_s + Vec3::Y * self.v_m_s
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
        // The doors open when a side's outer thrusters can no longer carry its half.
        let outer = lift(d, &self.parts, false);
        if self.doors_s.is_none() && !outer.holds {
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
        // Falling: short of its weight even with every hull thruster it has.
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
        self.angle_rad += self.speed_m_s / cfg.circle_radius_m * dt_s;
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

    pub fn fortress() -> (FortressData, Fortress) {
        let d = data();
        let f = Fortress::new(&d, Vec2::ZERO, 0.0, 42);
        (d, f)
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
    fn three_kills_on_one_side_open_the_hull_thrusters() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3"]);
        let e = run(&d, &mut f, 1.0);
        assert!(e.contains(&Event::DoorsOpening));
        assert!(!f.falling, "the hull thrusters will hold it");
        let e = run(&d, &mut f, 20.0);
        assert!(e.contains(&Event::HullLit));
        let y = f.pos.y;
        assert!(y < d.cfg.altitude_m - 5.0, "it sank while the doors opened: {y}");
        run(&d, &mut f, 20.0);
        assert!((f.pos.y - y).abs() < 0.5, "and holds again: {} -> {}", y, f.pos.y);
    }

    #[test]
    fn two_hull_thrusters_lost_on_that_side_bring_it_down() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3"]);
        run(&d, &mut f, 10.0);
        kill(&d, &mut f, &["HL1", "HL2"]);
        let e = run(&d, &mut f, 1.0);
        assert!(e.contains(&Event::Falling));
        let e = run(&d, &mut f, 120.0);
        assert!(e.contains(&Event::Crashed));
        assert!(f.crashed);
        assert!(f.list_rad > 0.1, "it lists toward its weak side, port: {}", f.list_rad);
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
    fn the_sink_speed_never_passes_its_terminal_speed() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "HL1", "HL2", "HL3"]);
        let mut fastest: f32 = 0.0;
        for _ in 0..(40.0 / DT) as usize {
            f.step(&d, &flat, DT);
            fastest = fastest.max(-f.v_m_s);
        }
        // one side at none: L = 0.5 W, terminal sqrt(g / 2k)
        let terminal = (G_M_S2 * 0.5 / d.cfg.sink_drag_per_m).sqrt();
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
}
