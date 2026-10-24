//! The capture instrument (`pilot-boarding` design section 4; CLAUDE.md section 4 allows it as a
//! measurement instrument): `--stills <dir>` writes stills from the street and the cockpit and
//! logs frame time over a stride, then exits. It only writes input actions, as a player would,
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
use crate::mech::MechAccessNow;
use crate::pilot::PilotSim;
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
    /// The building variant: the building, where the pilot stands, and the shells' aim points.
    buildings: Option<(u32, Vec3, Vec<Vec3>)>,
    shells_fired: usize,
    /// The street tour before the demolition: how many of its shots are taken.
    tour_done: usize,
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
            buildings: None,
            shells_fired: 0,
            tour_done: 0,
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
    in_beat: f32,
    next: &mut bool,
    exit: &mut MessageWriter<AppExit>,
) {
    const EYE_M: f32 = 1.7;
    let place = |pilot: &mut PilotSim, feet: Vec3, target: Vec3| {
        let (yaw, pitch) = look_at(feet, EYE_M, target);
        let mut s = PilotState::standing(feet, yaw);
        s.pitch_rad = pitch;
        pilot.prev = s.clone();
        pilot.curr = s;
    };
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
        // Each shot 3 s after the last (the pilot is placed every frame, so the high one stays up).
        if in_beat > 3.0 * (script.tour_done + 1) as f32 {
            shoot_image(commands, script, &format!("tour-{}-{name}.png", script.tour_done));
            println!("STILLS_TOUR {name} building={} storeys={}", b.id, b.storeys);
            script.tour_done += 1;
        }
        return;
    }
    match script.beat {
        // Pick the mid-rise nearest the mech and stand 45 m off its -z face.
        1 => {
            if script.buildings.is_none() {
                let Some(a) = access.0 else { return };
                let here = Vec2::new(a.ladder_base.x, a.ladder_base.z);
                let Some(b) = plan
                    .0
                    .buildings
                    .iter()
                    .filter(|b| (8..=20).contains(&b.storeys))
                    .min_by(|p, q| p.footprint.centre().distance(here).total_cmp(&q.footprint.centre().distance(here)))
                else {
                    exit.write(AppExit::error());
                    return;
                };
                let ground = plan.0.cities[b.city as usize].ground_m;
                let c = b.footprint.centre();
                let feet = Vec3::new(c.x, ground, b.footprint.min.y - 11.0);
                let feet = Vec3::new(feet.x, plan.0.terrain.height_at(feet.x, feet.z).max(ground), feet.z);
                // The foot of every column on the facing side, as a player taking out a side would aim.
                let cells = &live.structures.buildings[b.id as usize];
                let aims = (0..=cells.n[0])
                    .map(|a| Vec3::new(b.footprint.min.x + cells.size.x * a as f32, ground, b.footprint.min.y + 1.0))
                    .collect();
                println!("STILLS_BUILDING id={} storeys={} at ({:.0}, {:.0})", b.id, b.storeys, c.x, c.y);
                script.buildings = Some((b.id, feet, aims));
            }
            let (id, feet, _) = script.buildings.clone().unwrap();
            let b = &plan.0.buildings[id as usize];
            let ground = plan.0.cities[b.city as usize].ground_m;
            let mid = Vec3::new(b.footprint.centre().x, ground + b.height_m * 0.45, b.footprint.min.y);
            place(pilot, feet, mid);
            if in_beat > 2.0 {
                shoot_image(commands, script, "building-whole.png");
                *next = true;
            }
        }
        // F9 at the foot of each facing column, 0.5 s apart.
        2 => {
            let (_, feet, aims) = script.buildings.clone().unwrap();
            let i = (in_beat / 0.5) as usize;
            if i < aims.len() {
                place(pilot, feet, aims[i]);
                let fire = i >= script.shells_fired && in_beat - i as f32 * 0.5 > 0.2;
                acc.script = Some(Actions::default());
                if fire {
                    // Pressed like the key: latched until the next simulation step takes it.
                    acc.debug = Some(cb_core::input::DebugCommand::Shell);
                    script.shells_fired += 1;
                }
            } else {
                acc.script = Some(Actions::default());
                *next = true;
            }
        }
        // Look back up at it and shoot it as it creaks, falls and settles.
        3 => {
            let (id, feet, _) = script.buildings.clone().unwrap();
            let b = &plan.0.buildings[id as usize];
            let ground = plan.0.cities[b.city as usize].ground_m;
            place(
                pilot,
                feet,
                Vec3::new(b.footprint.centre().x, ground + b.height_m * 0.4, b.footprint.min.y),
            );
            const SHOTS: [(f32, &str); 4] = [
                (1.0, "building-hit.png"),
                (2.2, "building-creak.png"),
                (5.0, "building-fall.png"),
                (12.0, "building-rubble.png"),
            ];
            if let Some(&(t, name)) = SHOTS.get(script.fall_shots)
                && in_beat >= t
            {
                shoot_image(commands, script, name);
                script.fall_shots += 1;
            }
            if script.fall_shots == SHOTS.len() && in_beat > 13.0 {
                *next = true;
            }
        }
        // Walk up to the rubble and look down at it.
        4 => {
            let (id, feet, _) = script.buildings.clone().unwrap();
            let b = &plan.0.buildings[id as usize];
            let ground = plan.0.cities[b.city as usize].ground_m;
            let close = Vec3::new(feet.x, feet.y, b.footprint.min.y - 6.0);
            place(
                pilot,
                close,
                Vec3::new(b.footprint.centre().x, ground + 2.0, b.footprint.min.y + 6.0),
            );
            if in_beat > 2.0 {
                shoot_image(commands, script, "building-rubble-close.png");
                *next = true;
            }
        }
        5 if in_beat > 1.5 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
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

/// Faces the pilot standing at `feet` toward `target`: (yaw, pitch).
fn look_at(feet: Vec3, eye_m: f32, target: Vec3) -> (f32, f32) {
    let d = target - (feet + Vec3::Y * eye_m);
    ((-d.x).atan2(-d.z), d.y.atan2(Vec2::new(d.x, d.z).length()))
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
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if let Some((id, _, _)) = script.buildings.as_ref()
        && script.beat >= 2
        && (now * 2.0) as u32 != ((now - time.delta_secs()) * 2.0) as u32
    {
        let c = &live.structures.buildings[*id as usize];
        println!(
            "STILLS_STANDING beat={} cells={}/{} frame_elements={}",
            script.beat,
            c.standing,
            c.intact.len(),
            c.frame.as_ref().map_or(0, |f| f.standing())
        );
    }
    let in_beat = now - script.beat_started_s;
    let mut next = false;
    if script.beat >= 1 && crate::cli::switched_off("CB_STILLS_BUILDINGS") {
        buildings_beats(
            &mut commands,
            &mut script,
            &mut acc,
            &mut pilot,
            &access,
            &plan,
            &live,
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
        // Walk to the ladder by input, then press Use.
        2 => {
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
        3 => {
            // Seated; wait for the hatch to seal behind the pilot.
            if pilot.curr.mode == PilotMode::Seated && in_beat > 12.0 {
                shoot(&mut commands, &script.dir, "cockpit.png");
                next = true;
            }
        }
        // Stand up and stride: throttle forward (the mech stands first), hold, measure frame time
        // over 10 s.
        4 => {
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
        5 if in_beat > 1.5 => {
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
