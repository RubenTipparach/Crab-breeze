//! The game's menus (openspec change `game-menus`): the title menu a plain run opens on, the pause
//! menu on Escape, the Options page and the Controls page. A menu page pauses virtual time (so the
//! fixed step, and with it the simulation and the take, stop) and frees the mouse. The pages are UI
//! on the cockpit pass, which draws last. The Controls page lists every control from the bindings
//! data (`cb_core::input::Bindings::controls`); the Options page changes `settings`.

use crate::pilot::CockpitCamera;
use crate::settings::{Settings, save};
use crate::sim::Data;
use crate::state::GameApp;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.55, 0.95, 0.6);
const DIM: Color = Color::srgb(0.35, 0.6, 0.4);
const PANEL: Color = Color::srgba(0.01, 0.04, 0.03, 0.94);
const WARN: Color = Color::srgb(1.0, 0.75, 0.3);

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
    /// A hangar bay terminal's screen (air-bases): the jobs board, the shop and the refit, walked up to on foot.
    Terminal,
    /// Free roam's mech page: the roster, a pick restarting free roam in it (mech-select).
    Mech,
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

/// Which page a terminal's screen shows (air-bases, mech-shop, mech-custom).
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalTab {
    /// The career's jobs board: sign up here.
    #[default]
    Jobs,
    /// The mech shop.
    Shop,
    /// Refit the mech standing in this bay (mech-custom).
    Refit,
    /// The career's mechs, and calling one parked here out into a bay (owner, survey J16).
    Hangar,
}

/// The page Back (and Escape) goes to from Options and Controls: the title or the pause menu.
#[derive(Resource, Clone, Copy)]
struct BackTo(Menu);

/// The menu's root node.
#[derive(Component)]
struct MenuRoot;

/// What a menu button does.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(crate) enum MenuButton {
    /// A terminal screen's tab.
    Tab(TerminalTab),
    /// Buy the shop's listing at this index (mech-shop).
    Buy(usize),
    /// A refit of the mech in the bay (mech-custom).
    Refit(crate::career::Refit),
    /// Call the owned mech with this key out into a bay here (owner, survey J16).
    CallOut(u32),
    NewCareer,
    Continue,
    Mail,
    /// Free roam's Mech page.
    MechPage,
    /// Free roam again in this roster mech.
    PickMech(usize),
    Accept(u32),
    Decline(u32),
    StartMission(cb_core::missions::MissionKind),
    StopMission,
    Resume,
    FreeRoam,
    Defence,
    Options,
    Controls,
    Back,
    Quit,
    /// Free roam's time of day (owner, 2026-10-28: "in free roam there is a toggle to pick time of
    /// day, or stop changinign time of day (freeze ToD)", "this is in the pause menu"): the clock an
    /// hour on (or back).
    Hour(i32),
    /// The clock to this hour of the day.
    SetHour(u32),
    /// The clock frozen, or running again.
    FreezeClock,
}

/// A change to one setting, from the Options page of the pause menu or of the main menu: one implementation for both
/// (owner, 2026-10-28: "game needs options in main menu"). Each is saved at once, so it survives a quit or a crash
/// (owner, 2026-10-25), and takes effect at once.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tweak {
    Mouse(i32),
    Fov(i32),
    /// Sound effects volume down or up a step.
    Sfx(i32),
    /// Music volume down or up a step.
    Music(i32),
    Shake(i32),
    Invert,
    Shadows,
    Vsync,
    Fullscreen,
}

impl Tweak {
    fn apply(self, s: &mut cb_core::settings::Settings) {
        match self {
            Tweak::Mouse(n) => s.nudge_mouse(n),
            Tweak::Fov(n) => s.nudge_fov(n),
            Tweak::Sfx(n) => s.nudge_sfx(n),
            Tweak::Music(n) => s.nudge_music(n),
            Tweak::Shake(n) => s.nudge_shake(n),
            Tweak::Invert => s.invert_pitch ^= true,
            Tweak::Shadows => s.shadows ^= true,
            Tweak::Vsync => s.vsync ^= true,
            Tweak::Fullscreen => s.fullscreen ^= true,
        }
    }
}

/// A button an option row carries: a menu button, or a setting's change.
trait RowButton: Copy {
    fn spawn(self, parent: &mut ChildSpawnerCommands, text: &str, width: f32);
}

impl RowButton for MenuButton {
    fn spawn(self, parent: &mut ChildSpawnerCommands, text: &str, width: f32) {
        button_sized(parent, text, self, width);
    }
}

