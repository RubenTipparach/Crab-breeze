//! The main menu, in the game's own window (openspec change `menu-in-game`; owner, 2026-10-25: "to be an acutal menu
//! that is NOT INA GAME", "when NOT INA GAME, there should be no GAME LOADING, NO WORLD", "it should all be the same
//! instance, not relaunch the game"). A plain run opens on it in `GameState::Menu`: its pages and a 2D camera, no
//! world, no mech, no loading. Picking a mode sets the run up (`begin_run`, the one implementation a `--play` run
//! uses too) and moves to `Loading`, and the world spawns behind the loading screen in the same program. Continue
//! has two options, career or free roam; starting either new warns that it deletes the existing save ("do warn that
//! starting new career or free roam will delete exisitng save"). City defence is not here: it is offered by mail in
//! free roam. Free roam asks which mech first (openspec change `mech-select`), from the roster,
//! `cb_core::GameData::roster`.

use crate::state::GameState;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use std::path::PathBuf;

/// The two kinds of game, each with its own save.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Career,
    FreeRoam,
}

impl Kind {
    fn save(self) -> PathBuf {
        match self {
            Kind::Career => crate::career::save_path(),
            Kind::FreeRoam => free_roam_save_path(),
        }
    }

    fn saved(self) -> bool {
        self.save().exists()
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Career => "career",
            Kind::FreeRoam => "free roam",
        }
    }

    /// The `--play` mode that starts it fresh, or continues its save.
    fn mode(self, continue_save: bool) -> &'static str {
        match (self, continue_save) {
            (Kind::Career, false) => "career-new",
            (Kind::Career, true) => "career-continue",
            (Kind::FreeRoam, false) => "free",
            (Kind::FreeRoam, true) => "free-continue",
        }
    }
}

/// Where a free-roam game is saved (written by `save-game`).
pub fn free_roam_save_path() -> PathBuf {
    crate::cli::repo_root().join("saves").join("free_roam.ron")
}

/// The data file of the mech this program's game data was loaded with (`--mech`, `CB_MECH` or the default), kept so
/// a run picked in another mech knows to load that one's data.
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct MechFile(pub String);

/// Sets a player's run up, the one way both the menu and `--play <mode>` do it: the mode's resources (`menu::Play`,
/// `career::CareerStart`, `save_game::Saving`), and the game data reloaded when the run flies another mech than the
/// one loaded. A `Play` already inserted (a command-line mission or start) is kept. Fails, changing nothing, when the
/// mech's data does not load.
pub fn begin_run(world: &mut World, mode: &str, mech: &str) -> Result<(), String> {
    if world.get_resource::<MechFile>().is_none_or(|m| m.0 != mech) {
        let data = cb_core::GameData::load(&crate::cli::repo_root().join("data"), mech).map_err(|e| e.to_string())?;
        world.insert_resource(crate::sim::Data(data));
        world.insert_resource(MechFile(mech.to_string()));
    }
    if world.get_resource::<crate::menu::Play>().is_none() {
        if let Some(career) = mode.strip_prefix("career-") {
            world.insert_resource(crate::menu::Play::Career);
            world.insert_resource(if career == "new" {
                crate::career::CareerStart::New
            } else {
                crate::career::CareerStart::Continue
            });
        } else {
            world.insert_resource(crate::menu::Play::FreeRoam);
        }
    }
    world.insert_resource(crate::save_game::Saving {
        free_roam: mode.starts_with("free"),
        restore: mode.ends_with("continue"),
        mech: mech.to_string(),
        write: true,
    });
    Ok(())
}

/// The mech a free-roam save was made in, so Continue brings back the mech the player left in (a swapped one too).
fn saved_free_roam_mech() -> Option<String> {
    let text = std::fs::read_to_string(free_roam_save_path()).ok()?;
    cb_core::save::FreeRoamSave::from_ron(&text).ok().map(|s| s.mech)
}

/// The menu's page.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Main,
    Continue,
    /// Which mech free roam starts in (the roster).
    PickMech,
    /// Starting this kind new deletes its save: say so first (and the mech picked for free roam, if any).
    Confirm(Kind, Option<usize>),
    /// The player's settings, the pause menu's Options page (owner, 2026-10-28: "game needs options in main menu").
    Options,
}

/// The roster the mech page lists, read once.
#[derive(Resource)]
struct Roster(Vec<cb_core::RosterMech>);

/// What a button does.
#[derive(Component, Clone, Copy)]
enum Action {
    OpenContinue,
    Continue(Kind),
    New(Kind),
    /// Free roam in this roster mech.
    Mech(usize),
    Start(Kind, Option<usize>),
    OpenOptions,
    Back,
    Quit,
}

