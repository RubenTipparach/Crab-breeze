//! The game's menus (openspec change `game-menus`): the title menu a plain run opens on, the pause
//! menu on Escape, the Options page and the Controls page. A menu page pauses virtual time (so the
//! fixed step, and with it the simulation and the take, stop) and frees the mouse. The pages are UI
//! on the cockpit pass, which draws last. The Controls page lists every control from the bindings
//! data (`cb_core::input::Bindings::controls`); the Options page changes `settings`.

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
    /// The mail: the job offers to accept or decline, the current job, the messages (a career).
    Mail,
}

impl Menu {
    /// True while a menu page is open (the game is paused).
    pub fn is_open(self) -> bool {
        self != Menu::Closed
    }
}

/// The encounter mechs near the start are wanted (`data/world/encounters.ron`): only a scripted take
/// asks for them. Free roam as a player plays it starts with no enemies (owner, 2026-10-25: "no
/// enemies in free roam!", "its CALLED FREE ROAM", "why are there enemeies at spawn!").
#[derive(Resource)]
pub struct Encounters;

/// How this run plays, chosen on the title menu or by the command line: the encounter mechs near
/// the start (free roam) or the city defence. Inserted once the run begins.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Play {
    /// Free roam: the city defence offered by mail (the menu's MAIL page and the cockpit's), the encounter
    /// mechs only for a scripted take (`Encounters`).
    FreeRoam,
    /// The city defence mission.
    Defence,
    /// A career: jobs by mail (career, mission-jobs).
    Career,
}

/// Free roam's mail offering the city defence, on the menu's MAIL page and the cockpit's (`mfd_apps`).
pub const DEFENCE_OFFER: &str = "City defence: three waves of raiders are coming for the cities. Stop them.";
/// The same, once accepted.
pub const DEFENCE_UNDER_WAY: &str = "UNDER WAY: City defence. Raiders are coming for the cities.";

/// Starts the city defence: what accepting free roam's mail does, from the menu's MAIL page or the cockpit's,
/// and the title's City defence button.
pub fn start_defence(commands: &mut Commands) {
    commands.insert_resource(Play::Defence);
    commands.insert_resource(crate::defence::Defending);
    commands.init_resource::<crate::defence::DefenceRun>();
}

/// The page Back (and Escape) goes to from Options and Controls: the title or the pause menu.
#[derive(Resource, Clone, Copy)]
struct BackTo(Menu);

/// The menu's root node.
#[derive(Component)]
struct MenuRoot;

/// What a menu button does.
#[derive(Component, Clone, Copy)]
enum MenuButton {
    NewCareer,
    Continue,
    Mail,
    Accept(u32),
    Decline(u32),
    Resume,
    FreeRoam,
    Defence,
    Options,
    Controls,
    Back,
    Quit,
    Mouse(i32),
    Fov(i32),
    /// Sound effects volume down or up a step.
    Sfx(i32),
    /// Music volume down or up a step.
    Music(i32),
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
            .add_systems(Update, (escape, draw).chain().before(crate::sim::PresentSet::Input));
    }
}

