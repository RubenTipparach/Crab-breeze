//! Sound effects (openspec change `audio`): every one placed in 3D and heard from the pilot's head
//! (owner, 2026-10-25: "verify all sounds are 3d based (except ambience and music and ui)").
//!
//! Other plugins ask for a sound by cue name with a `Sound` message, or keep a loop going by sending
//! a `Loop` message every frame it should play; most sounds ride messages the game already sends
//! (`Fx` effects through `fx_sounds` in `data/audio/cues.ron`, `Explosion`). This plugin turns them
//! into voices: placed at a point, riding a mech's socket, or at a cockpit speaker; started after
//! the time sound takes to arrive; muffled through the cockpit's seal; set each frame to the level
//! `cb_core::audio` says for their distance; and capped. The rules are `cb_core::audio`.
//!
//! Bevy 0.20 places a sound by panning between two ears with rodio's inverse-square falloff, which
//! is far too steep for a battlefield (-40 dB at 10 m). The positions are scaled down so rodio's
//! falloff never applies, its panning (which depends only on the difference between the ears'
//! distances) still does, and the level is our own law, set as the voice's volume.
//! Presentation only: nothing here decides an outcome, so it uses its own cheap random numbers.

use crate::blast::Explosion;
use crate::fx::Fx;
use crate::mech::{MechRig, MechRoot, MechSim, MechSpec, Piloted};
use crate::pilot::{PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet, SimSet};
use crate::state::GameApp;
use crate::targeting::TargetState;
use bevy::audio::{
    AudioPlayer, AudioSinkPlayback, AudioSource, PlaybackMode, PlaybackSettings, SpatialAudioSink, SpatialListener, SpatialScale, Volume,
};
use bevy::prelude::*;
use cb_core::audio::{self as rules, AudioConfig, LoopLevel, Space, Variant, db_to_gain};
use cb_core::pilot::{PilotEvent, PilotMode};
use std::collections::HashMap;

/// The scale Bevy's spatial audio sees positions at: 4 km becomes 0.4, inside rodio's unit radius
/// where its own falloff is 1, so only the panning and our level apply.
pub const SPATIAL_SCALE: f32 = 1e-4;
// 4 km stays inside rodio's unit radius, where its falloff of min(1, 1 / d^2) never applies.
const _: () = assert!(4000.0 * SPATIAL_SCALE < 1.0);
/// A sound that starts this close to a mech rides it, so a muzzle blast or a footstep is not left
/// behind a mech skating at 100 m/s, m.
const RIDE_WITHIN_M: f32 = 15.0;
/// Loops are not kept for mechs farther than this from the listener, m.
const LOOPS_WITHIN_M: f32 = 1500.0;
/// A loop's level eases toward its target with this time constant, s (a fade, never a click).
const LOOP_EASE_S: f32 = 0.12;
/// A loop no longer asked for is stopped once it has faded below this, dB.
const LOOP_SILENT_DB: f32 = -60.0;
/// Explosions at or above this energy sound large, MJ (`audio` design section 1).
const LARGE_BLAST_MJ: f32 = 3.0;

/// Where a sound plays from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoundAt {
    /// A point in the world (it rides a mech if one is within `RIDE_WITHIN_M`).
    Point(Vec3),
    /// A node it rides (a mech's socket).
    Node(Entity),
    /// The cue's own cockpit speaker (`Space::Cockpit`).
    Speaker,
}

/// Play cue `cue` once.
#[derive(Message, Clone, Copy, Debug)]
pub struct Sound {
    /// A cue name in `data/audio/cues.ron`.
    pub cue: &'static str,
    /// Where.
    pub at: SoundAt,
}

impl Sound {
    /// At a point.
    pub fn at(cue: &'static str, at: Vec3) -> Self {
        Self {
            cue,
            at: SoundAt::Point(at),
        }
    }

    /// Riding a node.
    pub fn on(cue: &'static str, node: Entity) -> Self {
        Self {
            cue,
            at: SoundAt::Node(node),
        }
    }

    /// At the cue's cockpit speaker.
    pub fn speaker(cue: &'static str) -> Self {
        Self { cue, at: SoundAt::Speaker }
    }
}

