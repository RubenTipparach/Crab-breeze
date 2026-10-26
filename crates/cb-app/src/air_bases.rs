//! The air bases in the game (openspec change `air-bases`; owner, 2026-10-24: "We'll need some hanger bays or air base
//! for the player to land and get their mech fixed", 2026-10-25: "going to airbase should replenish ammo for free, just
//! need to enter airbase hangar, and I would get repairs and free ammo (free repairs fornow)"). Each base the world plan
//! placed (`cb_core::world::bases`) is drawn as a block-out: a concrete apron over its flattened site, its hangar bays
//! along the back edge (open toward the apron, tall enough for a 24 m mech to walk in), landing pads and a control
//! tower. A piloted mech that stops inside a bay is repaired and rearmed at once, free, once each time it walks in
//! (`cb_core::bases::service`), and the MECH MFD shows what the bay did. Each bay has a terminal just inside its
//! mouth: the pilot walks up to it on foot, looks at it and presses Use, and its screen (the menu's TERMINAL page)
//! signs them up for jobs from the career's board (air-bases design section 5).

use crate::mech::{MechSim, MechSpec, Piloted};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::{MeshBuilder, Plan};
use bevy::prelude::*;
use cb_core::bases::service;
use cb_core::pilot::PilotMode;
use cb_core::world::bases::{Terminal, hangar_at, terminal_in_reach};

/// Draws the air bases, services a mech that stops in a hangar bay, and opens a terminal the pilot uses.
pub struct AirBasesPlugin;

impl Plugin for AirBasesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastService>()
            .init_resource::<TerminalNear>()
            .add_world_spawn(spawn_bases)
            .add_game_systems(FixedUpdate, repair_in_hangars.in_set(SimSet::Mech).after(crate::mech::step_mech))
            .add_game_systems(
                FixedUpdate,
                (
                    find_terminal.in_set(SimSet::Input).after(crate::input::take_step_actions),
                    use_terminal.in_set(SimSet::Pilot).after(crate::interact::choose),
                ),
            )
            .add_game_systems(Update, open_terminal.before(crate::sim::PresentSet::Input));
    }
}

/// What the last hangar bay did to the piloted mech, for the MECH MFD (air-bases task 2.3).
#[derive(Resource, Default)]
pub struct LastService {
    /// The base and bay, and the report's lines; `None` before any bay has serviced the mech.
    pub last: Option<(u32, usize, Vec<String>)>,
    /// Seconds of play since it, for how long the MFD keeps showing it.
    pub since_s: f32,
}

/// How long the MECH MFD shows a bay's report, seconds of play.
pub const SERVICE_SHOWN_S: f32 = 30.0;

/// The terminal the pilot on foot can use now, if any (the prompt reads it), and the one they asked for
/// by pressing Use at it, until the menu opens it.
#[derive(Resource, Default)]
pub struct TerminalNear {
    /// In reach and looked at.
    pub near: Option<Terminal>,
    /// Use was pressed at it this step.
    pub wanted: Option<Terminal>,
}

/// The terminal whose screen is open (the menu's TERMINAL page reads it).
#[derive(Resource, Clone, Copy)]
pub struct AtTerminal(pub Terminal);

/// Wall and roof thickness of a hangar, metres.
const WALL_M: f32 = cb_core::world::bases::HANGAR_WALL_M;
/// The apron's lift over the flattened ground, metres (clear of the terrain, no z-fighting).
const APRON_LIFT_M: f32 = 0.12;
/// A landing pad's side, metres.
const PAD_M: f32 = 40.0;

