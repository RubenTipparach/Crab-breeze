//! The game's menus (openspec change `game-menus`): the title menu a plain run opens on, the pause
//! menu on Escape, the Options page and the Controls page. A menu page pauses virtual time (so the
//! fixed step, and with it the simulation and the take, stop) and frees the mouse. The pages are UI
//! on the cockpit pass, which draws last. The Controls page lists every control from the bindings
//! data (`cb_core::input::Bindings::controls`); the Options page changes `settings`.

use crate::loading::GameReady;
use crate::pilot::CockpitCamera;
use crate::settings::{Settings, save};
use crate::sim::Data;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.55, 0.95, 0.6);
const DIM: Color = Color::srgb(0.35, 0.6, 0.4);
const PANEL: Color = Color::srgba(0.01, 0.04, 0.03, 0.94);

/// Which page of the menu is open, if any.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Menu {
    /// Playing.
    #[default]
    Closed,
    /// The title menu: Free roam, City defence, Options, Controls, Quit.
    Title,
    /// Resume, Options, Controls, Quit.
    Pause,
    /// The player's settings.
    Options,
    /// Every control and its keys.
    Controls,
}

impl Menu {
    /// True while a menu page is open (the game is paused).
    pub fn is_open(self) -> bool {
        self != Menu::Closed
    }
}

/// How this run plays, chosen on the title menu or by the command line: the encounter mechs near
/// the start (free roam) or the city defence. Inserted once the run begins.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Play {
    /// The start with its encounter mechs.
    FreeRoam,
    /// The city defence mission.
    Defence,
}

/// A plain run: open the title menu once the game is ready.
#[derive(Resource)]
pub struct TitleWanted;

/// The page Back (and Escape) goes to from Options and Controls: the title or the pause menu.
#[derive(Resource, Clone, Copy)]
struct BackTo(Menu);

/// The menu's root node.
#[derive(Component)]
struct MenuRoot;

/// What a menu button does.
#[derive(Component, Clone, Copy)]
enum MenuButton {
    Resume,
    FreeRoam,
    Defence,
    Options,
    Controls,
    Back,
    Quit,
    Mouse(i32),
    Fov(i32),
    Invert,
    Shadows,
    Vsync,
    Fullscreen,
}

/// Opens, closes and draws the menu.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Menu>()
            .insert_resource(BackTo(Menu::Pause))
            .add_systems(Update, (open_title, escape, draw).chain().before(crate::sim::PresentSet::Input));
    }
}

/// A plain run opens on the title menu the moment the game is ready (the loading screen waits on the
/// simulation running, so it cannot pause before then).
fn open_title(
    mut commands: Commands,
    wanted: Option<Res<TitleWanted>>,
    ready: Res<GameReady>,
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if wanted.is_some() && ready.0 {
        commands.remove_resource::<TitleWanted>();
        set(&mut menu, Menu::Title, &mut time, &mut cursor);
    }
}

/// Escape: open the pause menu from play, step back a page, or resume from the pause menu. On the
/// title menu there is nothing to go back to.
fn escape(
    input: Res<ButtonInput<KeyCode>>,
    keys: Res<crate::input::Keys>,
    back: Res<BackTo>,
    settings: Res<Settings>,
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if !input.any_just_pressed(keys.release_mouse.iter().copied()) {
        return;
    }
    let next = match *menu {
        Menu::Closed => Menu::Pause,
        Menu::Pause => Menu::Closed,
        Menu::Title => Menu::Title,
        Menu::Options => {
            save(&settings);
            back.0
        }
        Menu::Controls => back.0,
    };
    set(&mut menu, next, &mut time, &mut cursor);
}

fn set(menu: &mut Menu, next: Menu, time: &mut Time<Virtual>, cursor: &mut CursorOptions) {
    *menu = next;
    if next.is_open() {
        time.pause();
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    } else {
        time.unpause();
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    }
}

/// Everything a button press may touch.
struct Press<'a> {
    menu: &'a mut Menu,
    back: &'a mut BackTo,
    settings: &'a mut Settings,
    time: &'a mut Time<Virtual>,
    cursor: &'a mut CursorOptions,
}

