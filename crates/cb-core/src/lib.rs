//! Crab Breeze simulation core: everything the game decides, with no engine in it.
//!
//! This crate holds the rules (world generation, mech locomotion, the pilot) as pure functions of
//! state, input, data and a fixed time step in seconds. It has no rendering, windowing, audio or
//! asset dependency, so it runs headless and is tested without a GPU (CLAUDE.md section 6.2).
//! `cb-app` adapts it to Bevy.

pub mod ai;
pub mod air_drop;
pub mod air_traffic;
pub mod audio;
pub mod bases;
pub mod battle_fx;
pub mod boarding;
pub mod car;
pub mod career;
pub mod civil_standing;
pub mod civilians;
pub mod cockpit;
pub mod data;
pub mod day_night;
pub mod fortress;
pub mod fx;
pub mod grass;
pub mod ground_marks;
pub mod input;
pub mod interact;
pub mod jobs;
pub mod mech;
pub mod mfd_focus;
pub mod mission;
pub mod missions;
pub mod people_vat;
pub mod perf;
pub mod pilot;
pub mod playtest;
pub mod refit;
pub mod rng;
pub mod save;
pub mod scope;
pub mod settings;
pub mod ships;
pub mod shop;
pub mod sim;
pub mod skins;
pub mod sky_light;
pub mod sun_flare;
pub mod targeting;
pub mod traffic;
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
    /// The mech, `data/mechs/<name>.ron`, with its loadout `data/mechs/<name>/weapons.ron` when it has one.
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
    /// `data/world/day_night.ron`: the world's clock and the light of each hour.
    pub day_night: day_night::DayNightConfig,
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
    /// `data/world/air_traffic.ron`: civil aircraft (`civil-air-traffic`).
    pub air_traffic: air_traffic::AirTrafficConfig,
    /// `data/enemies/skins.ron`.
    pub skins: skins::Skins,
    /// `data/missions/fortress.ron` and `fortress_shape.ron`.
    pub fortress: fortress::FortressData,
    /// `data/missions/mail.ron`: free roam's mission mail.
    pub missions: missions::MissionsMail,
    /// `data/audio/cues.ron`, `music.ron` and `ambience.ron`.
    pub audio: audio::AudioData,
    /// `data/world/traffic.ron` and every car it names in `data/cars/`.
    pub traffic: traffic::TrafficData,
    /// `data/people/people.ron` and `lines.ron` (openspec change `civilians`).
    pub people: civilians::PeopleData,
    /// `data/world/ships.ron`: every boat on the island (openspec change `harbours`).
    pub ships: ships::ShipsConfig,
}

