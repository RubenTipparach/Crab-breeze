//! The showcase take (`--showcase`): the game played start to finish for a recording, the way the
//! scripted pilot (`playtest-pipeline`) will play it. The pilot walks to the mech and boards; the
//! mech strides down the street, jets up through a cloud, lands in a river by a wood, wades,
//! strides through the trees, drops test shells, loses an arm, its head and a leg, and limps off.
//! It writes only input actions, as a player's keyboard and mouse do, deciding from what a player
//! could see, and prints `PLAYTEST_START`, `PLAYTEST_RESULT` and `PLAYTEST_END` for the recorder.

use crate::input::InputAccum;
use crate::mech::{Enemy, MechAccessNow, MechSim, Piloted};
use crate::pilot::PilotSim;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::input::{Actions, DebugCommand, ModeRequest};
use cb_core::mech::{LocomotionMode, MechState, PartKind};
use cb_core::pilot::{PilotMode, PilotState};
use cb_core::world::Ground;
use std::f32::consts::{PI, TAU};

/// Plays a showcase take: the world tour, or the city fight.
pub struct ShowcasePlugin {
    /// The city fight instead of the world tour.
    pub fight: bool,
}

/// Where the take is.
#[derive(Resource, Default)]
pub struct Take {
    /// The player's brain in the fight: the enemies' brain, its orders played as actions.
    brain: Option<cb_core::ai::AiPilot>,
    /// The beat playing now.
    pub beat: usize,
    beat_started_s: f32,
    cloud: Vec3,
    landing: Vec2,
    wood: Vec2,
    in_cloud_s: f32,
    shots: u32,
}

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Take>();
        if self.fight {
            app.add_systems(Update, play_fight.in_set(crate::sim::PresentSet::Input));
        } else {
            app.add_systems(Update, play.in_set(crate::sim::PresentSet::Input));
        }
    }
}

/// Actions that walk the pilot toward `target` (feet), turning the view onto it as a mouse would,
/// with the view pitched to `pitch_rad`. Shared with the capture instrument.
pub fn walk_to(pilot: &PilotState, target: Vec3, pitch_rad: f32) -> Actions {
    let to = target - pilot.feet;
    let want = (-to.x).atan2(-to.z);
    let turn = (want - pilot.yaw_rad + PI).rem_euclid(TAU) - PI;
    let near = Vec2::new(to.x, to.z).length() < 2.0;
    Actions {
        move_xy: if near { Vec2::ZERO } else { Vec2::new(0.0, 1.0) },
        look_delta_deg: Vec2::new(-turn.to_degrees() * 0.15, (pitch_rad - pilot.pitch_rad).to_degrees() * 0.1),
        ..default()
    }
}

/// Leg turn toward a point, -1..1 (left positive).
fn steer(m: &MechState, target: Vec2) -> f32 {
    let v = target - Vec2::new(m.pos.x, m.pos.z);
    let want = (-v.x).atan2(-v.y);
    let diff = (want - m.heading_rad + PI).rem_euclid(TAU) - PI;
    (diff * 3.0).clamp(-1.0, 1.0)
}

/// Throttle lever motion toward a setting.
fn throttle_to(m: &MechState, want: f32) -> f32 {
    ((want - m.throttle) * 5.0).clamp(-1.0, 1.0)
}

/// Torso aim toward a twist and pitch, as mouse motion.
fn aim(m: &MechState, twist_rad: f32, pitch_rad: f32) -> Vec2 {
    Vec2::new(
        -(twist_rad - m.aim_twist_rad).to_degrees() * 0.2,
        (pitch_rad - m.aim_pitch_rad).to_degrees() * 0.2,
    )
}

fn say(line: &str) {
    println!("{line}");
}

