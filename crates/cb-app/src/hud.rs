//! The cockpit HUD prototype (openspec change `cockpit-hud`; owner, 2026-10-24: "quick rapid ui
//! prototyping in the cockpit"): a crosshair, a leg compass (which way the legs face against the
//! torso), booster and skate gauges (heat, seconds left, seconds to recharge), throttle, speed,
//! altitude, mode, guns, damage, for exploring the compass heading and the nearest city, and in the
//! defence mission (`city-defence`) the wave, its clock, the attacked city's damage and the raiders left.
//! Targeting (`targeting`, after Ace Combat): a box round every enemy in range with its distance, the
//! selected one marked with its range, speed and `IN RANGE` or `LOCK`, the lead marker where the main
//! gun must point (`weapons::lead`, the AI's own), and an arrow at the edge toward a target off screen. Shown only while seated. Every number is the simulation's own
//! (CLAUDE.md section 6.1). A 2D overlay for now; it moves onto the cockpit's screens later.

use crate::mech::{MechSim, MechSpec, Piloted};
use crate::pilot::PilotSim;
use crate::sim::PresentSet;
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use bevy::ui::UiTransform;
use cb_core::mech::{LocomotionMode, OnWater, PartKind};
use cb_core::pilot::PilotMode;

/// Draws the HUD.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_world_spawn(spawn_hud).add_game_systems(
            Update,
            (
                update_hud,
                draw_targets,
                draw_lock,
                exit_ring,
                update_stations,
                draw_job,
                draw_signs,
            )
                .in_set(PresentSet::Camera),
        );
    }
}

#[derive(Component)]
struct HudRoot;

/// The root of the MFD pages (cockpit-mfds).
#[derive(Component)]
pub struct MfdRoot;

/// One MFD page's side, pixels (a row a multiple of 256 bytes).
pub const MFD_PAGE_PX: f32 = 512.0;

/// Pages side by side on the MFD texture, left to right: MECH, FLIGHT, MISSION, WEAPONS, TARGET (weapon-select),
/// then the cockpit's own MENU and MAIL (`mfd_apps`, openspec change `mfd-menus`). The pages' row and the texture are
/// both this many pages wide, so each screen's slice holds exactly one page.
pub const MFD_PAGES: usize = 7;

/// The readout pages, the first of `MFD_PAGES`; the rest are `mfd_apps`'.
pub const MFD_READOUTS: usize = 5;

#[derive(Component)]
struct LegArrow;

/// One dot of the climb-out ring round the crosshair, by its place round the ring (0 at the top).
#[derive(Component)]
struct ExitDot(usize);

/// A row of the WEAPONS page: one weapon station, boxed when selected.
#[derive(Component)]
struct StationRow(usize);

/// Rows on the WEAPONS page.
const STATION_ROWS: usize = 6;

/// The accepted job's site over the screens.
#[derive(Component)]
struct JobMarker;

/// `[MELEE MODE]` under the crosshair while close combat is on.
#[derive(Component)]
struct MeleeSign;

/// The selected weapon's name under the crosshair: what the left mouse fires.
#[derive(Component)]
struct WeaponSign;

/// The job line on the MISSION page.
#[derive(Component)]
struct JobLine;

/// The TARGET page's readout.
#[derive(Component)]
struct TargetReadout;

/// Dots in the climb-out ring.
const EXIT_DOTS: usize = 24;
/// The climb-out ring's radius, pixels.
const EXIT_RING_PX: f32 = 34.0;

/// One of the two missile-lock boxes: 0 square, 1 turned 45 degrees.
#[derive(Component, Clone, Copy)]
struct LockBox(usize);

/// A pooled target box, by slot.
#[derive(Component, Clone, Copy)]
struct TargetBox(usize);

/// A target box's label, by slot.
#[derive(Component, Clone, Copy)]
struct TargetLabel(usize);

/// The selected target's lead marker.
#[derive(Component)]
struct LeadRing;

/// The arrow toward a selected target off screen.
#[derive(Component)]
struct EdgeArrow;

/// Target boxes drawn at most (the nearest).
const BOXES: usize = 32;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Gauge {
    Boost,
    Skate,
    Throttle,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Readout {
    Boost,
    Skate,
    Legs,
    Flight,
    Guns,
    Damage,
    Nav,
    Mission,
    /// The last hangar bay's repair and rearm (air-bases), on the MECH page.
    Service,
}

pub(crate) const GREEN: Color = Color::srgb(0.55, 0.95, 0.6);
pub(crate) const AMBER: Color = Color::srgb(1.0, 0.75, 0.25);
const RED: Color = Color::srgb(1.0, 0.3, 0.25);
/// A neutral (a civil aircraft: `civil-air-traffic`): never red, never amber.
const NEUTRAL: Color = Color::srgb(0.85, 0.9, 0.95);
pub(crate) const FRAME: Color = Color::srgba(0.55, 0.95, 0.6, 0.35);

pub(crate) fn text(s: &str, size: f32) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont::from_font_size(size), TextColor(GREEN))
}

