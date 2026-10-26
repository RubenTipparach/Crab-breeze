//! The career in the game (openspec changes `career` and `mission-jobs`): a career run begins from the
//! title menu (New career, or Continue from `saves/career.ron`), steps the job board every fixed step
//! with where the pilot is and how many of the job's enemies are left, brings a job's enemy mechs in
//! when the pilot arrives at its site, pays a won job through the career's ledger, and saves. The rules
//! are `cb_core::career` and `cb_core::jobs`; the mail is read and answered on the menu's MAIL page and the
//! cockpit's (`mfd_apps`).

use crate::mech::{Loadout, MechId, MechSim, Piloted, SpawnAssets, spawn_enemy};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::career::{Career, CareerData, Field};
use cb_core::jobs::{Event, Site};
use cb_core::pilot::PilotMode;
use cb_core::rng::SimRng;
use cb_core::shop::To;

/// The career being played, and its data.
#[derive(Resource)]
pub struct CareerRun {
    /// The career.
    pub career: Career,
    /// The career's data: how it starts, the jobs, the shop's catalogue and the mechs it lists.
    pub data: CareerData,
    /// What the shop's or refit page last said (a purchase made or refused), until another tab is opened.
    pub shop_note: Option<String>,
    /// The piloted mech's fit or livery changed (or the career just began): the flown mech is rebuilt from its
    /// design's data plus the fit (mech-custom).
    pub refit_dirty: bool,
    /// Seconds since the last save.
    since_save_s: f32,
    /// Events from the mail page (a job abandoned for another), handled with the board's own next step.
    pub pending: Vec<Event>,
    /// Owned mechs whose place in the field a rule just set (a call-out at a base, a save continued): the world
    /// puts each one there (`air_drop::place_owned`).
    pub moved: Vec<u32>,
    /// The world has been matched to the career once (the piloted design, every mech in the field).
    pub placed: bool,
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
        app.add_game_systems(Update, begin.run_if(resource_added::<CareerStart>))
            .add_game_systems(Update, (apply_fit, paint_livery).chain().run_if(resource_exists::<CareerRun>))
            .add_game_systems(
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
        refit_dirty: true,
        since_save_s: 0.0,
        pending: Vec::new(),
        moved: Vec::new(),
        placed: false,
    });
}

/// The piloted mech's owned record, if the career has one.
pub fn piloted(run: &CareerRun) -> Option<&cb_core::career::OwnedMech> {
    run.career.piloting.and_then(|k| run.career.mech(k))
}

/// Rebuilds the piloted mech from its design's data plus the career's fit (mech-custom): the guns it fires, its
/// armour's structure (damage kept as a share of each part) and its mass. Its guns come back loaded.
fn apply_fit(
    mut commands: Commands,
    mut run: ResMut<CareerRun>,
    data: Res<Data>,
    mut mech: Query<(Entity, &mut MechSim, &crate::mech::MechSpec), With<Piloted>>,
) {
    if !run.refit_dirty {
        return;
    }
    let Some((entity, mut sim, spec)) = mech.iter_mut().next() else {
        return;
    };
    run.refit_dirty = false;
    let Some(owned) = piloted(&run).cloned() else { return };
    let design = run.data.design(&owned.design).unwrap_or(&data.0.mech);
    let flown = cb_core::refit::fitted(design, &owned, &run.data.parts);
    let sim = &mut *sim;
    for (i, p) in flown.parts.iter().enumerate() {
        let (Some(s), Some(old)) = (sim.damage.structure.get_mut(i), spec.parts.get(i)) else {
            continue;
        };
        *s = *s / old.structure.max(1e-3) * p.structure;
    }
    sim.guns = cb_core::weapons::Guns::default();
    sim.selected = cb_core::weapons::first_station(&flown.weapons);
    let names: Vec<&str> = flown.weapons.iter().map(|w| w.name.as_str()).collect();
    info!(
        "REFIT {}: {}, armour grade {}, {:.0} t",
        flown.name,
        names.join(", "),
        owned.armour,
        flown.locomotion.mass_kg / 1000.0
    );
    let tint = run.data.parts.liveries.get(owned.livery as usize).map_or([1.0; 3], |l| l.tint);
    commands
        .entity(entity)
        .insert((crate::mech::MechSpec(std::sync::Arc::new(flown)), Livery(tint)));
}

