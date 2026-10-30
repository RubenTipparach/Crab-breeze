//! Civil air traffic in the game (openspec change `civil-air-traffic`): every fixed step the aircraft
//! near the pilot fly through `cb_core::air_traffic`, keeping clear of every live enemy; a crash is an
//! `Explosion` and a wreck laid down as debris chunks (`debris-physics`); the pilot's own hits are
//! tallied per base and sent as `CivilAirHit` for the career loop. A kind with a model (the freighter, survey D8) is
//! drawn from its glb, its three levels of detail switched by distance and its gear shown only near the ground; the
//! others as labelled block-out stand-ins at true size (`cb_core::air_traffic::shape`). Each plays its engine loop.
//! `Neutrals` lists them for targeting and the cockpit, which show them as neutral. The aircraft the airport's page
//! parks at its stands stand there in the same models (`park_aircraft`).

use crate::audio::{Ear, Loop, SoundAt};
use crate::blast::Explosion;
use crate::fx::Fx;
use crate::mech::{MechId, MechSim, Piloted};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::units::{Hostile, Hostiles};
use crate::world::{Live, Plan};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use cb_core::air_traffic::shape::{self, Spin};
use cb_core::air_traffic::{AircraftSpec, Happened, Island, Standing, Traffic, Watcher, Zone, nearest_base, rest_m};
use cb_core::audio::LoopLevel;
use cb_core::pilot::PilotMode;
use cb_core::world::Ground;
use std::collections::{BTreeMap, HashMap};

/// Ids from here up are civil aircraft (below the fortress's, above every vehicle the invasion sends).
pub const CIVIL_ID_BASE: u32 = 800_000;

/// The looping cues the civil aircraft's engines play (survey W49: cut from public domain recordings).
pub const ENGINE_CUES: [&str; 3] = ["civil_turbofan", "civil_propeller", "civil_rotor"];

/// The looping cue for a kind's engine.
pub fn engine_cue(e: cb_core::air_traffic::Engine) -> &'static str {
    use cb_core::air_traffic::Engine;
    match e {
        Engine::Turbofan => ENGINE_CUES[0],
        Engine::Propeller => ENGINE_CUES[1],
        Engine::Rotor => ENGINE_CUES[2],
    }
}

/// True for an id in the civil aircraft's range.
pub fn is_civil(id: u32) -> bool {
    (CIVIL_ID_BASE..crate::fortress::FORTRESS_ID).contains(&id)
}

/// The civil aircraft near the pilot.
#[derive(Resource)]
pub struct AirTraffic(pub Traffic);

/// Every civil aircraft in the air this step, in id order, shaped like the hostiles list so the
/// cockpit and targeting draw both alike; never in `Hostiles` (the music, the scripted pilot and the
/// defence count only enemies).
#[derive(Resource, Default)]
pub struct Neutrals(pub Vec<Hostile>);

/// The pilot's hits on civil aircraft per base (survey W34), counted for the log. `CivilAirHit` carries each one
/// as it happens, and `civil_standing` charges it to the career's standing.
#[derive(Resource, Default)]
pub struct CivilStanding(pub Standing);

/// A shot or blow landed on a civil aircraft.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CivilAirHit {
    /// The aircraft's id (`CIVIL_ID_BASE` and up).
    pub aircraft: u32,
    /// Where it was hit.
    pub at: Vec3,
    /// The air base nearest the hit: whose standing it costs.
    pub base: Option<u32>,
    /// This hit brought it down.
    pub downed: bool,
    /// The pilot fired it (otherwise a crab did).
    pub by_pilot: bool,
}

/// What `combat` needs to hit a civil aircraft.
#[derive(SystemParam)]
pub struct CivilHits<'w, 's> {
    air: Option<ResMut<'w, AirTraffic>>,
    plan: Res<'w, Plan>,
    out: MessageWriter<'w, CivilAirHit>,
    broke: MessageWriter<'w, crate::airliners::AirlinerBroke>,
    pilot: Query<'w, 's, &'static MechId, With<Piloted>>,
}

