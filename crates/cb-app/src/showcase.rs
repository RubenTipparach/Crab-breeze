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
use cb_core::input::{Actions, DebugCommand};
use cb_core::mech::{LocomotionMode, MechState, PartKind};
use cb_core::pilot::{PilotMode, PilotState};
use cb_core::world::Ground;
use std::f32::consts::{PI, TAU};

/// Plays a showcase take: the world tour, the city fight, or the city defence.
pub struct ShowcasePlugin {
    /// `world`, `fight` or `defence`.
    pub take: String,
}

/// Where the take is.
#[derive(Resource, Default)]
pub struct Take {
    /// The beats' names, for the benchmark's report.
    pub names: &'static [&'static str],
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
    /// The defence take is on its rockets, crossing the city by air.
    flying: bool,
}

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        let names: &'static [&'static str] = match self.take.as_str() {
            "fight" => &["load", "look", "board", "stand", "fight", "tower", "charge", "result", "end"],
            "defence" => &DEFENCE_BEATS,
            _ => &[
                "load", "look", "board", "stride", "rockets", "landing", "wade", "wood", "shells", "damage", "result", "end",
            ],
        };
        app.insert_resource(Take { names, ..default() });
        match self.take.as_str() {
            "fight" => app.add_systems(Update, play_fight.in_set(crate::sim::PresentSet::Input)),
            "defence" => app.add_systems(Update, play_defence.in_set(crate::sim::PresentSet::Input)),
            _ => app.add_systems(Update, play.in_set(crate::sim::PresentSet::Input)),
        };
    }
}

/// Rocket lift that holds the mech level: gravity over the rockets' thrust (9.81 / 56).
const HOVER_LIFT: f32 = 0.175;

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