impl GameData {
    /// Loads every data file under `root` (the `data/` directory), with the mech file `mech`
    /// (relative to `root`) and its boarding timeline beside it (`mechs/<name>/boarding.ron`). The first failure is returned, naming its file and field.
    pub fn load(root: &Path, mech: &str) -> Result<Self, data::DataError> {
        let mut world: world::WorldConfig = data::load_ron(&root.join("world/world.ron"))?;
        // The houses' and small businesses' kits, read from the exported assets beside `data/`.
        let kits_dir = root.join("../assets/models/small_buildings");
        let mut kits = world::small_buildings::Kits::load(&kits_dir).map_err(|reason| data::DataError {
            path: kits_dir.join("manifest.json"),
            reason,
        })?;
        // The airport's and the seaport's kits and layouts (airport-layout, seaport-layout), beside the houses'.
        let sites_dir = root.join("../assets/models/sites");
        world.site_templates = world::sites::SiteTemplates::load(&sites_dir, &mut kits).map_err(|reason| data::DataError {
            path: sites_dir.clone(),
            reason,
        })?;
        world.small_kits = std::sync::Arc::new(kits);
        let audio = audio::AudioData {
            cues: data::load_ron(&root.join("audio/cues.ron"))?,
            music: data::load_ron(&root.join("audio/music.ron"))?,
            ambience: data::load_ron(&root.join("audio/ambience.ron"))?,
        };
        audio.check().map_err(|reason| data::DataError {
            path: root.join("audio/music.ron"),
            reason,
        })?;
        let skins_path = root.join("enemies/skins.ron");
        let skins: skins::Skins = data::load_ron(&skins_path)?;
        let encounters: ai::Encounters = data::load_ron(&root.join("world/encounters.ron"))?;
        let defence: mission::DefenceConfig = data::load_ron(&root.join("missions/defence.ron"))?;
        let shape_path = root.join("missions/fortress_shape.ron");
        let fortress = fortress::FortressData {
            cfg: data::load_ron(&root.join("missions/fortress.ron"))?,
            shape: data::load_ron(&shape_path)?,
        };
        fortress.check().map_err(|reason| data::DataError { path: shape_path, reason })?;
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
            }))
            .chain(fortress.cfg.guard.crabs.iter().filter_map(|(name, c)| {
                c.skin
                    .as_deref()
                    .map(|s| (s, format!("missions/fortress.ron guard.crabs.{name}.skin")))
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
        let ships_path = root.join("world/ships.ron");
        let ships: ships::ShipsConfig = data::load_ron(&ships_path)?;
        let units: units::UnitsConfig = data::load_ron(&root.join("world/units.ron"))?;
        // The gunboat is a vehicle in a fight and a boat when it sinks: one size in both files.
        if let (Some(u), Some(b)) = (units.units.get(ships::GUNBOAT), ships.kinds.get(ships::GUNBOAT))
            && (u.size_m != b.size_m || u.draft_m != b.draft_m)
        {
            return Err(data::DataError {
                path: ships_path,
                reason: format!(
                    "kinds.gunboat: size_m and draft_m must match units.ron's gunboat ({:?}, {})",
                    u.size_m, u.draft_m
                ),
            });
        }
        Ok(Self {
            sim: data::load_ron(&root.join("sim.ron"))?,
            world,
            mech: mech::MechConfig::load(root, mech)?,
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
            day_night: data::load_ron(&root.join("world/day_night.ron"))?,
            grass,
            ground_marks: data::load_ron(&root.join("fx/ground_marks.ron"))?,
            battle_fx: data::load_ron(&root.join("fx/battle_fx.ron"))?,
            sun: data::load_ron(&root.join("world/sun_flare.ron"))?,
            units,
            air_traffic: data::load_ron(&root.join("world/air_traffic.ron"))?,
            skins,
            fortress,
            missions: data::load_ron(&root.join("missions/mail.ron"))?,
            audio,
            traffic: traffic::TrafficData::load(root)?,
            people: civilians::PeopleData {
                cfg: data::load_ron(&root.join("people/people.ron"))?,
                lines: data::load_ron(&root.join("people/lines.ron"))?,
            },
            ships,
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
    fn every_roster_mech_carries_its_own_signature_weapon() {
        // Survey G20 (owner, 2026-10-26): each mech its signature, and no two the same.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let want = [
            ("driller", "cutting lance"),
            ("foreman", "boom cannon"),
            ("harrow", "twin racks"),
            ("ironhorse", "steam cannon"),
            ("mule", "rocket ripple"),
            ("picket", "rail rifle"),
            ("roustabout", "breaching gun"),
            ("sapper", "grenade launcher"),
        ];
        let mut seen = Vec::new();
        for (id, name) in want {
            let m = mech::MechConfig::load(&root, &format!("mechs/{id}.ron")).unwrap_or_else(|e| panic!("{e}"));
            let sig: Vec<_> = m.weapons.iter().filter(|w| w.signature).collect();
            assert_eq!(sig.len(), 1, "{id}: one signature");
            assert_eq!(sig[0].name, name, "{id}");
            assert!(m.weapons[0].signature, "{id}: the signature is listed first");
            assert!(!seen.contains(&name));
            seen.push(name);
        }
    }

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

    /// Every roster mech's own file, loaded: (file, config).
    fn roster_configs() -> Vec<(String, mech::MechConfig)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        GameData::roster(&root)
            .into_iter()
            .map(|m| {
                let cfg = data::load_ron(&root.join(&m.file)).unwrap_or_else(|e| panic!("{e}"));
                (m.file, cfg)
            })
            .collect()
    }

    /// Every roster mech has rockets and skates, each with stats of its own (survey C15, owner, 2026-10-26: "every
    /// mech has dash/skate, just diferent stats for them").
    #[test]
    fn every_roster_mech_has_rockets_and_skates_of_its_own() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mechs = roster_configs();
        assert_eq!(mechs.len(), 8);
        for (file, cfg) in &mechs {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            assert!(
                text.contains("\n    jet: (") && text.contains("\n    skate: ("),
                "{file} sets its own rockets and skates"
            );
            assert!(cfg.jet.burn_s > 0.0 && cfg.skate.burn_s > 0.0, "{file}");
            assert!(cfg.jet.thrust_m_s2 > 2.0 * 9.81, "{file}'s rockets lift it with room to climb");
        }
        for (i, (a, ca)) in mechs.iter().enumerate() {
            for (b, cb) in &mechs[i + 1..] {
                assert!((&ca.jet, &ca.skate) != (&cb.jet, &cb.skate), "{a} and {b} move alike");
            }
        }
    }

    /// The stats in each mech's file are the ones its simulation runs on: its skates give out at its own seconds of
    /// use and never pass its own top speed, its rockets give out at its own seconds of burn.
    #[test]
    fn each_mech_skates_and_flies_on_its_own_budgets() {
        use mech::{LocomotionMode, MechControls, MechState};
        const DT: f32 = 1.0 / 60.0;
        let ground = world::FlatGround::default();
        let mut at_ten_s = std::collections::BTreeMap::new();
        for (file, cfg) in roster_configs() {
            let mut m = MechState::standing(glam::Vec3::ZERO, 0.0, &cfg.gait);
            let on = MechControls {
                skate_toggle: true,
                throttle_axis: 1.0,
                ..Default::default()
            };
            m.step(&on, &cfg, &ground, DT);
            assert_eq!(m.mode, LocomotionMode::Skate, "{file}");
            let go = MechControls {
                throttle_axis: 1.0,
                ..Default::default()
            };
            let (mut t, mut top) = (DT, 0.0f32);
            while !m.skate.locked && t < 120.0 {
                m.step(&go, &cfg, &ground, DT);
                t += DT;
                top = top.max(m.vel.length());
                if (t - 10.0).abs() < DT / 2.0 {
                    at_ten_s.insert(file.clone(), m.vel.length());
                }
            }
            assert!(
                (t - cfg.skate.burn_s).abs() < 0.5,
                "{file}: skates gave out at {t} s, not {}",
                cfg.skate.burn_s
            );
            assert!(
                top <= cfg.skate.speed_max_m_s + 0.5,
                "{file}: {top} m/s past {}",
                cfg.skate.speed_max_m_s
            );

            let mut m = MechState::standing(glam::Vec3::ZERO, 0.0, &cfg.gait);
            let up = MechControls {
                lift_axis: 1.0,
                ..Default::default()
            };
            let mut t = 0.0;
            while !m.boost.locked && t < 120.0 {
                m.step(&up, &cfg, &ground, DT);
                t += DT;
            }
            assert!(
                (t - cfg.jet.burn_s).abs() < 0.5,
                "{file}: rockets gave out at {t} s, not {}",
                cfg.jet.burn_s
            );
        }
        assert!(
            at_ten_s["mechs/picket.ron"] > at_ten_s["mechs/mule.ron"],
            "the light Picket out-skates the hauling Mule: {at_ten_s:?}"
        );
    }

    #[test]
    fn the_fortress_launches_a_unit_that_exists() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let d = GameData::load(&root, "mechs/placeholder.ron").unwrap();
        let unit = &d.fortress.cfg.launch.unit;
        assert!(d.units.units.contains_key(unit), "no unit `{unit}` in data/world/units.ron");
    }

    #[test]
    fn the_fortress_mail_names_the_fortress_by_its_data_name() {
        // Survey H2 (owner, 2026-10-26): the crab air fortress is the Stratostacean, wherever the player reads it.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let d = GameData::load(&root, "mechs/placeholder.ron").unwrap();
        assert_eq!(d.fortress.cfg.name, "Stratostacean");
        let mail = d
            .missions
            .mail
            .iter()
            .find(|m| m.mission == missions::MissionKind::Fortress)
            .unwrap();
        assert!(mail.subject.contains(&d.fortress.cfg.name), "{}", mail.subject);
        assert!(mail.body.contains(&d.fortress.cfg.name), "{}", mail.body);
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
