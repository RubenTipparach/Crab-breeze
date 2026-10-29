//! The game's sound rules, with no engine in them (openspec changes `audio`, `music`, `ambience`).
//!
//! What a sound is (its files, level, pitch range and where it plays from), how loud a placed sound
//! is at a distance and when it arrives, how the rockets, skates and torso set their loops, which
//! music cue the game's state calls for and when a change lands on the bar, and which ambience beds
//! fit where the listener is. All pure functions of their inputs, so they are tested here; `cb-app`
//! plays what they return. Data: `data/audio/cues.ron`, `music.ron`, `ambience.ron`.

use crate::data::{Validate, finite, positive, within};
use serde::Deserialize;
use std::collections::BTreeMap;

// ---------------------------------------------------------------- cues

/// Where a cue plays from (owner, 2026-10-25: "verify all sounds are 3d based (except ambience and
/// music and ui)"). Every sound effect is placed in 3D; only ambience, music and menus are not.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum Space {
    /// Placed in the world by whoever asks for it: at a node of a mech (a muzzle, a jet, a foot, a
    /// wheel, a hand) or at a point (an impact, a blast). Outside the cockpit: muffled through its
    /// seal when the pilot is seated.
    Placed,
    /// Placed inside the cockpit at a named speaker (`AudioConfig::speakers`), so it turns with the
    /// pilot's head; inside the seal, never muffled.
    Cockpit(String),
    /// A stereo ambience bed, not placed.
    Ambience,
    /// Music, not placed.
    Music,
    /// A menu sound, not placed.
    Ui,
}

impl Space {
    /// Whether the cue is heard from a point in 3D.
    pub fn is_placed(&self) -> bool {
        matches!(self, Space::Placed | Space::Cockpit(_))
    }
}

/// One named sound.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Cue {
    /// Files under `assets/` (`audio/sfx/...`), one picked at random each time it plays.
    pub files: Vec<String>,
    /// Its level, dB (0 is the file as written).
    pub volume_db: f32,
    /// The playback speed range it is picked from, (low, high).
    pub pitch: (f32, f32),
    /// Where it plays from.
    pub space: Space,
    /// Whether it loops (a loop is started and stopped by its owner, and its level set each frame).
    pub looping: bool,
    /// At most this many of it at once; a new one past it is dropped.
    pub max_voices: u32,
}

impl Default for Cue {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            volume_db: 0.0,
            pitch: (1.0, 1.0),
            space: Space::Placed,
            looping: false,
            max_voices: 4,
        }
    }
}

/// The listener and the mix (`data/audio/cues.ron`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct AudioConfig {
    /// The distance between the pilot's ears, m.
    pub ear_gap_m: f32,
    /// The speed of sound, m/s: a placed sound starts `distance / this` after its cause.
    pub speed_of_sound_m_s: f32,
    /// Within this a placed sound is at full level; beyond it the level falls as 1 / distance.
    pub full_level_within_m: f32,
    /// Beyond this the distant copy of a sound plays (low passed toward 2 kHz).
    pub far_beyond_m: f32,
    /// Past this a placed sound is not started at all, m.
    pub audible_within_m: f32,
    /// What the cockpit's seal takes off an outside sound when the pilot is seated, dB (its low
    /// pass is the muffled copy of the file).
    pub seal_db: f32,
    /// At most this many sounds at once; past it the quietest new one is dropped.
    pub max_voices: u32,
    /// The whole mix, dB.
    pub master_db: f32,
    /// The cockpit's speakers: name to its offset from the pilot's eye in the seat, m (x right,
    /// y up, z back, the seat's frame).
    pub speakers: BTreeMap<String, [f32; 3]>,
    /// Every sound effect, by name.
    pub cues: BTreeMap<String, Cue>,
    /// The effect recipe (`crate::fx::RECIPES`) that announces an event, to the cue it sounds:
    /// the effect and its sound are asked for by one message, so they cannot disagree.
    pub fx_sounds: BTreeMap<String, String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            ear_gap_m: 0.2,
            speed_of_sound_m_s: 343.0,
            full_level_within_m: 5.0,
            far_beyond_m: 600.0,
            audible_within_m: 4000.0,
            seal_db: -6.0,
            max_voices: 32,
            master_db: 0.0,
            speakers: BTreeMap::new(),
            cues: BTreeMap::new(),
            fx_sounds: BTreeMap::new(),
        }
    }
}

impl Validate for AudioConfig {
    fn validate(&self) -> Result<(), String> {
        positive("ear_gap_m", self.ear_gap_m)?;
        positive("speed_of_sound_m_s", self.speed_of_sound_m_s)?;
        positive("full_level_within_m", self.full_level_within_m)?;
        positive("far_beyond_m", self.far_beyond_m)?;
        positive("audible_within_m", self.audible_within_m)?;
        within("seal_db", self.seal_db, -60.0, 0.0)?;
        within("master_db", self.master_db, -60.0, 12.0)?;
        if self.max_voices == 0 {
            return Err("`max_voices` must be at least 1".into());
        }
        for (name, at) in &self.speakers {
            for v in at {
                finite(&format!("speakers.{name}"), *v)?;
            }
        }
        for (name, cue) in &self.cues {
            let field = |f: &str| format!("cues.{name}.{f}");
            if cue.files.is_empty() {
                return Err(format!("`{}` must name at least one file", field("files")));
            }
            within(&field("volume_db"), cue.volume_db, -60.0, 12.0)?;
            positive(&field("pitch"), cue.pitch.0)?;
            positive(&field("pitch"), cue.pitch.1)?;
            if cue.pitch.0 > cue.pitch.1 {
                return Err(format!("`{}` must be (low, high)", field("pitch")));
            }
            if cue.max_voices == 0 {
                return Err(format!("`{}` must be at least 1", field("max_voices")));
            }
            match &cue.space {
                Space::Cockpit(s) if !self.speakers.contains_key(s) => {
                    return Err(format!("`{}` names speaker `{s}`, which `speakers` lacks", field("space")));
                }
                // Sound effects are placed (owner, 2026-10-25), menu sounds excepted (`Ui`, `ui_feedback`); ambience
                // and music have their own files.
                Space::Ambience | Space::Music => {
                    return Err(format!(
                        "`{}`: a sound effect must be placed (Placed, Cockpit or a menu's Ui); ambience and music have their own files",
                        field("space")
                    ));
                }
                _ => {}
            }
        }
        for (recipe, cue) in &self.fx_sounds {
            if !crate::fx::RECIPES.contains(&recipe.as_str()) {
                return Err(format!("`fx_sounds.{recipe}` is not an effect recipe"));
            }
            if !self.cues.contains_key(cue) {
                return Err(format!("`fx_sounds.{recipe}` names cue `{cue}`, which `cues` lacks"));
            }
        }
        Ok(())
    }
}

