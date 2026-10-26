//! Car traffic and the car you drive in the game (openspec changes `car-traffic` and `car-driving`): builds the
//! lane graph once from the plan, keeps the lanes near the pilot filled and steps every car in the fixed step
//! through `cb_core::traffic`, hands it what the fight does (foot strikes, explosions, mechs moving through),
//! takes and leaves cars for the pilot, and draws the cars, their fires and their running drivers.
//!
//! Harm to a car is published as `CivilianHarmed` and counted in `CivilianToll`; `civil_standing` charges it
//! to the career's standing per base (survey W34, W37), without this module knowing about careers.
//!
//! The cars are drawn from their block-out models (`tools/blender/build_cars.py`), stand-ins until the
//! detailed models are approved: a crushed car is its block-out pressed flat, a burning one carries the
//! `burning` stream, and a running driver is a 1.8 m block-out figure.

use crate::ground_marks::{Contact, GroundContact};
use crate::mech::{MechSim, MechSpec, Piloted};
use crate::pilot::{PilotEvents, PilotSim};
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use cb_core::car::{CarSpec, Drive, Obstacle};
use cb_core::civilians::Doors;
use cb_core::pilot::{CarAccess, PilotEvent, PilotMode};
use cb_core::traffic::{CarState, Harm, MechNear, Traffic, TrafficView};
use cb_core::world::lanes::LaneGraph;
use cb_core::world::{Ground, OnRoads, Scene as WorldScene};
use std::collections::BTreeMap;

/// Ids from here up in the weapons' part list are cars (mechs below 5000, vehicles from 5000, the fortress
/// at 900 000).
pub const CAR_ID_BASE: u32 = 2_000_000;

/// The island's lanes, built once from the plan.
#[derive(Resource)]
pub struct Roads(pub LaneGraph);

/// The traffic, and every car's place at the step before, for drawing between steps.
#[derive(Resource)]
pub struct TrafficSim {
    /// The cars.
    pub traffic: Traffic,
    /// Each car's ground point and heading at the step before.
    pub prev: BTreeMap<u32, (Vec3, f32)>,
    filled: bool,
}

impl TrafficSim {
    /// Car `c` as it is drawn between fixed steps at `alpha` (0..1 past the last step): its ground point and
    /// heading, eased from the step before. The cars' models and the chase camera both place by it.
    pub fn drawn_pose(&self, c: &cb_core::traffic::Car, alpha: f32) -> (Vec3, f32) {
        let (p0, h0) = self.prev.get(&c.id).copied().unwrap_or((c.pos, c.heading_rad));
        let turned = (c.heading_rad - h0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        (p0.lerp(c.pos, alpha), h0 + turned * alpha)
    }
}

/// The car the pilot can reach now, or the one they are in (for the pilot's step and the prompt).
#[derive(Resource, Default)]
pub struct CarAccessNow(pub Option<CarAccess>);

/// A car was harmed (survey W34's hook: the career charges standing for `by_player` harm to an
/// `occupied` car).
#[derive(Message, Clone, Copy, Debug)]
pub struct CivilianHarmed(pub Harm);

/// Running counts of harm to cars, for whatever keeps score.
#[derive(Resource, Default, Debug)]
pub struct CivilianToll {
    /// Occupied cars the player's fire, feet or car harmed.
    pub by_player: u32,
    /// Every harmed car.
    pub all: u32,
}

/// Draws a car.
#[derive(Component)]
struct CarDraw {
    id: u32,
    kind: usize,
    fire: Option<Entity>,
}

/// The dashboard parts of the car being driven: found once its scene has spawned.
#[derive(Component)]
struct Dash {
    needle: Entity,
    needle_rest: Quat,
    wheel: Entity,
    wheel_rest: Quat,
}

/// A car's four wheels, found once its scene has spawned, and how far they have turned.
#[derive(Component)]
struct Spin {
    wheels: [(Entity, Quat); 4],
    angle_rad: f32,
}

/// Draws a running driver.
#[derive(Component)]
struct RunnerDraw(u32);

/// The steering wheel turns this many times the front wheels' angle.
const STEERING_RATIO: f32 = 12.0;
/// A crushed car is drawn at this share of its height until crushed variants are modelled.
const CRUSHED_SCALE: f32 = 0.35;

/// Traffic and driving.
pub struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        let plan = app.world().resource::<Plan>().0.clone();
        let data = &app.world().resource::<Data>().0;
        let started = std::time::Instant::now();
        let graph = LaneGraph::build(&plan.cities, &plan.highways, &data.traffic.cfg.lanes, data.world.arterial_width_m);
        info!(
            "traffic: {} lanes at {} junctions, built in {:.0} ms",
            graph.lanes.len(),
            graph.nodes.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        app.insert_resource(Roads(graph))
            .insert_resource(TrafficSim {
                traffic: Traffic::new(plan.seed),
                prev: BTreeMap::new(),
                filled: false,
            })
            .init_resource::<CarAccessNow>()
            .init_resource::<CivilianToll>()
            .add_message::<CivilianHarmed>()
            .add_game_systems(FixedUpdate, find_car.in_set(SimSet::Pilot).before(crate::pilot::step_pilot))
            .add_game_systems(FixedUpdate, step_traffic.in_set(SimSet::Mech).before(crate::mech::step_mech))
            .add_game_systems(
                Update,
                (draw_cars, find_dash, turn_dash, find_wheels, spin_wheels, draw_runners).chain(),
            )
            .add_game_systems(Update, report_harm)
            .add_world_spawn(spawn_speed_readout)
            .add_game_systems(Update, show_speed);
    }
}

