//! The career in the game (openspec changes `career` and `mission-jobs`): a career run begins from the
//! title menu (New career, or Continue from `saves/career.ron`), steps the job board every fixed step
//! with where the pilot is and how many of the job's enemies are left, brings a job's enemy mechs in
//! when the pilot arrives at its site, pays a won job through the career's ledger, and saves. The rules
//! are `cb_core::career` and `cb_core::jobs`; the mail is read and answered on the menu's MAIL page.

use crate::mech::{Loadout, MechId, MechSim, Piloted, SpawnAssets, spawn_enemy};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::career::{Career, CareerConfig};
use cb_core::jobs::{Event, JobsConfig, Site};
use cb_core::pilot::PilotMode;
use cb_core::rng::SimRng;

/// The career being played, and its data.
#[derive(Resource)]
pub struct CareerRun {
    /// The career.
    pub career: Career,
    /// `data/career/career.ron`.
    pub cfg: CareerConfig,
    /// `data/jobs/jobs.ron`.
    pub jobs: JobsConfig,
    /// Seconds since the last save.
    since_save_s: f32,
}

/// How the career begins, chosen on the title menu.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum CareerStart {
    /// A new career.
    New,
    /// The saved one.
    Continue,
}

/// An enemy mech brought in by a job (its id).
#[derive(Component)]
pub struct JobEnemy(pub u32);

/// The save's path.
pub fn save_path() -> std::path::PathBuf {
    crate::cli::repo_root().join("saves").join("career.ron")
}

/// True when there is a career to continue.
pub fn can_continue() -> bool {
    save_path().exists()
}

/// Writes the career (through a temporary file, so a crash keeps the last good save).
/// Writes the career to its save file (through a temporary file); also used by `save_game`.
pub(crate) fn save(c: &Career) {
    let p = save_path();
    let _ = std::fs::create_dir_all(p.parent().unwrap_or(std::path::Path::new(".")));
    let tmp = p.with_extension("ron.tmp");
    if let Err(e) = std::fs::write(&tmp, c.to_ron()).and_then(|_| std::fs::rename(&tmp, &p)) {
        warn!("career: could not save {}: {e}", p.display());
    }
}

/// Runs the career.
pub struct CareerPlugin;

impl Plugin for CareerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, begin.run_if(resource_added::<CareerStart>)).add_systems(
            FixedUpdate,
            step_jobs
                .in_set(SimSet::Mech)
                .after(crate::combat::fly_shots)
                .after(crate::mech::step_mech)
                .run_if(resource_exists::<CareerRun>),
        );
    }
}

/// Starts the career: a new one, or the save (a bad save is reported and a new career begun, the bad
/// file left where it is).
fn begin(mut commands: Commands, start: Res<CareerStart>, data: Res<Data>, plan: Res<Plan>) {
    let root = crate::cli::repo_root().join("data");
    let cfg: CareerConfig = cb_core::data::load_ron(&root.join("career/career.ron")).unwrap_or_else(|e| {
        warn!("career: {e}");
        CareerConfig::default()
    });
    let jobs: JobsConfig = match cb_core::data::load_ron(&root.join("jobs/jobs.ron")) {
        Ok(j) => j,
        Err(e) => {
            error!("career: {e}");
            return;
        }
    };
    let whole: Vec<f32> = data.0.mech.parts.iter().map(|p| p.structure).collect();
    let fresh = || Career::new("career", plan.seed ^ 0x5eed, &cfg, &whole);
    let career = match *start {
        CareerStart::New => fresh(),
        CareerStart::Continue => std::fs::read_to_string(save_path())
            .map_err(|e| e.to_string())
            .and_then(|t| Career::from_ron(&t))
            .unwrap_or_else(|e| {
                warn!("career: {} not loaded: {e}; a new career begins", save_path().display());
                fresh()
            }),
    };
    info!(
        "career: {} credits, {} mechs, {} jobs done",
        career.money,
        career.mechs.len(),
        career.done.len()
    );
    save(&career);
    commands.insert_resource(CareerRun {
        career,
        cfg,
        jobs,
        since_save_s: 0.0,
    });
}

