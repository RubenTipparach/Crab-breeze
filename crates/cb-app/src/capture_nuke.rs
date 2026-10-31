//! The nuke test scene (a measurement instrument, CLAUDE.md section 4; crab-airship section 11; owner, 2026-10-30:
//! "Just build it in game in a test scene", "You should be testing that in an open plane. No trees"): with
//! `CB_STILLS_NUKE=1 CB_NO_TREES=1 --stills <dir>`, the scene takes the world seed's stored spot (`data/scenes/nuke.ron`), or finds an
//! open plain (`open_plain`) and prints it to store, stands the pilot 2.5 km from its middle, facing it; once the world is ready a `Nuke` goes off there (the
//! same blast the Stratostacean's crash sets off), shot as the fireball swells and the mushroom climbs; then the pilot
//! stands on the crater's rim, and back from it, looking in. `NUKE_START` and `NUKE_END` bracket it for the recorder. The pilot is placed
//! and turned directly, as the other stills do.

use crate::capture::{shoot, stand_looking};
use crate::loading::GameReady;
use crate::nuke::Nuke;
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::world::Ground;
use std::path::PathBuf;

/// Runs the nuke test scene.
pub struct NukeStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Scene {
    dir: PathBuf,
    ready_at: Option<f32>,
    boom_at: Option<f32>,
    target: Vec3,
    watcher: Vec3,
    done: Vec<&'static str>,
}

impl Plugin for NukeStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Scene {
            dir: self.dir.clone(),
            ready_at: None,
            boom_at: None,
            target: Vec3::ZERO,
            watcher: Vec3::ZERO,
            done: Vec::new(),
        })
        .add_game_systems(Update, run);
    }
}

/// Seconds after the blast for each shot, and the shot's name.
const SHOTS: [(f32, &str); 9] = [
    (0.5, "nuke-00.5s.png"),
    (2.0, "nuke-02s.png"),
    (4.0, "nuke-04s.png"),
    (8.0, "nuke-08s.png"),
    (15.0, "nuke-15s.png"),
    (25.0, "nuke-25s.png"),
    (40.0, "nuke-40s.png"),
    (55.0, "nuke-55s.png"),
    (70.0, "nuke-70s.png"),
];

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    ready: Res<GameReady>,
    plan: Res<Plan>,
    data: Res<Data>,
    time: Res<Time<Real>>,
    mut pilot: ResMut<PilotSim>,
    mut nukes: MessageWriter<Nuke>,
    mut meshes: MessageWriter<crate::nuke_mesh::MeshNuke>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if !ready.0 {
        return;
    }
    let ready_at = *scene.ready_at.get_or_insert(now);
    let ground = |x: f32, z: f32| plan.terrain.height_at(x, z);
    // Stand 2.5 km off an open plain's middle, facing it.
    if now - ready_at > 4.0 && scene.boom_at.is_none() && scene.target == Vec3::ZERO {
        let Some((c, spot)) = stored_spot(plan.seed).or_else(|| open_plain(&plan, &data.0.grass)) else {
            println!("NUKE_END no open plain found");
            exit.write(AppExit::error());
            return;
        };
        scene.target = Vec3::new(c.x, ground(c.x, c.y), c.y);
        let feet = Vec3::new(spot.x, ground(spot.x, spot.y), spot.y);
        scene.watcher = feet;
        // at the cloud, high enough to keep its spread cap in view; with the mesh one beside it, between the two
        let across = Vec3::new(-(scene.target.z - feet.z), 0.0, scene.target.x - feet.x).normalize_or(Vec3::X);
        let aside = if with_mesh() { 650.0 } else { 0.0 };
        stand_looking(&mut pilot, feet, scene.target + across * aside + Vec3::Y * 800.0);
        println!(
            "NUKE spot for seed {}: (blast: ({:.1}, {:.1}), watcher: ({:.1}, {:.1}))",
            plan.seed, c.x, c.y, spot.x, spot.y
        );
        println!(
            "NUKE_START looking from ({:.0}, {:.0}) at an open plain at ({:.0}, {:.0}), 2.5 km off",
            spot.x, spot.y, c.x, c.y
        );
    }
    // It goes off 4 s after the pilot is placed.
    if now - ready_at > 8.0 && scene.boom_at.is_none() && scene.target != Vec3::ZERO {
        nukes.write(Nuke { at: scene.target });
        // `CB_NUKE_MESH=1`: Mining Mike's mesh nuke beside it, its reference (owner: "Place mesh nuke side by side of
        // this nuke"), 1.3 km to the side across the view, on the same timeline. Off by default (owner, 2026-10-30:
        // "Just render with the new nuke").
        let across = Vec3::new(-(scene.target.z - scene.watcher.z), 0.0, scene.target.x - scene.watcher.x).normalize_or(Vec3::X);
        let side = scene.target + across * 1300.0;
        if with_mesh() {
            meshes.write(crate::nuke_mesh::MeshNuke(Vec3::new(
                side.x,
                plan.terrain.height_at(side.x, side.z),
                side.z,
            )));
        }
        scene.boom_at = Some(now);
    }
    let Some(boom) = scene.boom_at else { return };
    let t = now - boom;
    for (s, name) in SHOTS {
        if t > s && !scene.done.contains(&name) {
            scene.done.push(name);
            shoot(&mut commands, &scene.dir, name);
        }
    }
    // Then the crater from its rim, looking down into it.
    let r = data.0.fortress.cfg.crash.nuke.map_or(300.0, |n| n.crater_radius_m);
    let c = Vec2::new(scene.target.x, scene.target.z);
    let floor = Vec3::new(c.x, ground(c.x, c.y), c.y);
    for (at_s, shot_s, off_m, name) in [
        (80.0, 83.0, r + 30.0, "nuke-crater-rim.png"),
        (85.0, 88.0, r * 2.2, "nuke-crater-back.png"),
    ] {
        if t > at_s && !scene.done.contains(&name) && t < shot_s {
            let p = c + Vec2::new(1.0, 0.0) * off_m;
            stand_looking(&mut pilot, Vec3::new(p.x, ground(p.x, p.y), p.y), floor);
        }
        if t > shot_s && !scene.done.contains(&name) {
            scene.done.push(name);
            shoot(&mut commands, &scene.dir, name);
        }
    }
    if t > 90.0 && !scene.done.contains(&"end") {
        scene.done.push("end");
        println!("NUKE_END");
    }
    if t > 92.0 {
        exit.write(AppExit::Success);
    }
}

