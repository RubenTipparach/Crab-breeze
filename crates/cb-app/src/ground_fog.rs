//! How thick the ground fog is now (openspec change `ground-fog`, survey F1; owner, 2026-10-26:
//! "Approve all"): the fog's own clock runs in the fixed step from the moment the world begins, and
//! each step the strength is the running mission's preset or the morning mist thinning by the
//! clock (`cb_core::world::mist::FogConfig::strength`). The fog the compose pass draws and the fog
//! the targeting sees through are both this one number. It lives in the fixed step because the
//! targeting, which is simulation, reads it.

use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use bevy::prelude::*;
use cb_core::missions::MissionKind;
use cb_core::world::mist::FogStrength;

/// The fog's clock and its strength this step.
#[derive(Resource, Clone, Copy, Debug)]
pub struct FogNow {
    /// Seconds since the world began.
    pub elapsed_s: f32,
    /// The fog's hour.
    pub hour_h: f32,
    /// The mission whose preset sets the fog, if one runs.
    pub mission: Option<MissionKind>,
    /// How thick it is.
    pub strength: FogStrength,
}

impl Default for FogNow {
    fn default() -> Self {
        Self {
            elapsed_s: 0.0,
            hour_h: 0.0,
            mission: None,
            strength: FogStrength::PLAIN,
        }
    }
}

/// The weather at the pilot's eye as the view's fog sees it: the cloud's density, the rain and the
/// clouds' shade on the sun, each followed over `FogConfig::eye_weather_s`. Every pass that fogs the
/// whole view by the weather at the eye reads these, never a raw sample: a raw sample swings as a
/// fast mech crosses the edge of a puff, a shaft or a cloud's shadow, and the whole sky and the
/// ground's fog pulsed with it (owner, 2026-10-26: "the whole sky ios flikering", "also ground
/// color fog flickerewd as well"). Presentation only: the targeting, which is simulation, samples
/// the rain itself.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EyeWeather {
    /// Cloud density at the eye, 0..1.
    pub cloud: f32,
    /// Rain at the eye, 0..1.
    pub rain: f32,
    /// The sun's share through the clouds over the eye, 0..1.
    pub sun: f32,
    /// False until the first sample: the first frame takes the weather as it is.
    pub primed: bool,
}

impl Default for EyeWeather {
    fn default() -> Self {
        Self {
            cloud: 0.0,
            rain: 0.0,
            sun: 1.0,
            primed: false,
        }
    }
}

/// Keeps `FogNow` and `EyeWeather`.
pub struct GroundFogPlugin;

impl Plugin for GroundFogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FogNow>()
            .init_resource::<EyeWeather>()
            .add_world_spawn(restart_clock)
            .add_game_systems(FixedUpdate, tick.in_set(SimSet::Input));
    }
}

/// A new world begins at the clock's start, in the strength it begins with. `CB_FOG_HOUR=<h>` starts the clock at that
/// hour instead (an instrument, CLAUDE.md section 4: stills of the morning mist at 06:00 against 09:00 and 12:00, for
/// survey question F4); the clock then runs on from there.
fn restart_clock(data: Res<Data>, mut fog: ResMut<FogNow>) {
    let cfg = &data.0.world.fog;
    let elapsed_s = std::env::var("CB_FOG_HOUR")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|h| h.is_finite() && cfg.clock.hours_per_s > 0.0)
        .map(|h| ((h - cfg.clock.start_h) / cfg.clock.hours_per_s).max(0.0))
        .unwrap_or(0.0);
    let hour_h = cfg.clock.hour_at(elapsed_s);
    println!("FOG clock starts at {hour_h:.1} h");
    *fog = FogNow {
        elapsed_s,
        hour_h,
        mission: None,
        strength: cfg.strength(None, hour_h),
    };
}

/// Follows the weather at the pilot's eye (`EyeWeather`), once the camera is placed for the frame
/// (`pilot`'s camera chain runs it, before the fog reads it).
pub(crate) fn follow_eye_weather(
    data: Res<Data>,
    plan: Res<crate::world::Plan>,
    time: Res<Time>,
    eye: Option<Single<&Transform, With<crate::pilot::PilotCamera>>>,
    mut weather: ResMut<EyeWeather>,
) {
    let Some(eye) = eye else { return };
    let at = eye.translation;
    let t = time.elapsed_secs();
    let clouds = &plan.clouds;
    let (cloud, rain, sun) = (
        clouds.density_at(at, t),
        clouds.rain_at(at, t),
        clouds.sun_at(Vec2::new(at.x, at.z), t),
    );
    let tau = if weather.primed { data.0.world.fog.eye_weather_s } else { 0.0 };
    let dt = time.delta_secs();
    let follow = |now: f32, to: f32| cb_core::sun_flare::fade_toward(now, to, dt, tau);
    *weather = EyeWeather {
        cloud: follow(weather.cloud, cloud),
        rain: follow(weather.rain, rain),
        sun: follow(weather.sun, sun),
        primed: true,
    };
}

/// Runs the clock and sets the strength from the mission running.
fn tick(
    data: Res<Data>,
    time: Res<Time<Fixed>>,
    run: Option<Res<crate::missions::FreeRoamRun>>,
    defending: Option<Res<crate::defence::Defending>>,
    mut fog: ResMut<FogNow>,
) {
    let cfg = &data.0.world.fog;
    let mission = run.and_then(|r| r.free.active).or(defending.map(|_| MissionKind::Defence));
    fog.elapsed_s += time.delta_secs();
    fog.hour_h = cfg.clock.hour_at(fog.elapsed_s);
    if mission != fog.mission {
        let preset = mission.and_then(|m| cfg.missions.get(&m));
        println!("FOG mission {mission:?}: preset {preset:?} (else the clock's, {:.1} h)", fog.hour_h);
        fog.mission = mission;
    }
    fog.strength = cfg.strength(mission, fog.hour_h);
}

#[cfg(test)]
mod tests {
    /// Survey F3 (owner, 2026-10-26: "Approve all"): no ray-marched volumetric fog over the island.
    /// Bevy's ray march (its volumetric fog on a camera, a fog volume in the world) may come later
    /// only for a small local volume (steam at boarding, waterfall spray), and a line that adds one
    /// must say so with the words `local fog volume (F3)`. Any other use in the game's source fails
    /// here.
    #[test]
    fn no_ray_marched_fog_lies_over_the_island() {
        let banned = [concat!("Volumetric", "Fog"), concat!("Fog", "Volume")];
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("the source folder reads") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).expect("a source file reads");
                    for (n, line) in text.lines().enumerate() {
                        if banned.iter().any(|b| line.contains(b)) && !line.contains("local fog volume (F3)") {
                            found.push(format!("{}:{}", path.display(), n + 1));
                        }
                    }
                }
            }
        }
        assert!(found.is_empty(), "ray-marched fog in the game (survey F3): {found:?}");
    }
}