/// The car the pilot is in, or the one whose driver's door is nearest their feet.
pub fn find_car(sim: Res<TrafficSim>, pilot: Res<PilotSim>, data: Res<Data>, mut out: ResMut<CarAccessNow>) {
    let kinds = &data.0.traffic.kinds;
    let access = |c: &cb_core::traffic::Car| {
        let spec = &kinds[c.kind as usize];
        CarAccess {
            id: c.id,
            door: CarSpec::to_world(c.pos, c.heading_rad, spec.door_driver_m),
            eye: CarSpec::to_world(c.pos, c.heading_rad, spec.eye_driver_m),
            centre: c.pos,
            heading_rad: c.heading_rad,
            speed_m_s: c.vel().length(),
            wrecked: c.state.wreck(),
        }
    };
    let t = &sim.traffic;
    let stopped = data.0.pilot.car_stopped_m_s;
    out.0 = if pilot.curr.mode.in_car() {
        t.driven().map(access)
    } else {
        let feet = pilot.curr.feet;
        t.cars
            .iter()
            .filter(|c| c.pos.distance(feet) < 25.0 && !c.state.wreck())
            .map(access)
            // the stopped car's door first: one driving past nearer never hides the parked car the pilot stands at
            .min_by(|a, b| {
                let key = |c: &CarAccess| c.door.distance(feet) + if c.speed_m_s > stopped { 100.0 } else { 0.0 };
                key(a).total_cmp(&key(b))
            })
    };
}

/// True when the point is inside the view cone from `eye` along `fwd` (half-angle `half_rad`).
fn in_cone(eye: Vec3, fwd: Vec3, half_rad: f32, p: Vec3) -> bool {
    let d = p - eye;
    d.length_squared() < 1.0 || d.normalize().dot(fwd) > half_rad.cos()
}

/// True when a building or the ground stands between `eye` and `p`.
fn covered(plan: &cb_core::world::WorldPlan, eye: Vec3, p: Vec3) -> bool {
    let n = (eye.distance(p) / 10.0).ceil().max(1.0) as usize;
    (1..n).any(|k| plan.solid_at(eye.lerp(p, k as f32 / n as f32)))
}

