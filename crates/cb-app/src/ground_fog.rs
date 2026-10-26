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

/// Keeps `FogNow`.
pub struct GroundFogPlugin;

impl Plugin for GroundFogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FogNow>()
            .add_world_spawn(restart_clock)
            .add_game_systems(FixedUpdate, tick.in_set(SimSet::Input));
    }
}

/// A new world begins at the clock's start, in the strength it begins with.
fn restart_clock(data: Res<Data>, mut fog: ResMut<FogNow>) {
    let cfg = &data.0.world.fog;
    let hour_h = cfg.clock.hour_at(0.0);
    *fog = FogNow {
        elapsed_s: 0.0,
        hour_h,
        mission: None,
        strength: cfg.strength(None, hour_h),
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