/// The piloted mech's livery: a tint over its paint (mech-custom, free).
#[derive(Component, Clone, Copy, PartialEq)]
pub struct Livery(pub [f32; 3]);

/// A model mesh's own material, kept so a livery tints from it, not from the last tint.
#[derive(Component)]
struct Painted {
    original: Handle<StandardMaterial>,
    tint: [f32; 3],
}

/// Tints every mesh of the piloted mech's model (not its cockpit) by its livery, as the model loads.
#[allow(clippy::type_complexity)]
fn paint_livery(
    mut commands: Commands,
    mechs: Query<(&crate::mech::MechRoot, &Livery), With<Piloted>>,
    children: Query<&Children>,
    mut meshes: Query<(&mut MeshMaterial3d<StandardMaterial>, Option<&Painted>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (root, livery) in &mechs {
        for e in children.iter_descendants(root.model) {
            let Ok((mut mat, painted)) = meshes.get_mut(e) else { continue };
            if painted.is_some_and(|p| p.tint == livery.0) {
                continue;
            }
            let original = painted.map_or_else(|| mat.0.clone(), |p| p.original.clone());
            let Some(base) = materials.get(&original).cloned() else { continue };
            let [r, g, b] = livery.0;
            let c = base.base_color.to_linear();
            let mut tinted = base;
            tinted.base_color = Color::LinearRgba(LinearRgba::new(c.red * r, c.green * g, c.blue * b, c.alpha));
            mat.0 = materials.add(tinted);
            commands.entity(e).insert(Painted { original, tint: livery.0 });
        }
    }
}

/// What a refit at the terminal does (mech-custom), on the piloted mech; returns what the page says.
pub fn refit(run: &mut CareerRun, what: Refit) -> String {
    use cb_core::refit as r;
    let Some(owned) = piloted(run).cloned() else {
        return "NO MECH".into();
    };
    let kept = run.data.career.ledger_kept;
    let stock = run.data.design(&owned.design).map(|c| c.weapons.clone()).unwrap_or_default();
    let parts = run.data.parts.clone();
    let armour_max = run.data.catalogue.listing(&owned.design).map_or(0, |l| l.armour_max);
    let name = |i: usize| parts.weapons.get(i).map_or("?".to_string(), |w| w.name.to_uppercase());
    let done = match what {
        Refit::BuyPart(i) => r::buy_part(&mut run.career, &parts, &parts.weapons[i.min(parts.weapons.len() - 1)].id, kept)
            .map(|_| format!("BOUGHT {}", name(i))),
        Refit::Fit(station, part) => {
            let id = part.and_then(|i| parts.weapons.get(i)).map(|w| w.id.as_str());
            r::fit(&mut run.career, &parts, owned.key, station, id, &stock).map(|_| {
                let on = part.map_or_else(|| "STOCK".to_string(), name);
                format!("FITTED {on} TO STATION {}", station + 1)
            })
        }
        Refit::Armour => r::raise_armour(&mut run.career, &parts, owned.key, armour_max, kept).map(|g| format!("ARMOUR GRADE {g}")),
        Refit::Livery(n) => {
            r::paint(&mut run.career, &parts, owned.key, n).map(|_| format!("PAINTED {}", parts.liveries[n as usize].name.to_uppercase()))
        }
    };
    match done {
        Ok(text) => {
            info!("REFIT {text}");
            run.refit_dirty = true;
            save(&run.career);
            text
        }
        Err(e) => {
            info!("REFIT refused: {}", e.text());
            e.text()
        }
    }
}

/// A refit asked for on the terminal's REFIT tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refit {
    /// Buy the parts list's weapon at this index into the spares.
    BuyPart(usize),
    /// Fit the spare weapon at this parts index to this station, or put the stock one back (`None`).
    Fit(usize, Option<usize>),
    /// Raise the armour a grade.
    Armour,
    /// Paint this livery.
    Livery(u32),
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

/// Buys `design` at the shop (mech-shop section 6): air dropped on good ground near the player (`near`: where they
/// are and the mechs standing), the site found by `air_drop::drop_site`; with no good ground near, delivered to a free
/// bay of the base the pilot is at (`at`), else of the nearest base with one. Returns what the page says.
pub fn buy_mech(
    run: &mut CareerRun,
    plan: &cb_core::world::WorldPlan,
    bases: &cb_core::world::bases::BasesConfig,
    design: &str,
    at: Option<u32>,
    near: &crate::air_drop::Where,
) -> String {
    let centres: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
    let bays = cb_core::shop::Bays {
        per_base: bases.hangars as u32,
        centres: &centres,
    };
    let (Some(whole), Some(cfg)) = (run.data.whole(design), run.data.design(design)) else {
        return cb_core::shop::Refusal::NotListed.text();
    };
    let name = cfg.name.clone();
    // Where it lands: fixed now, from where the player is (design section 6.1).
    let rng = SimRng::new(run.career.seed).fork(&format!("drop/{}", run.career.mechs.len()));
    let ground = cb_core::air_drop::PlanGround { plan, bases };
    let keep = cb_core::air_drop::Keep {
        pilot: near.pilot,
        mechs: &near.mechs,
    };
    let needs = cb_core::air_drop::Needs::of(cfg);
    let site = cb_core::air_drop::drop_site(&run.data.drop, &ground, needs, &keep, &rng).map(|s| s.at);
    let kept = run.data.career.ledger_kept;
    let tier_before = max_tier(run);
    let order = cb_core::shop::Order {
        design,
        at,
        from: near.pilot,
        site,
        rng: &rng,
    };
    let inbox = run.data.jobs.inbox_kept;
    match cb_core::shop::buy(&mut run.career, &run.data.catalogue, &bays, &order, &whole, kept) {
        Ok(d) => {
            let note = match d.to {
                To::Drop(site) => {
                    let to = Vec2::new(site.x, site.z) - near.pilot;
                    let (m, brg) = (to.length(), crate::hud::whole_deg(crate::hud::bearing_deg(to)));
                    let land_s = run.data.drop.delay_s + cb_core::air_drop::fall_s(&run.data.drop);
                    info!(
                        "SHOP bought {name}: air drop at ({:.0}, {:.0}), {m:.0} m brg {brg:03}, {} credits left",
                        site.x, site.z, run.career.money
                    );
                    run.career.board.say(
                        "Hangar Office",
                        &format!("Your {name} is on its way by air"),
                        &format!(
                            "Your {name} is being air dropped near you. It lands {m:.0} m from where you ordered it, bearing \
                             {brg:03}, in about {land_s:.0} s. Follow the AIR DROP marker, and stand clear of the drop zone."
                        ),
                        None,
                        inbox,
                    );
                    format!("BOUGHT {}: AIR DROP INBOUND, {m:.0} M BRG {brg:03}", name.to_uppercase())
                }
                To::Bay { base, bay, away } => {
                    info!(
                        "SHOP bought {name}: no ground for a drop, base {base} bay {bay}, {} credits left",
                        run.career.money
                    );
                    if away {
                        run.career.board.say(
                            "Hangar Office",
                            &format!("Your {name} has arrived"),
                            &format!(
                                "There was no clear ground near you for an air drop. Your {name} is in bay {} at base {}.",
                                bay + 1,
                                base + 1
                            ),
                            None,
                            inbox,
                        );
                    }
                    format!(
                        "BOUGHT {}: NO GROUND FOR A DROP, BASE {} BAY {}",
                        name.to_uppercase(),
                        base + 1,
                        bay + 1
                    )
                }
            };
            // A riskier tier opened: the mail says so and its first job is offered at once.
            let tier = max_tier(run);
            if tier > tier_before {
                let sites = job_sites(plan);
                let bonus = run.career.pay_bonuses(centres.len(), &run.data.career);
                let offers = cb_core::jobs::Offers {
                    sites: &sites,
                    harbours: &[],
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
            note
        }
        Err(r) => {
            info!("SHOP refused {name}: {}", r.text());
            r.text()
        }
    }
}

/// Calls owned mech `key` out at the terminal's base (owner, survey J16: "you can swap mechs out at base, with foreman
/// always availible"): into the terminal's own bay, or the base's first bay no mech stands in (`near.mechs`), facing
/// the apron, to be walked up to and boarded. A mech parked at this base comes; the Foreman comes from anywhere, whole
/// if it was wrecked (`wrecked`). Returns what the page says.
pub fn call_out(
    run: &mut CareerRun,
    plan: &cb_core::world::WorldPlan,
    bases: &cb_core::world::bases::BasesConfig,
    terminal: cb_core::world::bases::Terminal,
    key: u32,
    wrecked: bool,
    near: &crate::air_drop::Where,
) -> String {
    let Some(base) = plan.bases.iter().find(|b| b.id == terminal.base) else {
        return "NO BASE".into();
    };
    let hangars = base.hangars(bases);
    let clear = |h: &cb_core::world::layout::Rect| near.mechs.iter().all(|(p, r)| h.distance(*p) > *r);
    let bay = std::iter::once(terminal.bay)
        .chain(0..hangars.len())
        .find(|&i| hangars.get(i).is_some_and(clear));
    let Some(bay) = bay else {
        return "NO CLEAR BAY".into();
    };
    let h = hangars[bay];
    let ground = plan.terrain.height_at(h.centre().x, h.centre().y);
    // Standing in the bay facing out of its mouth (low z): heading pi faces +z, so 0 faces the apron.
    let at = Field {
        pos: [h.centre().x, ground, h.centre().y],
        heading_rad: 0.0,
    };
    let site = base.rect();
    let whole = run.career.mech(key).and_then(|m| run.data.whole(&m.design)).unwrap_or_default();
    let start = run.data.career.start_mech.clone();
    let name = run
        .career
        .mech(key)
        .and_then(|m| run.data.design(&m.design))
        .map_or_else(|| "MECH".to_string(), |c| c.name.to_uppercase());
    match run
        .career
        .call_out(key, base.id, bay as u32, at, &|p| site.distance(p) <= 0.0, &start, wrecked, &whole)
    {
        Ok(repaired) => {
            info!("HANGAR called out {name} (key {key}) to base {} bay {}", base.id, bay);
            run.moved.push(key);
            save(&run.career);
            format!("CALLED OUT {name} TO BAY {}{}", bay + 1, if repaired { ", REBUILT" } else { "" })
        }
        Err(e) => {
            info!("HANGAR refused {name}: {}", e.text());
            e.text()
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
    (sea, mut sea_started): (Option<Res<crate::boats::SeaJobs>>, MessageWriter<crate::boats::SeaJobStarted>),
) {
    let dt = time.delta_secs();
    let run = &mut *run;
    run.career.time_s += f64::from(dt);
    let sites = job_sites(&plan);
    let centres: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
    let bonus = run.career.pay_bonuses(centres.len(), &run.data.career);
    // Shipping jobs are sited at the harbours (harbours).
    let harbours = sea.as_deref().map_or(&[][..], |s| &s.sites[..]);
    let offers = cb_core::jobs::Offers {
        sites: &sites,
        harbours,
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
    // A job's enemies left: its mechs, and at sea its gunboats and an escort's freighter not yet in (harbours).
    let left =
        current.map(|id| enemies.iter().filter(|(s, e)| e.0 == id && !s.dead).count() as u32 + sea.as_deref().map_or(0, |s| s.left(id)));
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
                // A shipping job's gunboats and ward are the boats' to bring (harbours).
                if job.kind.at_sea() {
                    info!(
                        "JOB {id} started at sea: {} gunboats at ({:.0}, {:.0})",
                        job.mechs, job.site.x, job.site.y
                    );
                    sea_started.write(crate::boats::SeaJobStarted(id));
                    continue;
                }
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
                    let ring = rng.range_f32(0.0, 1.0);
                    let xz = cb_core::jobs::enemy_spot(&run.data.jobs, job.site, at, a, ring);
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
