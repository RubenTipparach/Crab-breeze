//! Civilians in the game (openspec change `civilians`): steps `cb_core::civilians` every fixed step round
//! the pilot, feeds it the dangers of the step (every round in flight and every close-combat blow, every
//! explosion, every mech foot and skate wheel on the ground), counts the deaths the pilot's own mech
//! caused against the nearest base and sends each as a `CivilianLoss` (the career's standing hook,
//! survey W34), lets the pilot greet the person in front of them with Use, and draws everyone.
//!
//! **The bodies are stand-ins** until the humanoid pipeline (`blender-humanoid-characters`) runs here:
//! box figures built in code at true human size (1.55-1.95 m, from the person's own height), in eight
//! clothing palettes, named `person (stand-in)` in the scene. Near the eye (`draw.near_m`) each is an
//! entity from a fixed pool posed standing, in two walking strides, sitting or lying; past it everyone
//! is drawn in one merged mesh rebuilt each frame (a low figure in the middle distance, a flat card far
//! off), one draw for the whole crowd. Park benches (`routes::park_benches`) are drawn here too, in one
//! mesh. `CB_NO_PEOPLE=1` leaves the plugin out, for the benchmark's comparison.

use crate::career::CareerRun;
use crate::combat::Shots;
use crate::ground_marks::{Contact, GroundContact};
use crate::mech::{MechId, Piloted};
use crate::pilot::{PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet, SimSet};
use crate::state::GameApp;
use crate::street_doors::RefugeDoors;
use crate::world::{Live, MeshBuilder, Plan};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use cb_core::civilians::{Activity, Danger, Doors, DrawConfig, NoDoors, People, Person, StandingLoss, Toll, World, routes};
use cb_core::jobs::JobState;
use cb_core::pilot::PilotMode;
use cb_core::world::Scene;

/// Everyone in the world, and the pilot's toll.
#[derive(Resource)]
pub struct Crowd {
    /// The people.
    pub people: People,
    /// The deaths so far, the pilot's counted by base.
    pub toll: Toll,
}

/// A civilian killed by the pilot's own mech (its rounds, blasts or feet): the career's standing with
/// `loss.base` reads it (survey W34). Nothing here decides the cost.
#[derive(Message, Clone, Copy, Debug)]
pub struct CivilianLoss(pub StandingLoss);

/// The greeting on screen: who answered, what they said, and for how long yet.
#[derive(Resource, Default)]
struct Greeting {
    line: String,
    left_s: f32,
}

/// The greeting's text on screen.
#[derive(Component)]
struct GreetingText;

/// A pooled near body.
#[derive(Component)]
struct NearBody;

/// The merged mesh of everyone past the near distance.
#[derive(Component)]
struct CrowdMesh;

/// The stand-in bodies: `[palette][pose]`.
#[derive(Resource)]
struct Bodies {
    meshes: Vec<[Handle<Mesh>; POSES]>,
    crowd: Handle<Mesh>,
}

/// Standing, stride A, stride B, sitting.
const POSES: usize = 4;
const STAND: usize = 0;
const SIT: usize = 3;

/// The figures are modelled 1.75 m tall and scaled to each person's height.
const MODEL_HEIGHT_M: f32 = 1.75;

