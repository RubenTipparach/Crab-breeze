//! Crab Breeze simulation core: everything the game decides, with no engine in it.
//!
//! This crate holds the rules (world generation, mech locomotion, the pilot) as pure functions of
//! state, input, data and a fixed time step in seconds. It has no rendering, windowing, audio or
//! asset dependency, so it runs headless and is tested without a GPU (CLAUDE.md section 6.2).
//! `cb-app` adapts it to Bevy.

pub mod ai;
pub mod audio;
pub mod battle_fx;
pub mod boarding;
pub mod career;
pub mod cockpit;
pub mod data;
pub mod fx;
pub mod grass;
pub mod ground_marks;
pub mod input;
pub mod jobs;
pub mod mech;
pub mod mission;
pub mod pilot;
pub mod rng;
pub mod settings;
pub mod sim;
pub mod skins;
pub mod sky_light;
pub mod sun_flare;
pub mod targeting;
pub mod units;
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
    /// `data/world/grass.ron`.
    pub grass: grass::GrassConfig,
    /// `data/fx/ground_marks.ron`.
    pub ground_marks: ground_marks::GroundMarksConfig,
    /// `data/fx/battle_fx.ron`.
    pub battle_fx: battle_fx::BattleFxConfig,
    /// `data/world/sun_flare.ron`.
    pub sun: sun_flare::SunLook,
    /// `data/world/units.ron`.
    pub units: units::UnitsConfig,
    /// `data/enemies/skins.ron`.
    pub skins: skins::Skins,
    /// `data/audio/cues.ron`, `music.ron` and `ambience.ron`.
    pub audio: audio::AudioData,
}

impl GameData {
    /// Loads every data file under `root` (the `data/` directory), with the mech file `mech`
    /// (relative to `root`) and its boarding timeline beside it (`mechs/<name>/boarding.ron`). The first failure is returned, naming its file and field.
    pub fn load(root: &Path, mech: &str) -> Result<Self, data::DataError> {
        let world: world::WorldConfig = data::load_ron(&root.join("world/world.ron"))?;
        let skins_path = root.join("enemies/skins.ron");
        let skins: skins::Skins = data::load_ron(&skins_path)?;
        let encounters: ai::Encounters = data::load_ron(&root.join("world/encounters.ron"))?;
        let defence: mission::DefenceConfig = data::load_ron(&root.join("missions/defence.ron"))?;
        // Every skin an enemy names is in the list (openspec change `blockout-enemies`).
        let named = encounters
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.skin.as_deref().map(|s| (s, format!("world/encounters.ron enemies[{i}].skin"))))
            .chain(
                defence
                    .raiders
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| r.skin.as_deref().map(|s| (s, format!("missions/defence.ron raiders[{i}].skin")))),
            )
            .chain(defence.skins_by_wave.iter().enumerate().flat_map(|(w, list)| {
                list.iter()
                    .map(move |s| (s.as_str(), format!("missions/defence.ron skins_by_wave[{w}]")))
            }));
        skins.check_names(named).map_err(|reason| data::DataError {
            path: skins_path.clone(),
            reason,
        })?;
        let grass_path = root.join("world/grass.ron");
        let grass: grass::GrassConfig = data::load_ron(&grass_path)?;
        grass
            .validate_with(&world)
            .map_err(|reason| data::DataError { path: grass_path, reason })?;
        Ok(Self {
            sim: data::load_ron(&root.join("sim.ron"))?,
            world,
            mech: data::load_ron(&root.join(mech))?,
            boarding: data::load_ron(&root.join(mech.trim_end_matches(".ron")).join("boarding.ron"))?,
            pilot: data::load_ron(&root.join("pilot/pilot.ron"))?,
            bindings: data::load_ron(&root.join("input/bindings.ron"))?,
            encounters,
            water_look: data::load_ron(&root.join("world/water_look.ron"))?,
            defence,
            fx: data::load_ron(&root.join("fx/fx.ron"))?,
            targeting: data::load_ron(&root.join("hud/targeting.ron"))?,
            cockpit_light: data::load_ron(&root.join("hud/cockpit_light.ron"))?,
            sky_light: data::load_ron(&root.join("world/sky_light.ron"))?,
            grass,
            ground_marks: data::load_ron(&root.join("fx/ground_marks.ron"))?,
            battle_fx: data::load_ron(&root.join("fx/battle_fx.ron"))?,
            sun: data::load_ron(&root.join("world/sun_flare.ron"))?,
            units: data::load_ron(&root.join("world/units.ron"))?,
            skins,
            audio: audio::AudioData {
                cues: data::load_ron(&root.join("audio/cues.ron"))?,
                music: data::load_ron(&root.join("audio/music.ron"))?,
                ambience: data::load_ron(&root.join("audio/ambience.ron"))?,
            },
        })
    }

    /// The roster (openspec change `mech-select`): every mech under `root/mechs` with a cockpit interior, in file-name
    /// order, as the menu and free roam's pause menu list them. A mech file without an interior is the stand-in or a
    /// retired mech. A file that does not load is left out (startup and `cb-tools validate` report it).
    pub fn roster(root: &Path) -> Vec<RosterMech> {
        Self::mech_files(root)
            .into_iter()
            .filter_map(|file| {
                let cfg: mech::MechConfig = data::load_ron(&root.join(&file)).ok()?;
                cfg.interior.as_ref()?;
                Some(RosterMech { file, name: cfg.name })
            })
            .collect()
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

/// A mech the player can pilot (`GameData::roster`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RosterMech {
    /// Its data file, relative to the data root (`mechs/foreman.ron`).
    pub file: String,
    /// Its display name.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_roster_is_the_eight_mechs_with_cockpits() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let files: Vec<String> = GameData::roster(&root).into_iter().map(|m| m.file).collect();
        for id in [
            "driller",
            "foreman",
            "harrow",
            "ironhorse",
            "mule",
            "picket",
            "roustabout",
            "sapper",
        ] {
            assert!(files.contains(&format!("mechs/{id}.ron")), "{id} is on the roster: {files:?}");
        }
        assert!(
            !files.iter().any(|f| f.contains("placeholder") || f.contains("breeze")),
            "{files:?}"
        );
        for f in &files {
            GameData::load(&root, f).unwrap_or_else(|e| panic!("{f}: {e}"));
        }
    }

    #[test]
    fn every_shipped_data_file_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        for mech in GameData::mech_files(&root) {
            GameData::load(&root, &mech).unwrap_or_else(|e| panic!("{e}"));
        }
    }

    /// Every roster mech sits in a cockpit of its own (openspec change `cockpit-identity`).
    #[test]
    fn no_two_mechs_share_a_cockpit_interior() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut seen: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        for mech in GameData::mech_files(&root) {
            let cfg: mech::MechConfig = data::load_ron(&root.join(&mech)).unwrap_or_else(|e| panic!("{e}"));
            let Some(interior) = cfg.interior else { continue };
            if let Some(other) = seen.insert(interior.model.clone(), mech.clone()) {
                panic!("{mech} and {other} both sit in {}", interior.model);
            }
        }
    }

    #[test]
    fn every_defence_wave_names_units_that_exist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let d = GameData::load(&root, "mechs/placeholder.ron").unwrap();
        for (w, wave) in d.defence.forces.iter().enumerate() {
            for f in wave {
                assert!(d.units.units.contains_key(&f.unit), "wave {w}: no unit `{}`", f.unit);
                if f.by_air {
                    assert!(d.units.units.contains_key("cargo_plane"), "wave {w}: drops need `cargo_plane`");
                }
            }
        }
    }
}
