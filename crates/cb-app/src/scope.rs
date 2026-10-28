//! The railgun's scope (openspec change `railgun-scope`; survey G24, owner, 2026-10-26: a magnified window in the
//! middle of the front screen, at 4 and 10 times; the pilot's own eyes never zoom). While Scope is held with a scoped
//! weapon on board, a third world camera looks down the aim ray from the sensor head (`CAM_main`, else the pilot's
//! eye) at the magnified field of view into an 896 x 640 picture, and that picture is a pane on the cockpit layer laid
//! on the front screen where the torso's forward line meets the screen wall. The sight is drawn into the picture from
//! the simulation's own numbers: the range from `cb_core::weapons::trace` (the laser's trace), the drop mark from
//! `weapons::lead` with drop (the function the slug flies by), the rail's charge and the weapon heat bank from
//! `Guns`. With the head shot off the window shows `NO CAM` and closes. The view stays the player's: scoped, the mouse
//! turns it one over the magnification as far (`input`), and nothing here turns it.

use crate::input::{InputAccum, StepActions};
use crate::mech::{CabinGlass, MechRig, MechSim, MechSpec, Piloted};
use crate::pilot::{COCKPIT_LAYER, PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiTargetCamera;
use cb_core::mech::PartKind;
use cb_core::pilot::PilotMode;
use cb_core::weapons::{PartCapsule, ShotKind};
use cb_core::world::Scene;

/// The scope picture, pixels (design section 1).
const PICTURE_PX: UVec2 = UVec2::new(896, 640);
/// The window's width as an angle at the pilot's eye, degrees: about 0.42 of a front screen seen from the seat.
const WINDOW_DEG: f32 = 30.0;
/// The window stands this far in front of the glass, toward the eye, metres (no z-fighting with the screen).
const OFF_GLASS_M: f32 = 0.01;
/// The rangefinder's reach, metres.
const RANGE_M: f32 = 4000.0;

/// The scope's state this frame.
#[derive(Resource, Default)]
pub struct Scope {
    /// The window is open: Scope held, seated, a scoped weapon on board, the head on.
    pub open: bool,
    /// The magnification it shows.
    pub zoom: f32,
    /// Which of the weapon's magnifications is chosen.
    pub step: usize,
}

#[derive(Component)]
struct ScopeCamera;

#[derive(Component)]
struct ScopeOverlay;

#[derive(Component)]
struct ScopeWindow;

#[derive(Component)]
enum Sight {
    Range,
    State,
    DropMark,
    NoCam,
}

/// Opens, aims and draws the scope.
pub struct ScopePlugin;

impl Plugin for ScopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Scope>()
            .add_world_spawn(setup)
            .add_game_systems(Update, (open, aim, draw_sight).chain().after(PresentSet::Camera));
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let image = images.add(Image::new_target_texture(
        PICTURE_PX.x,
        PICTURE_PX.y,
        TextureFormat::Rgba16Float,
        None,
    ));
    commands.spawn((
        Name::new("scope camera"),
        ScopeCamera,
        Camera3d::default(),
        Camera {
            order: -30,
            is_active: false,
            ..default()
        },
        RenderTarget::Image(image.clone().into()),
        Projection::from(PerspectiveProjection {
            fov: 0.5,
            aspect_ratio: PICTURE_PX.x as f32 / PICTURE_PX.y as f32,
            near: 1.0,
            ..default()
        }),
        Transform::default(),
    ));
    // The sight, drawn over the picture by a UI camera into the same texture.
    let overlay = commands
        .spawn((
            Name::new("scope sight"),
            ScopeOverlay,
            Camera2d,
            Camera {
                order: -29,
                is_active: false,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
        ))
        .id();
    let amber = Color::srgb(1.0, 0.72, 0.2);
    let line = |w: f32, h: f32, l: f32, t: f32| {
        (
            Node {
                position_type: PositionType::Absolute,
                left: percent(l),
                top: percent(t),
                width: px(w),
                height: px(h),
                ..default()
            },
            BackgroundColor(amber),
        )
    };
    let text = |s: Sight, l: f32, t: f32| {
        (
            s,
            Text::new(""),
            TextFont::from_font_size(22.0),
            TextColor(amber),
            Node {
                position_type: PositionType::Absolute,
                left: percent(l),
                top: percent(t),
                ..default()
            },
        )
    };
    commands
        .spawn((
            UiTargetCamera(overlay),
            Node {
                width: percent(100.0),
                height: percent(100.0),
                border: UiRect::all(px(6.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.08, 0.08, 0.09)),
        ))
        .with_children(|p| {
            // The centre reticle: the aim ray.
            p.spawn(line(120.0, 2.0, 43.3, 50.0));
            p.spawn(line(2.0, 120.0, 50.0, 40.6));
            p.spawn(text(Sight::Range, 3.0, 4.0));
            p.spawn(text(Sight::State, 3.0, 88.0));
            p.spawn(text(Sight::NoCam, 44.0, 46.0));
            p.spawn((
                Sight::DropMark,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(16.0),
                    height: px(16.0),
                    border: UiRect::all(px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(1.0, 0.2, 0.15)),
                Visibility::Hidden,
            ));
        });
    // The window pane on the front screen, on the cockpit layer, lit by nothing (like the screens).
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(image),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Name::new("scope window"),
        ScopeWindow,
        Mesh3d(meshes.add(Rectangle::new(1.0, PICTURE_PX.y as f32 / PICTURE_PX.x as f32))),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(COCKPIT_LAYER),
        bevy::light::NotShadowCaster,
    ));
}

