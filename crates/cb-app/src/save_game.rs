//! Saving and continuing a player's game (openspec change `save-game`; owner, 2026-10-25: "where is the persistence? if
//! I click continue, it should load the game state where I was at last"). A free-roam game or a career saves where the
//! pilot and their mech are (`cb_core::save::Place`) every 30 s, when the pause menu opens and when the game quits;
//! Continue (`--play free-continue`, `--play career-continue`) puts them back once the world is ready. Scripted runs
//! neither save nor restore.

use crate::mech::{MechSim, Piloted};
use crate::pilot::PilotSim;
use bevy::prelude::*;
use cb_core::pilot::{PilotMode, wrap_rad};
use cb_core::save::{FreeRoamSave, Place, VERSION};

/// How often a game saves while it is played, seconds of real time.
const EVERY_S: f32 = 30.0;

/// Which game this run saves, and whether it continues one.
#[derive(Resource, Clone)]
pub struct Saving {
    /// A free-roam game (else a career, saved with its books).
    pub free_roam: bool,
    /// Continue: restore the save once the world is ready.
    pub restore: bool,
    /// The piloted mech's data file, written with a free-roam save and checked on restore.
    pub mech: String,
}

/// The save is restored (or there was none to restore): saving may begin, so a fresh start never
/// overwrites the save before it is read.
#[derive(Resource, Default)]
struct Restored(bool);

/// Saves and continues a player's game.
pub struct SaveGamePlugin;

impl Plugin for SaveGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Restored>()
            .add_systems(Update, (restore, save_now).chain().run_if(resource_exists::<Saving>))
            .add_systems(Last, save_on_quit.run_if(resource_exists::<Saving>));
    }
}

/// Where the player and their mech are now, unless the pilot is mid-boarding or climbing out.
fn place(pilot: &PilotSim, sim: &MechSim) -> Option<Place> {
    let p = &pilot.curr;
    let seated = match p.mode {
        PilotMode::OnFoot => false,
        PilotMode::Seated => true,
        PilotMode::Boarding(_) | PilotMode::Disembarking(_) => return None,
    };
    let m = &sim.curr;
    Some(Place {
        pilot_feet: p.feet.to_array(),
        pilot_yaw_rad: p.yaw_rad,
        pilot_pitch_rad: p.pitch_rad,
        seated,
        mech_pos: m.pos.to_array(),
        mech_heading_rad: m.heading_rad,
        structure: sim.damage.structure.clone(),
        lost: sim.damage.lost.clone(),
        rounds_left: sim.guns.rounds_left.clone(),
    })
}

/// Writes `text` to `path` through a temporary file, so a crash never leaves a half-written save.
fn write(path: &std::path::Path, text: &str) {
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
    let tmp = path.with_extension("ron.tmp");
    if let Err(e) = std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, path)) {
        warn!("save: could not write {}: {e}", path.display());
    }
}

/// Saves the game: a free-roam save file, or the career's place written with its books.
fn save(saving: &Saving, seed: u64, place: Place, career: Option<&mut crate::career::CareerRun>) {
    if saving.free_roam {
        let s = FreeRoamSave {
            version: VERSION,
            seed,
            mech: saving.mech.clone(),
            place,
        };
        write(&crate::launcher::free_roam_save_path(), &s.to_ron());
    } else if let Some(run) = career {
        run.career.place = Some(place);
        crate::career::save(&run.career);
    }
}

/// Every 30 s of play, and when the pause menu opens.
#[allow(clippy::too_many_arguments)]
fn save_now(
    saving: Res<Saving>,
    restored: Res<Restored>,
    seed: Res<crate::world::Seed>,
    time: Res<Time<Real>>,
    menu: Res<crate::menu::Menu>,
    pilot: Res<PilotSim>,
    mech: Query<&MechSim, With<Piloted>>,
    mut career: Option<ResMut<crate::career::CareerRun>>,
    mut since_s: Local<f32>,
) {
    if !restored.0 {
        return;
    }
    *since_s += time.delta_secs();
    let paused = menu.is_changed() && *menu == crate::menu::Menu::Pause;
    if *since_s < EVERY_S && !paused {
        return;
    }
    let Some(place) = mech.iter().next().and_then(|sim| place(&pilot, sim)) else {
        return;
    };
    *since_s = 0.0;
    save(&saving, seed.0, place, career.as_deref_mut());
}

