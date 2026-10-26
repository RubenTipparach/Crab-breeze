//! The title menu, on its own (owner, 2026-10-25: "to be an acutal menu that is NOT INA GAME", "when
//! NOT INA GAME, there should be no GAME LOADING, NO WORLD", "free roam top priority", "free roam and
//! carreer", "continue will have two options, careeer or free roam", "do warn that starting new career
//! or free roam will delete exisitng save"). A plain run of the game opens this small app first: a
//! window with the menu and nothing else, no world, no mech, no loading. Picking a mode starts the game
//! itself in that mode (`--play`), with every other argument passed on (`--mech` and the like), and the
//! menu closes. City defence is not here: it is offered by mail in free roam. Scripted runs never see it.

use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use std::path::PathBuf;

/// The two kinds of game, each with its own save.
#[derive(Clone, Copy, PartialEq, Eq)]
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

/// Where a free-roam game is saved (written by `save-game` when it lands).
pub fn free_roam_save_path() -> PathBuf {
    crate::cli::repo_root().join("saves").join("free_roam.ron")
}

/// The menu's page.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum Page {
    Main,
    Continue,
    /// Starting this kind new deletes its save: say so first.
    Confirm(Kind),
}

/// What a button does.
#[derive(Component, Clone, Copy)]
enum Action {
    OpenContinue,
    Continue(Kind),
    New(Kind),
    Start(Kind),
    Back,
    Quit,
}

#[derive(Component)]
struct Root;

const INK: Color = Color::srgb(0.55, 0.95, 0.6);
const WARN: Color = Color::srgb(1.0, 0.75, 0.3);
const PANEL: Color = Color::srgb(0.02, 0.05, 0.04);
const EDGE: Color = Color::srgb(0.25, 0.55, 0.32);

/// Runs the menu until a game is started (the menu then closes) or Quit.
pub fn run() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Crab Breeze".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.01, 0.02, 0.02)))
        .insert_resource(Page::Main)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, draw)
        .run()
}

/// Rebuilds the page whenever it changes.
fn draw(mut commands: Commands, page: Res<Page>, roots: Query<Entity, With<Root>>) {
    if !page.is_changed() {
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
        Page::Confirm(kind) => {
            lines.push(("This deletes your saved", WARN));
            lines.push((kind.name(), WARN));
            lines.push(("and starts again.", WARN));
            let start = if kind == Kind::Career {
                "Start new career"
            } else {
                "Start free roam"
            };
            buttons.push((Action::Start(kind), start.into()));
            buttons.push((Action::Back, "Back".into()));
        }
    }
    commands
        .spawn((
            Root,
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
                for (action, label) in buttons {
                    p.spawn((
                        action,
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
                        move |_: On<PointerClick>, mut page: ResMut<Page>, mut exit: MessageWriter<AppExit>| {
                            press(action, &mut page, &mut exit);
                        },
                    )
                    .with_child((Text::new(label), TextFont::from_font_size(28.0), TextColor(INK)));
                }
            });
        });
}

/// A click moves between pages, or starts the game (or quits).
fn press(action: Action, page: &mut Page, exit: &mut MessageWriter<AppExit>) {
    match action {
        Action::OpenContinue => *page = Page::Continue,
        Action::Back => *page = Page::Main,
        Action::Quit => {
            exit.write(AppExit::Success);
        }
        Action::Continue(kind) => {
            start_game(kind.mode(true));
            exit.write(AppExit::Success);
        }
        // A new game over a save asks first.
        Action::New(kind) if kind.saved() => *page = Page::Confirm(kind),
        Action::New(kind) | Action::Start(kind) => {
            if let Err(e) = std::fs::remove_file(kind.save())
                && e.kind() != std::io::ErrorKind::NotFound
            {
                error!("menu: could not delete the old {} save: {e}", kind.name());
                return;
            }
            start_game(kind.mode(false));
            exit.write(AppExit::Success);
        }
    }
}

/// Starts this same program as the game, in `mode`, with the menu's own arguments passed on.
fn start_game(mode: &str) {
    let Ok(exe) = std::env::current_exe() else {
        error!("menu: cannot find the game's own program to start it");
        return;
    };
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    args.extend(["--play".to_string(), mode.to_string()]);
    if let Err(e) = std::process::Command::new(exe).args(&args).spawn() {
        error!("menu: starting the game failed: {e}");
    }
}