/// The file a placed sound plays, chosen by how it is heard: the muffled copy through the cockpit's
/// seal, the distant copy past `far_beyond_m` (both low passed when they are rendered).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// The file as written.
    Near,
    /// Low passed at 4 kHz: heard through the sealed cockpit.
    Muffled,
    /// Low passed toward 2 kHz: heard from far off.
    Far,
}

impl Variant {
    /// The variant's path for a cue file: `audio/sfx/x.ogg` becomes `audio/sfx/muffled/x.ogg` or `audio/sfx/far/x.ogg`.
    pub fn path(self, file: &str) -> String {
        let sub = match self {
            Variant::Near => return file.to_string(),
            Variant::Muffled => "muffled",
            Variant::Far => "far",
        };
        match file.rsplit_once('/') {
            Some((dir, name)) => format!("{dir}/{sub}/{name}"),
            None => format!("{sub}/{file}"),
        }
    }
}

/// Decibels to a linear gain.
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// How a placed sound reaches the listener: its gain (before the cue's own level), its delay and
/// which copy of the file plays. `None` past `audible_within_m`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heard {
    /// Linear gain from distance and the seal.
    pub gain: f32,
    /// Seconds after its cause that it starts.
    pub delay_s: f32,
    /// The copy of the file to play.
    pub variant: Variant,
}

/// How a sound at `distance_m` is heard; `through_seal` when the pilot is seated and the sound is
/// outside the cockpit.
pub fn heard(cfg: &AudioConfig, distance_m: f32, through_seal: bool) -> Option<Heard> {
    if !distance_m.is_finite() || distance_m > cfg.audible_within_m {
        return None;
    }
    let d = distance_m.max(0.0);
    let mut gain = distance_gain(cfg.full_level_within_m, d);
    if through_seal {
        gain *= db_to_gain(cfg.seal_db);
    }
    let variant = if d > cfg.far_beyond_m {
        Variant::Far
    } else if through_seal {
        Variant::Muffled
    } else {
        Variant::Near
    };
    Some(Heard {
        gain,
        delay_s: d / cfg.speed_of_sound_m_s,
        variant,
    })
}

/// Full level within `full_within_m`, then 1 / distance (6 dB down per doubling).
pub fn distance_gain(full_within_m: f32, distance_m: f32) -> f32 {
    if distance_m <= full_within_m {
        1.0
    } else {
        full_within_m / distance_m
    }
}

// ---------------------------------------------------------------- loops

/// A loop's level, dB, and playback speed this frame; `None` is silent (the loop is stopped).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopLevel {
    /// Level, dB.
    pub db: f32,
    /// Playback speed.
    pub pitch: f32,
}

/// The two rocket loops and the cockpit shake while the rockets burn, from the lift (0..1) and
/// the air skates' thrust share (0..1): `audio` design sections 2 and 2b. One voice per loop at
/// the louder of the two levels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RocketSound {
    /// `rocket_loop`.
    pub roar: Option<LoopLevel>,
    /// `rocket_rumble`.
    pub rumble: Option<LoopLevel>,
    /// The cockpit's position shake, m (never its direction).
    pub shake_m: f32,
}

/// The rockets' sound from the lift (0 when the rockets are off) and the air skates' thrust share
/// (0 on the ground or with no direction held).
pub fn rocket_sound(lift: Option<f32>, air_skate: f32) -> RocketSound {
    let k = air_skate.clamp(0.0, 1.0);
    let skates = (k > 0.0).then(|| {
        (
            LoopLevel {
                db: -22.0 + 8.0 * k,
                pitch: 0.8 + 0.1 * k,
            },
            LoopLevel {
                db: -14.0 + 6.0 * k,
                pitch: 1.0,
            },
        )
    });
    let rockets = lift.map(|l| {
        let l = l.clamp(0.0, 1.0);
        (
            LoopLevel {
                db: -12.0 + 10.0 * l,
                pitch: 0.9 + 0.2 * l,
            },
            LoopLevel {
                db: -8.0 + 6.0 * l,
                pitch: 1.0,
            },
        )
    });
    let louder = |a: Option<LoopLevel>, b: Option<LoopLevel>| match (a, b) {
        (Some(a), Some(b)) => Some(if a.db >= b.db { a } else { b }),
        (a, b) => a.or(b),
    };
    RocketSound {
        roar: louder(rockets.map(|r| r.0), skates.map(|s| s.0)),
        rumble: louder(rockets.map(|r| r.1), skates.map(|s| s.1)),
        shake_m: lift.map_or(0.0, |l| 0.004 * l.clamp(0.0, 1.0)),
    }
}

