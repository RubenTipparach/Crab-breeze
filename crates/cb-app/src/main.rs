//! Crab Breeze, the game. This file is the composition root (CLAUDE.md section 6.2): it reads the
//! command line and the data, and wires the plugins. It holds no game logic.

mod bench;
mod blast;
mod buildings;
mod capture;
mod cli;
mod combat;
mod fx;
mod hud;
mod input;
mod mech;
mod physics;
mod pilot;
mod showcase;
mod sim;
mod textures;
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
                    resolution: if args.bench.is_some() {
                        WindowResolution::new(640, 360)
                    } else {
                        WindowResolution::new(1920, 1080)
                    },
                    // The capture instrument measures frame cost, so it must not wait for vsync.
                    present_mode: if args.stills.is_some() {
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
    .add_plugins(textures::TexturesPlugin)
    .add_plugins((
        sim::SimPlugin,
        input::InputPlugin,
        world::WorldPlugin,
        mech::MechPlugin,
        pilot::PilotPlugin,
        fx::FxPlugin,
        blast::BlastPlugin,
        combat::CombatPlugin,
        buildings::BuildingsPlugin,
        physics::DebrisPhysicsPlugin,
        hud::HudPlugin,
        water_render::WaterRenderPlugin,
    ));
    if let Some(dir) = args.stills {
        app.add_plugins(capture::StillsPlugin { dir });
    } else if let Some(take) = args.showcase.clone().or(args.bench.clone()) {
        app.add_plugins(showcase::ShowcasePlugin { fight: take == "fight" });
    }
    if args.bench.is_some() {
        app.add_plugins(bench::BenchPlugin);
    }
    app.run()
}
