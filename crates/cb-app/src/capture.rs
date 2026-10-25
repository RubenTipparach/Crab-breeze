//! The capture instrument (`pilot-boarding` design section 4; CLAUDE.md section 4 allows it as a
//! measurement instrument): `--stills <dir>` writes stills from the street and the cockpit and
//! logs frame time over a stride, then exits. It only writes input actions, as a player would,
//! except for placing the pilot at the start of the street shot. It moves into the scripted pilot
//! (`playtest-pipeline`) when that lands.

use crate::input::InputAccum;
use crate::mech::MechAccessNow;
use crate::pilot::PilotSim;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use cb_core::input::Actions;
use cb_core::pilot::{PilotMode, PilotState};
use std::path::PathBuf;

/// Runs the capture script.
pub struct StillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    frames_ms: Vec<f32>,
}

impl Plugin for StillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            frames_ms: Vec::new(),
        })
        .add_systems(Update, run_script);
    }
}

fn shoot(commands: &mut Commands, dir: &std::path::Path, name: &str) {
    let path = dir.join(name);
    info!("capture: {}", path.display());
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

fn run_script(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut acc: ResMut<InputAccum>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    enemies: Query<&GlobalTransform, With<crate::mech::Enemy>>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
    mut exit_frames: Local<u32>,
) {
    let now = time.elapsed_secs();
    let in_beat = now - script.beat_started_s;
    let mut next = false;
    match script.beat {
        // Wait for the world, the rig and shaders to settle.
        0 => next = in_beat > 6.0 && access.0.is_some(),
        // The street: the pilot 12 m behind the mech, looking up at it along the street.
        1 => {
            if in_beat < 0.05 {
                let a = access.0.unwrap();
                let feet = a.ladder_base + Vec3::new(-6.0, 0.0, 10.0);
                let to = a.ladder_base - feet;
                let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
                s.pitch_rad = 0.32;
                pilot.prev = s.clone();
                pilot.curr = s;
            }
            if in_beat > 1.5 {
                shoot(&mut commands, &script.dir, "street.png");
                next = true;
            }
        }
        // An enemy in its skin (`blockout-enemies`): the pilot stood 70 m off the nearest, looking up at it.
        // Skipped when there is none; then back behind the mech to board.
        2 if enemies.is_empty() => next = true,
        2 => {
            let a = access.0.unwrap();
            let e = enemies
                .iter()
                .map(|g| g.translation())
                .min_by(|p, q| p.distance(a.ladder_base).total_cmp(&q.distance(a.ladder_base)))
                .unwrap();
            if in_beat < 0.05 {
                let flat = Vec3::new(a.ladder_base.x - e.x, 0.0, a.ladder_base.z - e.z).normalize_or(Vec3::X);
                let feet = Vec3::new(e.x, a.ladder_base.y, e.z) + flat * 70.0;
                let to = e - feet;
                let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
                s.pitch_rad = 0.12;
                pilot.prev = s.clone();
                pilot.curr = s;
            }
            if in_beat > 2.0 {
                shoot(&mut commands, &script.dir, "enemy.png");
                let a = access.0.unwrap();
                let feet = a.ladder_base + Vec3::new(-6.0, 0.0, 10.0);
                let s = PilotState::standing(feet, 0.0);
                pilot.prev = s.clone();
                pilot.curr = s;
                next = true;
            }
        }
        // Walk to the ladder by input, then press Use.
        3 => {
            let a = access.0.unwrap();
            acc.script = Some(if pilot.curr.mode == PilotMode::OnFoot && pilot.curr.distance_to_stair(&a) > 2.0 {
                crate::showcase::walk_to(&pilot.curr, a.ladder_base, 0.0)
            } else {
                Actions {
                    use_pressed: pilot.curr.mode == PilotMode::OnFoot,
                    ..default()
                }
            });
            if matches!(pilot.curr.mode, PilotMode::Boarding(_)) {
                // The stair is down and the climb begins: the open hatch from the stair's foot.
                acc.script = Some(Actions::default());
                shoot(&mut commands, &script.dir, "hatch-open.png");
                next = true;
            }
        }
        4 => {
            // Seated; wait for the hatch to seal behind the pilot.
            if pilot.curr.mode == PilotMode::Seated && in_beat > 12.0 {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Stand up and stride: throttle forward (the mech stands first), hold, measure frame time
        // over 10 s.
        5 => {
            acc.script = Some(Actions {
                throttle_axis: if in_beat < 5.0 { 0.6 } else { 0.0 },
                ..default()
            });
            if in_beat > 7.0 {
                script.frames_ms.push(time.delta_secs() * 1000.0);
            }
            if in_beat > 17.0 {
                shoot(&mut commands, &script.dir, "cockpit-stride.png");
                next = true;
            }
        }
        // A few frames after the last shot, so its readback lands even at a software renderer's frame rate.
        6 if in_beat > 1.5 && {
            *exit_frames += 1;
            *exit_frames > 6
        } =>
        {
            let mut f = script.frames_ms.clone();
            f.sort_by(f32::total_cmp);
            println!(
                "FRAME_TIME_MS frames={} p50={:.2} p95={:.2} p99={:.2} max={:.2}",
                f.len(),
                percentile(&f, 0.5),
                percentile(&f, 0.95),
                percentile(&f, 0.99),
                f.last().copied().unwrap_or(0.0)
            );
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
        script.done.clear();
    }
}

/// Frames from a showcase take, for checking effects headless (openspec change `mech-damage-fx` task 4.1; CLAUDE.md
/// section 4 allows it as an instrument): with `CB_FRAMES_DIR` set, the primary window is saved every
/// `CB_FRAMES_EVERY_S` seconds of game time (default 2) as `frame-<n>.png`, at most `CB_FRAMES_MAX` (default 90).
/// It saves pictures and nothing else; a take with frames saved is not a frame-time result (CLAUDE.md section 7).
pub struct FramesPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Frames {
    dir: PathBuf,
    every_s: f32,
    max: u32,
    taken: u32,
    next_s: f32,
}

impl Plugin for FramesPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        let env = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        app.insert_resource(Frames {
            dir: self.dir.clone(),
            every_s: env("CB_FRAMES_EVERY_S", 2.0).max(0.1),
            max: env("CB_FRAMES_MAX", 90.0) as u32,
            taken: 0,
            next_s: 0.0,
        })
        .add_systems(Update, save_frames);
    }
}

fn save_frames(mut commands: Commands, mut frames: ResMut<Frames>, time: Res<Time>, ready: Option<Res<crate::loading::GameReady>>) {
    if !ready.is_some_and(|r| r.0) || frames.taken >= frames.max {
        return;
    }
    let now = time.elapsed_secs();
    if now < frames.next_s {
        return;
    }
    frames.next_s = now + frames.every_s;
    let name = format!("frame-{:03}.png", frames.taken);
    frames.taken += 1;
    shoot(&mut commands, &frames.dir.clone(), &name);
}

/// The effects, staged in front of the eye for headless stills (openspec changes `fx-sprites`, `blast-scorch`,
/// `rocket-plumes`, `mech-damage-fx`; their stills tasks; an instrument, CLAUDE.md section 4): with `--stills <dir>` and
/// `CB_STILLS_FX=1`, the pilot stands on foot 70 m from the nearest enemy and the player's own guns are fired at it as
/// real shots (rockets and autocannon rounds, resolved by `combat`), then the enemy is worn down by the damage rule's
/// own state (critical parts, a leg lost), then the pilot faces open soil where real explosions dig craters. Stills at
/// each stage; then it exits. It writes no input and plays no game: it only stages what a fight would make.
pub struct FxStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct FxScript {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    enemy: Option<Entity>,
    soil: Option<Vec3>,
    /// What this beat has done already: a frame is about 0.25 s of game time on a software renderer, so each step runs
    /// once, the first frame it is due, never in a narrow time window.
    done: Vec<&'static str>,
}

impl FxScript {
    /// True the first time `step` is asked for in this beat.
    fn once(&mut self, step: &'static str) -> bool {
        if self.done.contains(&step) {
            return false;
        }
        self.done.push(step);
        true
    }
}

impl Plugin for FxStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(FxScript {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            enemy: None,
            soil: None,
            done: Vec::new(),
        })
        .add_systems(Update, run_fx_script);
    }
}

