//! Every boat in the game (openspec change `harbours`): the harbours drawn by the coastal cities with their walkable
//! quays and the harbour office's terminal, the timetabled boats made real round the pilot and stepped through
//! `cb_core::ships`, drawn from their block-out models riding the water's swell with spray at the bow, hit through
//! the weapons' hit list, splashed by blasts, shoved and swamped by mechs, fleeing from fights and hunted by crabs;
//! the `BoatHarmed` report for career standing; and the shipping jobs' gunboats, escort freighter and blockade. The
//! rules are `cb_core::ships`; this is the Bevy adapter.

use crate::fx::Fx;
use crate::mech::{Enemy, EnemyGoal, MechId, MechSim, MechSpec, Piloted};
use crate::pilot::PilotSim;
use crate::sim::{Data, PresentSet, SimSet};
use crate::state::GameApp;
use crate::units::{Hostiles, UnitTargets, Units};
use crate::world::{Live, MeshBuilder, Plan};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use cb_core::jobs::{JobKind, JobState, SeaSite};
use cb_core::pilot::PilotMode;
use cb_core::rng::SimRng;
use cb_core::ships::harm::{self, Cause, Prey};
use cb_core::ships::{Boat, Errand, Footprint, GUNBOAT, Harbour, Phase, Sea, ShipsConfig, Traffic, float};
use cb_core::units::{Unit, heading_of as unit_heading};
use cb_core::world::platform::{FIXED_PREFIX, Platform, Slab};
use std::collections::{BTreeMap, BTreeSet};

/// Hit-list ids from here up are boats (a boat's id plus this); units are from `units::UNIT_ID_BASE`.
pub const BOAT_ID_BASE: u32 = 20_000;

/// Every boat on the island and the harbours they sail from.
#[derive(Resource)]
pub struct Boats {
    /// The harbours, lanes, timetable and the boats real now.
    pub traffic: Traffic,
}

/// A round struck a boat (`boat`: its id), or a round's blast went off near the boats (`boat` none): the boats take
/// it in `hit_boats`, knowing who fired.
#[derive(Message, Clone, Copy, Debug)]
pub struct BoatHit {
    /// The boat struck, if a round struck one; none for a blast.
    pub boat: Option<u32>,
    /// Where.
    pub at: Vec3,
    /// Armour the round beats, millimetres.
    pub penetration_mm: f32,
    /// Hit points it takes.
    pub damage: f32,
    /// The blast's energy, megajoules (a blast).
    pub blast_mj: f32,
    /// The mech or vehicle that fired it.
    pub owner: u32,
}

/// Harm by the pilot to a crewed boat (openspec change `harbours`, design section 6; the cars' `W37` rule): written
/// once on the pilot's first harm to a boat and once more if the pilot's harm sinks it. Career standing reads it.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct BoatHarmed {
    /// The boat's id.
    pub boat: u32,
    /// Its kind.
    pub kind: String,
    /// Its civil crew.
    pub crew: u32,
    /// The air base nearest it, whose standing it costs.
    pub base: Option<u32>,
    /// Where (x, z).
    pub at: Vec2,
    /// The pilot's harm sank it.
    pub sunk: bool,
}

/// A shipping job's fight starts (the pilot reached its site): its gunboats and ward are brought in (`career`).
#[derive(Message, Clone, Copy, Debug)]
pub struct SeaJobStarted(pub u32);

/// A shipping job under way: its gunboats (unit ids) and its ward (boat id).
#[derive(Clone, Debug, Default)]
pub struct SeaJob {
    /// The raiders' gunboats.
    pub gunboats: Vec<u32>,
    /// An escort's freighter.
    pub ward: Option<u32>,
    /// The ward has moored.
    pub ward_in: bool,
    /// Its enemies left: gunboats afloat, plus one while the ward is not in.
    pub left: u32,
}

/// The harbours as the jobs board sees them, and the shipping jobs under way.
#[derive(Resource, Default)]
pub struct SeaJobs {
    /// Where shipping jobs are sited.
    pub sites: Vec<SeaSite>,
    /// The jobs under way, by job id.
    pub jobs: BTreeMap<u32, SeaJob>,
    /// Jobs started whose boats are not in yet.
    pub pending: BTreeSet<u32>,
}

impl SeaJobs {
    /// Enemies left on job `id` (zero for a job that is not at sea).
    pub fn left(&self, id: u32) -> u32 {
        if self.pending.contains(&id) {
            return 1;
        }
        self.jobs.get(&id).map_or(0, |j| j.left)
    }
}

/// A boat drawn: its id, the slick it left, and when it last threw spray.
#[derive(Component)]
struct BoatDraw {
    id: u32,
}

/// An oil slick: the boat that left it.
#[derive(Component)]
struct Slick {
    id: u32,
    material: Handle<StandardMaterial>,
}

/// Draws and runs the boats.
pub struct BoatsPlugin;

impl Plugin for BoatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BoatHit>()
            .add_message::<BoatHarmed>()
            .add_message::<SeaJobStarted>()
            .init_resource::<SeaJobs>()
            .add_world_spawn(spawn_harbours)
            .add_game_systems(
                FixedUpdate,
                (start_sea_jobs, aim_raiders, crabs_hunt)
                    .chain()
                    .in_set(SimSet::Input)
                    .after(crate::units::list_hostiles)
                    .run_if(resource_exists::<Boats>),
            )
            .add_game_systems(
                FixedUpdate,
                step_boats
                    .in_set(SimSet::Mech)
                    .after(crate::mech::step_mech)
                    .before(crate::combat::fly_shots)
                    .run_if(resource_exists::<Boats>),
            )
            .add_game_systems(
                FixedUpdate,
                (hit_boats, keep_sea_jobs)
                    .chain()
                    .in_set(SimSet::Mech)
                    .after(crate::combat::fly_shots)
                    .run_if(resource_exists::<Boats>),
            )
            .add_game_systems(
                Update,
                (draw_boats, draw_slicks).in_set(PresentSet::Pose).run_if(resource_exists::<Boats>),
            );
    }
}

