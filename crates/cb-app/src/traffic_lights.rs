//! 1980s traffic lights at the arterials' junctions in the game (survey W39; owner, 2026-10-26: "Lights on the
//! arterials"). Every junction the lane graph lit (`Node::signal`) gets a pole and a three-lamp head at the kerb of
//! each road into it, facing the cars that come down that road. Each frame the lamps show what
//! `cb_core::traffic::light` says for that road's phase at the traffic's own clock: the one rule the cars obey.
//!
//! The heads are block-outs (a pole, a housing and three lenses, named so in their scene nodes), stand-ins until a
//! modelled set is built in Blender. They share four meshes and eight materials, so Bevy batches them, and are drawn
//! within `SHOWN_M` of the eye.

use crate::sim::Data;
use crate::state::GameApp;
use crate::traffic::{Roads, TrafficSim};
use crate::world::Plan;
use bevy::camera::visibility::VisibilityRange;
use bevy::prelude::*;
use cb_core::traffic::{Light, light};
use cb_core::world::lanes::right_of;

/// A light head: its junction and its phase.
#[derive(Component)]
struct LightHead {
    node: u32,
    phase: u8,
}

/// One of a head's three lenses, and what it shows when lit.
#[derive(Component)]
struct Lens(Light);

/// The lenses' materials, dark and lit, by colour (red, amber, green).
#[derive(Resource)]
struct LensLooks {
    dark: [Handle<StandardMaterial>; 3],
    lit: [Handle<StandardMaterial>; 3],
}

/// Heads are drawn within this of the eye, metres.
const SHOWN_M: f32 = 700.0;
/// The pole stands this far out past the road's edge, metres.
const KERB_OUT_M: f32 = 1.2;
/// The pole's height and radius, metres.
const POLE_M: (f32, f32) = (4.6, 0.11);
/// The housing: width, height and depth, metres; its middle over the ground.
const HOUSING_M: Vec3 = Vec3::new(0.42, 1.15, 0.3);
const HOUSING_AT_M: f32 = 4.9;
/// A lens's radius and the step between lenses, metres.
const LENS_R_M: f32 = 0.15;
const LENS_STEP_M: f32 = 0.36;

/// Draws the traffic lights and lights their lamps.
pub struct TrafficLightsPlugin;

impl Plugin for TrafficLightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_world_spawn(spawn_lights).add_game_systems(Update, show_lights);
    }
}

fn index(l: Light) -> usize {
    match l {
        Light::Red => 0,
        Light::Amber => 1,
        Light::Green => 2,
    }
}

