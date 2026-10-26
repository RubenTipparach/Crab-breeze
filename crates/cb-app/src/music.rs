//! Music (openspec change `music`): one cue per game state, switched on the bar with a one-bar crossfade (survey A2),
//! over everything like a film score and ducked under the radio and the alarms (survey A3; owner, 2026-10-26: "Approve
//! all"), and silence when nothing is happening (owner, 2026-10-25: "if theres not mutch action on screen, some
//! silence would be nice"). The rules are `cb_core::audio` (`music_state`, `plan_switch`, `fade_s`, `duck_target_db`);
//! this file reads their inputs from the game and plays what they return. Music is not placed in 3D, not delayed and
//! not muffled by the cockpit's seal. It runs in every state, not only in the game, because the title menu has its
//! cue and the score carries across the loading screen into the game.

use crate::audio::{Ear, HearSet, Voice};
use crate::blast::Explosion;
use crate::fx::Fx;
use crate::pilot::PilotSim;
use crate::sim::Data;
use crate::state::{GameApp, GameState};
use crate::units::Hostiles;
use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;
use cb_core::audio::{self as rules, MusicCue, MusicInputs, MusicState, Playing, Switch, db_to_gain};
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
    now: Playing,
    voice: Entity,
    cue: MusicCue,
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
    /// A change waiting for its bar line.
    switch: Option<Switch>,
    /// A cue that does not loop and has played out: it is not started again while its state holds.
    finished: Option<MusicState>,
    since_shot_s: f32,
    /// How far the score is ducked under the radio and alarms now, dB.
    duck_db: f32,
}

/// The music.
pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Player {
            since_shot_s: 1e9,
            ..Default::default()
        })
        .add_game_systems(Update, hear_shots.after(HearSet))
        .add_systems(Update, choose_and_play.after(hear_shots));
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

/// What the music rule reads from the game this frame. The title menu, and the loading screen after it, is the
/// title; in the game, the pilot, the hostiles, the fortress and the defence say the rest.
#[allow(clippy::too_many_arguments)]
fn inputs(
    state: GameState,
    ready: bool,
    pilot: &PilotSim,
    hostiles: Option<&Hostiles>,
    defence: Option<&crate::defence::DefenceRun>,
    fortress: Option<&crate::fortress::FortressRun>,
    ear: &Ear,
    since_shot_s: f32,
) -> MusicInputs {
    if state != GameState::Playing || !ready {
        return MusicInputs {
            title: true,
            since_shot_s: 1e9,
            ..Default::default()
        };
    }
    let fortress_won = fortress.is_some_and(|f| f.outcome == Some(true));
    MusicInputs {
        title: false,
        boarding: matches!(pilot.curr.mode, PilotMode::Boarding(_)),
        seated: pilot.curr.mode == PilotMode::Seated,
        threatened: hostiles.is_some_and(|h| h.0.iter().any(|x| !x.shelling && x.feet.distance(ear.at) < THREAT_WITHIN_M)),
        since_shot_s,
        // the set piece: the crab air fortress, aloft and fighting
        set_piece: fortress.is_some_and(|f| f.bound() && f.outcome.is_none()),
        won: fortress_won || defence.is_some_and(|r| matches!(r.state.phase, cb_core::mission::Phase::Won)),
    }
}

#[allow(clippy::too_many_arguments)]
fn choose_and_play(
    mut commands: Commands,
    mut player: ResMut<Player>,
    mut sinks: Query<&mut AudioSink>,
    voices: Query<&Voice>,
    state: Res<State<GameState>>,
    pilot: Res<PilotSim>,
    hostiles: Option<Res<Hostiles>>,
    defence: Option<Res<crate::defence::DefenceRun>>,
    fortress: Option<Res<crate::fortress::FortressRun>>,
    ear: Res<Ear>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    time: Res<Time>,
    ready: Res<crate::loading::GameReady>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let cfg = &data.0.audio.music;
    let now = time.elapsed_secs();
    let i = inputs(
        *state.get(),
        ready.0,
        &pilot,
        hostiles.as_deref(),
        defence.as_deref(),
        fortress.as_deref(),
        &ear,
        player.since_shot_s,
    );
    let wanted = rules::music_state(&i, cfg.hold_s);
    // under the radio and the alarms the score ducks, and comes back when they stop (survey A3)
    let duck = rules::duck_target_db(cfg, voices.iter().map(|v| v.cue()));
    player.duck_db = rules::duck_step(cfg, player.duck_db, duck, time.delta_secs());
    let base = db_to_gain(cfg.volume_db + player.duck_db) * settings.map_or(1.0, |s| s.0.music_volume);

    // a stinger that has played out stays finished while its state holds
    let played_out = player
        .playing
        .as_ref()
        .filter(|t| !t.cue.looping && sinks.get(t.voice).is_ok_and(|s| s.empty()))
        .map(|t| (t.now.state, t.voice));
    if let Some((s, voice)) = played_out {
        player.finished = Some(s);
        commands.entity(voice).despawn();
        player.playing = None;
    }
    if player.finished.is_some_and(|f| f != wanted) {
        player.finished = None;
    }
    let target = if player.finished == Some(wanted) {
        MusicState::Silent
    } else {
        wanted
    };
    player.switch = rules::plan_switch(player.playing.as_ref().map(|t| &t.now), target, player.switch, now);

    if let Some(sw) = player.switch
        && now >= sw.at_s
    {
        player.switch = None;
        let cue = sw.to.key().and_then(|k| cfg.cues.get(k));
        let fade_s = rules::fade_s(cfg, player.playing.as_ref().map(|t| &t.now), cue);
        if let Some(old) = player.playing.take() {
            let from_gain = sinks.get(old.voice).map(|s| s.volume().to_linear()).unwrap_or(base);
            player.fading.push(Fading {
                voice: old.voice,
                from_gain,
                from_s: now,
                fade_s,
            });
        }
        if let Some(cue) = cue {
            let voice = commands
                .spawn((
                    Name::new(format!("music {:?}", sw.to)),
                    AudioPlayer::new(assets.load(cue.file.clone())),
                    PlaybackSettings {
                        mode: if cue.looping { PlaybackMode::Loop } else { PlaybackMode::Once },
                        volume: Volume::Linear(0.0),
                        ..PlaybackSettings::ONCE
                    },
                ))
                .id();
            player.playing = Some(Track {
                now: Playing {
                    state: sw.to,
                    started_s: now,
                    bpm: cue.bpm,
                    beats_per_bar: cue.beats_per_bar,
                },
                voice,
                cue: cue.clone(),
                from_s: now,
                fade_s,
            });
        }
    }

    // levels: the playing cue fades in (ducked), the old ones fade out from where they were and stop
    if let Some(t) = &player.playing
        && let Ok(mut s) = sinks.get_mut(t.voice)
    {
        s.set_volume(Volume::Linear(base * rules::fade_progress(now, t.from_s, t.fade_s)));
    }
    player.fading.retain(|f| {
        let k = rules::fade_progress(now, f.from_s, f.fade_s);
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