impl RowButton for Tweak {
    /// Needs nothing but the settings, so it works on the main menu, with no world behind it.
    fn spawn(self, parent: &mut ChildSpawnerCommands, text: &str, width: f32) {
        button_frame(parent, text, self, width).observe(move |_: On<PointerClick>, mut settings: ResMut<Settings>| {
            info!("menu: {self:?} pressed");
            self.apply(&mut settings.0);
            save(&settings);
        });
    }
}

/// The Options page's rows, on the pause menu and the main menu alike.
pub(crate) fn options_rows(p: &mut ChildSpawnerCommands, s: &cb_core::settings::Settings) {
    let on = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
    option_row(
        p,
        "Mouse sensitivity",
        format!("{:.2}", s.mouse_deg_per_px),
        &[("-", Tweak::Mouse(-1)), ("+", Tweak::Mouse(1))],
    );
    option_row(p, "Invert mouse", on(s.invert_pitch), &[("Flip", Tweak::Invert)]);
    option_row(
        p,
        "Field of view",
        format!("{:.0} deg", s.fov_deg),
        &[("-", Tweak::Fov(-1)), ("+", Tweak::Fov(1))],
    );
    option_row(
        p,
        "Sound effects",
        format!("{:.0} %", s.sfx_volume * 100.0),
        &[("-", Tweak::Sfx(-1)), ("+", Tweak::Sfx(1))],
    );
    option_row(
        p,
        "Music",
        format!("{:.0} %", s.music_volume * 100.0),
        &[("-", Tweak::Music(-1)), ("+", Tweak::Music(1))],
    );
    option_row(
        p,
        "Camera shake",
        format!("{:.0} %", s.camera_shake * 100.0),
        &[("-", Tweak::Shake(-1)), ("+", Tweak::Shake(1))],
    );
    option_row(p, "Shadows", on(s.shadows), &[("Flip", Tweak::Shadows)]);
    option_row(p, "Vsync", on(s.vsync), &[("Flip", Tweak::Vsync)]);
    option_row(p, "Fullscreen", on(s.fullscreen), &[("Flip", Tweak::Fullscreen)]);
}

/// Opens, closes and draws the menu.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Menu>()
            .init_resource::<TerminalTab>()
            .insert_resource(BackTo(Menu::Pause))
            .insert_resource(Roster(cb_core::GameData::roster(&crate::cli::repo_root().join("data"))))
            .add_game_systems(Update, (escape, draw).chain().before(crate::sim::PresentSet::Input));
    }
}

