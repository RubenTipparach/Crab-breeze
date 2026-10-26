//! The app's states (openspec change `menu-in-game`): the main menu, the one frame a run is set up in, and the game.
//! One program and one window go from the menu to the game (owner, 2026-10-25: "it should all be the same instance,
//! not relaunch the game"). The menu has no world behind it: every world spawn runs on entering `Playing`, and every
//! gameplay system is in `InGame`, which runs only while playing. It lives here, apart from any one capability, because
//! every plugin that spawns or runs the world hangs off it.

use bevy::ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;

/// Where the program is.
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameState {
    /// The main menu: its pages and a 2D camera, no world, no mech, no loading.
    #[default]
    Menu,
    /// A run is picked (on the menu or the command line) and its resources are in: the next frame plays it. A run
    /// starts here rather than in `Playing` so the world spawns after every startup schedule, as it did when they were
    /// `Startup` systems (the first state's `OnEnter` runs before `PreStartup`).
    Loading,
    /// The game: the world is spawned and the simulation runs. The loading screen shows until it is ready.
    Playing,
}

/// Every gameplay system in `First`, `FixedUpdate`, `Update` and `Last`: they run only while playing, so nothing steps
/// or draws the world behind the menu.
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct InGame;

/// The world's spawns, on entering `Playing` (each plugin's part: roads, ground, mech, pilot, HUD and the rest). They
/// were `Startup` systems.
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WorldSpawn;

/// What reads the spawned world on entering `Playing` (the MFD pages, the screens cube, a capture's offscreen
/// target). They were `PostStartup` systems, and run after `WorldSpawn` as they ran after `Startup`.
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct AfterWorldSpawn;

/// Registers a plugin's systems on the game's states.
pub trait GameApp {
    /// Adds systems to `schedule` that run only while playing (`InGame`).
    fn add_game_systems<M>(&mut self, schedule: impl ScheduleLabel, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self;
    /// Adds a world spawn, run on entering `Playing` (`WorldSpawn`).
    fn add_world_spawn<M>(&mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self;
    /// Adds a system that reads the spawned world on entering `Playing` (`AfterWorldSpawn`).
    fn add_after_world_spawn<M>(&mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self;
}

impl GameApp for App {
    fn add_game_systems<M>(&mut self, schedule: impl ScheduleLabel, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self {
        self.add_systems(schedule, systems.in_set(InGame))
    }

    fn add_world_spawn<M>(&mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self {
        self.add_systems(OnEnter(GameState::Playing), systems.in_set(WorldSpawn))
    }

    fn add_after_world_spawn<M>(&mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self {
        self.add_systems(OnEnter(GameState::Playing), systems.in_set(AfterWorldSpawn))
    }
}

/// The game's states, starting in `first`: `Menu` for a plain run, `Loading` for a mode named on the command line.
pub struct StatePlugin {
    /// The state the program opens in.
    pub first: GameState,
}

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(self.first)
            .configure_sets(First, InGame.run_if(in_state(GameState::Playing)))
            .configure_sets(FixedUpdate, InGame.run_if(in_state(GameState::Playing)))
            .configure_sets(Update, InGame.run_if(in_state(GameState::Playing)))
            .configure_sets(Last, InGame.run_if(in_state(GameState::Playing)))
            .configure_sets(OnEnter(GameState::Playing), AfterWorldSpawn.after(WorldSpawn))
            .add_systems(OnEnter(GameState::Loading), play);
    }
}

/// A run is set up: play it on the next frame.
fn play(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::Playing);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[derive(Resource, Default)]
    struct Ran(u32);

    fn app(first: GameState) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .add_plugins(StatePlugin { first })
            .init_resource::<Ran>()
            .add_game_systems(Update, |mut ran: ResMut<Ran>| ran.0 += 1);
        app
    }

    #[test]
    fn nothing_in_game_runs_behind_the_menu() {
        let mut app = app(GameState::Menu);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Ran>().0, 0);
        assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::Menu);
    }

    #[test]
    fn a_run_set_up_plays_on_the_next_frame() {
        let mut app = app(GameState::Loading);
        app.update();
        app.update();
        assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::Playing);
        let ran = app.world().resource::<Ran>().0;
        app.update();
        assert_eq!(app.world().resource::<Ran>().0, ran + 1, "it runs every frame once playing");
    }

    #[test]
    fn the_world_spawns_once_on_entering_play_and_what_reads_it_runs_after() {
        #[derive(Resource, Default)]
        struct Order(Vec<&'static str>);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .add_plugins(StatePlugin { first: GameState::Loading })
            .init_resource::<Order>()
            .add_after_world_spawn(|mut o: ResMut<Order>| o.0.push("after"))
            .add_world_spawn(|mut o: ResMut<Order>| o.0.push("spawn"));
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(app.world().resource::<Order>().0, vec!["spawn", "after"]);
    }
}