/// The skates rolling: only with the skates on and the wheels on the ground, louder and higher with
/// speed (`audio` design section 2b); silent at a standstill.
pub fn skate_roll(skates_on: bool, on_ground: bool, speed_m_s: f32, top_speed_m_s: f32) -> Option<LoopLevel> {
    if !skates_on || !on_ground || !speed_m_s.is_finite() || top_speed_m_s <= 0.0 {
        return None;
    }
    let s = (speed_m_s.abs() / top_speed_m_s).clamp(0.0, 1.0);
    (s >= 0.01).then(|| LoopLevel {
        db: -14.0 + 10.0 * s,
        pitch: 0.8 + 0.4 * s,
    })
}

/// The torso's slew motor: playing while the torso turns, louder and higher with the turn rate
/// against its top rate; silent below 2 % of it.
pub fn torso_servo(turn_rate_deg_s: f32, top_rate_deg_s: f32) -> Option<LoopLevel> {
    if !turn_rate_deg_s.is_finite() || top_rate_deg_s <= 0.0 {
        return None;
    }
    let r = (turn_rate_deg_s.abs() / top_rate_deg_s).clamp(0.0, 1.0);
    (r >= 0.02).then(|| LoopLevel {
        db: -16.0 + 12.0 * r,
        pitch: 0.85 + 0.3 * r,
    })
}

/// Eases a level toward its target with time constant `tau_s` over `dt_s` (a fade, never a cut).
pub fn ease(current: f32, target: f32, dt_s: f32, tau_s: f32) -> f32 {
    if tau_s <= 0.0 {
        return target;
    }
    current + (target - current) * (1.0 - (-dt_s / tau_s).exp())
}

// ---------------------------------------------------------------- music

/// The music the game's state calls for (`music` design section 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MusicState {
    /// The title screen.
    Title,
    /// No music: on foot, or calm in the seat (owner, 2026-10-25: "some silence would be nice").
    Silent,
    /// The boarding sequence.
    Boarding,
    /// A threat or a recent shot.
    Combat,
    /// Big Boy is alive.
    SetPiece,
    /// The mission is won.
    Won,
}

impl MusicState {
    /// The key of this state's cue in `data/audio/music.ron`; `None` for silence.
    pub fn key(self) -> Option<&'static str> {
        match self {
            MusicState::Title => Some("title"),
            MusicState::Silent => None,
            MusicState::Boarding => Some("boarding"),
            MusicState::Combat => Some("combat"),
            MusicState::SetPiece => Some("set_piece"),
            MusicState::Won => Some("won"),
        }
    }
}

/// What the music rule reads from the game.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MusicInputs {
    /// The title screen is up.
    pub title: bool,
    /// The boarding sequence is running.
    pub boarding: bool,
    /// The pilot is seated.
    pub seated: bool,
    /// An enemy within 2 km has the mech as its target.
    pub threatened: bool,
    /// Seconds since enemy fire last landed near the mech (large when none). The pilot's own shots never count.
    pub since_shot_s: f32,
    /// Big Boy is alive in the wave.
    pub set_piece: bool,
    /// The mission is won.
    pub won: bool,
}

/// The music state. Later entries win: title, on foot (silent), boarding, combat (a threat, or a
/// shot within `hold_s`), set piece, won. Seated with none of them is calm: silent.
pub fn music_state(i: &MusicInputs, hold_s: f32) -> MusicState {
    let mut s = if i.title { MusicState::Title } else { MusicState::Silent };
    if i.boarding {
        s = MusicState::Boarding;
    }
    // Combat only while enemies are about and their fire is landing near the mech (owner, 2026-10-26: "shooting
    // should not trigger combat music, only if enemies are present and are firing at me").
    if i.seated && i.threatened && i.since_shot_s < hold_s {
        s = MusicState::Combat;
    }
    if i.seated && i.set_piece {
        s = MusicState::SetPiece;
    }
    if i.won {
        s = MusicState::Won;
    }
    s
}

/// The time of the next bar line at or after `t_s` for a cue started at `t0_s`.
pub fn next_bar_s(t_s: f32, t0_s: f32, bpm: f32, beats_per_bar: f32) -> f32 {
    let bar = bar_s(bpm, beats_per_bar);
    let bars = ((t_s - t0_s) / bar).max(0.0).ceil();
    t0_s + bars * bar
}

/// What is playing, for the switch rule: its state, when it started and its bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playing {
    /// The state whose cue it is.
    pub state: MusicState,
    /// When it started, s.
    pub started_s: f32,
    /// Its tempo.
    pub bpm: f32,
    /// Beats in its bar.
    pub beats_per_bar: f32,
}

impl Playing {
    /// One bar of this cue, s.
    pub fn bar_s(&self) -> f32 {
        bar_s(self.bpm, self.beats_per_bar)
    }
}

/// One bar at `bpm` with `beats_per_bar` beats, s.
pub fn bar_s(bpm: f32, beats_per_bar: f32) -> f32 {
    60.0 / bpm * beats_per_bar
}

/// The switch the music waits for: at `at_s`, to `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Switch {
    /// When, s: the next bar line of the cue that is playing (now, from silence).
    pub at_s: f32,
    /// To what.
    pub to: MusicState,
}

