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
use crate::world::Plan;
use bevy::prelude::*;
use bevy::ui::UiTransform;
use cb_core::mech::{LocomotionMode, OnWater, PartKind};
use cb_core::pilot::PilotMode;

/// Draws the HUD.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud).add_systems(
            Update,
            (update_hud, draw_targets, exit_ring, update_stations).in_set(PresentSet::Camera),
        );
    }
}

#[derive(Component)]
struct HudRoot;

/// The root of the MFD pages (cockpit-mfds).
#[derive(Component)]
pub struct MfdRoot;

/// One MFD page's side, pixels; the MFD texture is three pages wide (a row a multiple of 256 bytes).
pub const MFD_PAGE_PX: f32 = 512.0;

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

/// The TARGET page's readout.
#[derive(Component)]
struct TargetReadout;

/// Dots in the climb-out ring.
const EXIT_DOTS: usize = 24;
/// The climb-out ring's radius, pixels.
const EXIT_RING_PX: f32 = 34.0;

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
}

const GREEN: Color = Color::srgb(0.55, 0.95, 0.6);
const AMBER: Color = Color::srgb(1.0, 0.75, 0.25);
const RED: Color = Color::srgb(1.0, 0.3, 0.25);
const FRAME: Color = Color::srgba(0.55, 0.95, 0.6, 0.35);

fn text(s: &str, size: f32) -> (Text, TextFont, TextColor) {
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
    // The mech's readouts, on the cockpit's MFDs (cockpit-mfds): three 512 x 512 pages in a row,
    // drawn by the MFD camera into one texture each screen shows a third of. Without MFDs (the
    // Canopy) the same pages sit along the bottom of the view.
    commands
        .spawn((
            HudRoot,
            MfdRoot,
            Node {
                width: px(MFD_PAGE_PX * 3.0),
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
    hostiles: Res<crate::units::Hostiles>,
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
    // The enemies in range, mechs and vehicles, nearest first.
    let mut shown: Vec<(f32, crate::units::Hostile)> = hostiles
        .0
        .iter()
        .filter(|_| seated)
        .map(|h| (h.aim.distance(eye), *h))
        .filter(|(d, _)| *d <= cfg.max_range_m)
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
    let mut texts: Vec<(String, Color)> = vec![(String::new(), GREEN); BOXES];
    let mut placed = vec![None; BOXES];
    let mut lead_at = None;
    let mut arrow_dir = None;
    for (slot, (d, h)) in shown.iter().take(BOXES).enumerate() {
        let (id, feet, torso, vel) = (&h.id, h.feet, h.aim, h.vel);
        let selected = state.selected == Some(*id);
        let colour = if selected && locked == Some(*id) {
            RED
        } else if h.shelling {
            AMBER
        } else {
            RED.with_alpha(0.85)
        };
        if selected {
            let w = me.iter().next().and_then(|spec| spec.weapons.first().cloned());
            if let Some(w) = w {
                let p = cb_core::weapons::lead(eye, torso, vel, w.speed_m_s, !w.rocket);
                lead_at = cam.world_to_viewport(at, p).ok().filter(|p| on_screen(*p));
            }
            let in_range = *d <= cfg.lock_range_m;
            texts[slot] = (fmt_target(*d, vel.length(), in_range, locked == Some(*id)), colour);
        } else {
            texts[slot] = (format!("{d:.0} m"), colour);
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
fn bearing_deg(dir: Vec2) -> f32 {
    dir.x.atan2(-dir.y).to_degrees().rem_euclid(360.0)
}

/// A bearing as the compass shows it, whole degrees 0..359.
fn whole_deg(deg: f32) -> i32 {
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
            // One fact a line on the MFD.
            Readout::Nav => (
                fmt_nav(pilot.curr.yaw_rad, Vec2::new(m.pos.x, m.pos.z), &plan.cities).replace("   ", "\n"),
                GREEN,
            ),
            Readout::Mission => {
                let (s, c) = fmt_mission(run.as_deref(), &data.0.defence);
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
            Readout::Guns => 'guns: {
                let mut parts = vec![format!("THR {:.0}%", m.throttle * 100.0)];
                // In close combat the mouse buttons strike and the guns hold (close-combat).
                if sim.close.on {
                    parts.push("CLOSE COMBAT".into());
                    for (button, kind) in [("LMB", PartKind::ArmL), ("RMB", PartKind::ArmR)] {
                        let state = if sim.damage.is_lost(&spec.parts, kind) { "LOST" } else { "STRIKE" };
                        parts.push(format!("{button} {state}"));
                    }
                    break 'guns (parts.join("\n"), AMBER);
                }
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
        let cd = sim.guns.cooldown_s.get(row.0).copied().unwrap_or(0.0);
        let state = if lost {
            "LOST".to_string()
        } else if cd > 0.05 {
            format!("{cd:.1}s")
        } else {
            "READY".to_string()
        };
        let name = w.name.to_uppercase();
        t.0 = format!("STA {}  {name}  {state}", row.0 + 1);
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
    let (name, health) = if id >= crate::units::UNIT_ID_BASE {
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
