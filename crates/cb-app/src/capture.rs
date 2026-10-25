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
        .add_systems(PostStartup, offscreen_for_buildings)
        .add_systems(Update, run_script);
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

/// A still of the building variant's offscreen image.
fn shoot_image(commands: &mut Commands, script: &Script, name: &str) {
    let Some(img) = script.offscreen.clone() else {
        return shoot(commands, &script.dir, name);
    };
    let path = script.dir.join(name);
    info!("capture: {}", path.display());
    commands.spawn(Screenshot::image(img)).observe(save_to_disk(path));
}

fn shoot(commands: &mut Commands, dir: &std::path::Path, name: &str) {
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
    if !crate::cli::switched_off("CB_STILLS_BUILDINGS") {
        return;
    }
    use bevy::camera::RenderTarget;
    use bevy::render::render_resource::TextureFormat;
    let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb));
    let handle = images.add(image);
    commands.entity(*camera).insert(RenderTarget::Image(handle.clone().into()));
    commands.entity(*cockpit).insert(RenderTarget::Image(handle.clone().into()));
    script.offscreen = Some(handle);
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
    mechs: Query<&MechSim, With<Piloted>>,
    data: Res<Data>,
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
            if matches!(pilot.curr.mode, PilotMode::Boarding(_)) {
                // The stair is down and the climb begins: the open hatch from the stair's foot.
                acc.script = Some(Actions::default());
                shoot(&mut commands, &script.dir, "hatch-open.png");
                next = true;
            }
        }
        4 => {
            // Seated; wait for the hatch to seal behind the pilot.
            if pilot.curr.mode == PilotMode::Seated && in_beat > 12.0 {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Stand up and stride: throttle forward (the mech stands first), hold, measure frame time
        // over 10 s.
        5 => {
            acc.script = Some(Actions {
                throttle_axis: if in_beat < 5.0 { 0.6 } else { 0.0 },
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
        6 => {
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
        7 => {
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
        8 if in_beat > 1.5 && {
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