/// Escape: open the pause menu from play, step back a page, or resume from the pause menu. On the
/// title menu there is nothing to go back to.
#[allow(clippy::too_many_arguments)]
fn escape(
    mut input: ResMut<ButtonInput<KeyCode>>,
    keys: Res<crate::input::Keys>,
    back: Res<BackTo>,
    settings: Res<Settings>,
    (career, play, free): (
        Option<Res<crate::career::CareerRun>>,
        Option<Res<Play>>,
        Option<Res<crate::missions::FreeRoamRun>>,
    ),
    (pointed, mut acc): (Res<crate::mfd_apps::Pointed>, ResMut<crate::input::InputAccum>),
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    // M opens and closes the mail, in a career and in free roam (survey J1: "press M to focus on the
    // mail screen"; free roam's mail starts its missions: free-roam-missions).
    let has_mail = career.is_some() || free.is_some() || play.as_deref() == Some(&Play::FreeRoam);
    if has_mail && input.any_just_pressed(keys.mail.iter().copied()) {
        match *menu {
            Menu::Closed => set(&mut menu, Menu::Mail, &mut time, &mut cursor),
            Menu::Mail => set(&mut menu, Menu::Closed, &mut time, &mut cursor),
            _ => {}
        }
        return;
    }
    // Use again steps back from a terminal's screen (air-bases design section 5).
    if *menu == Menu::Terminal && input.any_just_pressed(keys.use_key.iter().copied()) {
        // The press is spent here, so the game does not read it again and open the terminal straight back up.
        for k in &keys.use_key {
            input.clear_just_pressed(*k);
        }
        set(&mut menu, Menu::Closed, &mut time, &mut cursor);
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
        Menu::Mail | Menu::Terminal | Menu::Mech => Menu::Closed,
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

/// Opens (or closes, with `Menu::Closed`) a menu page from outside the menu: a terminal the pilot used.
pub fn open(menu: &mut Menu, next: Menu, time: &mut Time<Virtual>, cursor: &mut CursorOptions) {
    set(menu, next, time, cursor);
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
    tab: &'a mut TerminalTab,
    /// The terminal whose screen is open, and the world (for where a bought mech lands).
    terminal: Option<cb_core::world::bases::Terminal>,
    plan: &'a cb_core::world::WorldPlan,
    bases: &'a cb_core::world::bases::BasesConfig,
    /// Where the player is and what stands near them (the air drop's search).
    near: &'a crate::air_drop::Where,
    menu: &'a mut Menu,
    back: &'a mut BackTo,
    settings: &'a mut Settings,
    time: &'a mut Time<Virtual>,
    cursor: &'a mut CursorOptions,
    /// The world's clock (`day_night`).
    clock: &'a mut crate::day_night::Clock,
}

/// The roster free roam's Mech page lists (`cb_core::GameData::roster`, mech-select), read once.
#[derive(Resource)]
struct Roster(Vec<cb_core::RosterMech>);

/// A menu button was pressed.
fn press(
    what: MenuButton,
    p: Press,
    roster: &[cb_core::RosterMech],
    career: Option<&mut crate::career::CareerRun>,
    free: Option<&mut crate::missions::FreeRoamRun>,
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
) {
    info!("menu: {what:?} pressed");
    match what {
        MenuButton::Tab(t) => {
            *p.tab = t;
            if let Some(c) = career {
                c.shop_note = None;
            }
        }
        MenuButton::Buy(i) => {
            if let Some(c) = career
                && let Some(design) = c.data.catalogue.listings.get(i).map(|l| l.design.clone())
            {
                let at = p.terminal.map(|t| t.base);
                let note = crate::career::buy_mech(c, p.plan, p.bases, &design, at, p.near);
                c.shop_note = Some(note);
            }
        }
        MenuButton::CallOut(key) => {
            if let (Some(c), Some(t)) = (career, p.terminal) {
                let wrecked = p.near.wrecks.contains(&key);
                c.shop_note = Some(crate::career::call_out(c, p.plan, p.bases, t, key, wrecked, p.near));
            }
        }
        MenuButton::Refit(r) => {
            if let Some(c) = career {
                c.shop_note = Some(crate::career::refit(c, r));
            }
        }
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
        MenuButton::MechPage => {
            p.back.0 = Menu::Pause;
            set(p.menu, Menu::Mech, p.time, p.cursor);
        }
        // The picked mech swapped in where the piloted one stands, in this program (mech-select: "In free play i can
        // select any mech from the menu and it will swap out models"; menu-in-game: "it should all be the same
        // instance, not relaunch the game"). Its data loads first; a mech that does not load changes nothing.
        MenuButton::PickMech(i) => {
            if let Some(m) = roster.get(i) {
                let (file, name) = (m.file.clone(), m.name.clone());
                commands.queue(
                    move |world: &mut World| match cb_core::GameData::load(&crate::cli::repo_root().join("data"), &file) {
                        Ok(data) => {
                            info!("menu: swapping in the {name}");
                            world.insert_resource(Data(data));
                            world.insert_resource(crate::main_menu::MechFile(file.clone()));
                            if let Some(mut saving) = world.get_resource_mut::<crate::save_game::Saving>() {
                                saving.mech = file;
                            }
                            // Where it lands near a pilot on foot: the career's air drop search (mech-shop 6.1).
                            let drop = cb_core::data::load_ron(&crate::cli::repo_root().join("data/career/air_drop.ron"))
                                .map_err(|e| warn!("menu: no drop sites, swapping in place: {e}"))
                                .ok();
                            world.insert_resource(crate::mech::SwapMech { drop });
                        }
                        Err(e) => error!("menu: the {name} does not load, keeping this mech: {e}"),
                    },
                );
                set(p.menu, Menu::Closed, p.time, p.cursor);
            }
        }
        MenuButton::Accept(id) => {
            // One job at a time: accepting abandons the one under way (free-roam-missions).
            if let Some(c) = career {
                let events = c.career.board.accept(id);
                c.pending.extend(events);
            }
        }
        // Free roam's missions: starting one turns off the one running (free-roam-missions).
        MenuButton::StartMission(kind) => {
            if let Some(f) = free {
                f.start(kind);
            }
            set(p.menu, Menu::Closed, p.time, p.cursor);
        }
        MenuButton::StopMission => {
            if let Some(f) = free {
                f.stop();
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
        MenuButton::Hour(n) => p.clock.0.shift(n as f32),
        MenuButton::SetHour(h) => p.clock.0.set_hour(h as f32),
        MenuButton::FreezeClock => p.clock.0.frozen ^= true,
    }
}

fn label(s: &str, size: f32, colour: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont::from_font_size(size), TextColor(colour))
}

/// A button's frame and label, carrying `what`; the caller says what a click does.
fn button_frame<'a>(parent: &'a mut ChildSpawnerCommands, text: &str, what: impl Component, width: f32) -> EntityCommands<'a> {
    let mut b = parent.spawn((
        what,
        crate::ui_feedback::UiButton::default(),
        Node {
            width: px(width),
            padding: UiRect::axes(px(16.0), px(10.0)),
            border: UiRect::all(px(2.0)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderColor::all(DIM),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
    ));
    b.with_children(|b| {
        b.spawn(label(text, 28.0, INK));
    });
    b
}

fn button_sized(parent: &mut ChildSpawnerCommands, text: &str, what: MenuButton, width: f32) {
    button_frame(parent, text, what, width).observe(
        move |_: On<PointerClick>,
              mut commands: Commands,
              mut menu: ResMut<Menu>,
              mut back: ResMut<BackTo>,
              mut settings: ResMut<Settings>,
              mut time: ResMut<Time<Virtual>>,
              mut cursor: Single<&mut CursorOptions>,
              mut career: Option<ResMut<crate::career::CareerRun>>,
              roster: Res<Roster>,
              mut free: Option<ResMut<crate::missions::FreeRoamRun>>,
              mut exit: MessageWriter<AppExit>,
              mut clock: ResMut<crate::day_night::Clock>,
              (mut tab, terminal, plan, data, near): (
            ResMut<TerminalTab>,
            Option<Res<crate::air_bases::AtTerminal>>,
            Res<crate::world::Plan>,
            Res<Data>,
            Res<crate::air_drop::Where>,
        )| {
            press(
                what,
                Press {
                    tab: &mut tab,
                    terminal: terminal.map(|t| t.0),
                    plan: &plan,
                    bases: &data.0.world.bases,
                    near: &near,
                    menu: &mut menu,
                    back: &mut back,
                    settings: &mut settings,
                    time: &mut time,
                    cursor: &mut cursor,
                    clock: &mut clock,
                },
                &roster.0,
                career.as_deref_mut(),
                free.as_deref_mut(),
                &mut exit,
                &mut commands,
            );
        },
    );
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
fn option_row(p: &mut ChildSpawnerCommands, name: &str, value: String, buttons: &[(&str, impl RowButton)]) {
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
            b.spawn(row, t, 90.0);
        }
    });
}

/// Rebuilds the menu when its page or the settings change.
fn draw(
    mut commands: Commands,
    menu: Res<Menu>,
    back: Res<BackTo>,
    settings: Res<Settings>,
    (data, roster): (Res<Data>, Res<Roster>),
    career: Option<Res<crate::career::CareerRun>>,
    free: Option<Res<crate::missions::FreeRoamRun>>,
    terminal: Option<Res<crate::air_bases::AtTerminal>>,
    tab: Res<TerminalTab>,
    (plan, me, near): (
        Res<crate::world::Plan>,
        Query<(&crate::mech::MechSim, &crate::mech::MechSpec), With<crate::mech::Piloted>>,
        Res<crate::air_drop::Where>,
    ),
    roots: Query<Entity, With<MenuRoot>>,
    overlay: Query<Entity, With<CockpitCamera>>,
    clock: Res<crate::day_night::Clock>,
) {
    let career_changed = career.as_ref().is_some_and(|c| c.is_changed()) || free.as_ref().is_some_and(|f| f.is_changed());
    let live = matches!(*menu, Menu::Mail | Menu::Terminal);
    let tab_changed = tab.is_changed() && *menu == Menu::Terminal;
    let clock_changed = clock.is_changed() && *menu == Menu::Pause;
    if !menu.is_changed()
        && !tab_changed
        && !clock_changed
        && !(settings.is_changed() && *menu == Menu::Options)
        && !(career_changed && live)
    {
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
                if career.is_some() || free.is_some() {
                    button(p, "Mail", MenuButton::Mail);
                }
                if free.is_some() {
                    button(p, "Mech", MenuButton::MechPage);
                    // The time of day: the hour, an hour back or on, frozen or running, and four picks.
                    let hour = clock.0.hour(&data.0.day_night);
                    let (h, m) = (hour.floor() as u32 % 24, ((hour.fract() * 60.0).floor() as u32).min(59));
                    let state = if clock.0.frozen { "FROZEN" } else { "RUNNING" };
                    option_row(
                        p,
                        "Time of day",
                        format!("{h:02}:{m:02} {state}"),
                        &[
                            ("-1h", MenuButton::Hour(-1)),
                            ("+1h", MenuButton::Hour(1)),
                            (if clock.0.frozen { "Run" } else { "Freeze" }, MenuButton::FreezeClock),
                        ],
                    );
                    p.spawn(Node {
                        column_gap: px(12.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for (name, hour) in [("Dawn", 6), ("Noon", 12), ("Dusk", 18), ("Night", 0)] {
                            button_sized(row, name, MenuButton::SetHour(hour), 110.0);
                        }
                    });
                }
                button(p, "Options", MenuButton::Options);
                button(p, "Controls", MenuButton::Controls);
                button(p, "Quit", MenuButton::Quit);
            }
            Menu::Options => {
                tabs(p, back.0);
                p.spawn(label("OPTIONS", 40.0, INK));
                options_rows(p, &s);
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
            Menu::Mech => {
                tabs(p, back.0);
                p.spawn(label("MECH", 40.0, INK));
                p.spawn(label("Free roam starts again in the mech you pick.", 24.0, DIM));
                for (i, m) in roster.0.iter().enumerate() {
                    let here = m.name == data.0.mech.name;
                    button(
                        p,
                        &if here { format!("{} (now)", m.name) } else { m.name.clone() },
                        MenuButton::PickMech(i),
                    );
                }
                button(p, "Back", MenuButton::Back);
            }
            Menu::Mail => {
                if let Some(run) = career.as_deref() {
                    mail_page(p, &run.career);
                } else if let Some(f) = free.as_deref() {
                    // Free roam's mail: every mission, started when accepted (owner, 2026-10-25: "city defense is a
                    // menu mail option in free roam", "add an email in free roam to fight the crab fortress").
                    missions_page(p, &f.free, &data.0.missions);
                }
                button(p, "Back", MenuButton::Resume);
            }
            Menu::Terminal => {
                // The terminal's screen: the career's jobs board, signed up for here (or free roam's missions).
                let title = terminal.as_deref().map_or_else(
                    || "TERMINAL".to_string(),
                    |t| match t.0.office {
                        Some(h) => format!("HARBOUR OFFICE {}", h + 1),
                        None => format!("TERMINAL   BASE {}  BAY {}", t.0.base + 1, t.0.bay + 1),
                    },
                );
                p.spawn(label(&title, 26.0, DIM));
                // A harbour office's screen is the jobs board alone (harbours).
                let office = terminal.as_deref().is_some_and(|t| t.0.office.is_some());
                if let Some(run) = career.as_deref()
                    && office
                {
                    mail_page(p, &run.career);
                } else if let Some(run) = career.as_deref() {
                    terminal_tabs(p, *tab);
                    match *tab {
                        TerminalTab::Jobs => mail_page(p, &run.career),
                        TerminalTab::Shop => shop_page(p, run),
                        TerminalTab::Hangar => hangar_page(p, run, terminal.as_deref().map(|t| t.0), &plan, &near),
                        TerminalTab::Refit => {
                            // Only the mech stopped in this terminal's bay is refitted.
                            let here = terminal.as_deref().map(|t| (t.0.base, t.0.bay));
                            let in_bay = me.iter().next().is_some_and(|(sim, spec)| {
                                sim.curr.is_stopped(&spec.locomotion)
                                    && cb_core::world::bases::hangar_at(
                                        &plan.bases,
                                        &data.0.world.bases,
                                        Vec2::new(sim.curr.pos.x, sim.curr.pos.z),
                                    ) == here
                                    && here.is_some()
                            });
                            match me.iter().next().filter(|_| in_bay) {
                                Some((_, spec)) => refit_page(p, run, spec),
                                None => {
                                    p.spawn(label("REFIT: walk your mech into this bay and stop it here.", 24.0, WARN));
                                }
                            }
                        }
                    }
                } else if let Some(f) = free.as_deref() {
                    missions_page(p, &f.free, &data.0.missions);
                }
                button(p, "Back", MenuButton::Resume);
            }
            Menu::Closed => {}
        });
    });
}