/// A menu button was pressed.
fn press(what: MenuButton, p: Press, exit: &mut MessageWriter<AppExit>, commands: &mut Commands) {
    match what {
        MenuButton::Resume => set(p.menu, Menu::Closed, p.time, p.cursor),
        // A run begins: the encounters near the start, or the city defence (one way a mission
        // begins, as `--mission defence` does before startup).
        MenuButton::FreeRoam => {
            commands.insert_resource(Play::FreeRoam);
            set(p.menu, Menu::Closed, p.time, p.cursor);
        }
        MenuButton::Defence => {
            commands.insert_resource(Play::Defence);
            commands.insert_resource(crate::defence::Defending);
            commands.init_resource::<crate::defence::DefenceRun>();
            set(p.menu, Menu::Closed, p.time, p.cursor);
        }
        MenuButton::Options | MenuButton::Controls => {
            if matches!(*p.menu, Menu::Title | Menu::Pause) {
                p.back.0 = *p.menu;
            }
            if *p.menu == Menu::Options {
                save(p.settings);
            }
            let next = if matches!(what, MenuButton::Options) {
                Menu::Options
            } else {
                Menu::Controls
            };
            set(p.menu, next, p.time, p.cursor);
        }
        MenuButton::Back => {
            if *p.menu == Menu::Options {
                save(p.settings);
            }
            set(p.menu, p.back.0, p.time, p.cursor);
        }
        MenuButton::Quit => {
            exit.write(AppExit::Success);
        }
        MenuButton::Mouse(n) => p.settings.0.nudge_mouse(n),
        MenuButton::Fov(n) => p.settings.0.nudge_fov(n),
        MenuButton::Invert => p.settings.0.invert_pitch ^= true,
        MenuButton::Shadows => p.settings.0.shadows ^= true,
        MenuButton::Vsync => p.settings.0.vsync ^= true,
        MenuButton::Fullscreen => p.settings.0.fullscreen ^= true,
    }
}

fn label(s: &str, size: f32, colour: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont::from_font_size(size), TextColor(colour))
}