impl CivilHits<'_, '_> {
    /// Every flying civil aircraft's hit capsules, under its civil id.
    pub fn capsules(&self, data: &Data) -> Vec<cb_core::weapons::PartCapsule> {
        let Some(air) = &self.air else { return Vec::new() };
        air.0
            .list
            .iter()
            .filter(|a| !a.downed())
            .filter_map(|a| Some((a, data.0.air_traffic.kinds.get(&a.kind)?)))
            .flat_map(|(a, spec)| {
                shape::capsules(a, spec).into_iter().map(|c| cb_core::weapons::PartCapsule {
                    mech: CIVIL_ID_BASE + a.id,
                    part: c.part,
                    a: c.a,
                    b: c.b,
                    radius_m: c.radius_m,
                })
            })
            .collect()
    }

    /// Where every flying civil aircraft is, by civil id, for a missile locked on one on purpose.
    pub fn centres(&self) -> Vec<(u32, Vec3)> {
        let Some(air) = &self.air else { return Vec::new() };
        air.0
            .list
            .iter()
            .filter(|a| !a.downed())
            .map(|a| (CIVIL_ID_BASE + a.id, a.pos))
            .collect()
    }

    /// A shot from `owner` struck part `part` of civil aircraft `id` at `at` (a modelled airliner's damage zone, else
    /// the aircraft as a whole): the armour rule, the tally and the message, and a zone shot away
    /// (`airliners::AirlinerBroke`). True when it went down.
    #[allow(clippy::too_many_arguments)]
    pub fn hit(
        &mut self,
        id: u32,
        part: usize,
        owner: u32,
        at: Vec3,
        penetration_mm: f32,
        damage: f32,
        data: &Data,
        fx: &mut MessageWriter<Fx>,
    ) -> bool {
        let by_pilot = self.pilot.iter().any(|m| m.0 == owner);
        let Some(air) = &mut self.air else { return false };
        let Some(a) = air.0.get_mut(id - CIVIL_ID_BASE) else {
            return false;
        };
        let Some(spec) = data.0.air_traffic.kinds.get(&a.kind) else {
            return false;
        };
        let out = a.hit_part(spec, part, penetration_mm, damage);
        let downed = out.downed;
        if !out.broke.is_empty() {
            self.broke.write(crate::airliners::AirlinerBroke {
                aircraft: id,
                zones: out.broke,
                at,
            });
        }
        if downed {
            fx.write(Fx::new("explosion_air", a.pos));
        }
        let base = nearest_base(&self.plan.bases, Vec2::new(at.x, at.z));
        self.out.write(CivilAirHit {
            aircraft: id,
            at,
            base,
            downed,
            by_pilot,
        });
        downed
    }
}

/// Flies, draws and counts civil air traffic.
pub struct AirTrafficPlugin;

impl Plugin for AirTrafficPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Neutrals>()
            .init_resource::<CivilStanding>()
            .init_resource::<KindLooks>()
            .add_message::<CivilAirHit>()
            .add_message::<crate::airliners::AirlinerBroke>()
            .add_message::<crate::airliners::AirlinerCrashed>()
            .add_world_spawn(start)
            .add_world_spawn(park_aircraft)
            .add_game_systems(FixedUpdate, list_neutrals.in_set(SimSet::Input).after(crate::units::list_hostiles))
            .add_game_systems(
                FixedUpdate,
                step_traffic
                    .in_set(SimSet::Mech)
                    .after(crate::mech::step_mech)
                    .before(crate::combat::fly_shots),
            )
            .add_game_systems(FixedUpdate, tally_hits.in_set(SimSet::Mech).after(crate::combat::fly_shots))
            .add_game_systems(
                Update,
                (
                    crate::airliners::crash_airliners,
                    crate::airliners::break_airliners,
                    draw_traffic,
                    bind_models,
                    spin_parts,
                    crate::airliners::pose_airliners,
                    crate::airliners::fall_parts,
                )
                    .chain(),
            );
    }
}

fn start(mut commands: Commands, plan: Res<Plan>) {
    commands.insert_resource(AirTraffic(Traffic::new(plan.seed)));
}

