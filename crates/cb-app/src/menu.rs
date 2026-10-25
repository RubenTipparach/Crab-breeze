//! The pause menu (openspec change `game-menus`): Escape pauses the game (virtual time stops, so the
//! fixed step and the simulation stop) and opens a menu on the cockpit pass, the camera that draws
//! last; Escape inside it goes back a page, and from the first page resumes. The Controls page lists
//! every control from the bindings data (`cb_core::input::Bindings::controls`).

use crate::pilot::CockpitCamera;
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
    /// Resume, Controls, Quit.
    Pause,
    /// Every control and its keys.
    Controls,
}

impl Menu {
    /// True while a menu page is open (the game is paused).
    pub fn is_open(self) -> bool {
        self != Menu::Closed
    }
}

/// The menu's root node.
#[derive(Component)]
struct MenuRoot;

/// What a menu button does.
#[derive(Component, Clone, Copy)]
enum MenuButton {
    Resume,
    Controls,
    Back,
    Quit,
}

/// Opens, closes and draws the menu.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Menu>()
            .add_systems(Update, (escape, draw).chain().before(crate::sim::PresentSet::Input));
    }
}

/// Escape: open the menu from play, go back a page, or resume from the first page.
fn escape(
    input: Res<ButtonInput<KeyCode>>,
    keys: Res<crate::input::Keys>,
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
        Menu::Controls => Menu::Pause,
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

/// A menu button was pressed.
fn press(what: MenuButton, menu: &mut Menu, time: &mut Time<Virtual>, cursor: &mut CursorOptions, exit: &mut MessageWriter<AppExit>) {
    match what {
        MenuButton::Resume => set(menu, Menu::Closed, time, cursor),
        MenuButton::Controls => set(menu, Menu::Controls, time, cursor),
        MenuButton::Back => set(menu, Menu::Pause, time, cursor),
        MenuButton::Quit => {
            exit.write(AppExit::Success);
        }
    }
}

fn label(s: &str, size: f32, colour: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont::from_font_size(size), TextColor(colour))
}

fn button(parent: &mut ChildSpawnerCommands, text: &str, what: MenuButton) {
    parent
        .spawn((
            what,
            Node {
                width: px(320.0),
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
                  mut menu: ResMut<Menu>,
                  mut time: ResMut<Time<Virtual>>,
                  mut cursor: Single<&mut CursorOptions>,
                  mut exit: MessageWriter<AppExit>| {
                press(what, &mut menu, &mut time, &mut cursor, &mut exit);
            },
        )
        .with_children(|b| {
            b.spawn(label(text, 28.0, INK));
        });
}

/// Rebuilds the menu when its page changes.
fn draw(
    mut commands: Commands,
    menu: Res<Menu>,
    data: Res<Data>,
    roots: Query<Entity, With<MenuRoot>>,
    overlay: Query<Entity, With<CockpitCamera>>,
) {
    if !menu.is_changed() {
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
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        GlobalZIndex(1000),
    ));
    if let Some(cam) = overlay.iter().next() {
        root.insert(UiTargetCamera(cam));
    }
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
            Menu::Pause => {
                p.spawn(label("PAUSED", 44.0, INK));
                button(p, "Resume", MenuButton::Resume);
                button(p, "Controls", MenuButton::Controls);
                button(p, "Quit", MenuButton::Quit);
            }
            Menu::Controls => {
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