#[allow(clippy::too_many_arguments)]
fn step_traffic(
    mut sim: ResMut<TrafficSim>,
    roads: Res<Roads>,
    data: Res<Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    pilot: Res<PilotSim>,
    events: Res<PilotEvents>,
    actions: Res<crate::input::StepActions>,
    ready: Res<crate::loading::GameReady>,
    mechs: Query<(&MechSim, &MechSpec, Option<&Piloted>)>,
    piloted_id: Query<&crate::mech::MechId, With<Piloted>>,
    mut contacts: MessageReader<GroundContact>,
    mut blasts: MessageReader<crate::blast::Explosion>,
    mut harmed: MessageWriter<CivilianHarmed>,
    mut toll: ResMut<CivilianToll>,
    (time, doors): (Res<Time<Fixed>>, Option<Res<crate::street_doors::RefugeDoors>>),
) {
    let _t = crate::perf_hud::slow("step_traffic");
    let d = &data.0.traffic;
    let sim = &mut *sim;
    let scene = WorldScene {
        plan: &plan,
        dug: &live.dug,
        trees: None,
        structures: Some(&live.structures),
        platforms: &live.platforms,
        interiors: None,
    };
    // the wheels meet the road as it is drawn, `road_lift_m` over its grade
    let ground = OnRoads {
        ground: &scene,
        plan: &plan,
    };
    // the pilot and the cars
    for e in &events.0 {
        match *e {
            PilotEvent::EnteringCar(id) => {
                // the driver runs from the pilot for the nearest street door, as people on foot do (the
                // buildings' real doors, `street_doors::RefugeDoors`, in the people's reach); with none in
                // reach, straight away from the pilot
                let pilot_at = Vec2::new(pilot.curr.feet.x, pilot.curr.feet.z);
                let reach = data.0.people.cfg.door_reach_m;
                let refuge = |p: Vec2| {
                    doors
                        .as_deref()
                        .and_then(|d| d.0.refuge(p, pilot_at, reach))
                        .unwrap_or_else(|| p + (p - pilot_at).normalize_or(Vec2::X) * 30.0)
                };
                if sim.traffic.take(d, id, &ground, &refuge) {
                    info!("traffic: the pilot takes car {id}");
                } else {
                    warn!("traffic: car {id} could not be taken");
                }
            }
            PilotEvent::OutOfCar(id) => sim.traffic.leave(id),
            _ => {}
        }
    }
    // what the fight does: feet coming down, blasts, mechs moving through
    let marks = &data.0.ground_marks;
    for c in contacts.read() {
        if let Contact::Step {
            foot, heading_rad, lead_m, ..
        } = c.contact
        {
            let sole = marks.sole_centre(foot, heading_rad, lead_m);
            sim.traffic.crush(d, Vec2::new(sole.x, sole.z), c.piloted);
        }
    }
    let pilot_mech = piloted_id.iter().next().map(|m| m.0);
    for b in blasts.read() {
        // whose blast it was, from the explosion itself (civilians' `Explosion::by`)
        let mine = b.by.is_some() && b.by == pilot_mech;
        sim.traffic.blast(d, b.at, b.energy_mj, mine);
    }
    let near: Vec<MechNear> = mechs
        .iter()
        .filter(|(m, ..)| !m.dead)
        .map(|(m, spec, piloted)| MechNear {
            at: Vec2::new(m.curr.pos.x, m.curr.pos.z),
            vel: m.curr.vel,
            radius_m: spec.locomotion.body_radius_m,
            piloted: piloted.is_some(),
        })
        .collect();
    // the mechs' legs stop a car dead; a person on foot is in the road
    let mut obstacles: Vec<Obstacle> = near
        .iter()
        .enumerate()
        .map(|(i, m)| Obstacle {
            id: u32::MAX - i as u32,
            at: m.at,
            radius_m: m.radius_m,
            vel: m.vel,
            mass_kg: f32::INFINITY,
        })
        .collect();
    if pilot.curr.mode == PilotMode::OnFoot {
        obstacles.push(Obstacle {
            id: u32::MAX - 100,
            at: Vec2::new(pilot.curr.feet.x, pilot.curr.feet.z),
            radius_m: 0.4,
            vel: pilot.curr.velocity,
            mass_kg: 80.0,
        });
    }
    let a = &actions.0;
    let driver = (pilot.curr.mode == PilotMode::Driving).then_some(Drive {
        throttle: a.move_xy.y,
        steer: a.move_xy.x,
        handbrake: a.lift_axis > 0.0,
    });
    sim.prev = sim.traffic.cars.iter().map(|c| (c.id, (c.pos, c.heading_rad))).collect();
    let view = TrafficView {
        graph: &roads.0,
        data: d,
        ground: &ground,
        mechs: &near,
        obstacles: &obstacles,
        driver,
    };
    let eye = pilot.curr.eye(&data.0.pilot);
    sim.traffic.step(&view, eye, time.delta_secs());
    // fill the lanes near the eye: everything at once behind the loading screen, then a few a step out of sight
    let fwd = Quat::from_rotation_y(pilot.curr.yaw_rad) * Quat::from_rotation_x(pilot.curr.pitch_rad) * Vec3::NEG_Z;
    let half = {
        // the view's half diagonal at a 16:9 screen
        let v = (data.0.pilot.view_fov_deg.to_radians() * 0.5).tan();
        (v * (1.0f32 + (16.0f32 / 9.0).powi(2)).sqrt()).atan() + 0.1
    };
    let initial = !sim.filled || !ready.0;
    let seen = |p: Vec3| !initial && in_cone(eye, fwd, half, p);
    let behind = |p: Vec3| covered(&plan, eye, p);
    let budget = if initial { u32::MAX } else { d.cfg.spawn_per_step };
    let mech_bodies: Vec<(Vec2, f32)> = near.iter().map(|m| (m.at, m.radius_m)).collect();
    sim.traffic.fill(&roads.0, d, &ground, eye, &seen, &behind, &mech_bodies, budget);
    sim.filled = true;
    for h in sim.traffic.take_harms() {
        toll.all += 1;
        if h.by_player && h.occupied {
            toll.by_player += 1;
        }
        harmed.write(CivilianHarmed(h));
    }
    sim.traffic.take_driver_thrown();
}