/// Keep loop `cue` playing at `at` this frame, at `level`. A loop not asked for in a frame fades
/// out and stops.
#[derive(Message, Clone, Copy, Debug)]
pub struct Loop {
    /// A looping cue in `data/audio/cues.ron`.
    pub cue: &'static str,
    /// A node or the speaker (a loop is keyed by its cue and where it plays).
    pub at: SoundAt,
    /// Its level and speed.
    pub level: LoopLevel,
}

/// Every cue the game's code asks for by name; a test checks each is in `data/audio/cues.ron`.
pub const ASKED: &[&str] = &[
    "foot_stomp",
    "foot_splash",
    "servo_torso",
    "hydraulic",
    "hatch_seal",
    "hatch_steam",
    "hatch_motor",
    "jet_ignite",
    "rocket_loop",
    "rocket_rumble",
    "skate_roll",
    "reactor_hum",
    "lock_tone",
    "lock_on",
    "explosion_small",
    "explosion_large",
    "heat_lock",
    "alarm_warning",
    "mfd_click",
    "radio_squelch",
    "systems_boot",
    crate::air_traffic::ENGINE_CUES[0],
    crate::air_traffic::ENGINE_CUES[1],
    crate::air_traffic::ENGINE_CUES[2],
];

/// The listener is placed for this frame: music and ambience read `Ear` after it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HearSet;

/// Every cue's files, loaded once, by path under `assets/`.
#[derive(Resource, Default)]
struct Library(HashMap<String, Handle<AudioSource>>);

/// The listener: the pilot's head.
#[derive(Resource, Default)]
pub struct Ear {
    /// Where the head is.
    pub at: Vec3,
    /// The pilot is seated and the hatch closed: outside sounds come through the seal.
    pub sealed: bool,
    /// The pilot is seated (cockpit speakers and the reactor play).
    pub seated: bool,
    /// The seat's eye socket: cockpit speakers ride it, so they stay put while the head turns.
    pub seat: Option<Entity>,
}

/// A sound waiting for its sound to arrive.
struct Due {
    at_s: f32,
    cue: &'static str,
    at: SoundAt,
}

#[derive(Resource, Default)]
struct Pending(Vec<Due>);

/// A playing sound. The music reads which cues are playing, to duck under the radio and the alarms.
#[derive(Component)]
pub struct Voice {
    cue: &'static str,
    /// The cue's own level, linear (before distance and the seal).
    level: f32,
    /// Inside the cockpit (a speaker): never muffled, never far.
    inside: bool,
}

impl Voice {
    /// The cue this voice plays (its `data/audio/cues.ron` name).
    pub fn cue(&self) -> &'static str {
        self.cue
    }
}

/// Where a loop is, for keying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LoopAt {
    Node(Entity),
    Speaker,
}

/// A playing loop.
#[derive(Component)]
struct LoopVoice {
    db: f32,
    target: Option<LoopLevel>,
}