/// A grass plain for the blast and a place 2.5 km off to watch from (owner, 2026-10-30: "pick grass area, turn off
/// trees"; the scene runs with `CB_NO_TREES=1`): thick grass at both and at the line's middle, dry, 2 km from any city,
/// the sun not in the watcher's eyes (more than 50 deg off the line), the ground under the sight line to 250 m over the
/// blast at every 12.5 m (a slope just ahead filled the whole view, nuke scene 7). The island scanned on a 600 m grid
/// from its middle out, 16 ways round each, so the same world always picks the same plain; the run prints it to store.
fn open_plain(plan: &cb_core::world::WorldPlan, grass: &cb_core::grass::GrassConfig) -> Option<(Vec2, Vec2)> {
    let mut cells: Vec<Vec2> = (-32..=32)
        .flat_map(|j| (-32..=32).map(move |i| Vec2::new(i as f32, j as f32) * 600.0))
        .collect();
    cells.sort_by(|a, b| {
        a.length()
            .total_cmp(&b.length())
            .then(a.x.total_cmp(&b.x))
            .then(a.y.total_cmp(&b.y))
    });
    let h = |p: Vec2| plan.terrain.height_at(p.x, p.y);
    let to_sun = cb_core::world::sun_shade::to_sun();
    let sun = Vec2::new(to_sun.x, to_sun.z).normalize_or(Vec2::X);
    let grassy = |p: Vec2| {
        plan.water_at(p.x, p.y).is_none()
            && h(p) > plan.waters.sea_level_m + 5.0
            && plan.cities.iter().all(|c| c.centre.distance(p) > 2000.0)
            && cb_core::grass::grows_at(plan, p, h(p), 0.0, grass) >= 0.7
    };
    for &c in &cells {
        if !grassy(c) {
            continue;
        }
        for k in 0..16 {
            let v = c + Vec2::from_angle(k as f32 * std::f32::consts::FRAC_PI_8) * 2500.0;
            if !grassy(v) || !grassy((c + v) * 0.5) || (c - v).normalize_or(Vec2::X).dot(sun) > 0.64 {
                continue;
            }
            let (eye, aim) = (h(v) + 2.0, h(c) + 250.0);
            let seen = (1..200).all(|i| {
                let f = i as f32 / 200.0;
                let q = v + (c - v) * f;
                h(q) < eye + (aim - eye) * f - 8.0
            });
            if seen {
                return Some((c, v));
            }
        }
    }
    None
}

/// The stored spot for world seed `seed`, if there is one (`data/scenes/nuke.ron`; a file that fails to load is said,
/// and the scene searches).
fn stored_spot(seed: u64) -> Option<(Vec2, Vec2)> {
    let path = crate::cli::repo_root().join("data/scenes/nuke.ron");
    match cb_core::data::load_ron::<cb_core::scenes::NukeSpots>(&path) {
        Ok(s) => s.spots.get(&seed).map(|p| (Vec2::from(p.blast), Vec2::from(p.watcher))),
        Err(e) => {
            println!("NUKE {e}");
            None
        }
    }
}

/// Whether the scene sets Mining Mike's mesh nuke beside the game's (`CB_NUKE_MESH=1`).
fn with_mesh() -> bool {
    std::env::var("CB_NUKE_MESH").is_ok_and(|v| v == "1")
}
