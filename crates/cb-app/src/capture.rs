//! The capture instrument (`pilot-boarding` design section 4; CLAUDE.md section 4 allows it as a
//! measurement instrument): `--stills <dir>` writes stills from the street and the cockpit, logs
//! frame time over a stride, looks up at the clouds and jets into one for the rain on the glass
//! (`cloud-puffs`, `cloud-rain`), then exits. It only writes input actions, as a player would,
//! except for placing the pilot at the start of the street shot. It moves into the scripted pilot
//! (`playtest-pipeline`) when that lands.
//!
//! With `CB_STILLS_BUILDINGS=1` it shoots the destructible buildings instead
//! (`structural-destruction`): the pilot stands in the street 11 m off a mid-rise near the start, it is
//! shot whole, then the pilot presses F9 (drop a shell at the aim point, a key every player has)
//! at the foot of each column on that side, and it is shot as the frame creaks, falls and leaves
//! its rubble.
//! The pilot is placed and turned directly (as for the street shot); everything that breaks,
//! breaks through the game's own shell.

use crate::input::InputAccum;
use crate::mech::{MechAccessNow, MechSim, Piloted};
use crate::pilot::PilotSim;
use crate::showcase::{fly_to_cloud, nearest_cloud};
use crate::sim::Data;
use crate::state::GameApp;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use cb_core::input::Actions;
use cb_core::pilot::{PilotMode, PilotState};
use std::path::PathBuf;

/// Runs the capture script.
pub struct StillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    frames_ms: Vec<f32>,
    cloud: Vec3,
    in_cloud_s: f32,
    shot_near: bool,
    shot_climb: bool,
    start_y_m: f32,
    /// The building variant's demolition (`demolition`), once its building is picked.
    buildings: Option<crate::demolition::Demolition>,
    /// The street tour before the demolition: how many of its shots are taken.
    tour_done: usize,
    /// When the current tour shot's dressing settled (seconds into the beat), if it has.
    tour_settled_s: Option<f32>,
    /// How many of the fall's timed shots are taken (each once its time has passed, so a slow
    /// renderer's long frames cannot step over one).
    fall_shots: usize,
    /// The offscreen image the building variant draws the pilot's view into (window shots come out
    /// black on macOS when the window is behind another; the bench's offscreen image does not).
    offscreen: Option<Handle<Image>>,
}

impl Plugin for StillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            frames_ms: Vec::new(),
            cloud: Vec3::ZERO,
            in_cloud_s: 0.0,
            shot_near: false,
            shot_climb: false,
            start_y_m: 0.0,
            buildings: None,
            tour_done: 0,
            tour_settled_s: None,
            fall_shots: 0,
            offscreen: None,
        })
        .add_after_world_spawn(offscreen_for_buildings)
        .add_game_systems(Update, run_script)
        .add_game_systems(Last, (hold_poster_hatch, poster_lens));
    }
}

/// The building variant's beats (see the module comment).
#[allow(clippy::too_many_arguments)]
fn buildings_beats(
    commands: &mut Commands,
    script: &mut Script,
    acc: &mut InputAccum,
    pilot: &mut PilotSim,
    access: &MechAccessNow,
    plan: &crate::world::Plan,
    live: &crate::world::Live,
    settled: bool,
    in_beat: f32,
    next: &mut bool,
    exit: &mut MessageWriter<AppExit>,
) {
    use crate::demolition::{Demolition, Stage, place};
    // First a street tour, for comparing with the mockup: the nearest walk-up (up to 6 storeys),
    // office (7 to 14) and tower (15 and up) to the mech, each from across the street (24 m off
    // its -z face, eye height) once its dressing has loaded, then one from cockpit height.
    if script.beat == 1 && script.tour_done < 4 {
        // (The demolition's own beat 1 timing starts after the tour: its 2 s wait is long past.)
        let Some(a) = access.0 else { return };
        let here = Vec2::new(a.ladder_base.x, a.ladder_base.z);
        let kinds: [(&str, std::ops::RangeInclusive<u32>); 4] = [
            ("walkup", 2..=6),
            ("office", 7..=14),
            ("tower", 15..=80),
            ("cockpit-height", 2..=80),
        ];
        let (name, range) = kinds[script.tour_done].clone();
        let Some(b) = plan
            .0
            .buildings
            .iter()
            .filter(|b| range.contains(&b.storeys))
            .min_by(|p, q| p.footprint.centre().distance(here).total_cmp(&q.footprint.centre().distance(here)))
        else {
            script.tour_done += 1;
            return;
        };
        let ground = plan.0.cities[b.city as usize].ground_m;
        let c = b.footprint.centre();
        let eye_up = if name == "cockpit-height" { 7.9 } else { 0.0 };
        // Stand on the far side of the street in front of the -z face (the street's own rect).
        let street_z = plan.0.cities[b.city as usize]
            .streets
            .iter()
            .filter(|s| {
                s.rect.max.y <= b.footprint.min.y + 0.5 && s.rect.min.x <= c.x && s.rect.max.x >= c.x && s.rect.size().x > s.rect.size().y
            })
            .map(|s| s.rect.min.y + 3.0)
            .max_by(f32::total_cmp)
            .unwrap_or(b.footprint.min.y - 20.0);
        let feet = Vec3::new(c.x, ground + eye_up, street_z);
        let target = Vec3::new(c.x, ground + (b.height_m * 0.35).min(14.0), b.footprint.min.y);
        place(pilot, feet, target);
        // Each shot once the street around the pilot is dressed and has drawn for 3 s more (the
        // pilot is placed every frame, so the high one stays up). Dressing goes one building a
        // frame, so a slow renderer takes many seconds before the street is all there.
        if settled && script.tour_settled_s.is_none() {
            script.tour_settled_s = Some(in_beat);
        }
        if script.tour_settled_s.is_some_and(|t| in_beat > t + 3.0) {
            shoot_image(commands, script, &format!("tour-{}-{name}.png", script.tour_done));
            println!("STILLS_TOUR {name} building={} storeys={}", b.id, b.storeys);
            script.tour_done += 1;
            script.tour_settled_s = None;
        }
        return;
    }
    // Then the demolition (`demolition`), shooting each stage.
    if script.buildings.is_none() {
        let Some(a) = access.0 else { return };
        let Some(d) = Demolition::pick(plan, live, Vec2::new(a.ladder_base.x, a.ladder_base.z)) else {
            exit.write(AppExit::error());
            return;
        };
        script.buildings = Some(d);
    }
    let stage = match script.beat {
        1 => Stage::Whole,
        2 => Stage::Shelling,
        3 => Stage::Fall,
        4 => Stage::Close,
        _ => {
            if in_beat > 1.5 {
                exit.write(AppExit::Success);
            }
            return;
        }
    };
    let mut demolition = script.buildings.take().expect("picked above");
    let over = demolition.play(stage, in_beat, plan, pilot, acc);
    script.buildings = Some(demolition);
    // The stills, each once its time in the stage has passed (so a slow renderer's long frames
    // cannot step over one).
    const FALL_SHOTS: [(f32, &str); 4] = [
        (1.0, "building-hit.png"),
        (2.2, "building-creak.png"),
        (5.0, "building-fall.png"),
        (12.0, "building-rubble.png"),
    ];
    match stage {
        Stage::Whole if over => shoot_image(commands, script, "building-whole.png"),
        Stage::Fall => {
            if let Some(&(t, name)) = FALL_SHOTS.get(script.fall_shots)
                && in_beat >= t
            {
                shoot_image(commands, script, name);
                script.fall_shots += 1;
            }
        }
        Stage::Close if over => shoot_image(commands, script, "building-rubble-close.png"),
        _ => {}
    }
    *next = over && (stage != Stage::Fall || script.fall_shots == FALL_SHOTS.len());
}

/// The poster variant's shots: (where the camera stands round the mech from straight ahead of it,
/// degrees; knelt and open; how far the view turns off the mech, degrees, so it sits to one side).
/// The stills script's beat for taking a job on the MAIL page (`CB_STILLS_MAIL`), out of the numbered run: entered
/// after the outline shot, it goes on to the stride.
const MAIL_BEAT: usize = 50;

const POSTER_SHOTS: [(f32, bool, f32); 2] = [(45.0, false, 13.0), (45.0, true, 13.0)];

/// How far the poster variant turns the mech from facing the pilot's start, degrees, so it stands
/// side-on in the frame.
pub const POSTER_MECH_TURN_DEG: f32 = -90.0;

/// How far the poster camera stands from the mech, metres (the 45 degree lens wants distance).
const POSTER_RANGE_M: f32 = 62.0;

/// The poster variant's lens: a 45 degree view, everything sharp (the city too), a vignette and a little
/// chromatic aberration. Runs last, after the settings set the player's lens.
fn poster_lens(mut commands: Commands, camera: Single<(Entity, &mut Projection), With<crate::pilot::PilotCamera>>, mut added: Local<bool>) {
    if !crate::cli::switched_off("CB_STILLS_POSTER") {
        return;
    }
    let (e, mut lens) = camera.into_inner();
    if let Projection::Perspective(p) = lens.as_mut() {
        p.fov = 45f32.to_radians();
    }
    use bevy::post_process::effect_stack::{ChromaticAberration, Vignette};
    if !*added {
        *added = true;
        commands.entity(e).insert((
            Vignette {
                intensity: 0.55,
                radius: 0.9,
                smoothness: 2.5,
                ..default()
            },
            ChromaticAberration {
                intensity: 0.004,
                ..default()
            },
        ));
    }
}