/// The switch pending after this frame (`music` design section 3; survey A2, owner 2026-10-26): none when the
/// music already plays `target`, so a state that flips back before the bar line cancels its switch; otherwise the
/// one already waiting for that target, or a new one on the next bar line of what plays (now, from silence).
pub fn plan_switch(playing: Option<&Playing>, target: MusicState, pending: Option<Switch>, now_s: f32) -> Option<Switch> {
    let current = playing.map_or(MusicState::Silent, |p| p.state);
    if target == current {
        return None;
    }
    if let Some(p) = pending.filter(|p| p.to == target) {
        return Some(p);
    }
    let at_s = match playing {
        Some(p) => next_bar_s(now_s, p.started_s, p.bpm, p.beats_per_bar),
        None => now_s,
    };
    Some(Switch { at_s, to: target })
}

/// How long a switch from `old` to `new` fades, s: into a cue, over `crossfade_bars` bars of the new cue (both
/// fade together); into silence, over `fade_out_bars` bars of the old one.
pub fn fade_s(cfg: &MusicConfig, old: Option<&Playing>, new: Option<&MusicCue>) -> f32 {
    match (new, old) {
        (Some(c), _) => cfg.crossfade_bars * bar_s(c.bpm, c.beats_per_bar),
        (None, Some(o)) => cfg.fade_out_bars * o.bar_s(),
        (None, None) => 0.0,
    }
}

/// The level the score ducks to this frame, dB (survey A3, owner 2026-10-26): `duck_db` while any cue in
/// `duck_under` plays (the radio and the alarms), else 0.
pub fn duck_target_db<'a>(cfg: &MusicConfig, playing: impl IntoIterator<Item = &'a str>) -> f32 {
    if playing.into_iter().any(|c| cfg.duck_under.iter().any(|d| d == c)) {
        cfg.duck_db
    } else {
        0.0
    }
}

/// Moves the duck from `current_db` toward `target_db` over `dt_s`: down in `duck_attack_s`, back up in
/// `duck_release_s`, never past the target.
pub fn duck_step(cfg: &MusicConfig, current_db: f32, target_db: f32, dt_s: f32) -> f32 {
    let tau = if target_db < current_db {
        cfg.duck_attack_s
    } else {
        cfg.duck_release_s
    };
    ease(current_db, target_db, dt_s, tau)
}

/// How far a fade begun at `from_s` and lasting `fade_s` has come at `now_s`, 0 to 1.
pub fn fade_progress(now_s: f32, from_s: f32, fade_s: f32) -> f32 {
    if fade_s <= 0.0 {
        return 1.0;
    }
    ((now_s - from_s) / fade_s).clamp(0.0, 1.0)
}

/// One music cue (`data/audio/music.ron`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct MusicCue {
    /// The file under `assets/`.
    pub file: String,
    /// Its tempo; `cb-tools validate` checks it against the score's `setcps`.
    pub bpm: f32,
    /// Beats in a bar.
    pub beats_per_bar: f32,
    /// Whether it loops.
    pub looping: bool,
}

impl Default for MusicCue {
    fn default() -> Self {
        Self {
            file: String::new(),
            bpm: 120.0,
            beats_per_bar: 4.0,
            looping: true,
        }
    }
}

/// `data/audio/music.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct MusicConfig {
    /// State key (`MusicState::key`) to its cue. Silence has no entry.
    pub cues: BTreeMap<String, MusicCue>,
    /// A change crossfades over this many bars of the new cue.
    pub crossfade_bars: f32,
    /// Fading to silence takes this many bars of the cue that was playing.
    pub fade_out_bars: f32,
    /// Combat holds its cue this long after the last threat or shot, s.
    pub hold_s: f32,
    /// The music's level, dB.
    pub volume_db: f32,
    /// How far the score ducks while a cue in `duck_under` plays, dB (survey A3: "ducks under the radio and the
    /// alarms").
    pub duck_db: f32,
    /// The sound effects the score ducks under (`data/audio/cues.ron` names): the radio and the alarms.
    pub duck_under: Vec<String>,
    /// The duck comes down with this time constant, s.
    pub duck_attack_s: f32,
    /// ...and goes back up with this one, s.
    pub duck_release_s: f32,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            cues: BTreeMap::new(),
            crossfade_bars: 1.0,
            fade_out_bars: 2.0,
            hold_s: 30.0,
            volume_db: -4.0,
            duck_db: -6.0,
            duck_under: Vec::new(),
            duck_attack_s: 0.05,
            duck_release_s: 0.6,
        }
    }
}

