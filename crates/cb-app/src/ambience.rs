//! Ambience (openspec change `ambience`): stereo beds chosen by where the listener is (city,
//! industrial district, forest, open ground), crossfaded as they move, the wind rising with height,
//! all muffled through the cockpit's seal. The weights are `cb_core::audio::ambience_weights`; this
//! file reads its inputs from the world and plays the beds. Beds are not placed in 3D (owner,
//! 2026-10-25: "verify all sounds are 3d based (except ambience and music and ui)").

use crate::audio::{Ear, HearSet};
use crate::sim::Data;
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::audio::{AddAudioSource, AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, Decodable, PlaybackSettings, Source, Volume};
use bevy::prelude::*;
use cb_core::audio::{self as rules, ZoneInputs, ZoneWeights, db_to_gain};
use std::collections::HashMap;
use std::time::Duration;

/// Buildings within this are counted for the industrial share, m.
const DISTRICT_M: f32 = 200.0;
/// Trees within this are counted for the forest, m.
const TREES_M: f32 = 80.0;
/// A low, broad building is a warehouse: the city generator's own rule for its warehouse style
/// (`dressing.rs`: at most this many storeys over this footprint).
const WAREHOUSE_MAX_STOREYS: u32 = 4;
/// See `WAREHOUSE_MAX_STOREYS`, m2.
const WAREHOUSE_MIN_AREA_M2: f32 = 900.0;
/// The wind bed's name.
const WIND: &str = "amb_wind_high";

/// One bed playing.
struct Bed {
    voice: Entity,
    db: f32,
}

#[derive(Resource, Default)]
struct Beds {
    playing: HashMap<String, Bed>,
    /// Every bed's recording, kept loaded once asked for.
    files: HashMap<String, Handle<AudioSource>>,
    weights: ZoneWeights,
    read_in_s: f32,
    seed: u64,
}

/// The ambience.
pub struct AmbiencePlugin;

impl Plugin for AmbiencePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Beds {
            seed: 0x2545_F491_4F6C_DD1D,
            ..Default::default()
        })
        .add_audio_source::<BedLoop>()
        .add_game_systems(Update, (read_zone, play_beds).chain().after(HearSet));
    }
}

/// A bed's recording looped from its start, entered `start` into the loop. Bevy's own start position
/// (`PlaybackSettings::start_position`) decodes and throws away everything before it on the main thread as the
/// sound starts, and the Vorbis decoder cannot seek: 20 to 43 ms for a start up to 30 s into the loop, the largest
/// main-thread stalls on the streaming route (2026-10-26). Here the skip happens on the audio thread instead, a
/// few samples at a time while the bed is still silent (`LateSkip`).
#[derive(Asset, TypePath, Clone)]
pub struct BedLoop {
    source: AudioSource,
    start: Duration,
}

/// Input samples skipped for each (silent) output sample while catching up to a bed's start: 30 s into a loop
/// is reached in under 2 s, while the bed fades in from silence.
const SKIP_PER_SAMPLE: u64 = 16;

impl Decodable for BedLoop {
    type Decoder = LateSkip;

    fn decoder(&self) -> LateSkip {
        let inner = self.source.decoder();
        let per_s = u64::from(inner.sample_rate().get()) * u64::from(inner.channels().get());
        LateSkip {
            left: (self.start.as_secs_f64() * per_s as f64) as u64 / u64::from(inner.channels().get()) * u64::from(inner.channels().get()),
            inner,
            source: self.source.clone(),
        }
    }
}

/// A bed's samples: silence while it skips to its start, then the recording, over and over from its beginning.
pub struct LateSkip {
    inner: <AudioSource as Decodable>::Decoder,
    source: AudioSource,
    /// Input samples still to skip (whole frames).
    left: u64,
}

impl Iterator for LateSkip {
    type Item = bevy::audio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left > 0 {
            let n = self.left.min(SKIP_PER_SAMPLE);
            for _ in 0..n {
                if self.inner.next().is_none() {
                    self.left = 0;
                    break;
                }
            }
            self.left = self.left.saturating_sub(n);
            return Some(0.0);
        }
        match self.inner.next() {
            Some(s) => Some(s),
            // The end of the recording: round again from its beginning.
            None => {
                self.inner = self.source.decoder();
                self.inner.next()
            }
        }
    }
}

