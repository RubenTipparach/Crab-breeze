//! The city defence mission in the game (openspec change `city-defence`): steps
//! `cb_core::mission::Defence` every fixed step with what the simulation knows, spawns each wave's
//! raiders where `spawn_points` says when the mission says so, gives every raider its goal (the
//! pilot or a building of the attacked city, with its siege burst window) before the mechs step,
//! and logs `MISSION` lines. The rules are in `cb-core`; the cockpit readout reads `DefenceRun`.

use crate::mech::{EnemyGoal, Loadout, MechId, MechSim, Piloted, SpawnAssets, spawn_enemy};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::mission::{
    Defence, Event, Goal, Loss, Phase, RaiderView, Situation, city_damage, raider_goal, siege_fires, siege_point, spawn_points,
};
use cb_core::pilot::PilotMode;
use cb_core::rng::SimRng;

/// Marks a run of the defence mission (`--mission defence` and its showcase and bench).
#[derive(Resource)]
pub struct Defending;

/// The mission as it stands, for the cockpit and the scripted pilot.
#[derive(Resource, Default)]
pub struct DefenceRun {
    /// The rules' state.
    pub state: Defence,
    /// Each city's damage, 0..1, by city index (`city_damage`), this step.
    pub damage: Vec<f32>,
    /// Raiders of the current wave destroyed.
    pub destroyed: u32,
    /// Raiders of the current wave still fighting.
    pub alive: u32,
    status_in_s: f32,
}

impl DefenceRun {
    /// The attacked city's damage now.
    pub fn city_damage(&self, cfg: &cb_core::mission::DefenceConfig) -> f32 {
        self.state
            .city(cfg)
            .and_then(|c| self.damage.get(c as usize).copied())
            .unwrap_or(0.0)
    }
}

/// A raider of the defence mission: its wave, its place in the wave, and the time since it was hit.
#[derive(Component)]
pub struct Raider {
    /// Wave (0 first).
    pub wave: usize,
    /// Its place in the wave (0 first): its loadout and siege burst window.
    pub index: usize,
    since_hit_s: f32,
}

/// Runs the mission.
pub struct DefencePlugin;

impl Plugin for DefencePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Defending)
            .init_resource::<DefenceRun>()
            .add_systems(FixedUpdate, give_goals.in_set(SimSet::Input))
            .add_systems(
                FixedUpdate,
                step_mission
                    .in_set(SimSet::Mech)
                    .after(crate::combat::fly_shots)
                    .after(crate::mech::step_mech),
            );
    }
}

fn say(line: &str) {
    println!("{line}");
}

/// The mission clock and the waves.
#[allow(clippy::too_many_arguments)]
fn step_mission(
    mut commands: Commands,
    mut run: ResMut<DefenceRun>,
    data: Res<Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    pilot: Res<PilotSim>,
    time: Res<Time<Fixed>>,
    me: Query<&MechSim, With<Piloted>>,
    raiders: Query<(&MechSim, &Raider)>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cfg = &data.0.defence;
    let dt = time.delta_secs();
    run.damage = (0..plan.cities.len() as u32)
        .map(|c| city_damage(&plan, &live.structures, c))
        .collect();
    let wave = run.state.wave();
    let (mut alive, mut destroyed) = (0, 0);
    for (sim, r) in &raiders {
        if Some(r.wave) == wave {
            if sim.dead {
                destroyed += 1;
            } else {
                alive += 1;
            }
        }
    }
    run.alive = alive;
    run.destroyed = destroyed;
    let situation = Situation {
        seated: pilot.curr.mode == PilotMode::Seated,
        destroyed,
        city_damage: run.city_damage(cfg),
        pilot_down: me.iter().any(|m| m.dead),
    };
    let event = run.state.step(cfg, &situation, dt);
    match event {
        Some(Event::WaveStarts(w)) => {
            let city = cfg.cities[w];
            say(&format!(
                "MISSION wave {} of {} starts: {} raiders on city {}, {:.0} s on the clock",
                w + 1,
                cfg.cities.len(),
                cfg.raiders_per_wave,
                city + 1,
                cfg.wave_time_s
            ));
            let points = spawn_points(&plan, cfg, w, &SimRng::new(plan.seed));
            let mut sa = SpawnAssets {
                tex: &tex,
                assets: &assets,
                meshes: &mut meshes,
                materials: &mut materials,
            };
            for (i, (xz, heading)) in points.into_iter().enumerate() {
                let spec = &cfg.raiders[i % cfg.raiders.len()];
                let loadout = Loadout {
                    mech: &spec.mech,
                    weapons: spec.weapons.as_deref(),
                    armour_scale: spec.armour_scale,
                    tint: spec.tint,
                    skill: &spec.skill,
                };
                let id = MechId(100 + (w * cfg.raiders_per_wave as usize + i) as u32);
                let rng = SimRng::new(plan.seed).fork(&format!("raider/{w}/{i}"));
                if let Some(e) = spawn_enemy(&mut commands, &plan, &loadout, xz, heading, rng, id, &mut sa) {
                    commands.entity(e).insert(Raider {
                        wave: w,
                        index: i,
                        since_hit_s: f32::INFINITY,
                    });
                }
            }
        }
        Some(Event::WaveCleared(w)) => say(&format!(
            "MISSION wave {} cleared, city {} at {:.1} % damage",
            w + 1,
            cfg.cities[w] + 1,
            run.damage.get(cfg.cities[w] as usize).copied().unwrap_or(0.0) * 100.0
        )),
        Some(Event::Won) => say(&format!("MISSION won: {}", damage_line(&run.damage, cfg))),
        Some(Event::Lost(why)) => say(&format!("MISSION lost: {}: {}", loss_text(why), damage_line(&run.damage, cfg))),
        None => {}
    }
    run.status_in_s -= dt;
    if run.status_in_s <= 0.0 {
        run.status_in_s = 10.0;
        if let Phase::Wave { wave, elapsed_s } = run.state.phase {
            say(&format!(
                "MISSION status wave {} t {:.0} s raiders {}/{} {}",
                wave + 1,
                elapsed_s,
                run.alive,
                cfg.raiders_per_wave,
                damage_line(&run.damage, cfg)
            ));
        }
    }
}