/// In the poster variant's knelt shots, holds the mech sat down with its chest open: the boarding
/// clock at "open" whatever the boarding rules would do with no pilot at the stair. Runs last, so
/// the frame drawn next shows it open.
fn hold_poster_hatch(script: Res<Script>, data: Res<Data>, mut mechs: Query<&mut MechSim>) {
    if !crate::cli::switched_off("CB_STILLS_POSTER") || script.beat == 0 {
        return;
    }
    let i = script.beat - 1;
    if !POSTER_SHOTS.get(i).is_some_and(|s| s.1) {
        return;
    }
    let tl = &data.0.boarding;
    for mut m in &mut mechs {
        m.hatch.target = cb_core::boarding::HatchTarget::Open;
        let open = m.hatch.target_s(tl);
        m.hatch.step(tl, 0.25);
        if m.hatch.t_s > open {
            m.hatch.t_s = open;
        }
    }
}

/// A still of the building variant's offscreen image.
fn shoot_image(commands: &mut Commands, script: &Script, name: &str) {
    match script.offscreen.clone() {
        Some(img) => shoot_offscreen(commands, img, &script.dir, name),
        None => shoot(commands, &script.dir, name),
    }
}

pub(crate) fn shoot(commands: &mut Commands, dir: &std::path::Path, name: &str) {
    let path = dir.join(name);
    info!("capture: {}", path.display());
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

/// The building variant draws the pilot's eye and the cockpit pass into an offscreen image.
fn offscreen_for_buildings(
    mut commands: Commands,
    camera: Single<Entity, With<crate::pilot::PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut script: ResMut<Script>,
) {
    if !crate::cli::switched_off("CB_STILLS_BUILDINGS") && !crate::cli::switched_off("CB_STILLS_POSTER") {
        return;
    }
    script.offscreen = Some(draw_offscreen(&mut commands, *camera, *cockpit, &mut images));
}

/// Draws the pilot's eye and the cockpit pass into a 1920 x 1080 offscreen image from now on, for stills that must not
/// depend on the window being in front (on macOS a window behind another saves black). The HUD is not in it.
pub(crate) fn draw_offscreen(commands: &mut Commands, camera: Entity, cockpit: Entity, images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::camera::RenderTarget;
    use bevy::render::render_resource::TextureFormat;
    let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb));
    let handle = images.add(image);
    commands.entity(camera).insert(RenderTarget::Image(handle.clone().into()));
    commands.entity(cockpit).insert(RenderTarget::Image(handle.clone().into()));
    handle
}

/// Saves the offscreen image `img` as `dir/name`.
pub(crate) fn shoot_offscreen(commands: &mut Commands, img: Handle<Image>, dir: &std::path::Path, name: &str) {
    let path = dir.join(name);
    info!("capture: {}", path.display());
    commands.spawn(Screenshot::image(img)).observe(save_to_disk(path));
}