/// Says in the log when the player harms a civilian's car. The career's standing reads `CivilianHarmed` in
/// `civil_standing`.
fn report_harm(mut harmed: MessageReader<CivilianHarmed>, toll: Res<CivilianToll>) {
    for CivilianHarmed(h) in harmed.read() {
        if h.by_player && h.occupied {
            info!(
                "traffic: the player harmed car {} ({:?}) at {:.0?}; {} of {} harmed cars so far",
                h.car, h.cause, h.at, toll.by_player, toll.all
            );
        }
    }
}

/// The weapons' hit shapes for cars near a shot this step (a capsule along each body), with ids from
/// `CAR_ID_BASE`. Only cars within reach of a shot are listed, so a full street costs the shots nothing.
pub fn car_capsules(sim: &TrafficSim, data: &Data, near: &[Vec3], reach_m: f32) -> Vec<cb_core::weapons::PartCapsule> {
    if near.is_empty() {
        return Vec::new();
    }
    sim.traffic
        .capsules(&data.0.traffic)
        .into_iter()
        .filter(|(_, a, b, _)| near.iter().any(|p| p.distance((*a + *b) * 0.5) < reach_m))
        .map(|(id, a, b, radius_m)| cb_core::weapons::PartCapsule {
            mech: CAR_ID_BASE + id,
            part: 0,
            a,
            b,
            radius_m,
        })
        .collect()
}

/// Places every car's block-out between steps; builds one when a car appears and removes it when it goes.
#[allow(clippy::too_many_arguments)]
fn draw_cars(
    mut commands: Commands,
    sim: Res<TrafficSim>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    fixed: Res<Time<Fixed>>,
    mut drawn: Query<(Entity, &mut CarDraw, &mut Transform)>,
    mut known: Local<BTreeMap<u32, Entity>>,
    mut scenes: Local<Vec<Handle<WorldAsset>>>,
) {
    let _t = crate::perf_hud::slow("draw_cars");
    let kinds = &data.0.traffic.kinds;
    if scenes.is_empty() {
        *scenes = kinds
            .iter()
            .map(|k| assets.load(GltfAssetLabel::Scene(0).from_asset(k.model.clone())))
            .collect();
    }
    let alpha = fixed.overstep_fraction();
    let t = &sim.traffic;
    let mut seen = std::collections::BTreeSet::new();
    for (e, mut d, mut tf) in &mut drawn {
        let Some(c) = t.get(d.id) else {
            commands.entity(e).despawn();
            continue;
        };
        seen.insert(d.id);
        let (pos, heading) = sim.drawn_pose(c, alpha);
        let (pitch, roll) = c.body.as_ref().map_or((0.0, 0.0), |b| (b.pitch_rad, b.roll_rad));
        // Written only when it moved: a parked or waiting car left alone is not walked by the transform
        // propagation (its whole scene, every frame: the main world's largest cost in a city, 2026-10-26).
        tf.set_if_neq(Transform {
            translation: pos,
            rotation: Quat::from_rotation_y(heading) * Quat::from_rotation_x(pitch) * Quat::from_rotation_z(-roll),
            scale: Vec3::new(1.0, if c.state == CarState::Crushed { CRUSHED_SCALE } else { 1.0 }, 1.0),
        });
        let burning = matches!(c.state, CarState::Burning { .. });
        match (burning, d.fire) {
            (true, None) => {
                let h = kinds[d.kind].size_m[1];
                let fire = commands
                    .spawn((
                        crate::fx::FxStream {
                            recipe: "burning",
                            dir: Vec3::Y,
                            on: true,
                            density: 1.0,
                        },
                        Transform::from_xyz(0.0, h * 0.6, 0.0),
                        ChildOf(e),
                    ))
                    .id();
                d.fire = Some(fire);
            }
            (false, Some(fire)) => {
                commands.entity(fire).despawn();
                d.fire = None;
            }
            _ => {}
        }
    }
    known.retain(|id, _| seen.contains(id));
    let new: Vec<&cb_core::traffic::Car> = t
        .cars
        .iter()
        .filter(|c| !seen.contains(&c.id) && !known.contains_key(&c.id))
        .collect();
    for c in new {
        let kind = c.kind as usize;
        let e = commands
            .spawn((
                Name::new(format!("car {} {} (block-out stand-in)", data.0.traffic.cfg.kinds[kind], c.id)),
                WorldAssetRoot(scenes[kind].clone()),
                Transform::from_translation(c.pos).with_rotation(Quat::from_rotation_y(c.heading_rad)),
                CarDraw {
                    id: c.id,
                    kind,
                    fire: None,
                },
            ))
            .id();
        known.insert(c.id, e);
    }
}