/// Clothing palettes: shirt, trousers, skin, hair (linear RGB, 0-1). Stand-in colours, chosen to read
/// as a varied street crowd in the 1980s industrial setting (survey G1): workwear, overcoats, denim.
const PALETTES: [[[f32; 4]; 4]; 8] = [
    [
        [0.42, 0.18, 0.12, 1.0],
        [0.10, 0.11, 0.14, 1.0],
        [0.80, 0.62, 0.50, 1.0],
        [0.10, 0.07, 0.05, 1.0],
    ],
    [
        [0.20, 0.30, 0.45, 1.0],
        [0.25, 0.24, 0.22, 1.0],
        [0.55, 0.38, 0.27, 1.0],
        [0.04, 0.03, 0.03, 1.0],
    ],
    [
        [0.55, 0.50, 0.38, 1.0],
        [0.18, 0.20, 0.30, 1.0],
        [0.92, 0.75, 0.62, 1.0],
        [0.45, 0.30, 0.15, 1.0],
    ],
    [
        [0.65, 0.40, 0.08, 1.0],
        [0.20, 0.22, 0.20, 1.0],
        [0.35, 0.23, 0.16, 1.0],
        [0.05, 0.04, 0.04, 1.0],
    ],
    [
        [0.30, 0.32, 0.30, 1.0],
        [0.08, 0.08, 0.09, 1.0],
        [0.72, 0.55, 0.42, 1.0],
        [0.55, 0.52, 0.48, 1.0],
    ],
    [
        [0.62, 0.62, 0.58, 1.0],
        [0.30, 0.20, 0.14, 1.0],
        [0.62, 0.45, 0.33, 1.0],
        [0.20, 0.12, 0.06, 1.0],
    ],
    [
        [0.18, 0.35, 0.25, 1.0],
        [0.40, 0.36, 0.30, 1.0],
        [0.85, 0.68, 0.56, 1.0],
        [0.60, 0.42, 0.20, 1.0],
    ],
    [
        [0.50, 0.12, 0.25, 1.0],
        [0.12, 0.14, 0.20, 1.0],
        [0.45, 0.31, 0.22, 1.0],
        [0.08, 0.06, 0.05, 1.0],
    ],
];

/// Steps, greets and draws the people.
pub struct CiviliansPlugin;

impl Plugin for CiviliansPlugin {
    fn build(&self, app: &mut App) {
        let seed = app.world().resource::<crate::world::Seed>().0;
        app.insert_resource(Crowd {
            people: People::new(seed),
            toll: Toll::default(),
        })
        .init_resource::<Greeting>()
        .add_message::<CivilianLoss>()
        .add_world_spawn((spawn_bodies, spawn_benches, spawn_greeting_text))
        .add_game_systems(
            FixedUpdate,
            (
                greet
                    .in_set(SimSet::Pilot)
                    .after(crate::interact::choose)
                    .before(crate::pilot::step_pilot),
                rounds_through_crowd
                    .in_set(SimSet::Mech)
                    .after(crate::mech::step_mech)
                    .before(crate::combat::fly_shots),
                step_crowd.in_set(SimSet::Mech).after(crate::combat::fly_shots),
            ),
        )
        .add_game_systems(Update, (draw_people.in_set(PresentSet::Pose), show_greeting, report_losses));
    }
}

/// The ground and walls people walk on, and the street doors they run for (`street_doors::RefugeDoors`,
/// the buildings' real doors; none before the world has found them, and people flee down the street).
fn with_world<R>(plan: &Plan, live: &Live, data: &Data, doors: Option<&RefugeDoors>, f: impl FnOnce(&World) -> R) -> R {
    let ground = Scene {
        plan,
        dug: &live.dug,
        trees: None,
        structures: Some(&live.structures),
        platforms: &[],
        interiors: None,
    };
    let w = World {
        cities: &plan.cities,
        bases: &plan.bases,
        cfg: &data.0.world,
        ground: &ground,
        doors: doors.map_or(&NoDoors as &dyn Doors, |d| &d.0 as &dyn Doors),
    };
    f(&w)
}

/// The id of the mech the pilot drives (0 when there is none yet).
fn pilot_mech(piloted: &Query<&MechId, With<Piloted>>) -> u32 {
    piloted.iter().next().map_or(0, |m| m.0)
}

/// Counts the step's deaths, and sends the pilot's as standing losses.
fn count(crowd: &mut Crowd, deaths: Vec<cb_core::civilians::Casualty>, pilot: u32, plan: &Plan, losses: &mut MessageWriter<CivilianLoss>) {
    for c in deaths {
        info!("civilians: person {} killed ({:?}) by {:?} at {:?}", c.person, c.cause, c.by, c.at);
        if let Some(loss) = crowd.toll.record(&c, pilot, &plan.bases) {
            losses.write(CivilianLoss(loss));
        }
    }
}