/// Escape: open the pause menu from play, step back a page, or resume from the pause menu. On the
/// title menu there is nothing to go back to.
#[allow(clippy::too_many_arguments)]
fn escape(
    input: Res<ButtonInput<KeyCode>>,
    keys: Res<crate::input::Keys>,
    back: Res<BackTo>,
    settings: Res<Settings>,
    (career, play): (Option<Res<crate::career::CareerRun>>, Option<Res<Play>>),
    (pointed, mut acc): (Res<crate::mfd_apps::Pointed>, ResMut<crate::input::InputAccum>),
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    // M opens and closes the mail, in a career and in free roam (survey J1: "press M to focus on the
    // mail screen"; free roam's mail offers the city defence).
    let has_mail = career.is_some() || play.as_deref() == Some(&Play::FreeRoam);
    if has_mail && input.any_just_pressed(keys.mail.iter().copied()) {
        match *menu {
            Menu::Closed => set(&mut menu, Menu::Mail, &mut time, &mut cursor),
            Menu::Mail => set(&mut menu, Menu::Closed, &mut time, &mut cursor),
            _ => {}
        }
        return;
    }
    if !input.any_just_pressed(keys.release_mouse.iter().copied()) {
        return;
    }
    // In play, pointing at an MFD's MENU or MAIL in free look, Escape is that screen's BACK (mfd-menus), not the
    // pause menu.
    if *menu == Menu::Closed && pointed.page.is_some_and(|p| p >= crate::mfd_apps::MENU_PAGE) {
        acc.mfd_back = true;
        return;
    }
    let next = match *menu {
        Menu::Mail => Menu::Closed,
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
fn press(
    what: MenuButton,
    p: Press,
    career: Option<&mut crate::career::CareerRun>,
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
) {
    match what {
        MenuButton::NewCareer | MenuButton::Continue => {
            commands.insert_resource(Play::Career);
            commands.insert_resource(if matches!(what, MenuButton::NewCareer) {
                crate::career::CareerStart::New
            } else {
                crate::career::CareerStart::Continue
            });
            set(p.menu, Menu::Closed, p.time, p.cursor);
        }
        MenuButton::Mail => {
            if matches!(*p.menu, Menu::Title | Menu::Pause) {
                p.back.0 = *p.menu;
            }
            set(p.menu, Menu::Mail, p.time, p.cursor);
        }
        MenuButton::Accept(id) => {
            if let Some(c) = career {
                c.career.board.accept(id);
            }
        }
        MenuButton::Decline(id) => {
            if let Some(c) = career {
                c.career.board.decline(id);
            }
        }
        MenuButton::Resume => set(p.menu, Menu::Closed, p.time, p.cursor),
        // A run begins: the encounters near the start, or the city defence (one way a mission
        // begins, as `--mission defence` does before startup).
        MenuButton::FreeRoam => {
            commands.insert_resource(Play::FreeRoam);
            set(p.menu, Menu::Closed, p.time, p.cursor);
        }
        MenuButton::Defence => {
            start_defence(commands);
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
        // Every change is saved at once, so it survives a quit or a crash (owner, 2026-10-25).
        MenuButton::Mouse(n) => changed(p, |s| s.nudge_mouse(n)),
        MenuButton::Fov(n) => changed(p, |s| s.nudge_fov(n)),
        MenuButton::Sfx(n) => changed(p, |s| s.nudge_sfx(n)),
        MenuButton::Music(n) => changed(p, |s| s.nudge_music(n)),
        MenuButton::Invert => changed(p, |s| s.invert_pitch ^= true),
        MenuButton::Shadows => changed(p, |s| s.shadows ^= true),
        MenuButton::Vsync => changed(p, |s| s.vsync ^= true),
        MenuButton::Fullscreen => changed(p, |s| s.fullscreen ^= true),
    }
}

/// Changes the settings with `f` and saves them.
fn changed(p: Press, f: impl FnOnce(&mut cb_core::settings::Settings)) {
    f(&mut p.settings.0);
    save(p.settings);
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
                  mut career: Option<ResMut<crate::career::CareerRun>>,
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
                    career.as_deref_mut(),
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
    career: Option<Res<crate::career::CareerRun>>,
    (play, defending): (Option<Res<Play>>, Option<Res<crate::defence::Defending>>),
    roots: Query<Entity, With<MenuRoot>>,
    overlay: Query<Entity, With<CockpitCamera>>,
) {
    let career_changed = career.as_ref().is_some_and(|c| c.is_changed());
    let free_roam = play.as_deref() == Some(&Play::FreeRoam);
    if !menu.is_changed() && !(settings.is_changed() && *menu == Menu::Options) && !(career_changed && *menu == Menu::Mail) {
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
                if crate::career::can_continue() {
                    button(p, "Continue", MenuButton::Continue);
                }
                button(p, "New career", MenuButton::NewCareer);
                button(p, "Free roam", MenuButton::FreeRoam);
                button(p, "City defence", MenuButton::Defence);
                button(p, "Options", MenuButton::Options);
                button(p, "Controls", MenuButton::Controls);
                button(p, "Quit", MenuButton::Quit);
            }
            Menu::Pause => {
                p.spawn(label("PAUSED", 44.0, INK));
                button(p, "Resume", MenuButton::Resume);
                if career.is_some() || free_roam {
                    button(p, "Mail", MenuButton::Mail);
                }
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
                option_row(
                    p,
                    "Sound effects",
                    format!("{:.0} %", s.sfx_volume * 100.0),
                    &[("-", MenuButton::Sfx(-1)), ("+", MenuButton::Sfx(1))],
                );
                option_row(
                    p,
                    "Music",
                    format!("{:.0} %", s.music_volume * 100.0),
                    &[("-", MenuButton::Music(-1)), ("+", MenuButton::Music(1))],
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
            Menu::Mail => {
                if let Some(run) = career.as_deref() {
                    mail_page(p, &run.career);
                } else if free_roam {
                    // Free roam's mail: the city defence, started when accepted (owner, 2026-10-25: "city defense
                    // is a menu mail option in free roam").
                    p.spawn(label("MAIL", 40.0, INK));
                    if defending.is_some() {
                        p.spawn(label(DEFENCE_UNDER_WAY, 24.0, INK));
                    } else {
                        p.spawn(label(DEFENCE_OFFER, 24.0, INK));
                        button(p, "Accept", MenuButton::Defence);
                    }
                }
                button(p, "Back", MenuButton::Resume);
            }
            Menu::Closed => {}
        });
    });
}

/// The MAIL page: the credits, the job under way, every open offer with Accept and Decline, and the
/// newest messages.
fn mail_page(p: &mut ChildSpawnerCommands, c: &cb_core::career::Career) {
    use cb_core::jobs::JobState;
    p.spawn(label(&format!("MAIL      CREDITS {}", c.money), 40.0, INK));
    let what = |k: cb_core::jobs::JobKind| match k {
        cb_core::jobs::JobKind::Defend => "Defend",
        cb_core::jobs::JobKind::Intercept => "Intercept",
        cb_core::jobs::JobKind::Destroy => "Destroy",
    };
    match c.board.current() {
        Some(j) => {
            let m = (j.clock_s / 60.0).floor();
            let s = j.clock_s - m * 60.0;
            let state = if j.state == JobState::Active { "UNDER WAY" } else { "ACCEPTED" };
            p.spawn(label(
                &format!(
                    "{state}: {} City {}, {} mechs, {} credits, {m:.0}:{s:02.0} left",
                    what(j.kind),
                    j.city + 1,
                    j.mechs,
                    j.pay
                ),
                24.0,
                INK,
            ));
        }
        None => {
            p.spawn(label("No job under way: accept one below.", 24.0, DIM));
        }
    }
    let offers: Vec<_> = c.board.jobs.iter().filter(|j| j.state == JobState::Offered).collect();
    if offers.is_empty() {
        p.spawn(label("No offers yet. They arrive by mail every minute or so.", 22.0, DIM));
    }
    for j in offers {
        let subject = c
            .board
            .inbox
            .iter()
            .rev()
            .find(|m| m.job == Some(j.id))
            .map_or_else(String::new, |m| format!("{}: {}", m.from, m.subject));
        p.spawn(Node {
            column_gap: px(14.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(620.0),
                    ..default()
                },
                label(
                    &format!(
                        "{subject}\n{} City {}, {} mechs, {} credits",
                        what(j.kind),
                        j.city + 1,
                        j.mechs,
                        j.pay
                    ),
                    20.0,
                    INK,
                ),
            ));
            if c.board.current().is_none() {
                button_sized(row, "Accept", MenuButton::Accept(j.id), 130.0);
            }
            button_sized(row, "Decline", MenuButton::Decline(j.id), 130.0);
        });
    }
    for m in c
        .board
        .inbox
        .iter()
        .rev()
        .filter(|m| m.job.is_none() || m.from == "Accounts")
        .take(3)
    {
        p.spawn((
            Node {
                max_width: px(900.0),
                ..default()
            },
            label(&format!("{}: {}. {}", m.from, m.subject, m.body), 18.0, DIM),
        ));
    }
}