/// Parks the aircraft each site's page parks at its stands (airport-layout: "parked aircraft are the civil traffic's own
/// models in the game"): each kind's model, its wheels on the apron, its nose as the page turns it.
fn park_aircraft(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    mut looks: ResMut<KindLooks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for site in &plan.sites {
        for a in &site.template.parked {
            // `GameData::load` has checked every parked kind is one.
            let Some(spec) = data.0.air_traffic.kinds.get(&a.kind) else {
                continue;
            };
            let at = site.point_to_world(a.at) + Vec3::Y * rest_m(spec);
            let root = spawn_aircraft(
                &mut commands,
                &mut looks,
                &mut meshes,
                &mut materials,
                &assets,
                &a.kind,
                spec,
                format!("parked {}", a.kind),
                Transform::from_translation(at).with_rotation(
                    Quat::from_rotation_y(a.heading_rad + site.heading_rad())
                        * Quat::from_rotation_x(cb_core::air_traffic::rest_pitch_rad(spec)),
                ),
                true,
            );
            commands.entity(root).insert(Parked);
        }
    }
}

/// The civil aircraft in the air, for targeting and the cockpit.
pub fn list_neutrals(air: Option<Res<AirTraffic>>, data: Res<Data>, mut out: ResMut<Neutrals>) {
    out.0.clear();
    let Some(air) = air else { return };
    for a in air.0.list.iter().filter(|a| !a.downed()) {
        let Some(spec) = data.0.air_traffic.kinds.get(&a.kind) else {
            continue;
        };
        let rest = rest_m(spec);
        out.0.push(Hostile {
            id: CIVIL_ID_BASE + a.id,
            aim: a.pos,
            feet: a.pos - Vec3::Y * rest,
            height_m: spec.height_m.max(spec.length_m * 0.3),
            vel: a.vel,
            shelling: false,
            wave: None,
        });
    }
}