/// The sea the boats sail on: the plan's ground and water.
fn sea(plan: &Plan) -> Sea<'_> {
    Sea {
        ground: plan.0.as_ref(),
        sea_level_m: plan.waters.sea_level_m,
    }
}

/// Where the pilot is (x, z): their mech's place when seated, else their feet.
fn pilot_at(pilot: &PilotSim, me: Option<&MechSim>) -> Vec2 {
    match (pilot.curr.mode == PilotMode::Seated, me) {
        (true, Some(m)) => Vec2::new(m.curr.pos.x, m.curr.pos.z),
        _ => Vec2::new(pilot.curr.feet.x, pilot.curr.feet.z),
    }
}

/// The hit shapes of every boat afloat: one capsule along its hull, from its keel to its deck (`BOAT_ID_BASE` ids).
pub fn boat_capsules(boats: &Boats, data: &Data) -> Vec<cb_core::weapons::PartCapsule> {
    let cfg = &data.0.ships;
    boats
        .traffic
        .boats
        .iter()
        .filter(|b| b.afloat())
        .filter_map(|b| {
            let spec = cfg.kinds.get(&b.kind)?;
            let (a, z, r) = b.hull(spec);
            // The capsule's axis at mid freeboard; its radius the larger of half the beam and half the freeboard.
            let free = spec.size_m[1] - spec.draft_m;
            let y = b.surface_m + free * 0.25;
            Some(cb_core::weapons::PartCapsule {
                mech: BOAT_ID_BASE + b.id,
                part: 0,
                a: Vec3::new(a.x, y, a.y),
                b: Vec3::new(z.x, y, z.y),
                radius_m: r.max(free * 0.35),
            })
        })
        .collect()
}

/// The harbour office terminal a pilot on foot at `feet` looking along `yaw_rad` can use (the air bases' reach and look
/// rule), as a terminal whose `base` is the air base nearest the harbour.
pub fn office_terminal(
    boats: &Boats,
    plan: &Plan,
    feet: Vec2,
    yaw_rad: f32,
    reach_m: f32,
    look_deg: f32,
) -> Option<cb_core::world::bases::Terminal> {
    let hs = &boats.traffic.harbours;
    let id = cb_core::ships::harbour::office_in_reach(hs, feet, yaw_rad, reach_m, look_deg)?;
    let h = hs.iter().find(|h| h.id == id)?;
    let centres: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
    let base = harm::nearest(&centres, h.terminal).map_or(0, |i| plan.bases[i].id);
    Some(cb_core::world::bases::Terminal {
        base,
        bay: 0,
        at: h.terminal,
        office: Some(h.id),
    })
}

/// Lays out the harbours, lanes and timetable, draws each harbour and lays its quay as a fixed platform.
#[allow(clippy::too_many_arguments)]
fn spawn_harbours(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    mut live: ResMut<Live>,
    mut sea_jobs: ResMut<SeaJobs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
) {
    let started = std::time::Instant::now();
    let w = &data.0.world;
    let road_m = w.highway_width_m * 0.5 + w.highway_flat_margin_m;
    let cfg = &data.0.ships;
    let traffic = Traffic::from_plan(&plan, cfg, road_m, w.world_size_m, plan.seed);
    info!(
        "boats: {} harbours, {} sea lanes, {} timetabled boats{} laid out in {:.0} ms",
        traffic.harbours.len(),
        traffic.lanes,
        traffic.slots.len(),
        if traffic.islet.is_some() { ", a ferry," } else { "," },
        started.elapsed().as_secs_f64() * 1000.0
    );
    *sea_jobs = SeaJobs {
        sites: traffic.sea_sites(cfg.lanes.escort_out_m, cfg.harbours.approach_m),
        ..default()
    };
    // The quays, walkable: fixed platforms in each harbour's frame (a world spawn again replaces them).
    live.platforms.retain(|p| !p.is_fixed());
    let concrete = crate::buildings::concrete_material(&mut images, &mut materials);
    let paint = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.75,
        ..default()
    });
    let screen = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.2, 0.08),
        emissive: LinearRgba::rgb(0.3, 1.6, 0.5),
        ..default()
    });
    let crane = assets.load(GltfAssetLabel::Scene(0).from_asset("models/boats/crane.glb"));
    for h in &traffic.harbours {
        if h.port {
            // The seaport (seaport-layout): its buildings, cranes and quays are the site's lots and ground
            // (`small_buildings`); its quays' tops are walked on as fixed platforms in the world's frame.
            if let Some(site) = plan.sites.iter().find(|s| s.city == h.city) {
                let slabs = site
                    .solids
                    .iter()
                    .enumerate()
                    .map(|(i, (lo, hi))| Slab {
                        name: format!("{FIXED_PREFIX}port_quay_{}_{i}", h.id),
                        min: Vec2::new(lo.x, lo.z),
                        max: Vec2::new(hi.x, hi.z),
                        centre: Vec2::new(lo.x + hi.x, lo.z + hi.z) * 0.5,
                        top_m: hi.y,
                        slope: Vec2::ZERO,
                    })
                    .collect();
                live.platforms.push(Platform {
                    slabs,
                    pose: bevy::math::Affine3A::IDENTITY,
                    was: bevy::math::Affine3A::IDENTITY,
                });
            }
            continue;
        }
        let frame = harbour_frame(h);
        live.platforms.push(Platform {
            slabs: quay_slabs(h),
            pose: frame,
            was: frame,
        });
        let (slab, marks, screens) = harbour_meshes(h, cfg, &plan);
        let (s, r, t) = frame.to_scale_rotation_translation();
        let root = commands
            .spawn((
                Name::new(format!("harbour {}", h.id)),
                Transform::from_translation(t).with_rotation(r).with_scale(s),
                Visibility::default(),
            ))
            .id();
        for (mesh, mat, name) in [
            (slab.build_with_tangents(), concrete.clone(), "quay and buildings"),
            (marks.build(), paint.clone(), "markings"),
            (screens.build(), screen.clone(), "terminal screen"),
        ] {
            commands.spawn((
                Name::new(format!("harbour {} {name}", h.id)),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mat),
                Transform::IDENTITY,
                ChildOf(root),
            ));
        }
        // The cranes on the pier head, jibs out to sea (local -z), from the block-out model.
        for c in &h.cranes {
            let l = local(h, c.centre);
            commands.spawn((
                Name::new(format!("harbour {} crane", h.id)),
                WorldAssetRoot(crane.clone()),
                Transform::from_xyz(l.x, h.deck_y, l.y),
                ChildOf(root),
            ));
        }
    }
    commands.insert_resource(Boats { traffic });
}

