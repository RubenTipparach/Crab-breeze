//! Menu buttons answer the pointer (owner, 2026-10-29: "also need hover/click feedback from ui buttons, please
//! implement sfx for those too"): a button under the pointer lights its border and ticks, a press clicks. Every menu
//! button (the main menu's, the pause menu's and its settings rows) carries `UiButton`; the look and the sounds are
//! here once. The sounds are the cues `ui_hover` and `ui_click` in `data/audio/cues.ron`, menu sounds, so unplaced
//! (`Space::Ui`), played at the player's sound effects volume in every state: the world's sound pipeline
//! (`audio::SoundPlugin`) runs only in a game, and the main menu has none.
//!
//! A press also flashes the button (owner, 2026-10-30: "the buttons in the menu are kinda lame, add a flash when you
//! click on them"): its face fills with the lit colour and its border goes white, then both fade back over
//! `FLASH_S`. The fade runs on real time, so it plays while the pause menu has stopped the game's clock.

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

/// A pressed button's face at the start of its flash (the lit colour, nearly opaque)...
const FLASH_FACE: Color = Color::srgba(1.0, 0.78, 0.3, 0.85);
/// ...and its border.
const FLASH_EDGE: Color = Color::WHITE;
/// How long a flash takes to fade back, seconds.
const FLASH_S: f32 = 0.3;

/// A button flashing from a press: real seconds since the press, and the face and border it fades back to (the
/// border its own, or the hover light while the pointer stays on it).
#[derive(Component)]
struct Flash {
    t_s: f32,
    face: BackgroundColor,
    edge: Color,
}

/// Lights, ticks and clicks the menu buttons.
pub struct UiFeedbackPlugin;

impl Plugin for UiFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (light, fade).chain()).add_observer(click);
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

/// A press on a button (or on its label, bubbling up to it) clicks and flashes it.
fn click(
    ev: On<PointerPress>,
    mut commands: Commands,
    mut buttons: Query<(&mut BackgroundColor, &BorderColor, &UiButton, Option<&Flash>)>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let target = ev.event_target();
    let Ok((mut face, border, button, flashing)) = buttons.get_mut(target) else {
        return;
    };
    let sfx = settings.map_or(1.0, |s| s.0.sfx_volume);
    play(&mut commands, &data, &assets, sfx, "ui_click");
    // A press during a flash starts it again, fading back to the face the button had before the first.
    let (idle, edge) = match flashing {
        Some(f) => (f.face, f.edge),
        None => (*face, button.idle.unwrap_or(*border).top),
    };
    *face = BackgroundColor(FLASH_FACE);
    commands.entity(target).insert(Flash {
        t_s: 0.0,
        face: idle,
        edge,
    });
}

/// Fades each flashing button's face and border back: the face to its own, the border to the hover light while the
/// pointer is on it (`light`), else to its own.
fn fade(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut buttons: Query<(Entity, &mut Flash, &Hovered, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (e, mut flash, hovered, mut face, mut border) in &mut buttons {
        flash.t_s += time.delta_secs();
        let k = (flash.t_s / FLASH_S).clamp(0.0, 1.0);
        let edge_to = if hovered.get() { LIT } else { flash.edge };
        *face = BackgroundColor(FLASH_FACE.mix(&flash.face.0, k));
        *border = BorderColor::all(FLASH_EDGE.mix(&edge_to, k));
        if k >= 1.0 {
            *face = flash.face;
            commands.entity(e).remove::<Flash>();
        }
    }
}
