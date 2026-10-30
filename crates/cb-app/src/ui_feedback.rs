//! Menu buttons answer the pointer (owner, 2026-10-29: "also need hover/click feedback from ui buttons, please
//! implement sfx for those too"): a button under the pointer lights its border and ticks, a press clicks. Every menu
//! button (the main menu's, the pause menu's and its settings rows) carries `UiButton`; the look and the sounds are
//! here once. The sounds are the cues `ui_hover` and `ui_click` in `data/audio/cues.ron`, menu sounds, so unplaced
//! (`Space::Ui`), played at the player's sound effects volume in every state: the world's sound pipeline
//! (`audio::SoundPlugin`) runs only in a game, and the main menu has none.

use crate::sim::Data;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
use bevy::picking::events::PointerPress;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use cb_core::audio::db_to_gain;

/// A menu button: lit and ticking under the pointer, clicking when pressed. Holds its border's own colour while lit.
#[derive(Component, Default)]
#[require(Hovered)]
pub(crate) struct UiButton {
    idle: Option<BorderColor>,
}

/// The border of a button under the pointer.
const LIT: Color = Color::srgb(1.0, 0.78, 0.3);

/// Lights, ticks and clicks the menu buttons.
pub struct UiFeedbackPlugin;

impl Plugin for UiFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, light).add_observer(click);
    }
}

/// Plays menu cue `name` once, unplaced, at the player's sound effects volume.
fn play(commands: &mut Commands, data: &Data, assets: &AssetServer, sfx: f32, name: &str) {
    let Some(cue) = data.0.audio.cues.cues.get(name) else {
        warn!("sound `{name}` is not in data/audio/cues.ron");
        return;
    };
    let Some(file) = cue.files.first() else { return };
    commands.spawn((
        AudioPlayer::new(assets.load(file.clone())),
        PlaybackSettings {
            volume: Volume::Linear(db_to_gain(cue.volume_db) * sfx),
            speed: (cue.pitch.0 + cue.pitch.1) * 0.5,
            ..PlaybackSettings::DESPAWN
        },
    ));
}

/// A button the pointer comes onto lights and ticks; one it leaves takes its own border back.
fn light(
    mut commands: Commands,
    data: Res<Data>,
    assets: Res<AssetServer>,
    settings: Option<Res<crate::settings::Settings>>,
    mut buttons: Query<(&Hovered, &mut UiButton, &mut BorderColor), Changed<Hovered>>,
) {
    let sfx = settings.map_or(1.0, |s| s.0.sfx_volume);
    for (hovered, mut button, mut border) in &mut buttons {
        if hovered.get() {
            if button.idle.is_none() {
                button.idle = Some(*border);
                play(&mut commands, &data, &assets, sfx, "ui_hover");
            }
            *border = BorderColor::all(LIT);
        } else if let Some(idle) = button.idle.take() {
            *border = idle;
        }
    }
}

/// A press on a button (or on its label, bubbling up to it) clicks.
fn click(
    ev: On<PointerPress>,
    mut commands: Commands,
    buttons: Query<(), With<UiButton>>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    if buttons.contains(ev.event_target()) {
        let sfx = settings.map_or(1.0, |s| s.0.sfx_volume);
        play(&mut commands, &data, &assets, sfx, "ui_click");
    }
}