#[allow(clippy::too_many_arguments)]
fn run_script(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut acc: ResMut<InputAccum>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    plan: Res<crate::world::Plan>,
    live: Res<crate::world::Live>,
    ready: Res<crate::loading::GameReady>,
    settled: Res<crate::buildings::DressingSettled>,
    enemies: Query<&GlobalTransform, With<crate::mech::Enemy>>,
    // The piloted mech's sim and the data: the boarding clock and its timeline for the shot of the seals venting
    // (`boarding-vents`), the mech's pose for the clouds (`cloud-puffs`).
    (mechs, data, mut steam_shot): (
        Query<&MechSim, With<Piloted>>,
        Res<Data>,
        // The steam shot is taken; frames drawn since the cabin sealed (the screens power up over 2.5 s of game,
        // 10 frames at the cap), counting on through the outline shot.
        Local<(bool, u32)>,
    ),
    // `CB_STILLS_MAIL`: where the view points on the MFDs, a career's board (none in free roam), whether the city
    // defence runs (free roam's mail), and how far through taking the offer the script is (its step, frames in it).
    (pointed, career, defending, mut mail): (
        Res<crate::mfd_apps::Pointed>,
        Option<Res<crate::career::CareerRun>>,
        Option<Res<crate::defence::Defending>>,
        Local<(u32, u32)>,
    ),
    time: Res<Time<Real>>,
    game_time: Res<Time<Virtual>>,
    mut exit: MessageWriter<AppExit>,
    mut exit_frames: Local<u32>,
) {
    // The building variant keeps the game's clock, so its beats keep pace with the fall however
    // slow the frames (a software renderer's frames are longer than the clock's 0.25 s cap).
    let buildings = crate::cli::switched_off("CB_STILLS_BUILDINGS");
    let (now, dt) = if buildings {
        (game_time.elapsed_secs(), game_time.delta_secs())
    } else {
        (time.elapsed_secs(), time.delta_secs())
    };
    if let Some(d) = script.buildings.as_ref()
        && script.beat >= 2
        && (now * 2.0) as u32 != ((now - dt) * 2.0) as u32
    {
        println!("STILLS_STANDING beat={} {}", script.beat, d.standing(&live));
    }
    let in_beat = now - script.beat_started_s;
    let mut next = false;
    if script.beat >= 1 && buildings {
        buildings_beats(
            &mut commands,
            &mut script,
            &mut acc,
            &mut pilot,
            &access,
            &plan,
            &live,
            settled.0,
            in_beat,
            &mut next,
            &mut exit,
        );
        if next {
            script.beat += 1;
            script.beat_started_s = now;
        }
        return;
    }
    // The poster variant (`CB_STILLS_POSTER=1`, with `--start overlook`): on foot downhill of the
    // mech, looking up at it with the city and the sun beyond, a few angles standing, then knelt with
    // its chest open (`hold_poster_hatch`), into the offscreen image.
    if script.beat >= 1 && crate::cli::switched_off("CB_STILLS_POSTER") {
        let i = script.beat - 1;
        if i >= POSTER_SHOTS.len() {
            // The last screenshot is written a frame or two later: on a software renderer, whose
            // frames take seconds, wait for the file itself (up to 30 s).
            let last = script.dir.join(match POSTER_SHOTS.last() {
                Some((_, true, _)) => format!("poster-{}-knelt.png", POSTER_SHOTS.len() - 1),
                _ => format!("poster-{}.png", POSTER_SHOTS.len() - 1),
            });
            if in_beat > 2.0 && (last.exists() || in_beat > 30.0) {
                exit.write(AppExit::Success);
            }
            return;
        }
        let (angle_deg, knelt, turn_deg) = POSTER_SHOTS[i];
        if access.0.is_none() {
            return;
        }
        let mech = plan.0.start.mech_xz;
        let ahead = (plan.0.start.pilot_xz - mech).normalize_or(Vec2::Y);
        if in_beat < 0.05 {
            let at = mech + Vec2::from_angle(angle_deg.to_radians()).rotate(ahead) * POSTER_RANGE_M;
            let feet = Vec3::new(at.x, plan.0.terrain.height_at(at.x, at.y), at.y);
            let to = Vec3::new(mech.x, feet.y, mech.y) - feet;
            let mut st = PilotState::standing(feet, (-to.x).atan2(-to.z) + turn_deg.to_radians());
            st.pitch_rad = if knelt { 0.06 } else { 0.13 };
            pilot.prev = st.clone();
            pilot.curr = st;
        }
        // Standing shots after 4 s; knelt ones once the mech has sat and opened (about 9 s).
        if in_beat > if knelt { 12.0 } else { 4.0 } {
            shoot_image(
                &mut commands,
                &script,
                &format!("poster-{i}{}.png", if knelt { "-knelt" } else { "" }),
            );
            script.beat += 1;
            script.beat_started_s = now;
        }
        return;
    }
    match script.beat {
        // Wait for the world, the rig and shaders to settle (GAME_READY), so no still is shot on
        // the loading screen or a half-built world (CLAUDE.md section 9).
        0 => next = ready.0 && in_beat > 6.0 && access.0.is_some(),
        // The street: the pilot 12 m behind the mech, looking up at it along the street.
        1 => {
            if in_beat < 0.05 {
                let a = access.0.unwrap();
                let feet = a.ladder_base + Vec3::new(-6.0, 0.0, 10.0);
                let to = a.ladder_base - feet;
                let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
                s.pitch_rad = 0.32;
                pilot.prev = s.clone();
                pilot.curr = s;
            }
            if in_beat > 1.5 {
                shoot(&mut commands, &script.dir, "street.png");
                next = true;
            }
        }
        // An enemy in its skin (`blockout-enemies`): the pilot stood 70 m off the nearest, looking up at it.
        // Skipped when there is none; then back behind the mech to board.
        2 if enemies.is_empty() => next = true,
        2 => {
            let a = access.0.unwrap();
            let e = enemies
                .iter()
                .map(|g| g.translation())
                .min_by(|p, q| p.distance(a.ladder_base).total_cmp(&q.distance(a.ladder_base)))
                .unwrap();
            if in_beat < 0.05 {
                let flat = Vec3::new(a.ladder_base.x - e.x, 0.0, a.ladder_base.z - e.z).normalize_or(Vec3::X);
                let feet = Vec3::new(e.x, a.ladder_base.y, e.z) + flat * 70.0;
                let to = e - feet;
                let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
                s.pitch_rad = 0.12;
                pilot.prev = s.clone();
                pilot.curr = s;
            }
            if in_beat > 2.0 {
                shoot(&mut commands, &script.dir, "enemy.png");
                let a = access.0.unwrap();
                let feet = a.ladder_base + Vec3::new(-6.0, 0.0, 10.0);
                let s = PilotState::standing(feet, 0.0);
                pilot.prev = s.clone();
                pilot.curr = s;
                next = true;
            }
        }
        // Walk to the ladder by input, then press Use.
        3 => {
            let a = access.0.unwrap();
            acc.script = Some(if pilot.curr.mode == PilotMode::OnFoot && pilot.curr.distance_to_stair(&a) > 2.0 {
                crate::showcase::walk_to(&pilot.curr, a.ladder_base, 0.0)
            } else {
                Actions {
                    use_pressed: pilot.curr.mode == PilotMode::OnFoot,
                    ..default()
                }
            });
            // The chest opening as its seals break, looked up at from the rope's foot: their steam (`boarding-vents`).
            // Only the view is turned (as the street shot sets it); where the pilot stands and their wait for the
            // hatch are left alone, so the boarding goes on.
            let tl = &data.0.boarding;
            let t = mechs.single().map_or(0.0, |m| m.hatch.t_s);
            let sealed = tl.sealed_sitting_s();
            if !steam_shot.0 && pilot.curr.mode == PilotMode::OnFoot && t >= sealed {
                let to = a.hatch_entry - (pilot.curr.feet + Vec3::Y * 1.7);
                pilot.curr.yaw_rad = (-to.x).atan2(-to.z);
                pilot.curr.pitch_rad = to.y.atan2(Vec2::new(to.x, to.z).length()).min(1.3);
                pilot.prev = pilot.curr.clone();
                if t >= sealed + 0.3 {
                    shoot(&mut commands, &script.dir, "hatch-steam.png");
                    steam_shot.0 = true;
                }
            }
            if matches!(pilot.curr.mode, PilotMode::Boarding(_)) {
                // The stair is down and the climb begins: the open hatch from the stair's foot.
                acc.script = Some(Actions::default());
                shoot(&mut commands, &script.dir, "hatch-open.png");
                next = true;
            }
        }
        4 => {
            // Seated; wait for the hatch to seal behind the pilot and the screens to power up: on the boarding
            // clock, not the wall clock, which a software renderer's long frames outrun (0.25 s of game a frame).
            let sealed = mechs.single().is_ok_and(|m| m.hatch.is_closed(&data.0.boarding));
            if pilot.curr.mode == PilotMode::Seated && sealed && in_beat > 12.0 && {
                steam_shot.1 += 1;
                steam_shot.1 > 12
            } {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Free look held, looking down at the dash's centre screen: the outline on the screen pointed at
        // (`mfd-menus`), by input as a player does it.
        5 => {
            let down = pilot.curr.pitch_rad > -0.6;
            acc.script = Some(Actions {
                free_look: true,
                look_delta_deg: if down { Vec2::new(0.0, -1.5) } else { Vec2::ZERO },
                ..default()
            });
            // Held on the screen a few frames first, so the outline built when it is first pointed at is drawn.
            if !down && in_beat > 2.0 && {
                steam_shot.1 += 1;
                steam_shot.1 > 18
            } {
                if crate::cli::switched_off("CB_STILLS_MAIL") {
                    steam_shot.1 = 0;
                    script.beat = MAIL_BEAT;
                    script.beat_started_s = now;
                    shoot(&mut commands, &script.dir, "cockpit-outline.png");
                    return;
                }
                shoot(&mut commands, &script.dir, "cockpit-outline.png");
                next = true;
            }
        }
        // `CB_STILLS_MAIL`: a job taken on the cockpit's MAIL page, as a player takes one in free roam (mission-jobs
        // 4.1): click the pointed screen (its MENU opens), aim at MAIL and click, aim at the first ACCEPT and click,
        // aim at BACK and click (the screen's own page again). Aiming is by look input until the row lights, as a
        // player's mouse does; a click is the press free look and fire make. A still at each step.
        MAIL_BEAT => {
            use crate::mfd_apps::{Item, MAIL_PAGE, MENU_PAGE, item_centre};
            let (step, frames) = &mut *mail;
            *frames += 1;
            let seek = |page: usize, item: Item| -> Vec2 {
                match (pointed.at, item_centre(page, item)) {
                    (Some(at), Some(c)) => ((c - at) * Vec2::new(0.02, -0.02)).clamp(Vec2::splat(-0.6), Vec2::splat(0.6)),
                    _ => Vec2::ZERO,
                }
            };
            let mut look = Vec2::ZERO;
            let mut press = false;
            let board = career.as_ref().map(|c| &c.career.board);
            let advance = |step: &mut u32, frames: &mut u32| {
                *step += 1;
                *frames = 0;
            };
            match *step {
                // Raise the view onto the screen (the outline beat stops a little under the dash's centre screen),
                // then click it.
                0 if pointed.screen.is_none() => look = Vec2::new(0.0, 0.4),
                0 if *frames > 4 => {
                    press = true;
                    advance(step, frames);
                }
                1 if *frames > 8 && pointed.page == Some(MENU_PAGE) => {
                    shoot(&mut commands, &script.dir, "mfd-menu.png");
                    advance(step, frames);
                }
                2 => {
                    look = seek(MENU_PAGE, Item::Mail);
                    if pointed.item == Some(Item::Mail) && *frames > 4 {
                        press = true;
                        advance(step, frames);
                    }
                }
                3 => {
                    // A career's job offered, or free roam's standing city defence.
                    let offered = board.is_none_or(|b| b.jobs.iter().any(|j| j.state == cb_core::jobs::JobState::Offered));
                    if *frames > 8 && pointed.page == Some(MAIL_PAGE) && offered {
                        shoot(&mut commands, &script.dir, "mfd-mail.png");
                        advance(step, frames);
                    }
                }
                4 => {
                    look = seek(MAIL_PAGE, Item::Accept(0));
                    if pointed.item == Some(Item::Accept(0)) && *frames > 4 {
                        press = true;
                        advance(step, frames);
                    }
                }
                5 if *frames > 8 && (defending.is_some() || board.is_some_and(|b| b.current().is_some())) => {
                    match board.and_then(|b| b.current()) {
                        Some(j) => println!(
                            "STILLS_MAIL accepted job {} at ({:.0}, {:.0}), {} mechs",
                            j.id, j.site.x, j.site.y, j.mechs
                        ),
                        None => println!("STILLS_MAIL accepted the city defence"),
                    }
                    shoot(&mut commands, &script.dir, "mfd-accepted.png");
                    advance(step, frames);
                }
                6 => {
                    look = seek(MAIL_PAGE, Item::Back);
                    if pointed.item == Some(Item::Back) && *frames > 4 {
                        press = true;
                        advance(step, frames);
                    }
                }
                7 if *frames > 8 && pointed.page.is_some_and(|p| p < MENU_PAGE) => {
                    println!("STILLS_MAIL back on page {:?}", pointed.page);
                    shoot(&mut commands, &script.dir, "mfd-back.png");
                    next = true;
                }
                _ => {}
            }
            if in_beat > 900.0 && !next {
                println!("STILLS_MAIL stuck at step {} (pointed {:?})", *step, *pointed);
                next = true;
            }
            // The aim goes in once a frame, as a mouse's does (`look_deg` is taken by the next fixed step only): held in
            // the script's actions it would turn the view again on every step of a long software-rendered frame.
            acc.script = Some(Actions {
                free_look: true,
                ..default()
            });
            acc.look_deg += look;
            acc.mfd_press |= press;
            if next {
                // On to the stride, as after the outline shot.
                script.beat = 6;
                script.beat_started_s = now;
                return;
            }
        }
        // Stand up and stride: throttle forward (the mech stands first), hold, measure frame time
        // over 10 s. The view comes back up to level first, by input.
        6 => {
            acc.script = Some(Actions {
                throttle_axis: if in_beat < 5.0 { 0.6 } else { 0.0 },
                look_delta_deg: if pilot.curr.pitch_rad < -0.02 {
                    Vec2::new(0.0, 1.5)
                } else {
                    Vec2::ZERO
                },
                ..default()
            });
            if in_beat > 7.0 {
                script.frames_ms.push(time.delta_secs() * 1000.0);
            }
            if in_beat > 17.0 {
                shoot(&mut commands, &script.dir, "cockpit-stride.png");
                next = true;
            }
        }
        // Stop and look up through the front screens at the clouds (cloud-puffs).
        7 => {
            let Some(m) = mechs.iter().next().map(|s| &s.curr) else { return };
            acc.script = Some(Actions {
                throttle_zero: in_beat < 0.1,
                look_delta_deg: Vec2::new(0.0, (0.3 - m.aim_pitch_rad).to_degrees() * 0.2),
                ..default()
            });
            if in_beat > 6.0 {
                shoot(&mut commands, &script.dir, "sky.png");
                script.cloud = nearest_cloud(&plan, m);
                script.start_y_m = m.pos.y;
                next = true;
            }
        }
        // Rockets up and skates across into the nearest cloud: one still 400 m up, clear of the
        // roofs, one on the way in, one after 2.5 s inside, when the glass is wet (cloud-rain). The
        // limit is in real seconds and generous: a software renderer runs the simulation slower.
        8 => {
            let Some(m) = mechs.iter().next().map(|s| &s.curr) else { return };
            acc.script = Some(fly_to_cloud(m, script.cloud));
            let eye = m.pos + Vec3::Y * data.0.mech.body.eye_height_m;
            if !script.shot_climb && m.pos.y - script.start_y_m > 400.0 {
                shoot(&mut commands, &script.dir, "climb.png");
                script.shot_climb = true;
            }
            if !script.shot_near && Vec2::new(eye.x, eye.z).distance(Vec2::new(script.cloud.x, script.cloud.z)) < 700.0 {
                shoot(&mut commands, &script.dir, "cloud-near.png");
                script.shot_near = true;
            }
            if plan.clouds.density_at(eye, now) > 0.05 {
                // Game time, as the glass wets by it.
                script.in_cloud_s += game_time.delta_secs();
            }
            if script.in_cloud_s > 2.5 || in_beat > 900.0 || m.boost.locked {
                info!("capture: {:.1} s in the cloud", script.in_cloud_s);
                shoot(&mut commands, &script.dir, "cloud-rain.png");
                next = true;
            }
        }
        // A few frames after the last shot, so its readback lands even at a software renderer's frame rate.
        9 if in_beat > 1.5 && {
            *exit_frames += 1;
            *exit_frames > 6
        } =>
        {
            let mut f = script.frames_ms.clone();
            f.sort_by(f32::total_cmp);
            println!(
                "FRAME_TIME_MS frames={} p50={:.2} p95={:.2} p99={:.2} max={:.2}",
                f.len(),
                percentile(&f, 0.5),
                percentile(&f, 0.95),
                percentile(&f, 0.99),
                f.last().copied().unwrap_or(0.0)
            );
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
    }
}

/// Frames from a showcase take, for checking effects headless (openspec change `mech-damage-fx` task 4.1; CLAUDE.md
/// section 4 allows it as an instrument): with `CB_FRAMES_DIR` set, the primary window is saved every
/// `CB_FRAMES_EVERY_S` seconds of game time (default 2) as `frame-<n>.png`, at most `CB_FRAMES_MAX` (default 90).
/// It saves pictures and nothing else; a take with frames saved is not a frame-time result (CLAUDE.md section 7).
/// With `CB_FRAMES_OFFSCREEN=1` the pilot's view is drawn into a 1920 x 1080 image and that is saved instead (on macOS a
/// window behind another saves black), without the HUD.
pub struct FramesPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Frames {
    dir: PathBuf,
    every_s: f32,
    max: u32,
    taken: u32,
    next_s: f32,
    offscreen: Option<Handle<Image>>,
}

impl Plugin for FramesPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        let env = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        app.insert_resource(Frames {
            dir: self.dir.clone(),
            every_s: env("CB_FRAMES_EVERY_S", 2.0).max(0.1),
            max: env("CB_FRAMES_MAX", 90.0) as u32,
            taken: 0,
            next_s: 0.0,
            offscreen: None,
        })
        .add_after_world_spawn(offscreen_for_frames)
        .add_game_systems(Update, save_frames);
    }
}

/// `CB_FRAMES_OFFSCREEN=1`: the pilot's eye and the cockpit pass draw into an offscreen image.
fn offscreen_for_frames(
    mut commands: Commands,
    camera: Single<Entity, With<crate::pilot::PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut frames: ResMut<Frames>,
) {
    if !crate::cli::switched_off("CB_FRAMES_OFFSCREEN") {
        return;
    }
    frames.offscreen = Some(draw_offscreen(&mut commands, *camera, *cockpit, &mut images));
}

fn save_frames(mut commands: Commands, mut frames: ResMut<Frames>, time: Res<Time>, ready: Option<Res<crate::loading::GameReady>>) {
    if !ready.is_some_and(|r| r.0) || frames.taken >= frames.max {
        return;
    }
    let now = time.elapsed_secs();
    if now < frames.next_s {
        return;
    }
    frames.next_s = now + frames.every_s;
    let name = format!("frame-{:03}.png", frames.taken);
    frames.taken += 1;
    match frames.offscreen.clone() {
        Some(img) => {
            commands.spawn(Screenshot::image(img)).observe(save_to_disk(frames.dir.join(name)));
        }
        None => shoot(&mut commands, &frames.dir.clone(), &name),
    }
}

/// The effects, staged in front of the eye for headless stills (openspec changes `fx-sprites`, `blast-scorch`,
/// `rocket-plumes`, `mech-damage-fx`; their stills tasks; an instrument, CLAUDE.md section 4): with `--stills <dir>` and
/// `CB_STILLS_FX=1`, the pilot stands on foot 70 m from the nearest enemy and the player's own guns are fired at it as
/// real shots (rockets and autocannon rounds, resolved by `combat`), then the enemy is worn down by the damage rule's
/// own state (critical parts, a leg lost), then the pilot faces open soil where real explosions dig craters. Stills at
/// each stage; then it exits. It writes no input and plays no game: it only stages what a fight would make.
pub struct FxStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct FxScript {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    enemy: Option<Entity>,
    soil: Option<Vec3>,
    /// What this beat has done already: a frame is about 0.25 s of game time on a software renderer, so each step runs
    /// once, the first frame it is due, never in a narrow time window.
    done: Vec<&'static str>,
}

impl FxScript {
    /// True the first time `step` is asked for in this beat.
    fn once(&mut self, step: &'static str) -> bool {
        if self.done.contains(&step) {
            return false;
        }
        self.done.push(step);
        true
    }
}

impl Plugin for FxStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(FxScript {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            enemy: None,
            soil: None,
            done: Vec::new(),
        })
        .add_game_systems(Update, (run_fx_script, run_depth_bench, run_tower_corner));
    }
}