/// The fights traffic keeps clear of: every live enemy, `keep_clear_m` round it.
fn zones(hostiles: &Hostiles, keep_clear_m: f32) -> Vec<Zone> {
    hostiles
        .0
        .iter()
        .map(|h| Zone {
            centre: Vec2::new(h.aim.x, h.aim.z),
            radius_m: keep_clear_m,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn step_traffic(
    mut air: ResMut<AirTraffic>,
    data: Res<Data>,
    plan: Res<Plan>,
    pilot: Res<PilotSim>,
    me: Query<&MechSim, With<Piloted>>,
    hostiles: Res<Hostiles>,
    time: Res<Time<Fixed>>,
    mut live: ResMut<Live>,
    mut explosions: MessageWriter<Explosion>,
    mut crashed: MessageWriter<crate::airliners::AirlinerCrashed>,
) {
    let _t = crate::perf_hud::slow("step_traffic");
    let cfg = &data.0.air_traffic;
    let p = &pilot.curr;
    let eye = match me.iter().next() {
        Some(m) if p.mode == PilotMode::Seated => m.curr.pos + Vec3::Y * data.0.mech.body.eye_height_m,
        _ => p.feet,
    };
    let w = Watcher {
        eye,
        look: crate::targeting::aim_dir(p.yaw_rad, p.pitch_rad),
    };
    let zones = zones(&hostiles, cfg.avoid.keep_clear_m);
    let ground = |x: f32, z: f32| plan.height_at(x, z);
    let island = Island {
        ground: &ground,
        bases: &plan.bases,
        cities: &plan.cities,
        zones: &zones,
    };
    for e in air.0.step(cfg, &w, &island, time.delta_secs()) {
        match e.what {
            Happened::Crashed => {
                let Some(spec) = cfg.kinds.get(&e.kind) else { continue };
                explosions.write(Explosion {
                    at: e.at,
                    energy_mj: spec.crash_mj,
                    // a crash is nobody's blast: the hit that downed it was charged already (survey W42)
                    by: None,
                });
                // a modelled airliner is strewn as its own pieces (airliner-models), not boxes
                if !spec.zones().is_empty() {
                    crashed.write(crate::airliners::AirlinerCrashed {
                        aircraft: e.id,
                        heading_rad: e.heading_rad,
                    });
                    info!("air traffic: {} {} crashed at ({:.0}, {:.0})", e.kind, e.id, e.at.x, e.at.z);
                    continue;
                }
                // The wreck, as the debris rules lay any falling piece down: a body near the pilot,
                // a dust puff far away.
                for (centre, size) in shape::wreck(spec, e.id, e.at, e.heading_rad, cfg.fall.wreck_pieces) {
                    live.new_chunks.push(cb_core::world::Chunk {
                        building: CIVIL_ID_BASE + e.id,
                        centre,
                        size,
                    });
                }
                info!("air traffic: {} {} crashed at ({:.0}, {:.0})", e.kind, e.id, e.at.x, e.at.z);
            }
            Happened::Diverted => info!("air traffic: {} {} diverted from a fight", e.kind, e.id),
            Happened::Landed | Happened::Passed(_) => {}
        }
    }
}

/// The pilot's hits go on the tally of the base nearest each.
fn tally_hits(mut hits: MessageReader<CivilAirHit>, mut standing: ResMut<CivilStanding>) {
    for h in hits.read() {
        if let (true, Some(base)) = (h.by_pilot, h.base) {
            standing.0.record(base, h.downed);
            warn!(
                "civil aircraft {} hit by the pilot near air base {base}{}",
                h.aircraft - CIVIL_ID_BASE,
                if h.downed { ": shot down" } else { "" }
            );
        }
    }
}

/// An aircraft's drawing: its root, by aircraft id.
#[derive(Component)]
pub struct CivilDraw(pub u32);

/// A part turning in flight (a rotor or propeller), radians a second.
#[derive(Component)]
struct Spinner(Spin, f32);

/// Each kind's stand-in, built the first time it is drawn: shared by the traffic and the air drop's
/// cargo aircraft (`air_drop`), so a kind looks the same wherever it flies.
#[derive(Resource, Default)]
pub struct KindLooks(BTreeMap<String, KindLook>);

/// Spawns kind `kind` named `name` at `t`: its model when it has one (survey D8; bound to its levels of detail and gear
/// by `bind_models` once the scene is in, the gear kept up when `gear` is false), else its block-out stand-in
/// (`shape::blockout`), its rotors and propellers turning; returns its root.
#[allow(clippy::too_many_arguments)]
pub fn spawn_aircraft(
    commands: &mut Commands,
    looks: &mut KindLooks,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    kind: &str,
    spec: &AircraftSpec,
    name: String,
    t: Transform,
    gear: bool,
) -> Entity {
    if let Some(m) = &spec.model {
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(m.scene.clone()));
        return commands
            .spawn((
                Name::new(name),
                t,
                Visibility::default(),
                bevy::world_serialization::WorldAssetRoot(scene),
                ModelToBind {
                    lod_m: m.lod_m,
                    gear,
                    loaded: m.loaded.clone(),
                },
            ))
            .id();
    }
    let kl = looks.0.entry(kind.to_string()).or_insert_with(|| look(spec, meshes, materials));
    let root = commands
        .spawn((Name::new(format!("{name} (block-out stand-in)")), t, Visibility::default()))
        .id();
    for (mesh, t, dark, spin) in &kl.parts {
        let mat = if *dark { kl.dark.clone() } else { kl.livery.clone() };
        let mut part = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat), *t, ChildOf(root)));
        if let Some(s) = spin {
            // A rotor at about 400 rpm, a propeller at about 2 400.
            let rate = if *s == Spin::Up { 42.0 } else { 250.0 };
            part.insert(Spinner(*s, rate));
        }
    }
    root
}

/// An aircraft parked at a stand: its gear down, its engines stopped, its beacons off.
#[derive(Component)]
pub struct Parked;

/// A modelled aircraft whose scene is still to be bound: where its levels of detail switch, and whether its gear may
/// show.
#[derive(Component)]
pub struct ModelToBind {
    lod_m: [f32; 2],
    gear: bool,
    /// Its manifest and zones: an airliner built of zones binds through `airliners::bind`.
    loaded: Option<std::sync::Arc<cb_core::air_traffic::model::LoadedModel>>,
}

/// A modelled aircraft's landing gear nodes (`GEAR_*`), shown near the ground only.
#[derive(Component)]
pub struct GearNodes(pub Vec<Entity>);