/// The boarding beat every take shares: walk to the stair, press Use once, look up as the mech
/// kneels and opens, and be carried into the seat.
fn board(p: &PilotState, access: Option<&cb_core::pilot::MechAccess>) -> Actions {
    let Some(a) = access else { return Actions::default() };
    match p.mode {
        PilotMode::OnFoot if p.waiting.is_none() && p.distance_to_stair(a) < 2.5 => Actions {
            use_pressed: true,
            ..default()
        },
        PilotMode::OnFoot if p.waiting.is_none() => walk_to(p, a.ladder_base, 0.2),
        PilotMode::OnFoot => walk_to(p, a.ladder_base + Vec3::Y, 0.45),
        _ => Actions::default(),
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
                for r in &plan.waters.rivers {
                    for q in r.path.iter().step_by(3) {
                        let trees: Vec<u32> = live.forest.near(*q, 160.0);
                        let dry = trees
                            .iter()
                            .filter(|id| {
                                plan.water_at(live.forest.trees[**id as usize].base.x, live.forest.trees[**id as usize].base.z)
                                    .is_none()
                            })
                            .count() as f32;
                        let score = dry - q.distance(cloud_xz) / 15.0;
                        if score > best.0 {
                            // The nearest dry tree, and on into the wood beyond it.
                            let tree = trees
                                .iter()
                                .map(|id| live.forest.trees[*id as usize].base)
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
            act = board(p, access.0.as_ref());
            next = p.mode == PilotMode::Seated;
        }
        // Stand up and stride down the street.
        3 => {
            act.throttle_axis = throttle_to(m, 0.8);
            next = t > 12.0;
        }
        // Rockets up, skates across: climb toward the cloud, fly into it.
        4 => {
            let cloud_xz = Vec2::new(take.cloud.x, take.cloud.z);
            act.skate_toggle = !m.skates_on;
            act.move_xy = Vec2::new(0.0, 1.0);
            act.turn_axis = steer(m, cloud_xz);
            act.throttle_axis = throttle_to(m, 1.0);
            let climb = take.cloud.y - m.pos.y;
            act.lift_axis = if climb > 30.0 {
                1.0
            } else {
                (HOVER_LIFT - m.vel_y_m_s * 0.025).clamp(0.0, 1.0)
            };
            act.look_delta_deg = aim(m, 0.0, 0.15);
            if plan.clouds.density_at(m.pos + Vec3::Y * 9.6, now) > 0.05 {
                take.in_cloud_s += time.delta_secs();
            }
            next = take.in_cloud_s > 2.5 || t > 40.0 || m.boost.locked;
        }
        // Out of the cloud, down to the river: glide, then brake with the jets and land in it.
        5 => {
            act.turn_axis = steer(m, take.landing);
            let d = Vec2::new(m.pos.x, m.pos.z).distance(take.landing);
            let ground = plan.height_at(m.pos.x, m.pos.z);
            let agl = m.pos.y - ground;
            act.throttle_axis = throttle_to(m, if d > 300.0 { 1.0 } else { (d / 300.0).max(0.1) });
            let want_vy = if d > 200.0 { -6.0 } else { (-agl * 0.25).clamp(-14.0, -2.0) };
            act.lift_axis = if m.airborne() {
                (HOVER_LIFT + (want_vy - m.vel_y_m_s) * 0.04).clamp(0.0, 1.0)
            } else {
                0.0
            };
            // Skates push across until near, then off to land in the river on the legs.
            act.move_xy = if d > 200.0 { Vec2::new(0.0, 1.0) } else { Vec2::ZERO };
            act.skate_toggle = m.skates_on && agl < 30.0;
            act.look_delta_deg = aim(m, 0.0, -0.25);
            next = !m.airborne() && m.mode == LocomotionMode::Walk && t > 2.0 || t > 60.0;
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
            act = board(p, access.0.as_ref());
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
                drop: true,
                health: 1.0,
                hit,
                allies: &[],
                clear: &clear,
                cover: &cover,
            };
            let o = brain.orders(m, &view, time.delta_secs());
            play_orders(&o, &mut act);
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

/// The orders of an AI brain flying the player's mech, played as the player's actions.
fn play_orders(o: &cb_core::ai::Orders, act: &mut Actions) {
    act.throttle_axis = o.controls.throttle_axis;
    act.turn_axis = o.controls.turn_axis;
    act.look_delta_deg = o.controls.aim_delta_deg;
    act.lift_axis = o.controls.lift_axis;
    act.mode_request = o.controls.mode_request;
    act.fire = o.fire;
}

/// The defence take's beats: travel to each wave and fight it.
const DEFENCE_BEATS: [&str; 12] = [
    "load", "look", "board", "stand", "travel1", "fight1", "travel2", "fight2", "travel3", "fight3", "result", "end",
];

/// The city defence (openspec change `city-defence`): board, then for each wave skate out to the
/// attacked city and fight the raiders one by one, the nearest first, with the enemies' own brain
/// played as the player's actions, until the mission is won or lost.
#[allow(clippy::too_many_arguments)]
fn play_defence(
    mut take: ResMut<Take>,
    mut acc: ResMut<InputAccum>,
    pilot: Res<PilotSim>,
    access: Res<MechAccessNow>,
    me: Query<(&MechSim, &crate::mech::MechSpec), With<Piloted>>,
    raiders: Query<(&MechSim, &crate::defence::Raider)>,
    run: Res<crate::defence::DefenceRun>,
    targets: Res<crate::targeting::TargetState>,
    data: Res<crate::sim::Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    use cb_core::mission::Phase;
    let now = time.elapsed_secs();
    let t = now - take.beat_started_s;
    let mut next = false;
    let p = &pilot.curr;
    let Some((sim, spec)) = me.iter().next() else { return };
    let m = &sim.curr;
    let eye = m.pos + Vec3::Y * 9.6;
    let cfg = &data.0.defence;
    let mut act = Actions::default();
    let over = matches!(run.state.phase, Phase::Won | Phase::Lost(_));
    // Every 5 s, where the pilot and the wave's live raiders are, for reading a stall from the log.
    if (now / 5.0).floor() != ((now - time.delta_secs()) / 5.0).floor() && take.beat >= 4 {
        let others: Vec<String> = raiders
            .iter()
            .filter(|(r, _)| !r.dead)
            .map(|(r, x)| {
                format!(
                    "w{}#{} ({:.0},{:.0},{:.0})",
                    x.wave + 1,
                    x.index,
                    r.curr.pos.x,
                    r.curr.pos.y,
                    r.curr.pos.z
                )
            })
            .collect();
        info!(
            "take: beat {} me ({:.0},{:.0},{:.0}) {:.1} m/s mode {:?} | {}",
            take.names.get(take.beat).copied().unwrap_or("?"),
            m.pos.x,
            m.pos.y,
            m.pos.z,
            m.vel.length(),
            m.mode,
            others.join(" ")
        );
    }
    match take.beat {
        0 => {
            if t > 5.0 && access.0.is_some() {
                say("PLAYTEST_READY defence");
                say("PLAYTEST_START defence");
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
            act = board(p, access.0.as_ref());
            next = p.mode == PilotMode::Seated;
        }
        3 => {
            act.throttle_axis = throttle_to(m, 0.5);
            next = t > 4.0;
        }
        // Travel to wave w (beat 4 + 2w), then fight it (beat 5 + 2w).
        4..=9 if !over => {
            let w = (take.beat - 4) / 2;
            let fighting = (take.beat - 4) % 2 == 1;
            let wave = run.state.wave().unwrap_or(w);
            if wave > w {
                // The wave is cleared: on to the next city.
                take.beat = 4 + 2 * wave;
                take.beat_started_s = now;
                take.brain = None;
                acc.script = Some(act);
                return;
            }
            let here = Vec2::new(m.pos.x, m.pos.z);
            let nearest = raiders
                .iter()
                .filter(|(r, x)| x.wave == w && !r.dead)
                .min_by(|a, b| a.0.curr.pos.distance(m.pos).total_cmp(&b.0.curr.pos.distance(m.pos)))
                .map(|(r, _)| r.curr);
            let city = &plan.cities[cfg.cities[w] as usize];
            let goal = nearest.map_or(city.centre, |r| Vec2::new(r.pos.x, r.pos.z));
            let d = here.distance(goal);
            // Fighting, a raider over 600 m off is travelled to, not orbited.
            if !fighting || d > 600.0 {
                // Far off, up on the rockets and skate across the roofs while the rockets last
                // (start with them under 30 % used, come down past 85 %); on the ground skate
                // down the streets while they recharge, walking the last stretch.
                if take.flying && (m.boost.heat > 0.85 || m.boost.locked || d < 250.0) {
                    take.flying = false;
                } else if !take.flying && d > 400.0 && m.boost.heat < 0.3 && !m.boost.locked {
                    take.flying = true;
                }
                let yaw = if take.flying {
                    (-(goal - here).x).atan2(-(goal - here).y)
                } else {
                    open_heading(&plan, &live, here, goal)
                };
                let toward = here + Vec2::new(-yaw.sin(), -yaw.cos()) * 50.0;
                act.turn_axis = steer(m, toward);
                let hold = nearest.is_none() && d < city.half_size_m * 0.5;
                act.throttle_axis = throttle_to(m, if hold { 0.0 } else { 1.0 });
                if take.flying {
                    act.skate_toggle = !m.skates_on && !m.skate.locked;
                    act.move_xy = Vec2::new(0.0, 1.0);
                    let cruise = cruise_height(&plan, &live, here, goal);
                    let want_vy = ((cruise - m.pos.y) * 0.5).clamp(-8.0, 14.0);
                    act.lift_axis = (HOVER_LIFT + (want_vy - m.vel_y_m_s) * 0.05).clamp(0.0, 1.0);
                } else {
                    let want = d > 500.0 && !m.skate.locked;
                    act.skate_toggle = want != m.skates_on;
                }
                act.look_delta_deg = aim(m, 0.0, -0.03);
                next = !fighting && nearest.is_some() && d < 450.0;
                if next {
                    say(&format!("PLAYTEST_BEAT fight{}", w + 1));
                }
            } else if let Some(target) = nearest {
                take.flying = false;
                act.skate_toggle = m.skates_on;
                let brain = take.brain.get_or_insert_with(|| {
                    cb_core::ai::AiPilot::new(
                        cb_core::ai::Skill {
                            aim_noise_deg: 0.3,
                            fire_error_deg: 3.5,
                            range_m: 140.0,
                            hop_chance: 0.3,
                            ..Default::default()
                        },
                        cb_core::rng::SimRng::new(plan.seed).fork("player"),
                    )
                });
                let clear = |a: Vec3, b: Vec3| crate::mech::line_clear(&plan, &live, a, b);
                let cover = |me: Vec3, threat: Vec3| crate::mech::cover_point(&plan, &live, me, threat);
                let view = cb_core::ai::View {
                    eye,
                    target: target.pos,
                    target_vel: Vec3::new(target.vel.x, target.vel_y_m_s, target.vel.y),
                    muzzle_speed_m_s: spec.weapons.first().map_or(1100.0, |w| w.speed_m_s),
                    drop: true,
                    health: crate::mech::torso_health(sim, spec),
                    hit: sim.took_hit,
                    allies: &[],
                    clear: &clear,
                    cover: &cover,
                };
                let o = brain.orders(m, &view, time.delta_secs());
                play_orders(&o, &mut act);
            }
            // A lock, travelling or fighting: missile away (as a player holding F would).
            act.fire_missile = targets.locked(&data.0.targeting).is_some();
        }
        4..=9 => {
            take.beat = 10;
            take.beat_started_s = now;
        }
        10 => {
            act.throttle_zero = t < 0.1;
            if t > 3.0 {
                let line = match run.state.phase {
                    Phase::Won => format!(
                        "PLAYTEST_RESULT win defence: all {} raiders destroyed, {}",
                        cfg.raiders_per_wave as usize * cfg.cities.len(),
                        damage_summary(&run.damage, cfg)
                    ),
                    Phase::Lost(why) => format!(
                        "PLAYTEST_RESULT lose defence: {}, {}",
                        crate::defence::loss_text(why),
                        damage_summary(&run.damage, cfg)
                    ),
                    _ => "PLAYTEST_RESULT lose defence: the take ended early".into(),
                };
                say(&line);
                next = true;
            }
        }
        11 if t > 2.0 => {
            say("PLAYTEST_END defence");
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

/// Height to cross at on the rockets toward `goal`: 45 m over the ground, or 15 m over the tallest
/// standing building within 80 m of the next 200 m of the way.
fn cruise_height(plan: &cb_core::world::WorldPlan, live: &crate::world::Live, here: Vec2, goal: Vec2) -> f32 {
    let dir = (goal - here).normalize_or(Vec2::X);
    let mut top = plan.height_at(here.x, here.y) + 45.0;
    for i in 0..=5 {
        let p = here + dir * 40.0 * i as f32;
        top = top.max(plan.height_at(p.x, p.y) + 45.0);
        for b in plan.collision.candidates(p, 80.0) {
            let c = &live.structures.buildings[b as usize];
            if c.standing > 0 {
                top = top.max(c.origin.y + plan.buildings[b as usize].height_m + 15.0);
            }
        }
    }
    top
}

/// The heading (yaw, 0 along -z) nearest the way to `goal` whose next 40 m are clear of standing
/// buildings at chest height, trying 15 degree turns either side; straight at the goal if none is.
fn open_heading(plan: &cb_core::world::WorldPlan, live: &crate::world::Live, here: Vec2, goal: Vec2) -> f32 {
    let to = goal - here;
    let want = (-to.x).atan2(-to.y);
    let reach = to.length().min(40.0);
    let clear = |yaw: f32| {
        let dir = Vec2::new(-yaw.sin(), -yaw.cos());
        (1..=10).all(|i| {
            let p = here + dir * reach * i as f32 / 10.0;
            let at = Vec3::new(p.x, plan.height_at(p.x, p.y) + 6.0, p.y);
            plan.collision
                .candidates(p, 6.0)
                .into_iter()
                .all(|b| !live.structures.buildings[b as usize].solid_at(at))
        })
    };
    (0..13)
        .map(|k| {
            let turn = (k as f32 / 2.0).ceil() * if k % 2 == 0 { 1.0 } else { -1.0 };
            want + turn * 15f32.to_radians()
        })
        .find(|y| clear(*y))
        .unwrap_or(want)
}

/// `city 1 23 %, city 2 8 %, city 3 31 %`.
fn damage_summary(damage: &[f32], cfg: &cb_core::mission::DefenceConfig) -> String {
    cfg.cities
        .iter()
        .map(|c| format!("city {} {:.0} %", c + 1, damage.get(*c as usize).copied().unwrap_or(0.0) * 100.0))
        .collect::<Vec<_>>()
        .join(", ")
}