#[derive(Resource, Default)]
struct Loops(HashMap<(&'static str, LoopAt), Entity>);

/// Presentation-only random numbers (xorshift).
#[derive(Resource)]
struct Dice(u64);

impl Dice {
    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Sound effects in 3D.
pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Sound>()
            .add_message::<Loop>()
            .init_resource::<Library>()
            .init_resource::<Ear>()
            .init_resource::<Pending>()
            .init_resource::<Loops>()
            .insert_resource(Dice(0x9E37_79B9_7F4A_7C15))
            .add_world_spawn(load_library)
            .add_game_systems(
                FixedUpdate,
                (
                    pilot_sounds.in_set(SimSet::Pilot).after(crate::pilot::step_pilot),
                    control_clicks.in_set(SimSet::Pilot).before(crate::pilot::step_pilot),
                ),
            )
            .add_game_systems(
                Update,
                (
                    (attach_listener, track_ear).chain().in_set(HearSet).after(PresentSet::Camera),
                    (fx_sounds, blast_sounds, mech_sounds, hatch_sounds, cockpit_sounds, mission_sounds).after(track_ear),
                    (ask, start_due, drive_loops, level_voices)
                        .chain()
                        .after(fx_sounds)
                        .after(blast_sounds)
                        .after(mech_sounds)
                        .after(hatch_sounds)
                        .after(cockpit_sounds)
                        .after(mission_sounds),
                ),
            );
    }
}

fn load_library(mut lib: ResMut<Library>, assets: Res<AssetServer>, data: Res<Data>, mut volume: ResMut<GlobalVolume>) {
    let cfg = &data.0.audio.cues;
    *volume = GlobalVolume::new(Volume::Linear(db_to_gain(cfg.master_db)));
    for cue in ASKED.iter().filter(|c| !cfg.cues.contains_key(**c)) {
        error!("sound `{cue}` is asked for by the game but is not in data/audio/cues.ron");
    }
    for cue in cfg.cues.values() {
        for file in &cue.files {
            for v in [Variant::Near, Variant::Muffled, Variant::Far] {
                let path = v.path(file);
                lib.0.entry(path.clone()).or_insert_with(|| assets.load(path));
            }
        }
    }
}

/// The pilot's eyes are the listener.
fn attach_listener(mut commands: Commands, cams: Query<Entity, (With<PilotCamera>, Without<SpatialListener>)>, data: Res<Data>) {
    for e in &cams {
        commands.entity(e).insert(SpatialListener::new(data.0.audio.cues.ear_gap_m));
    }
}

fn track_ear(
    mut ear: ResMut<Ear>,
    cam: Single<&Transform, With<PilotCamera>>,
    pilot: Res<PilotSim>,
    mech: Query<(&MechSim, Option<&MechRig>), With<Piloted>>,
    data: Res<Data>,
) {
    ear.at = cam.translation;
    ear.seated = pilot.curr.mode == PilotMode::Seated;
    let (closed, seat) = mech
        .iter()
        .next()
        .map(|(sim, rig)| (sim.hatch.is_closed(&data.0.boarding), rig.and_then(|r| r.socket("EYE_pilot"))))
        .unwrap_or((false, None));
    ear.sealed = ear.seated && closed;
    ear.seat = seat;
}

// ---------------------------------------------------------------- the game's events

/// Effects the game already announces sound through `fx_sounds`: muzzles, impacts, footfalls,
/// splashes, smashes and collapses.
fn fx_sounds(mut fx: MessageReader<Fx>, data: Res<Data>, mut sounds: MessageWriter<Sound>) {
    let map = &data.0.audio.cues.fx_sounds;
    for f in fx.read() {
        if let Some((cue, _)) = map.get(f.recipe).and_then(|c| data.0.audio.cues.cues.get_key_value(c.as_str())) {
            sounds.write(Sound::at(cue_name(cue), f.at));
        }
    }
}

/// The `&'static str` for a cue name from the data: the data is loaded once and lives for the
/// run, so its names are leaked once each.
fn cue_name(name: &str) -> &'static str {
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES.get_or_init(Default::default).lock().expect("cue names");
    if let Some(n) = names.get(name) {
        return n;
    }
    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.insert(name.to_string(), leaked);
    leaked
}

fn blast_sounds(mut blasts: MessageReader<Explosion>, mut sounds: MessageWriter<Sound>) {
    for b in blasts.read() {
        let cue = if b.energy_mj >= LARGE_BLAST_MJ {
            "explosion_large"
        } else {
            "explosion_small"
        };
        sounds.write(Sound::at(cue, b.at));
    }
}

/// What a mech's sounds remember between frames, to sound an edge once.
#[derive(Default, Clone, Copy)]
struct MechEdges {
    boosting: bool,
    airborne: bool,
    boost_empty: bool,
    skate_empty: bool,
    parts_lost: usize,
    twist_rad: f32,
}