/// The tower corner stills (`CB_STILLS_FX=tower`, owner, 2026-10-28: "Glass towers aren't joining at the
/// corners", "screenshots of fix"): the pilot stands on the ground off a corner of the glass tower (15
/// storeys and up) nearest their mech, on the diagonal, and looks up the corner from three distances:
/// `tower-near.png` (12 m), `tower-mid.png` (25 m), `tower-far.png` (60 m). A measurement instrument: it
/// writes the stills and exits.
fn run_tower_corner(
    mut commands: Commands,
    script: Res<FxScript>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    time: Res<Time>,
    plan: Res<crate::world::Plan>,
    mut exit: MessageWriter<AppExit>,
    mut state: Local<Option<(f32, Vec3, Vec2, Vec<&'static str>)>>,
) {
    use cb_core::world::Ground;
    if script.beat != usize::MAX - 1 {
        return;
    }
    let now = time.elapsed_secs();
    let Some(home) = access.0.map(|a| a.ladder_base) else { return };
    if state.is_none() {
        let clear = |p: Vec2| plan.collision.push_out_at(p, 2.0, plan.height_at(p.x, p.y)).distance(p) < 1e-3;
        let mut towers: Vec<_> = plan.buildings.iter().filter(|b| b.storeys >= 15).collect();
        towers.sort_by(|a, b| {
            let d = |r: &cb_core::world::Building| r.footprint.min.distance(home.xz());
            d(a).total_cmp(&d(b)).then(a.id.cmp(&b.id))
        });
        for b in towers {
            let (lo, hi) = (b.footprint.min, b.footprint.max);
            for c in [lo, Vec2::new(hi.x, lo.y), Vec2::new(lo.x, hi.y), hi] {
                let out = (c - (lo + hi) * 0.5).signum() * std::f32::consts::FRAC_1_SQRT_2;
                if [12.0, 25.0, 60.0].iter().all(|m| clear(c + out * *m)) {
                    info!("tower corner: building {} ({} storeys) corner {c:?}", b.id, b.storeys);
                    *state = Some((now, Vec3::new(c.x, plan.height_at(c.x, c.y), c.y), out, Vec::new()));
                    break;
                }
            }
            if state.is_some() {
                break;
            }
        }
        if state.is_none() {
            eprintln!("tower corner: no glass tower with an open corner");
            exit.write(AppExit::error());
            return;
        }
    }
    let Some((start_s, corner, out, done)) = state.as_mut() else {
        return;
    };
    let t = now - *start_s;
    let shots = [
        ("tower-near.png", 12.0, 0.0),
        ("tower-mid.png", 25.0, 4.0),
        ("tower-far.png", 60.0, 8.0),
    ];
    let Some(&(_, dist, _)) = shots.iter().rev().find(|s| t >= s.2) else {
        return;
    };
    let p = corner.xz() + *out * dist;
    let feet = Vec3::new(p.x, plan.height_at(p.x, p.y), p.y);
    stand_looking(&mut pilot, feet, *corner + Vec3::Y * (6.0 + dist * 0.3));
    for (name, _, at) in shots {
        if t > at + 3.0 && !done.contains(&name) {
            done.push(name);
            shoot(&mut commands, &script.dir, name);
        }
    }
    if t > 12.0 {
        exit.write(AppExit::Success);
    }
}

/// One test of the particle depth bench (openspec change `particle-depth`): a recipe, where it plays relative
/// to the pilot (metres ahead, metres up), and whether it is a moving stream rather than a one-off.
struct DepthTest {
    name: &'static str,
    recipe: &'static str,
    ahead_m: f32,
    up_m: f32,
    stream: bool,
}

const DEPTH_TESTS: [DepthTest; 5] = [
    DepthTest {
        name: "ground-near",
        recipe: "explosion_ground",
        ahead_m: 15.0,
        up_m: 0.0,
        stream: false,
    },
    DepthTest {
        name: "ground-far",
        recipe: "explosion_ground",
        ahead_m: 450.0,
        up_m: 0.0,
        stream: false,
    },
    DepthTest {
        name: "trail",
        recipe: "jet_trail",
        ahead_m: 120.0,
        up_m: 30.0,
        stream: true,
    },
    DepthTest {
        name: "skate",
        recipe: "skate_dirt",
        ahead_m: 40.0,
        up_m: 0.5,
        stream: true,
    },
    DepthTest {
        name: "sky",
        recipe: "explosion_air",
        ahead_m: 300.0,
        up_m: 300.0,
        stream: false,
    },
];

/// Seconds each depth test runs; stills at 0.5 s and 3 s from one heading, and at 3.5 s from 20 degrees round.
const DEPTH_TEST_S: f32 = 4.5;

/// The particle depth bench (`CB_STILLS_FX=depth`, openspec change `particle-depth`): the pilot stands by their
/// mech and each test plays in turn in front of them; a stream test's emitter crosses the view at 30 m/s. A
/// measurement instrument: it writes stills (`depth-<test>-<shot>.png`) and exits, and says nothing of frame time.
#[allow(clippy::too_many_arguments)]
fn run_depth_bench(
    mut commands: Commands,
    script: Res<FxScript>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    time: Res<Time>,
    plan: Res<crate::world::Plan>,
    mut fx: MessageWriter<crate::fx::Fx>,
    mut streams: Query<(&mut Transform, &mut crate::fx::FxStream)>,
    mut exit: MessageWriter<AppExit>,
    mut state: Local<Option<(f32, usize, Vec<&'static str>, Option<Entity>)>>,
) {
    use cb_core::world::Ground;
    if script.beat != usize::MAX {
        return;
    }
    let now = time.elapsed_secs();
    let (start_s, test, done, emitter) = state.get_or_insert((now, 0, Vec::new(), None));
    let Some(home) = access.0.map(|a| a.ladder_base) else { return };
    // Stand 20 m off the mech's ladder on the heading with the longest run of open ground (no building within
    // 4 m of the line, out to 450 m), looking along it.
    let clear = |p: Vec3| plan.collision.push_out_at(p.xz(), 4.0, plan.height_at(p.x, p.z)).distance(p.xz()) < 1e-3;
    let (heading, _) = (0..32)
        .map(|i| {
            let a = i as f32 / 32.0 * std::f32::consts::TAU;
            let d = Vec3::new(a.cos(), 0.0, a.sin());
            let run = (4..=450).step_by(5).take_while(|&m| clear(home + d * m as f32)).last().unwrap_or(0);
            (a, run)
        })
        .max_by_key(|&(_, run)| run)
        .unwrap_or((0.0, 0));
    let along = Vec3::new(heading.cos(), 0.0, heading.sin());
    let side = Vec3::new(-along.z, 0.0, along.x);
    let p = home + along * 20.0;
    let feet = Vec3::new(p.x, plan.height_at(p.x, p.z), p.z);
    let t = now - *start_s;
    let Some(d) = DEPTH_TESTS.get(*test) else {
        exit.write(AppExit::Success);
        return;
    };
    let turn = if t > 3.2 { 20f32.to_radians() } else { 0.0 };
    let fwd = along * turn.cos() + side * turn.sin();
    let target = |ahead: f32| {
        let p = feet + along * ahead;
        Vec3::new(p.x, plan.height_at(p.x, p.z), p.z)
    };
    let at = target(d.ahead_m) + Vec3::Y * d.up_m;
    stand_looking(&mut pilot, feet, feet + fwd * d.ahead_m + Vec3::Y * (at.y - feet.y + 1.7));
    if !done.contains(&"start") {
        done.push("start");
        if d.stream {
            *emitter = Some(
                commands
                    .spawn((
                        Name::new("depth bench emitter"),
                        Transform::from_translation(at - side * 60.0),
                        crate::fx::FxStream {
                            recipe: d.recipe,
                            dir: Vec3::NEG_Y,
                            on: true,
                            density: 1.0,
                        },
                    ))
                    .id(),
            );
        } else {
            fx.write(crate::fx::Fx::new(d.recipe, at));
        }
    }
    if let Some(Ok((mut tf, _))) = emitter.map(|e| streams.get_mut(e)) {
        tf.translation = at + side * (t * 30.0 - 60.0);
    }
    for (when, shot) in [(0.5, "a"), (3.0, "b"), (3.9, "turned")] {
        if t > when && !done.contains(&shot) {
            done.push(shot);
            shoot(&mut commands, &script.dir, &format!("depth-{}-{}.png", d.name, shot));
        }
    }
    if t > DEPTH_TEST_S {
        if let Some(e) = emitter.take()
            && let Ok((_, mut s)) = streams.get_mut(e)
        {
            s.on = false;
        }
        *test += 1;
        *start_s = now;
        done.clear();
    }
}

/// The pilot on foot at `feet`, looking at `at`.
pub(crate) fn stand_looking(pilot: &mut PilotSim, feet: Vec3, at: Vec3) {
    let to = at - feet;
    let mut s = PilotState::standing(feet, (-to.x).atan2(-to.z));
    s.pitch_rad = (to.y - 1.7).atan2(to.with_y(0.0).length());
    pilot.prev = s.clone();
    pilot.curr = s;
}

#[allow(clippy::too_many_arguments)]
fn run_fx_script(
    mut commands: Commands,
    mut script: ResMut<FxScript>,
    mut pilot: ResMut<PilotSim>,
    access: Res<MechAccessNow>,
    time: Res<Time>,
    plan: Res<crate::world::Plan>,
    mut shots: ResMut<crate::combat::Shots>,
    mut explosions: MessageWriter<crate::blast::Explosion>,
    mut mechs: Query<(
        Entity,
        &crate::mech::MechId,
        &crate::mech::MechSpec,
        &mut crate::mech::MechSim,
        &GlobalTransform,
    )>,
    mut exit: MessageWriter<AppExit>,
    ready: Option<Res<crate::loading::GameReady>>,
) {
    use cb_core::world::Ground;
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    let mut next = false;
    let enemy_at = |mechs: &Query<(
        Entity,
        &crate::mech::MechId,
        &crate::mech::MechSpec,
        &mut crate::mech::MechSim,
        &GlobalTransform,
    )>,
                    e: Option<Entity>| { e.and_then(|e| mechs.get(e).ok()).map(|m| m.4.translation()) };
    match script.beat {
        0 => {
            next = ready.is_some_and(|r| r.0) && access.0.is_some() && t > 4.0;
            // `CB_STILLS_FX=soil` stages the blasts on soil alone, with a still every half second.
            if next && std::env::var("CB_STILLS_FX").is_ok_and(|v| v == "soil") {
                script.beat = 2;
            }
            // `CB_STILLS_FX=tower` looks at the corner of the nearest glass tower (`run_tower_corner`) alone.
            if next && std::env::var("CB_STILLS_FX").is_ok_and(|v| v == "tower") {
                script.beat = usize::MAX - 1;
                return;
            }
            // `CB_STILLS_FX=depth` is the particle depth bench (`run_depth_bench`) alone.
            if next && std::env::var("CB_STILLS_FX").is_ok_and(|v| v == "depth") {
                script.beat = usize::MAX;
                return;
            }
            if next {
                // The nearest live enemy to the player's mech.
                let home = access.0.unwrap().ladder_base;
                script.enemy = mechs
                    .iter()
                    .filter(|m| m.1.0 != 0 && !m.3.dead)
                    .min_by(|a, b| a.4.translation().distance(home).total_cmp(&b.4.translation().distance(home)))
                    .map(|m| m.0);
                if script.enemy.is_none() {
                    eprintln!("fx stills: no enemy");
                    exit.write(AppExit::error());
                }
            }
        }
        // Stand 70 m off the enemy, then fire the player's own rockets and rounds at it.
        1 => {
            let Some(e) = enemy_at(&mechs, script.enemy) else { return };
            // Down the street from the enemy toward the player's mech: an open line of sight in the city.
            let home = access.0.map_or(e + Vec3::X, |a| a.ladder_base);
            let flat = (home - e).with_y(0.0).normalize_or(Vec3::X);
            let feet = Vec3::new(e.x, plan.height_at(e.x + flat.x * 70.0, e.z + flat.z * 70.0), e.z) + flat * 70.0;
            let torso = e + Vec3::Y * 14.0;
            // `CB_STILLS_FX=jets` (with `CB_ENEMY_STUNT=hover`): keep looking at the enemy as it hovers on
            // its rockets, a still every second, to see whether its jet flame and smoke draw.
            let jets = std::env::var("CB_STILLS_FX").is_ok_and(|v| v == "jets");
            if script.once("stand") || jets {
                stand_looking(&mut pilot, feet, torso);
            }
            if jets {
                let k = t as u32;
                if (1..=14).contains(&k) && !script.dir.join(format!("jets-{k:02}.png")).exists() {
                    shoot(&mut commands, &script.dir, &format!("jets-{k:02}.png"));
                }
                if t > 15.0 {
                    exit.write(AppExit::Success);
                }
            }
            if t > 1.0 && script.once("fire") {
                let player = mechs.iter().find(|m| m.1.0 == 0).map(|m| m.2.0.clone());
                if let Some(spec) = player {
                    let launch = feet + Vec3::new(-flat.z, 0.0, flat.x) * 25.0 + Vec3::Y * 12.0;
                    for (wi, w) in spec.weapons.iter().enumerate() {
                        let n = if w.shot().straight() {
                            3
                        } else if w.blast_mj < 1.0 {
                            12
                        } else {
                            2
                        };
                        for k in 0..n {
                            let aim = torso + Vec3::new((k as f32 - 5.0) * 0.8, (k % 4) as f32 * 3.0 - 6.0, 0.0);
                            let from = launch + Vec3::Y * k as f32 * 0.6 - flat * (k as f32 * if w.shot().straight() { 25.0 } else { 0.0 });
                            shots.0.push(cb_core::weapons::Projectile {
                                owner: 0,
                                weapon: wi,
                                pos: from,
                                vel: (aim - from).normalize() * w.speed_m_s,
                                age_s: 0.0,
                                rocket: w.shot().straight(),
                                target: None,
                            });
                        }
                    }
                }
            }
            if t > 1.5 && script.once("rockets") {
                shoot(&mut commands, &script.dir, "fx-1-rockets.png");
            }
            if t > 5.0 && !jets {
                shoot(&mut commands, &script.dir, "fx-2-hits.png");
                next = true;
            }
        }
        // Wear it down: every part to a fifth (they smoke), the hull under a quarter (it burns), the left leg off.
        2 => {
            if script.once("wear")
                && let Some(e) = script.enemy
                && let Ok((_, _, spec, mut sim, _)) = mechs.get_mut(e)
            {
                let parts = spec.parts.clone();
                for (i, p) in parts.iter().enumerate() {
                    sim.damage.structure[i] = p.structure * 0.18;
                    if p.kind == cb_core::mech::damage::PartKind::LegL {
                        sim.damage.structure[i] = 0.0;
                        sim.damage.lost[i] = true;
                        sim.cuts.push(i);
                    }
                }
            }
            if t > 3.0 && script.once("damaged") {
                shoot(&mut commands, &script.dir, "fx-3-damaged.png");
            }
            if t > 9.0 {
                shoot(&mut commands, &script.dir, "fx-4-burning.png");
                next = true;
            }
        }
        // Open soil: the nearest point from the pilot that is soil, 50 m in front; blasts there.
        3 => {
            if script.once("soil") {
                let from = pilot.curr.feet;
                let mut found = None;
                'search: for ring in 1..80 {
                    for k in 0..24 {
                        let a = k as f32 / 24.0 * std::f32::consts::TAU;
                        let p = Vec2::new(from.x, from.z) + Vec2::new(a.cos(), a.sin()) * ring as f32 * 15.0;
                        if plan.0.surface_at(p) == cb_core::world::Surface::Dirt
                            && plan.0.surface_at(p + Vec2::X * 30.0) == cb_core::world::Surface::Dirt
                            && plan.0.surface_at(p - Vec2::X * 30.0) == cb_core::world::Surface::Dirt
                            && plan.water_at(p.x, p.y).is_none()
                        {
                            found = Some(Vec3::new(p.x, plan.height_at(p.x, p.y), p.y));
                            break 'search;
                        }
                    }
                }
                script.soil = found;
                if let Some(c) = found {
                    // Close and looking down into it, so the scorch shows over the grass.
                    let feet = c + Vec3::new(18.0, 0.0, 10.0);
                    let feet = feet.with_y(plan.height_at(feet.x, feet.z));
                    stand_looking(&mut pilot, feet, c);
                    explosions.write(crate::blast::Explosion {
                        at: c + Vec3::Y * 0.5,
                        energy_mj: 6.0,
                        by: None,
                    });
                    explosions.write(crate::blast::Explosion {
                        at: c + Vec3::new(-14.0, 0.5, -10.0),
                        energy_mj: 25.0,
                        by: None,
                    });
                } else {
                    eprintln!("fx stills: no open soil near");
                }
            }
            if t > 0.5 && script.once("blast") {
                shoot(&mut commands, &script.dir, "fx-5-blast.png");
            }
            if std::env::var("CB_STILLS_FX").is_ok_and(|v| v == "soil") {
                let k = (t * 2.0) as u32;
                if k > 0 && k <= 24 && !script.dir.join(format!("soil-{k:02}.png")).exists() {
                    let name = format!("soil-{k:02}.png");
                    shoot(&mut commands, &script.dir, &name);
                }
            }
            if t > 4.0 && script.once("smoke") {
                shoot(&mut commands, &script.dir, "fx-6-smoke.png");
            }
            if t > 30.0 {
                shoot(&mut commands, &script.dir, "fx-7-scorch.png");
                next = true;
            }
        }
        4 if t > 3.0 => {
            println!("FX_STILLS done");
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
        script.done.clear();
    }
}

/// The weather, shot from named views for headless stills (openspec change `cloud-weather`, task
/// 3.1; `docs/design/world-look-plan.md` step 1; an instrument, CLAUDE.md section 4): with
/// `--stills <dir>` and `CB_STILLS_WEATHER=1`, once the game is ready the pilot's eye is put at each
/// view in turn (held there, on foot, looking at its target), the ground and the sky are given time
/// to stream in round it, and one still is saved per view as `weather-<name>.png`; then it exits.
/// The views are the island mockup's (`island-biomes`) and two of a storm.
/// `CB_STILLS_ONLY=<name>,<name>` shoots only those views. It writes no input and plays no game.
/// They move into a data file shared with the mockup when step 1 of
/// `docs/design/world-look-plan.md` lands.
pub struct WeatherStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

/// A view: a name, where the eye is (x, y, z) and what it looks at (x, y, z). With
/// `above_ground`, the eye's y is metres above the ground or the sea under it; otherwise it is a
/// height, held at least 2 m above the ground.
struct View {
    name: &'static str,
    eye: [f32; 3],
    at: [f32; 3],
    above_ground: bool,
}

const fn view(name: &'static str, eye: [f32; 3], at: [f32; 3]) -> View {
    View {
        name,
        eye,
        at,
        above_ground: false,
    }
}

/// The views (x east, z south, metres): the island mockup's named views (`docs/mockups/island.html`,
/// `VIEWS`), worked out from its generator for seed 42, so a still of the game and a shot of the
/// mockup can be laid side by side; and two of the storm over the jungle at (-5.9, -4.2) km.
const VIEWS: &[View] = &[
    // The largest city at the hour (day-night-cycle): down its middle street 2 m up, and from 1,500 m
    // up 3.5 km off (placed from the plan: `city_view`).
    View {
        name: "city-street",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    View {
        name: "city-air",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // A dune suburb from 900 m, where the ground is drawn at step 2: its streets lie on that ground
    // (lot-access task 2.3; seed 42's street that floated highest, 2.7 m, measured 2026-10-28).
    View {
        name: "suburb-far",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    view("overview", [6000.0, 22000.0, 26000.0], [0.0, 0.0, 0.0]),
    view("range", [-8811.0, 1900.0, 4558.0], [189.0, 1466.0, -442.0]),
    view("desert", [3300.0, 862.0, 7300.0], [4200.0, 476.0, 6650.0]),
    view("canyon", [7758.0, 407.0, 3585.0], [8828.0, 288.0, 3598.0]),
    view("jungle", [-8000.0, 323.0, 4000.0], [-7100.0, 541.0, 3600.0]),
    view("cliffs", [10578.0, 70.0, -8876.0], [9174.0, 60.0, -7698.0]),
    view("islets", [-9348.0, 420.0, 8074.0], [-11548.0, 0.0, 10274.0]),
    view("coast", [13421.0, 380.0, 7136.0], [11838.0, 0.0, 2951.0]),
    view("storm", [-3387.0, 205.0, 8814.0], [-5887.0, 2600.0, -4186.0]),
    view("weather", [-24000.0, 6500.0, 26000.0], [-9000.0, 1500.0, -2000.0]),
    // Along the longest river from 40 m over its bank, downstream (placed from the plan: `river_view`).
    View {
        name: "river",
        eye: [0.0, 40.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: true,
    },
    // The steepest river reach that is not a waterfall, from 250 m to its side and from above
    // (placed from the plan: `steep_river_view`).
    View {
        name: "river-steep",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    View {
        name: "river-steep-far",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    View {
        name: "river-steep-top",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // Up close on the steepest reach from beside it and from downhill, the eye lower than the water
    // (the case that drew high rivers black: `close_river_view`).
    View {
        name: "river-close",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    View {
        name: "river-close-below",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // 8 m over the water off the north-east cliffs, looking at them (the reflection trace).
    View {
        name: "cliffs-low",
        eye: [9800.0, 8.0, -8200.0],
        at: [9174.0, 30.0, -7698.0],
        above_ground: true,
    },
    // In the west storm's rain, on the ground, looking up through it (the shafts seen from inside).
    View {
        name: "in-the-rain-up",
        eye: [-5600.0, 2.0, -3900.0],
        at: [-5900.0, 900.0, -4300.0],
        above_ground: true,
    },
    // Inside the cloud nearest a city's towers, looking up through it (`city_in_cloud_view`).
    View {
        name: "city-in-cloud",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // Open sea in the clear south-east, 40 m up, looking toward the sun (its glitter path and the
    // light through the crests) and away from it (the sky in the water): `sea_view`.
    View {
        name: "sea-sun",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    View {
        name: "sea-away",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // The same toward the sun from 1.5 km and 6.5 km up (the glitter and the haze from altitude).
    View {
        name: "sea-sun-high",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    View {
        name: "sea-sun-top",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // From the street in the biggest city, looking up past the towers at the sky (`city_up_view`).
    View {
        name: "city-up",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // Mist in the low ground, from a hill over the wet west at dawn-like low height.
    view("valley-mist", [-9000.0, 260.0, -1000.0], [-6000.0, 120.0, 2500.0]),
    // In the west storm's base at 650 m, 2.5 km north-west of its centre, flying south-east toward the sun (south-east,
    // 43 degrees up, at the top of the view): where the view flashed white among the puffs on 2026-10-27
    // (`docs/playtests/2026-10-27-sky-flicker.md`). A burst (`CB_STILLS_BURST_STEP_M`) flies it into the base.
    view("storm-base-sun", [-7700.0, 650.0, -6000.0], [-2700.0, 650.0, -1000.0]),
    // Through the biggest low puff near the west storm's base, a quarter of its radius off its centre, toward the sun
    // (`puff_pass_view`): a burst flies the eye past a puff's centre, where its billboard swings round.
    View {
        name: "puff-pass",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // The owner's view of 2026-10-27 ("cloud fog blend should be a thing"): 80 m over low land 6-14 km from the west
    // storm, looking past it at the horizon, where the far clouds should sit in the haze as the far land does
    // (`storm_ahead_view`).
    View {
        name: "storm-ahead",
        eye: [0.0, 0.0, 0.0],
        at: [0.0, 0.0, 0.0],
        above_ground: false,
    },
    // Under the storm, in its rain, 40 m up, facing away from the sun.
    View {
        name: "under-the-storm",
        eye: [-5600.0, 40.0, -3900.0],
        at: [-11600.0, 500.0, -9600.0],
        above_ground: true,
    },
    // Under the west storm's anvil, 2 km east of its centre, on the ground looking up into it.
    View {
        name: "storm-under-anvil",
        eye: [-3887.0, 2.0, -4186.0],
        at: [-5287.0, 1800.0, -4186.0],
        above_ground: true,
    },
    // 2.8 km north of the west storm's centre, on the ground looking up at its base and tower.
    View {
        name: "storm-under-edge",
        eye: [-5887.0, 2.0, -6986.0],
        at: [-5887.0, 1800.0, -5026.0],
        above_ground: true,
    },
    // The storm's shadow on the ground, from 900 m up outside it, looking down across its edge.
    View {
        name: "storm-shadow",
        eye: [500.0, 900.0, 2000.0],
        at: [-5000.0, 0.0, -3000.0],
        above_ground: true,
    },
];

/// The river view: 150 m off the longest river's middle, 40 m over the bank, looking 1.2 km
/// downstream along it.
fn river_view(plan: &crate::world::Plan) -> ([f32; 3], [f32; 3]) {
    let Some(r) = plan.waters.rivers.iter().max_by_key(|r| r.path.len()) else {
        return ([0.0, 40.0, 0.0], [0.0, 0.0, 100.0]);
    };
    let i = r.path.len() / 2;
    let (p, q) = (r.path[i], r.path[(i + 1).min(r.path.len() - 1)]);
    let along = (q - p).normalize_or(Vec2::X);
    let eye = p - along * 300.0 + along.perp() * 150.0;
    let at = p + along * 1200.0;
    ([eye.x, 40.0, eye.y], [at.x, r.surface_m[i], at.y])
}

/// A steep river reach seen from a clear eye (no cloud there, at time zero): the reach whose
/// surface falls most over 10 points (off every waterfall) on the range's west flank, seen from
/// `back_m` across the slope and `up_m` over its water, or from straight above.
fn steep_river_view(plan: &crate::world::Plan, back_m: f32, up_m: f32, top: bool) -> ([f32; 3], [f32; 3]) {
    let w = &plan.waters;
    let mut best: Option<(f32, Vec3, Vec3)> = None;
    for (ri, r) in w.rivers.iter().enumerate() {
        for i in 0..r.path.len().saturating_sub(10) {
            if w.falls.iter().any(|f| f.river == ri && i + 12 >= f.lip && i <= f.foot + 2) {
                continue;
            }
            let drop = r.surface_m[i] - r.surface_m[i + 10];
            let p = r.path[i + 5];
            let along = (r.path[i + 10] - r.path[i]).normalize_or(Vec2::X);
            let s = r.surface_m[i + 5];
            let side = p + along.perp() * back_m;
            let eye = if top {
                Vec3::new(p.x + 1.0, s + up_m, p.y + 1.0)
            } else {
                Vec3::new(side.x, s.max(plan.terrain.height_at(side.x, side.y)) + up_m, side.y)
            };
            if plan.clouds.density_at(eye, 0.0) > 0.0 || best.is_some_and(|b| drop <= b.0) {
                continue;
            }
            best = Some((drop, eye, Vec3::new(p.x, s, p.y)));
        }
    }
    let Some((drop, eye, at)) = best else {
        return ([0.0, 400.0, 0.0], [0.0, 0.0, 100.0]);
    };
    info!("steep river: {drop:.0} m over 10 points at {at}");
    (eye.to_array(), at.to_array())
}

/// Inside the puff nearest any city's middle (where the wind has carried it by `now_s`), a third of
/// its height under its centre, looking up past it.
fn city_in_cloud_view(plan: &crate::world::Plan, now_s: f32) -> ([f32; 3], [f32; 3]) {
    let d = plan.clouds.drift(now_s);
    let near = |p: &cb_core::world::clouds::Puff| {
        plan.cities
            .iter()
            .map(|c| Vec2::new(p.centre.x + d.x, p.centre.z + d.y).distance(c.centre))
            .fold(f32::MAX, f32::min)
    };
    let Some(p) = plan.clouds.puffs.iter().min_by(|a, b| near(a).total_cmp(&near(b))) else {
        return ([0.0, 400.0, 0.0], [0.0, 2000.0, 100.0]);
    };
    let centre = p.centre + Vec3::new(d.x, 0.0, d.y);
    let eye = centre - Vec3::Y * p.radii.y * 0.33 + Vec3::new(p.radii.x * 0.2, 0.0, 0.0);
    info!("city in cloud: puff at {centre} radii {}, eye {eye}", p.radii);
    (eye.to_array(), (centre + Vec3::new(-200.0, 1500.0, 300.0)).to_array())
}

/// The biggest puff under 900 m within 3 km of (-6500, -5000) (the west storm's north-west base), and an eye 1.5 of its
/// radii before its centre toward the sun's bearing, a quarter radius to the side, at the centre's height, looking toward
/// the sun's bearing and up, so the sun is near the top of the view. A burst steps the eye along that bearing.
fn puff_pass_view(plan: &crate::world::Plan, now_s: f32) -> ([f32; 3], [f32; 3]) {
    let d = plan.clouds.drift(now_s);
    let near = Vec2::new(-6500.0, -5000.0);
    let pick = plan
        .clouds
        .puffs
        .iter()
        .filter(|p| p.centre.y < 900.0 && Vec2::new(p.centre.x + d.x, p.centre.z + d.y).distance(near) < 3000.0)
        .max_by(|a, b| a.radii.x.total_cmp(&b.radii.x));
    let Some(p) = pick else {
        return ([-6500.0, 650.0, -5000.0], [-5500.0, 1150.0, -4000.0]);
    };
    let centre = p.centre + Vec3::new(d.x, 0.0, d.y);
    let sun = cb_core::world::sun_shade::to_sun();
    let dir = Vec3::new(sun.x, 0.0, sun.z).normalize_or(Vec3::X);
    let side = Vec3::new(-dir.z, 0.0, dir.x);
    let eye = centre - dir * p.radii.x * 1.5 + side * p.radii.x * 0.25;
    info!("puff pass: puff at {centre} radii {}, eye {eye}", p.radii);
    // Far ahead, so a burst stepping the eye along the bearing never reaches it and the view holds its direction.
    (eye.to_array(), (eye + dir * 100000.0 + Vec3::Y * 50000.0).to_array())
}

/// 80 m over the lowest land 6-14 km east of the west storm's centre (-5.9, -4.2 km), looking at the horizon past the
/// storm, a little above level.
fn storm_ahead_view(plan: &crate::world::Plan) -> ([f32; 3], [f32; 3]) {
    use cb_core::world::Ground;
    let storm = Vec2::new(-5887.0, -4186.0);
    let mut best: Option<(f32, Vec2)> = None;
    for b in -6..=6 {
        let bearing = Vec2::from_angle(b as f32 * 0.12);
        for k in 0..=16 {
            let at = storm + bearing * (6000.0 + k as f32 * 500.0);
            let h = plan.height_at(at.x, at.y);
            if h > plan.waters.sea_level_m + 5.0 && best.is_none_or(|(bh, _)| h < bh) {
                best = Some((h, at));
            }
        }
    }
    let (h, at) = best.unwrap_or((0.0, storm + Vec2::new(9000.0, 0.0)));
    let eye = Vec3::new(at.x, h + 80.0, at.y);
    let toward = (storm - at).normalize_or(Vec2::NEG_X);
    (
        eye.to_array(),
        [at.x + toward.x * 40000.0, eye.y + 900.0, at.y + toward.y * 40000.0],
    )
}

/// Over open sea in the clear south-east, `up_m` up, looking toward the sun's bearing (`toward`)
/// or away from it, at the sea 15 eye heights ahead (a few degrees down).
fn sea_view(toward: bool, up_m: f32) -> ([f32; 3], [f32; 3]) {
    let sun = cb_core::world::sun_shade::to_sun();
    let level = Vec3::new(sun.x, 0.0, sun.z).normalize_or(Vec3::X) * if toward { 1.0 } else { -1.0 };
    let eye = Vec3::new(24000.0, up_m, 16000.0);
    (eye.to_array(), (eye + level * up_m * 15.0 - Vec3::Y * up_m).to_array())
}

/// The largest city: down the street nearest its middle, 2 m up, along the street's length; or
/// (`air`) from 1,500 m up, 3.5 km to its south-east, looking at its middle.
fn city_view(plan: &crate::world::Plan, air: bool) -> ([f32; 3], [f32; 3]) {
    let Some(city) = plan.cities.iter().max_by(|a, b| a.half_size_m.total_cmp(&b.half_size_m)) else {
        return ([0.0, 400.0, 0.0], [0.0, 0.0, 1000.0]);
    };
    let c = city.centre;
    if air {
        let eye = Vec3::new(c.x + 2500.0, 1500.0, c.y + 2500.0);
        return (eye.to_array(), [c.x, plan.terrain.height_at(c.x, c.y), c.y]);
    }
    let Some(street) = city.streets.iter().min_by(|a, b| {
        let d = |s: &cb_core::world::layout::StreetPiece| s.rect.centre().distance(c);
        d(a).total_cmp(&d(b))
    }) else {
        return ([c.x, 2.0, c.y], [c.x + 300.0, 2.0, c.y]);
    };
    let r = street.rect;
    let size = r.max - r.min;
    let along = if size.x >= size.y { Vec2::X } else { Vec2::Y };
    let start = r.centre() - along * (size.max_element() * 0.5 - 2.0);
    let ground = plan.terrain.height_at(start.x, start.y);
    let ahead = start + along * 300.0;
    info!("city street: city {} from {start} along {along}", city.id);
    ([start.x, ground + 2.0, start.y], [ahead.x, ground + 8.0, ahead.y])
}

/// On the street nearest the middle of the city with the most cloud overhead, 2 m up, looking nearly straight up.
fn city_up_view(plan: &crate::world::Plan) -> ([f32; 3], [f32; 3]) {
    // The city with the most cloud overhead (the storm decks are where the cloud's base shows).
    let overhead = |c: &cb_core::world::layout::City| -> f32 {
        (0..30)
            .map(|k| {
                plan.clouds
                    .density_at(Vec3::new(c.centre.x, 500.0 + k as f32 * 100.0, c.centre.y), 0.0)
            })
            .sum()
    };
    let Some(city) = plan.cities.iter().max_by(|a, b| overhead(a).total_cmp(&overhead(b))) else {
        return ([0.0, 400.0, 0.0], [0.0, 2000.0, 100.0]);
    };
    let street = city
        .streets
        .iter()
        .min_by(|a, b| {
            let d = |s: &cb_core::world::layout::StreetPiece| s.rect.centre().distance(city.centre) - s.width_m;
            d(a).total_cmp(&d(b))
        })
        .map_or(city.centre, |s| s.rect.centre());
    let eye = Vec3::new(street.x, plan.terrain.height_at(street.x, street.y) + 2.0, street.y);
    info!("city up: city {} at {eye}", city.id);
    (eye.to_array(), (eye + Vec3::new(60.0, 1000.0, 30.0)).to_array())
}

/// Up close on the steepest river reach that is not a waterfall: 40 m to its side and 6 m over the
/// ground, or (`below`) on the ground 120 m downstream and to the side, looking up at the water.
fn close_river_view(plan: &crate::world::Plan, below: bool) -> ([f32; 3], [f32; 3]) {
    let w = &plan.waters;
    let mut best: Option<(f32, Vec3, Vec3)> = None;
    for (ri, r) in w.rivers.iter().enumerate() {
        for i in 0..r.path.len().saturating_sub(16) {
            if w.falls.iter().any(|f| f.river == ri && i + 18 >= f.lip && i <= f.foot + 2) {
                continue;
            }
            let drop = r.surface_m[i] - r.surface_m[i + 10];
            let p = r.path[i + 5];
            let s = r.surface_m[i + 5];
            let along = (r.path[i + 10] - r.path[i]).normalize_or(Vec2::X);
            let side = if below {
                p + along * 120.0 + along.perp() * 30.0
            } else {
                p + along.perp() * 40.0
            };
            let ground = plan.terrain.height_at(side.x, side.y);
            let eye = Vec3::new(side.x, ground + if below { 2.0 } else { 6.0 }, side.y);
            if below && eye.y >= s {
                continue;
            }
            if plan.clouds.density_at(eye, 0.0) > 0.0 || best.is_some_and(|b| drop <= b.0) {
                continue;
            }
            best = Some((drop, eye, Vec3::new(p.x, s, p.y)));
        }
    }
    let Some((drop, eye, at)) = best else {
        return ([0.0, 400.0, 0.0], [0.0, 0.0, 100.0]);
    };
    info!("close river: {drop:.0} m over 10 points at {at}, eye {eye}");
    (eye.to_array(), at.to_array())
}

#[derive(Resource)]
struct WeatherScript {
    dir: PathBuf,
    /// The view being shot (0 waits for the game to be ready).
    beat: usize,
    beat_started_s: f32,
    frames: u32,
    /// The stills asked for, to wait for before the exit.
    shots: Vec<PathBuf>,
    /// `CB_STILLS_OFFSCREEN=1`: the eye drawn into an offscreen image, so a locked screen or a window behind another
    /// does not save black.
    offscreen: Option<Handle<Image>>,
}

/// `CB_STILLS_OFFSCREEN=1`: the weather stills draw the pilot's eye and the cockpit pass offscreen (`draw_offscreen`).
fn offscreen_for_weather(
    mut commands: Commands,
    camera: Single<Entity, With<crate::pilot::PilotCamera>>,
    cockpit: Single<Entity, With<crate::pilot::CockpitCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut script: ResMut<WeatherScript>,
) {
    if !crate::cli::switched_off("CB_STILLS_OFFSCREEN") {
        return;
    }
    script.offscreen = Some(draw_offscreen(&mut commands, *camera, *cockpit, &mut images));
}

impl Plugin for WeatherStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(WeatherScript {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            frames: 0,
            shots: Vec::new(),
            offscreen: None,
        })
        .add_after_world_spawn(offscreen_for_weather)
        .add_game_systems(Update, run_weather_script);
    }
}

fn run_weather_script(
    mut commands: Commands,
    mut script: ResMut<WeatherScript>,
    mut pilot: ResMut<PilotSim>,
    plan: Res<crate::world::Plan>,
    time: Res<Time<Real>>,
    ready: Option<Res<crate::loading::GameReady>>,
    mut exit: MessageWriter<AppExit>,
) {
    use cb_core::world::Ground;
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    script.frames += 1;
    let mut next = false;
    if script.beat == 0 {
        next = ready.is_some_and(|r| r.0) && t > 6.0;
    } else if let Some(v) = VIEWS.get(script.beat - 1) {
        let only = std::env::var("CB_STILLS_ONLY").unwrap_or_default();
        if !only.is_empty() && !only.split(',').any(|n| n.trim() == v.name) {
            script.beat += 1;
            return;
        }
        let (v_eye, v_at) = match v.name {
            "river" => river_view(&plan),
            "river-steep" => steep_river_view(&plan, 700.0, 150.0, false),
            "river-steep-far" => steep_river_view(&plan, 3000.0, 400.0, false),
            "river-steep-top" => steep_river_view(&plan, 0.0, 500.0, true),
            "river-close" => close_river_view(&plan, false),
            "river-close-below" => close_river_view(&plan, true),
            "city-up" => city_up_view(&plan),
            "city-street" => city_view(&plan, false),
            "city-air" => city_view(&plan, true),
            "suburb-far" => {
                let at = Vec2::new(-6939.0, -3000.0);
                let eye = at + Vec2::new(600.0, 600.0);
                let ground = plan.terrain.height_at(at.x, at.y);
                let top = ground.max(plan.terrain.height_at(eye.x, eye.y));
                ([eye.x, top + 300.0, eye.y], [at.x, ground, at.y])
            }
            "sea-sun" => sea_view(true, 40.0),
            "sea-away" => sea_view(false, 40.0),
            "sea-sun-high" => sea_view(true, 1500.0),
            "sea-sun-top" => sea_view(true, 6500.0),
            "city-in-cloud" => city_in_cloud_view(&plan, now),
            "puff-pass" => puff_pass_view(&plan, now),
            "storm-ahead" => storm_ahead_view(&plan),
            _ => (v.eye, v.at),
        };
        let v = View {
            name: v.name,
            eye: v_eye,
            at: v_at,
            above_ground: v.above_ground,
        };
        let ground = plan.height_at(v.eye[0], v.eye[2]).max(plan.waters.sea_level_m);
        let eye = if v.above_ground {
            Vec3::new(v.eye[0], ground + v.eye[1], v.eye[2])
        } else {
            Vec3::new(v.eye[0], v.eye[1].max(ground + 2.0), v.eye[2])
        };
        // `CB_STILLS_BURST_STEP_M=<m>`: each still of a burst a step further along the view, as a flight past
        // the view would see on consecutive frames (60 m/s at 60 frames a second is 1 m).
        let step_m: f32 = std::env::var("CB_STILLS_BURST_STEP_M")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        let along = (Vec3::from_array(v.at) - eye).with_y(0.0).normalize_or_zero();
        let done = script
            .shots
            .iter()
            .filter(|p| p.to_string_lossy().contains(&format!("weather-{}-", v.name)))
            .count();
        let eye = eye + along * step_m * done as f32;
        // Held at the view every frame (a pilot on foot in the air would fall).
        stand_looking(
            &mut pilot,
            eye - Vec3::Y * 1.7,
            eye + (Vec3::from_array(v.at) - eye).normalize_or_zero() * 1000.0,
        );
        // Time for the ground's detail, the trees and the sky to stream in round the eye.
        // `CB_STILLS_BURST=<n>`: n stills of the view on consecutive frames (`weather-<name>-<k>.png`), the
        // instrument for flicker between frames (a value that changes frame to frame shows as a difference).
        let burst: usize = std::env::var("CB_STILLS_BURST").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        if t > 8.0 && script.frames > 12 {
            let k = script
                .shots
                .iter()
                .filter(|p| p.to_string_lossy().contains(&format!("weather-{}-", v.name)))
                .count();
            let name = if burst > 0 {
                format!("weather-{}-{k}.png", v.name)
            } else {
                format!("weather-{}.png", v.name)
            };
            match script.offscreen.clone() {
                Some(img) => shoot_offscreen(&mut commands, img, &script.dir.clone(), &name),
                None => shoot(&mut commands, &script.dir.clone(), &name),
            }
            let path = script.dir.join(name);
            script.shots.push(path);
            println!(
                "STILLS_WEATHER {} eye ({:.0}, {:.0}, {:.0}) cloud {:.2} rain {:.2} sun {:.2}",
                v.name,
                eye.x,
                eye.y,
                eye.z,
                plan.clouds.density_at(eye, now),
                plan.clouds.rain_at(eye, now),
                plan.clouds.sun_at(Vec2::new(eye.x, eye.z), now)
            );
            next = burst == 0 || k + 1 >= burst;
        }
    } else if script.shots.iter().all(|p| p.exists()) || t > 120.0 {
        // Once every still is on disk (a software renderer reads a frame back slowly).
        exit.write(AppExit::Success);
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
        script.frames = 0;
    }
}