/// Every round in flight and every close-combat blow this step, as a danger, before the shots fly on.
fn rounds_through_crowd(
    mut crowd: ResMut<Crowd>,
    shots: Res<Shots>,
    plan: Res<Plan>,
    live: Res<Live>,
    data: Res<Data>,
    time: Res<Time<Fixed>>,
    piloted: Query<&MechId, With<Piloted>>,
    mut losses: MessageWriter<CivilianLoss>,
    doors: Option<Res<RefugeDoors>>,
) {
    let _t = crate::perf_hud::slow("rounds_through_crowd");
    if crowd.people.list.is_empty() || (shots.0.is_empty() && shots.1.is_empty()) {
        return;
    }
    let dt = time.delta_secs();
    let dangers: Vec<Danger> = shots
        .0
        .iter()
        .map(|s| Danger::Shot {
            from: s.pos,
            to: s.pos + s.vel * dt,
            by: Some(s.owner),
        })
        .chain(shots.1.iter().map(|b| Danger::Shot {
            from: b.from,
            to: b.to,
            by: Some(b.owner),
        }))
        .collect();
    let crowd = &mut *crowd;
    let deaths = with_world(&plan, &live, &data, doors.as_deref(), |w| {
        dangers
            .iter()
            .flat_map(|d| crowd.people.danger(w, &data.0.people, d))
            .collect::<Vec<_>>()
    });
    count(crowd, deaths, pilot_mech(&piloted), &plan, &mut losses);
}

/// The step's blasts and mech feet, then people brought in round the pilot, and everyone stepped.
fn step_crowd(
    mut crowd: ResMut<Crowd>,
    mut explosions: MessageReader<crate::blast::Explosion>,
    mut contacts: MessageReader<GroundContact>,
    plan: Res<Plan>,
    live: Res<Live>,
    data: Res<Data>,
    pilot: Res<PilotSim>,
    time: Res<Time<Fixed>>,
    piloted: Query<&MechId, With<Piloted>>,
    mut losses: MessageWriter<CivilianLoss>,
    doors: Option<Res<RefugeDoors>>,
) {
    let _t = crate::perf_hud::slow("step_crowd");
    let mut dangers: Vec<Danger> = explosions
        .read()
        .map(|e| Danger::Blast {
            at: e.at,
            energy_mj: e.energy_mj,
            by: e.by,
        })
        .collect();
    dangers.extend(contacts.read().map(|c| match c.contact {
        Contact::Step { foot, .. } => Danger::Footfall {
            at: foot,
            by: Some(c.mech),
        },
        Contact::Wheel { at, .. } => Danger::Footfall { at, by: Some(c.mech) },
    }));
    let eye = pilot.curr.eye(&data.0.pilot);
    let people = &data.0.people;
    let crowd = &mut *crowd;
    let deaths = with_world(&plan, &live, &data, doors.as_deref(), |w| {
        let deaths: Vec<_> = dangers.iter().flat_map(|d| crowd.people.danger(w, people, d)).collect();
        crowd.people.keep_around(w, people, Vec2::new(eye.x, eye.z));
        crowd.people.step(w, people, time.delta_secs());
        deaths
    });
    count(crowd, deaths, pilot_mech(&piloted), &plan, &mut losses);
}

/// Prints each standing loss for the playtest log (`CIVILIAN_LOSS`); `civil_standing` charges it to the career.
fn report_losses(mut losses: MessageReader<CivilianLoss>, crowd: Res<Crowd>) {
    for CivilianLoss(l) in losses.read() {
        println!(
            "CIVILIAN_LOSS base={:?} cause={:?} at=({:.0}, {:.0}) pilot_total={}",
            l.base,
            l.cause,
            l.at.x,
            l.at.z,
            crowd.toll.by_pilot()
        );
    }
}

/// The subject of an open job offer, for a line that tells of one.
fn open_job(career: Option<&CareerRun>) -> Option<String> {
    let board = &career?.career.board;
    board
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Offered)
        .find_map(|j| board.inbox.iter().rev().find(|m| m.job == Some(j.id)).map(|m| m.subject.clone()))
}