/// Every mech's loops (rockets, air skates, wheels, the torso), and the piloted mech's edges: the
/// jets lighting, the legs taking a landing, a budget emptying, a part lost.
#[allow(clippy::too_many_arguments)]
fn mech_sounds(
    mechs: Query<(Entity, &MechSim, &MechSpec, Option<&MechRig>, &GlobalTransform, Has<Piloted>), With<MechRoot>>,
    ear: Res<Ear>,
    time: Res<Time>,
    mut seen: Local<HashMap<Entity, MechEdges>>,
    mut sounds: MessageWriter<Sound>,
    mut loops: MessageWriter<Loop>,
) {
    let dt = time.delta_secs().max(1e-4);
    for (e, sim, spec, rig, at, piloted) in &mechs {
        let s = &sim.curr;
        let near = at.translation().distance(ear.at) < LOOPS_WITHIN_M;
        let socket = |n: &str| rig.and_then(|r| r.socket(n)).unwrap_or(e);
        let was = *seen.entry(e).or_insert(MechEdges {
            twist_rad: s.twist_rad,
            ..Default::default()
        });
        if near && !sim.dead {
            // the rockets and the air skates (design 2, 2b): one voice per loop per jet
            let air_skate = if s.in_air && s.skates_on && s.skate.idle_s == 0.0 {
                1.0
            } else {
                0.0
            };
            let r = rules::rocket_sound(s.boosting.then_some(1.0), air_skate);
            for jet in ["JET_L", "JET_R"] {
                if let Some(l) = r.roar {
                    loops.write(Loop::on("rocket_loop", socket(jet), l));
                }
                if let Some(l) = r.rumble {
                    loops.write(Loop::on("rocket_rumble", socket(jet), l));
                }
            }
            if let Some(l) = rules::skate_roll(s.skates_on, !s.airborne(), s.speed_m_s, spec.skate.speed_max_m_s) {
                for wheel in ["WHEEL_L", "WHEEL_R"] {
                    loops.write(Loop::on("skate_roll", socket(wheel), l));
                }
            }
            if piloted {
                let rate = (s.twist_rad - was.twist_rad).to_degrees() / dt;
                if let Some(l) = rules::torso_servo(rate, spec.locomotion.twist_rate_deg_s) {
                    loops.write(Loop::on("servo_torso", rig.and_then(|r| r.head()).unwrap_or(e), l));
                }
            }
        }
        let lost = sim.damage.lost.iter().filter(|l| **l).count();
        if piloted {
            if s.boosting && !was.boosting {
                sounds.write(Sound::on("jet_ignite", socket("JET_L")));
            }
            if was.airborne && !s.airborne() {
                sounds.write(Sound::on("hydraulic", e));
            }
            if ear.seated {
                if (s.boost.locked && !was.boost_empty) || (s.skate.locked && !was.skate_empty) {
                    sounds.write(Sound::speaker("heat_lock"));
                }
                if lost > was.parts_lost {
                    sounds.write(Sound::speaker("alarm_warning"));
                }
                loops.write(Loop {
                    cue: "reactor_hum",
                    at: SoundAt::Speaker,
                    level: LoopLevel { db: 0.0, pitch: 1.0 },
                });
            }
        }
        seen.insert(
            e,
            MechEdges {
                boosting: s.boosting,
                airborne: s.airborne(),
                boost_empty: s.boost.locked,
                skate_empty: s.skate.locked,
                parts_lost: lost,
                twist_rad: s.twist_rad,
            },
        );
    }
    seen.retain(|e, _| mechs.contains(*e));
}

impl Loop {
    fn on(cue: &'static str, node: Entity, level: LoopLevel) -> Self {
        Self {
            cue,
            at: SoundAt::Node(node),
            level,
        }
    }
}

/// The hatch: a motor for each stage it runs, steam for a stage that vents, the seal when it closes
/// on the pilot or first unlatches.
fn hatch_sounds(
    mech: Query<(Entity, &MechSim, Option<&MechRig>), With<Piloted>>,
    data: Res<Data>,
    mut last_t: Local<Option<f32>>,
    mut sounds: MessageWriter<Sound>,
) {
    let Some((e, sim, rig)) = mech.iter().next() else { return };
    let tl = &data.0.boarding;
    let t = sim.hatch.t_s;
    let Some(prev) = last_t.replace(t) else { return };
    if (t - prev).abs() < 1e-6 {
        return;
    }
    let at = rig.and_then(|r| r.socket("HATCH_entry")).unwrap_or(e);
    let opening = t > prev;
    for st in &tl.stages {
        let edge = if opening { st.start_s } else { st.start_s + st.duration_s };
        if (prev < edge && edge <= t) || (t <= edge && edge < prev) {
            sounds.write(Sound::on(if st.vents.is_empty() { "hatch_motor" } else { "hatch_steam" }, at));
        }
    }
    // closing onto its seal, or the first movement off it
    if (prev > 0.0 && t <= 0.0) || (prev <= 0.0 && t > 0.0) {
        sounds.write(Sound::on("hatch_seal", at));
    }
}

