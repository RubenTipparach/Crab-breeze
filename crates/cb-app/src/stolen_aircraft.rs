//! Stealing and flying an aircraft (openspec change `fly-stolen-aircraft`; owner, 2026-10-30: "The player can steal
//! those and planes as well", and in the plan questions a chase camera and a simulated flight model). The pilot gets in
//! and out the way they get into a car (`cb_core::pilot`'s car flow, the access marked `Vehicle::Aircraft`): Use at the
//! door of an aircraft standing still on the ground, the eyes carried to its `EYE_pilot`, the chase camera eased out
//! behind it. While they fly it, `cb_core::flight_model` moves it every fixed step and writes it back into the air
//! traffic (`Traffic::fly`), so it is drawn, hit, broken and crashed by the traffic's own code. Flown into the ground
//! hard, it crashes and throws the pilot clear.

use crate::air_traffic::AirTraffic;
use crate::pilot::{PilotEvents, PilotSim};
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::traffic::CarAccessNow;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::air_traffic::{AircraftSpec, rest_m};
use cb_core::flight_model::{Body, Controls, Damage, Readout, step_plane, step_rotor};
use cb_core::input::Actions;
use cb_core::pilot::{CarAccess, PilotEvent, PilotMode, Vehicle};

/// The aircraft the pilot is in (getting in, flying, getting out), its flight body and the last step's readout.
#[derive(Resource, Default)]
pub struct Flown(pub Option<FlownAircraft>);

/// One stolen aircraft in the pilot's hands.
#[derive(Clone, Debug)]
pub struct FlownAircraft {
    /// Its id in the air traffic.
    pub id: u32,
    /// Its kind.
    pub kind: String,
    /// Its flight state.
    pub body: Body,
    /// Its centre and heading a step ago, for drawing between steps.
    pub prev: (Vec3, f32),
    /// The controls as the pilot holds them (the throttle or collective stays where they leave it).
    pub controls: Controls,
    /// What the cockpit shows: the last step's numbers.
    pub readout: Readout,
}

impl FlownAircraft {
    /// Its centre and heading between the last two steps.
    pub fn drawn(&self, alpha: f32) -> (Vec3, f32) {
        let (h, _, _) = self.body.attitude();
        let turn = cb_core::units::wrap(h - self.prev.1);
        (self.prev.0.lerp(self.body.pos, alpha), self.prev.1 + turn * alpha)
    }
}

/// How fast lift and throttle walk the throttle or collective, a share a second.
const THROTTLE_PER_S: f32 = 0.4;
/// The flaps' settings the weapon wheel steps through: up, take-off, landing.
const FLAP_STEPS: [f32; 3] = [0.0, 0.5, 1.0];
/// Coming down faster than this onto the wheels, or banked past this, is a crash, not a landing.
const CRASH_SINK_M_S: f32 = 7.0;
const CRASH_BANK_RAD: f32 = 0.6;

/// Flies, and lets the pilot board, the aircraft they steal.
pub struct StolenAircraftPlugin;

impl Plugin for StolenAircraftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Flown>()
            .add_game_systems(
                FixedUpdate,
                find_aircraft
                    .in_set(SimSet::Pilot)
                    .after(crate::traffic::find_car)
                    .before(crate::interact::choose)
                    .before(crate::pilot::step_pilot),
            )
            .add_game_systems(FixedUpdate, fly.in_set(SimSet::Mech).before(crate::air_traffic::step_traffic))
            .add_game_systems(Update, show_chute.in_set(crate::sim::PresentSet::Pose));
    }
}

/// An aircraft's access for the pilot's car flow: its door, its seat's eye, where it stands.
fn access(a: &cb_core::air_traffic::Aircraft, spec: &AircraftSpec) -> CarAccess {
    let (door, eye) = a.boarding(spec);
    CarAccess {
        id: a.id,
        vehicle: Vehicle::Aircraft,
        door,
        eye,
        centre: a.pos - Vec3::Y * rest_m(spec),
        heading_rad: a.heading_rad,
        speed_m_s: a.speed_m_s,
        wrecked: a.downed(),
    }
}

/// The aircraft the pilot flies, or, on foot, the stolen-able one whose door is nearer than any car's: standing still
/// on the ground, whole, of a kind that can be flown (`data/aircraft/flight.ron`).
pub fn find_aircraft(
    air: Option<Res<AirTraffic>>,
    flown: Res<Flown>,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mut out: ResMut<CarAccessNow>,
) {
    let Some(air) = air else { return };
    let kinds = &data.0.air_traffic.kinds;
    let flight = &data.0.flight;
    if pilot.curr.mode.in_car() {
        if let Some(f) = &flown.0 {
            out.0 = air.0.get(f.id).and_then(|a| Some(access(a, kinds.get(&a.kind)?)));
        }
        return;
    }
    if pilot.curr.mode != PilotMode::OnFoot {
        return;
    }
    let feet = pilot.curr.feet;
    let near = air
        .0
        .list
        .iter()
        .filter(|a| a.on_ground() && a.speed_m_s < 1.0 && !a.downed() && a.pos.distance(feet) < 80.0)
        .filter(|a| flight.planes.contains_key(&a.kind) || flight.rotors.contains_key(&a.kind))
        .filter_map(|a| Some(access(a, kinds.get(&a.kind)?)))
        .min_by(|x, y| x.door.distance(feet).total_cmp(&y.door.distance(feet)).then(x.id.cmp(&y.id)));
    let Some(plane) = near else { return };
    let car = out.0.map_or(f32::INFINITY, |c| c.door.distance(feet));
    if plane.door.distance(feet) < car {
        out.0 = Some(plane);
    }
}