/// Once the driven car's scene has spawned, finds its speedometer needle and steering wheel.
fn find_dash(
    mut commands: Commands,
    sim: Res<TrafficSim>,
    cars: Query<(Entity, &CarDraw), Without<Dash>>,
    children: Query<&Children>,
    names: Query<(&Name, &Transform)>,
) {
    let Some(driven) = sim.traffic.driven() else { return };
    let Some((root, _)) = cars.iter().find(|(_, d)| d.id == driven.id) else {
        return;
    };
    let mut needle = None;
    let mut wheel = None;
    for e in children.iter_descendants(root) {
        if let Ok((n, t)) = names.get(e) {
            match n.as_str() {
                "NEEDLE_speed" => needle = Some((e, t.rotation)),
                "STEER_wheel" => wheel = Some((e, t.rotation)),
                _ => {}
            }
        }
    }
    if let (Some((needle, needle_rest)), Some((wheel, wheel_rest))) = (needle, wheel) {
        commands.entity(root).insert(Dash {
            needle,
            needle_rest,
            wheel,
            wheel_rest,
        });
    }
}

/// The speedometer needle reads the car's simulated speed and the steering wheel follows the front wheels
/// (CLAUDE.md section 6.1: the dashboard shows the number the simulation uses).
fn turn_dash(sim: Res<TrafficSim>, data: Res<Data>, dashes: Query<(&CarDraw, &Dash)>, mut tf: Query<&mut Transform>) {
    for (d, dash) in &dashes {
        let Some(c) = sim.traffic.get(d.id) else { continue };
        let spec = &data.0.traffic.kinds[d.kind];
        let speed = c.vel().length();
        // the needle points up at mid-scale and sweeps clockwise, as the driver sees it
        let angle = spec.speedo_needle_rad(speed) - spec.speedo_sweep_deg.to_radians() * 0.5;
        if let Ok(mut t) = tf.get_mut(dash.needle) {
            t.rotation = dash.needle_rest * Quat::from_rotation_z(-angle);
        }
        let steer = c.body.as_ref().map_or(0.0, |b| b.steer_rad);
        if let Ok(mut t) = tf.get_mut(dash.wheel) {
            t.rotation = dash.wheel_rest * Quat::from_rotation_x(-steer * STEERING_RATIO);
        }
    }
}

/// Finds each car's `WHEEL_*` nodes once its scene has spawned.
fn find_wheels(
    mut commands: Commands,
    cars: Query<Entity, (With<CarDraw>, Without<Spin>)>,
    children: Query<&Children>,
    names: Query<(&Name, &Transform)>,
) {
    for root in &cars {
        let mut found: [Option<(Entity, Quat)>; 4] = [None; 4];
        for e in children.iter_descendants(root) {
            if let Ok((n, t)) = names.get(e)
                && let Some(i) = ["WHEEL_FL", "WHEEL_FR", "WHEEL_RL", "WHEEL_RR"]
                    .iter()
                    .position(|w| *w == n.as_str())
            {
                found[i] = Some((e, t.rotation));
            }
        }
        if let [Some(a), Some(b), Some(c), Some(d)] = found {
            commands.entity(root).insert(Spin {
                wheels: [a, b, c, d],
                angle_rad: 0.0,
            });
        }
    }
}