/// The lock tone while a lock builds, the lock-on tone when it holds.
fn cockpit_sounds(
    target: Res<TargetState>,
    data: Res<Data>,
    ear: Res<Ear>,
    mut was: Local<(bool, bool)>,
    mut sounds: MessageWriter<Sound>,
) {
    let building = target.lock.target.is_some();
    let locked = target.locked(&data.0.targeting).is_some();
    if ear.seated {
        if building && !locked && !was.0 {
            sounds.write(Sound::speaker("lock_tone"));
        }
        if locked && !was.1 {
            sounds.write(Sound::speaker("lock_on"));
        }
    }
    *was = (building, locked);
}

/// A defence wave starting comes in on the radio.
fn mission_sounds(run: Option<Res<crate::defence::DefenceRun>>, mut was_wave: Local<bool>, mut sounds: MessageWriter<Sound>) {
    let Some(run) = run else { return };
    let wave = matches!(run.state.phase, cb_core::mission::Phase::Wave { .. });
    if wave && !*was_wave {
        sounds.write(Sound::speaker("radio_squelch"));
    }
    *was_wave = wave;
}

/// The mech waking as the pilot sits (read in the fixed step: `PilotEvents` is cleared in it).
fn pilot_sounds(events: Res<crate::pilot::PilotEvents>, mut sounds: MessageWriter<Sound>) {
    for ev in &events.0 {
        if matches!(ev, PilotEvent::Seated) {
            sounds.write(Sound::speaker("systems_boot"));
        }
    }
}

/// A cockpit control thrown (skates, mode, throttle to zero, next target): its switch clicks.
fn control_clicks(actions: Res<crate::input::StepActions>, pilot: Res<PilotSim>, mut sounds: MessageWriter<Sound>) {
    let a = &actions.0;
    if pilot.curr.mode == PilotMode::Seated && (a.skate_toggle || a.mode_request.is_some() || a.throttle_zero || a.target_next) {
        sounds.write(Sound::speaker("mfd_click"));
    }
}

// ---------------------------------------------------------------- voices