/// The controls this step from the pilot's actions: the stick on the movement keys (forward pushes the nose down), the
/// rudder or pedals on turn, the throttle or collective walked up and down by lift and throttle and kept where it is
/// left, the brakes on full stop. The skate toggle (Shift) raises and lowers the gear, which will not come up while the
/// aircraft stands on it; the weapon wheel steps the flaps up and down through `FLAP_STEPS`.
fn controls(held: Controls, a: &Actions, on_wheels: bool, dt_s: f32) -> Controls {
    let throttle = (held.throttle + (a.lift_axis + a.throttle_axis) * THROTTLE_PER_S * dt_s).clamp(0.0, 1.0);
    let gear_down = if a.skate_toggle && !(held.gear_down && on_wheels) {
        !held.gear_down
    } else {
        held.gear_down
    };
    let now = FLAP_STEPS.iter().position(|f| (f - held.flaps).abs() < 1e-3).unwrap_or(0) as i32;
    let step = (now + a.weapon_cycle.signum()).clamp(0, FLAP_STEPS.len() as i32 - 1);
    Controls {
        pitch: -a.move_xy.y,
        roll: a.move_xy.x,
        yaw: -a.turn_axis,
        throttle,
        flaps: FLAP_STEPS[step as usize],
        gear_down,
        brake: if a.full_stop { 1.0 } else { 0.0 },
    }
}

/// What damage has left of the aircraft: its broken zones' controls and engines (an engine's side by its capsules).
fn damage(a: &cb_core::air_traffic::Aircraft, spec: &AircraftSpec) -> Damage {
    let zones = spec.zones();
    let engines = zones
        .iter()
        .filter(|z| z.kind == cb_core::air_traffic::model::ZoneKind::Engine)
        .count() as u32;
    let mut d = Damage::from_broken(&[], engines);
    let mut names = Vec::new();
    for (i, z) in zones.iter().enumerate() {
        if !a.broken.get(i).copied().unwrap_or(false) {
            continue;
        }
        if z.kind == cb_core::air_traffic::model::ZoneKind::Engine {
            let x = z.capsules.first().map_or(0.0, |c| (c.0.x + c.1.x) * 0.5);
            d = d.engine_lost(x, engines);
        } else {
            names.push(z.name.as_str());
        }
    }
    let rest = Damage::from_broken(&names, engines);
    Damage {
        roll: rest.roll,
        pitch: rest.pitch,
        yaw: rest.yaw,
        tail_rotor: rest.tail_rotor,
        ..d
    }
}

