//! Crab Breeze, the game. This file is the composition root (CLAUDE.md section 6.2): it reads the
//! command line and the data, and wires the plugins. It holds no game logic.

mod air_bases;
mod ambience;
mod audio;
mod battle_fx;
mod bench;
mod blast;
mod building_arrays;
mod building_state;
mod buildings;
mod capture;
mod capture_fortress;
mod career;
mod cli;
mod clouds_render;
mod combat;
mod defence;
mod demolition;
mod fortress;
mod fx;
mod fx_gallery;
mod grass;
mod ground;
mod ground_marks;
mod hud;
mod input;
mod launcher;
mod loading;
mod mech;
mod menu;
mod mfd;
mod mfd_apps;
mod missions;
mod music;
mod particles;
mod pbr_set;
mod perf_hud;
mod physics;
mod pilot;
mod plumes;
mod rocks;
mod rope;
mod save_game;
mod screen_cut;
mod screens_cube;
mod settings;
#[cfg(test)]
mod shader_check;
mod showcase;
mod sim;
mod skins;
mod sky_light;
mod soft_sprites;
mod sun_flare;
mod targeting;
mod textures;
mod traffic;
mod units;
mod vents;
mod water_render;
mod world;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use cb_core::GameData;

fn main() -> AppExit {
    let args = match cli::Args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n{}", cli::USAGE);
            return AppExit::error();
        }
    };
    // A plain run opens the menu on its own, with no world (launcher); picking a mode starts the game.
    let scripted = args.stills.is_some() || args.showcase.is_some() || args.bench.is_some();
    if !scripted && args.play.is_none() && args.mission.is_none() && args.start.is_none() {
        return launcher::run();
    }
    let root = cli::repo_root();
    let data = match GameData::load(&root.join("data"), &args.mech) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("data failed to load, stopping: {e}");
            return AppExit::error();
        }
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Crab Breeze".into(),
                    // An offscreen bench needs only a small window; a windowed one is the player's.
                    resolution: if args.bench.is_some() && !cli::switched_off("CB_BENCH_WINDOWED") {
                        WindowResolution::new(640, 360)
                    } else {
                        WindowResolution::new(1920, 1080)
                    },
                    // On the main display, centred, so a run is seen where the owner looks.
                    position: bevy::window::WindowPosition::Centered(bevy::window::MonitorSelection::Primary),
                    // The capture instrument measures frame cost, so it must not wait for vsync.
                    // `CB_UNCAPPED=1` (perf-bench switch): on screen but not waiting for vsync,
                    // so a 60 Hz display does not hide the headroom under 120 fps.
                    present_mode: if args.stills.is_some() || cli::switched_off("CB_UNCAPPED") {
                        PresentMode::AutoNoVsync
                    } else {
                        PresentMode::AutoVsync
                    },
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.join("assets").to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .insert_resource(sim::Data(data))
    .insert_resource(world::Seed(args.seed))
    .add_plugins(textures::TexturesPlugin);
    if args.start.as_deref() == Some("meadow") {
        app.insert_resource(world::StartInMeadow);
    }
    if args.start.as_deref() == Some("base") {
        app.insert_resource(world::StartAtBase);
    }
    // The blended smoke drawn sorted and soft (openspec change `particle-depth`).
    app.add_plugins(soft_sprites::SoftSpritesPlugin);
    if args.start.as_deref() == Some("fx") {
        app.add_plugins(fx_gallery::FxGalleryPlugin);
    }
    // The gallery stands on the overlook's rise, so its rows are seen from above the grass.
    if matches!(args.start.as_deref(), Some("overlook") | Some("fx")) {
        app.insert_resource(world::StartOnOverlook);
    }
    app.add_plugins((
        sim::SimPlugin,
        input::InputPlugin,
        world::WorldPlugin,
        mech::MechPlugin,
        pilot::PilotPlugin,
        particles::ParticlesPlugin,
        fx::FxPlugin,
        blast::BlastPlugin,
        combat::CombatPlugin,
        building_state::BuildingStatePlugin,
        buildings::BuildingsPlugin,
        physics::DebrisPhysicsPlugin,
        hud::HudPlugin,
        water_render::WaterRenderPlugin,
        targeting::TargetingPlugin,
    ))
    .add_plugins((
        screens_cube::ScreensCubePlugin,
        sky_light::SkyLightPlugin,
        skins::SkinsPlugin,
        battle_fx::BattleFxPlugin,
        plumes::PlumesPlugin,
        loading::LoadingPlugin,
        perf_hud::PerfHudPlugin,
        units::UnitsPlugin,
        mfd::MfdPlugin,
        mfd_apps::MfdAppsPlugin,
        menu::MenuPlugin,
        rope::RopePlugin,
        vents::VentsPlugin,
        career::CareerPlugin,
    ))
    // Free roam's mail and its missions (free-roam-missions); the Hermit (crab-airship).
    .add_plugins((missions::MissionsPlugin, fortress::FortressPlugin))
    // Car traffic and the car you drive (openspec changes `car-traffic`, `car-driving`).
    .add_plugins(traffic::TrafficPlugin)
    // sound: effects in 3D, music, ambience (openspec changes `audio`, `music`, `ambience`)
    .add_plugins((audio::SoundPlugin, music::MusicPlugin, ambience::AmbiencePlugin));
    // Cheap, correctly lit clouds (openspec change `cloud-puffs`); `CB_NO_CLOUDS=1` leaves them
    // out, so the benchmark can measure what they cost.
    if !cli::switched_off("CB_NO_CLOUDS") {
        app.add_plugins(clouds_render::CloudsRenderPlugin);
    }
    // GPU grass (openspec change `gpu-grass`); `CB_NO_GRASS=1` leaves it out, so the benchmark can
    // measure what it costs.
    if !cli::switched_off("CB_NO_GRASS") {
        app.add_plugins(grass::GrassPlugin);
    }
    // Footprints, skate trails and crater scorches (openspec change `ground-marks`);
    // `CB_NO_GROUND_MARKS=1` draws none (the dirt, the grit and the jolt stay), so the benchmark can
    // measure what the marks cost.
    app.add_plugins(ground_marks::GroundMarksPlugin {
        enabled: !cli::switched_off("CB_NO_GROUND_MARKS"),
    });
    // The sun's bloom and lens flare (openspec change `sun-flare`); `CB_NO_FLARE=1` and
    // `CB_NO_BLOOM=1` leave them out, so the benchmark can measure what they cost.
    app.add_plugins(sun_flare::SunFlarePlugin {
        flare: !cli::switched_off("CB_NO_FLARE"),
        bloom: !cli::switched_off("CB_NO_BLOOM"),
    });
    // The run begins: a command-line run as asked (the defence mission, or free roam for a take, a
    // bench or stills), or the mode picked on the menu (launcher): free roam or a career.
    app.add_plugins(defence::DefencePlugin);
    if args.defending() {
        app.insert_resource(defence::Defending)
            .init_resource::<defence::DefenceRun>()
            .insert_resource(menu::Play::Defence);
    } else if args.stills.is_some() && std::env::var("CB_STILLS_MAIL").is_ok() {
        // The stills of taking the free-roam mail on the cockpit's MAIL page: free roam as a player plays it, no
        // encounter mechs (capture::run_script's MAIL_BEAT).
        app.insert_resource(menu::Play::FreeRoam);
    } else if scripted || args.mission.is_some() || args.start.is_some() || args.play.as_deref().is_some_and(|m| m.starts_with("free")) {
        app.insert_resource(menu::Play::FreeRoam);
        if args.mission.as_deref() == Some("fortress") {
            app.insert_resource(missions::StartWith(cb_core::missions::MissionKind::Fortress));
        }
        // A scripted take fights the encounter mechs; free roam as a player plays it has none.
        if scripted {
            app.insert_resource(menu::Encounters);
        }
    } else if let Some(career) = args.play.as_deref().and_then(|m| m.strip_prefix("career-")) {
        app.insert_resource(menu::Play::Career).insert_resource(if career == "new" {
            career::CareerStart::New
        } else {
            career::CareerStart::Continue
        });
    }
    // A player's game (picked on the menu) saves where they are and continues from it (save-game).
    if let Some(mode) = args.play.as_deref() {
        app.insert_resource(save_game::Saving {
            free_roam: mode.starts_with("free"),
            restore: mode.ends_with("continue"),
            mech: args.mech.clone(),
        });
    }
    app.add_plugins(save_game::SaveGamePlugin);
    app.add_plugins(air_bases::AirBasesPlugin);
    // The player's settings; a measurement or capture run keeps the window it set up.
    app.add_plugins(settings::SettingsPlugin {
        window: !scripted && !cli::switched_off("CB_UNCAPPED"),
    });
    if let Some(dir) = args.stills {
        if std::env::var("CB_STILLS_FORTRESS").is_ok() {
            app.add_plugins(capture_fortress::FortressStillsPlugin { dir });
        } else if std::env::var("CB_STILLS_FX").is_ok() {
            app.add_plugins(capture::FxStillsPlugin { dir });
        } else if std::env::var("CB_STILLS_WEATHER").is_ok() {
            app.add_plugins(capture::WeatherStillsPlugin { dir });
        } else {
            app.add_plugins(capture::StillsPlugin { dir });
        }
    } else if let Some(take) = args.showcase.clone().or(args.bench.clone()) {
        app.add_plugins(showcase::ShowcasePlugin { take });
        if let Ok(dir) = std::env::var("CB_FRAMES_DIR") {
            app.add_plugins(capture::FramesPlugin { dir: dir.into() });
        }
    }
    if args.bench.is_some() {
        app.add_plugins(bench::BenchPlugin);
    } else {
        app.add_plugins(sim::FrameCapPlugin);
    }
    app.run()
}
