//! The cockpit's MFDs (openspec change `cockpit-mfds`): one UI camera draws the mech's readouts
//! (`hud`'s MFD pages) into one texture three pages wide, and each `HUD_mfd_N_screen` in the interior
//! shows a third of it. With free look held, clicking while looking at a screen moves it to its next
//! page. A cockpit with no MFD screens keeps the pages along the bottom of the pilot's view.

use crate::hud::{MFD_PAGE_PX, MfdRoot};
use crate::input::InputAccum;
use crate::pilot::{CockpitCamera, PilotCamera, PilotSim};
use bevy::camera::RenderTarget;
use bevy::camera::primitives::Aabb;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiTargetCamera;
use cb_core::pilot::PilotMode;

/// Pages on the texture, left to right.
const PAGES: usize = 3;

/// The MFD texture and the camera that draws it.
#[derive(Resource)]
struct MfdTarget {
    image: Handle<Image>,
    camera: Entity,
}

/// A screen showing page `page` (0..3) of the MFD texture through `material`.
#[derive(Component)]
struct MfdScreen {
    page: usize,
    material: Handle<StandardMaterial>,
}

/// Draws the MFDs.
pub struct MfdPlugin;

impl Plugin for MfdPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (bind_screens, place_pages, press, shoot).chain());
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let side = MFD_PAGE_PX as u32;
    let image = Image::new_target_texture(
        side * PAGES as u32,
        side,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    let image = images.add(image);
    let camera = commands
        .spawn((
            Name::new("MFD camera"),
            Camera2d,
            Camera {
                order: -20,
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.05, 0.04)),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
        ))
        .id();
    commands.insert_resource(MfdTarget { image, camera });
}

/// The page `page`'s third of the texture, as a UV transform on a screen's 0..1 UVs.
fn page_uv(page: usize) -> Affine2 {
    Affine2::from_scale_angle_translation(Vec2::new(1.0 / PAGES as f32, 1.0), 0.0, Vec2::new(page as f32 / PAGES as f32, 0.0))
}

/// Gives every `HUD_mfd_N_screen` mesh (and any mesh under a node of that name) its page of the
/// texture, once.
fn bind_screens(
    mut commands: Commands,
    target: Res<MfdTarget>,
    named: Query<(Entity, &Name)>,
    children: Query<&Children>,
    meshes: Query<(), (With<Mesh3d>, Without<MfdScreen>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, name) in &named {
        let Some(rest) = name.as_str().strip_prefix("HUD_mfd_").and_then(|r| r.strip_suffix("_screen")) else {
            continue;
        };
        let Ok(page) = rest.parse::<usize>() else { continue };
        let page = page.min(PAGES - 1);
        let mut found = Vec::new();
        if meshes.contains(e) {
            found.push(e);
        }
        found.extend(children.iter_descendants(e).filter(|c| meshes.contains(*c)));
        for m in found {
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(target.image.clone()),
                unlit: true,
                uv_transform: page_uv(page),
                ..default()
            });
            commands
                .entity(m)
                .insert((MeshMaterial3d(material.clone()), MfdScreen { page, material }));
        }
    }
}

/// The pages render into the texture while the pilot is seated in a cockpit with MFD screens; with
/// none, they sit along the bottom of the pilot's view.
fn place_pages(
    mut commands: Commands,
    target: Res<MfdTarget>,
    pilot: Res<PilotSim>,
    screens: Query<(), With<MfdScreen>>,
    overlay: Single<Entity, With<CockpitCamera>>,
    mut cameras: Query<&mut Camera>,
    root: Single<(Entity, &mut Node, Option<&UiTargetCamera>), With<MfdRoot>>,
) {
    let seated = pilot.curr.mode == PilotMode::Seated;
    let on_screens = !screens.is_empty();
    if let Ok(mut cam) = cameras.get_mut(target.camera)
        && cam.is_active != (seated && on_screens)
    {
        cam.is_active = seated && on_screens;
    }
    let (e, mut node, current) = root.into_inner();
    // Without screens, on the cockpit pass: it draws last, so the cockpit frame does not cover it.
    let want = if on_screens { target.camera } else { *overlay };
    if current.map(|c| c.entity()) != Some(want) {
        commands.entity(e).insert(UiTargetCamera(want));
        if on_screens {
            node.left = px(0.0);
            node.top = px(0.0);
            node.bottom = Val::Auto;
            node.margin = UiRect::ZERO;
        } else {
            // Along the bottom of the view, centred.
            node.left = Val::Percent(50.0);
            node.margin = UiRect::left(px(-MFD_PAGE_PX * PAGES as f32 * 0.5));
            node.top = Val::Auto;
            node.bottom = px(8.0);
        }
    }
}

/// Free look and fire pressed: the MFD screen the view's centre falls on moves to its next page.
fn press(
    mut acc: ResMut<InputAccum>,
    eye: Single<&GlobalTransform, With<PilotCamera>>,
    mut screens: Query<(&mut MfdScreen, &Aabb, &GlobalTransform)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !std::mem::take(&mut acc.mfd_press) {
        return;
    }
    let ray = Ray3d::new(eye.translation(), eye.forward());
    let mut nearest: Option<(f32, Mut<MfdScreen>)> = None;
    for (screen, aabb, at) in &mut screens {
        let Some(d) = ray_hits_box(ray, aabb, at) else { continue };
        if nearest.as_ref().is_none_or(|n| d < n.0) {
            nearest = Some((d, screen));
        }
    }
    if let Some((_, mut screen)) = nearest {
        screen.page = (screen.page + 1) % PAGES;
        if let Some(mut m) = materials.get_mut(&screen.material) {
            m.uv_transform = page_uv(screen.page);
        }
    }
}

/// World distance along `ray` to where it enters a box (`aabb` in the frame `at`), if within 3 m.
fn ray_hits_box(ray: Ray3d, aabb: &Aabb, at: &GlobalTransform) -> Option<f32> {
    let inv = at.affine().inverse();
    let o = inv.transform_point3(ray.origin);
    let d = inv.transform_vector3(*ray.direction);
    // Thin screens: a few millimetres of depth so a flat face can be hit.
    let pad = Vec3::splat(0.003);
    let (lo, hi) = (Vec3::from(aabb.min()) - pad, Vec3::from(aabb.max()) + pad);
    let (mut t0, mut t1) = (f32::NEG_INFINITY, f32::INFINITY);
    for i in 0..3 {
        if d[i].abs() < 1e-8 {
            if o[i] < lo[i] || o[i] > hi[i] {
                return None;
            }
            continue;
        }
        let (a, b) = ((lo[i] - o[i]) / d[i], (hi[i] - o[i]) / d[i]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    if t1 < t0.max(0.0) {
        return None;
    }
    let hit = at.transform_point(o + d * t0.max(0.0));
    let w = ray.origin.distance(hit);
    (w < 3.0).then_some(w)
}

/// `CB_MFD_SHOT=1` (a check, not for play): the MFD texture saved to `output/mfd.png` 40 s in, to see
/// the pages as drawn.
fn shoot(mut commands: Commands, target: Res<MfdTarget>, time: Res<Time<Real>>, mut done: Local<bool>) {
    if *done || time.elapsed_secs() < 40.0 || !crate::cli::switched_off("CB_MFD_SHOT") {
        return;
    }
    *done = true;
    let path = crate::cli::repo_root().join("output/mfd.png");
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::image(target.image.clone()))
        .observe(bevy::render::view::screenshot::save_to_disk(path));
}