/// The row of a terminal screen's tabs, the open one marked.
fn terminal_tabs(p: &mut ChildSpawnerCommands, open: TerminalTab) {
    p.spawn(Node {
        column_gap: px(12.0),
        ..default()
    })
    .with_children(|row| {
        for (t, name) in [
            (TerminalTab::Jobs, "JOBS"),
            (TerminalTab::Shop, "SHOP"),
            (TerminalTab::Refit, "REFIT"),
            (TerminalTab::Hangar, "HANGAR"),
        ] {
            let text = if t == open { format!("[{name}]") } else { name.to_string() };
            button_sized(row, &text, MenuButton::Tab(t), 180.0);
        }
    });
}

/// The SHOP page (mech-shop): the money, then every mech for sale with the numbers the game flies it by
/// (read from its own data, CLAUDE.md 6.1), how many the career owns, and BUY with its price; then what the
/// last purchase did or why it was refused.
fn shop_page(p: &mut ChildSpawnerCommands, run: &crate::career::CareerRun) {
    let c = &run.career;
    p.spawn(label(&format!("SHOP      CREDITS {}", c.money), 40.0, INK));
    // Owner, 2026-10-26: a bought mech is air dropped near you; a bay only when there is no ground for a drop.
    p.spawn(label("DELIVERY: AIR DROP NEAR YOU", 22.0, DIM));
    if let Some(note) = &run.shop_note {
        p.spawn(label(note, 24.0, if note.starts_with("BOUGHT") { INK } else { WARN }));
    }
    for (i, l) in run.data.catalogue.listings.iter().enumerate() {
        let Some(m) = run.data.design(&l.design) else { continue };
        let owned = c.mechs.iter().filter(|o| o.design == l.design).count();
        let torso = m
            .parts
            .iter()
            .find(|p| p.kind == cb_core::mech::PartKind::Torso)
            .map_or(0.0, |p| p.armour_mm);
        let guns: Vec<&str> = m.weapons.iter().map(|w| w.name.as_str()).collect();
        p.spawn(Node {
            column_gap: px(14.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(720.0),
                    ..default()
                },
                label(
                    &format!(
                        "{}  TIER {}{}\n{:.0} T  WALK {:.0} KM/H  ARMOUR {:.0} MM  {}",
                        m.name.to_uppercase(),
                        m.tier,
                        if owned > 0 { format!("   OWNED {owned}") } else { String::new() },
                        m.locomotion.mass_kg / 1000.0,
                        m.locomotion.walk_speed_max_m_s * 3.6,
                        torso,
                        guns.join(", ").to_uppercase()
                    ),
                    18.0,
                    if owned > 0 { INK } else { DIM },
                ),
            ));
            button_sized(row, &format!("BUY {}", l.price), MenuButton::Buy(i), 220.0);
        });
    }
}