impl Source for LateSkip {
    fn current_span_len(&self) -> Option<usize> {
        // One recording: its rate and channels hold to the end.
        None
    }
    fn channels(&self) -> bevy::audio::ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> bevy::audio::SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Where the listener is, a few times a second.
#[allow(clippy::too_many_arguments)]
fn read_zone(
    mut beds: ResMut<Beds>,
    ear: Res<Ear>,
    plan: Res<Plan>,
    live: Res<Live>,
    data: Res<Data>,
    time: Res<Time>,
    light: Option<Res<crate::day_night::Light>>,
) {
    beds.read_in_s -= time.delta_secs();
    if beds.read_in_s > 0.0 {
        return;
    }
    beds.read_in_s = 1.0 / data.0.audio.ambience.read_hz;
    let p = Vec2::new(ear.at.x, ear.at.z);
    let city = plan
        .cities
        .iter()
        .map(|c| (c.centre.distance(p), c.half_size_m))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or((f32::INFINITY, 0.0));
    let near = plan.collision.candidates(p, DISTRICT_M);
    let warehouses = near
        .iter()
        .filter_map(|&b| plan.buildings.get(b as usize))
        .filter(|b| {
            let s = b.footprint.max - b.footprint.min;
            b.storeys <= WAREHOUSE_MAX_STOREYS && s.x * s.y > WAREHOUSE_MIN_AREA_M2
        })
        .count();
    let z = ZoneInputs {
        city_distance_m: city.0,
        city_radius_m: city.1,
        warehouse_share: if near.is_empty() {
            0.0
        } else {
            warehouses as f32 / near.len() as f32
        },
        trees_near: live.forest.near(p, TREES_M).len() as u32,
        height_m: ear.at.y - plan.terrain.height_at(p.x, p.y),
        // Night from the hour (`day_night`): from dusk, once the sun is well down.
        night: light.is_some_and(|l| l.0.day < 0.5),
        rain: false,
    };
    beds.weights = rules::ambience_weights(&z);
}

/// Each bed toward its level: started when it is wanted, faded, stopped when it is not.
fn play_beds(
    mut commands: Commands,
    mut beds: ResMut<Beds>,
    mut sinks: Query<&mut AudioSink>,
    ear: Res<Ear>,
    data: Res<Data>,
    assets: Res<AssetServer>,
    (sources, mut loops): (Res<Assets<AudioSource>>, ResMut<Assets<BedLoop>>),
    time: Res<Time>,
    ready: Res<crate::loading::GameReady>,
    settings: Res<crate::settings::Settings>,
) {
    if !ready.0 {
        return;
    }
    let cfg = &data.0.audio.ambience;
    // ambience is a sound effect: the player's sound effects volume (Options) scales it
    let sfx = settings.0.sfx_volume;
    let seal = if ear.sealed { cfg.seal_db } else { 0.0 };
    let dt = time.delta_secs();
    let weights = beds.weights;
    let mut seed = beds.seed;
    for (name, bed) in &cfg.beds {
        let target = if name == WIND {
            Some(cfg.wind_db.0 + cfg.wind_db.1 * weights.height + bed.volume_db)
        } else {
            rules::bed_db(bed.volume_db, weights.of(name))
        }
        .map(|db| db + seal);
        let silent = cfg.stop_below_db - 10.0;
        if !beds.playing.contains_key(name) {
            let Some(_) = target else { continue };
            let file = beds.files.entry(name.clone()).or_insert_with(|| assets.load(bed.file.clone()));
            // Started once its recording is loaded (Bevy waited the same way).
            let Some(source) = sources.get(&*file) else { continue };
            // start somewhere in the loop, so two visits do not open on the same bird
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let start = Duration::from_secs_f32((seed >> 40) as f32 / (1u64 << 24) as f32 * 30.0);
            let voice = commands
                .spawn((
                    Name::new(format!("ambience {name}")),
                    AudioPlayer(loops.add(BedLoop {
                        source: source.clone(),
                        start,
                    })),
                    // `BedLoop` loops itself, from the recording's beginning.
                    PlaybackSettings::ONCE.with_volume(Volume::Linear(0.0)),
                ))
                .id();
            beds.playing.insert(name.clone(), Bed { voice, db: silent });
        }
        let Some(b) = beds.playing.get_mut(name) else { continue };
        b.db = rules::ease(b.db, target.unwrap_or(silent), dt, cfg.ease_s);
        if let Ok(mut s) = sinks.get_mut(b.voice) {
            s.set_volume(Volume::Linear(db_to_gain(b.db) * sfx));
        }
        if target.is_none() && b.db < cfg.stop_below_db {
            commands.entity(b.voice).despawn();
            beds.playing.remove(name);
        }
    }
    beds.seed = seed;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recording(name: &str) -> AudioSource {
        let p = crate::cli::repo_root().join("assets/audio").join(name);
        AudioSource {
            bytes: std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())).into(),
        }
    }

    #[test]
    fn a_bed_plays_from_its_start_after_a_silent_catch_up_as_bevys_start_position_did() {
        let source = recording("sfx/alarm_warning.ogg");
        let start = Duration::from_millis(500);
        let mut late = BedLoop {
            source: source.clone(),
            start,
        }
        .decoder();
        let skipped = late.left;
        // the lead-in is silence, a sixteenth of what it skips
        let lead = skipped.div_ceil(SKIP_PER_SAMPLE);
        for _ in 0..lead {
            assert_eq!(late.next(), Some(0.0));
        }
        let bevy: Vec<f32> = source.decoder().skip_duration(start).take(4096).collect();
        let after: Vec<f32> = late.by_ref().take(4096).collect();
        assert_eq!(after, bevy, "the same samples Bevy's start position gave");
    }

    #[test]
    fn a_bed_goes_round_again_from_the_recordings_beginning() {
        let source = recording("sfx/autocannon.ogg");
        let whole: Vec<f32> = source.decoder().collect();
        let mut bed = BedLoop {
            source: source.clone(),
            start: Duration::ZERO,
        }
        .decoder();
        let first: Vec<f32> = bed.by_ref().take(whole.len()).collect();
        assert_eq!(first, whole);
        let again: Vec<f32> = bed.take(1000).collect();
        assert_eq!(again, whole[..1000].to_vec(), "round again from the start");
    }
}