/// Turns each car's wheels with its speed over their radius; the front pair also steers on a driven car.
fn spin_wheels(
    sim: Res<TrafficSim>,
    data: Res<Data>,
    time: Res<Time>,
    mut cars: Query<(&CarDraw, &mut Spin)>,
    mut tf: Query<&mut Transform, Without<CarDraw>>,
) {
    let dt = time.delta_secs();
    for (d, mut spin) in &mut cars {
        let Some(c) = sim.traffic.get(d.id) else { continue };
        let spec = &data.0.traffic.kinds[d.kind];
        let v = c.body.as_ref().map_or(c.speed_m_s, |b| b.speed_m_s());
        spin.angle_rad = (spin.angle_rad - v / spec.wheel_radius_m * dt).rem_euclid(std::f32::consts::TAU);
        let steer = c.body.as_ref().map_or(0.0, |b| b.steer_rad);
        for (i, (e, rest)) in spin.wheels.iter().enumerate() {
            if let Ok(mut t) = tf.get_mut(*e) {
                let turn = if i < 2 { Quat::from_rotation_y(-steer) } else { Quat::IDENTITY };
                let rotation = turn * *rest * Quat::from_rotation_x(spin.angle_rad);
                // A wheel at rest is left alone (see `draw_cars`).
                if t.rotation != rotation {
                    t.rotation = rotation;
                }
            }
        }
    }
}

/// Drivers who left their cars, as 1.8 m block-out figures (stand-ins until people are in the game).
fn draw_runners(
    mut commands: Commands,
    sim: Res<TrafficSim>,
    plan: Res<Plan>,
    mut drawn: Query<(Entity, &RunnerDraw, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut look: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let runners = &sim.traffic.runners;
    let mut seen = std::collections::BTreeSet::new();
    for (e, r, mut t) in &mut drawn {
        match runners.iter().find(|x| x.car == r.0) {
            Some(x) => {
                seen.insert(r.0);
                let y = plan.height_at(x.at.x, x.at.y);
                let dir = (x.to - x.at).normalize_or(Vec2::Y);
                *t = Transform::from_xyz(x.at.x, y + 0.9, x.at.y).with_rotation(Quat::from_rotation_y((-dir.x).atan2(-dir.y)));
            }
            None => commands.entity(e).despawn(),
        }
    }
    let (mesh, mat) = look
        .get_or_insert_with(|| {
            (
                meshes.add(Capsule3d::new(0.25, 1.3)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.75, 0.55, 0.35),
                    perceptual_roughness: 0.8,
                    ..default()
                }),
            )
        })
        .clone();
    for r in runners.iter().filter(|r| !seen.contains(&r.car)) {
        let y = plan.height_at(r.at.x, r.at.y);
        commands.spawn((
            Name::new("driver (block-out stand-in)"),
            RunnerDraw(r.car),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(r.at.x, y + 0.9, r.at.y),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_straight_ahead_is_in_view_and_one_behind_is_not() {
        let eye = Vec3::ZERO;
        assert!(in_cone(eye, Vec3::NEG_Z, 0.8, Vec3::new(0.0, 0.0, -100.0)));
        assert!(!in_cone(eye, Vec3::NEG_Z, 0.8, Vec3::new(0.0, 0.0, 100.0)));
    }

    #[test]
    fn car_ids_sit_above_every_other_hit_id() {
        const { assert!(CAR_ID_BASE > crate::fortress::FORTRESS_ID && CAR_ID_BASE > crate::units::UNIT_ID_BASE) };
    }
}

/// The speed readout while driving (openspec change `car-chase-camera`, DC2, provisional): from outside the car its
/// speedometer cannot be read, so the simulation's speed is shown low on the screen, the same number the needle
/// shows (`cb_core::car::speed_readout`).
#[derive(Component)]
struct SpeedReadout;

fn spawn_speed_readout(mut commands: Commands) {
    commands.spawn((
        Name::new("speed readout"),
        SpeedReadout,
        Text::new(""),
        TextFont::from_font_size(30.0),
        TextColor(Color::srgb(0.95, 0.95, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            bottom: percent(6.0),
            width: percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center),
    ));
}

fn show_speed(sim: Res<TrafficSim>, pilot: Res<PilotSim>, mut text: Single<&mut Text, With<SpeedReadout>>) {
    let want = match sim.traffic.driven() {
        Some(c) if pilot.curr.mode == PilotMode::Driving => cb_core::car::speed_readout(c.vel().length()),
        _ => String::new(),
    };
    if text.0 != want {
        text.0 = want;
    }
}