/// Opens the window while Scope is held with a scoped weapon on board, the pilot seated and the head on; steps the
/// magnification.
fn open(
    actions: Res<StepActions>,
    pilot: Res<PilotSim>,
    mut acc: ResMut<InputAccum>,
    me: Query<(&MechSim, &MechSpec), With<Piloted>>,
    mut scope: ResMut<Scope>,
) {
    let steps = std::mem::take(&mut acc.scope_zoom);
    let Some((sim, spec)) = me.iter().next() else {
        *scope = Scope::default();
        return;
    };
    let zooms = cb_core::scope::zooms(&spec.weapons, sim.selected);
    if zooms.is_empty() {
        *scope = Scope::default();
        return;
    }
    let head = sim.damage.is_lost(&spec.parts, PartKind::Head);
    scope.open = actions.scope && pilot.curr.mode == PilotMode::Seated && !head;
    // The wheel and B step the magnification only while the window is open: the wheel is the weapon
    // selector seated and unscoped, and the interaction selector out of the seat (survey W40).
    if steps != 0 && scope.open {
        scope.step = (scope.step as i64 + i64::from(steps)).rem_euclid(zooms.len() as i64) as usize;
    }
    scope.step = scope.step.min(zooms.len() - 1);
    scope.zoom = zooms[scope.step];
}

/// Points the scope camera down the aim ray and lays the window on the front screen.
#[allow(clippy::type_complexity)]
fn aim(
    scope: Res<Scope>,
    data: Res<Data>,
    settings: Res<crate::settings::Settings>,
    eye: Single<&Transform, (With<PilotCamera>, Without<ScopeCamera>, Without<ScopeWindow>)>,
    me: Query<(&MechSim, Option<&MechRig>), With<Piloted>>,
    globals: Query<&GlobalTransform>,
    glass: Query<(&CabinGlass, &GlobalTransform)>,
    mut camera: Single<(&mut Camera, &mut Transform, &mut Projection), (With<ScopeCamera>, Without<ScopeWindow>)>,
    mut overlay: Single<&mut Camera, (With<ScopeOverlay>, Without<ScopeCamera>)>,
    mut window: Single<(&mut Transform, &mut Visibility), (With<ScopeWindow>, Without<ScopeCamera>, Without<PilotCamera>)>,
    frame: Res<crate::cockpit_space::CockpitFrame>,
) {
    let (cam, at, lens) = &mut *camera;
    cam.is_active = scope.open;
    overlay.is_active = scope.open;
    *window.1 = if scope.open { Visibility::Inherited } else { Visibility::Hidden };
    if !scope.open {
        return;
    }
    let Some((sim, rig)) = me.iter().next() else { return };
    // From the sensor head's camera when the model has one, along the pilot's aim (the ray the guns fire along).
    let from = rig
        .and_then(|r| r.named("CAM_main"))
        .and_then(|e| globals.get(e).ok())
        .map_or(eye.translation, |g| g.translation());
    **at = Transform::from_translation(from).with_rotation(eye.rotation);
    // Seated (blend 1), so the on-foot measure and the screen's shape play no part.
    let view = cb_core::pilot::fov_deg(&data.0.pilot, settings.0.fov_deg, 1.0, 16.0 / 9.0).to_radians();
    if let Projection::Perspective(p) = lens.as_mut() {
        p.fov = cb_core::scope::fov_rad(view, scope.zoom);
    }
    // The front screen: where the torso's forward line from the eye meets the screen wall.
    let ahead = cb_core::mech::forward(sim.curr.heading_rad + sim.curr.twist_rad);
    let o = eye.translation;
    let hit = glass
        .iter()
        .filter(|(_, g)| g.translation().distance(o) < 20.0)
        .flat_map(|(c, g)| c.0.iter().map(move |t| t.map(|p| g.transform_point(p))))
        .filter_map(|t| ray_triangle(o, ahead, t).map(|d| (d, t)))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let (dist, normal) = match hit {
        Some((d, t)) => (d - OFF_GLASS_M, (t[1] - t[0]).cross(t[2] - t[0]).normalize_or(-ahead)),
        None => (1.2, -ahead),
    };
    let centre = o + ahead * dist.max(0.2);
    let facing = if normal.dot(o - centre) > 0.0 { normal } else { -normal };
    let width = 2.0 * dist.max(0.2) * (WINDOW_DEG.to_radians() * 0.5).tan();
    let world = Transform::from_translation(centre)
        .looking_to(-facing, Vec3::Y)
        .with_scale(Vec3::splat(width));
    // In the cockpit pass's own frame when it draws in the mech's (`cockpit_space`).
    *window.0 = frame.local(world).unwrap_or(world);
}