fn play(
    mut take: ResMut<Take>,
    mut acc: ResMut<InputAccum>,
    pilot: Res<PilotSim>,
    access: Res<MechAccessNow>,
    mechs: Query<&MechSim, With<Piloted>>,
    plan: Res<Plan>,
    live: Res<Live>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let t = now - take.beat_started_s;
    let mut next = false;
    let p = &pilot.curr;
    let Some(sim) = mechs.iter().next() else { return };
    let m = &sim.curr;
    let mut act = Actions::default();
    match take.beat {
        // The world and rig are up and shaders have settled. Pick the cloud to fly into, a river
        // landing by the thickest wood, and that wood.
        0 => {
            if t > 5.0 && access.0.is_some() {
                let here = Vec2::new(m.pos.x, m.pos.z);
                take.cloud = plan
                    .clouds
                    .puffs
                    .iter()
                    .filter(|c| Vec2::new(c.centre.x, c.centre.z).distance(here) > 500.0)
                    .min_by(|a, b| {
                        Vec2::new(a.centre.x, a.centre.z)
                            .distance(here)
                            .total_cmp(&Vec2::new(b.centre.x, b.centre.z).distance(here))
                    })
                    .map(|c| c.centre)
                    .unwrap_or(Vec3::new(here.x, 800.0, here.y - 800.0));
                let cloud_xz = Vec2::new(take.cloud.x, take.cloud.z);
                let mut best = (f32::MIN, here, here);
                // Trees grow only near bodies (island-world), so probe a copy of the forest grown
                // round each river point within 3 km of the cloud.
                let mut probe = live.forest.clone();
                for r in &plan.waters.rivers {
                    for q in r.path.iter().step_by(3) {
                        if q.distance(cloud_xz) > 3000.0 {
                            continue;
                        }
                        probe.grow_around(&plan, *q, 160.0);
                        let trees: Vec<u32> = probe.near(*q, 160.0);
                        let dry = trees
                            .iter()
                            .filter_map(|id| probe.tree(*id))
                            .filter(|t| plan.water_at(t.base.x, t.base.z).is_none())
                            .count() as f32;
                        let score = dry - q.distance(cloud_xz) / 15.0;
                        if score > best.0 {
                            // The nearest dry tree, and on into the wood beyond it.
                            let tree = trees
                                .iter()
                                .filter_map(|id| probe.tree(*id))
                                .map(|t| t.base)
                                .filter(|b| plan.water_at(b.x, b.z).is_none())
                                .map(|b| Vec2::new(b.x, b.z))
                                .min_by(|a, b| a.distance(*q).total_cmp(&b.distance(*q)))
                                .unwrap_or(*q);
                            let wood = tree + (tree - *q).normalize_or(Vec2::X) * 60.0;
                            best = (score, *q, wood);
                        }
                    }
                }
                take.landing = best.1;
                take.wood = best.2;
                info!(
                    "showcase: cloud at {:?}, landing at {:?}, wood at {:?}",
                    take.cloud, take.landing, take.wood
                );
                say("PLAYTEST_READY showcase");
                say("PLAYTEST_START showcase");
                next = true;
            }
        }
        // Look along the street.
        1 => {
            let sweep = if t < 1.2 {
                -1.0
            } else if t < 3.6 {
                1.0
            } else if t < 4.8 {
                -1.0
            } else {
                0.0
            };
            act.look_delta_deg = Vec2::new(sweep * 0.45, 0.0);
            next = t > 5.0;
        }
        // Walk to the stair, Use once, look up as the mech sits and opens, climb in.
        2 => {
            let a = access.0.unwrap();
            act = match p.mode {
                PilotMode::OnFoot if p.waiting.is_none() && p.distance_to_stair(&a) < 2.5 => Actions {
                    use_pressed: true,
                    ..default()
                },
                PilotMode::OnFoot if p.waiting.is_none() => walk_to(p, a.ladder_base, 0.2),
                PilotMode::OnFoot => walk_to(p, a.ladder_base + Vec3::Y, 0.45),
                _ => Actions::default(),
            };
            next = p.mode == PilotMode::Seated;
        }
        // Stand up and stride down the street.
        3 => {
            act.throttle_axis = throttle_to(m, 0.8);
            next = t > 12.0;
        }
        // Jets: climb toward the cloud, fly into it.
        4 => {
            let cloud_xz = Vec2::new(take.cloud.x, take.cloud.z);
            act.mode_request = (m.mode != LocomotionMode::Jet).then_some(ModeRequest::Jet);
            act.turn_axis = steer(m, cloud_xz);
            act.throttle_axis = throttle_to(m, 1.0);
            let climb = take.cloud.y - m.pos.y;
            act.lift_axis = if climb > 30.0 {
                1.0
            } else {
                (0.35 - m.vel_y_m_s * 0.05).clamp(0.0, 1.0)
            };
            act.look_delta_deg = aim(m, 0.0, 0.15);
            if plan.clouds.density_at(m.pos + Vec3::Y * 9.6, now) > 0.05 {
                take.in_cloud_s += time.delta_secs();
            }
            next = take.in_cloud_s > 2.5 || t > 40.0 || m.jet_overheated;
        }
        // Out of the cloud, down to the river: glide, then brake with the jets and land in it.
        5 => {
            act.turn_axis = steer(m, take.landing);
            let d = Vec2::new(m.pos.x, m.pos.z).distance(take.landing);
            let ground = plan.height_at(m.pos.x, m.pos.z);
            let agl = m.pos.y - ground;
            act.throttle_axis = throttle_to(m, if d > 300.0 { 1.0 } else { (d / 300.0).max(0.1) });
            let want_vy = if d > 200.0 { -6.0 } else { (-agl * 0.25).clamp(-14.0, -2.0) };
            act.lift_axis = if m.mode == LocomotionMode::Jet {
                (0.35 + (want_vy - m.vel_y_m_s) * 0.08).clamp(0.0, 1.0)
            } else {
                0.0
            };
            act.look_delta_deg = aim(m, 0.0, -0.25);
            next = m.mode == LocomotionMode::Walk && t > 2.0 || t > 60.0;
        }
        // Wade along the river: spray and rings on every stride.
        6 => {
            act.throttle_axis = throttle_to(m, 0.7);
            act.look_delta_deg = aim(m, if t < 4.0 { 0.6 } else { -0.4 }, -0.3);
            next = t > 9.0;
        }
        // Out of the water and through the wood, snapping trunks.
        7 => {
            act.turn_axis = steer(m, take.wood);
            act.throttle_axis = throttle_to(m, 0.9);
            act.look_delta_deg = aim(m, 0.0, -0.1);
            next = Vec2::new(m.pos.x, m.pos.z).distance(take.wood) < 12.0 || t > 45.0;
        }
        // Test shells where the pilot looks, left, ahead and right.
        8 => {
            act.throttle_axis = throttle_to(m, 0.0);
            let twist = [0.7, 0.0, -0.7, 0.3][(take.shots as usize).min(3)];
            act.look_delta_deg = aim(m, twist, -0.33);
            if t > 2.0 + 2.2 * take.shots as f32 && take.shots < 4 {
                act.debug = Some(DebugCommand::Shell);
                take.shots += 1;
            }
            next = t > 12.0;
        }
        // Lose the left arm, the head, then the left leg; limp off.
        9 => {
            act.look_delta_deg = aim(m, if t < 3.0 { 0.9 } else { 0.0 }, 0.0);
            act.debug = match () {
                _ if (1.0..1.05).contains(&t) => Some(DebugCommand::Hit(PartKind::ArmL)),
                _ if (5.0..5.05).contains(&t) => Some(DebugCommand::Hit(PartKind::Head)),
                _ if (9.0..9.05).contains(&t) => Some(DebugCommand::Hit(PartKind::LegL)),
                _ => None,
            };
            act.throttle_axis = if t > 10.0 { throttle_to(m, 1.0) } else { 0.0 };
            next = t > 22.0;
        }
        10 => {
            act.throttle_zero = t < 0.1;
            if t > 3.0 {
                say("PLAYTEST_RESULT ok showcase: boarded, strode, flew, waded, felled trees, shelled, limped");
                next = true;
            }
        }
        11 if t > 2.0 => {
            say("PLAYTEST_END showcase");
            exit.write(AppExit::Success);
            next = true;
        }
        _ => {}
    }
    acc.script = Some(act);
    if next {
        take.beat += 1;
        take.beat_started_s = now;
    }
}