/// On foot, Use with the greeting selected greets the person in front of the pilot (`interact`): the greeting is
/// selected first when the mech is in reach too (survey W40), and the wheel picks the mech instead.
fn greet(
    mut crowd: ResMut<Crowd>,
    choice: Res<crate::interact::UseChoice>,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    career: Option<Res<CareerRun>>,
    mut greeting: ResMut<Greeting>,
) {
    let p = &pilot.curr;
    if choice.now != Some(cb_core::interact::Interaction::Greet) || p.mode != PilotMode::OnFoot {
        return;
    }
    let job = open_job(career.as_deref());
    if let Some((id, answer)) = crowd.people.greet(&data.0.people, p.feet, p.yaw_rad, job.as_deref()) {
        info!("civilians: greeted person {id}: {}: {}", answer.who, answer.text);
        greeting.line = format!("{}: \"{}\"", answer.who, answer.text);
        greeting.left_s = data.0.people.cfg.greet_hold_s;
    }
}

fn spawn_greeting_text(mut commands: Commands) {
    commands.spawn((
        GreetingText,
        Text::new(""),
        TextFont::from_font_size(24.0),
        TextColor(Color::srgb(0.95, 0.93, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            bottom: percent(22.0),
            width: percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center),
    ));
}

/// The answer while it lasts. "[E] Greet" itself is one of the prompt's options (`pilot::show_prompt`, survey W40).
fn show_greeting(mut greeting: ResMut<Greeting>, time: Res<Time>, mut text: Single<&mut Text, With<GreetingText>>) {
    greeting.left_s = (greeting.left_s - time.delta_secs()).max(0.0);
    let want = if greeting.left_s > 0.0 { greeting.line.as_str() } else { "" };
    if text.0 != want {
        text.0 = want.into();
    }
}

/// Adds a box whose bottom face is centred on `bottom` and top face on `top` (sheared when they are not
/// above one another), `half` its half width (x) and depth (z).
fn limb(m: &mut MeshBuilder, bottom: Vec3, top: Vec3, half: Vec2, color: [f32; 4]) {
    let v = |sx: f32, up: bool, sz: f32| (if up { top } else { bottom }) + Vec3::new(sx * half.x, 0.0, sz * half.y);
    m.quad(
        [v(-1.0, true, -1.0), v(-1.0, true, 1.0), v(1.0, true, 1.0), v(1.0, true, -1.0)],
        color,
    );
    m.quad(
        [v(-1.0, false, 1.0), v(1.0, false, 1.0), v(1.0, true, 1.0), v(-1.0, true, 1.0)],
        color,
    );
    m.quad(
        [v(1.0, false, -1.0), v(-1.0, false, -1.0), v(-1.0, true, -1.0), v(1.0, true, -1.0)],
        color,
    );
    m.quad(
        [v(1.0, false, 1.0), v(1.0, false, -1.0), v(1.0, true, -1.0), v(1.0, true, 1.0)],
        color,
    );
    m.quad(
        [v(-1.0, false, -1.0), v(-1.0, false, 1.0), v(-1.0, true, 1.0), v(-1.0, true, -1.0)],
        color,
    );
    m.quad(
        [v(-1.0, false, -1.0), v(1.0, false, -1.0), v(1.0, false, 1.0), v(-1.0, false, 1.0)],
        color,
    );
}

/// The stand-in body, 1.75 m tall, feet at the origin, facing -z, in `pose`.
fn body_mesh(palette: &[[f32; 4]; 4], pose: usize) -> Mesh {
    let [shirt, trousers, skin, hair] = *palette;
    let mut m = MeshBuilder::default();
    if pose == SIT {
        // on a seat 0.46 m up (the bench's top), thighs level, shins down, back upright
        for sx in [-0.09f32, 0.09] {
            m.block(Vec3::new(sx - 0.065, 0.46, -0.45), Vec3::new(sx + 0.065, 0.58, 0.02), trousers);
            m.block(Vec3::new(sx - 0.06, 0.0, -0.50), Vec3::new(sx + 0.06, 0.46, -0.38), trousers);
            limb(
                &mut m,
                Vec3::new(sx * 2.6, 0.68, -0.18),
                Vec3::new(sx * 2.7, 1.10, 0.05),
                Vec2::new(0.05, 0.055),
                shirt,
            );
        }
        m.block(Vec3::new(-0.19, 0.56, -0.06), Vec3::new(0.19, 1.12, 0.16), shirt);
        m.block(Vec3::new(-0.05, 1.12, 0.0), Vec3::new(0.05, 1.18, 0.10), skin);
        m.block(Vec3::new(-0.10, 1.18, -0.06), Vec3::new(0.10, 1.43, 0.16), skin);
        m.block(Vec3::new(-0.105, 1.34, -0.03), Vec3::new(0.105, 1.45, 0.165), hair);
        return m.build();
    }
    // stride A: left leg forward (-z), right back, arms the other way; B the mirror; standing: neither
    let swing = match pose {
        1 => 1.0,
        2 => -1.0,
        _ => 0.0,
    };
    for (sx, side) in [(-0.09f32, 1.0f32), (0.09, -1.0)] {
        let foot_z = -0.2 * swing * side;
        limb(
            &mut m,
            Vec3::new(sx, 0.0, foot_z),
            Vec3::new(sx, 0.88, 0.0),
            Vec2::new(0.065, 0.075),
            trousers,
        );
        let hand_z = 0.16 * swing * side;
        let hand = Vec3::new(sx * 2.75, 0.80, hand_z);
        limb(&mut m, hand, Vec3::new(sx * 2.7, 1.40, 0.0), Vec2::new(0.05, 0.055), shirt);
        m.block(hand - Vec3::new(0.04, 0.09, 0.04), hand + Vec3::new(0.04, 0.0, 0.04), skin);
    }
    m.block(Vec3::new(-0.19, 0.86, -0.11), Vec3::new(0.19, 1.42, 0.11), shirt);
    m.block(Vec3::new(-0.05, 1.42, -0.05), Vec3::new(0.05, 1.48, 0.05), skin);
    m.block(Vec3::new(-0.10, 1.48, -0.11), Vec3::new(0.10, 1.73, 0.11), skin);
    m.block(Vec3::new(-0.105, 1.64, -0.08), Vec3::new(0.105, 1.75, 0.115), hair);
    m.build()
}

fn spawn_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    data: Res<Data>,
) {
    let cloth = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        ..default()
    });
    let bodies: Vec<[Handle<Mesh>; POSES]> = PALETTES
        .iter()
        .map(|p| std::array::from_fn(|pose| meshes.add(body_mesh(p, pose))))
        .collect();
    for _ in 0..data.0.people.cfg.draw.near_max {
        commands.spawn((
            Name::new("person (stand-in)"),
            NearBody,
            Mesh3d(bodies[0][STAND].clone()),
            MeshMaterial3d(cloth.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let crowd = meshes.add(MeshBuilder::default().build());
    commands.spawn((
        Name::new("crowd (stand-ins, merged)"),
        CrowdMesh,
        Mesh3d(crowd.clone()),
        MeshMaterial3d(cloth),
        Transform::default(),
        NoFrustumCulling,
        NotShadowCaster,
    ));
    commands.insert_resource(Bodies { meshes: bodies, crowd });
}

/// Every park's benches, in one mesh: a seat 0.45 m up, a back and two legs, 1.6 m long (stand-ins).
fn spawn_benches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    plan: Res<Plan>,
    data: Res<Data>,
) {
    let wood = [0.38, 0.25, 0.14, 1.0];
    let iron = [0.12, 0.12, 0.13, 1.0];
    let mut m = MeshBuilder::default();
    let mut n = 0;
    for city in &plan.cities {
        let y = city.ground_m + data.0.world.kerb_height_m;
        for b in city.blocks.iter().filter(|b| b.park) {
            let Some(path) = b.rect.inset(data.0.people.cfg.park_path_inset_m) else {
                continue;
            };
            for (seat, facing) in routes::park_benches(&path) {
                // along: the bench's length runs across the way it faces
                let along = Vec3::new(facing.y.abs(), 0.0, facing.x.abs()) * 0.8;
                let deep = Vec3::new(facing.x.abs(), 0.0, facing.y.abs()) * 0.22;
                let c = Vec3::new(seat.x, y, seat.y);
                let back = c - Vec3::new(facing.x, 0.0, facing.y) * 0.24;
                let span = |a: Vec3, b: Vec3| (a.min(b), a.max(b));
                let (s0, s1) = span(c - along - deep + Vec3::Y * 0.40, c + along + deep + Vec3::Y * 0.46);
                m.block(s0, s1, wood);
                let thin = Vec3::new(facing.x.abs(), 0.0, facing.y.abs()) * 0.03;
                let (b0, b1) = span(back - along - thin + Vec3::Y * 0.50, back + along + thin + Vec3::Y * 0.90);
                m.block(b0, b1, wood);
                for end in [-0.7f32, 0.7] {
                    let at = c + along * (end / 0.8);
                    let leg = Vec3::new(0.04, 0.0, 0.04) + deep;
                    m.block(at - leg, at + leg + Vec3::Y * 0.40, iron);
                }
                n += 1;
            }
        }
    }
    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        ..default()
    });
    info!("civilians: {n} park benches (stand-ins)");
    commands.spawn((
        Name::new("park benches (stand-ins)"),
        Mesh3d(meshes.add(m.build())),
        MeshMaterial3d(mat),
    ));
}