/// A harbour's frame: its origin at the shore point at sea level, local +x along the shore, local -z out to sea.
fn harbour_frame(h: &Harbour) -> bevy::math::Affine3A {
    let yaw = Footprint {
        centre: h.shore,
        along: h.side(),
        half: Vec2::ONE,
    }
    .yaw();
    bevy::math::Affine3A::from_rotation_translation(Quat::from_rotation_y(yaw), Vec3::new(h.shore.x, 0.0, h.shore.y))
}

/// A world point (x, z) in a harbour's frame (local x, local z).
fn local(h: &Harbour, p: Vec2) -> Vec2 {
    let d = p - h.shore;
    Vec2::new(d.dot(h.side()), -d.dot(h.out))
}

/// A footprint's corners in a harbour's frame (least, greatest).
fn local_box(h: &Harbour, f: &Footprint) -> (Vec2, Vec2) {
    let c = f.corners().map(|p| local(h, p));
    let lo = c.iter().fold(Vec2::splat(f32::MAX), |a, b| a.min(*b));
    let hi = c.iter().fold(Vec2::splat(f32::MIN), |a, b| a.max(*b));
    (lo, hi)
}

/// The quay's walkable tops in the harbour's frame: the apron, the pier and the head, at the deck's height.
fn quay_slabs(h: &Harbour) -> Vec<Slab> {
    h.decks()
        .iter()
        .zip(["apron", "pier", "head"])
        .map(|(f, name)| {
            let (min, max) = local_box(h, f);
            Slab {
                name: format!("{FIXED_PREFIX}quay_{name}_{}", h.id),
                min,
                max,
                centre: (min + max) * 0.5,
                top_m: h.deck_y,
                slope: Vec2::ZERO,
            }
        })
        .collect()
}