/// Torso aim onto a world point from the eye, as mouse motion.
fn aim_at(m: &MechState, eye: Vec3, point: Vec3) -> Vec2 {
    let d = point - eye;
    let yaw = (-d.x).atan2(-d.z);
    let twist = (yaw - m.heading_rad + PI).rem_euclid(TAU) - PI;
    let pitch = d.y.atan2(Vec2::new(d.x, d.z).length());
    Vec2::new(
        -(twist - m.aim_twist_rad).to_degrees() * 0.35,
        (pitch - m.aim_pitch_rad).to_degrees() * 0.35,
    )
}

/// The city fight: board, walk out to meet two enemy mechs, shoot it out with the autocannon and
/// cannon, then bring down the nearest tower by its ground floor and stride through the rubble.
fn play_fight(
    mut take: ResMut<Take>,
    mut acc: ResMut<InputAccum>,
    pilot: Res<PilotSim>,
    access: Res<MechAccessNow>,
    me: Query<&MechSim, With<Piloted>>,
    enemies: Query<&MechSim, With<Enemy>>,
    plan: Res<Plan>,
    live: Res<Live>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let t = now - take.beat_started_s;
    let mut next = false;
    let p = &pilot.curr;
    let Some(sim) = me.iter().next() else { return };
    let m = &sim.curr;
    let eye = m.pos + Vec3::Y * 9.6;
    let mut act = Actions::default();
    let alive: Vec<&MechSim> = enemies.iter().filter(|e| !e.dead).collect();
    if (now * 1.0).fract() < 0.017 {
        let es: Vec<String> = enemies
            .iter()
            .map(|e| format!("({:.0},{:.0}) v{:.1}", e.curr.pos.x, e.curr.pos.z, e.curr.speed_m_s))
            .collect();
        info!(
            "track: beat {} me ({:.0},{:.0}) v{:.1} throttle {:.2} | enemies {}",
            take.beat,
            m.pos.x,
            m.pos.z,
            m.speed_m_s,
            m.throttle,
            es.join(" ")
        );
    }
    match take.beat {
        0 => {
            if t > 5.0 && access.0.is_some() {
                say("PLAYTEST_READY fight");
                say("PLAYTEST_START fight");
                next = true;
            }
        }
        1 => {
            act.look_delta_deg = Vec2::new(
                if t < 1.5 {
                    -0.4
                } else if t < 3.0 {
                    0.4
                } else {
                    0.0
                },
                0.0,
            );
            next = t > 3.5;
        }
        2 => {
            let a = access.0.unwrap();
            act = match p.mode {
                PilotMode::OnFoot if p.waiting.is_none() && p.distance_to_stair(&a) < 2.5 => Actions {
                    use_pressed: true,
                    ..default()
                },
                PilotMode::OnFoot if p.waiting.is_none() => walk_to(p, a.ladder_base, 0.2),
                PilotMode::OnFoot => walk_to(p, a.ladder_base + Vec3::Y, 0.45),
                _ => Actions::default(),
            };
            next = p.mode == PilotMode::Seated;
        }
        // Stand and walk out toward them.
        3 => {
            act.throttle_axis = throttle_to(m, 0.5);
            next = t > 8.0;
        }
        // The fight: the enemies' own brain flies the player's mech (orbit the nearest at a
        // stand-off, evade when hit, hop, find a firing line), its orders played as actions.
        4 | 5 => {
            let (beat, tower) = (take.beat, take.shots);
            let brain = take.brain.get_or_insert_with(|| {
                cb_core::ai::AiPilot::new(
                    cb_core::ai::Skill {
                        aim_noise_deg: 0.3,
                        fire_error_deg: 3.5,
                        range_m: if beat == 4 { 130.0 } else { 120.0 },
                        hop_chance: 0.3,
                        ..Default::default()
                    },
                    cb_core::rng::SimRng::new(plan.seed).fork("player"),
                )
            });
            let (target, target_vel, wall) = if beat == 4 {
                match alive
                    .iter()
                    .min_by(|a, b| a.curr.pos.distance(m.pos).total_cmp(&b.curr.pos.distance(m.pos)))
                {
                    Some(e) => (e.curr.pos, cb_core::mech::forward(e.curr.heading_rad) * e.curr.speed_m_s, false),
                    None => (m.pos, Vec3::ZERO, false),
                }
            } else {
                let b = &plan.buildings[tower as usize];
                let c = &live.structures.buildings[b.id as usize];
                (
                    Vec3::new(
                        b.footprint.centre().x,
                        c.origin.y + 2.0 - cb_core::ai::TORSO_AIM_M,
                        b.footprint.centre().y,
                    ),
                    Vec3::ZERO,
                    true,
                )
            };
            let hit = sim.took_hit;
            let clear = |a: Vec3, b: Vec3| wall || crate::mech::line_clear(&plan, &live, a, b);
            let cover = |me: Vec3, threat: Vec3| crate::mech::cover_point(&plan, &live, me, threat);
            let view = cb_core::ai::View {
                eye,
                target,
                target_vel,
                muzzle_speed_m_s: 1100.0,
                health: 1.0,
                hit,
                allies: &[],
                clear: &clear,
                cover: &cover,
            };
            let o = brain.orders(m, &view, time.delta_secs());
            act.throttle_axis = o.controls.throttle_axis;
            act.turn_axis = o.controls.turn_axis;
            act.look_delta_deg = o.controls.aim_delta_deg;
            act.lift_axis = o.controls.lift_axis;
            act.mode_request = o.controls.mode_request;
            act.fire = o.fire;
            if beat == 4 && alive.is_empty() && t > 2.0 || t > 100.0 {
                next = true;
                take.brain = None;
                // The tower to bring down: the tallest standing within 400 m.
                let here = Vec2::new(m.pos.x, m.pos.z);
                take.shots = plan
                    .buildings
                    .iter()
                    .filter(|b| b.footprint.centre().distance(here) < 400.0 && live.structures.buildings[b.id as usize].standing > 0)
                    .max_by_key(|b| b.storeys)
                    .map_or(0, |b| b.id);
            } else if beat == 5 && (live.structures.buildings[tower as usize].standing == 0 || t > 40.0) {
                next = true;
            }
        }
        // Charge into the ruin and through the dust.
        6 => {
            let b = &plan.buildings[take.shots as usize];
            act.look_delta_deg = aim_at(m, eye, Vec3::new(b.footprint.centre().x, eye.y + 10.0, b.footprint.centre().y));
            act.throttle_axis = throttle_to(m, 1.0);
            act.turn_axis = steer(m, b.footprint.centre());
            next = t > 14.0;
        }
        7 => {
            act.throttle_zero = t < 0.1;
            if t > 3.0 {
                say("PLAYTEST_RESULT ok fight: boarded, fought two mechs, brought down a tower");
                next = true;
            }
        }
        8 if t > 2.0 => {
            say("PLAYTEST_END fight");
            exit.write(AppExit::Success);
            next = true;
        }
        _ => {}
    }
    acc.script = Some(act);
    if next {
        take.beat += 1;
        take.beat_started_s = now;
    }
}
