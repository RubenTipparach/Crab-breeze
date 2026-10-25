//! Ambience (openspec change `ambience`): stereo beds chosen by where the listener is (city,
//! industrial district, forest, open ground), crossfaded as they move, the wind rising with height,
//! all muffled through the cockpit's seal. The weights are `cb_core::audio::ambience_weights`; this
//! file reads its inputs from the world and plays the beds. Beds are not placed in 3D (owner,
//! 2026-10-25: "verify all sounds are 3d based (except ambience and music and ui)").

use crate::audio::{Ear, HearSet};
use crate::sim::Data;
use crate::world::{Live, Plan};
use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackSettings, Volume};
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
        .add_systems(Update, (read_zone, play_beds).chain().after(HearSet));
    }
}

/// Where the listener is, a few times a second.
fn read_zone(mut beds: ResMut<Beds>, ear: Res<Ear>, plan: Res<Plan>, live: Res<Live>, data: Res<Data>, time: Res<Time>) {
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
        night: false,
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
    time: Res<Time>,
    ready: Res<crate::loading::GameReady>,
) {
    if !ready.0 {
        return;
    }
    let cfg = &data.0.audio.ambience;
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
            // start somewhere in the loop, so two visits do not open on the same bird
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let start = Duration::from_secs_f32((seed >> 40) as f32 / (1u64 << 24) as f32 * 30.0);
            let voice = commands
                .spawn((
                    Name::new(format!("ambience {name}")),
                    AudioPlayer::new(assets.load(bed.file.clone())),
                    PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)).with_start_position(start),
                ))
                .id();
            beds.playing.insert(name.clone(), Bed { voice, db: silent });
        }
        let Some(b) = beds.playing.get_mut(name) else { continue };
        b.db = rules::ease(b.db, target.unwrap_or(silent), dt, cfg.ease_s);
        if let Ok(mut s) = sinks.get_mut(b.voice) {
            s.set_volume(Volume::Linear(db_to_gain(b.db)));
        }
        if target.is_none() && b.db < cfg.stop_below_db {
            commands.entity(b.voice).despawn();
            beds.playing.remove(name);
        }
    }
    beds.seed = seed;
}