fn spawn_lights(
    mut commands: Commands,
    roads: Res<Roads>,
    plan: Res<Plan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let g = &roads.0;
    let colours = [
        LinearRgba::rgb(1.0, 0.08, 0.04),
        LinearRgba::rgb(1.0, 0.55, 0.05),
        LinearRgba::rgb(0.1, 1.0, 0.45),
    ];
    let mut lens = |c: LinearRgba, lit: bool| {
        materials.add(StandardMaterial {
            base_color: Color::LinearRgba(if lit { c } else { c * 0.08 }.with_alpha(1.0)),
            emissive: if lit { c * 40.0 } else { LinearRgba::BLACK },
            perceptual_roughness: 0.25,
            ..default()
        })
    };
    let looks = LensLooks {
        dark: colours.map(|c| lens(c, false)),
        lit: colours.map(|c| lens(c, true)),
    };
    // 1980s signal green-black, painted steel.
    let paint = materials.add(StandardMaterial {
        base_color: Color::srgb(0.07, 0.09, 0.07),
        perceptual_roughness: 0.55,
        metallic: 0.4,
        ..default()
    });
    let pole = meshes.add(Cylinder::new(POLE_M.1, POLE_M.0));
    let housing = meshes.add(Cuboid::from_size(HOUSING_M));
    let disc = meshes.add(Cylinder::new(LENS_R_M, 0.05));
    let range = VisibilityRange::abrupt(0.0, SHOWN_M);
    let mut heads = 0;
    for (ri, r) in g.roads.iter().enumerate() {
        for (end, &n) in r.nodes.iter().enumerate() {
            let node = &g.nodes[n as usize];
            if !node.signal {
                continue;
            }
            // the road's lanes into this junction carry its phase
            let Some(phase) = g.road_lanes().find(|l| l.road == ri as u32 && l.to_node == n).and_then(|l| l.phase) else {
                continue;
            };
            let pts = &r.line.points;
            let away = if end == 0 {
                pts[1] - pts[0]
            } else {
                pts[pts.len() - 2] - pts[pts.len() - 1]
            }
            .normalize_or_zero();
            // on the kerb to the right of the cars coming in, at the junction's mouth
            let at = node.at + away * node.radius_m + right_of(-away) * (r.width_m * 0.5 + KERB_OUT_M);
            let ground = plan.terrain.height_at(at.x, at.y);
            let facing = Vec3::new(away.x, 0.0, away.y);
            commands
                .spawn((
                    Name::new(format!("traffic light (block-out) junction {n} road {ri}")),
                    LightHead { node: n, phase },
                    Transform::from_translation(Vec3::new(at.x, ground, at.y)).looking_to(facing, Vec3::Y),
                    Visibility::default(),
                ))
                .with_children(|h| {
                    h.spawn((
                        Name::new("pole (block-out)"),
                        Mesh3d(pole.clone()),
                        MeshMaterial3d(paint.clone()),
                        Transform::from_xyz(0.0, POLE_M.0 * 0.5, 0.0),
                        range.clone(),
                    ));
                    h.spawn((
                        Name::new("housing (block-out)"),
                        Mesh3d(housing.clone()),
                        MeshMaterial3d(paint.clone()),
                        Transform::from_xyz(0.0, HOUSING_AT_M, 0.0),
                        range.clone(),
                    ));
                    // red at the top, amber, green at the bottom, on the face toward the cars (local -z)
                    for (k, shows) in [Light::Red, Light::Amber, Light::Green].into_iter().enumerate() {
                        h.spawn((
                            Name::new(format!("lens {shows:?} (block-out)")),
                            Lens(shows),
                            Mesh3d(disc.clone()),
                            MeshMaterial3d(looks.dark[index(shows)].clone()),
                            Transform::from_xyz(0.0, HOUSING_AT_M + LENS_STEP_M * (1.0 - k as f32), -HOUSING_M.z * 0.5)
                                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                            range.clone(),
                        ));
                    }
                });
            heads += 1;
        }
    }
    info!(
        "traffic lights: {heads} heads at {} junctions",
        g.nodes.iter().filter(|n| n.signal).count()
    );
    commands.insert_resource(looks);
}

/// Lights each head's lens for what its phase shows now, near the eye only; a lens's material changes only when
/// its state does.
fn show_lights(
    sim: Res<TrafficSim>,
    data: Res<Data>,
    looks: Option<Res<LensLooks>>,
    eye: Query<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    heads: Query<(&LightHead, &GlobalTransform, &Children)>,
    mut lenses: Query<(&Lens, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let (Some(looks), Some(eye)) = (looks, eye.iter().next()) else {
        return;
    };
    let eye = eye.translation();
    let times = &data.0.traffic.cfg.lights;
    let now = sim.traffic.time_s();
    for (head, at, children) in &heads {
        if at.translation().distance(eye) > SHOWN_M {
            continue;
        }
        let shows = light(times, head.node, head.phase, now);
        for c in children {
            let Ok((lens, mut mat)) = lenses.get_mut(*c) else { continue };
            let want = if lens.0 == shows { &looks.lit } else { &looks.dark }[index(lens.0)].clone();
            if mat.0 != want {
                mat.0 = want;
            }
        }
    }
}
