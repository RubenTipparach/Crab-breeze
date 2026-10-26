//! Civil air traffic's tuning (openspec change `civil-air-traffic`): every kind of aircraft, how near
//! the pilot traffic lives, how routes are flown between the air bases, how a fight is avoided and how a
//! downed aircraft falls. `data/world/air_traffic.ron`, validated on load.

use crate::data::{Validate, finite, finite3, positive, positive_range, within};
use serde::Deserialize;
use std::collections::BTreeMap;

/// What an aircraft is for, and so how it flies.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Flies routes between air bases; low wing, two engines under it.
    Airliner,
    /// Flies routes between air bases; high wing, four engines, a T tail. The shape the career loop's
    /// mech air drop reuses.
    Freighter,
    /// Circles a city; high wing, a propeller.
    LightPlane,
    /// Circles a city; a cabin, a tail boom and a rotor.
    Helicopter,
}

impl Role {
    /// True for the kinds that fly between bases; the rest circle cities.
    pub fn flies_routes(self) -> bool {
        matches!(self, Role::Airliner | Role::Freighter)
    }
}

/// One kind of civil aircraft, in data.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AircraftSpec {
    /// How it flies and is drawn.
    pub role: Role,
    /// Nose to tail (a helicopter: rotor tip to tail), metres.
    pub length_m: f32,
    /// Wing span (a helicopter: rotor diameter), metres.
    pub span_m: f32,
    /// Ground to the top of the fin (or the rotor head), metres.
    pub height_m: f32,
    /// The fuselage's diameter, metres.
    pub fuselage_m: f32,
    /// Cruising speed; the speed it circles a city at.
    pub cruise_m_s: f32,
    /// Route aircraft: the final approach speed, and the speed it lifts off at.
    pub approach_m_s: f32,
    /// Most it climbs, metres a second.
    pub climb_m_s: f32,
    /// Most it descends, metres a second.
    pub descent_m_s: f32,
    /// Most it turns, degrees a second.
    pub turn_deg_s: f32,
    /// Height over the ground it cruises or circles at, metres.
    pub cruise_agl_m: f32,
    /// City aircraft: the radius it circles a city's centre at, [low, high] metres.
    #[serde(default)]
    pub orbit_radius_m: [f32; 2],
    /// Most of this kind near the pilot at once.
    pub max_near: u32,
    /// Structure points.
    pub hp: f32,
    /// Armour a shot must beat to do damage, millimetres of rolled steel equivalent.
    pub armour_mm: f32,
    /// The crash's explosion, megajoules (fuel and speed).
    pub crash_mj: f32,
    /// Its jet engines' sound, played pitched by this much (none: no engine sound yet).
    #[serde(default)]
    pub engine_pitch: Option<f32>,
    /// Livery colour, sRGB 0..1.
    pub tint: [f32; 3],
}

/// Where traffic lives: only near the pilot.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct NearConfig {
    /// Beyond this from the pilot an aircraft may appear in any direction (a speck at most), metres.
    pub spawn_far_m: f32,
    /// Beyond this, and out of the pilot's view cone, an aircraft may appear, metres.
    pub spawn_near_m: f32,
    /// Half the view cone an aircraft never appears inside nearer than `spawn_far_m`, degrees.
    pub view_half_deg: f32,
    /// An aircraft this far from the pilot is removed, metres.
    pub despawn_m: f32,
    /// Seconds between looks for room to spawn.
    pub check_s: f32,
}

impl Default for NearConfig {
    fn default() -> Self {
        Self {
            spawn_far_m: 10_000.0,
            spawn_near_m: 3_000.0,
            view_half_deg: 75.0,
            despawn_m: 14_000.0,
            check_s: 4.0,
        }
    }
}

/// How routes are flown between the bases.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RouteConfig {
    /// The glide path's angle, degrees.
    pub glide_deg: f32,
    /// The final approach fix: this far out along the strip's centre line, metres.
    pub final_m: f32,
    /// After lift-off it climbs straight ahead to this height over the ground, metres.
    pub climb_out_m: f32,
    /// Off the final approach it never flies lower than this over the ground, metres.
    pub min_clear_m: f32,
    /// The strip runs along the apron's long side, this far in from each end, metres.
    pub strip_margin_m: f32,
    /// The strip's line, this far from the base's centre toward its front (the pads' side), metres.
    pub strip_front_m: f32,
    /// After stopping it waits this long on the apron, then leaves the world, seconds.
    pub park_s: f32,
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            glide_deg: 3.0,
            final_m: 4_000.0,
            climb_out_m: 150.0,
            min_clear_m: 120.0,
            strip_margin_m: 40.0,
            strip_front_m: 60.0,
            park_s: 30.0,
        }
    }
}