/// `city 1 12.3 %, city 2 0.0 %, city 3 0.0 %` for the mission's cities.
fn damage_line(damage: &[f32], cfg: &cb_core::mission::DefenceConfig) -> String {
    cfg.cities
        .iter()
        .map(|c| format!("city {} {:.1} %", c + 1, damage.get(*c as usize).copied().unwrap_or(0.0) * 100.0))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why the mission was lost, in the pilot's words.
pub fn loss_text(why: Loss) -> String {
    match why {
        Loss::CityFell { city, damage } => format!("city {} fell ({:.0} % destroyed)", city + 1, damage * 100.0),
        Loss::OutOfTime { wave } => format!("wave {} was not stopped in time", wave + 1),
        Loss::PilotDown => "your mech was destroyed".into(),
    }
}

/// Every live raider's goal this step, before the mechs step: the pilot, or the nearest standing
/// building of the attacked city with its burst window.
fn give_goals(
    data: Res<Data>,
    run: Res<DefenceRun>,
    plan: Res<Plan>,
    live: Res<Live>,
    time: Res<Time<Fixed>>,
    me: Query<&MechSim, With<Piloted>>,
    mut raiders: Query<(&MechSim, &mut Raider, &mut EnemyGoal)>,
) {
    let cfg = &data.0.defence;
    let dt = time.delta_secs();
    let Some(city) = run.state.city(cfg) else { return };
    let Some(player) = me.iter().next().map(|m| m.curr.pos) else {
        return;
    };
    // The live raiders in a stable order: wave, then place in the wave.
    let mut order: Vec<(usize, usize, RaiderView)> = raiders
        .iter_mut()
        .filter(|(sim, _, _)| !sim.dead)
        .map(|(sim, mut r, _)| {
            r.since_hit_s = if sim.took_hit { 0.0 } else { r.since_hit_s + dt };
            (
                r.wave,
                r.index,
                RaiderView {
                    pos: sim.curr.pos,
                    since_hit_s: r.since_hit_s,
                },
            )
        })
        .collect();
    order.sort_by_key(|o| (o.0, o.1));
    let views: Vec<RaiderView> = order.iter().map(|o| o.2).collect();
    let t = time.elapsed_secs();
    for (sim, r, mut goal) in &mut raiders {
        if sim.dead {
            continue;
        }
        let Some(i) = order.iter().position(|o| o.0 == r.wave && o.1 == r.index) else {
            continue;
        };
        *goal = match raider_goal(i, &views, player, cfg) {
            Goal::Pilot => EnemyGoal::default(),
            Goal::City => EnemyGoal {
                point: siege_point(&plan, &live.structures, city, sim.curr.pos, cfg.facade_out_m),
                hold_fire: !siege_fires(r.index, cfg.raiders_per_wave as usize, t, cfg),
            },
        };
    }
}