/// The harbour's block-out in its own frame: the quay deck on its piles, the warehouses, the office with its
/// awning and the terminal; the edge markings; the terminal's screen.
fn harbour_meshes(h: &Harbour, cfg: &ShipsConfig, plan: &Plan) -> (MeshBuilder, MeshBuilder, MeshBuilder) {
    let mut slab = MeshBuilder::tiled(4.0, 4.0);
    let mut marks = MeshBuilder::default();
    let mut screens = MeshBuilder::default();
    let deck = h.deck_y;
    let deck_tone = [0.34, 0.33, 0.31, 1.0];
    let pile_tone = [0.25, 0.24, 0.22, 1.0];
    let ground_at = |l: Vec2| {
        let p = h.shore + h.side() * l.x - h.out * l.y;
        plan.terrain.height_at(p.x, p.y)
    };
    for f in h.decks() {
        let (lo, hi) = local_box(h, &f);
        slab.block(Vec3::new(lo.x, deck - 1.2, lo.y), Vec3::new(hi.x, deck, hi.y), deck_tone);
        // Piles every 12 m down to the bed, where the deck stands over water.
        let mut x = lo.x + 3.0;
        while x < hi.x {
            let mut z = lo.y + 3.0;
            while z < hi.y {
                let g = ground_at(Vec2::new(x, z));
                if g < deck - 1.2 {
                    slab.block(
                        Vec3::new(x - 0.5, g - 1.0, z - 0.5),
                        Vec3::new(x + 0.5, deck - 1.2, z + 0.5),
                        pile_tone,
                    );
                }
                z += 12.0;
            }
            x += 12.0;
        }
    }
    // A yellow line along the berth face.
    let (lo, hi) = local_box(h, &h.head);
    let y = deck + 0.02;
    marks.quad(
        [
            Vec3::new(lo.x, y, lo.y + 1.5),
            Vec3::new(hi.x, y, lo.y + 1.5),
            Vec3::new(hi.x, y, lo.y + 1.0),
            Vec3::new(lo.x, y, lo.y + 1.0),
        ],
        [0.95, 0.75, 0.15, 1.0],
    );
    let [_, wh, _] = cfg.harbours.warehouse_m;
    for (f, foot) in &h.warehouses {
        let (lo, hi) = local_box(h, f);
        slab.block(
            Vec3::new(lo.x, foot - 1.0, lo.y),
            Vec3::new(hi.x, foot + wh, hi.y),
            [0.55, 0.5, 0.42, 1.0],
        );
        // A pitched roof's ridge as a low block (a block-out).
        slab.block(
            Vec3::new(lo.x, foot + wh, (lo.y + hi.y) * 0.5 - 2.0),
            Vec3::new(hi.x, foot + wh + 1.5, (lo.y + hi.y) * 0.5 + 2.0),
            [0.4, 0.42, 0.44, 1.0],
        );
    }
    let [_, oh, _] = cfg.harbours.office_m;
    let (lo, hi) = local_box(h, &h.office);
    slab.block(
        Vec3::new(lo.x, deck - 1.0, lo.y),
        Vec3::new(hi.x, deck + oh, hi.y),
        [0.78, 0.76, 0.7, 1.0],
    );
    // The door on its seaward face (local -z, the low z side), and an awning over the terminal.
    let door = (lo.x + hi.x) * 0.5;
    marks.quad(
        [
            Vec3::new(door + 1.0, deck, lo.y - 0.02),
            Vec3::new(door - 1.0, deck, lo.y - 0.02),
            Vec3::new(door - 1.0, deck + 2.3, lo.y - 0.02),
            Vec3::new(door + 1.0, deck + 2.3, lo.y - 0.02),
        ],
        [0.15, 0.2, 0.3, 1.0],
    );
    let t = local(h, h.terminal);
    slab.block(
        Vec3::new(t.x - 4.0, deck + 3.2, lo.y - (cfg.harbours.terminal_out_m + 1.5)),
        Vec3::new(t.x + 4.0, deck + 3.4, lo.y),
        [0.6, 0.18, 0.12, 1.0],
    );
    // The terminal: a pedestal with its screen facing the quay (toward the sea, local -z).
    slab.block(
        Vec3::new(t.x - 0.4, deck, t.y - 0.3),
        Vec3::new(t.x + 0.4, deck + 1.2, t.y + 0.3),
        [0.2, 0.22, 0.24, 1.0],
    );
    screens.quad(
        [
            Vec3::new(t.x + 0.35, deck + 1.25, t.y - 0.31),
            Vec3::new(t.x - 0.35, deck + 1.25, t.y - 0.31),
            Vec3::new(t.x - 0.35, deck + 1.75, t.y - 0.31),
            Vec3::new(t.x + 0.35, deck + 1.75, t.y - 0.31),
        ],
        [1.0, 1.0, 1.0, 1.0],
    );
    slab.block(
        Vec3::new(t.x - 0.4, deck + 1.2, t.y - 0.3),
        Vec3::new(t.x + 0.4, deck + 1.8, t.y + 0.3),
        [0.2, 0.22, 0.24, 1.0],
    );
    (slab, marks, screens)
}

/// A shipping job's fight starts: its gunboats come in, and an escort's freighter sails for the harbour.
#[allow(clippy::too_many_arguments)]
fn start_sea_jobs(
    mut started: MessageReader<SeaJobStarted>,
    mut boats: ResMut<Boats>,
    mut sea_jobs: ResMut<SeaJobs>,
    mut units: ResMut<Units>,
    data: Res<Data>,
    plan: Res<Plan>,
    career: Option<Res<crate::career::CareerRun>>,
) {
    for SeaJobStarted(id) in started.read().copied() {
        sea_jobs.pending.remove(&id);
        let Some(job) = career.as_deref().and_then(|c| c.career.board.job(id)).cloned() else {
            continue;
        };
        let (Some(h), Some(spec)) = (
            job.harbour.and_then(|h| boats.traffic.harbours.iter().find(|x| x.id == h)).cloned(),
            data.0.units.units.get(GUNBOAT),
        ) else {
            continue;
        };
        let cfg = &data.0.ships;
        let sea = sea(&plan);
        let mut sj = SeaJob::default();
        // Where they come in: an escort's raiders out along the lane past the freighter, a raid's round the ground,
        // a blockade's across the mouth.
        let out_dir = (job.site - h.berth).normalize_or(h.out);
        let side = out_dir.perp();
        if job.kind == JobKind::Escort
            && let Some((route, s_m, p)) = boats.traffic.escort_start(h.id, cfg.lanes.escort_out_m, cfg.harbours.approach_m)
        {
            let dir = boats.traffic.routes[route as usize].at(s_m).1;
            let errand = Errand::Route {
                route,
                s_m,
                forward: false,
            };
            if let Some(w) = boats
                .traffic
                .add(cb_core::ships::FREIGHTER, cfg, errand, p, cb_core::ships::heading_of(-dir))
            {
                if let Some(b) = boats.traffic.boat_mut(w) {
                    b.ward = Some(id);
                }
                sj.ward = Some(w);
            }
        }
        for k in 0..job.mechs {
            let mut rng = SimRng::new(plan.seed).fork(&format!("job/{id}/gunboat/{k}"));
            let spread = (k as f32 - (job.mechs as f32 - 1.0) * 0.5) * 150.0;
            let mut at = match job.kind {
                JobKind::Escort => job.site + out_dir * rng.range_f32(1500.0, 2500.0) + side * spread,
                JobKind::Raid => {
                    let a = rng.range_f32(0.0, std::f32::consts::TAU);
                    job.site + Vec2::new(a.cos(), a.sin()) * rng.range_f32(300.0, 600.0)
                }
                _ => h.mouth + side * spread,
            };
            // On water that floats it: out to sea until it does.
            for _ in 0..40 {
                if sea.sea_depth(at) >= spec.draft_m + 2.0 {
                    break;
                }
                at += out_dir * 50.0;
            }
            let heading = unit_heading(Vec3::new(h.berth.x - at.x, 0.0, h.berth.y - at.y));
            let y = Unit::riding_y(spec, plan.waters.sea_level_m);
            let unit = units.add_for(
                |u| Unit::new(u, GUNBOAT, spec, Vec3::new(at.x, y, at.y), heading, u),
                None,
                None,
                None,
            );
            sj.gunboats.push(unit);
        }
        sj.left = sj.gunboats.len() as u32 + u32::from(sj.ward.is_some());
        info!(
            "JOB {id} at sea: {} gunboats{} off harbour {}",
            sj.gunboats.len(),
            if sj.ward.is_some() { " and the freighter" } else { "" },
            h.id
        );
        sea_jobs.jobs.insert(id, sj);
    }
}