/// Once a modelled aircraft's scene is in: every mesh under `LOD0`, `LOD1` and `LOD2` gets the visibility range of its
/// level (each blends into the next over 10 % of its switch distance, so a level never pops), and its gear nodes are
/// found, hidden when its gear never shows.
#[allow(clippy::too_many_arguments)]
pub fn bind_models(
    mut commands: Commands,
    todo: Query<(Entity, &ModelToBind, Has<Parked>)>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    is_mesh: Query<(), With<Mesh3d>>,
    transforms: Query<&Transform>,
    mut vis: Query<&mut Visibility>,
) {
    use bevy::camera::visibility::VisibilityRange;
    for (root, m, parked) in &todo {
        // an airliner built of damage zones (airliner-models): its own binding
        if let Some(l) = m.loaded.as_ref().filter(|l| !l.zones.is_empty()) {
            if crate::airliners::bind(
                &mut commands,
                root,
                l,
                m.lod_m,
                parked,
                &children,
                &parents,
                &names,
                &is_mesh,
                &transforms,
            ) {
                commands.entity(root).remove::<ModelToBind>();
            }
            continue;
        }
        let mut by_name: HashMap<&str, Entity> = HashMap::new();
        let mut gear = Vec::new();
        for e in children.iter_descendants(root) {
            if let Ok(n) = names.get(e) {
                by_name.insert(n.as_str(), e);
                if n.as_str().starts_with("GEAR_") {
                    gear.push(e);
                }
            }
        }
        let (Some(&l0), Some(&l1), Some(&l2)) = (by_name.get("LOD0"), by_name.get("LOD1"), by_name.get("LOD2")) else {
            continue; // not spawned yet
        };
        let [a, b] = m.lod_m;
        let (ma, mb) = (a * 0.1, b * 0.1);
        let ranges = [
            (
                l0,
                VisibilityRange {
                    start_margin: 0.0..0.0,
                    end_margin: (a - ma)..(a + ma),
                    use_aabb: false,
                },
            ),
            (
                l1,
                VisibilityRange {
                    start_margin: (a - ma)..(a + ma),
                    end_margin: (b - mb)..(b + mb),
                    use_aabb: false,
                },
            ),
            (
                l2,
                VisibilityRange {
                    start_margin: (b - mb)..(b + mb),
                    end_margin: f32::MAX..f32::MAX,
                    use_aabb: false,
                },
            ),
        ];
        for (lod, range) in ranges {
            for e in children.iter_descendants(lod).filter(|e| is_mesh.contains(*e)) {
                commands.entity(e).insert(range.clone());
            }
        }
        if !m.gear {
            for g in &gear {
                if let Ok(mut v) = vis.get_mut(*g) {
                    *v = Visibility::Hidden;
                }
            }
        }
        commands.entity(root).remove::<ModelToBind>().insert(GearNodes(gear));
    }
}

/// One kind's stand-in: its parts' meshes and placements, and its two materials.
struct KindLook {
    parts: Vec<(Handle<Mesh>, Transform, bool, Option<Spin>)>,
    livery: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
}

fn look(spec: &AircraftSpec, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> KindLook {
    let livery = materials.add(StandardMaterial {
        base_color: Color::srgb(spec.tint[0], spec.tint[1], spec.tint[2]),
        perceptual_roughness: 0.55,
        metallic: 0.2,
        ..default()
    });
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.11),
        perceptual_roughness: 0.6,
        ..default()
    });
    let parts = shape::blockout(spec)
        .into_iter()
        .map(|b| {
            let mesh = meshes.add(Cuboid::new(b.size.x, b.size.y, b.size.z));
            (mesh, Transform::from_translation(b.centre), b.dark, b.spin)
        })
        .collect();
    KindLook { parts, livery, dark }
}