fn button_sized(parent: &mut ChildSpawnerCommands, text: &str, what: MenuButton, width: f32) {
    parent
        .spawn((
            what,
            Node {
                width: px(width),
                padding: UiRect::axes(px(16.0), px(10.0)),
                border: UiRect::all(px(2.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(DIM),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ))
        .observe(
            move |_: On<PointerClick>,
                  mut commands: Commands,
                  mut menu: ResMut<Menu>,
                  mut back: ResMut<BackTo>,
                  mut settings: ResMut<Settings>,
                  mut time: ResMut<Time<Virtual>>,
                  mut cursor: Single<&mut CursorOptions>,
                  mut exit: MessageWriter<AppExit>| {
                press(
                    what,
                    Press {
                        menu: &mut menu,
                        back: &mut back,
                        settings: &mut settings,
                        time: &mut time,
                        cursor: &mut cursor,
                    },
                    &mut exit,
                    &mut commands,
                );
            },
        )
        .with_children(|b| {
            b.spawn(label(text, 28.0, INK));
        });
}

fn button(parent: &mut ChildSpawnerCommands, text: &str, what: MenuButton) {
    button_sized(parent, text, what, 320.0);
}

/// The row of page buttons along the top of the pause menu's pages, so any page is one click from
/// any other (owner, 2026-10-25, survey J1: "ESC inside the mech will bring up the menu with buttons
/// caan switch to other menus").
fn tabs(p: &mut ChildSpawnerCommands, back: Menu) {
    p.spawn(Node {
        column_gap: px(12.0),
        ..default()
    })
    .with_children(|row| {
        let first = if back == Menu::Title {
            ("Title", MenuButton::Back)
        } else {
            ("Resume", MenuButton::Resume)
        };
        for (t, b) in [
            first,
            ("Options", MenuButton::Options),
            ("Controls", MenuButton::Controls),
            ("Quit", MenuButton::Quit),
        ] {
            button_sized(row, t, b, 180.0);
        }
    });
}

/// One row of the Options page: its name, its value, and its buttons.
fn option_row(p: &mut ChildSpawnerCommands, name: &str, value: String, buttons: &[(&str, MenuButton)]) {
    p.spawn(Node {
        column_gap: px(16.0),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Node {
                width: px(260.0),
                ..default()
            },
            label(name, 24.0, DIM),
        ));
        row.spawn((
            Node {
                width: px(150.0),
                ..default()
            },
            label(&value, 24.0, INK),
        ));
        for (t, b) in buttons {
            button_sized(row, t, *b, 90.0);
        }
    });
}

/// Rebuilds the menu when its page or the settings change.
fn draw(
    mut commands: Commands,
    menu: Res<Menu>,
    back: Res<BackTo>,
    settings: Res<Settings>,
    data: Res<Data>,
    roots: Query<Entity, With<MenuRoot>>,
    overlay: Query<Entity, With<CockpitCamera>>,
) {
    if !menu.is_changed() && !(settings.is_changed() && *menu == Menu::Options) {
        return;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    if !menu.is_open() {
        return;
    }
    let mut root = commands.spawn((
        MenuRoot,
        Node {
            width: percent(100.0),
            height: percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        // The title sits over the live world, dimmed less than the pause menu so the world shows.
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, if *menu == Menu::Title { 0.3 } else { 0.55 })),
        GlobalZIndex(1000),
    ));
    if let Some(cam) = overlay.iter().next() {
        root.insert(UiTargetCamera(cam));
    }
    let s = settings.0;
    let on = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
    root.with_children(|r| {
        r.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(14.0),
                padding: UiRect::all(px(32.0)),
                border: UiRect::all(px(2.0)),
                max_height: percent(92.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(DIM),
            BackgroundColor(PANEL),
        ))
        .with_children(|p| match *menu {
            Menu::Title => {
                p.spawn(label("CRAB BREEZE", 64.0, INK));
                button(p, "Free roam", MenuButton::FreeRoam);
                button(p, "City defence", MenuButton::Defence);
                button(p, "Options", MenuButton::Options);
                button(p, "Controls", MenuButton::Controls);
                button(p, "Quit", MenuButton::Quit);
            }
            Menu::Pause => {
                p.spawn(label("PAUSED", 44.0, INK));
                button(p, "Resume", MenuButton::Resume);
                button(p, "Options", MenuButton::Options);
                button(p, "Controls", MenuButton::Controls);
                button(p, "Quit", MenuButton::Quit);
            }
            Menu::Options => {
                tabs(p, back.0);
                p.spawn(label("OPTIONS", 40.0, INK));
                option_row(
                    p,
                    "Mouse sensitivity",
                    format!("{:.2}", s.mouse_deg_per_px),
                    &[("-", MenuButton::Mouse(-1)), ("+", MenuButton::Mouse(1))],
                );
                option_row(p, "Invert mouse", on(s.invert_pitch), &[("Flip", MenuButton::Invert)]);
                option_row(
                    p,
                    "Field of view",
                    format!("{:.0} deg", s.fov_deg),
                    &[("-", MenuButton::Fov(-1)), ("+", MenuButton::Fov(1))],
                );
                option_row(p, "Shadows", on(s.shadows), &[("Flip", MenuButton::Shadows)]);
                option_row(p, "Vsync", on(s.vsync), &[("Flip", MenuButton::Vsync)]);
                option_row(p, "Fullscreen", on(s.fullscreen), &[("Flip", MenuButton::Fullscreen)]);
                button(p, "Back", MenuButton::Back);
            }
            Menu::Controls => {
                tabs(p, back.0);
                p.spawn(label("CONTROLS", 40.0, INK));
                // Two columns of groups, so the whole table fits a 1080 p screen.
                p.spawn(Node {
                    column_gap: px(48.0),
                    ..default()
                })
                .with_children(|cols| {
                    let controls = data.0.bindings.controls();
                    let groups: Vec<&str> = controls.iter().fold(Vec::new(), |mut g, c| {
                        if !g.contains(&c.group) {
                            g.push(c.group);
                        }
                        g
                    });
                    let half = groups.len().div_ceil(2);
                    for column in groups.chunks(half.max(1)) {
                        cols.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4.0),
                            ..default()
                        })
                        .with_children(|col| {
                            for g in column {
                                col.spawn((
                                    Node {
                                        margin: UiRect::top(px(10.0)),
                                        ..default()
                                    },
                                    label(g, 24.0, INK),
                                ));
                                for c in controls.iter().filter(|c| c.group == *g) {
                                    col.spawn(Node {
                                        column_gap: px(16.0),
                                        ..default()
                                    })
                                    .with_children(|row| {
                                        row.spawn((
                                            Node {
                                                width: px(150.0),
                                                ..default()
                                            },
                                            label(&c.keys, 18.0, INK),
                                        ));
                                        row.spawn(label(c.name, 18.0, DIM));
                                    });
                                }
                            }
                        });
                    }
                });
                button(p, "Back", MenuButton::Back);
            }
            Menu::Closed => {}
        });
    });
}