/// Each job gunboat's target this step: the pilot once in range, else the job's ward or the boats it raids.
fn aim_raiders(
    boats: Res<Boats>,
    sea_jobs: Res<SeaJobs>,
    units: Res<Units>,
    data: Res<Data>,
    pilot: Res<PilotSim>,
    me: Query<&MechSim, With<Piloted>>,
    mut targets: ResMut<UnitTargets>,
    career: Option<Res<crate::career::CareerRun>>,
) {
    targets.0.clear();
    let Some(spec) = data.0.units.units.get(GUNBOAT) else { return };
    let at = pilot_at(&pilot, me.iter().next());
    let t = &boats.traffic;
    for (job, sj) in &sea_jobs.jobs {
        let site = career.as_deref().and_then(|c| c.career.board.job(*job)).map(|j| (j.kind, j.site));
        let marks: Vec<Prey> = match site {
            Some((JobKind::Escort, _)) => sj.ward.and_then(|w| t.boat(w)).map(prey).into_iter().collect(),
            Some((JobKind::Raid, s)) => t.boats.iter().filter(|b| b.pos.distance(s) < 3000.0).map(prey).collect(),
            _ => Vec::new(),
        };
        for g in &sj.gunboats {
            let Some(u) = units.list.iter().find(|p| p.unit.id == *g && p.unit.alive() && !p.gone) else {
                continue;
            };
            let here = Vec2::new(u.unit.pos.x, u.unit.pos.z);
            if let Some(mark) = harm::raider_target(here, at, &marks, spec.fire_range_m).and_then(|m| t.boat(m)) {
                let v = mark.vel();
                let y = mark.surface_m + 3.0;
                targets
                    .0
                    .insert(*g, (Vec3::new(mark.pos.x, y, mark.pos.y), Vec3::new(v.x, 0.0, v.y)));
            }
        }
    }
}

/// A boat as the fighting sees it.
fn prey(b: &Boat) -> Prey {
    Prey {
        id: b.id,
        pos: b.pos,
        crewed: b.crewed(),
    }
}

/// A crab with no defence orders and the pilot far off goes for a crewed ship near it; it lets go when the ship is
/// gone or the pilot comes near.
fn crabs_hunt(
    boats: Res<Boats>,
    data: Res<Data>,
    pilot: Res<PilotSim>,
    me: Query<&MechSim, With<Piloted>>,
    mut crabs: Query<(&MechSim, &mut EnemyGoal, Option<&mut Hunting>, Entity), (With<Enemy>, Without<crate::defence::Raider>)>,
    mut commands: Commands,
) {
    let f = &data.0.ships.fight;
    let at = pilot_at(&pilot, me.iter().next());
    let ships: Vec<Prey> = boats.traffic.boats.iter().map(prey).collect();
    for (sim, mut goal, hunting, e) in &mut crabs {
        if sim.dead {
            continue;
        }
        let here = Vec2::new(sim.curr.pos.x, sim.curr.pos.z);
        let pick = harm::crab_prey(here, at, &ships, f.crab_reach_m, f.crab_pilot_m).and_then(|id| boats.traffic.boat(id));
        match (pick, hunting) {
            (Some(b), _) => {
                let y = b.surface_m;
                goal.point = Some(Vec3::new(b.pos.x, y, b.pos.y));
                commands.entity(e).insert(Hunting);
            }
            (None, Some(_)) => {
                goal.point = None;
                commands.entity(e).remove::<Hunting>();
            }
            (None, None) => {}
        }
    }
}

/// A crab going after a ship (its goal point is the ship's).
#[derive(Component)]
struct Hunting;

