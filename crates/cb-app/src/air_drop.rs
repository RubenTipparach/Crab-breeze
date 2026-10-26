//! The air drop in the game (openspec change `mech-shop`, design section 6; owner, 2026-10-26: "when a player orders a
//! mech it should be air dropped close to wherever the player is on a good are on the ground (not stuck on a
//! building)"; survey J16: "a marker appears on your hud to show mech delivery. you can swap mechs out at base, with
//! foreman always availible"). The rules are `cb_core::air_drop` and the career's books; this adapts them:
//!
//! - where the player is and what stands near them, for the shop's search (`Where`);
//! - a drop's mech brought into the world at the release and held to its fall each fixed step, steered clear of a
//!   pilot in the site, then handed to its own simulation over the ground so it lands through the mech's own landing
//!   (its footprints and dust are `ground-marks`'), with `jet_wash` under its retros and `fall_dust` at touchdown;
//! - the cargo aircraft (civil air traffic's freighter, `aircraft_kind`) and the canopies, both block-out stand-ins
//!   (named so in their scene nodes);
//! - the HUD marker over every drop, from the order until the pilot boards the mech, on foot and in the seat;
//! - which owned mech the pilot on foot is walking up to: that one carries `Piloted`, so it is the one that kneels and
//!   lowers its rope (`mech-boarding`, `rope`: the existing boarding, nothing cut);
//! - every owned mech in the field put where the career says, when a career is continued and when one is called out
//!   at a base, and the career kept told where each stands, so a save keeps them there.

use crate::career::CareerRun;
use crate::fx::Fx;
use crate::mech::{MechId, MechSim, MechSpec, Owned, Piloted, SpawnAssets, owned_id, spawn_owned};
use crate::pilot::{PilotCamera, PilotSim};
use crate::sim::{Data, SimSet};
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::air_drop::{self as rules, FallStage, Keep, Needs, PlanGround, Stage};
use cb_core::boarding::HatchTarget;
use cb_core::career::Field;
use cb_core::mech::MechConfig;
use cb_core::pilot::PilotMode;
use std::sync::Arc;

/// Where the player is and what stands near them, for the shop's search and the call-out (written every frame).
#[derive(Resource, Default, Clone, Debug)]
pub struct Where {
    /// The pilot's feet on foot, their mech's feet in the seat (x, z).
    pub pilot: Vec2,
    /// What a drop keeps clear of round the pilot: half a metre on foot, their mech's footprint in the seat.
    pub body_radius_m: f32,
    /// Every mech standing (feet x, z, and footprint radius), and every drop's site with its mech's footprint.
    pub mechs: Vec<(Vec2, f32)>,
    /// Owned mechs that are wrecks, by key.
    pub wrecks: Vec<u32>,
}

/// An owned mech still falling from its drop (its key): held to the fall each fixed step until it touches down.
#[derive(Component)]
pub struct Dropping(pub u32);

/// The cargo aircraft of a drop (its mech's key). A block-out stand-in.
#[derive(Component)]
struct Aircraft(u32);

/// The canopies over a falling mech (its key). Block-out stand-ins.
#[derive(Component)]
struct Canopies(u32);

/// A drop's marker on the HUD (its mech's key).
#[derive(Component)]
struct DropMarker(u32);

/// The HUD layer the drop markers hang on.
#[derive(Component)]
struct MarkerRoot;

/// The block-outs' meshes and materials, made once.
#[derive(Resource)]
struct DropAssets {
    cube: Handle<Mesh>,
    dome: Handle<Mesh>,
    canopy: Handle<StandardMaterial>,
    line: Handle<StandardMaterial>,
}

/// The marker's colours: amber as the HUD's job marker, red for the warning.
const AMBER: Color = Color::srgb(1.0, 0.72, 0.2);
const WARN: Color = Color::srgb(1.0, 0.3, 0.2);

/// The pilot on foot is this wide to a drop, metres.
const PILOT_RADIUS_M: f32 = 0.5;

/// A falling mech is handed to its own simulation this high over the ground, metres: it lands on its next step.
const HANDOFF_M: f32 = 0.5;