/// Steps the job board: offers, arrival (the job's enemies come in), the win (pay) or the loss.
#[allow(clippy::too_many_arguments)]
fn step_jobs(
    mut commands: Commands,
    mut run: ResMut<CareerRun>,
    data: Res<Data>,
    plan: Res<Plan>,
    pilot: Res<PilotSim>,
    time: Res<Time<Fixed>>,
    me: Query<&MechSim, With<Piloted>>,
    enemies: Query<(&MechSim, &JobEnemy)>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let run = &mut *run;
    run.career.time_s += f64::from(dt);
    let sites: Vec<Site> = plan
        .cities
        .iter()
        .map(|c| Site {
            city: c.id,
            centre: c.centre,
            half_size_m: c.half_size_m,
        })
        .collect();
    let mech = me.iter().next();
    let at = match (pilot.curr.mode == PilotMode::Seated, mech) {
        (true, Some(m)) => Vec2::new(m.curr.pos.x, m.curr.pos.z),
        _ => Vec2::new(pilot.curr.feet.x, pilot.curr.feet.z),
    };
    let current = run.career.board.current().map(|j| j.id);
    let left = current.map(|id| enemies.iter().filter(|(s, e)| e.0 == id && !s.dead).count() as u32);
    let active = current.is_some_and(|id| run.career.board.job(id).is_some_and(|j| j.state == cb_core::jobs::JobState::Active));
    let pilot_down = mech.is_some_and(|m| m.dead);
    let seed = run.career.seed;
    let events = run
        .career
        .board
        .step(&run.jobs, &sites, seed, at, if active { left } else { None }, pilot_down, dt);
    let kept = run.cfg.ledger_kept;
    let inbox = run.jobs.inbox_kept;
    let mut dirty = false;
    for e in events {
        match e {
            Event::Offered(id) => info!("career: job {id} offered"),
            Event::Arrived(id) => {
                let Some(job) = run.career.board.job(id).cloned() else { continue };
                info!("JOB {id} started: {} mechs at ({:.0}, {:.0})", job.mechs, job.site.x, job.site.y);
                let cfg = &data.0.defence;
                let mut sa = SpawnAssets {
                    tex: &tex,
                    assets: &assets,
                    meshes: &mut meshes,
                    materials: &mut materials,
                };
                // The job's mechs come in round its site, facing it (the defence's raiders' loadouts).
                for i in 0..job.mechs as usize {
                    let spec = &cfg.raiders[i % cfg.raiders.len().max(1)];
                    let loadout = Loadout {
                        mech: &spec.mech,
                        weapons: spec.weapons.as_deref(),
                        armour_scale: spec.armour_scale,
                        tint: spec.tint,
                        skill: &spec.skill,
                        skin: spec.skin.as_deref().and_then(|s| data.0.skins.skins.get(s)),
                    };
                    let mut rng = SimRng::new(seed).fork(&format!("job/{id}/mech/{i}"));
                    let a = rng.range_f32(0.0, std::f32::consts::TAU);
                    let r = rng.range_f32(150.0, 350.0);
                    let xz = job.site + Vec2::new(a.cos(), a.sin()) * r;
                    let heading = (xz.x - job.site.x).atan2(xz.y - job.site.y);
                    let mech_id = MechId(1000 + id * 16 + i as u32);
                    if let Some(ent) = spawn_enemy(&mut commands, &plan, &loadout, xz, heading, rng, mech_id, &mut sa) {
                        commands.entity(ent).insert(JobEnemy(id));
                    }
                }
            }
            Event::Won(id, pay) => {
                info!("JOB {id} won: {pay} credits");
                run.career.pay(pay, &format!("job {id}"), kept);
                run.career.done.push(cb_core::career::JobRecord {
                    job: id,
                    won: true,
                    paid: pay,
                });
                run.career.board.say(
                    "Accounts",
                    &format!("Paid: {pay} credits"),
                    &format!("Job {id} is complete. {pay} credits have been paid to your account."),
                    Some(id),
                    inbox,
                );
                dirty = true;
            }
            Event::Failed(id) => {
                info!("JOB {id} failed");
                run.career.done.push(cb_core::career::JobRecord {
                    job: id,
                    won: false,
                    paid: 0,
                });
                run.career.board.say(
                    "Accounts",
                    &format!("Job {id} failed"),
                    "The contract has lapsed. No payment will be made.",
                    Some(id),
                    inbox,
                );
                dirty = true;
            }
        }
    }
    run.since_save_s += dt;
    if dirty || run.since_save_s > 60.0 {
        run.since_save_s = 0.0;
        save(&run.career);
    }
}