/// Steps the boats round the pilot; boats near a hostile run; wading and skimming mechs shove and swamp small boats.
#[allow(clippy::too_many_arguments)]
fn step_boats(
    mut boats: ResMut<Boats>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Fixed>>,
    pilot: Res<PilotSim>,
    hostiles: Res<Hostiles>,
    mechs: Query<(&MechSim, &MechSpec, Has<Piloted>)>,
    mut harmed: MessageWriter<BoatHarmed>,
    mut fx: MessageWriter<Fx>,
) {
    let _t = crate::perf_hud::slow("step_boats");
    let cfg = &data.0.ships;
    let me = mechs.iter().find(|m| m.2).map(|m| m.0);
    let at = pilot_at(&pilot, me);
    let sea = sea(&plan);
    let boats = &mut *boats;
    boats.traffic.step(cfg, &sea, at, time.delta_secs());
    let flee = cfg.fight.flee_hostile_m;
    for b in &mut boats.traffic.boats {
        if let Some(h) = hostiles.0.iter().find(|h| Vec2::new(h.aim.x, h.aim.z).distance(b.pos) < flee) {
            b.alarm(Vec2::new(h.aim.x, h.aim.z), cfg.fight.flee_s);
        }
    }
    // Mechs in the water against small boats.
    for (sim, spec, piloted) in &mechs {
        let wet = sim.curr.wading_depth_m > 0.5 || sim.curr.on_water != cb_core::mech::locomotion::OnWater::Dry;
        if sim.dead || !wet {
            continue;
        }
        let here = Vec2::new(sim.curr.pos.x, sim.curr.pos.z);
        let vel = sim.curr.vel;
        let radius = spec.locomotion.body_radius_m;
        let bases: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
        for b in boats.traffic.boats.iter_mut().filter(|b| b.pos.distance(here) < 40.0) {
            let Some(bs) = cfg.kinds.get(&b.kind) else { continue };
            let crewed = b.crewed();
            if b.shove(bs, here, vel, radius, &cfg.fight) == Some(true) {
                fx.write(Fx::new("skate_spray", Vec3::new(b.pos.x, plan.waters.sea_level_m, b.pos.y)).scaled(2.0));
                let cause = if piloted { Cause::PilotMech } else { Cause::Other };
                if let Some(h) = harm::report(b, cause, crewed, true) {
                    harmed.write(harmed_msg(b, &bases, &plan, h.sunk));
                }
            }
        }
    }
}

/// The harm report for boat `b`.
fn harmed_msg(b: &Boat, bases: &[Vec2], plan: &Plan, sunk: bool) -> BoatHarmed {
    BoatHarmed {
        boat: b.id,
        kind: b.kind.clone(),
        crew: b.crew,
        base: harm::nearest(bases, b.pos).map(|i| plan.bases[i].id),
        at: b.pos,
        sunk,
    }
}

/// Rounds and blasts land on the boats; the pilot's harm to crewed boats is reported; blasts set boats running; a
/// sunk gunboat goes down as a boat.
#[allow(clippy::too_many_arguments)]
fn hit_boats(
    mut hits: MessageReader<BoatHit>,
    mut explosions: MessageReader<crate::blast::Explosion>,
    mut boats: ResMut<Boats>,
    data: Res<Data>,
    plan: Res<Plan>,
    units: Res<Units>,
    piloted: Query<&MechId, With<Piloted>>,
    mut harmed: MessageWriter<BoatHarmed>,
    mut fx: MessageWriter<Fx>,
    mut wrecked: Local<BTreeSet<u32>>,
) {
    let cfg = &data.0.ships;
    let pilot_id = piloted.iter().next().map(|m| m.0);
    let bases: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
    let crater_r = data.0.world.craters.radius_m_per_cbrt_mj;
    let t = &mut boats.traffic;
    for hit in hits.read().copied() {
        let cause = if Some(hit.owner) == pilot_id {
            Cause::PilotFire
        } else {
            Cause::Other
        };
        let at = Vec2::new(hit.at.x, hit.at.z);
        let mut struck: Vec<(u32, f32, f32)> = Vec::new();
        match hit.boat {
            Some(id) => struck.push((id, hit.penetration_mm, hit.damage)),
            None => {
                let r = crater_r * hit.blast_mj.max(0.01).cbrt();
                for b in t.boats.iter().filter(|b| b.afloat()) {
                    if let Some((pen, dmg)) = crate::blast::splash(b.pos.distance(at), r, hit.blast_mj) {
                        struck.push((b.id, pen, dmg));
                    }
                }
            }
        }
        for (id, pen, dmg) in struck {
            let Some(b) = t.boat_mut(id) else { continue };
            let Some(spec) = cfg.kinds.get(&b.kind) else { continue };
            let crewed = b.crewed();
            let hp = b.hp;
            let sank = b.hit(pen, dmg, spec);
            b.alarm(at, cfg.fight.flee_s);
            if b.hp < hp
                && let Some(h) = harm::report(b, cause, crewed, sank)
            {
                harmed.write(harmed_msg(b, &bases, &plan, h.sunk));
            }
            if sank {
                info!("BOAT {} ({}) sinking", b.id, b.kind);
                fx.write(
                    Fx::new("explosion_water", Vec3::new(b.pos.x, plan.waters.sea_level_m, b.pos.y)).scaled(spec.length_m().sqrt() * 0.3),
                );
            }
        }
    }
    // Every blast sets the boats near it running.
    for e in explosions.read() {
        let at = Vec2::new(e.at.x, e.at.z);
        for b in t.boats.iter_mut().filter(|b| b.pos.distance(at) < cfg.fight.flee_blast_m) {
            b.alarm(at, cfg.fight.flee_s);
        }
    }
    // A raider's gunboat destroyed goes down as a boat (its wreck and slick).
    for p in units.list.iter().filter(|p| p.unit.kind == GUNBOAT && !p.unit.alive()) {
        if wrecked.insert(p.unit.id) {
            let pos = Vec2::new(p.unit.pos.x, p.unit.pos.z);
            if let Some(id) = t.add(GUNBOAT, cfg, Errand::Anchor { at: pos }, pos, p.unit.heading_rad)
                && let Some(b) = t.boat_mut(id)
            {
                b.founder();
            }
        }
    }
}