/// Each asked-for sound waits for its sound to arrive (distance at the speed of sound).
fn ask(mut asked: MessageReader<Sound>, ear: Res<Ear>, data: Res<Data>, time: Res<Time>, mut pending: ResMut<Pending>) {
    let cfg = &data.0.audio.cues;
    let now = time.elapsed_secs();
    for s in asked.read() {
        let delay_s = match s.at {
            SoundAt::Point(p) => p.distance(ear.at) / cfg.speed_of_sound_m_s,
            _ => 0.0,
        };
        pending.0.push(Due {
            at_s: now + delay_s,
            cue: s.cue,
            at: s.at,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn start_due(
    mut commands: Commands,
    mut pending: ResMut<Pending>,
    time: Res<Time>,
    ear: Res<Ear>,
    data: Res<Data>,
    lib: Res<Library>,
    mut dice: ResMut<Dice>,
    voices: Query<(Entity, &Voice)>,
    roots: Query<(Entity, &GlobalTransform), With<MechRoot>>,
    nodes: Query<&GlobalTransform>,
    settings: Res<crate::settings::Settings>,
) {
    let cfg = &data.0.audio.cues;
    let sfx = settings.0.sfx_volume;
    let now = time.elapsed_secs();
    let (due, waiting): (Vec<Due>, Vec<Due>) = std::mem::take(&mut pending.0).into_iter().partition(|d| d.at_s <= now);
    pending.0 = waiting;
    let mut count = voices.iter().count();
    for d in due {
        let Some(cue) = cfg.cues.get(d.cue) else {
            warn!("sound `{}` is not in data/audio/cues.ron", d.cue);
            continue;
        };
        if voices.iter().filter(|(_, v)| v.cue == d.cue).count() >= cue.max_voices as usize || count >= cfg.max_voices as usize {
            continue;
        }
        if spawn_voice(
            &mut commands,
            cfg,
            d.cue,
            cue,
            d.at,
            None,
            sfx,
            &ear,
            &lib,
            &mut dice,
            &roots,
            &nodes,
        )
        .is_some()
        {
            count += 1;
        }
    }
}

/// Spawns a voice of `cue` at `at`; a loop if `looping` is given. Returns the voice, or `None`
/// when it cannot be heard (too far, a speaker with the pilot not seated, a missing file).
#[allow(clippy::too_many_arguments)]
fn spawn_voice(
    commands: &mut Commands,
    cfg: &AudioConfig,
    name: &'static str,
    cue: &rules::Cue,
    at: SoundAt,
    looping: Option<LoopLevel>,
    // the player's sound effects volume (Options), for a one-shot's first frame; `level_voices` keeps it after
    sfx: f32,
    ear: &Ear,
    lib: &Library,
    dice: &mut Dice,
    roots: &Query<(Entity, &GlobalTransform), With<MechRoot>>,
    nodes: &Query<&GlobalTransform>,
) -> Option<Entity> {
    let file = &cue.files[(dice.unit() * cue.files.len() as f32) as usize % cue.files.len()];
    let pitch = cue.pitch.0 + (cue.pitch.1 - cue.pitch.0) * dice.unit();
    let level = db_to_gain(cue.volume_db);
    // where it is, what rides what, and how it is heard
    let (parent, local, world, inside) = match (&cue.space, at) {
        (Space::Cockpit(speaker), _) => {
            if !ear.seated {
                return None;
            }
            let o = cfg.speakers.get(speaker).copied().unwrap_or_default();
            let offset = Vec3::from(o);
            match ear.seat {
                Some(seat) => (
                    Some(seat),
                    offset,
                    nodes.get(seat).map(|g| g.transform_point(offset)).unwrap_or(ear.at),
                    true,
                ),
                None => (None, ear.at + offset, ear.at + offset, true),
            }
        }
        (_, SoundAt::Node(n)) => (Some(n), Vec3::ZERO, nodes.get(n).map(|g| g.translation()).unwrap_or(ear.at), false),
        (_, SoundAt::Point(p)) => {
            let rider = roots
                .iter()
                .map(|(e, g)| (e, g, g.translation().distance(p)))
                .filter(|(_, _, d)| *d < RIDE_WITHIN_M)
                .min_by(|a, b| a.2.total_cmp(&b.2));
            match rider {
                Some((e, g, _)) => (Some(e), g.affine().inverse().transform_point3(p), p, false),
                None => (None, p, p, false),
            }
        }
        (_, SoundAt::Speaker) => (None, ear.at, ear.at, false),
    };
    let (gain, variant) = if inside {
        (1.0, Variant::Near)
    } else {
        let h = rules::heard(cfg, world.distance(ear.at), ear.sealed)?;
        (h.gain, h.variant)
    };
    let handle = lib.0.get(&variant.path(file)).or_else(|| lib.0.get(file))?.clone();
    // a loop starts silent and fades in (`drive_loops`); a one-shot starts at its level
    let (mode, volume) = match looping {
        Some(_) => (PlaybackMode::Loop, 0.0),
        None => (PlaybackMode::Despawn, level * gain * sfx),
    };
    let settings = PlaybackSettings {
        mode,
        volume: Volume::Linear(volume),
        speed: looping.map_or(pitch, |l| l.pitch),
        spatial: true,
        spatial_scale: Some(SpatialScale::new(SPATIAL_SCALE)),
        ..PlaybackSettings::ONCE
    };
    let mut ent = commands.spawn((
        Name::new(format!("sound {name}")),
        AudioPlayer::new(handle),
        settings,
        Transform::from_translation(local),
        Voice { cue: name, level, inside },
    ));
    if let Some(p) = parent {
        ent.insert(ChildOf(p));
    }
    Some(ent.id())
}

/// Starts, sets and fades the loops asked for this frame.
#[allow(clippy::too_many_arguments)]
fn drive_loops(
    mut commands: Commands,
    mut asked: MessageReader<Loop>,
    mut loops: ResMut<Loops>,
    mut voices: Query<(&mut LoopVoice, Option<&mut SpatialAudioSink>)>,
    time: Res<Time>,
    ear: Res<Ear>,
    data: Res<Data>,
    lib: Res<Library>,
    mut dice: ResMut<Dice>,
    roots: Query<(Entity, &GlobalTransform), With<MechRoot>>,
    nodes: Query<&GlobalTransform>,
) {
    let cfg = &data.0.audio.cues;
    for (mut v, _) in &mut voices {
        v.target = None;
    }
    for l in asked.read() {
        let key = (
            l.cue,
            match l.at {
                SoundAt::Node(n) => LoopAt::Node(n),
                _ => LoopAt::Speaker,
            },
        );
        if let Some(&e) = loops.0.get(&key)
            && let Ok((mut v, _)) = voices.get_mut(e)
        {
            v.target = Some(l.level);
            continue;
        }
        let Some(cue) = cfg.cues.get(l.cue) else { continue };
        if let Some(e) = spawn_voice(
            &mut commands,
            cfg,
            l.cue,
            cue,
            l.at,
            Some(l.level),
            0.0,
            &ear,
            &lib,
            &mut dice,
            &roots,
            &nodes,
        ) {
            commands.entity(e).insert(LoopVoice {
                db: LOOP_SILENT_DB,
                target: Some(l.level),
            });
            loops.0.insert(key, e);
        }
    }
    let dt = time.delta_secs();
    let mut stopped = Vec::new();
    for (key, &e) in &loops.0 {
        let Ok((mut v, sink)) = voices.get_mut(e) else {
            // spawned this frame: its components arrive with the commands
            continue;
        };
        let target_db = v.target.map_or(LOOP_SILENT_DB - 10.0, |l| l.db);
        v.db = rules::ease(v.db, target_db, dt, LOOP_EASE_S);
        if let Some(sink) = sink
            && let Some(l) = v.target
        {
            sink.set_speed(l.pitch);
        }
        if v.target.is_none() && v.db <= LOOP_SILENT_DB {
            commands.entity(e).despawn();
            stopped.push(*key);
        }
    }
    for k in stopped {
        loops.0.remove(&k);
    }
}

/// Every placed voice's level this frame: its cue's level, its distance and the seal (and a loop's
/// own eased level).
fn level_voices(
    mut voices: Query<(&Voice, Option<&LoopVoice>, &GlobalTransform, &mut SpatialAudioSink)>,
    ear: Res<Ear>,
    data: Res<Data>,
    settings: Res<crate::settings::Settings>,
) {
    let cfg = &data.0.audio.cues;
    let player = settings.0.sfx_volume;
    for (v, l, at, mut sink) in &mut voices {
        let g = if v.inside {
            1.0
        } else {
            let d = at.translation().distance(ear.at);
            let seal = if ear.sealed { db_to_gain(cfg.seal_db) } else { 1.0 };
            rules::distance_gain(cfg.full_level_within_m, d) * seal
        };
        let loop_gain = l.map_or(1.0, |l| db_to_gain(l.db));
        sink.set_volume(Volume::Linear(v.level * g * loop_gain * player));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cue_the_game_asks_for_is_in_the_file() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data = cb_core::GameData::load(&root, "mechs/placeholder.ron").expect("data loads");
        for cue in ASKED {
            assert!(
                data.audio.cues.cues.contains_key(*cue),
                "cue `{cue}` is asked for but not in cues.ron"
            );
        }
    }

    #[test]
    fn the_dice_stay_in_the_unit_interval() {
        let mut d = Dice(12345);
        for _ in 0..1000 {
            let x = d.unit();
            assert!((0.0..1.0).contains(&x));
        }
    }
}