/// The pose a near body takes: lying, sitting, a stride by the distance walked, or standing.
fn pose_of(p: &Person, time_s: f32, draw: &DrawConfig) -> usize {
    match p.activity {
        Activity::Sitting { .. } => SIT,
        _ if p.pace_m_s > 0.2 => {
            // the stride by how far they have come, offset by who they are so a crowd is not in step
            let phase = (time_s * p.pace_m_s / (draw.stride_m * 0.5) + (p.look % 97) as f32 * 0.37) as i64;
            [1, STAND, 2, STAND][phase.rem_euclid(4) as usize]
        }
        _ => STAND,
    }
}

/// Where a person's body goes: feet, facing, their height, laid flat along their heading when struck.
fn body_transform(p: &Person) -> Transform {
    let s = p.height_m / MODEL_HEIGHT_M;
    let turn = Quat::from_rotation_y(p.heading_rad);
    match p.activity {
        Activity::Down { .. } => Transform::from_translation(p.feet() + Vec3::Y * 0.12)
            .with_rotation(turn * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(s)),
        _ => Transform::from_translation(p.feet()).with_rotation(turn).with_scale(Vec3::splat(s)),
    }
}

/// Everyone past the near distance into one mesh: a low three-box figure within `mid_m` of the eye, a
/// card facing the eye beyond. Those drawn near are in `skip`.
fn crowd_mesh(people: &[Person], skip: &[bool], eye: Vec3, draw: &DrawConfig) -> MeshBuilder {
    let mut m = MeshBuilder::default();
    for (p, near) in people.iter().zip(skip) {
        if *near {
            continue;
        }
        let pal = &PALETTES[(p.look as usize) % PALETTES.len()];
        let t = body_transform(p);
        let d = eye.distance(p.feet());
        if d <= draw.mid_m {
            let corner = |v: Vec3| t.transform_point(v);
            let mut boxed = |lo: Vec3, hi: Vec3, c: [f32; 4]| {
                let v = |x: f32, y: f32, z: f32| corner(Vec3::new(x, y, z));
                let (x0, y0, z0, x1, y1, z1) = (lo.x, lo.y, lo.z, hi.x, hi.y, hi.z);
                m.quad([v(x0, y1, z0), v(x0, y1, z1), v(x1, y1, z1), v(x1, y1, z0)], c);
                m.quad([v(x0, y0, z1), v(x1, y0, z1), v(x1, y1, z1), v(x0, y1, z1)], c);
                m.quad([v(x1, y0, z0), v(x0, y0, z0), v(x0, y1, z0), v(x1, y1, z0)], c);
                m.quad([v(x1, y0, z1), v(x1, y0, z0), v(x1, y1, z0), v(x1, y1, z1)], c);
                m.quad([v(x0, y0, z0), v(x0, y0, z1), v(x0, y1, z1), v(x0, y1, z0)], c);
            };
            let sit = matches!(p.activity, Activity::Sitting { .. });
            let (legs, torso, head) = if sit { (0.58, 1.12, 1.43) } else { (0.88, 1.44, 1.74) };
            boxed(Vec3::new(-0.16, 0.0, -0.08), Vec3::new(0.16, legs, 0.08), pal[1]);
            boxed(Vec3::new(-0.21, legs - 0.02, -0.11), Vec3::new(0.21, torso, 0.11), pal[0]);
            boxed(Vec3::new(-0.10, torso + 0.04, -0.10), Vec3::new(0.10, head, 0.10), pal[2]);
        } else {
            // a card the person's height, turned to the eye about the vertical
            let to_eye = Vec3::new(eye.x - p.pos.x, 0.0, eye.z - p.pos.y).normalize_or(Vec3::Z);
            let side = Vec3::new(to_eye.z, 0.0, -to_eye.x) * 0.24;
            let f = p.feet();
            let h = if p.standing() { p.height_m } else { 0.3 };
            m.quad([f - side, f + side, f + side + Vec3::Y * h, f - side + Vec3::Y * h], pal[0]);
        }
    }
    m
}

