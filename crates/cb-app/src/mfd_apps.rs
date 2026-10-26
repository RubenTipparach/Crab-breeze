//! The cockpit MFD's own pages (openspec changes `mfd-menus` and `mission-jobs` 2.1): MENU, the list a screen shows
//! when it is clicked in free look, and MAIL, the job offers with ACCEPT, read and answered on the screens. They sit
//! on the MFD texture after the five readouts (`hud::MFD_PAGES`). Every row and button is a rectangle in page pixels
//! from one table (`items`), which both lays the page out and answers which row a point on the page falls in, so the
//! row that lights is the row a click presses. `mfd` finds the screen and the point; this module says what is there
//! and what pressing it does.

use crate::career::CareerRun;
use crate::hud::{AMBER, FRAME, GREEN, MFD_PAGE_PX, MFD_READOUTS, MfdRoot, text};
use bevy::prelude::*;
use cb_core::jobs::{Board, JobState};
use cb_core::missions::FreeRoam;

/// The MENU page's index on the MFD texture.
pub const MENU_PAGE: usize = MFD_READOUTS;
/// The MAIL page's index.
pub const MAIL_PAGE: usize = MFD_READOUTS + 1;

/// The readouts' names, in page order.
const READOUTS: [&str; MFD_READOUTS] = ["MECH", "FLIGHT", "MISSION", "WEAPONS", "TARGET"];

/// Offers shown on the MAIL page at once (the board keeps three open: `data/jobs/jobs.ron`).
const MAIL_SLOTS: usize = 3;

/// A row or button on an app page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// BACK: the screen goes back to the page it showed before its menu was opened.
    Back,
    /// A readout row on the MENU (its page).
    Readout(usize),
    /// The MAIL row on the MENU.
    Mail,
    /// ACCEPT on the MAIL page's offer slot.
    Accept(usize),
    /// DECLINE on the MAIL page's offer slot.
    Decline(usize),
}

/// A rectangle on a page, pixels from the page's top left.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Area {
    const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn holds(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
}

/// The top of a page that every screen shows: the dash's glare shield hides the top 15 % of the dash's screens from
/// the seat, so nothing that is read or pressed sits above it.
const TOP: f32 = 80.0;

const BACK: Area = Area::new(372.0, TOP, 118.0, 52.0);

/// Every row and button on `page`, where it is: the one table the page is drawn from and pressed against.
fn items(page: usize) -> Vec<(Item, Area)> {
    let mut out = vec![(Item::Back, BACK)];
    match page {
        MENU_PAGE => {
            for (i, _) in READOUTS.iter().enumerate() {
                let (col, row) = (i % 2, i / 2);
                out.push((
                    Item::Readout(i),
                    Area::new(22.0 + col as f32 * 238.0, TOP + 66.0 + row as f32 * 62.0, 230.0, 54.0),
                ));
            }
            out.push((Item::Mail, Area::new(22.0, TOP + 262.0, 468.0, 64.0)));
        }
        MAIL_PAGE => {
            for k in 0..MAIL_SLOTS {
                let y = TOP + 124.0 + k as f32 * 96.0;
                out.push((Item::Accept(k), Area::new(372.0, y, 118.0, 40.0)));
                out.push((Item::Decline(k), Area::new(372.0, y + 46.0, 118.0, 40.0)));
            }
        }
        _ => out.clear(),
    }
    out
}

/// The middle of `item` on `page`, page pixels: where a scripted pilot aims to press it.
pub fn item_centre(page: usize, item: Item) -> Option<Vec2> {
    items(page)
        .into_iter()
        .find(|(i, _)| *i == item)
        .map(|(_, a)| Vec2::new(a.x + a.w / 2.0, a.y + a.h / 2.0))
}

/// The row or button at `at` (page pixels) on `page`, if any.
pub fn item_at(page: usize, at: Vec2) -> Option<Item> {
    items(page).into_iter().find(|(_, a)| a.holds(at)).map(|(i, _)| i)
}

/// An offer on the MAIL page: a career's job, or one of free roam's mission messages (`free-roam-missions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    /// A job on the career's board, by id.
    Job(u32),
    /// A free-roam mission, by the index of its message in `data/missions/mail.ron` (and in the free-roam inbox).
    Mission(usize),
}