/// The canopies' radius and how high their middle rides over the mech's feet, past its height, metres.
const CANOPY_R_M: f32 = 14.0;
const CANOPY_OVER_M: f32 = 22.0;

/// Runs the air drop, the HUD marker and the walk up to an owned mech, in a career.
pub struct AirDropPlugin;

impl Plugin for AirDropPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Where>()
            .add_systems(Startup, (make_assets, spawn_marker_root))
            .add_systems(
                Update,
                (track_where, place_owned, walk_up, draw_aircraft, draw_canopies, draw_markers)
                    .chain()
                    .run_if(resource_exists::<CareerRun>),
            )
            .add_systems(
                FixedUpdate,
                (step_drops, keep_places)
                    .chain()
                    .in_set(SimSet::Mech)
                    .after(crate::mech::step_mech)
                    .run_if(resource_exists::<CareerRun>),
            );
    }
}

fn make_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut flat = |c: Color, rough: f32| {
        materials.add(StandardMaterial {
            base_color: c,
            perceptual_roughness: rough,
            ..default()
        })
    };
    commands.insert_resource(DropAssets {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        dome: meshes.add(Sphere::new(1.0)),
        canopy: flat(Color::srgb(0.92, 0.55, 0.18), 0.9),
        line: flat(Color::srgb(0.85, 0.85, 0.8), 0.9),
    });
}