/// Keeping clear of a fight.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct AvoidConfig {
    /// A fight is everywhere within this of an enemy, metres.
    pub keep_clear_m: f32,
}

impl Default for AvoidConfig {
    fn default() -> Self {
        Self { keep_clear_m: 2_500.0 }
    }
}

/// A downed aircraft's fall.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FallConfig {
    /// Its speed over the ground decays at this rate, per second.
    pub drag_per_s: f32,
    /// It turns as it falls, degrees a second (a helicopter three times as fast).
    pub spin_deg_s: f32,
    /// Its wreck is laid down as at most this many pieces.
    pub wreck_pieces: u32,
}

impl Default for FallConfig {
    fn default() -> Self {
        Self {
            drag_per_s: 0.15,
            spin_deg_s: 25.0,
            wreck_pieces: 6,
        }
    }
}

/// `data/world/air_traffic.ron`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct AirTrafficConfig {
    /// Every kind of aircraft by name.
    pub kinds: BTreeMap<String, AircraftSpec>,
    /// Where traffic lives.
    pub near: NearConfig,
    /// Routes between bases.
    pub routes: RouteConfig,
    /// Keeping clear of a fight.
    pub avoid: AvoidConfig,
    /// A downed aircraft's fall.
    pub fall: FallConfig,
}

impl Validate for AirTrafficConfig {
    fn validate(&self) -> Result<(), String> {
        for (name, k) in &self.kinds {
            let f = |field: &str| format!("kinds.{name}.{field}");
            for (field, v) in [
                ("length_m", k.length_m),
                ("span_m", k.span_m),
                ("height_m", k.height_m),
                ("fuselage_m", k.fuselage_m),
                ("cruise_m_s", k.cruise_m_s),
                ("approach_m_s", k.approach_m_s),
                ("climb_m_s", k.climb_m_s),
                ("descent_m_s", k.descent_m_s),
                ("turn_deg_s", k.turn_deg_s),
                ("cruise_agl_m", k.cruise_agl_m),
                ("hp", k.hp),
                ("crash_mj", k.crash_mj),
            ] {
                positive(&f(field), v)?;
            }
            if !(k.armour_mm >= 0.0 && k.armour_mm.is_finite()) {
                return Err(format!("`{}` must be zero or more", f("armour_mm")));
            }
            if k.fuselage_m > k.height_m {
                return Err(format!("`{}` must not be above `height_m`", f("fuselage_m")));
            }
            finite3(&f("tint"), k.tint)?;
            if let Some(p) = k.engine_pitch {
                positive(&f("engine_pitch"), p)?;
            }
            if k.role.flies_routes() {
                if k.approach_m_s > k.cruise_m_s {
                    return Err(format!("`{}` must not be above `cruise_m_s`", f("approach_m_s")));
                }
                // It must be able to descend at least as steeply as the glide path from cruise.
                let need = k.cruise_m_s * self.routes.glide_deg.to_radians().tan();
                if k.descent_m_s < need {
                    return Err(format!(
                        "`{}` must be at least {need:.1} m/s to hold the glide path from cruise",
                        f("descent_m_s")
                    ));
                }
            } else {
                positive_range(&f("orbit_radius_m"), k.orbit_radius_m)?;
            }
        }
        let n = &self.near;
        positive("near.spawn_near_m", n.spawn_near_m)?;
        positive("near.spawn_far_m", n.spawn_far_m)?;
        positive("near.despawn_m", n.despawn_m)?;
        positive("near.check_s", n.check_s)?;
        within("near.view_half_deg", n.view_half_deg, 0.0, 180.0)?;
        if !(n.spawn_near_m <= n.spawn_far_m && n.spawn_far_m < n.despawn_m) {
            return Err("near: spawn_near_m <= spawn_far_m < despawn_m".into());
        }
        let r = &self.routes;
        within("routes.glide_deg", r.glide_deg, 1.0, 10.0)?;
        positive("routes.final_m", r.final_m)?;
        positive("routes.climb_out_m", r.climb_out_m)?;
        positive("routes.min_clear_m", r.min_clear_m)?;
        positive("routes.strip_margin_m", r.strip_margin_m)?;
        positive("routes.park_s", r.park_s)?;
        finite("routes.strip_front_m", r.strip_front_m)?;
        positive("avoid.keep_clear_m", self.avoid.keep_clear_m)?;
        positive("fall.drag_per_s", self.fall.drag_per_s)?;
        positive("fall.spin_deg_s", self.fall.spin_deg_s)?;
        if self.fall.wreck_pieces == 0 {
            return Err("`fall.wreck_pieces` must be above zero".into());
        }
        Ok(())
    }
}