/// The offers the MAIL page lists: in a career the jobs still waiting for an answer, oldest first; in free roam its
/// mission messages, newest first, as the menu's MAIL page lists them.
pub fn offers(board: Option<&Board>, free: Option<&FreeRoam>) -> Vec<Offer> {
    let mut out: Vec<Offer> = match (board, free) {
        (Some(b), _) => b
            .jobs
            .iter()
            .filter(|j| j.state == JobState::Offered)
            .map(|j| Offer::Job(j.id))
            .collect(),
        (None, Some(f)) => (0..f.inbox.len()).rev().map(Offer::Mission).collect(),
        (None, None) => Vec::new(),
    };
    out.truncate(MAIL_SLOTS);
    out
}

/// A job's subject line, from the message that offered it.
fn subject(board: &Board, job: u32) -> String {
    board
        .inbox
        .iter()
        .find(|m| m.job == Some(job))
        .map(|m| m.subject.to_uppercase())
        .unwrap_or_else(|| format!("JOB {job}"))
}

/// Credits as `1 234 000`.
fn credits(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// An app page's row or button node, by page and item.
#[derive(Component)]
struct ItemNode(usize, Item);

/// A text on an app page that changes: what it shows.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Live {
    /// The MENU's MAIL row: MAIL and the count unread.
    MailRow,
    /// The MAIL page's title: the credits.
    Credits,
    /// The job under way, or how to take one.
    UnderWay,
    /// Offer slot k: its subject.
    Subject(usize),
    /// Offer slot k: where, how many, the pay (a job), or who sent it (a mission).
    Terms(usize),
    /// Offer slot k's main button: ACCEPT, START or STOP.
    Act(usize),
}

/// Where the pilot points on the MFDs this frame (`mfd::point`): the page under the view's centre and the row on it.
#[derive(Resource, Default, Clone, Copy, PartialEq, Debug)]
pub struct Pointed {
    /// The screen pointed at.
    pub screen: Option<Entity>,
    /// The page the pointed screen shows, if a screen is pointed at.
    pub page: Option<usize>,
    /// The row or button under the point.
    pub item: Option<Item>,
    /// The point on the page, pixels from its top left.
    pub at: Option<Vec2>,
}

/// The MENU and MAIL pages.
pub struct MfdAppsPlugin;

impl Plugin for MfdAppsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Pointed>()
            .add_systems(PostStartup, spawn_pages)
            .add_systems(Update, (light_items, write_live));
    }
}

/// One app page's node, after the readouts in the MFD root's row.
fn page_node(pages: &mut ChildSpawnerCommands, title: &str, page: usize, extra: impl FnOnce(&mut ChildSpawnerCommands)) {
    pages
        .spawn((
            Node {
                width: px(MFD_PAGE_PX),
                height: px(MFD_PAGE_PX),
                flex_shrink: 0.0,
                border: UiRect::all(px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(FRAME),
            BackgroundColor(Color::srgb(0.02, 0.05, 0.04)),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(22.0),
                    top: px(TOP + 8.0),
                    ..default()
                },
                text(title, 30.0),
            ));
            for (item, a) in items(page) {
                let label = match item {
                    Item::Back => "BACK".to_string(),
                    Item::Readout(i) => READOUTS[i].to_string(),
                    Item::Mail => "MAIL".to_string(),
                    Item::Accept(_) => "ACCEPT".to_string(),
                    Item::Decline(_) => "DECLINE".to_string(),
                };
                let mut e = p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(a.x),
                        top: px(a.y),
                        width: px(a.w),
                        height: px(a.h),
                        border: UiRect::all(px(2.0)),
                        padding: UiRect::horizontal(px(12.0)),
                        align_items: AlignItems::Center,
                        justify_content: if matches!(item, Item::Back | Item::Accept(_) | Item::Decline(_)) {
                            JustifyContent::Center
                        } else {
                            JustifyContent::FlexStart
                        },
                        ..default()
                    },
                    BorderColor::all(FRAME),
                    BackgroundColor(Color::NONE),
                    ItemNode(page, item),
                ));
                e.with_children(|b| {
                    if item == Item::Mail {
                        b.spawn((text(&label, 30.0), Live::MailRow));
                    } else if let Item::Accept(k) = item {
                        b.spawn((text(&label, 26.0), Live::Act(k)));
                    } else {
                        b.spawn(text(&label, if matches!(item, Item::Readout(_)) { 30.0 } else { 26.0 }));
                    }
                });
                if matches!(item, Item::Accept(_) | Item::Decline(_)) {
                    e.insert(Visibility::Hidden);
                }
            }
            extra(p);
        });
}