/// Shipping jobs under way: their enemies left, an escort's freighter in or lost, and a finished job's boats let go.
fn keep_sea_jobs(
    mut boats: ResMut<Boats>,
    mut sea_jobs: ResMut<SeaJobs>,
    mut units: ResMut<Units>,
    mut career: Option<ResMut<crate::career::CareerRun>>,
) {
    let t = &mut boats.traffic;
    let mut done = Vec::new();
    for (job, sj) in sea_jobs.jobs.iter_mut() {
        let state = career.as_deref().and_then(|c| c.career.board.job(*job)).map(|j| j.state);
        if !matches!(state, Some(JobState::Accepted | JobState::Active)) {
            done.push(*job);
            continue;
        }
        let afloat = sj
            .gunboats
            .iter()
            .filter(|g| units.list.iter().any(|p| p.unit.id == **g && p.unit.alive() && !p.gone))
            .count() as u32;
        if let Some(w) = sj.ward {
            match t.boat(w) {
                Some(b) if matches!(b.phase, Phase::Moored { .. }) => sj.ward_in = true,
                Some(b) if b.afloat() => {}
                _ => {
                    // The freighter is lost: the job fails.
                    info!("JOB {job}: the freighter is lost");
                    if let Some(run) = career.as_deref_mut()
                        && let Some(e) = run.career.board.fail(*job)
                    {
                        run.pending.push(e);
                    }
                    sj.ward = None;
                }
            }
        }
        sj.left = afloat + u32::from(sj.ward.is_some() && !sj.ward_in);
    }
    for job in done {
        if let Some(sj) = sea_jobs.jobs.remove(&job) {
            for g in sj.gunboats {
                if let Some(p) = units.get_mut(g)
                    && p.unit.alive()
                {
                    p.gone = true;
                }
            }
            if let Some(b) = sj.ward.and_then(|w| t.boat_mut(w)) {
                b.ward = None;
            }
        }
    }
}

/// Draws each boat from its block-out model, riding the swell, listing and going down when sunk; spray at the bow
/// of a boat under way near the pilot.
#[allow(clippy::too_many_arguments)]
fn draw_boats(
    mut commands: Commands,
    boats: Res<Boats>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time>,
    pilot: Res<PilotSim>,
    me: Query<&MechSim, With<Piloted>>,
    assets: Res<AssetServer>,
    mut drawn: Query<(Entity, &BoatDraw, &mut Transform)>,
    mut fx: MessageWriter<Fx>,
    mut spray: Local<f32>,
) {
    let _t = crate::perf_hud::slow("draw_boats");
    let cfg = &data.0.ships;
    let look = &data.0.water_look;
    let seconds = time.elapsed_secs();
    let t = &boats.traffic;
    let mut seen = BTreeSet::new();
    for (e, d, mut tf) in &mut drawn {
        let Some(b) = t.boat(d.id) else {
            commands.entity(e).despawn();
            continue;
        };
        let Some(spec) = cfg.kinds.get(&b.kind) else { continue };
        seen.insert(d.id);
        *tf = pose(b, spec, &plan, look, seconds, cfg.fight.list_deg);
    }
    for b in t.boats.iter().filter(|b| !seen.contains(&b.id)) {
        let Some(spec) = cfg.kinds.get(&b.kind) else { continue };
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(spec.model.clone()));
        commands.spawn((
            Name::new(format!("boat {} {}", b.kind, b.id)),
            WorldAssetRoot(scene),
            pose(b, spec, &plan, look, seconds, cfg.fight.list_deg),
            BoatDraw { id: b.id },
        ));
    }
    // Spray at the bows of boats under way within 500 m, every 0.4 s.
    *spray += time.delta_secs();
    if *spray >= 0.4 {
        *spray = 0.0;
        let eye = pilot_at(&pilot, me.iter().next());
        for b in t
            .boats
            .iter()
            .filter(|b| b.afloat() && b.speed_m_s > 2.0 && b.pos.distance(eye) < 500.0)
        {
            let Some(spec) = cfg.kinds.get(&b.kind) else { continue };
            let bow = b.pos + cb_core::ships::ahead(b.heading_rad) * spec.length_m() * 0.45;
            let y = b.surface_m;
            fx.write(Fx::new("skate_spray", Vec3::new(bow.x, y, bow.y)).scaled((b.speed_m_s / 8.0).clamp(0.3, 1.5)));
        }
    }
}

/// A boat's transform: its waterline on the surface with the swell's ride, or listing down when sunk.
fn pose(
    b: &Boat,
    spec: &cb_core::ships::BoatSpec,
    plan: &Plan,
    look: &cb_core::water_look::WaterLook,
    seconds: f32,
    list_deg: f32,
) -> Transform {
    let surface = b.surface_m;
    let bed = plan.terrain.height_at(b.pos.x, b.pos.y);
    let ride = float::ride(look, b.pos, b.heading_rad, spec.length_m(), spec.beam_m(), seconds);
    let (y, pitch, roll) = b.pose(spec, surface, bed, ride, list_deg);
    Transform::from_xyz(b.pos.x, y, b.pos.y)
        .with_rotation(Quat::from_rotation_y(b.heading_rad) * Quat::from_rotation_x(pitch) * Quat::from_rotation_z(roll))
}