impl Validate for MusicConfig {
    fn validate(&self) -> Result<(), String> {
        positive("crossfade_bars", self.crossfade_bars)?;
        positive("fade_out_bars", self.fade_out_bars)?;
        positive("hold_s", self.hold_s)?;
        within("volume_db", self.volume_db, -60.0, 12.0)?;
        within("duck_db", self.duck_db, -40.0, 0.0)?;
        positive("duck_attack_s", self.duck_attack_s)?;
        positive("duck_release_s", self.duck_release_s)?;
        let known = ["title", "boarding", "combat", "set_piece", "won"];
        for (key, cue) in &self.cues {
            if !known.contains(&key.as_str()) {
                return Err(format!("`cues.{key}` is not a music state (one of {known:?})"));
            }
            if cue.file.is_empty() {
                return Err(format!("`cues.{key}.file` is empty"));
            }
            positive(&format!("cues.{key}.bpm"), cue.bpm)?;
            positive(&format!("cues.{key}.beats_per_bar"), cue.beats_per_bar)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- ambience

/// What the ambience rule reads around the listener (`ambience` design section 2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoneInputs {
    /// Distance to the nearest city's centre, m.
    pub city_distance_m: f32,
    /// That city's flat-ground radius, m.
    pub city_radius_m: f32,
    /// The share of warehouses among buildings within 200 m, 0..1.
    pub warehouse_share: f32,
    /// Trees within 80 m.
    pub trees_near: u32,
    /// The listener's height above the ground, m.
    pub height_m: f32,
    /// Night (from the hour, `day_night`).
    pub night: bool,
    /// Rain (false until the game has weather).
    pub rain: bool,
}

/// Each bed's weight, 0..1; the zone beds sum to 1. The wind is separate, by height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZoneWeights {
    /// `amb_meadow_day`.
    pub meadow: f32,
    /// `amb_night`.
    pub night: f32,
    /// `amb_forest`.
    pub forest: f32,
    /// `amb_city_street`.
    pub city: f32,
    /// `amb_city_night_rain`.
    pub city_night_rain: f32,
    /// `amb_industrial`.
    pub industrial: f32,
    /// `amb_rain`.
    pub rain: f32,
    /// The height share for `amb_wind_high`, 0 on the ground to 1 at 24 m.
    pub height: f32,
}

impl ZoneWeights {
    /// The weight for a bed by its cue name; 0 for an unknown one.
    pub fn of(&self, bed: &str) -> f32 {
        match bed {
            "amb_meadow_day" => self.meadow,
            "amb_night" => self.night,
            "amb_forest" => self.forest,
            "amb_city_street" => self.city,
            "amb_city_night_rain" => self.city_night_rain,
            "amb_industrial" => self.industrial,
            "amb_rain" => self.rain,
            _ => 0.0,
        }
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The beds' weights where the listener is.
pub fn ambience_weights(z: &ZoneInputs) -> ZoneWeights {
    let c = 1.0 - smoothstep(z.city_radius_m, z.city_radius_m + 300.0, z.city_distance_m.max(0.0));
    let i = c * z.warehouse_share.clamp(0.0, 1.0);
    let trees = (z.trees_near as f32 / 40.0).min(1.0);
    let f = (1.0 - c) * trees;
    let o = (1.0 - c) * (1.0 - trees);
    let mut w = ZoneWeights {
        height: smoothstep(2.0, 24.0, z.height_m),
        forest: f,
        industrial: i,
        ..Default::default()
    };
    let street = c - i;
    if z.rain || z.night {
        w.city_night_rain = street;
    } else {
        w.city = street;
    }
    if z.rain {
        w.rain = o;
    } else if z.night {
        w.night = o;
    } else {
        w.meadow = o;
    }
    w
}

/// One bed (`data/audio/ambience.ron`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Bed {
    /// The file under `assets/`.
    pub file: String,
    /// Its level at full weight, dB.
    pub volume_db: f32,
}

impl Default for Bed {
    fn default() -> Self {
        Self {
            file: String::new(),
            volume_db: -18.0,
        }
    }
}

/// `data/audio/ambience.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct AmbienceConfig {
    /// Bed name to its file and level.
    pub beds: BTreeMap<String, Bed>,
    /// The wind bed at height 0 and the dB it gains at full height.
    pub wind_db: (f32, f32),
    /// A bed's level eases toward its target with this time constant, s.
    pub ease_s: f32,
    /// A bed below this is stopped, dB.
    pub stop_below_db: f32,
    /// How often the zone is read, Hz.
    pub read_hz: f32,
    /// What the seal takes off the beds when the pilot is seated, dB.
    pub seal_db: f32,
}

impl Default for AmbienceConfig {
    fn default() -> Self {
        Self {
            beds: BTreeMap::new(),
            wind_db: (-18.0, 12.0),
            ease_s: 4.0,
            stop_below_db: -40.0,
            read_hz: 4.0,
            seal_db: -6.0,
        }
    }
}

impl Validate for AmbienceConfig {
    fn validate(&self) -> Result<(), String> {
        finite("wind_db", self.wind_db.0)?;
        finite("wind_db", self.wind_db.1)?;
        positive("ease_s", self.ease_s)?;
        within("stop_below_db", self.stop_below_db, -120.0, 0.0)?;
        positive("read_hz", self.read_hz)?;
        within("seal_db", self.seal_db, -60.0, 0.0)?;
        for (name, bed) in &self.beds {
            if bed.file.is_empty() {
                return Err(format!("`beds.{name}.file` is empty"));
            }
            within(&format!("beds.{name}.volume_db"), bed.volume_db, -60.0, 12.0)?;
        }
        Ok(())
    }
}

/// A bed's target level, dB, for its weight (equal power, so a crossfade keeps its loudness);
/// `None` at zero weight.
pub fn bed_db(volume_db: f32, weight: f32) -> Option<f32> {
    (weight > 1e-4).then(|| volume_db + 10.0 * weight.log10())
}

/// The three audio data files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioData {
    /// `data/audio/cues.ron`.
    pub cues: AudioConfig,
    /// `data/audio/music.ron`.
    pub music: MusicConfig,
    /// `data/audio/ambience.ron`.
    pub ambience: AmbienceConfig,
}