/// A labelled bar: the fill's width is set each frame.
fn bar(parent: &mut ChildSpawnerCommands, gauge: Gauge, readout: Readout, label: &str) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(3.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((text(label, 34.0), readout));
            col.spawn((
                Node {
                    width: px(400.0),
                    height: px(12.0),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BorderColor::all(FRAME),
            ))
            .with_children(|frame| {
                frame.spawn((
                    Node {
                        width: percent(100.0),
                        height: percent(100.0),
                        ..default()
                    },
                    BackgroundColor(GREEN),
                    gauge,
                ));
            });
        });
}

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: percent(100.0),
                height: percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            // Target boxes, the lead marker and the edge arrow (targeting).
            for slot in 0..BOXES {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(40.0),
                        height: px(40.0),
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BorderColor::all(AMBER),
                    Visibility::Hidden,
                    TargetBox(slot),
                ))
                .with_children(|b| {
                    b.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Percent(100.0),
                            left: px(-2.0),
                            margin: UiRect::top(px(3.0)),
                            ..default()
                        },
                        text("", 13.0),
                        TextLayout::no_wrap(),
                        TargetLabel(slot),
                    ));
                });
            }
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: px(16.0),
                    height: px(16.0),
                    border: UiRect::all(px(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(GREEN),
                Visibility::Hidden,
                LeadRing,
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                text("^", 30.0),
                UiTransform::IDENTITY,
                Visibility::Hidden,
                EdgeArrow,
            ));
            // Under the crosshair: the weapon the left mouse fires, and [MELEE MODE] in close combat
            // (owner, 2026-10-25: "MELEE MODE on the main screen, not mfd").
            for (top, size, colour, melee) in [(36.0, 20.0, GREEN, false), (62.0, 28.0, AMBER, true)] {
                let mut sign = root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(0.0),
                        width: Val::Percent(100.0),
                        top: Val::Percent(50.0),
                        margin: UiRect::top(px(top)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Visibility::Hidden,
                ));
                sign.with_children(|s| {
                    s.spawn((Text::new(""), TextFont::from_font_size(size), TextColor(colour)));
                });
                if melee {
                    sign.insert(MeleeSign);
                } else {
                    sign.insert(WeaponSign);
                }
            }
            // The accepted job's site (draw_job).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                (Text::new(""), TextFont::from_font_size(22.0), TextColor(AMBER)),
                Visibility::Hidden,
                JobMarker,
            ));
            // The missile lock's two boxes (draw_lock).
            for i in 0..2 {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(40.0),
                        height: px(40.0),
                        border: UiRect::all(px(3.0)),
                        ..default()
                    },
                    BorderColor::all(RED),
                    UiTransform::IDENTITY,
                    Visibility::Hidden,
                    LockBox(i),
                ));
            }
            // The climb-out ring: fills clockwise while Use is held (owner, 2026-10-25: "there would be
            // a ccircle timer thing that shows progress of confirming E to exit").
            for i in 0..EXIT_DOTS {
                let a = i as f32 / EXIT_DOTS as f32 * std::f32::consts::TAU;
                let (x, y) = (EXIT_RING_PX * a.sin(), -EXIT_RING_PX * a.cos());
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        width: px(5.0),
                        height: px(5.0),
                        margin: UiRect {
                            left: px(x - 2.5),
                            top: px(y - 2.5),
                            ..default()
                        },
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(AMBER),
                    Visibility::Hidden,
                    ExitDot(i),
                ));
            }
            // Crosshair.
            for (w, h) in [(22.0, 2.0), (2.0, 22.0)] {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        width: px(w),
                        height: px(h),
                        margin: UiRect {
                            left: px(-w / 2.0),
                            top: px(-h / 2.0),
                            ..default()
                        },
                        ..default()
                    },
                    BackgroundColor(GREEN.with_alpha(0.8)),
                ));
            }
        });
    // The mech's readouts, on the cockpit's MFDs (cockpit-mfds, weapon-select): five 512 x 512 pages
    // in a row, drawn by the MFD camera into one texture each screen shows one page of. Every roster
    // cockpit has six screens (cockpit-parity); only a mech with no interior (the stand-in) keeps the
    // pages along the bottom of the view.
    commands
        .spawn((
            HudRoot,
            MfdRoot,
            Node {
                // One page wide per page on the texture: a narrower strip squeezes the pages and every
                // screen's slice cuts across two of them (owner, 2026-10-25: "mfds are all busted").
                width: px(MFD_PAGE_PX * MFD_PAGES as f32),
                height: px(MFD_PAGE_PX),
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|pages| {
            let page = |pages: &mut ChildSpawnerCommands, title: &str| {
                pages
                    .spawn((
                        Node {
                            width: px(MFD_PAGE_PX),
                            flex_shrink: 0.0,
                            height: px(MFD_PAGE_PX),
                            padding: UiRect::all(px(24.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: px(10.0),
                            border: UiRect::all(px(4.0)),
                            // A long readout wraps or is cut at the page's edge, never drawn over the
                            // next page.
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BorderColor::all(FRAME),
                        BackgroundColor(Color::srgb(0.02, 0.05, 0.04)),
                    ))
                    .with_children(|p| {
                        p.spawn(text(title, 26.0));
                    })
                    .id()
            };
            // MECH: the legs against the torso, and damage.
            let mech = page(pages, "MECH");
            pages.commands().entity(mech).with_children(|c| {
                c.spawn((
                    Node {
                        width: px(200.0),
                        height: px(200.0),
                        border: UiRect::all(px(2.0)),
                        border_radius: BorderRadius::MAX,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(FRAME),
                ))
                .with_children(|ring| {
                    // Torso marker at the top of the ring.
                    ring.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(-4.0),
                            width: px(14.0),
                            height: px(26.0),
                            ..default()
                        },
                        BackgroundColor(AMBER),
                    ));
                    // The legs: a bar through the centre, its head marked, turned by the twist.
                    ring.spawn((
                        Node {
                            width: px(18.0),
                            height: px(164.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        UiTransform::IDENTITY,
                        LegArrow,
                    ))
                    .with_children(|arrow| {
                        arrow.spawn((
                            Node {
                                width: px(32.0),
                                height: px(32.0),
                                border_radius: BorderRadius::all(px(3.0)),
                                ..default()
                            },
                            BackgroundColor(GREEN),
                        ));
                        arrow.spawn((
                            Node {
                                width: px(7.0),
                                height: px(132.0),
                                ..default()
                            },
                            BackgroundColor(GREEN.with_alpha(0.7)),
                        ));
                    });
                });
                c.spawn((text("LEGS", 44.0), Readout::Legs));
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    text("", 36.0),
                    Readout::Damage,
                ));
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    text("", 26.0),
                    Readout::Service,
                ));
            });
            // FLIGHT: boosters and skates, throttle, speed, altitude, mode, guns.
            let flight = page(pages, "FLIGHT");
            pages.commands().entity(flight).with_children(|c| {
                bar(c, Gauge::Boost, Readout::Boost, "BOOST");
                bar(c, Gauge::Skate, Readout::Skate, "SKATE");
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    text("", 38.0),
                    Readout::Flight,
                ));
                bar(c, Gauge::Throttle, Readout::Guns, "");
            });
            // MISSION: heading, the nearest city, the wave.
            let mission = page(pages, "MISSION");
            // WEAPONS: every station, the selected one boxed (weapon-select).
            let weapons = page(pages, "WEAPONS");
            pages.commands().entity(weapons).with_children(|c| {
                for i in 0..STATION_ROWS {
                    c.spawn((
                        Node {
                            width: percent(100.0),
                            padding: UiRect::axes(px(12.0), px(6.0)),
                            border: UiRect::all(px(3.0)),
                            ..default()
                        },
                        text("", 30.0),
                        BorderColor::all(Color::NONE),
                        Visibility::Hidden,
                        StationRow(i),
                    ));
                }
            });
            // TARGET: what is targeted, where, the lock, how hurt (weapon-select).
            let target = page(pages, "TARGET");
            pages.commands().entity(target).with_children(|c| {
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    text("", 36.0),
                    TargetReadout,
                ));
            });
            pages.commands().entity(mission).with_children(|c| {
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    text("", 36.0),
                    Readout::Nav,
                ));
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    text("", 36.0),
                    Readout::Mission,
                ));
                c.spawn((
                    Node {
                        max_width: percent(100.0),
                        ..default()
                    },
                    (Text::new(""), TextFont::from_font_size(32.0), TextColor(AMBER)),
                    JobLine,
                ));
            });
        });
}