/// The HANGAR page (owner, survey J16: "you can swap mechs out at base, with foreman always availible"): every mech the
/// career owns, where it is, and CALL OUT for each one parked at this base, which brings it into a bay here to be
/// walked up to and boarded; the Foreman can always be called, from anywhere, whole again if it was wrecked. Then what
/// the last call did or why it was refused.
fn hangar_page(
    p: &mut ChildSpawnerCommands,
    run: &crate::career::CareerRun,
    terminal: Option<cb_core::world::bases::Terminal>,
    plan: &cb_core::world::WorldPlan,
    near: &crate::air_drop::Where,
) {
    let c = &run.career;
    p.spawn(label("HANGAR      YOUR MECHS", 40.0, INK));
    if let Some(note) = &run.shop_note {
        p.spawn(label(note, 24.0, if note.starts_with("CALLED") { INK } else { WARN }));
    }
    let base = terminal.and_then(|t| plan.bases.iter().find(|b| b.id == t.base));
    for m in &c.mechs {
        let name = run
            .data
            .design(&m.design)
            .map_or_else(|| m.design.clone(), |d| d.name.to_uppercase());
        let foreman = m.design == run.data.career.start_mech;
        let in_use = c.piloting == Some(m.key);
        let dropping = c.drop_of(m.key).is_some_and(|d| !d.landed);
        let here = match (m.field, base) {
            (Some(f), Some(b)) => b.rect().distance(Vec2::new(f.pos[0], f.pos[2])) <= 0.0,
            (None, Some(b)) => m.base == b.id,
            _ => false,
        };
        let wreck = near.wrecks.contains(&m.key);
        let place = match (m.field, dropping) {
            (_, true) => "AIR DROP INBOUND".to_string(),
            (Some(_), _) if here => "HERE".to_string(),
            (Some(f), _) => format!("IN THE FIELD {:.1} KM", near.pilot.distance(Vec2::new(f.pos[0], f.pos[2])) / 1000.0),
            (None, _) => format!("BASE {} BAY {}", m.base + 1, m.bay + 1),
        };
        let state = if wreck { "  WRECKED" } else { "" };
        p.spawn(Node {
            column_gap: px(14.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(720.0),
                    ..default()
                },
                label(
                    &format!("{name}{}   {place}{state}", if foreman { "  ALWAYS AVAILABLE" } else { "" }),
                    18.0,
                    if here || foreman { INK } else { DIM },
                ),
            ));
            if in_use {
                row.spawn(label("IN USE", 18.0, DIM));
            } else if (here || foreman) && !dropping && terminal.is_some() {
                button_sized(row, "CALL OUT", MenuButton::CallOut(m.key), 220.0);
            }
        });
    }
}