/// An oil slick where a boat went down: a dark disc on the water growing to one and a half lengths, fading out.
#[allow(clippy::too_many_arguments)]
fn draw_slicks(
    mut commands: Commands,
    boats: Res<Boats>,
    data: Res<Data>,
    mut slicks: Query<(Entity, &Slick, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut disc: Local<Option<Handle<Mesh>>>,
) {
    let cfg = &data.0.ships;
    let t = &boats.traffic;
    // Seconds since it was destroyed, for a sinking or sunk boat.
    let since = |b: &Boat| match b.phase {
        Phase::Sinking { t_s } => Some(t_s),
        Phase::Sunk { t_s } => cfg.kinds.get(&b.kind).map(|k| k.sink_s + t_s),
        _ => None,
    };
    let mut have = BTreeSet::new();
    for (e, s, mut tf) in &mut slicks {
        let Some((b, age)) = t.boat(s.id).and_then(|b| since(b).map(|a| (b, a))) else {
            commands.entity(e).despawn();
            continue;
        };
        have.insert(s.id);
        let len = cfg.kinds.get(&b.kind).map_or(10.0, |k| k.length_m());
        let grow = (age / 60.0).min(1.0);
        tf.scale = Vec3::new(len * 0.75 * grow.max(0.05), 1.0, len * 0.75 * grow.max(0.05));
        let fade = (1.0 - age / cfg.fight.slick_s).clamp(0.0, 1.0);
        if let Some(mut m) = materials.get_mut(&s.material) {
            m.base_color.set_alpha(0.75 * fade);
        }
    }
    for b in t.boats.iter().filter(|b| !have.contains(&b.id) && since(b).is_some()) {
        let mesh = disc
            .get_or_insert_with(|| meshes.add(Circle::new(1.0).mesh().resolution(32)))
            .clone();
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.02, 0.02, 0.025, 0.75),
            perceptual_roughness: 0.08,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let y = b.surface_m + 0.06;
        commands.spawn((
            Name::new(format!("oil slick {}", b.id)),
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(b.pos.x, y, b.pos.y)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(0.1)),
            Slick { id: b.id, material },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harbour() -> Harbour {
        let side = Vec2::new(0.0, 1.0);
        let f = |c: Vec2| Footprint {
            centre: c,
            along: side,
            half: Vec2::new(10.0, 5.0),
        };
        Harbour {
            id: 0,
            city: 0,
            out: Vec2::new(1.0, 0.0),
            shore: Vec2::new(100.0, 50.0),
            berth: Vec2::new(300.0, 50.0),
            mouth: Vec2::new(1500.0, 50.0),
            anchorage: Vec2::new(1200.0, 50.0),
            deck_y: 2.5,
            apron: f(Vec2::new(85.0, 50.0)),
            pier: f(Vec2::new(200.0, 50.0)),
            head: f(Vec2::new(285.0, 50.0)),
            cranes: vec![],
            warehouses: vec![],
            office: f(Vec2::new(80.0, 0.0)),
            terminal: Vec2::new(90.0, 0.0),
            port: false,
        }
    }

    #[test]
    fn a_harbours_frame_puts_the_sea_at_local_minus_z_and_agrees_with_local() {
        let h = harbour();
        assert!((h.side() - Vec2::new(0.0, 1.0)).length() < 1e-6, "{:?}", h.side());
        let frame = harbour_frame(&h);
        for p in [h.berth, h.terminal, h.mouth] {
            let l = local(&h, p);
            let w = frame.transform_point3(Vec3::new(l.x, 0.0, l.y));
            assert!((Vec2::new(w.x, w.z) - p).length() < 1e-3, "{p:?} went to {w:?}");
        }
        assert!(local(&h, h.berth).y < 0.0, "out to sea is local -z");
    }

    #[test]
    fn the_quay_is_a_fixed_platform_the_pilot_stands_on() {
        let h = harbour();
        let p = Platform {
            slabs: quay_slabs(&h),
            pose: harbour_frame(&h),
            was: harbour_frame(&h),
        };
        assert!(p.is_fixed());
        let on_pier = h.pier.centre;
        assert_eq!(p.top_under(on_pier.x, on_pier.y, h.deck_y + 0.2), Some(h.deck_y));
        assert_eq!(p.top_under(h.mouth.x, h.mouth.y, h.deck_y), None, "nothing out at sea");
    }

    /// Every system this change adds or widens is built against a world: Bevy refuses one whose queries clash (B0001)
    /// when it is first set up, which a test catches without running the game.
    #[test]
    fn the_boats_systems_have_no_clashing_queries() {
        fn init<M>(world: &mut World, sys: impl IntoSystem<(), (), M>) {
            let mut s = IntoSystem::into_system(sys);
            s.initialize(world);
        }
        let mut world = World::new();
        init(&mut world, spawn_harbours);
        init(&mut world, start_sea_jobs);
        init(&mut world, aim_raiders);
        init(&mut world, crabs_hunt);
        init(&mut world, step_boats);
        init(&mut world, hit_boats);
        init(&mut world, keep_sea_jobs);
        init(&mut world, draw_boats);
        init(&mut world, draw_slicks);
        init(&mut world, crate::combat::fly_shots);
    }

    #[test]
    fn sea_jobs_count_a_started_job_as_unfinished_until_its_boats_are_in() {
        let mut s = SeaJobs::default();
        s.pending.insert(4);
        assert_eq!(s.left(4), 1);
        s.pending.clear();
        s.jobs.insert(4, SeaJob { left: 3, ..default() });
        assert_eq!(s.left(4), 3);
        assert_eq!(s.left(5), 0);
    }
}