/// Poses the near bodies from the pool and rebuilds the crowd's merged mesh.
fn draw_people(
    crowd: Res<Crowd>,
    bodies: Option<Res<Bodies>>,
    data: Res<Data>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<PilotCamera>>,
    mut near: Query<(&mut Transform, &mut Mesh3d, &mut Visibility), With<NearBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut last_crowd: Local<(f32, (usize, u64))>,
) {
    let _t = crate::perf_hud::slow("draw_people");
    let Some(bodies) = bodies else { return };
    let draw = &data.0.people.cfg.draw;
    let eye = camera.translation();
    let list = &crowd.people.list;
    let near2 = draw.near_m * draw.near_m;
    let mut skip = vec![false; list.len()];
    let mut pool = near.iter_mut();
    for (i, p) in list.iter().enumerate() {
        if p.feet().distance_squared(eye) > near2 {
            continue;
        }
        let Some((mut t, mut mesh, mut vis)) = pool.next() else { break };
        skip[i] = true;
        *t = body_transform(p);
        let want = &bodies.meshes[(p.look as usize) % bodies.meshes.len()][pose_of(p, time.elapsed_secs(), draw)];
        if mesh.0 != *want {
            mesh.0 = want.clone();
        }
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
    }
    for (_, _, mut vis) in pool {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
    // rebuilt at its rate, and at once whenever someone crosses the near distance (so nobody is drawn
    // twice or not at all between rebuilds)
    let now = time.elapsed_secs();
    let near_set = list.iter().zip(&skip).filter(|(_, n)| **n).fold((0usize, 0u64), |(n, h), (p, _)| {
        (n + 1, h.wrapping_mul(31).wrapping_add(u64::from(p.id)))
    });
    if now - last_crowd.0 >= 1.0 / draw.crowd_refresh_hz || now < last_crowd.0 || near_set != last_crowd.1 {
        *last_crowd = (now, near_set);
        meshes.insert(&bodies.crowd, crowd_mesh(list, &skip, eye, draw).build()).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A measurement instrument, not a check: the CPU time to build the crowd's merged mesh for 2,000
    /// people, all past the near distance (the worst case). Run it in release:
    /// `cargo test --release -p cb-app civilians::tests::cost_of_the_crowd_mesh -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn cost_of_the_crowd_mesh() {
        use cb_core::civilians::{PeopleData, Place};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data: PeopleData = PeopleData {
            cfg: cb_core::data::load_ron(&root.join("people/people.ron")).unwrap(),
            lines: cb_core::data::load_ron(&root.join("people/lines.ron")).unwrap(),
        };
        let _ = Place::Pavement;
        let draw = data.cfg.draw.clone();
        // 2,000 people on a 400 m disc round the eye: about two fifths within the middle distance
        let wcfg: cb_core::world::WorldConfig = cb_core::data::load_ron(&root.join("world/world.ron")).unwrap();
        let city = cb_core::world::layout::lay_out_city(&mut cb_core::rng::SimRng::new(3), 0, Vec2::ZERO, 900.0, 0.0, &wcfg);
        let ground = cb_core::world::FlatGround::default();
        let w = World {
            cities: std::slice::from_ref(&city),
            bases: &[],
            cfg: &wcfg,
            ground: &ground,
            doors: &NoDoors,
        };
        let mut big = data.clone();
        big.cfg.pavement_per_100m = [20.0, 10.0];
        let mut people = People::new(1);
        for _ in 0..100 {
            people.keep_around(&w, &big, Vec2::ZERO);
        }
        let list = &people.list;
        let skip = vec![false; list.len()];
        let eye = Vec3::new(0.0, 1.7, 0.0);
        let mut ms = Vec::new();
        for _ in 0..200 {
            let t = std::time::Instant::now();
            let mesh = crowd_mesh(list, &skip, eye, &draw).build();
            std::hint::black_box(&mesh);
            ms.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        ms.sort_by(f64::total_cmp);
        let mid = list.iter().filter(|p| p.feet().distance(eye) <= draw.mid_m).count();
        eprintln!(
            "{} people ({mid} mid, {} far): crowd mesh p50 {:.3} ms, p95 {:.3} ms, {} vertices",
            list.len(),
            list.len() - mid,
            ms[ms.len() / 2],
            ms[ms.len() * 95 / 100],
            crowd_mesh(list, &skip, eye, &draw).build().count_vertices()
        );
    }

    /// Lowest and highest vertex of a mesh, metres.
    fn height_span(mesh: &Mesh) -> (f32, f32) {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(ps)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("positions");
        };
        ps.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p[1]), hi.max(p[1])))
    }

    #[test]
    fn the_stand_in_is_true_human_size() {
        for pose in 0..POSES {
            let (lo, hi) = height_span(&body_mesh(&PALETTES[0], pose));
            assert!(lo.abs() < 1e-4, "feet on the ground in pose {pose}: {lo}");
            if pose == SIT {
                assert!((1.35..1.5).contains(&hi), "seated head height {hi}");
            } else {
                assert!((hi - MODEL_HEIGHT_M).abs() < 0.01, "pose {pose}: {hi}");
            }
        }
    }

    /// The stand-ins' geometry as the review page draws it (`docs/mockups/civilians.html`, CLAUDE.md
    /// section 10: reviewed orbitable, at true scale): each pose's positions, each vertex's colour role
    /// (shirt, trousers, skin, hair) and the palettes. The committed file must be what this writes; run
    /// with `CB_WRITE_MOCKUP=1` to rewrite it after changing a body.
    #[test]
    fn the_review_page_draws_these_stand_ins() {
        let mut out = String::from("// Generated by cb-app civilians::tests::the_review_page_draws_these_stand_ins. Do not edit.\n");
        out.push_str("window.STANDINS = {\n  palettes: [\n");
        for p in PALETTES {
            let rgb: Vec<String> = p.iter().map(|c| format!("[{:.2}, {:.2}, {:.2}]", c[0], c[1], c[2])).collect();
            out.push_str(&format!("    [{}],\n", rgb.join(", ")));
        }
        out.push_str("  ],\n  poses: {\n");
        for (pose, name) in ["stand", "stride_a", "stride_b", "sit"].iter().enumerate() {
            let mesh = body_mesh(&PALETTES[0], pose);
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(ps)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
                panic!("positions");
            };
            let Some(bevy::mesh::VertexAttributeValues::Float32x4(cs)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) else {
                panic!("colours");
            };
            let Some(bevy::mesh::Indices::U32(idx)) = mesh.indices() else {
                panic!("indices")
            };
            let pos: Vec<String> = ps.iter().flat_map(|p| p.iter().map(|v| format!("{v:.3}"))).collect();
            let roles: Vec<String> = cs
                .iter()
                .map(|c| PALETTES[0].iter().position(|q| q == c).expect("a palette colour").to_string())
                .collect();
            let tri: Vec<String> = idx.iter().map(u32::to_string).collect();
            out.push_str(&format!(
                "    {name}: {{\n      position: [{}],\n      role: [{}],\n      index: [{}],\n    }},\n",
                pos.join(","),
                roles.join(","),
                tri.join(",")
            ));
        }
        out.push_str(&format!("  }},\n  model_height_m: {MODEL_HEIGHT_M},\n}};\n"));
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/mockups/civilians_standins.js");
        if std::env::var("CB_WRITE_MOCKUP").is_ok_and(|v| v == "1") {
            std::fs::write(&path, &out).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == out,
            "docs/mockups/civilians_standins.js is stale: run this test with CB_WRITE_MOCKUP=1"
        );
    }
}