/// The pilot on foot at `feet`, looking at `at`.
fn stand_looking(pilot: &mut PilotSim, feet: Vec3, at: Vec3) {
    let to = at - feet;
    let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
    s.pitch_rad = (to.y - 1.7).atan2(to.with_y(0.0).length());
    pilot.prev = s.clone();
    pilot.curr = s;
}

#[allow(clippy::too_many_arguments)]
fn run_fx_script(
    mut commands: Commands,
    mut script: ResMut<FxScript>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    time: Res<Time>,
    plan: Res<crate::world::Plan>,
    mut shots: ResMut<crate::combat::Shots>,
    mut explosions: MessageWriter<crate::blast::Explosion>,
    mut mechs: Query<(
        Entity,
        &crate::mech::MechId,
        &crate::mech::MechSpec,
        &mut crate::mech::MechSim,
        &GlobalTransform,
    )>,
    mut exit: MessageWriter<AppExit>,
    ready: Option<Res<crate::loading::GameReady>>,
) {
    use cb_core::world::Ground;
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    let mut next = false;
    let enemy_at = |mechs: &Query<(
        Entity,
        &crate::mech::MechId,
        &crate::mech::MechSpec,
        &mut crate::mech::MechSim,
        &GlobalTransform,
    )>,
                    e: Option<Entity>| { e.and_then(|e| mechs.get(e).ok()).map(|m| m.4.translation()) };
    match script.beat {
        0 => {
            next = ready.is_some_and(|r| r.0) && access.0.is_some() && t > 4.0;
            if next {
                // The nearest live enemy to the player's mech.
                let home = access.0.unwrap().ladder_base;
                script.enemy = mechs
                    .iter()
                    .filter(|m| m.1.0 != 0 && !m.3.dead)
                    .min_by(|a, b| a.4.translation().distance(home).total_cmp(&b.4.translation().distance(home)))
                    .map(|m| m.0);
                if script.enemy.is_none() {
                    eprintln!("fx stills: no enemy");
                    exit.write(AppExit::error());
                }
            }
        }
        // Stand 70 m off the enemy, then fire the player's own rockets and rounds at it.
        1 => {
            let Some(e) = enemy_at(&mechs, script.enemy) else { return };
            let flat = Vec3::new(1.0, 0.0, 0.6).normalize();
            let feet = Vec3::new(e.x, plan.height_at(e.x + flat.x * 70.0, e.z + flat.z * 70.0), e.z) + flat * 70.0;
            let torso = e + Vec3::Y * 14.0;
            if script.once("stand") {
                stand_looking(&mut pilot, feet, torso);
            }
            if t > 1.0 && script.once("fire") {
                let player = mechs.iter().find(|m| m.1.0 == 0).map(|m| m.2.0.clone());
                if let Some(spec) = player {
                    let launch = feet + Vec3::new(-flat.z, 0.0, flat.x) * 25.0 + Vec3::Y * 12.0;
                    for (wi, w) in spec.weapons.iter().enumerate() {
                        let n = if w.rocket {
                            3
                        } else if w.blast_mj < 1.0 {
                            12
                        } else {
                            2
                        };
                        for k in 0..n {
                            let aim = torso + Vec3::new((k as f32 - 5.0) * 0.8, (k % 4) as f32 * 3.0 - 6.0, 0.0);
                            let from = launch + Vec3::Y * k as f32 * 0.6 - flat * (k as f32 * if w.rocket { 25.0 } else { 0.0 });
                            shots.0.push(cb_core::weapons::Projectile {
                                owner: 0,
                                weapon: wi,
                                pos: from,
                                vel: (aim - from).normalize() * w.speed_m_s,
                                age_s: 0.0,
                                rocket: w.rocket,
                                target: None,
                            });
                        }
                    }
                }
            }
            if t > 1.5 && script.once("rockets") {
                shoot(&mut commands, &script.dir, "fx-1-rockets.png");
            }
            if t > 5.0 {
                shoot(&mut commands, &script.dir, "fx-2-hits.png");
                next = true;
            }
        }
        // Wear it down: every part to a fifth (they smoke), the hull under a quarter (it burns), the left leg off.
        2 => {
            if script.once("wear")
                && let Some(e) = script.enemy
                && let Ok((_, _, spec, mut sim, _)) = mechs.get_mut(e)
            {
                let parts = spec.parts.clone();
                for (i, p) in parts.iter().enumerate() {
                    sim.damage.structure[i] = p.structure * 0.18;
                    if p.kind == cb_core::mech::damage::PartKind::LegL {
                        sim.damage.structure[i] = 0.0;
                        sim.damage.lost[i] = true;
                        sim.cuts.push(i);
                    }
                }
            }
            if t > 3.0 && script.once("damaged") {
                shoot(&mut commands, &script.dir, "fx-3-damaged.png");
            }
            if t > 9.0 {
                shoot(&mut commands, &script.dir, "fx-4-burning.png");
                next = true;
            }
        }
        // Open soil: the nearest point from the pilot that is soil, 50 m in front; blasts there.
        3 => {
            if script.once("soil") {
                let from = pilot.curr.feet;
                let mut found = None;
                'search: for ring in 1..80 {
                    for k in 0..24 {
                        let a = k as f32 / 24.0 * std::f32::consts::TAU;
                        let p = Vec2::new(from.x, from.z) + Vec2::new(a.cos(), a.sin()) * ring as f32 * 15.0;
                        if plan.0.surface_at(p) == cb_core::world::Surface::Dirt
                            && plan.0.surface_at(p + Vec2::X * 30.0) == cb_core::world::Surface::Dirt
                            && plan.0.surface_at(p - Vec2::X * 30.0) == cb_core::world::Surface::Dirt
                            && plan.water_at(p.x, p.y).is_none()
                        {
                            found = Some(Vec3::new(p.x, plan.height_at(p.x, p.y), p.y));
                            break 'search;
                        }
                    }
                }
                script.soil = found;
                if let Some(c) = found {
                    let feet = c + Vec3::new(40.0, 0.0, 22.0);
                    let feet = feet.with_y(plan.height_at(feet.x, feet.z));
                    stand_looking(&mut pilot, feet, c);
                    explosions.write(crate::blast::Explosion {
                        at: c + Vec3::Y * 0.5,
                        energy_mj: 6.0,
                    });
                    explosions.write(crate::blast::Explosion {
                        at: c + Vec3::new(-14.0, 0.5, -10.0),
                        energy_mj: 25.0,
                    });
                } else {
                    eprintln!("fx stills: no open soil near");
                }
            }
            if t > 0.5 && script.once("blast") {
                shoot(&mut commands, &script.dir, "fx-5-blast.png");
            }
            if t > 4.0 && script.once("smoke") {
                shoot(&mut commands, &script.dir, "fx-6-smoke.png");
            }
            if t > 16.0 {
                shoot(&mut commands, &script.dir, "fx-7-scorch.png");
                next = true;
            }
        }
        4 if t > 3.0 => {
            println!("FX_STILLS done");
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
    }
}