/// When the game quits.
fn save_on_quit(
    mut quits: MessageReader<AppExit>,
    saving: Res<Saving>,
    restored: Res<Restored>,
    seed: Res<crate::world::Seed>,
    pilot: Res<PilotSim>,
    mech: Query<&MechSim, With<Piloted>>,
    mut career: Option<ResMut<crate::career::CareerRun>>,
) {
    if quits.read().next().is_none() || !restored.0 {
        return;
    }
    if let Some(place) = mech.iter().next().and_then(|sim| place(&pilot, sim)) {
        save(&saving, seed.0, place, career.as_deref_mut());
    }
}

/// On Continue, once the world is ready and the piloted mech exists, puts the pilot and their mech
/// back where the save says. A save that does not read, or was made in another world or mech, is
/// left alone and the game starts fresh, saying why.
#[allow(clippy::too_many_arguments)]
fn restore(
    saving: Res<Saving>,
    mut restored: ResMut<Restored>,
    ready: Res<crate::loading::GameReady>,
    seed: Res<crate::world::Seed>,
    career: Option<Res<crate::career::CareerRun>>,
    mut pilot: ResMut<PilotSim>,
    mut mech: Query<&mut MechSim, With<Piloted>>,
) {
    if restored.0 || !ready.0 {
        return;
    }
    let Some(mut sim) = mech.iter_mut().next() else { return };
    if !saving.restore {
        restored.0 = true;
        return;
    }
    let place = if saving.free_roam {
        let path = crate::launcher::free_roam_save_path();
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| FreeRoamSave::from_ron(&t))
        {
            Ok(s) if s.seed != seed.0 || s.mech != saving.mech => {
                warn!("save: {} is from another world or mech; starting fresh", path.display());
                None
            }
            Ok(s) => Some(s.place),
            Err(e) => {
                warn!("save: {} does not read ({e}); starting fresh", path.display());
                let _ = std::fs::rename(&path, path.with_extension("bad"));
                None
            }
        }
    } else {
        // The career is read by `career::begin`; wait for it.
        let Some(run) = career else { return };
        run.career.place.clone()
    };
    restored.0 = true;
    let Some(place) = place else { return };

    let heading = place.mech_heading_rad;
    sim.curr.pos = Vec3::from_array(place.mech_pos);
    sim.curr.heading_rad = heading;
    sim.curr.vel = Vec2::ZERO;
    sim.curr.vel_y_m_s = 0.0;
    sim.prev = sim.curr;
    if place.structure.len() == sim.damage.structure.len() {
        sim.damage.structure = place.structure.clone();
        sim.damage.lost = place.lost.clone();
        let lost: Vec<usize> = place.lost.iter().enumerate().filter(|(_, l)| **l).map(|(i, _)| i).collect();
        sim.cuts.extend(lost);
    }
    if place.rounds_left.len() == sim.guns.rounds_left.len() {
        sim.guns.rounds_left = place.rounds_left.clone();
    }
    let p = &mut pilot.curr;
    p.yaw_rad = place.pilot_yaw_rad;
    p.pitch_rad = place.pilot_pitch_rad;
    p.velocity = Vec2::ZERO;
    p.waiting = None;
    if place.seated {
        p.mode = PilotMode::Seated;
        p.seated_rel_rad = wrap_rad(place.pilot_yaw_rad - heading);
    } else {
        p.mode = PilotMode::OnFoot;
        p.feet = Vec3::from_array(place.pilot_feet);
    }
    pilot.prev = pilot.curr.clone();
    info!(
        "save: continued at ({:.0}, {:.0}), {}",
        place.mech_pos[0],
        place.mech_pos[2],
        if place.seated { "in the seat" } else { "on foot" }
    );
}
