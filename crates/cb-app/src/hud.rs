//! The cockpit HUD prototype (openspec change `cockpit-hud`; owner, 2026-10-24: "quick rapid ui
//! prototyping in the cockpit"): a crosshair, a leg compass (which way the legs face against the
//! torso), booster and skate gauges (heat, seconds left, seconds to recharge), throttle, speed,
//! altitude, mode, guns and damage. Shown only while seated. Every number is the simulation's own
//! (CLAUDE.md section 6.1). A 2D overlay for now; it moves onto the cockpit's screens later.

use crate::mech::{MechSim, MechSpec, Piloted};
use crate::pilot::PilotSim;
use crate::sim::PresentSet;
use crate::world::Plan;
use bevy::prelude::*;
use bevy::ui::UiTransform;
use cb_core::mech::{LocomotionMode, PartKind};
use cb_core::pilot::PilotMode;

/// Draws the HUD.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
            .add_systems(Update, update_hud.in_set(PresentSet::Camera));
    }
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct LegArrow;

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

fn update_hud(
    pilot: Res<PilotSim>,
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
    let mode = match m.mode {
        LocomotionMode::Walk => "WALK",
        LocomotionMode::Skate => "SKATE",
        LocomotionMode::Jet => "BOOST",
    };
    for (r, mut t, mut c) in &mut readouts {
        let (s, colour) = match r {
            Readout::Boost => fmt_budget(
                "BOOST [Space]",
                m.boost.heat,
                m.boost.locked,
                jet.burn_s,
                jet.recharge_s,
                m.boosting,
            ),
            Readout::Skate => fmt_budget(
                if m.skates_on { "SKATE ON [Shift]" } else { "SKATE [Shift]" },
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
            Readout::Flight => (
                format!("{mode}   {speed_kmh:.0} km/h   ALT {agl:.0} m"),
                if m.mode == LocomotionMode::Walk { GREEN } else { AMBER },
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
                    let button = if w.group == 0 { "LMB" } else { "RMB" };
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
