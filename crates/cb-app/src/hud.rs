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
        app.add_systems(Startup, spawn_hud)
            .add_systems(Update, (update_hud, draw_targets).in_set(PresentSet::Camera));
    }
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct LegArrow;

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
            col.spawn((text(label, 15.0), readout));
            col.spawn((
                Node {
                    width: px(220.0),
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
            // Navigation, top centre: heading and the nearest city.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(24.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|c| {
                c.spawn((text("", 18.0), Readout::Nav));
            });
            // The mission, under it.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(52.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|c| {
                c.spawn((text("", 22.0), Readout::Mission));
            });
            // Leg compass, bottom centre: the torso is straight up; the arrow is the legs.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: px(28.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(px(-70.0)),
                width: px(140.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(6.0),
                ..default()
            })
            .with_children(|c| {
                c.spawn((
                    Node {
                        width: px(110.0),
                        height: px(110.0),
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
                            top: px(-2.0),
                            width: px(8.0),
                            height: px(14.0),
                            ..default()
                        },
                        BackgroundColor(AMBER),
                    ));
                    // The legs: a bar through the centre, its head marked, turned by the twist.
                    ring.spawn((
                        Node {
                            width: px(10.0),
                            height: px(90.0),
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
                                width: px(18.0),
                                height: px(18.0),
                                border_radius: BorderRadius::all(px(3.0)),
                                ..default()
                            },
                            BackgroundColor(GREEN),
                        ));
                        arrow.spawn((
                            Node {
                                width: px(4.0),
                                height: px(72.0),
                                ..default()
                            },
                            BackgroundColor(GREEN.with_alpha(0.7)),
                        ));
                    });
                });
                c.spawn((text("LEGS", 14.0), Readout::Legs));
            });
            // Budgets, bottom left.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: px(28.0),
                left: px(28.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            })
            .with_children(|c| {
                bar(c, Gauge::Boost, Readout::Boost, "BOOST");
                bar(c, Gauge::Skate, Readout::Skate, "SKATE");
            });
            // Flight, right: throttle, speed, altitude, mode.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: px(28.0),
                right: px(28.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                align_items: AlignItems::End,
                ..default()
            })
            .with_children(|c| {
                c.spawn((text("", 16.0), Readout::Flight));
                bar(c, Gauge::Throttle, Readout::Guns, "");
                c.spawn((text("", 14.0), Readout::Damage));
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
    enemies: Query<(&MechSim, &crate::mech::MechId, &MechSpec, Option<&crate::mech::EnemyGoal>), With<crate::mech::Enemy>>,
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
    // The enemies in range, nearest first.
    let mut shown: Vec<(f32, &MechSim, u32, bool)> = enemies
        .iter()
        .filter(|(e, ..)| !e.dead && seated)
        .map(|(e, id, _, goal)| {
            let torso = e.curr.pos + Vec3::Y * cb_core::ai::TORSO_AIM_M;
            (torso.distance(eye), e, id.0, goal.is_some_and(|g| g.point.is_some()))
        })
        .filter(|(d, ..)| *d <= cfg.max_range_m)
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
    let mut texts: Vec<(String, Color)> = vec![(String::new(), GREEN); BOXES];
    let mut placed = vec![None; BOXES];
    let mut lead_at = None;
    let mut arrow_dir = None;
    for (slot, (d, e, id, shelling)) in shown.iter().take(BOXES).enumerate() {
        let feet = e.curr.pos;
        let torso = feet + Vec3::Y * cb_core::ai::TORSO_AIM_M;
        let selected = state.selected == Some(*id);
        let colour = if selected && locked == Some(*id) {
            RED
        } else if *shelling {
            AMBER
        } else {
            RED.with_alpha(0.85)
        };
        let vel = Vec3::new(e.curr.vel.x, e.curr.vel_y_m_s, e.curr.vel.y);
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
        let (Ok(top), Ok(bottom)) = (cam.world_to_viewport(at, feet + Vec3::Y * 12.0), cam.world_to_viewport(at, feet)) else {
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
    if locked {
        (format!("{name}  EMPTY  recharging {recharge:.1} s"), RED)
    } else if active {
        (format!("{name}  {left:.1} s left  (recharge {recharge:.1} s)"), AMBER)
    } else if heat > 0.0 {
        (format!("{name}  {left:.1} s  recharging {recharge:.1} s"), GREEN)
    } else {
        (format!("{name}  {burn_s:.0} s  ready"), GREEN)
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
                "WAVE {}/{waves}   CITY {}  DAMAGE {:.0} %   RAIDERS {}/{}   {}:{:02}",
                wave + 1,
                cfg.cities[wave] + 1,
                damage * 100.0,
                run.alive,
                cfg.raiders_per_wave,
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
    mut root: Single<&mut Visibility, With<HudRoot>>,
    mut arrow: Single<&mut UiTransform, With<LegArrow>>,
    mut gauges: Query<(&Gauge, &mut Node, &mut BackgroundColor)>,
    mut readouts: Query<(&Readout, &mut Text, &mut TextColor)>,
) {
    let seated = pilot.curr.mode == PilotMode::Seated;
    let Some((sim, spec)) = mech.iter().next() else { return };
    **root = if seated { Visibility::Inherited } else { Visibility::Hidden };
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
            Readout::Boost => fmt_budget(
                "ROCKETS [Space up, Ctrl dive]",
                m.boost.heat,
                m.boost.locked,
                jet.burn_s,
                jet.recharge_s,
                m.boosting,
            ),
            Readout::Skate => fmt_budget(
                if m.skates_on { "SKATES ON [Shift]" } else { "SKATES [Shift]" },
                m.skate.heat,
                m.skate.locked,
                sk.burn_s,
                sk.recharge_s,
                m.mode == LocomotionMode::Skate,
            ),
            Readout::Legs => {
                let turn = if m.turn_rate_rad_s > 0.01 {
                    "  turning left"
                } else if m.turn_rate_rad_s < -0.01 {
                    "  turning right"
                } else {
                    ""
                };
                (format!("LEGS {:+.0} deg{turn}", -m.twist_rad.to_degrees()), GREEN)
            }
            Readout::Nav => (fmt_nav(pilot.curr.yaw_rad, Vec2::new(m.pos.x, m.pos.z), &plan.cities), GREEN),
            Readout::Mission => fmt_mission(run.as_deref(), &data.0.defence),
            Readout::Flight => (
                format!("{mode}   {speed_kmh:.0} km/h   ALT {agl:.0} m"),
                if m.on_water == OnWater::Sunk {
                    RED
                } else if m.mode == LocomotionMode::Walk {
                    GREEN
                } else {
                    AMBER
                },
            ),
            Readout::Guns => {
                let mut parts = vec![format!("THROTTLE {:.0}%", m.throttle * 100.0)];
                for (i, w) in spec.weapons.iter().enumerate() {
                    let cd = sim.guns.cooldown_s.get(i).copied().unwrap_or(0.0);
                    let gone = sim.damage.is_lost(&spec.parts, w.arm);
                    let state = if gone {
                        "LOST".to_string()
                    } else if cd > 0.05 {
                        format!("{cd:.1} s")
                    } else {
                        "READY".to_string()
                    };
                    let button = match w.group {
                        0 => "LMB",
                        1 => "RMB",
                        _ => "F",
                    };
                    parts.push(format!("{} [{button}] {state}", w.name));
                }
                (parts.join("\n"), GREEN)
            }
            Readout::Damage => {
                let mut worst = 1.0f32;
                let rows: Vec<String> = spec
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
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
                        if sim.damage.lost[i] {
                            format!("{name} --")
                        } else {
                            format!("{name} {:.0}%", f * 100.0)
                        }
                    })
                    .collect();
                (
                    rows.join("  "),
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