/// The distance along the unit ray from `o` along `d` to the triangle `t`, if it meets it (Moller-Trumbore).
fn ray_triangle(o: Vec3, d: Vec3, t: [Vec3; 3]) -> Option<f32> {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let s = o - t[0];
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = d.dot(q) / det;
    let dist = e2.dot(q) / det;
    (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && dist > 0.0).then_some(dist)
}

/// The sight's numbers, from the simulation: range, drop mark, charge, heat.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_sight(
    scope: Res<Scope>,
    plan: Res<Plan>,
    live: Res<Live>,
    hostiles: Res<crate::units::Hostiles>,
    targeting: Res<crate::targeting::TargetState>,
    me: Query<(&MechSim, &MechSpec), With<Piloted>>,
    camera: Single<(&Camera, &Transform), With<ScopeCamera>>,
    mut texts: Query<(&Sight, &mut Text)>,
    mut mark: Query<(&Sight, &mut Node, &mut Visibility), Without<Text>>,
) {
    let Some((sim, spec)) = me.iter().next() else { return };
    let head = sim.damage.is_lost(&spec.parts, PartKind::Head);
    let (cam, at) = *camera;
    let g = GlobalTransform::from(*at);
    let (o, d) = (at.translation, at.forward().as_vec3());
    let range = scope.open.then(|| {
        // The rangefinder: the laser's trace along the aim, against the world and the hostiles' bodies.
        let parts: Vec<PartCapsule> = hostiles
            .0
            .iter()
            .map(|h| PartCapsule {
                mech: h.id,
                part: 0,
                a: h.feet,
                b: h.feet + Vec3::Y * h.height_m,
                radius_m: (h.height_m * 0.15).max(1.5),
            })
            .collect();
        let scene = Scene {
            plan: &plan,
            dug: &live.dug,
            trees: None,
            structures: Some(&live.structures),
            platforms: &[],
            interiors: None,
        };
        cb_core::weapons::trace(o, d, RANGE_M, None, &scene, &live.structures, &live.forest, &parts).map(|h| o.distance(h.1))
    });
    let scoped = spec
        .weapons
        .iter()
        .enumerate()
        .find(|(i, w)| *i == sim.selected && !w.scope_zoom.is_empty())
        .or_else(|| spec.weapons.iter().enumerate().find(|(_, w)| !w.scope_zoom.is_empty()));
    let state = scoped.map_or_else(String::new, |(i, w)| {
        let charge = match (w.shot(), sim.guns.charge_s.get(i)) {
            (ShotKind::Slug { charge_s }, Some(c)) if *c > 0.0 => format!("CHARGE {:.0}%   ", (c / charge_s * 100.0).min(100.0)),
            _ => String::new(),
        };
        format!(
            "{}{}X   {charge}HEAT {:.0}%",
            w.name.to_uppercase() + "   ",
            scope.zoom,
            sim.guns.heat.heat * 100.0
        )
    });
    for (s, mut t) in &mut texts {
        t.0 = match s {
            Sight::Range => range.flatten().map_or_else(|| "---- M".to_string(), |r| format!("{r:.0} M")),
            Sight::State => state.clone(),
            Sight::NoCam => {
                if head {
                    "NO CAM".to_string()
                } else {
                    String::new()
                }
            }
            Sight::DropMark => continue,
        };
    }
    // The drop mark: where to put the reticle for the slug to strike the selected target.
    let drop = scoped.and_then(|(_, w)| {
        let id = targeting.selected?;
        let h = hostiles.0.iter().find(|h| h.id == id)?;
        let p = cb_core::weapons::lead(o, h.aim, h.vel, w.shot().lead_speed_m_s(w.speed_m_s), !w.shot().straight());
        cam.world_to_viewport(&g, p).ok()
    });
    for (s, mut node, mut vis) in &mut mark {
        if !matches!(s, Sight::DropMark) {
            continue;
        }
        match drop.filter(|_| scope.open) {
            Some(p) => {
                node.left = px(p.x - 8.0);
                node.top = px(p.y - 8.0);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A system whose queries overlap panics when Bevy first initialises it (error B0001), in the game, not at
    /// compile time: initialising each here proves their access is disjoint without running the game.
    #[test]
    fn the_scope_systems_queries_do_not_overlap() {
        let mut world = World::new();
        IntoSystem::into_system(open).initialize(&mut world);
        IntoSystem::into_system(aim).initialize(&mut world);
        IntoSystem::into_system(draw_sight).initialize(&mut world);
    }

    #[test]
    fn a_ray_meets_a_pane_ahead_and_misses_one_beside() {
        let pane = [Vec3::new(-1.0, -1.0, -2.0), Vec3::new(1.0, -1.0, -2.0), Vec3::new(0.0, 1.0, -2.0)];
        assert!((ray_triangle(Vec3::ZERO, Vec3::NEG_Z, pane).unwrap() - 2.0).abs() < 1e-5);
        assert_eq!(ray_triangle(Vec3::new(3.0, 0.0, 0.0), Vec3::NEG_Z, pane), None);
        assert_eq!(ray_triangle(Vec3::ZERO, Vec3::Z, pane), None, "behind");
    }
}
