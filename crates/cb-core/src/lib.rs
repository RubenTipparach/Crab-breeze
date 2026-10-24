//! Crab Breeze simulation core: everything the game decides, with no engine in it.
//!
//! This crate holds the rules (world generation, mech locomotion, the pilot) as pure functions of
//! state, input, data and a fixed time step in seconds. It has no rendering, windowing, audio or
//! asset dependency, so it runs headless and is tested without a GPU (CLAUDE.md section 6.2).
//! `cb-app` adapts it to Bevy.

pub mod ai;
pub mod boarding;
pub mod cockpit;
pub mod data;
pub mod fx;
pub mod input;
pub mod mech;
pub mod mission;
pub mod pilot;
pub mod rng;
pub mod sim;
pub mod sky_light;
pub mod targeting;
pub mod water_look;
pub mod weapons;
pub mod world;

use std::path::Path;

/// Every data file the game loads, in one place.
#[derive(Clone, Debug)]
pub struct GameData {
    /// `data/sim.ron`.
    pub sim: sim::SimConfig,
    /// `data/world/world.ron`.
    pub world: world::WorldConfig,
    /// The mech, `data/mechs/<name>.ron`.
    pub mech: mech::MechConfig,
    /// The mech's boarding timeline, `data/mechs/<name>/boarding.ron`.
    pub boarding: boarding::BoardingTimeline,
    /// `data/pilot/pilot.ron`.
    pub pilot: pilot::PilotConfig,
    /// `data/input/bindings.ron`.
    pub bindings: input::Bindings,
    /// `data/world/encounters.ron`.
    pub encounters: ai::Encounters,
    /// `data/world/water_look.ron`.
    pub water_look: water_look::WaterLook,
    /// `data/missions/defence.ron`.
    pub defence: mission::DefenceConfig,
    /// `data/fx/fx.ron`.
    pub fx: fx::FxConfig,
    /// `data/hud/targeting.ron`.
    pub targeting: targeting::TargetingConfig,
    /// `data/hud/cockpit_light.ron`.
    pub cockpit_light: cockpit::CockpitLightConfig,
    /// `data/world/sky_light.ron`.
    pub sky_light: sky_light::SkyLightConfig,
}

impl GameData {
    /// Loads every data file under `root` (the `data/` directory), with the mech file `mech`
    /// (relative to `root`) and its boarding timeline beside it (`mechs/<name>/boarding.ron`). The first failure is returned, naming its file and field.
    pub fn load(root: &Path, mech: &str) -> Result<Self, data::DataError> {
        Ok(Self {
            sim: data::load_ron(&root.join("sim.ron"))?,
            world: data::load_ron(&root.join("world/world.ron"))?,
            mech: data::load_ron(&root.join(mech))?,
            boarding: data::load_ron(&root.join(mech.trim_end_matches(".ron")).join("boarding.ron"))?,
            pilot: data::load_ron(&root.join("pilot/pilot.ron"))?,
            bindings: data::load_ron(&root.join("input/bindings.ron"))?,
            encounters: data::load_ron(&root.join("world/encounters.ron"))?,
            water_look: data::load_ron(&root.join("world/water_look.ron"))?,
            defence: data::load_ron(&root.join("missions/defence.ron"))?,
            fx: data::load_ron(&root.join("fx/fx.ron"))?,
            targeting: data::load_ron(&root.join("hud/targeting.ron"))?,
            cockpit_light: data::load_ron(&root.join("hud/cockpit_light.ron"))?,
            sky_light: data::load_ron(&root.join("world/sky_light.ron"))?,
        })
    }

    /// The mech files under `root/mechs`, sorted by name.
    pub fn mech_files(root: &Path) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(root.join("mechs"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".ron"))
            .map(|n| format!("mechs/{n}"))
            .collect();
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_data_file_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        for mech in GameData::mech_files(&root) {
            GameData::load(&root, &mech).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}