fn spawn_bases(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let cfg = &data.0.world.bases;
    if plan.0.bases.is_empty() {
        return;
    }
    let concrete = crate::buildings::concrete_material(&mut images, &mut materials);
    let paint = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        ..default()
    });
    // The terminals' screens glow phosphor green, the colour of the pages they show.
    let screen = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.2, 0.08),
        emissive: LinearRgba::rgb(0.3, 1.6, 0.5),
        unlit: false,
        ..default()
    });
    // Darker than a pavement: the whole site is concrete and a light tone washed out in the sun.
    let apron_tone = [0.26, 0.26, 0.25, 1.0];
    let wall_tone = [0.72, 0.72, 0.7, 1.0];
    let roof_tone = [0.5, 0.52, 0.54, 1.0];
    let pad_tone = [0.95, 0.75, 0.15, 1.0];
    for b in &plan.0.bases {
        let mut slab = MeshBuilder::tiled(4.0, 4.0);
        let mut marks = MeshBuilder::default();
        let mut screens = MeshBuilder::default();
        let g = b.ground_m;
        let r = b.rect();
        // The apron over the whole site.
        slab.block(
            Vec3::new(r.min.x, g - 0.5, r.min.y),
            Vec3::new(r.max.x, g + APRON_LIFT_M, r.max.y),
            apron_tone,
        );
        // The hangar bays: side walls, back wall and roof; open at the front (low z).
        let h = cfg.hangar_m.y;
        for bay in b.hangars(cfg) {
            let (lo, hi) = (bay.min, bay.max);
            let top = g + APRON_LIFT_M + h;
            slab.block(Vec3::new(lo.x - WALL_M, g, lo.y), Vec3::new(lo.x, top, hi.y), wall_tone);
            slab.block(Vec3::new(hi.x, g, lo.y), Vec3::new(hi.x + WALL_M, top, hi.y), wall_tone);
            slab.block(
                Vec3::new(lo.x - WALL_M, g, hi.y),
                Vec3::new(hi.x + WALL_M, top, hi.y + WALL_M),
                wall_tone,
            );
            slab.block(
                Vec3::new(lo.x - WALL_M, top, lo.y),
                Vec3::new(hi.x + WALL_M, top + WALL_M, hi.y + WALL_M),
                roof_tone,
            );
            // A yellow stop line across the bay's mouth.
            let y = g + APRON_LIFT_M + 0.02;
            marks.quad(
                [
                    Vec3::new(lo.x, y, lo.y + 1.0),
                    Vec3::new(hi.x, y, lo.y + 1.0),
                    Vec3::new(hi.x, y, lo.y),
                    Vec3::new(lo.x, y, lo.y),
                ],
                pad_tone,
            );
        }
        // Three landing pads across the front of the apron, outlined.
        for i in 0..3 {
            let c = Vec2::new(b.centre.x + (i as f32 - 1.0) * (PAD_M + 40.0), r.min.y + PAD_M);
            let y = g + APRON_LIFT_M + 0.02;
            let half = PAD_M * 0.5;
            for (a, z) in [
                (Vec2::new(-half, -half), Vec2::new(half, -half + 1.0)),
                (Vec2::new(-half, half - 1.0), Vec2::new(half, half)),
                (Vec2::new(-half, -half), Vec2::new(-half + 1.0, half)),
                (Vec2::new(half - 1.0, -half), Vec2::new(half, half)),
            ] {
                let (p, q) = (c + a, c + z);
                marks.quad(
                    [
                        Vec3::new(p.x, y, q.y),
                        Vec3::new(q.x, y, q.y),
                        Vec3::new(q.x, y, p.y),
                        Vec3::new(p.x, y, p.y),
                    ],
                    pad_tone,
                );
            }
        }
        // The runway beside the site and its taxiway (survey W45): asphalt, darker than the apron, with a dashed
        // centre line, edge lines and threshold bars at both ends.
        let rw = b.runway;
        let asphalt = [0.13, 0.13, 0.13, 1.0];
        for f in [rw.rect(), rw.taxiway(&r)] {
            slab.block(
                Vec3::new(f.min.x, g - 0.5, f.min.y),
                Vec3::new(f.max.x, g + APRON_LIFT_M, f.max.y),
                asphalt,
            );
        }
        let y = g + APRON_LIFT_M + 0.02;
        let (d, side) = (rw.dir(), rw.dir().perp());
        let mut stripe = |from: f32, to: f32, off: f32, half_w: f32| {
            let (p, q) = (rw.a + d * from + side * off, rw.a + d * to + side * off);
            let s = side * half_w;
            marks.quad(
                [
                    Vec3::new(p.x - s.x, y, p.y - s.y),
                    Vec3::new(p.x + s.x, y, p.y + s.y),
                    Vec3::new(q.x + s.x, y, q.y + s.y),
                    Vec3::new(q.x - s.x, y, q.y - s.y),
                ],
                [1.0, 1.0, 1.0, 1.0],
            );
        };
        let len = rw.length_m();
        let edge = rw.width_m * 0.5 - 1.0;
        stripe(0.0, len, edge, 0.45);
        stripe(0.0, len, -edge, 0.45);
        for k in 0..8 {
            let off = (k as f32 - 3.5) * 4.0;
            stripe(8.0, 38.0, off, 0.9);
            stripe(len - 38.0, len - 8.0, off, 0.9);
        }
        let mut at = 150.0;
        while at + 30.0 <= len - 150.0 {
            stripe(at, at + 30.0, 0.0, 0.45);
            at += 50.0;
        }
        // The control tower by the front corner.
        let t = b.tower().centre();
        slab.block(
            Vec3::new(t.x - 6.0, g, t.y - 6.0),
            Vec3::new(t.x + 6.0, g + 36.0, t.y + 6.0),
            wall_tone,
        );
        slab.block(
            Vec3::new(t.x - 9.0, g + 36.0, t.y - 9.0),
            Vec3::new(t.x + 9.0, g + 44.0, t.y + 9.0),
            roof_tone,
        );
        // A terminal inside each bay's mouth: a post with a lit screen facing the apron.
        for t in b.terminals(cfg) {
            let (x, z) = (t.at.x, t.at.y);
            slab.block(Vec3::new(x - 0.6, g, z - 0.3), Vec3::new(x + 0.6, g + 1.1, z + 0.3), roof_tone);
            slab.block(
                Vec3::new(x - 0.8, g + 1.1, z - 0.15),
                Vec3::new(x + 0.8, g + 2.2, z + 0.35),
                roof_tone,
            );
            let y0 = g + 1.2;
            screens.quad(
                [
                    Vec3::new(x - 0.7, y0, z - 0.17),
                    Vec3::new(x + 0.7, y0, z - 0.17),
                    Vec3::new(x + 0.7, y0 + 0.9, z - 0.17),
                    Vec3::new(x - 0.7, y0 + 0.9, z - 0.17),
                ],
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        commands.spawn((
            Name::new(format!("air base {} terminal screens", b.id)),
            Mesh3d(meshes.add(screens.build())),
            MeshMaterial3d(screen.clone()),
            Transform::IDENTITY,
        ));
        commands.spawn((
            Name::new(format!("air base {}", b.id)),
            Mesh3d(meshes.add(slab.build_with_tangents())),
            MeshMaterial3d(concrete.clone()),
            Transform::IDENTITY,
        ));
        commands.spawn((
            Name::new(format!("air base {} markings", b.id)),
            Mesh3d(meshes.add(marks.build())),
            MeshMaterial3d(paint.clone()),
            Transform::IDENTITY,
        ));
    }
    info!("air bases: {} drawn", plan.0.bases.len());
}

/// A piloted mech that stops inside a hangar bay is repaired (every part not lost back to full) and
/// rearmed (every weapon's rounds back to its full load), free, once each time it walks in; the base becomes
/// the career's current base and the MECH MFD shows the report.
fn repair_in_hangars(
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time<Fixed>>,
    mut mechs: Query<(&mut MechSim, &MechSpec), With<Piloted>>,
    mut last: ResMut<LastService>,
    mut career: Option<ResMut<crate::career::CareerRun>>,
    mut inside: Local<Option<(u32, usize)>>,
) {
    let cfg = &data.0.world.bases;
    last.since_s += time.delta_secs();
    for (mut sim, spec) in &mut mechs {
        let at = hangar_at(&plan.0.bases, cfg, Vec2::new(sim.curr.pos.x, sim.curr.pos.z));
        if at.is_none() {
            *inside = None;
            continue;
        }
        if *inside == at || !sim.curr.is_stopped(&spec.locomotion) || sim.dead {
            continue;
        }
        *inside = at;
        let Some((base, bay)) = at else { continue };
        let sim = &mut *sim;
        let done = service(&mut sim.damage, &spec.parts, &mut sim.guns.rounds_left, &spec.weapons);
        info!("air base {base}, bay {bay}: {}", done.lines().join(", "));
        last.last = Some((base, bay, done.lines()));
        last.since_s = 0.0;
        if let Some(run) = career.as_deref_mut()
            && run.career.base != base
        {
            run.career.base = base;
            crate::career::save(&run.career);
        }
    }
}

/// Finds the terminal the pilot on foot stands at and looks at, for the prompt's options (`interact`).
fn find_terminal(plan: Res<Plan>, data: Res<Data>, pilot: Res<PilotSim>, mut near: ResMut<TerminalNear>) {
    let p = &pilot.curr;
    near.near = (p.mode == PilotMode::OnFoot && p.waiting.is_none())
        .then(|| terminal_in_reach(&plan.0.bases, &data.0.world.bases, Vec2::new(p.feet.x, p.feet.z), p.yaw_rad))
        .flatten();
}

/// Use with the terminal selected (`interact`, survey W40) asks for its screen; the mech parked in the same bay is
/// boarded only when it is the one selected instead.
fn use_terminal(choice: Res<crate::interact::UseChoice>, mut near: ResMut<TerminalNear>) {
    if choice.now == Some(cb_core::interact::Interaction::Terminal) && near.near.is_some() {
        near.wanted = near.near;
    }
}

/// Opens the terminal's screen when the pilot asked for it.
fn open_terminal(
    mut commands: Commands,
    mut near: ResMut<TerminalNear>,
    mut menu: ResMut<crate::menu::Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut cursor: Single<&mut bevy::window::CursorOptions>,
) {
    let Some(t) = near.wanted.take() else { return };
    if menu.is_open() {
        return;
    }
    info!("TERMINAL base {} bay {} opened", t.base, t.bay);
    commands.insert_resource(AtTerminal(t));
    crate::menu::open(&mut menu, crate::menu::Menu::Terminal, &mut time, &mut cursor);
}