/// The REFIT page (mech-custom): each weapon station of the mech in the bay with what it fires (the flown mech's
/// own spec) and the spares that fit it, the weapons for sale, the armour grade, and the liveries; then what the
/// last refit did or why it was refused.
fn refit_page(p: &mut ChildSpawnerCommands, run: &crate::career::CareerRun, flown: &crate::mech::MechSpec) {
    use crate::career::Refit;
    let c = &run.career;
    let Some(owned) = crate::career::piloted(run) else { return };
    let parts = &run.data.parts;
    let stock = run.data.design(&owned.design).map(|d| d.weapons.clone()).unwrap_or_default();
    let max = run.data.catalogue.listing(&owned.design).map_or(0, |l| l.armour_max);
    p.spawn(label(
        &format!("REFIT {}      CREDITS {}", flown.name.to_uppercase(), c.money),
        34.0,
        INK,
    ));
    if let Some(note) = &run.shop_note {
        p.spawn(label(
            note,
            22.0,
            if note.contains("SHORT") || note.contains("NOT") || note.contains("NONE") || note.contains("WRONG") || note.contains("LIMIT") {
                WARN
            } else {
                INK
            },
        ));
    }
    let row = |p: &mut ChildSpawnerCommands, text: String, buttons: Vec<(String, MenuButton)>| {
        p.spawn(Node {
            column_gap: px(10.0),
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            max_width: px(1300.0),
            ..default()
        })
        .with_children(|r| {
            r.spawn((
                Node {
                    width: px(420.0),
                    ..default()
                },
                label(&text, 18.0, INK),
            ));
            for (t, b) in buttons {
                button_sized(r, &t, b, 200.0);
            }
        });
    };
    for (i, st) in stock.iter().enumerate() {
        let kind = cb_core::refit::StationKind::of(st);
        let now = flown.weapons.get(i).map_or("?", |w| w.name.as_str());
        let mut buttons = vec![("STOCK".to_string(), MenuButton::Refit(Refit::Fit(i, None)))];
        let mut seen = Vec::new();
        for id in &c.parts {
            if seen.contains(id) {
                continue;
            }
            seen.push(id.clone());
            if let Some((pi, w)) = parts.weapons.iter().enumerate().find(|(_, w)| &w.id == id && w.kind == kind) {
                buttons.push((format!("FIT {}", w.name.to_uppercase()), MenuButton::Refit(Refit::Fit(i, Some(pi)))));
            }
        }
        let place = match (kind, st.arm) {
            (cb_core::refit::StationKind::Missile, _) => "POD",
            (_, cb_core::mech::PartKind::ArmL) => "ARM L",
            _ => "ARM R",
        };
        row(p, format!("STATION {} {place}: {}", i + 1, now.to_uppercase()), buttons);
    }
    let spares: Vec<String> = c
        .parts
        .iter()
        .filter_map(|id| parts.weapon(id).map(|w| w.name.to_uppercase()))
        .collect();
    p.spawn(label(
        &format!("SPARES: {}", if spares.is_empty() { "NONE".to_string() } else { spares.join(", ") }),
        18.0,
        DIM,
    ));
    let buy: Vec<(String, MenuButton)> = parts
        .weapons
        .iter()
        .enumerate()
        .map(|(i, w)| {
            (
                format!("{} {}", w.name.to_uppercase(), w.price),
                MenuButton::Refit(Refit::BuyPart(i)),
            )
        })
        .collect();
    row(p, "BUY A WEAPON".to_string(), buy);
    let next = parts.grade(owned.armour + 1).filter(|_| owned.armour < max);
    row(
        p,
        format!(
            "ARMOUR GRADE {} OF {max}   {:.0} T",
            owned.armour,
            flown.locomotion.mass_kg / 1000.0
        ),
        next.map(|g| vec![(format!("RAISE {}", g.price), MenuButton::Refit(Refit::Armour))])
            .unwrap_or_default(),
    );
    let paint: Vec<(String, MenuButton)> = parts
        .liveries
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mark = if i as u32 == owned.livery { "*" } else { "" };
            (
                format!("{mark}{}", l.name.to_uppercase()),
                MenuButton::Refit(Refit::Livery(i as u32)),
            )
        })
        .collect();
    row(p, "LIVERY (FREE)".to_string(), paint);
}

