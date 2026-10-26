//! Music (openspec change `music`): the cue the game's state calls for, switched on the bar with a
//! crossfade, and silence when nothing is happening (owner, 2026-10-25: "if theres not mutch
//! action on screen, some silence would be nice"). The state is `cb_core::audio::music_state`; this
//! file reads its inputs from the game and plays what it returns. Music is not placed in 3D.

use crate::audio::{Ear, HearSet};
use crate::blast::Explosion;
use crate::fx::Fx;
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::GameApp;
use crate::units::Hostiles;
use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;
use cb_core::audio::{self as rules, MusicCue, MusicInputs, MusicState, db_to_gain};
use cb_core::pilot::PilotMode;

/// An enemy this close with the mech as its target is a threat, m (`music` design section 2).
const THREAT_WITHIN_M: f32 = 2000.0;
/// A round landing this close to the pilot is fire at them, m. The pilot's own rounds land at their target, and their
/// muzzle flashes never count.
const SHOT_WITHIN_M: f32 = 80.0;
/// An explosion this close is fire at them, m.
const BLAST_WITHIN_M: f32 = 80.0;

/// One playing cue.
struct Track {
    state: MusicState,
    voice: Entity,
    cue: MusicCue,
    started_s: f32,
    /// Fading in from `from_s` over `fade_s`.
    from_s: f32,
    fade_s: f32,
}

/// A cue fading out.
struct Fading {
    voice: Entity,
    from_gain: f32,
    from_s: f32,
    fade_s: f32,
}

#[derive(Resource, Default)]
struct Player {
    playing: Option<Track>,
    fading: Vec<Fading>,
    /// A change waiting for its bar line: (when, to what).
    switch_at: Option<(f32, MusicState)>,
    /// A cue that does not loop and has played out: it is not started again while its state holds.
    finished: Option<MusicState>,
    since_shot_s: f32,
}

/// The music.
pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Player {
            since_shot_s: 1e9,
            ..Default::default()
        })
        .add_game_systems(Update, (hear_shots, choose_and_play).chain().after(HearSet));
    }
}

/// Rounds and blasts landing near the pilot keep the combat music going (with enemies about: `music_state`).
fn hear_shots(mut player: ResMut<Player>, mut fx: MessageReader<Fx>, mut blasts: MessageReader<Explosion>, ear: Res<Ear>, time: Res<Time>) {
    player.since_shot_s += time.delta_secs();
    for f in fx.read() {
        let shot = f.recipe.starts_with("impact_");
        if shot && f.at.distance(ear.at) < SHOT_WITHIN_M {
            player.since_shot_s = 0.0;
        }
    }
    for b in blasts.read() {
        if b.at.distance(ear.at) < BLAST_WITHIN_M {
            player.since_shot_s = 0.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn choose_and_play(
    mut commands: Commands,
    mut player: ResMut<Player>,
    mut sinks: Query<&mut AudioSink>,
    pilot: Res<PilotSim>,
    hostiles: Option<Res<Hostiles>>,
    run: Option<Res<crate::defence::DefenceRun>>,
    ear: Res<Ear>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    time: Res<Time>,
    ready: Res<crate::loading::GameReady>,
    settings: Res<crate::settings::Settings>,
) {
    if !ready.0 {
        return;
    }
    let cfg = &data.0.audio.music;
    let now = time.elapsed_secs();
    let inputs = MusicInputs {
        title: false,
        boarding: matches!(pilot.curr.mode, PilotMode::Boarding(_)),
        seated: pilot.curr.mode == PilotMode::Seated,
        threatened: hostiles
            .as_ref()
            .is_some_and(|h| h.0.iter().any(|x| !x.shelling && x.feet.distance(ear.at) < THREAT_WITHIN_M)),
        since_shot_s: player.since_shot_s,
        set_piece: false,
        won: run.is_some_and(|r| matches!(r.state.phase, cb_core::mission::Phase::Won)),
    };
    let wanted = rules::music_state(&inputs, cfg.hold_s);
    let base = db_to_gain(cfg.volume_db) * settings.0.music_volume;

    // a stinger that has played out stays finished while its state holds
    let played_out = player
        .playing
        .as_ref()
        .filter(|t| !t.cue.looping && sinks.get(t.voice).is_ok_and(|s| s.empty()))
        .map(|t| (t.state, t.voice));
    if let Some((state, voice)) = played_out {
        player.finished = Some(state);
        commands.entity(voice).despawn();
        player.playing = None;
    }
    if player.finished.is_some_and(|f| f != wanted) {
        player.finished = None;
    }

    let current = player.playing.as_ref().map_or(MusicState::Silent, |t| t.state);
    let target = if player.finished == Some(wanted) {
        MusicState::Silent
    } else {
        wanted
    };
    if target != current {
        let at = match &player.playing {
            // change on the bar of the cue that is playing
            Some(t) => rules::next_bar_s(now, t.started_s, t.cue.bpm, t.cue.beats_per_bar),
            None => now,
        };
        if player.switch_at.is_none_or(|(_, s)| s != target) {
            player.switch_at = Some((at, target));
        }
    } else {
        player.switch_at = None;
    }

    if let Some((at, to)) = player.switch_at
        && now >= at
    {
        player.switch_at = None;
        if let Some(old) = player.playing.take() {
            let bar = 60.0 / old.cue.bpm * old.cue.beats_per_bar;
            let bars = if to == MusicState::Silent {
                cfg.fade_out_bars
            } else {
                cfg.crossfade_bars
            };
            let from_gain = sinks.get(old.voice).map(|s| s.volume().to_linear()).unwrap_or(base);
            player.fading.push(Fading {
                voice: old.voice,
                from_gain,
                from_s: now,
                fade_s: bars * bar,
            });
        }
        if let Some(cue) = to.key().and_then(|k| cfg.cues.get(k)) {
            let voice = commands
                .spawn((
                    Name::new(format!("music {:?}", to)),
                    AudioPlayer::new(assets.load(cue.file.clone())),
                    PlaybackSettings {
                        mode: if cue.looping { PlaybackMode::Loop } else { PlaybackMode::Once },
                        volume: Volume::Linear(0.0),
                        ..PlaybackSettings::ONCE
                    },
                ))
                .id();
            let bar = 60.0 / cue.bpm * cue.beats_per_bar;
            player.playing = Some(Track {
                state: to,
                voice,
                cue: cue.clone(),
                started_s: now,
                from_s: now,
                fade_s: cfg.crossfade_bars * bar,
            });
        }
    }

    // levels: the playing cue fades in, the old ones fade out and stop
    if let Some(t) = &player.playing
        && let Ok(mut s) = sinks.get_mut(t.voice)
    {
        let k = ((now - t.from_s) / t.fade_s.max(1e-3)).clamp(0.0, 1.0);
        s.set_volume(Volume::Linear(base * k));
    }
    player.fading.retain(|f| {
        let k = ((now - f.from_s) / f.fade_s.max(1e-3)).clamp(0.0, 1.0);
        if k >= 1.0 {
            commands.entity(f.voice).despawn();
            return false;
        }
        if let Ok(mut s) = sinks.get_mut(f.voice) {
            s.set_volume(Volume::Linear(f.from_gain * (1.0 - k)));
        }
        true
    });
}
