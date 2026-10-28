//! The player's settings in the game (openspec change `game-menus`, design section 4): read from
//! `settings.ron` at startup over the data's own defaults, applied live wherever each takes effect
//! (the input sampler, both cameras' lenses, the sun's shadows, the window), and saved when the
//! Options page closes. The record and its limits are `cb_core::settings`.

use crate::input::Keys;
use crate::sim::Data;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use cb_core::settings::Settings as Record;

/// The settings in force.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Settings(pub Record);

/// Applies the settings.
pub struct SettingsPlugin {
    /// The window's vsync and fullscreen follow the settings. Off for measurement and capture runs,
    /// which set the window themselves (`CB_UNCAPPED`, `--stills`, `--bench`).
    pub window: bool,
}

/// Whether the window follows the settings.
#[derive(Resource, Clone, Copy)]
struct WindowFollows(bool);

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WindowFollows(self.window))
            .add_systems(PreStartup, load)
            .add_systems(Update, apply.run_if(resource_changed::<Settings>));
    }
}

fn path() -> std::path::PathBuf {
    crate::cli::repo_root().join("settings.ron")
}

/// The data's defaults, then `settings.ron` over them if it is there and good. A bad file is
/// reported and left alone until Options saves over it.
fn load(mut commands: Commands, data: Res<Data>) {
    let defaults = Record::from_data(&data.0.bindings, &data.0.pilot);
    let record = match std::fs::read_to_string(path()) {
        Ok(text) => Record::from_ron(&text).unwrap_or_else(|e| {
            warn!("settings: {} not used: {e}", path().display());
            defaults
        }),
        Err(_) => defaults,
    };
    commands.insert_resource(Settings(record));
}

/// Writes the settings to `settings.ron` (through a temporary file, so a crash keeps the last good one).
pub fn save(s: &Settings) {
    let tmp = path().with_extension("ron.tmp");
    let done = std::fs::write(&tmp, s.0.to_ron()).and_then(|_| std::fs::rename(&tmp, path()));
    if let Err(e) = done {
        warn!("settings: could not save {}: {e}", path().display());
    }
}

/// Puts the settings into effect. The shadows setting is read by `day_night`, which owns which
/// body casts them; `CB_NO_SHADOWS` still keeps them off whatever the settings say.
fn apply(settings: Res<Settings>, follows: Res<WindowFollows>, mut keys: ResMut<Keys>, mut window: Query<&mut Window>) {
    let s = settings.0;
    keys.deg_per_px = s.mouse_deg_per_px;
    keys.invert_pitch = s.invert_pitch;
    if follows.0 {
        for mut w in &mut window {
            w.present_mode = if s.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            };
            w.mode = if s.fullscreen {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            } else {
                WindowMode::Windowed
            };
        }
    }
}