/// The selected target's label: `850 m  12 m/s  LOCK`.
fn fmt_target(range_m: f32, speed_m_s: f32, in_range: bool, locked: bool) -> String {
    let state = if locked {
        "  LOCK"
    } else if in_range {
        "  IN RANGE"
    } else {
        ""
    };
    format!("{range_m:.0} m  {speed_m_s:.0} m/s{state}")
}

/// Every enemy in range boxed on the screen, the selected one marked with its lock and its lead
/// marker, and an arrow toward it when it is off screen. The same state the missiles home on
/// (`TargetState`) and the same lead the AI aims with (`weapons::lead`).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_targets(
    pilot: Res<PilotSim>,
    data: Res<crate::sim::Data>,
    state: Res<crate::targeting::TargetState>,
    camera: Single<(&Camera, &GlobalTransform), With<crate::pilot::PilotCamera>>,
    me: Query<&MechSpec, With<Piloted>>,
    my_guns: Query<&MechSim, With<Piloted>>,
    hostiles: Res<crate::units::Hostiles>,
    neutrals: Res<crate::air_traffic::Neutrals>,
    mut boxes: Query<(&TargetBox, &mut Node, &mut BorderColor, &mut Visibility), (Without<LeadRing>, Without<EdgeArrow>)>,
    mut labels: Query<(&TargetLabel, &mut Text, &mut TextColor), Without<EdgeArrow>>,
    mut lead: Single<(&mut Node, &mut Visibility, &mut BorderColor), (With<LeadRing>, Without<TargetBox>, Without<EdgeArrow>)>,
    mut arrow: Single<
        (&mut Node, &mut Visibility, &mut UiTransform, &mut TextColor),
        (With<EdgeArrow>, Without<TargetBox>, Without<LeadRing>),
    >,
) {
    let cfg = &data.0.targeting;
    let (cam, at) = *camera;
    let eye = at.translation();
    let seated = pilot.curr.mode == PilotMode::Seated;
    let size = cam.logical_viewport_size().unwrap_or(Vec2::new(1920.0, 1080.0));
    let on_screen = |p: Vec2| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y;
    let locked = state.locked(cfg);
    // The enemies in range, mechs and vehicles, and civil aircraft as neutrals, nearest first: the
    // ones the targeting saw through the fog this step, at the range the fog leaves (survey F2).
    let mut shown: Vec<(f32, f32, crate::units::Hostile)> = hostiles
        .0
        .iter()
        .chain(&neutrals.0)
        .filter(|_| seated)
        .filter_map(|h| state.fog_toward(h.id).map(|fog| (h.aim.distance(eye), fog, *h)))
        .filter(|(d, fog, _)| cb_core::targeting::seen(cfg, *d, *fog))
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.id.cmp(&b.2.id)));
    let mut texts: Vec<(String, Color)> = vec![(String::new(), GREEN); BOXES];
    let mut placed = vec![None; BOXES];
    let mut lead_at = None;
    let mut arrow_dir = None;
    for (slot, (d, fog, h)) in shown.iter().take(BOXES).enumerate() {
        let (id, feet, torso, vel) = (&h.id, h.feet, h.aim, h.vel);
        let selected = state.selected == Some(*id);
        let neutral = crate::air_traffic::is_civil(*id);
        let colour = if selected && locked == Some(*id) {
            RED
        } else if neutral {
            NEUTRAL
        } else if h.shelling {
            AMBER
        } else {
            RED.with_alpha(0.85)
        };
        if selected {
            let w = me.iter().next().and_then(|spec| spec.weapons.first().cloned());
            if let Some(w) = w {
                let p = cb_core::weapons::lead(eye, torso, vel, w.shot().lead_speed_m_s(w.speed_m_s), !w.shot().straight());
                lead_at = cam.world_to_viewport(at, p).ok().filter(|p| on_screen(*p));
            }
            let in_range = cb_core::targeting::in_lock_range(cfg, *d, *fog);
            texts[slot] = (fmt_target(*d, vel.length(), in_range, locked == Some(*id)), colour);
        } else if neutral {
            texts[slot] = (format!("CIV {d:.0} m"), colour);
        } else {
            texts[slot] = (format!("{d:.0} m"), colour);
        }
        // A pod's locks on it, `xN` (multi-lock), from the locks the salvo will fire on.
        let n = my_guns.iter().next().map_or(0, |m| m.guns.locks.count(*id));
        if n > 0 {
            texts[slot].0.push_str(&format!(" x{n}"));
        }
        let (Ok(top), Ok(bottom)) = (
            cam.world_to_viewport(at, feet + Vec3::Y * h.height_m),
            cam.world_to_viewport(at, feet),
        ) else {
            if selected {
                arrow_dir = Some(torso);
            }
            continue;
        };
        let centre = (top + bottom) * 0.5;
        if !on_screen(centre) {
            if selected {
                arrow_dir = Some(torso);
            }
            continue;
        }
        let px_size = (bottom.y - top.y).abs().clamp(cfg.box_px[0], cfg.box_px[1]);
        placed[slot] = Some((centre, px_size, colour, selected));
    }
    for (b, mut node, mut border, mut vis) in &mut boxes {
        match placed[b.0] {
            Some((c, w, colour, selected)) => {
                let w = if selected { w * 1.25 } else { w };
                node.left = px(c.x - w * 0.5);
                node.top = px(c.y - w * 0.5);
                node.width = px(w);
                node.height = px(w);
                node.border = UiRect::all(px(if selected { 3.0 } else { 1.5 }));
                *border = BorderColor::all(colour);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    for (l, mut t, mut c) in &mut labels {
        let (s, colour) = &texts[l.0];
        if t.0 != *s {
            t.0 = s.clone();
        }
        c.0 = *colour;
    }
    let (node, vis, border) = &mut *lead;
    match lead_at {
        Some(p) => {
            node.left = px(p.x - 8.0);
            node.top = px(p.y - 8.0);
            **border = BorderColor::all(if locked.is_some() { RED } else { GREEN });
            **vis = Visibility::Inherited;
        }
        None => **vis = Visibility::Hidden,
    }
    let (node, vis, turn, colour) = &mut *arrow;
    match arrow_dir {
        Some(torso) => {
            // The way round to it on screen: its direction in the camera's frame, x right, y down.
            let local = at.rotation().inverse() * (torso - eye);
            let dir = Vec2::new(local.x, -local.y).normalize_or(Vec2::X);
            let r = size * 0.5 - Vec2::splat(60.0);
            let p = size * 0.5 + dir * r;
            node.left = px(p.x - 10.0);
            node.top = px(p.y - 18.0);
            turn.rotation = Rot2::radians(dir.x.atan2(-dir.y));
            colour.0 = RED;
            **vis = Visibility::Inherited;
        }
        None => **vis = Visibility::Hidden,
    }
}

/// Compass bearing in degrees, 0 north (-z), clockwise, of a flat direction (x, z).
pub(crate) fn bearing_deg(dir: Vec2) -> f32 {
    dir.x.atan2(-dir.y).to_degrees().rem_euclid(360.0)
}

/// A bearing as the compass shows it, whole degrees 0..359.
pub(crate) fn whole_deg(deg: f32) -> i32 {
    (deg.round() as i32).rem_euclid(360)
}

/// The compass point for a bearing.
fn point(deg: f32) -> &'static str {
    const POINTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    POINTS[((deg / 45.0).round() as usize) % 8]
}

/// `HDG 274 W   CITY 2  1.8 km  040 NE`: where the view points and the way to the nearest city.
fn fmt_nav(view_yaw_rad: f32, at: Vec2, cities: &[cb_core::world::City]) -> String {
    // A yaw of zero looks along -z (north); yaw turns left, the compass turns right.
    let heading = bearing_deg(Vec2::new(-view_yaw_rad.sin(), -view_yaw_rad.cos()));
    let mut s = format!("HDG {:03} {}", whole_deg(heading), point(heading));
    let nearest = cities
        .iter()
        .min_by(|a, b| a.centre.distance_squared(at).total_cmp(&b.centre.distance_squared(at)));
    if let Some(c) = nearest {
        let to = c.centre - at;
        if to.abs().max_element() <= c.half_size_m {
            s += &format!("   IN CITY {}", c.id + 1);
        } else {
            let b = bearing_deg(to);
            s += &format!(
                "   CITY {}  {:.1} km  {:03} {}",
                c.id + 1,
                to.length() / 1000.0,
                whole_deg(b),
                point(b)
            );
        }
    }
    s
}

fn fmt_budget(name: &str, heat: f32, locked: bool, burn_s: f32, recharge_s: f32, active: bool) -> (String, Color) {
    let left = (1.0 - heat) * burn_s;
    let recharge = heat * recharge_s;
    // Big and minimal on the MFDs (owner, 2026-10-24): the name and one number; the bar shows the
    // rest, recharging included.
    let _ = recharge;
    if locked {
        (format!("{name} EMPTY"), RED)
    } else if active || heat > 0.0 {
        (format!("{name} {left:.0}s"), if active { AMBER } else { GREEN })
    } else {
        (format!("{name} READY"), GREEN)
    }
}

/// `STRATOSTACEAN   LIFT PORT 100 %  STBD 67 %   ALT 600 M   CITY 2 DAMAGE 4 %`, the hull thrusters once they open, `FALLING`, then
/// `DOWN: WON`: every number the lift rule's own (`crab-airship` section 9; CLAUDE.md 6.1).
fn fmt_fortress(run: &crate::fortress::FortressRun, d: &cb_core::fortress::FortressData) -> (String, Color) {
    let f = &run.f;
    match run.outcome {
        Some(true) => {
            return (
                format!(
                    "THE {} IS DOWN: MISSION WON   CITY {} DAMAGE {:.0} %",
                    d.cfg.name.to_uppercase(),
                    run.city + 1,
                    run.city_damage * 100.0
                ),
                GREEN,
            );
        }
        Some(false) => return ("MISSION LOST: your mech was destroyed".into(), RED),
        None => {}
    }
    let l = f.lift(d);
    let mut s = format!(
        "{}   LIFT PORT {:.0} %  STBD {:.0} %   ALT {:.0} M   CITY {} DAMAGE {:.0} %",
        d.cfg.name.to_uppercase(),
        l.port * 200.0,
        l.stbd * 200.0,
        f.pos.y - f.ground_m,
        run.city + 1,
        run.city_damage * 100.0
    );
    if f.doors_s.is_some() {
        s += &format!("   HULL THRUSTERS {}", if l.lit { "LIT" } else { "OPENING" });
    }
    if f.falling {
        s += "   FALLING";
    }
    // amber while its hull thrusters open and hold it, green again once it is falling
    let colour = if f.doors_s.is_some() && !f.falling { AMBER } else { GREEN };
    (s, colour)
}

/// `WAVE 2/3   CITY 2  DAMAGE 23 %   RAIDERS 6/10   7:41`, coloured by how close the city or the
/// clock is to losing. Empty outside the mission.
fn fmt_mission(run: Option<&crate::defence::DefenceRun>, cfg: &cb_core::mission::DefenceConfig) -> (String, Color) {
    use cb_core::mission::Phase;
    let Some(run) = run else { return (String::new(), GREEN) };
    let waves = cfg.cities.len();
    match run.state.phase {
        Phase::Briefing => ("DEFEND THE CITIES: board your mech".into(), GREEN),
        Phase::Lull { wave, left_s } => (
            format!(
                "WAVE {}/{waves} IN {:.0} s   ON CITY {}",
                wave + 1,
                left_s.ceil(),
                cfg.cities[wave] + 1
            ),
            AMBER,
        ),
        Phase::Wave { wave, .. } => {
            let damage = run.city_damage(cfg);
            let left = run.state.time_left_s(cfg).unwrap_or(0.0);
            let s = format!(
                "WAVE {}/{waves}   CITY {}  DAMAGE {:.0} %   ENEMIES {}/{}   {}:{:02}",
                wave + 1,
                cfg.cities[wave] + 1,
                damage * 100.0,
                run.alive,
                cfg.wave_size(wave),
                (left / 60.0) as u32,
                (left % 60.0) as u32
            );
            let colour = if damage > 0.45 || left < 60.0 {
                RED
            } else if damage > 0.35 || left < 120.0 {
                AMBER
            } else {
                GREEN
            };
            (s, colour)
        }
        Phase::Won => ("ALL WAVES DESTROYED: THE CITIES STAND".into(), GREEN),
        Phase::Lost(why) => (format!("MISSION LOST: {}", crate::defence::loss_text(why)), RED),
    }
}

#[allow(clippy::too_many_arguments)]
fn update_hud(
    pilot: Res<PilotSim>,
    data: Res<crate::sim::Data>,
    run: Option<Res<crate::defence::DefenceRun>>,
    fortress: Option<Res<crate::fortress::FortressRun>>,
    free: Option<Res<crate::missions::FreeRoamRun>>,
    service: Res<crate::air_bases::LastService>,
    plan: Res<Plan>,
    mech: Query<(&MechSim, &MechSpec), With<Piloted>>,
    mut roots: Query<&mut Visibility, With<HudRoot>>,
    mut arrow: Single<&mut UiTransform, With<LegArrow>>,
    mut gauges: Query<(&Gauge, &mut Node, &mut BackgroundColor)>,
    mut readouts: Query<(&Readout, &mut Text, &mut TextColor)>,
) {
    let seated = pilot.curr.mode == PilotMode::Seated;
    let Some((sim, spec)) = mech.iter().next() else { return };
    for mut root in &mut roots {
        *root = if seated { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !seated {
        return;
    }
    let m = &sim.curr;
    // Legs relative to the torso: the torso is up; a torso twisted left puts the legs to the right.
    arrow.rotation = Rot2::radians(m.twist_rad);
    let (jet, sk) = (&spec.jet, &spec.skate);
    for (g, mut node, mut bg) in &mut gauges {
        let (fill, colour) = match g {
            Gauge::Boost => (
                1.0 - m.boost.heat,
                if m.boost.locked {
                    RED
                } else if m.boosting {
                    AMBER
                } else {
                    GREEN
                },
            ),
            Gauge::Skate => (
                1.0 - m.skate.heat,
                if m.skate.locked {
                    RED
                } else if m.skates_on {
                    AMBER
                } else {
                    GREEN
                },
            ),
            Gauge::Throttle => {
                let lo = spec.locomotion.throttle_min;
                (
                    ((m.throttle - lo) / (1.0 - lo)).clamp(0.0, 1.0),
                    if m.throttle < 0.0 { AMBER } else { GREEN },
                )
            }
        };
        node.width = percent(fill * 100.0);
        bg.0 = colour;
    }
    let speed_kmh = m.vel.length() * 3.6;
    let agl = m.pos.y - plan.terrain.height_at(m.pos.x, m.pos.z);
    let mode = match (m.on_water, m.mode) {
        (OnWater::Skimming, _) => "SKIMMING",
        (OnWater::Hovering, _) => "ROCKETS HOLDING US UP",
        (OnWater::Sunk, _) => "SUNK  recharging, Space to rise",
        (OnWater::Dry, LocomotionMode::Walk) => "WALK",
        (OnWater::Dry, LocomotionMode::Skate) => "SKATE",
        (OnWater::Dry, LocomotionMode::Jet) => "ROCKETS",
    };
    for (r, mut t, mut c) in &mut readouts {
        let (s, colour) = match r {
            Readout::Boost => fmt_budget("BOOST", m.boost.heat, m.boost.locked, jet.burn_s, jet.recharge_s, m.boosting),
            Readout::Skate => fmt_budget(
                if m.skates_on { "SKATE ON" } else { "SKATE" },
                m.skate.heat,
                m.skate.locked,
                sk.burn_s,
                sk.recharge_s,
                m.mode == LocomotionMode::Skate,
            ),
            Readout::Legs => (format!("LEGS {:+.0}", -m.twist_rad.to_degrees()), GREEN),
            // What the last bay did, from the report the bay's own rule returned (CLAUDE.md 6.1).
            Readout::Service => match &service.last {
                Some((base, bay, lines)) if service.since_s < crate::air_bases::SERVICE_SHOWN_S => {
                    (format!("BASE {} BAY {}\n{}", base + 1, bay + 1, lines.join("\n")), AMBER)
                }
                _ => (String::new(), GREEN),
            },
            // One fact a line on the MFD.
            Readout::Nav => (
                fmt_nav(pilot.curr.yaw_rad, Vec2::new(m.pos.x, m.pos.z), &plan.cities).replace("   ", "\n"),
                GREEN,
            ),
            Readout::Mission => {
                // The running mission's readout (one runs at a time: free-roam-missions), or free roam's.
                let (s, c) = match (&fortress, run.as_deref()) {
                    (Some(f), _) => fmt_fortress(f, &data.0.fortress),
                    (None, Some(_)) => fmt_mission(run.as_deref(), &data.0.defence),
                    (None, None) => (crate::missions::idle_line(free.as_deref()).unwrap_or_default().to_string(), GREEN),
                };
                (s.replace("   ", "\n").replace("  ", "\n"), c)
            }
            Readout::Flight => (
                format!("{mode}\n{speed_kmh:.0} KM/H\nALT {agl:.0} M"),
                if m.on_water == OnWater::Sunk {
                    RED
                } else if m.mode == LocomotionMode::Walk {
                    GREEN
                } else {
                    AMBER
                },
            ),
            Readout::Guns => {
                let mut parts = vec![format!("THR {:.0}%", m.throttle * 100.0)];
                for (i, w) in spec.weapons.iter().enumerate() {
                    let cd = sim.guns.cooldown_s.get(i).copied().unwrap_or(0.0);
                    let gone = sim.damage.is_lost(&spec.parts, w.arm);
                    let state = if gone {
                        "LOST".to_string()
                    } else if cd > 0.05 {
                        format!("{cd:.1}s")
                    } else {
                        "READY".to_string()
                    };
                    // The left mouse fires the selected station (weapon-select); the right mouse and F
                    // fire their groups.
                    let button = match w.group {
                        _ if i == sim.selected => "LMB",
                        1 => "RMB",
                        2 => "F",
                        _ => continue,
                    };
                    parts.push(format!("{button} {state}"));
                }
                (parts.join("\n"), GREEN)
            }
            Readout::Damage => {
                let mut worst = 1.0f32;
                let rows: Vec<String> = spec
                    .parts
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| {
                        let f = sim.damage.structure[i] / p.structure.max(1.0);
                        worst = worst.min(f);
                        let name = match p.kind {
                            PartKind::ArmL => "ARM L",
                            PartKind::ArmR => "ARM R",
                            PartKind::LegL => "LEG L",
                            PartKind::LegR => "LEG R",
                            PartKind::Head => "HEAD",
                            PartKind::Torso => "TORSO",
                        };
                        // Only what is hurt: big and minimal on the MFD.
                        if sim.damage.lost[i] {
                            Some(format!("{name} LOST"))
                        } else if f < 0.995 {
                            Some(format!("{name} {:.0}%", f * 100.0))
                        } else {
                            None
                        }
                    })
                    .collect();
                (
                    if rows.is_empty() {
                        "ALL PARTS OK".to_string()
                    } else {
                        rows.into_iter().take(4).collect::<Vec<_>>().join("\n")
                    },
                    if worst < 0.3 {
                        RED
                    } else if worst < 0.7 {
                        AMBER
                    } else {
                        GREEN
                    },
                )
            }
        };
        if t.0 != s {
            t.0 = s;
        }
        c.0 = colour;
    }
}

/// The climb-out ring: hidden until Use is held in the seat, then lit dot by dot as the hold fills,
/// from the pilot's own hold clock (`cb_core::pilot`), so the ring and the exit agree.
/// The signs under the crosshair, seated only: the selected weapon (with its rounds when it has a
/// limit), and `[MELEE MODE]` while close combat is on, with the punch keys.
#[allow(clippy::type_complexity)]
fn draw_signs(
    pilot: Res<PilotSim>,
    data: Res<crate::sim::Data>,
    me: Query<(&MechSim, &MechSpec), With<Piloted>>,
    mut signs: Query<(&Children, &mut Visibility, Has<MeleeSign>), Or<(With<MeleeSign>, With<WeaponSign>)>>,
    mut texts: Query<&mut Text>,
) {
    let seated = pilot.curr.mode == PilotMode::Seated;
    let mech = me.iter().next().filter(|_| seated);
    for (children, mut vis, melee) in &mut signs {
        let line = mech.and_then(|(sim, spec)| {
            if melee {
                sim.close.on.then(|| {
                    let keys = |names: &[String]| names.iter().map(|k| cb_core::input::key_label(k)).collect::<Vec<_>>().join("/");
                    let b = &data.0.bindings;
                    format!("[MELEE MODE]  {} {} punch", keys(&b.strike_left), keys(&b.strike_right))
                })
            } else {
                spec.weapons.get(sim.selected).map(|w| {
                    let rounds = sim.guns.rounds_left.get(sim.selected).copied().unwrap_or(w.rounds);
                    match rounds {
                        Some(n) => format!("{} x{n}", w.name.to_uppercase()),
                        None => w.name.to_uppercase(),
                    }
                })
            }
        });
        *vis = if line.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let (Some(line), Some(c)) = (line, children.first())
            && let Ok(mut t) = texts.get_mut(*c)
            && t.0 != line
        {
            t.0 = line;
        }
    }
}

fn exit_ring(pilot: Res<PilotSim>, data: Res<crate::sim::Data>, mut dots: Query<(&ExitDot, &mut Visibility, &mut BackgroundColor)>) {
    let k = (pilot.curr.exit_hold_s / data.0.pilot.exit_hold_s).clamp(0.0, 1.0);
    let lit = (k * EXIT_DOTS as f32).ceil() as usize;
    for (d, mut v, mut c) in &mut dots {
        *v = if k > 0.0 { Visibility::Visible } else { Visibility::Hidden };
        c.0 = if d.0 < lit { AMBER } else { AMBER.with_alpha(0.25) };
    }
}

/// The WEAPONS and TARGET pages, from the piloted mech's own stations and the targeting state.
#[allow(clippy::type_complexity)]
fn update_stations(
    data: Res<crate::sim::Data>,
    targeting: Res<crate::targeting::TargetState>,
    units: Res<crate::units::Units>,
    fortress: Option<Res<crate::fortress::FortressRun>>,
    me: Query<(&MechSim, &MechSpec), With<Piloted>>,
    mechs: Query<(&MechSim, &MechSpec, &crate::mech::MechId)>,
    mut rows: Query<(&StationRow, &mut Text, &mut TextColor, &mut BorderColor, &mut Visibility), Without<TargetReadout>>,
    mut target: Query<(&mut Text, &mut TextColor), (With<TargetReadout>, Without<StationRow>)>,
) {
    let Some((sim, spec)) = me.iter().next() else { return };
    for (row, mut t, mut c, mut b, mut v) in &mut rows {
        let Some(w) = spec.weapons.get(row.0) else {
            *v = Visibility::Hidden;
            continue;
        };
        *v = Visibility::Inherited;
        let lost = sim.damage.is_lost(&spec.parts, w.arm);
        let rounds = sim.guns.rounds_left.get(row.0).copied().unwrap_or(w.rounds);
        // The station's state from the clocks its shots fire from (weapon-kinds): charge, heat, reload.
        use cb_core::weapons::StationState as S;
        let state = match sim.guns.state(&spec.weapons, row.0, lost) {
            S::Lost => "LOST".to_string(),
            S::Empty => "EMPTY".to_string(),
            S::Hot => "HOT".to_string(),
            S::Charging(k) => format!("CHARGE {:.0}%", k * 100.0),
            S::Reloading(cd) => format!("{cd:.1}s"),
            S::Ready => "READY".to_string(),
        };
        // A pod shows the locks it holds (multi-lock).
        let state = match w.multi_lock() {
            Some(seeker) if !sim.guns.locks.held.is_empty() => {
                format!("LOCKS {}/{}", sim.guns.locks.held.len(), seeker.max_locks)
            }
            _ => state,
        };
        // A weapon that runs hot shows the bank it shares (weapon-kinds section 6).
        let state = if w.heat > 0.0 {
            format!("{state}  HEAT {:.0}%", sim.guns.heat.heat * 100.0)
        } else {
            state
        };
        let name = w.name.to_uppercase();
        let count = rounds.map_or_else(String::new, |n| format!(" x{n}"));
        // The signature weapon is starred (signature-weapons).
        let star = if w.signature { "*" } else { "" };
        t.0 = format!("STA {}{star}  {name}{count}  {state}", row.0 + 1);
        let selected = row.0 == sim.selected;
        c.0 = if lost {
            RED
        } else if selected {
            AMBER
        } else {
            GREEN
        };
        *b = BorderColor::all(if selected { AMBER } else { Color::NONE });
    }
    let Ok((mut t, mut c)) = target.single_mut() else { return };
    let Some(id) = targeting.selected else {
        t.0 = "NO TARGET".into();
        c.0 = GREEN;
        return;
    };
    // A civil aircraft is never selected (survey W47), so the TARGET page names enemies only. A part of the
    // fortress is named with the fortress (survey H2: `STRATOSTACEAN` over `LIFT THRUSTER L2`).
    let part = id
        .checked_sub(crate::fortress::FORTRESS_TARGET_BASE)
        .map(|i| i as usize)
        .zip(fortress.as_deref())
        .filter(|(i, run)| *i < run.f.parts.len());
    let (name, health) = if let Some((i, run)) = part {
        let d = &data.0.fortress;
        let k = d.kind(i);
        (
            format!("{}\n{} {}", d.cfg.name, k.name, d.shape.parts[i].id).to_uppercase(),
            run.f.parts[i].hp / k.hp.max(1.0),
        )
    } else if id >= crate::units::UNIT_ID_BASE {
        match units.list.iter().find(|p| p.unit.id == id) {
            Some(p) => {
                let full = data.0.units.units.get(&p.unit.kind).map_or(1.0, |s| s.hp.max(1.0));
                (p.unit.kind.to_uppercase(), p.unit.hp / full)
            }
            None => ("UNKNOWN".into(), 0.0),
        }
    } else {
        match mechs.iter().find(|m| m.2.0 == id) {
            Some((s, sp, _)) => {
                let torso = sp.parts.iter().position(|p| p.kind == PartKind::Torso);
                let h = torso.map_or(1.0, |i| s.damage.structure[i] / sp.parts[i].structure.max(1.0));
                (sp.name.to_uppercase(), h)
            }
            None => ("UNKNOWN".into(), 0.0),
        }
    };
    let lock = if targeting.lock.locked(&data.0.targeting) {
        "LOCK"
    } else {
        "LOCKING"
    };
    t.0 = format!(
        "TARGET\n{name}\n{:.0} M\n{lock}\nHEALTH {:.0}%",
        targeting.range_m,
        (health * 100.0).clamp(0.0, 100.0)
    );
    c.0 = if targeting.lock.locked(&data.0.targeting) { AMBER } else { GREEN };
}

/// The missile lock round the selected target, from the targeting's own lock clock: while it builds,
/// two amber boxes (one square, one turned 45 degrees) close in from wide; locked, they hold tight round
/// the target in red and pulse.
#[allow(clippy::type_complexity)]
fn draw_lock(
    pilot: Res<PilotSim>,
    data: Res<crate::sim::Data>,
    state: Res<crate::targeting::TargetState>,
    hostiles: Res<crate::units::Hostiles>,
    time: Res<Time>,
    camera: Single<(&Camera, &GlobalTransform), With<crate::pilot::PilotCamera>>,
    mut boxes: Query<(&LockBox, &mut Node, &mut BorderColor, &mut UiTransform, &mut Visibility)>,
) {
    let cfg = &data.0.targeting;
    let (cam, at) = *camera;
    let target = state
        .selected
        .filter(|_| pilot.curr.mode == PilotMode::Seated && state.lock.held_s > 0.0 && state.lock.target == state.selected)
        .and_then(|id| hostiles.0.iter().find(|h| h.id == id));
    let placed = target.and_then(|h| {
        let top = cam.world_to_viewport(at, h.feet + Vec3::Y * h.height_m).ok()?;
        let foot = cam.world_to_viewport(at, h.feet).ok()?;
        let centre = cam.world_to_viewport(at, h.aim).ok()?;
        Some((centre, (foot.y - top.y).abs().max(24.0)))
    });
    let k = (state.lock.held_s / cfg.lock_s.max(1e-3)).clamp(0.0, 1.0);
    let locked = state.lock.locked(cfg);
    let pulse = 0.65 + 0.35 * (time.elapsed_secs() * 9.0).sin().abs();
    for (b, mut node, mut border, mut turn, mut vis) in &mut boxes {
        let Some((c, w)) = placed else {
            *vis = Visibility::Hidden;
            continue;
        };
        // Wide while the lock is young, tight round the target once it holds.
        let tight = if b.0 == 0 { 1.45 } else { 1.15 };
        let size = w * (tight + (1.0 - k) * 2.0);
        node.left = px(c.x - size * 0.5);
        node.top = px(c.y - size * 0.5);
        node.width = px(size);
        node.height = px(size);
        turn.rotation = Rot2::degrees(if b.0 == 1 { 45.0 } else { 0.0 });
        *border = BorderColor::all(if locked {
            RED.with_alpha(pulse)
        } else {
            AMBER.with_alpha(0.4 + 0.6 * k)
        });
        *vis = Visibility::Inherited;
    }
}

/// The accepted job in a career: a diamond and its distance over the site on the screens, and a line on
/// the MISSION page (where, how far, the time left, the enemies left), from the career's own board.
#[allow(clippy::type_complexity)]
fn draw_job(
    career: Option<Res<crate::career::CareerRun>>,
    pilot: Res<PilotSim>,
    plan: Res<Plan>,
    camera: Single<(&Camera, &GlobalTransform), With<crate::pilot::PilotCamera>>,
    enemies: Query<(&MechSim, &crate::career::JobEnemy)>,
    mut marker: Query<(&mut Node, &mut Text, &mut Visibility), (With<JobMarker>, Without<JobLine>)>,
    mut line: Query<&mut Text, (With<JobLine>, Without<JobMarker>)>,
) {
    let job = career.as_ref().and_then(|c| c.career.board.current().cloned());
    let Ok((mut node, mut text, mut vis)) = marker.single_mut() else {
        return;
    };
    let Some(j) = job else {
        *vis = Visibility::Hidden;
        if let Ok(mut l) = line.single_mut() {
            l.0.clear();
        }
        return;
    };
    let (cam, at) = *camera;
    let eye = at.translation();
    let ground = plan.terrain.height_at(j.site.x, j.site.y);
    let site = Vec3::new(j.site.x, ground + 20.0, j.site.y);
    let d = Vec2::new(eye.x, eye.z).distance(j.site);
    let km = d / 1000.0;
    match cam
        .world_to_viewport(at, site)
        .ok()
        .filter(|_| pilot.curr.mode == PilotMode::Seated)
    {
        Some(p) => {
            node.left = px(p.x - 40.0);
            node.top = px(p.y - 14.0);
            text.0 = format!("<> JOB {km:.1} KM");
            *vis = Visibility::Inherited;
        }
        None => *vis = Visibility::Hidden,
    }
    if let Ok(mut l) = line.single_mut() {
        let bearing = bearing_deg(Vec2::new(j.site.x - eye.x, j.site.y - eye.z));
        let left = enemies.iter().filter(|(s, e)| e.0 == j.id && !s.dead).count();
        let m = (j.clock_s / 60.0).floor();
        let s = j.clock_s - m * 60.0;
        let fight = if j.state == cb_core::jobs::JobState::Active {
            format!("\nENEMIES {left}")
        } else {
            String::new()
        };
        l.0 = format!(
            "JOB CITY {}\n{km:.1} KM BRG {:03}\nTIME {m:.0}:{s:02.0}{fight}",
            j.city + 1,
            whole_deg(bearing)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_locked_target_reads_lock() {
        assert_eq!(fmt_target(850.4, 12.0, true, true), "850 m  12 m/s  LOCK");
        assert_eq!(fmt_target(850.4, 12.0, true, false), "850 m  12 m/s  IN RANGE");
        assert_eq!(fmt_target(1500.0, 3.0, false, false), "1500 m  3 m/s");
    }

    #[test]
    fn looking_along_minus_z_reads_north() {
        assert!(fmt_nav(0.0, Vec2::ZERO, &[]).starts_with("HDG 000 N"));
    }

    #[test]
    fn turning_right_reads_east() {
        // The yaw turns left: a quarter turn right is minus a quarter turn of yaw.
        assert!(fmt_nav(-std::f32::consts::FRAC_PI_2, Vec2::ZERO, &[]).starts_with("HDG 090 E"));
    }
}
