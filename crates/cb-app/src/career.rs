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
use cb_core::career::{Career, CareerData};
use cb_core::jobs::{Event, Site};
use cb_core::pilot::PilotMode;
use cb_core::rng::SimRng;

/// The career being played, and its data.
#[derive(Resource)]
pub struct CareerRun {
    /// The career.
    pub career: Career,
    /// The career's data: how it starts, the jobs, the shop's catalogue and the mechs it lists.
    pub data: CareerData,
    /// What the shop's page last said (a purchase made or refused), until another page is drawn.
    pub shop_note: Option<String>,
    /// Seconds since the last save.
    since_save_s: f32,
    /// Events from the mail page (a job abandoned for another), handled with the board's own next step.
    pub pending: Vec<Event>,
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
    let cdata = match CareerData::load(&root) {
        Ok(d) => d,
        Err(e) => {
            error!("career: {e}");
            return;
        }
    };
    let whole: Vec<f32> = data.0.mech.parts.iter().map(|p| p.structure).collect();
    let fresh = || Career::new("career", plan.seed ^ 0x5eed, &cdata.career, &whole);
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
        data: cdata,
        shop_note: None,
        since_save_s: 0.0,
        pending: Vec::new(),
    });
}

/// The cities jobs are sited by.
fn job_sites(plan: &cb_core::world::WorldPlan) -> Vec<Site> {
    plan.cities
        .iter()
        .map(|c| Site {
            city: c.id,
            centre: c.centre,
            half_size_m: c.half_size_m,
        })
        .collect()
}

/// The riskiest job tier the career's mechs open (each design's `tier`).
pub fn max_tier(run: &CareerRun) -> u8 {
    run.career.max_tier(|d| run.data.design(d).map(|c| c.tier))
}

/// A job ended: the standing with the base that offered it moves (career-progression).
fn standing(run: &mut CareerRun, id: u32, won: bool) {
    if let Some(base) = run.career.board.job(id).map(|j| j.base) {
        let now = run.career.job_ended(base, won, &run.data.career);
        info!("career: standing with base {} now {now}", base + 1);
    }
}

/// Buys `design` at the shop (mech-shop): delivered to a free bay of the base the pilot is at (`at`), else of the
/// nearest base to `from` with one; a delivery away from the pilot is mailed. Returns what the page says.
pub fn buy_mech(
    run: &mut CareerRun,
    plan: &cb_core::world::WorldPlan,
    bays_per_base: u32,
    design: &str,
    at: Option<u32>,
    from: Vec2,
) -> String {
    let centres: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
    let bays = cb_core::shop::Bays {
        per_base: bays_per_base,
        centres: &centres,
    };
    let Some(whole) = run.data.whole(design) else {
        return cb_core::shop::Refusal::NotListed.text();
    };
    let name = run.data.design(design).map_or_else(|| design.to_string(), |c| c.name.clone());
    let kept = run.data.career.ledger_kept;
    let tier_before = max_tier(run);
    match cb_core::shop::buy(&mut run.career, &run.data.catalogue, &bays, design, at, from, &whole, kept) {
        Ok(d) => {
            info!(
                "SHOP bought {name}: base {} bay {}, {} credits left",
                d.base, d.bay, run.career.money
            );
            if d.away {
                let inbox = run.data.jobs.inbox_kept;
                run.career.board.say(
                    "Hangar Office",
                    &format!("Your {name} has arrived"),
                    &format!("Your {name} is in bay {} at base {}.", d.bay + 1, d.base + 1),
                    None,
                    inbox,
                );
            }
            // A riskier tier opened: the mail says so and its first job is offered at once.
            let tier = max_tier(run);
            if tier > tier_before {
                let sites = job_sites(plan);
                let bonus = run.career.pay_bonuses(centres.len(), &run.data.career);
                let offers = cb_core::jobs::Offers {
                    sites: &sites,
                    bases: &centres,
                    max_tier: tier,
                    bonus: &bonus,
                };
                let seed = run.career.seed;
                for t in tier_before + 1..=tier {
                    if let Some(id) = run.career.board.open_tier(&run.data.jobs, &offers, seed, t) {
                        info!("career: tier {t} opened, job {id} offered");
                    }
                }
            }
            save(&run.career);
            format!("BOUGHT {}: BASE {} BAY {}", name.to_uppercase(), d.base + 1, d.bay + 1)
        }
        Err(r) => {
            info!("SHOP refused {name}: {}", r.text());
            r.text()
        }
    }
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
    enemy_entities: Query<(Entity, &JobEnemy)>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let run = &mut *run;
    run.career.time_s += f64::from(dt);
    let sites = job_sites(&plan);
    let centres: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
    let bonus = run.career.pay_bonuses(centres.len(), &run.data.career);
    let offers = cb_core::jobs::Offers {
        sites: &sites,
        bases: &centres,
        max_tier: max_tier(run),
        bonus: &bonus,
    };
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
    let mut events = std::mem::take(&mut run.pending);
    events.extend(
        run.career
            .board
            .step(&run.data.jobs, &offers, seed, at, if active { left } else { None }, pilot_down, dt),
    );
    let kept = run.data.career.ledger_kept;
    let inbox = run.data.jobs.inbox_kept;
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
                standing(run, id, true);
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
            Event::Abandoned(id) => {
                // given up for another job: its enemies leave, unpaid (free-roam-missions)
                info!("JOB {id} abandoned");
                standing(run, id, false);
                for (e, je) in &enemy_entities {
                    if je.0 == id {
                        commands.entity(e).despawn();
                    }
                }
                run.career.done.push(cb_core::career::JobRecord {
                    job: id,
                    won: false,
                    paid: 0,
                });
                dirty = true;
            }
            Event::Failed(id) => {
                info!("JOB {id} failed");
                standing(run, id, false);
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