/// Adds MENU and MAIL to the MFD root, after the readouts `hud` spawned.
fn spawn_pages(mut commands: Commands, root: Single<Entity, With<MfdRoot>>) {
    commands.entity(*root).with_children(|pages| {
        page_node(pages, "MENU", MENU_PAGE, |p| {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(22.0),
                    top: px(TOP + 350.0),
                    ..default()
                },
                text("LOOK AND CLICK.  BACK: YOUR PAGE", 22.0),
            ));
        });
        page_node(pages, "MAIL", MAIL_PAGE, |p| {
            let at = |p: &mut ChildSpawnerCommands, x: f32, y: f32, w: f32, size: f32, live: Live, colour: Color| {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x),
                        top: px(y),
                        max_width: px(w),
                        ..default()
                    },
                    (Text::new(""), TextFont::from_font_size(size), TextColor(colour)),
                    live,
                ));
            };
            at(p, 130.0, TOP + 12.0, 230.0, 24.0, Live::Credits, GREEN);
            at(p, 22.0, TOP + 62.0, 468.0, 22.0, Live::UnderWay, AMBER);
            for k in 0..MAIL_SLOTS {
                let y = TOP + 124.0 + k as f32 * 96.0;
                at(p, 22.0, y, 340.0, 22.0, Live::Subject(k), GREEN);
                at(p, 22.0, y + 30.0, 340.0, 20.0, Live::Terms(k), FRAME.with_alpha(1.0));
            }
        });
    });
}

/// The pointed row lights amber; the others keep the page's frame colour.
fn light_items(pointed: Res<Pointed>, mut nodes: Query<(&ItemNode, &mut BorderColor, &mut BackgroundColor)>) {
    if !pointed.is_changed() {
        return;
    }
    for (ItemNode(page, item), mut border, mut bg) in &mut nodes {
        let lit = pointed.page == Some(*page) && pointed.item == Some(*item);
        let (b, f) = if lit {
            (AMBER, AMBER.with_alpha(0.18))
        } else {
            (FRAME, Color::NONE)
        };
        if border.top != b {
            *border = BorderColor::all(b);
        }
        if bg.0 != f {
            bg.0 = f;
        }
    }
}