/// Builds each aircraft's stand-in when it first appears, poses it every frame, removes it when it
/// is gone or wrecked (the wreck is debris then), and keeps the jets' engine loops going.
#[allow(clippy::too_many_arguments)]
fn draw_traffic(
    mut commands: Commands,
    air: Option<Res<AirTraffic>>,
    data: Res<Data>,
    ear: Res<Ear>,
    plan: Res<Plan>,
    assets: Res<AssetServer>,
    mut drawn: Query<(Entity, &CivilDraw, &mut Transform, Option<&GearNodes>)>,
    mut gear_vis: Query<&mut Visibility>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut looks: ResMut<KindLooks>,
    mut loops: MessageWriter<Loop>,
) {
    let _t = crate::perf_hud::slow("draw_traffic");
    let Some(air) = air else { return };
    let cfg = &data.0.air_traffic;
    let flying: HashMap<u32, &cb_core::air_traffic::Aircraft> = air.0.list.iter().filter(|a| a.flying()).map(|a| (a.id, a)).collect();
    let mut seen = std::collections::BTreeSet::new();
    for (e, d, mut t, gear) in &mut drawn {
        let Some(a) = flying.get(&d.0) else {
            commands.entity(e).despawn();
            continue;
        };
        seen.insert(d.0);
        let q = cfg
            .kinds
            .get(&a.kind)
            .map_or_else(|| shape::attitude(a), |spec| shape::attitude_for(a, spec));
        *t = Transform::from_translation(a.pos).with_rotation(q);
        // a modelled aircraft's gear is down only near the ground (survey D8)
        if let (Some(gear), Some(m)) = (gear, cfg.kinds.get(&a.kind).and_then(|s| s.model.as_ref())) {
            let low = a.pos.y - plan.terrain.height_at(a.pos.x, a.pos.z) < m.gear_below_m;
            let want = if low { Visibility::Inherited } else { Visibility::Hidden };
            for g in &gear.0 {
                if let Ok(mut v) = gear_vis.get_mut(*g)
                    && *v != want
                {
                    *v = want;
                }
            }
        }
        if let Some(engine) = cfg.kinds.get(&a.kind).and_then(|s| s.engine)
            && !a.downed()
            && a.pos.distance(ear.at) < data.0.audio.cues.audible_within_m
        {
            loops.write(Loop {
                cue: engine_cue(engine.sound),
                at: SoundAt::Node(e),
                level: LoopLevel {
                    db: 0.0,
                    pitch: engine.pitch,
                },
            });
        }
    }
    for a in air.0.list.iter().filter(|a| a.flying() && !seen.contains(&a.id)) {
        let Some(spec) = cfg.kinds.get(&a.kind) else { continue };
        let root = spawn_aircraft(
            &mut commands,
            &mut looks,
            &mut meshes,
            &mut materials,
            &assets,
            &a.kind,
            spec,
            format!("civil {} {}", a.kind, a.id),
            Transform::from_translation(a.pos).with_rotation(shape::attitude_for(a, spec)),
            true,
        );
        commands.entity(root).insert(CivilDraw(a.id));
    }
}

fn spin_parts(time: Res<Time>, mut parts: Query<(&Spinner, &mut Transform)>) {
    let dt = time.delta_secs();
    for (s, mut t) in &mut parts {
        let axis = match s.0 {
            Spin::Up => Vec3::Y,
            Spin::Ahead => Vec3::Z,
        };
        t.rotate_local_axis(Dir3::new(axis).unwrap_or(Dir3::Y), s.1 * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_ids_are_neither_a_vehicles_nor_the_fortresss() {
        assert!(is_civil(CIVIL_ID_BASE) && is_civil(CIVIL_ID_BASE + 5));
        assert!(!is_civil(crate::units::UNIT_ID_BASE + 5), "a vehicle");
        assert!(!is_civil(crate::fortress::FORTRESS_ID), "the fortress");
        assert!(!is_civil(0), "the pilot's mech");
    }

    #[test]
    fn every_live_enemy_is_the_centre_of_a_fight() {
        let h = |id, x, z| Hostile {
            id,
            aim: Vec3::new(x, 50.0, z),
            feet: Vec3::new(x, 40.0, z),
            height_m: 20.0,
            vel: Vec3::ZERO,
            shelling: false,
            wave: None,
        };
        let z = zones(&Hostiles(vec![h(1, 100.0, -200.0), h(5001, -3000.0, 0.0)]), 2500.0);
        assert_eq!(z.len(), 2);
        assert_eq!(z[0].centre, Vec2::new(100.0, -200.0));
        assert_eq!(z[1].radius_m, 2500.0);
    }
}
