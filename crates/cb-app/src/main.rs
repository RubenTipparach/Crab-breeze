//! Crab Breeze, the game. This file is the composition root (CLAUDE.md section 6.2): it reads the
//! command line and the data, and wires the plugins. It holds no game logic.

mod battle_fx;
mod bench;
mod blast;
mod buildings;
mod capture;
mod cli;
mod combat;
mod defence;
mod fx;
mod grass;
mod ground;
mod ground_marks;
mod hud;
mod input;
mod loading;
mod mech;
mod mfd;
mod particles;
mod pbr_set;
mod perf_hud;
mod physics;
mod pilot;
mod plumes;
mod screen_cut;
mod screens_cube;
#[cfg(test)]
mod shader_check;
mod showcase;
mod sim;
mod skins;
mod sun_flare;
mod targeting;
mod textures;
mod units;
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
        buildings::BuildingsPlugin,
        physics::DebrisPhysicsPlugin,
        hud::HudPlugin,
        water_render::WaterRenderPlugin,
        targeting::TargetingPlugin,
    ))
    .add_plugins((
        screens_cube::ScreensCubePlugin,
        skins::SkinsPlugin,
        battle_fx::BattleFxPlugin,
        plumes::PlumesPlugin,
        loading::LoadingPlugin,
        perf_hud::PerfHudPlugin,
        units::UnitsPlugin,
        mfd::MfdPlugin,
    ));
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
    if args.defending() {
        app.add_plugins(defence::DefencePlugin);
    }
    if let Some(dir) = args.stills {
        if std::env::var("CB_STILLS_FX").is_ok() {
            app.add_plugins(capture::FxStillsPlugin { dir });
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