/// The MAIL page's words and buttons and the MENU's unread count: a career's board, or free roam's missions.
fn write_live(
    career: Option<Res<CareerRun>>,
    (free, data): (Option<Res<crate::missions::FreeRoamRun>>, Res<crate::sim::Data>),
    pilot: Res<crate::pilot::PilotSim>,
    mut texts: Query<(&Live, &mut Text)>,
    mut buttons: Query<(&ItemNode, &mut Visibility)>,
) {
    let board = career.as_ref().map(|c| &c.career.board);
    let free = free.as_ref().map(|f| &f.free);
    let mail = &data.0.missions.mail;
    let open = offers(board, free);
    let here = Vec2::new(pilot.curr.feet.x, pilot.curr.feet.z);
    let running = |i: usize| free.is_some_and(|f| mail.get(i).is_some_and(|m| f.active == Some(m.mission)));
    for (live, mut t) in &mut texts {
        let s = match *live {
            Live::MailRow => {
                let unread = board.map_or(0, |b| b.inbox.iter().filter(|m| !m.read).count());
                if unread > 0 {
                    format!("MAIL   {unread} NEW")
                } else {
                    "MAIL".into()
                }
            }
            Live::Credits => career
                .as_ref()
                .map(|c| format!("CR {}", credits(c.career.money)))
                .unwrap_or_default(),
            Live::UnderWay => match (board.and_then(|b| b.current()), free) {
                (Some(j), _) => {
                    let km = j.site.distance(here) / 1000.0;
                    let what = if j.state == JobState::Active { "FIGHT ON" } else { "UNDER WAY" };
                    format!(
                        "{what}: {}\nCITY {}  {km:.1} KM TO THE WAYPOINT",
                        subject(board.unwrap(), j.id),
                        j.city + 1
                    )
                }
                (None, Some(f)) => match f.active.and_then(|k| mail.iter().position(|m| m.mission == k)) {
                    Some(i) => format!("RUNNING: {}", f.inbox.get(i).map_or(String::new(), |m| m.subject.to_uppercase())),
                    None => "NO MISSION RUNNING: START ONE".into(),
                },
                (None, None) if board.is_none() => "NO MAIL HERE".into(),
                (None, None) if open.is_empty() => "NO OFFERS YET: THEY COME BY MAIL".into(),
                (None, None) => "NO JOB UNDER WAY: ACCEPT ONE".into(),
            },
            Live::Subject(k) => match open.get(k) {
                Some(Offer::Job(id)) => board.map(|b| subject(b, *id)).unwrap_or_default(),
                Some(Offer::Mission(i)) => free
                    .and_then(|f| f.inbox.get(*i))
                    .map_or(String::new(), |m| m.subject.to_uppercase()),
                None => String::new(),
            },
            Live::Terms(k) => match open.get(k) {
                Some(Offer::Job(id)) => board
                    .and_then(|b| b.job(*id))
                    .map(|j| {
                        let km = j.site.distance(here) / 1000.0;
                        format!("CITY {}  {} MECHS  {km:.1} KM  CR {}", j.city + 1, j.mechs, credits(j.pay))
                    })
                    .unwrap_or_default(),
                Some(Offer::Mission(i)) => {
                    let from = free.and_then(|f| f.inbox.get(*i)).map_or(String::new(), |m| m.from.to_uppercase());
                    if running(*i) { format!("{from}  RUNNING") } else { from }
                }
                None => String::new(),
            },
            // The slot's main button: ACCEPT a job, START a mission, STOP the one running.
            Live::Act(k) => match open.get(k) {
                Some(Offer::Mission(i)) if running(*i) => "STOP".into(),
                Some(Offer::Mission(_)) => "START".into(),
                _ => "ACCEPT".into(),
            },
        };
        if t.0 != s {
            t.0 = s;
        }
    }
    // The slot's buttons show on a slot with an offer in it; DECLINE only on a career's job.
    for (ItemNode(_, item), mut vis) in &mut buttons {
        let shown = match item {
            Item::Accept(k) => *k < open.len(),
            Item::Decline(k) => matches!(open.get(*k), Some(Offer::Job(_))),
            _ => continue,
        };
        let v = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != v {
            *vis = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_is_on_both_app_pages_and_no_readout() {
        assert_eq!(item_at(MENU_PAGE, Vec2::new(430.0, 100.0)), Some(Item::Back));
        assert_eq!(item_at(MAIL_PAGE, Vec2::new(430.0, 100.0)), Some(Item::Back));
        assert_eq!(item_at(0, Vec2::new(430.0, 100.0)), None);
    }

    #[test]
    fn the_menu_lists_every_readout_and_mail() {
        for i in 0..MFD_READOUTS {
            let (_, a) = items(MENU_PAGE).into_iter().find(|(it, _)| *it == Item::Readout(i)).unwrap();
            assert_eq!(
                item_at(MENU_PAGE, Vec2::new(a.x + a.w / 2.0, a.y + a.h / 2.0)),
                Some(Item::Readout(i))
            );
        }
        assert_eq!(item_at(MENU_PAGE, Vec2::new(250.0, 370.0)), Some(Item::Mail));
    }

    #[test]
    fn no_two_rows_overlap_and_all_lie_on_the_page() {
        for page in [MENU_PAGE, MAIL_PAGE] {
            let all = items(page);
            for (i, (_, a)) in all.iter().enumerate() {
                assert!(a.x >= 0.0 && a.y >= 0.0 && a.x + a.w <= MFD_PAGE_PX && a.y + a.h <= MFD_PAGE_PX);
                for (_, b) in &all[i + 1..] {
                    let apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                    assert!(apart, "{a:?} overlaps {b:?} on page {page}");
                }
            }
        }
    }

    #[test]
    fn nothing_to_read_or_press_sits_under_the_dashs_glare_shield() {
        for page in [MENU_PAGE, MAIL_PAGE] {
            for (item, a) in items(page) {
                assert!(a.y >= TOP, "{item:?} at {} on page {page}", a.y);
            }
        }
    }

    #[test]
    fn free_roams_mail_lists_every_mission_newest_first() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mail: cb_core::missions::MissionsMail = cb_core::data::load_ron(&root.join("missions/mail.ron")).unwrap();
        let free = FreeRoam::new(&mail);
        let n = mail.mail.len().min(MAIL_SLOTS);
        let want: Vec<Offer> = (0..mail.mail.len()).rev().map(Offer::Mission).take(n).collect();
        assert_eq!(offers(None, Some(&free)), want);
        assert!(offers(None, None).is_empty(), "no mail outside free roam and a career");
    }

    #[test]
    fn credits_read_in_groups_of_three() {
        assert_eq!(credits(1_234_000), "1 234 000");
        assert_eq!(credits(950), "950");
        assert_eq!(credits(-45_000), "-45 000");
    }
}