/// Takes the aircraft the pilot gets into, flies the one they are in, and lets it go when they are out.
#[allow(clippy::too_many_arguments)]
pub fn fly(
    air: Option<ResMut<AirTraffic>>,
    mut flown: ResMut<Flown>,
    events: Res<PilotEvents>,
    access: Res<CarAccessNow>,
    pilot: Res<PilotSim>,
    actions: Res<crate::input::StepActions>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Fixed>>,
    settings: Res<crate::settings::Settings>,
) {
    let Some(mut air) = air else { return };
    let aircraft = access.0.is_some_and(|c| c.vehicle == Vehicle::Aircraft);
    for e in &events.0 {
        match *e {
            PilotEvent::EnteringCar(id) if aircraft => {
                if air.0.steal(id)
                    && let Some(a) = air.0.get(id)
                {
                    let mut body = Body::resting(a.pos, a.heading_rad);
                    body.rot = Quat::from_rotation_y(a.heading_rad) * Quat::from_rotation_x(a.pitch_rad);
                    flown.0 = Some(FlownAircraft {
                        id,
                        kind: a.kind.clone(),
                        body,
                        prev: (a.pos, a.heading_rad),
                        controls: Controls {
                            gear_down: true,
                            ..default()
                        },
                        readout: Readout::default(),
                    });
                    info!("air traffic: the pilot steals {} {id}", a.kind);
                } else {
                    warn!("air traffic: aircraft {id} could not be taken");
                }
            }
            PilotEvent::OutOfCar(_) | PilotEvent::ThrownFromCar(_) if flown.0.is_some() => {
                flown.0 = None;
            }
            // with nobody at the controls it comes down where its momentum takes it, and crashes
            PilotEvent::BailedOut(id) => {
                air.0.crash(id);
                flown.0 = None;
                info!("air traffic: the pilot bails out of aircraft {id}");
            }
            _ => {}
        }
    }
    let Some(f) = flown.0.as_mut() else { return };
    let Some(a) = air.0.get(f.id) else {
        flown.0 = None;
        return;
    };
    if a.downed() || pilot.curr.mode != PilotMode::Driving {
        return;
    }
    let Some(spec) = data.0.air_traffic.kinds.get(&a.kind) else {
        return;
    };
    let dmg = damage(a, spec);
    let dt = time.delta_secs();
    let ground = |x: f32, z: f32| plan.terrain.height_at(x, z);
    let c = controls(f.controls, &actions.0, f.body.on_ground, dt);
    let (heading, _, _) = f.body.attitude();
    f.prev = (f.body.pos, heading);
    let before = f.body.vel;
    let was_down = f.body.on_ground;
    let assists = settings.0.flight_assists();
    f.readout = if let Some(p) = data.0.flight.planes.get(&f.kind) {
        step_plane(p, &mut f.body, c, assists, dmg, &ground, dt)
    } else if let Some(r) = data.0.flight.rotors.get(&f.kind) {
        step_rotor(r, &mut f.body, c, assists, dmg, &ground, dt)
    } else {
        return;
    };
    // the throttle and collective stay where the pilot leaves them; the assists' own stick is not kept
    f.controls = c;
    if f.body.on_ground && !was_down && (before.y < -CRASH_SINK_M_S || f.readout.bank_rad.abs() > CRASH_BANK_RAD) {
        warn!("air traffic: the pilot crashes {} {} at {:.1} m/s down", f.kind, f.id, -before.y);
        air.0.crash(f.id);
        return;
    }
    air.0.fly(
        f.id,
        f.body.pos,
        f.body.vel,
        f.readout.heading_rad,
        f.readout.pitch_rad,
        f.readout.bank_rad,
        f.body.on_ground,
    );
}

/// The cockpit's numbers as one line: airspeed, height over the ground, climb, the throttle, and the stall warning.
pub fn readout_line(f: &FlownAircraft) -> String {
    let r = &f.readout;
    let c = &f.controls;
    format!(
        "{:.0} M/S   {:.0} M   {:+.1} M/S   THR {:.0}%   FLAPS {:.0}%   GEAR {}{}",
        r.airspeed_m_s,
        r.agl_m.max(0.0),
        r.climb_m_s,
        c.throttle * 100.0,
        c.flaps * 100.0,
        if c.gear_down { "DOWN" } else { "UP" },
        if r.stall_warning { "   STALL" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_raises_the_gear_in_the_air_but_never_on_the_ground_and_the_wheel_steps_the_flaps() {
        let down = Controls {
            gear_down: true,
            ..default()
        };
        let shift = Actions {
            skate_toggle: true,
            ..default()
        };
        assert!(controls(down, &shift, true, 0.1).gear_down, "it stands on its gear");
        assert!(!controls(down, &shift, false, 0.1).gear_down, "in the air it comes up");
        let up = controls(down, &shift, false, 0.1);
        assert!(controls(up, &shift, false, 0.1).gear_down, "and down again");
        let wheel = |n: i32| Actions {
            weapon_cycle: n,
            ..default()
        };
        let half = controls(down, &wheel(1), true, 0.1);
        assert_eq!(half.flaps, 0.5);
        assert_eq!(controls(half, &wheel(3), true, 0.1).flaps, 1.0, "one step a turn of the wheel");
        assert_eq!(controls(half, &wheel(-1), true, 0.1).flaps, 0.0);
        assert_eq!(controls(down, &wheel(-1), true, 0.1).flaps, 0.0, "never below up");
    }
}

/// The parachute over a bailed-out pilot: a canopy above their head while `PilotState::chute` is set, seen looking up
/// (first person).
#[derive(Component)]
struct Chute;

fn show_chute(
    mut commands: Commands,
    pilot: Res<PilotSim>,
    fixed: Res<Time<Fixed>>,
    mut chute: Query<(Entity, &mut Transform), With<Chute>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let at = pilot.prev.feet.lerp(pilot.curr.feet, fixed.overstep_fraction()) + Vec3::Y * 8.0;
    match (chute.single_mut(), pilot.curr.chute) {
        (Ok((_, mut t)), true) => t.translation = at,
        (Ok((e, _)), false) => commands.entity(e).despawn(),
        (Err(_), true) => {
            let cloth = materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.36, 0.12),
                perceptual_roughness: 0.9,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                Name::new("parachute"),
                Chute,
                Mesh3d(meshes.add(Sphere::new(4.0).mesh().uv(24, 12))),
                MeshMaterial3d(cloth),
                Transform::from_translation(at).with_scale(Vec3::new(1.0, 0.4, 1.0)),
            ));
        }
        (Err(_), false) => {}
    }
}