fn spawn_marker_root(mut commands: Commands) {
    commands.spawn((
        Name::new("air drop markers"),
        MarkerRoot,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.0),
            height: percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

/// The design's data of owned mech `key`.
fn design_of(run: &CareerRun, key: u32) -> Option<&MechConfig> {
    run.career.mech(key).and_then(|m| run.data.design(&m.design))
}

/// Where the player is and what stands near them.
fn track_where(
    pilot: Res<PilotSim>,
    run: Res<CareerRun>,
    mechs: Query<(&MechSim, &MechSpec, Option<&Owned>, Has<Piloted>)>,
    mut out: ResMut<Where>,
) {
    let seated = pilot.curr.mode == PilotMode::Seated;
    let mine = mechs.iter().find(|m| m.3);
    (out.pilot, out.body_radius_m) = match (seated, mine) {
        (true, Some((sim, spec, ..))) => (Vec2::new(sim.curr.pos.x, sim.curr.pos.z), spec.locomotion.body_radius_m),
        _ => (Vec2::new(pilot.curr.feet.x, pilot.curr.feet.z), PILOT_RADIUS_M),
    };
    out.mechs.clear();
    out.wrecks.clear();
    for (sim, spec, owned, _) in &mechs {
        if sim.dead {
            if let Some(o) = owned {
                out.wrecks.push(o.0);
            }
            continue;
        }
        out.mechs
            .push((Vec2::new(sim.curr.pos.x, sim.curr.pos.z), spec.locomotion.body_radius_m));
    }
    for d in &run.career.drops {
        let r = design_of(&run, d.key).map_or(8.0, |c| c.locomotion.body_radius_m);
        out.mechs.push((d.site_xz(), r));
    }
}

/// Spawns owned mech `key` standing at its place in the field, with the damage the career keeps for it.
fn spawn_at_field(commands: &mut Commands, run: &CareerRun, key: u32, field: Field, a: &mut SpawnAssets) -> Option<Entity> {
    let owned = run.career.mech(key)?.clone();
    let cfg = run.data.design(&owned.design)?.clone();
    let parts = cfg.parts.len();
    let ent = spawn_owned(commands, Arc::new(cfg), Vec3::from_array(field.pos), field.heading_rad, key, a);
    if owned.structure.len() == parts && owned.lost.len() == parts {
        commands.entity(ent).queue(move |mut e: EntityWorldMut| {
            if let Some(mut sim) = e.get_mut::<MechSim>() {
                sim.damage.structure = owned.structure.clone();
                sim.damage.lost = owned.lost.clone();
                let lost: Vec<usize> = owned.lost.iter().enumerate().filter(|(_, l)| **l).map(|(i, _)| i).collect();
                sim.cuts.extend(lost);
            }
        });
    }
    Some(ent)
}

/// Matches the world to the career once (the piloted mech is the design the career pilots; every other owned mech
/// in the field stands where it was left), then puts each mech a rule moved (`CareerRun::moved`: a call-out) where
/// the career now says: moved if it stands in the world, built again whole if it was a wreck, spawned if it was not
/// in the world at all.
#[allow(clippy::too_many_arguments)]
fn place_owned(
    mut commands: Commands,
    mut run: ResMut<CareerRun>,
    data: Res<Data>,
    piloted: Query<(Entity, &MechSim), With<Piloted>>,
    mut owned: Query<(Entity, &Owned, &mut MechSim), Without<Piloted>>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Read first: the menu redraws on any change to the career, so it is touched only with work to do.
    if run.placed && run.moved.is_empty() {
        return;
    }
    let mut sa = SpawnAssets {
        tex: &tex,
        assets: &assets,
        meshes: &mut meshes,
        materials: &mut materials,
    };
    let run = &mut *run;
    if !run.placed {
        let Some((entity, sim)) = piloted.iter().next() else { return };
        run.placed = true;
        let Some(key) = run.career.piloting else { return };
        // The game starts every run in its startup mech (`--mech`, the Foreman); a career last piloting another of its
        // mechs gets that one, standing where the startup mech stands (the save's place is put back on it after).
        let want = design_of(run, key).cloned();
        match want {
            Some(cfg) if cfg.model != data.0.mech.model => {
                info!("career: piloting the {}, not the startup {}", cfg.name, data.0.mech.name);
                let (pos, heading) = (sim.curr.pos, sim.curr.heading_rad);
                commands.entity(entity).despawn();
                let ent = spawn_owned(&mut commands, Arc::new(cfg), pos, heading, key, &mut sa);
                commands.entity(ent).insert((Piloted, MechId(0)));
                run.refit_dirty = true;
            }
            _ => {
                commands.entity(entity).insert(Owned(key));
            }
        }
        let falling: Vec<u32> = run.career.drops.iter().filter(|d| !d.landed).map(|d| d.key).collect();
        let field: Vec<u32> = run
            .career
            .mechs
            .iter()
            .filter(|m| m.field.is_some() && Some(m.key) != run.career.piloting && !falling.contains(&m.key))
            .map(|m| m.key)
            .collect();
        info!("career: {} owned mechs stand in the field", field.len());
        run.moved.extend(field);
    }
    for key in std::mem::take(&mut run.moved) {
        let Some(field) = run.career.mech(key).and_then(|m| m.field) else {
            continue;
        };
        match owned.iter_mut().find(|(_, o, _)| o.0 == key) {
            Some((_, _, mut sim)) if !sim.dead => {
                sim.curr.pos = Vec3::from_array(field.pos);
                sim.curr.heading_rad = field.heading_rad;
                sim.curr.vel = Vec2::ZERO;
                sim.curr.speed_m_s = 0.0;
                sim.curr.vel_y_m_s = 0.0;
                sim.prev = sim.curr;
                sim.hatch = Default::default();
                sim.hatch_prev = sim.hatch;
            }
            Some((entity, ..)) => {
                commands.entity(entity).despawn();
                spawn_at_field(&mut commands, run, key, field, &mut sa);
            }
            None => {
                spawn_at_field(&mut commands, run, key, field, &mut sa);
            }
        }
    }
}

/// Which owned mech the pilot on foot is walking up to carries `Piloted` (design section 6.3), so it is the one that
/// turns to them, kneels and lowers its rope. Handed over only while the pilot is on foot and not waiting on a hatch,
/// and the mech they were last in has finished closing up. The career's piloted mech follows, and the refit rebuilds
/// it from its design and fit. Once the pilot is seated in a dropped mech, its drop is done and its marker goes.
#[allow(clippy::type_complexity)]
fn walk_up(
    mut commands: Commands,
    mut run: ResMut<CareerRun>,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mechs: Query<(Entity, &Owned, &MechSim, &MechSpec, Has<Piloted>), Without<Dropping>>,
) {
    let p = &pilot.curr;
    if p.mode == PilotMode::Seated
        && let Some(key) = run.career.piloting
        && run.career.drop_of(key).is_some()
    {
        let run = &mut *run;
        run.career.boarded(key);
        info!("AIR DROP boarded: mech {key}");
        crate::career::save(&run.career);
        return;
    }
    if p.mode != PilotMode::OnFoot || p.waiting.is_some() {
        return;
    }
    let Some((cur_e, cur, cur_sim, cur_spec, _)) = mechs.iter().find(|m| m.4) else {
        return;
    };
    let tl = &data.0.boarding;
    if cur_sim.hatch.target == HatchTarget::Open || cur_sim.hatch.t_s != cur_sim.hatch.target_s(tl) {
        return;
    }
    let near: Vec<(u32, Vec2)> = mechs
        .iter()
        .filter(|m| !m.2.dead)
        .map(|m| (m.1.0, Vec2::new(m.2.curr.pos.x, m.2.curr.pos.z)))
        .collect();
    let feet = Vec2::new(p.feet.x, p.feet.z);
    let Some(key) = rules::walking_up_to(&run.data.drop, feet, &near) else {
        return;
    };
    if key == cur.0 {
        return;
    }
    let run = &mut *run;
    let Some((new_e, ..)) = mechs.iter().find(|m| m.1.0 == key) else {
        return;
    };
    // The mech left behind keeps its place and its damage in the books, as a share of each part (a refit's armour
    // grade changes the flown structure, not the design's).
    let whole = run.career.mech(cur.0).and_then(|m| run.data.whole(&m.design));
    if let Some(m) = run.career.mech_mut(cur.0) {
        m.field = Some(Field {
            pos: cur_sim.curr.pos.to_array(),
            heading_rad: cur_sim.curr.heading_rad,
        });
        if let Some(whole) = whole.filter(|w| w.len() == cur_spec.parts.len() && w.len() == cur_sim.damage.structure.len()) {
            m.structure = (0..whole.len())
                .map(|i| cur_sim.damage.structure[i] / cur_spec.parts[i].structure.max(1e-3) * whole[i])
                .collect();
            m.lost = cur_sim.damage.lost.clone();
        }
    }
    commands.entity(cur_e).remove::<Piloted>().insert(owned_id(cur.0));
    commands.entity(new_e).insert((Piloted, MechId(0)));
    run.career.piloting = Some(key);
    run.refit_dirty = true;
    info!("BOARD walking up to mech {key} (was {})", cur.0);
}

/// Steps every drop on the career's clock: before the release the site keeps clear of the pilot; from the release the
/// mech is in the world, held to its fall over the site, steered aside if the pilot walks in, with the retros' wash
/// on the ground; at touchdown it is handed to its own simulation a hand's breadth over the ground, lands on its feet,
/// and stands in the field.
#[allow(clippy::too_many_arguments)]
fn step_drops(
    mut commands: Commands,
    mut run: ResMut<CareerRun>,
    plan: Res<Plan>,
    data: Res<Data>,
    near: Res<Where>,
    time: Res<Time<Fixed>>,
    mut mechs: Query<(Entity, &Owned, &mut MechSim), With<Dropping>>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut fx: MessageWriter<Fx>,
) {
    let run = &mut *run;
    let dt = time.delta_secs();
    let cfg = run.data.drop.clone();
    let now = run.career.time_s;
    let bases = &data.0.world.bases;
    let ground = PlanGround { plan: &plan, bases };
    let mut sa = SpawnAssets {
        tex: &tex,
        assets: &assets,
        meshes: &mut meshes,
        materials: &mut materials,
    };
    let mut landed_any = false;
    for d in run.career.drops.clone() {
        if d.landed {
            continue;
        }
        let Some(design) = design_of(run, d.key).cloned() else { continue };
        let needs = Needs::of(&design);
        let good = |p: Vec2| {
            let keep = Keep {
                pilot: near.pilot,
                mechs: &[],
            };
            rules::good_spot(&cfg, &ground, needs, &keep, p).is_some()
        };
        let site = d.site_xz();
        let clear = rules::clear_of(&cfg, needs, site, near.pilot, near.body_radius_m, &good);
        let stage = d.stage(&cfg, now);
        // Before the release the site simply moves; once falling, the canopies steer it there; at touchdown it lands
        // beside the pilot at once if they are still in the way.
        let next = match stage {
            Stage::Inbound(_) | Stage::Landed => clear,
            Stage::Falling(_) => rules::steer(site, clear, cfg.steer_m_s * dt),
        };
        let ground_m = plan.terrain.height_at(next.x, next.y);
        if next != site {
            if let Some(dd) = run.career.drops.iter_mut().find(|x| x.key == d.key) {
                dd.site = [next.x, ground_m, next.y];
            }
            if matches!(stage, Stage::Inbound(_)) {
                info!("AIR DROP site moved clear of the pilot to ({:.0}, {:.0})", next.x, next.y);
            }
        }
        let Stage::Inbound(_) = stage else {
            // In the world from the release on.
            let entity = mechs.iter_mut().find(|m| m.1.0 == d.key);
            let Some((entity, _, mut sim)) = entity else {
                let fall = rules::fall(&cfg, d.since_release_s(&cfg, now));
                let at = Vec3::new(next.x, ground_m + fall.height_m.max(HANDOFF_M), next.y);
                let ent = spawn_owned(&mut commands, Arc::new(design.clone()), at, d.track_rad, d.key, &mut sa);
                commands.entity(ent).insert(Dropping(d.key));
                info!("AIR DROP released: mech {} at {:.0} m", d.key, fall.height_m);
                continue;
            };
            match stage {
                Stage::Falling(f) => {
                    sim.curr.pos = Vec3::new(next.x, ground_m + f.height_m.max(HANDOFF_M), next.y);
                    sim.curr.vel = Vec2::ZERO;
                    sim.curr.speed_m_s = 0.0;
                    sim.curr.vel_y_m_s = -f.speed_m_s;
                    sim.curr.in_air = true;
                    sim.curr.heading_rad = d.track_rad;
                    if f.stage == FallStage::Retros {
                        let t = d.since_release_s(&cfg, now);
                        if (t * 5.0).floor() != ((t - dt) * 5.0).floor() {
                            fx.write(Fx::new("jet_wash", Vec3::new(next.x, ground_m, next.y)));
                        }
                    }
                }
                _ => {
                    // Touchdown: its own simulation lands it on its next step (the landing's prints and dust).
                    sim.curr.pos = Vec3::new(next.x, ground_m + HANDOFF_M, next.y);
                    sim.curr.vel = Vec2::ZERO;
                    sim.curr.vel_y_m_s = -cfg.touchdown_m_s;
                    sim.curr.in_air = true;
                    commands.entity(entity).remove::<Dropping>();
                    fx.write(Fx::new("fall_dust", Vec3::new(next.x, ground_m, next.y)));
                    run.career.landed(
                        d.key,
                        Field {
                            pos: [next.x, ground_m, next.y],
                            heading_rad: d.track_rad,
                        },
                    );
                    info!("AIR DROP landed: mech {} at ({:.0}, {:.0})", d.key, next.x, next.y);
                    landed_any = true;
                }
            }
            continue;
        };
    }
    if landed_any {
        crate::career::save(&run.career);
    }
}

/// Keeps the career told where every owned mech in the world stands, so a save keeps it there.
fn keep_places(mut run: ResMut<CareerRun>, mechs: Query<(&Owned, &MechSim), Without<Dropping>>) {
    for (o, sim) in &mechs {
        if run.career.drop_of(o.0).is_some_and(|d| !d.landed) {
            continue;
        }
        let field = Field {
            pos: sim.curr.pos.to_array(),
            heading_rad: sim.curr.heading_rad,
        };
        // Read first, written only when it moved (the menu redraws on any change to the career).
        if run.career.mech(o.0).is_some_and(|m| m.field != Some(field))
            && let Some(m) = run.career.mech_mut(o.0)
        {
            m.field = Some(field);
        }
    }
}

/// The cargo aircraft: civil air traffic's freighter (`aircraft_kind`, civil-air-traffic's stand-in from its shape
/// table, the same one the traffic flies), flown along its drop's track on the drop's own clock, its engine heard
/// as the traffic's jets are.
#[allow(clippy::too_many_arguments)]
fn draw_aircraft(
    mut commands: Commands,
    run: Res<CareerRun>,
    data: Res<Data>,
    ear: Res<crate::audio::Ear>,
    mut looks: ResMut<crate::air_traffic::KindLooks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut planes: Query<(Entity, &Aircraft, &mut Transform)>,
    mut loops: MessageWriter<crate::audio::Loop>,
) {
    let cfg = &run.data.drop;
    let now = run.career.time_s;
    let Some(spec) = data.0.air_traffic.kinds.get(&cfg.aircraft_kind) else {
        return;
    };
    for (e, plane, mut t) in &mut planes {
        match run
            .career
            .drop_of(plane.0)
            .and_then(|d| d.aircraft(cfg, now).map(|p| (p, d.track_rad)))
        {
            Some((p, track)) => {
                *t = Transform::from_translation(p).with_rotation(Quat::from_rotation_y(track));
                if let Some(pitch) = spec.engine_pitch
                    && p.distance(ear.at) < data.0.audio.cues.audible_within_m
                {
                    loops.write(crate::audio::Loop {
                        cue: crate::air_traffic::ENGINE_CUE,
                        at: crate::audio::SoundAt::Node(e),
                        level: cb_core::audio::LoopLevel { db: 0.0, pitch },
                    });
                }
            }
            None => commands.entity(e).despawn(),
        }
    }
    for d in &run.career.drops {
        if planes.iter().any(|p| p.1.0 == d.key) {
            continue;
        }
        let Some(p) = d.aircraft(cfg, now) else { continue };
        let root = crate::air_traffic::spawn_standin(
            &mut commands,
            &mut looks,
            &mut meshes,
            &mut materials,
            &cfg.aircraft_kind,
            spec,
            format!("cargo aircraft: {} (block-out stand-in)", cfg.aircraft_kind),
            Transform::from_translation(p).with_rotation(Quat::from_rotation_y(d.track_rad)),
        );
        commands.entity(root).insert(Aircraft(d.key));
    }
}

/// The canopies over a falling mech: three domes on rigging lines, opening over the first second after the free fall,
/// gone at touchdown. Block-out stand-ins.
#[allow(clippy::type_complexity)]
fn draw_canopies(
    mut commands: Commands,
    run: Res<CareerRun>,
    a: Res<DropAssets>,
    falling: Query<(&Dropping, &GlobalTransform, &MechSpec)>,
    mut rigs: Query<(Entity, &Canopies, &mut Transform)>,
) {
    let cfg = &run.data.drop;
    let now = run.career.time_s;
    let under = |key: u32| -> Option<f32> {
        let d = run.career.drop_of(key)?;
        let t = d.since_release_s(cfg, now);
        match d.stage(cfg, now) {
            Stage::Falling(f) if f.stage != FallStage::FreeFall => Some(((t - cfg.free_fall_s) / 1.0).clamp(0.15, 1.0)),
            _ => None,
        }
    };
    for (e, rig, mut t) in &mut rigs {
        match falling.iter().find(|f| f.0.0 == rig.0).zip(under(rig.0)) {
            Some(((_, g, _), open)) => {
                *t = Transform::from_translation(g.translation()).with_scale(Vec3::new(open, 1.0, open));
            }
            None => commands.entity(e).despawn(),
        }
    }
    for (d, g, spec) in &falling {
        if rigs.iter().any(|r| r.1.0 == d.0) || under(d.0).is_none() {
            continue;
        }
        let h = spec.locomotion.body_height_m;
        let top = h + CANOPY_OVER_M;
        commands
            .spawn((
                Name::new("drop canopies (block-out stand-in)"),
                Canopies(d.0),
                Transform::from_translation(g.translation()).with_scale(Vec3::new(0.15, 1.0, 0.15)),
                Visibility::default(),
            ))
            .with_children(|c| {
                for off in [Vec3::new(-15.0, 0.0, 6.0), Vec3::new(15.0, 0.0, 6.0), Vec3::new(0.0, 2.0, -14.0)] {
                    let centre = Vec3::new(off.x, top + off.y, off.z);
                    c.spawn((
                        Mesh3d(a.dome.clone()),
                        MeshMaterial3d(a.canopy.clone()),
                        Transform::from_translation(centre).with_scale(Vec3::new(CANOPY_R_M, 4.0, CANOPY_R_M)),
                    ));
                    let from = Vec3::new(0.0, h, 0.0);
                    let to = centre - Vec3::Y * 3.5;
                    let along = to - from;
                    c.spawn((
                        Mesh3d(a.cube.clone()),
                        MeshMaterial3d(a.line.clone()),
                        Transform::from_translation((from + to) * 0.5)
                            .with_rotation(Quat::from_rotation_arc(Vec3::Y, along.normalize_or(Vec3::Y)))
                            .with_scale(Vec3::new(0.2, along.length(), 0.2)),
                    ));
                }
            });
    }
}

/// A drop's marker on the HUD (owner, survey J16: "a marker appears on your hud to show mech delivery"): over its site
/// from the order until the pilot boards the mech, on foot and in the seat, with its distance and stage; and a warning
/// while the pilot stands in the drop zone.
#[allow(clippy::type_complexity)]
fn draw_markers(
    mut commands: Commands,
    run: Res<CareerRun>,
    near: Res<Where>,
    pilot: Res<PilotSim>,
    root: Query<Entity, With<MarkerRoot>>,
    camera: Query<(&Camera, &GlobalTransform), With<PilotCamera>>,
    mut markers: Query<(Entity, &DropMarker, &mut Node, &mut Text, &mut TextColor, &mut Visibility)>,
) {
    let cfg = &run.data.drop;
    let now = run.career.time_s;
    for (e, m, ..) in &markers {
        if run.career.drop_of(m.0).is_none() {
            commands.entity(e).despawn();
        }
    }
    let Some(root) = root.iter().next() else { return };
    for d in &run.career.drops {
        if !markers.iter().any(|m| m.1.0 == d.key) {
            commands.entity(root).with_children(|r| {
                r.spawn((
                    DropMarker(d.key),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    (Text::new(""), TextFont::from_font_size(22.0), TextColor(AMBER)),
                    Visibility::Hidden,
                ));
            });
        }
    }
    let Some((cam, at)) = camera.iter().next() else { return };
    let eye = at.translation();
    for (_, m, mut node, mut text, mut colour, mut vis) in &mut markers {
        let Some(d) = run.career.drop_of(m.0) else { continue };
        let site = Vec3::from_array(d.site);
        let dist = Vec2::new(eye.x, eye.z).distance(d.site_xz());
        let name = design_of(&run, d.key).map_or_else(|| "MECH".to_string(), |c| c.name.to_uppercase());
        let needs = design_of(&run, d.key).map_or(8.0, |c| c.locomotion.body_radius_m);
        let in_zone = d.site_xz().distance(near.pilot) < needs + cfg.pilot_clear_m + near.body_radius_m && !d.landed;
        let line = match d.stage(cfg, now) {
            Stage::Inbound(s) => format!("INBOUND {s:.0} S"),
            Stage::Falling(f) => format!("FALLING {:.0} M", f.height_m),
            Stage::Landed => {
                if pilot.curr.mode == PilotMode::OnFoot {
                    "WALK UP TO BOARD".to_string()
                } else {
                    "LANDED".to_string()
                }
            }
        };
        let warn = if in_zone { "\nDROP ZONE: MOVE CLEAR" } else { "" };
        match cam.world_to_viewport(at, site + Vec3::Y * 30.0) {
            Ok(p) => {
                node.left = px(p.x - 60.0);
                node.top = px(p.y - 14.0);
                text.0 = format!("<> AIR DROP {name} {dist:.0} M\n{line}{warn}");
                colour.0 = if in_zone { WARN } else { AMBER };
                *vis = Visibility::Inherited;
            }
            Err(_) => *vis = Visibility::Hidden,
        }
    }
}