/// The page's root node.
#[derive(Component)]
struct Root;

const INK: Color = Color::srgb(0.55, 0.95, 0.6);
const WARN: Color = Color::srgb(1.0, 0.75, 0.3);
const PANEL: Color = Color::srgb(0.02, 0.05, 0.04);
const EDGE: Color = Color::srgb(0.25, 0.55, 0.32);
const BACKDROP: Color = Color::srgb(0.01, 0.02, 0.02);

/// The main menu's pages, built on entering `Menu` and taken down on leaving it.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Page::Main)
            .insert_resource(Roster(cb_core::GameData::roster(&crate::cli::repo_root().join("data"))))
            .add_systems(OnEnter(GameState::Menu), open)
            .add_systems(Update, draw.run_if(in_state(GameState::Menu)));
    }
}

/// The menu's own camera, and its first page.
fn open(mut commands: Commands, mut page: ResMut<Page>) {
    commands.spawn((
        Name::new("main menu camera"),
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
        DespawnOnExit(GameState::Menu),
    ));
    *page = Page::Main;
    page.set_changed();
}

/// Rebuilds the page whenever it changes.
fn draw(
    mut commands: Commands,
    page: Res<Page>,
    roster: Res<Roster>,
    settings: Res<crate::settings::Settings>,
    roots: Query<Entity, With<Root>>,
) {
    if !page.is_changed() && !(settings.is_changed() && *page == Page::Options) {
        return;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    let mut lines: Vec<(&str, Color)> = Vec::new();
    let mut buttons: Vec<(Action, String)> = Vec::new();
    match *page {
        Page::Main => {
            if Kind::Career.saved() || Kind::FreeRoam.saved() {
                buttons.push((Action::OpenContinue, "Continue".into()));
            }
            buttons.push((Action::New(Kind::FreeRoam), "Free roam".into()));
            buttons.push((Action::New(Kind::Career), "New career".into()));
            buttons.push((Action::OpenOptions, "Options".into()));
            buttons.push((Action::Quit, "Quit".into()));
        }
        Page::Continue => {
            lines.push(("Continue", INK));
            for kind in [Kind::Career, Kind::FreeRoam] {
                if kind.saved() {
                    let label = if kind == Kind::Career { "Career" } else { "Free roam" };
                    buttons.push((Action::Continue(kind), label.into()));
                }
            }
            buttons.push((Action::Back, "Back".into()));
        }
        Page::PickMech => {
            lines.push(("Which mech?", INK));
            for (i, m) in roster.0.iter().enumerate() {
                buttons.push((Action::Mech(i), m.name.clone()));
            }
            buttons.push((Action::Back, "Back".into()));
        }
        Page::Confirm(kind, mech) => {
            lines.push(("This deletes your saved", WARN));
            lines.push((kind.name(), WARN));
            lines.push(("and starts again.", WARN));
            let start = if kind == Kind::Career {
                "Start new career"
            } else {
                "Start free roam"
            };
            buttons.push((Action::Start(kind, mech), start.into()));
            buttons.push((Action::Back, "Back".into()));
        }
        Page::Options => {
            lines.push(("Options", INK));
            buttons.push((Action::Back, "Back".into()));
        }
    }
    commands
        .spawn((
            Root,
            DespawnOnExit(GameState::Menu),
            Node {
                width: percent(100.0),
                height: percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(14.0),
                    padding: UiRect::all(px(32.0)),
                    border: UiRect::all(px(2.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(EDGE),
            ))
            .with_children(|p| {
                p.spawn((Text::new("CRAB BREEZE"), TextFont::from_font_size(64.0), TextColor(INK)));
                for (line, colour) in lines {
                    p.spawn((Text::new(line), TextFont::from_font_size(26.0), TextColor(colour)));
                }
                if *page == Page::Options {
                    crate::menu::options_rows(p, &settings.0);
                }
                for (action, label) in buttons {
                    p.spawn((
                        action,
                        crate::ui_feedback::UiButton::default(),
                        Node {
                            width: px(340.0),
                            height: px(56.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(2.0)),
                            ..default()
                        },
                        BackgroundColor(PANEL),
                        BorderColor::all(EDGE),
                    ))
                    .observe(
                        move |_: On<PointerClick>,
                              mut page: ResMut<Page>,
                              roster: Res<Roster>,
                              mut exit: MessageWriter<AppExit>,
                              mut commands: Commands| {
                            press(action, &mut page, &roster.0, &mut exit, &mut commands);
                        },
                    )
                    .with_child((Text::new(label), TextFont::from_font_size(28.0), TextColor(INK)));
                }
            });
        });
}

/// A click moves between pages, starts a run (or quits).
fn press(action: Action, page: &mut Page, roster: &[cb_core::RosterMech], exit: &mut MessageWriter<AppExit>, commands: &mut Commands) {
    let mech = |i: Option<usize>| i.and_then(|i| roster.get(i)).map(|m| m.file.clone());
    match action {
        Action::OpenContinue => *page = Page::Continue,
        Action::OpenOptions => *page = Page::Options,
        Action::Back => *page = Page::Main,
        Action::Quit => {
            exit.write(AppExit::Success);
        }
        Action::Continue(kind) => {
            let mech = if kind == Kind::FreeRoam { saved_free_roam_mech() } else { None };
            start(commands, kind.mode(true), mech);
        }
        // Free roam: which mech first.
        Action::New(Kind::FreeRoam) => *page = Page::PickMech,
        // A new game over a save asks first.
        Action::Mech(i) if Kind::FreeRoam.saved() => *page = Page::Confirm(Kind::FreeRoam, Some(i)),
        Action::New(kind) if kind.saved() => *page = Page::Confirm(kind, None),
        Action::Mech(i) => start(commands, Kind::FreeRoam.mode(false), mech(Some(i))),
        Action::New(kind) | Action::Start(kind, _) => {
            let picked = if let Action::Start(_, m) = action { mech(m) } else { None };
            if let Err(e) = std::fs::remove_file(kind.save())
                && e.kind() != std::io::ErrorKind::NotFound
            {
                error!("menu: could not delete the old {} save: {e}", kind.name());
                return;
            }
            start(commands, kind.mode(false), picked);
        }
    }
}

/// Starts a run in `mode` in this program: the run is set up, then the world loads behind the loading screen. With no
/// mech picked, the one this program was started with flies.
fn start(commands: &mut Commands, mode: &'static str, mech: Option<String>) {
    commands.queue(move |world: &mut World| {
        let mech = mech.unwrap_or_else(|| world.resource::<MechFile>().0.clone());
        match begin_run(world, mode, &mech) {
            Ok(()) => {
                info!("menu: {mode} in {mech}");
                world.resource_mut::<NextState<GameState>>().set(GameState::Loading);
            }
            Err(e) => error!("menu: {mech} does not load, staying on the menu: {e}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with(mech: &str) -> World {
        let mut world = World::new();
        world.insert_resource(MechFile(mech.into()));
        world
    }

    #[test]
    fn a_new_career_starts_a_career_and_saves_it_without_restoring() {
        let mut world = world_with(crate::cli::DEFAULT_MECH);
        begin_run(&mut world, "career-new", crate::cli::DEFAULT_MECH).unwrap();
        assert_eq!(*world.resource::<crate::menu::Play>(), crate::menu::Play::Career);
        assert!(matches!(
            *world.resource::<crate::career::CareerStart>(),
            crate::career::CareerStart::New
        ));
        let s = world.resource::<crate::save_game::Saving>();
        assert!(!s.free_roam && !s.restore);
    }

    #[test]
    fn continuing_free_roam_restores_its_save_in_its_mech() {
        let mut world = world_with(crate::cli::DEFAULT_MECH);
        begin_run(&mut world, "free-continue", crate::cli::DEFAULT_MECH).unwrap();
        assert_eq!(*world.resource::<crate::menu::Play>(), crate::menu::Play::FreeRoam);
        let s = world.resource::<crate::save_game::Saving>();
        assert!(s.free_roam && s.restore && s.mech == crate::cli::DEFAULT_MECH);
    }

    #[test]
    fn a_mission_named_on_the_command_line_keeps_its_play() {
        let mut world = world_with(crate::cli::DEFAULT_MECH);
        world.insert_resource(crate::menu::Play::Defence);
        begin_run(&mut world, "free", crate::cli::DEFAULT_MECH).unwrap();
        assert_eq!(*world.resource::<crate::menu::Play>(), crate::menu::Play::Defence);
    }

    #[test]
    fn a_mech_that_does_not_load_changes_nothing() {
        let mut world = world_with(crate::cli::DEFAULT_MECH);
        assert!(begin_run(&mut world, "free", "mechs/no_such_mech.ron").is_err());
        assert!(world.get_resource::<crate::menu::Play>().is_none());
        assert_eq!(world.resource::<MechFile>().0, crate::cli::DEFAULT_MECH);
    }
}