impl AudioData {
    /// What one file cannot check alone: every cue the music ducks under is a sound effect in `cues.ron`.
    pub fn check(&self) -> Result<(), String> {
        for name in &self.music.duck_under {
            if !self.cues.cues.contains_key(name) {
                return Err(format!("`duck_under` names `{name}`, which is not a cue in cues.ron"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AudioConfig {
        AudioConfig::default()
    }

    #[test]
    fn a_sound_within_five_metres_is_at_full_level() {
        assert_eq!(distance_gain(5.0, 3.0), 1.0);
        assert_eq!(distance_gain(5.0, 5.0), 1.0);
    }

    #[test]
    fn the_level_halves_with_each_doubling_of_distance() {
        assert!((distance_gain(5.0, 10.0) - 0.5).abs() < 1e-6);
        assert!((distance_gain(5.0, 20.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_shell_686_m_off_is_heard_two_seconds_late() {
        let h = heard(&cfg(), 686.0, false).unwrap();
        assert!((h.delay_s - 2.0).abs() < 1e-4);
        assert_eq!(h.variant, Variant::Far);
    }

    #[test]
    fn an_outside_sound_is_muffled_through_the_seal() {
        let open = heard(&cfg(), 50.0, false).unwrap();
        let sealed = heard(&cfg(), 50.0, true).unwrap();
        assert_eq!(sealed.variant, Variant::Muffled);
        assert!((sealed.gain / open.gain - db_to_gain(-6.0)).abs() < 1e-5);
    }

    #[test]
    fn a_sound_past_the_audible_range_is_not_played() {
        assert!(heard(&cfg(), 5000.0, false).is_none());
        assert!(heard(&cfg(), f32::NAN, false).is_none());
    }

    #[test]
    fn variant_paths_sit_beside_the_file() {
        assert_eq!(Variant::Muffled.path("sfx/cannon.ogg"), "sfx/muffled/cannon.ogg");
        assert_eq!(Variant::Far.path("sfx/cannon.ogg"), "sfx/far/cannon.ogg");
        assert_eq!(Variant::Near.path("sfx/cannon.ogg"), "sfx/cannon.ogg");
    }

    #[test]
    fn full_lift_is_ten_db_louder_than_idle_lift() {
        let idle = rocket_sound(Some(0.0), 0.0);
        let full = rocket_sound(Some(1.0), 0.0);
        assert!((full.roar.unwrap().db - idle.roar.unwrap().db - 10.0).abs() < 1e-5);
        assert!((full.shake_m - 0.004).abs() < 1e-6);
    }

    #[test]
    fn air_skates_rumble_lower_than_the_rockets_and_never_shake() {
        let skates = rocket_sound(None, 1.0);
        let rockets = rocket_sound(Some(1.0), 0.0);
        assert!(skates.rumble.unwrap().db < rockets.rumble.unwrap().db);
        assert!(skates.roar.unwrap().db < rockets.roar.unwrap().db);
        assert_eq!(skates.shake_m, 0.0);
    }

    #[test]
    fn rockets_and_air_skates_together_take_the_louder_level_once() {
        let both = rocket_sound(Some(0.0), 1.0);
        // skates' rumble (-8 dB) beats idle rockets' rumble (-8 dB): equal, either is the one voice
        assert!((both.rumble.unwrap().db - -8.0).abs() < 1e-5);
        // skates' roar (-14 dB) loses to idle rockets' roar (-12 dB)
        assert!((both.roar.unwrap().db - -12.0).abs() < 1e-5);
    }

    #[test]
    fn nothing_burns_with_the_rockets_off_and_no_air_skates() {
        let s = rocket_sound(None, 0.0);
        assert!(s.roar.is_none() && s.rumble.is_none());
    }

    #[test]
    fn the_skates_roll_only_on_the_ground() {
        assert!(skate_roll(true, true, 80.0, 120.0).is_some());
        assert!(skate_roll(true, false, 80.0, 120.0).is_none());
        assert!(skate_roll(false, true, 80.0, 120.0).is_none());
        assert!(skate_roll(true, true, 0.0, 120.0).is_none());
    }

    #[test]
    fn the_skates_are_louder_and_higher_at_speed() {
        let slow = skate_roll(true, true, 20.0, 120.0).unwrap();
        let fast = skate_roll(true, true, 110.0, 120.0).unwrap();
        assert!(fast.db > slow.db && fast.pitch > slow.pitch);
    }

    #[test]
    fn the_torso_servo_is_silent_when_the_torso_is_still() {
        assert!(torso_servo(0.0, 60.0).is_none());
        assert!(torso_servo(30.0, 60.0).is_some());
    }

    #[test]
    fn easing_moves_toward_the_target_and_never_past_it() {
        let x = ease(0.0, 1.0, 1.0, 4.0);
        assert!(x > 0.0 && x < 1.0);
        assert_eq!(ease(0.3, 1.0, 1.0, 0.0), 1.0);
    }

    fn seated() -> MusicInputs {
        MusicInputs {
            seated: true,
            since_shot_s: 1e9,
            ..Default::default()
        }
    }

    #[test]
    fn calm_in_the_seat_is_silent() {
        assert_eq!(music_state(&seated(), 30.0), MusicState::Silent);
    }

    #[test]
    fn on_foot_is_silent_even_under_fire() {
        let i = MusicInputs {
            threatened: true,
            ..Default::default()
        };
        assert_eq!(music_state(&i, 30.0), MusicState::Silent);
    }

    #[test]
    fn combat_holds_thirty_seconds_after_the_last_shot() {
        let mut i = seated();
        i.threatened = true;
        i.since_shot_s = 29.0;
        assert_eq!(music_state(&i, 30.0), MusicState::Combat);
        i.since_shot_s = 31.0;
        assert_eq!(music_state(&i, 30.0), MusicState::Silent);
    }

    #[test]
    fn enemies_about_but_not_firing_is_still_silent() {
        let mut i = seated();
        i.threatened = true;
        assert_eq!(music_state(&i, 30.0), MusicState::Silent);
    }

    #[test]
    fn fire_with_no_enemy_about_is_silent() {
        let mut i = seated();
        i.since_shot_s = 1.0;
        assert_eq!(music_state(&i, 30.0), MusicState::Silent);
    }

    #[test]
    fn a_set_piece_beats_combat_and_a_win_beats_everything() {
        let mut i = seated();
        i.threatened = true;
        i.set_piece = true;
        assert_eq!(music_state(&i, 30.0), MusicState::SetPiece);
        i.won = true;
        assert_eq!(music_state(&i, 30.0), MusicState::Won);
    }

    #[test]
    fn the_title_plays_its_theme() {
        let i = MusicInputs {
            title: true,
            since_shot_s: 1e9,
            ..Default::default()
        };
        assert_eq!(music_state(&i, 30.0), MusicState::Title);
    }

    #[test]
    fn a_change_lands_on_the_next_bar_line() {
        // 120 BPM in 4: a bar is 2 s. Started at 1 s, a change at 4.5 s lands at 5 s.
        assert!((next_bar_s(4.5, 1.0, 120.0, 4.0) - 5.0).abs() < 1e-5);
        assert!((next_bar_s(5.0, 1.0, 120.0, 4.0) - 5.0).abs() < 1e-5);
    }

    fn playing(state: MusicState, started_s: f32) -> Playing {
        // 120 BPM in 4: a bar is 2 s
        Playing {
            state,
            started_s,
            bpm: 120.0,
            beats_per_bar: 4.0,
        }
    }

    #[test]
    fn every_state_but_silence_has_its_own_cue() {
        // survey A2 (owner, 2026-10-26: "Approve all"): title, boarding, combat, set piece, won; on foot is silent
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/audio");
        let music: MusicConfig = crate::data::load_ron(&root.join("music.ron")).expect("music.ron");
        let states = [
            MusicState::Title,
            MusicState::Boarding,
            MusicState::Combat,
            MusicState::SetPiece,
            MusicState::Won,
        ];
        let mut files: Vec<&str> = states.iter().map(|s| music.cues[s.key().unwrap()].file.as_str()).collect();
        files.sort_unstable();
        files.dedup();
        assert_eq!(files.len(), states.len(), "one cue per state, none shared");
        assert_eq!(MusicState::Silent.key(), None);
        assert_eq!(music.crossfade_bars, 1.0, "a one-bar crossfade");
    }

    #[test]
    fn a_switch_waits_for_the_bar_line_of_what_plays() {
        let p = playing(MusicState::Boarding, 1.0);
        let sw = plan_switch(Some(&p), MusicState::Combat, None, 4.5).unwrap();
        assert_eq!(sw.to, MusicState::Combat);
        assert!((sw.at_s - 5.0).abs() < 1e-5);
    }

    #[test]
    fn from_silence_a_cue_starts_at_once() {
        let sw = plan_switch(None, MusicState::Combat, None, 7.25).unwrap();
        assert_eq!(sw.at_s, 7.25);
    }

    #[test]
    fn a_state_that_flips_back_before_the_bar_cancels_its_switch() {
        let p = playing(MusicState::Combat, 0.0);
        let sw = plan_switch(Some(&p), MusicState::Silent, None, 0.5);
        assert!(sw.is_some());
        assert_eq!(plan_switch(Some(&p), MusicState::Combat, sw, 0.9), None);
    }

    #[test]
    fn a_waiting_switch_keeps_its_bar_line() {
        // asked again next frame, the switch does not slide to a later bar
        let p = playing(MusicState::Combat, 0.0);
        let sw = plan_switch(Some(&p), MusicState::Won, None, 0.5);
        assert_eq!(plan_switch(Some(&p), MusicState::Won, sw, 1.9), sw);
    }

    #[test]
    fn a_crossfade_takes_one_bar_of_the_new_cue_and_silence_two_of_the_old() {
        let cfg = MusicConfig::default();
        let old = playing(MusicState::Combat, 0.0);
        let new = MusicCue {
            bpm: 60.0,
            beats_per_bar: 4.0,
            ..Default::default()
        };
        assert!((fade_s(&cfg, Some(&old), Some(&new)) - 4.0).abs() < 1e-5);
        assert!((fade_s(&cfg, Some(&old), None) - 4.0).abs() < 1e-5);
        assert_eq!(fade_progress(1.0, 0.0, 4.0), 0.25);
        assert_eq!(fade_progress(9.0, 0.0, 4.0), 1.0);
        assert_eq!(fade_progress(0.0, 0.0, 0.0), 1.0);
    }

    fn ducking() -> MusicConfig {
        MusicConfig {
            duck_under: vec!["radio_squelch".into(), "alarm_warning".into(), "heat_lock".into()],
            ..Default::default()
        }
    }

    #[test]
    fn the_score_ducks_six_db_under_the_radio_and_the_alarms() {
        // survey A3 (owner, 2026-10-26: "Approve all")
        let cfg = ducking();
        assert_eq!(duck_target_db(&cfg, ["radio_squelch"]), -6.0);
        assert_eq!(duck_target_db(&cfg, ["foot_stomp", "alarm_warning"]), -6.0);
        assert_eq!(duck_target_db(&cfg, ["heat_lock"]), -6.0);
    }

    #[test]
    fn the_score_plays_over_everything_else() {
        let cfg = ducking();
        assert_eq!(duck_target_db(&cfg, ["foot_stomp", "lock_tone", "cannon", "reactor_hum"]), 0.0);
        assert_eq!(duck_target_db(&cfg, std::iter::empty::<&str>()), 0.0);
    }

    #[test]
    fn the_duck_comes_down_fast_and_recovers_slowly() {
        let cfg = ducking();
        let down = duck_step(&cfg, 0.0, -6.0, 0.05);
        let up = duck_step(&cfg, -6.0, 0.0, 0.05);
        assert!(down < -3.0, "most of the way down in one attack time: {down}");
        assert!(up < -5.0, "barely back up in the same time: {up}");
        assert!(duck_step(&cfg, -6.0, -6.0, 1.0) == -6.0);
        let mut db = -6.0;
        for _ in 0..300 {
            db = duck_step(&cfg, db, 0.0, 1.0 / 60.0);
            assert!(db <= 0.0, "never past the target");
        }
        assert!(db > -0.01);
    }

    #[test]
    fn ducking_names_only_real_cues() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/audio");
        let mut audio = AudioData {
            cues: crate::data::load_ron(&root.join("cues.ron")).expect("cues.ron"),
            music: crate::data::load_ron(&root.join("music.ron")).expect("music.ron"),
            ambience: crate::data::load_ron(&root.join("ambience.ron")).expect("ambience.ron"),
        };
        assert!(audio.check().is_ok());
        assert!(
            audio.music.duck_under.iter().any(|c| c == "radio_squelch"),
            "the radio ducks the score"
        );
        assert!(
            audio.music.duck_under.iter().any(|c| c == "alarm_warning"),
            "the alarm ducks the score"
        );
        audio.music.duck_under.push("no_such_cue".into());
        assert!(audio.check().unwrap_err().contains("no_such_cue"));
    }

    #[test]
    fn a_duck_that_boosts_is_refused() {
        let cfg = MusicConfig {
            duck_db: 3.0,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    fn zone() -> ZoneInputs {
        ZoneInputs {
            city_distance_m: 5000.0,
            city_radius_m: 400.0,
            warehouse_share: 0.0,
            trees_near: 0,
            height_m: 1.7,
            night: false,
            rain: false,
        }
    }

    fn sum(w: &ZoneWeights) -> f32 {
        w.meadow + w.night + w.forest + w.city + w.city_night_rain + w.industrial + w.rain
    }

    #[test]
    fn the_city_centre_is_all_city() {
        let mut z = zone();
        z.city_distance_m = 0.0;
        let w = ambience_weights(&z);
        assert!((w.city - 1.0).abs() < 1e-6);
    }

    #[test]
    fn open_ground_far_from_a_city_is_meadow() {
        let w = ambience_weights(&zone());
        assert!((w.meadow - 1.0).abs() < 1e-6);
    }

    #[test]
    fn weights_sum_to_one() {
        for d in [0.0, 300.0, 500.0, 650.0, 2000.0] {
            for t in [0, 10, 40, 90] {
                for s in [0.0, 0.5, 1.0] {
                    let mut z = zone();
                    z.city_distance_m = d;
                    z.trees_near = t;
                    z.warehouse_share = s;
                    assert!((sum(&ambience_weights(&z)) - 1.0).abs() < 1e-5, "d {d} t {t} s {s}");
                }
            }
        }
    }

    #[test]
    fn warehouses_make_a_district_industrial() {
        let mut z = zone();
        z.city_distance_m = 0.0;
        z.warehouse_share = 0.75;
        let w = ambience_weights(&z);
        assert!((w.industrial - 0.75).abs() < 1e-6 && (w.city - 0.25).abs() < 1e-6);
    }

    #[test]
    fn the_seat_is_windier_than_the_street() {
        let mut z = zone();
        let street = ambience_weights(&z).height;
        z.height_m = 24.0;
        assert!(ambience_weights(&z).height > street);
        assert_eq!(street, 0.0);
    }

    #[test]
    fn night_beds_wait_for_a_time_of_day() {
        let w = ambience_weights(&zone());
        assert_eq!(w.night, 0.0);
        assert_eq!(w.rain, 0.0);
        assert_eq!(w.city_night_rain, 0.0);
    }

    #[test]
    fn equal_power_keeps_a_half_and_half_crossfade_three_db_down() {
        assert!((bed_db(0.0, 0.5).unwrap() - -3.0103).abs() < 1e-3);
        assert!(bed_db(0.0, 0.0).is_none());
    }

    #[test]
    fn a_sound_effect_left_unplaced_is_refused() {
        let mut c = cfg();
        c.cues.insert(
            "boom".into(),
            Cue {
                files: vec!["sfx/boom.ogg".into()],
                space: Space::Ambience,
                ..Default::default()
            },
        );
        assert!(c.validate().unwrap_err().contains("must be placed"));
    }

    #[test]
    fn an_effect_can_only_sound_a_cue_that_exists() {
        let mut c = cfg();
        c.fx_sounds.insert("impact_mech".into(), "clang".into());
        assert!(c.validate().is_err());
        c.fx_sounds.clear();
        c.fx_sounds.insert("no_such_effect".into(), "clang".into());
        assert!(c.validate().is_err());
    }

    #[test]
    fn the_shipped_cue_file_loads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/audio");
        let cues: AudioConfig = crate::data::load_ron(&root.join("cues.ron")).expect("cues.ron");
        assert!(cues.cues.values().all(|c| c.space.is_placed()));
        let _: MusicConfig = crate::data::load_ron(&root.join("music.ron")).expect("music.ron");
        let _: AmbienceConfig = crate::data::load_ron(&root.join("ambience.ron")).expect("ambience.ron");
    }

    #[test]
    fn a_cockpit_cue_needs_its_speaker() {
        let mut c = cfg();
        c.cues.insert(
            "beep".into(),
            Cue {
                files: vec!["sfx/beep.ogg".into()],
                space: Space::Cockpit("warning".into()),
                ..Default::default()
            },
        );
        assert!(c.validate().is_err());
        c.speakers.insert("warning".into(), [0.0, -0.25, -0.6]);
        assert!(c.validate().is_ok());
    }
}
