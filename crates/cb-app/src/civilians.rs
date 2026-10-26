//! Civilians in the game (openspec change `civilians`): steps `cb_core::civilians` every fixed step round
//! the pilot, feeds it the dangers of the step (every round in flight and every close-combat blow, every
//! explosion, every mech foot and skate wheel on the ground), counts the deaths the pilot's own mech
//! caused against the nearest base and sends each as a `CivilianLoss` (the career's standing hook,
//! survey W34), lets the pilot greet the person in front of them with Use, and draws everyone.
//!
//! The bodies are drawn by `people_vat` (openspec change `people-vat`): MPFB bodies whose baked clips play on
//! the GPU, a fixed few draws for the whole crowd. Park benches (`routes::park_benches`) are drawn here, in
//! one mesh (stand-ins). `CB_NO_PEOPLE=1` leaves the plugin out, for the benchmark's comparison.

use crate::career::CareerRun;
use crate::combat::Shots;
use crate::ground_marks::{Contact, GroundContact};
use crate::mech::{MechId, Piloted};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::street_doors::RefugeDoors;
use crate::world::{Live, MeshBuilder, Plan};
use bevy::prelude::*;
use cb_core::civilians::{Danger, Doors, NoDoors, People, StandingLoss, Toll, World, routes};
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
pub(crate) struct Greeting {
    /// Who answered and what they said, as the screen shows it.
    pub(crate) line: String,
    /// Seconds it stays on screen yet.
    pub(crate) left_s: f32,
}

/// The greeting's text on screen.
#[derive(Component)]
struct GreetingText;

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
        .add_plugins(crate::people_vat::PeopleVatPlugin)
        .add_world_spawn((spawn_benches, spawn_greeting_text))
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
        .add_game_systems(Update, (show_greeting, report_losses));
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