/// The MAIL page: the credits, the job under way, every open offer with Accept and Decline, and the
/// newest messages.
fn mail_page(p: &mut ChildSpawnerCommands, c: &cb_core::career::Career) {
    use cb_core::jobs::JobState;
    p.spawn(label(&format!("MAIL      CREDITS {}", c.money), 40.0, INK));
    // Standing with each base the pilot has worked for (career-progression).
    if !c.standing.is_empty() {
        let s: Vec<String> = c.standing.iter().map(|(b, v)| format!("BASE {} {v}", b + 1)).collect();
        p.spawn(label(&format!("STANDING  {}", s.join("   ")), 22.0, DIM));
    }
    let what = |k: cb_core::jobs::JobKind| match k {
        cb_core::jobs::JobKind::Defend => "Defend",
        cb_core::jobs::JobKind::Intercept => "Intercept",
        cb_core::jobs::JobKind::Destroy => "Destroy",
        cb_core::jobs::JobKind::Escort => "Escort",
        cb_core::jobs::JobKind::Raid => "Sink the gunboats",
        cb_core::jobs::JobKind::Blockade => "Clear the blockade",
    };
    // A shipping job's enemies are gunboats (harbours).
    let foes = |k: cb_core::jobs::JobKind| if k.at_sea() { "gunboats" } else { "mechs" };
    match c.board.current() {
        Some(j) => {
            let m = (j.clock_s / 60.0).floor();
            let s = j.clock_s - m * 60.0;
            let state = if j.state == JobState::Active { "UNDER WAY" } else { "ACCEPTED" };
            p.spawn(label(
                &format!(
                    "{state}: TIER {} {} City {}, {} {}, {} credits, {m:.0}:{s:02.0} left",
                    j.tier,
                    what(j.kind),
                    j.city + 1,
                    j.mechs,
                    foes(j.kind),
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
                        "{subject}\nTIER {}  {} City {}, {} {}, {} credits, base {} standing {}",
                        j.tier,
                        what(j.kind),
                        j.city + 1,
                        j.mechs,
                        foes(j.kind),
                        j.pay,
                        j.base + 1,
                        c.standing_at(j.base)
                    ),
                    20.0,
                    INK,
                ),
            ));
            // accepting while a job is under way abandons that one (one at a time)
            button_sized(row, "Accept", MenuButton::Accept(j.id), 130.0);
            button_sized(row, "Decline", MenuButton::Decline(j.id), 130.0);
        });
    }
    for m in c
        .board
        .inbox
        .iter()
        .rev()
        .filter(|m| m.job.is_none() || m.from == "Accounts")
        .filter(|m| !m.subject.is_empty())
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

/// Free roam's MAIL page (free-roam-missions): one message per mission, each with Start, or Stop for the one
/// running. Starting one turns off the one running.
fn missions_page(p: &mut ChildSpawnerCommands, f: &cb_core::missions::FreeRoam, mail: &cb_core::missions::MissionsMail) {
    p.spawn(label("MAIL      FREE ROAM", 40.0, INK));
    p.spawn(label(
        match f.active {
            Some(_) => "One mission runs at a time: starting another turns this one off.",
            None => "No mission running. Start one below.",
        },
        22.0,
        DIM,
    ));
    for (m, msg) in mail.mail.iter().zip(&f.inbox).rev() {
        let running = f.active == Some(m.mission);
        p.spawn(Node {
            column_gap: px(14.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(760.0),
                    ..default()
                },
                label(
                    &format!(
                        "{}{}: {}\n{}",
                        if running { "RUNNING   " } else { "" },
                        msg.from,
                        msg.subject,
                        msg.body
                    ),
                    18.0,
                    if running { INK } else { DIM },
                ),
            ));
            if running {
                button_sized(row, "Stop", MenuButton::StopMission, 130.0);
            } else {
                button_sized(row, "Start", MenuButton::StartMission(m.mission), 130.0);
            }
        });
    }
}
