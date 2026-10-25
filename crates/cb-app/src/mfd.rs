//! The cockpit's MFDs (openspec changes `cockpit-mfds`, `weapon-select`, `cockpit-parity`): one UI camera
//! draws the mech's readouts (`hud`'s MFD pages) into one texture five pages wide, and each of the
//! cockpit's six screens (`HUD_mfd_N_screen` hung above the front screen, `SCREEN_left/center/right_screen`
//! on the dash) shows one page of it. With free look held, clicking while looking at a screen moves it to
//! its next page. A mech with no interior (the stand-in) keeps the pages along the bottom of the view.

use crate::hud::{MFD_PAGE_PX, MFD_PAGES as PAGES, MfdRoot};
use crate::input::InputAccum;
use crate::pilot::{CockpitCamera, PilotCamera, PilotSim};
use bevy::camera::RenderTarget;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiTargetCamera;
use cb_core::pilot::PilotMode;

/// The dash's lower screens, bound by exact name (they were baked pictures), and the page each starts on.
const DASH_SCREENS: [(&str, usize); 3] = [("SCREEN_left_screen", 0), ("SCREEN_center_screen", 4), ("SCREEN_right_screen", 3)];

/// The MFD texture and the camera that draws it.
#[derive(Resource)]
struct MfdTarget {
    image: Handle<Image>,
    camera: Entity,
}

/// A screen showing page `page` (0..PAGES) of the MFD texture through `material`.
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
            .add_systems(Update, (bind_screens, place_pages, press, outline, shoot).chain());
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

/// The page `page`'s slice of the texture, as a UV transform on a screen's 0..1 UVs.
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
    // Not a screen yet, and not a screen's outline (a mesh hung under the screen, `mfd-menus`).
    meshes: Query<(), (With<Mesh3d>, Without<MfdScreen>, Without<OutlineFrame>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, name) in &named {
        let page = match name.as_str().strip_prefix("HUD_mfd_").and_then(|r| r.strip_suffix("_screen")) {
            Some(rest) => {
                let Ok(page) = rest.parse::<usize>() else { continue };
                page
            }
            None => match DASH_SCREENS.iter().find(|(n, _)| *n == name.as_str()) {
                Some((_, page)) => *page,
                None => continue,
            },
        };
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

/// The screen the view's centre falls on, nearest first, within reach: the one the outline marks and a click
/// presses (openspec change `mfd-menus`), found by this one function for both.
fn pointed<'a>(ray: Ray3d, screens: impl Iterator<Item = (Entity, &'a Aabb, &'a GlobalTransform)>) -> Option<Entity> {
    screens
        .filter_map(|(e, aabb, at)| ray_hits_box(ray, aabb, at).map(|d| (d, e)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, e)| e)
}

/// Free look and fire pressed: the MFD screen the view's centre falls on moves to its next page.
fn press(
    mut acc: ResMut<InputAccum>,
    eye: Single<&GlobalTransform, With<PilotCamera>>,
    mut screens: Query<(Entity, &mut MfdScreen, &Aabb, &GlobalTransform)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !std::mem::take(&mut acc.mfd_press) {
        return;
    }
    let ray = Ray3d::new(eye.translation(), eye.forward());
    let Some(hit) = pointed(ray, screens.iter().map(|(e, _, aabb, at)| (e, aabb, at))) else {
        return;
    };
    if let Ok((_, mut screen, _, _)) = screens.get_mut(hit) {
        screen.page = (screen.page + 1) % PAGES;
        if let Some(mut m) = materials.get_mut(&screen.material) {
            m.uv_transform = page_uv(screen.page);
        }
    }
}

/// A screen's outline (`mfd-menus`): the frame entity, a child of the screen.
#[derive(Component)]
struct Outline(Entity);

/// The outline's frame mesh: never bound as a screen itself.
#[derive(Component)]
struct OutlineFrame;

/// The outline: amber, unlit, a little over white so the bloom gives it a soft glow (at four times white the
/// tonemapper washed it to cream).
const OUTLINE_RGB: [f32; 3] = [1.0, 0.62, 0.15];
const OUTLINE_GLOW: f32 = 1.4;
/// The frame's bars, metres: how far outside the screen's edge they start, how wide they are, and how far in front
/// of the screen toward the eye they lie. The dash's bezels stand about 2 cm proud of their glass, so the frame floats
/// 1 cm in front of them: never inside a bezel, never on a shared plane.
const OUTLINE_GAP_M: f32 = 0.008;
const OUTLINE_WIDTH_M: f32 = 0.012;
const OUTLINE_LIFT_M: f32 = 0.03;

/// A rectangular frame round a screen, in the screen's own frame, from the screen mesh's corner points (a flat
/// quad, tilted however its console tilts it), the eye's position in that frame (the side the frame lies on), and the
/// bars' gap, width and lift. The quad's first edge sets the frame's long axis, its plane the frame's plane.
fn frame_mesh(points: &[Vec3], eye: Vec3, gap: f32, width: f32, lift: f32) -> Option<Mesh> {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};
    let (p0, p1, p2) = (*points.first()?, *points.get(1)?, *points.get(2)?);
    let centre = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    let u = (p1 - p0).try_normalize()?;
    let mut n = (p1 - p0).cross(p2 - p0).try_normalize()?;
    if n.dot(eye - centre) < 0.0 {
        n = -n;
    }
    let v = n.cross(u);
    // Centred on the points' extents in the plane, so a quad written as six loose vertices is not pulled off centre.
    let span = |axis: Vec3| {
        let d = points.iter().map(|p| (*p - centre).dot(axis));
        let (lo, hi) = d.fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        ((lo + hi) * 0.5, (hi - lo) * 0.5)
    };
    let ((mid_u, half_u), (mid_v, half_v)) = (span(u), span(v));
    let plane = centre + u * mid_u + v * mid_v + n * lift;
    let (inner_u, inner_v) = (half_u + gap, half_v + gap);
    let (outer_u, outer_v) = (inner_u + width, inner_v + width);
    // Four bars as quads: top and bottom run the full outer width, the sides fill between them.
    let bars = [
        (-outer_u, outer_u, inner_v, outer_v),
        (-outer_u, outer_u, -outer_v, -inner_v),
        (-outer_u, -inner_u, -inner_v, inner_v),
        (inner_u, outer_u, -inner_v, inner_v),
    ];
    let (mut pos, mut idx) = (Vec::new(), Vec::new());
    for (u0, u1, v0, v1) in bars {
        let base = pos.len() as u32;
        for (a, b) in [(u0, v0), (u1, v0), (u1, v1), (u0, v1)] {
            pos.push((plane + u * a + v * b).to_array());
        }
        idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Some(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![n.to_array(); pos.len()])
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_indices(Indices::U32(idx)),
    )
}

/// A mesh's vertex positions.
fn positions(mesh: &Mesh) -> Vec<Vec3> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) => p.iter().map(|a| Vec3::from_array(*a)).collect(),
        _ => Vec::new(),
    }
}

/// In free look, seated, the screen under the view's centre shows a lit outline round it, so the pilot sees it can
/// be pressed (owner, 2026-10-25: "make them have some outline that shows the player can interact with them when
/// mouse over"). Each screen's frame is built the first time it is pointed at, from its own bounds.
#[allow(clippy::too_many_arguments)]
fn outline(
    mut commands: Commands,
    actions: Res<crate::input::StepActions>,
    pilot: Res<PilotSim>,
    eye: Single<&GlobalTransform, With<PilotCamera>>,
    screens: Query<(Entity, &Aabb, &GlobalTransform, &Mesh3d, Option<&Outline>, Option<&RenderLayers>), With<MfdScreen>>,
    mut frames: Query<(&mut Visibility, &mut RenderLayers), (With<OutlineFrame>, Without<MfdScreen>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material: Local<Option<Handle<StandardMaterial>>>,
    mut last: Local<Option<Entity>>,
) {
    let on = actions.free_look && pilot.curr.mode == PilotMode::Seated;
    let ray = Ray3d::new(eye.translation(), eye.forward());
    let hit = if on {
        pointed(ray, screens.iter().map(|(e, aabb, at, _, _, _)| (e, aabb, at)))
    } else {
        None
    };
    if hit != *last {
        debug!(
            "MFD outline on {hit:?} (free look {}, {} screens)",
            actions.free_look,
            screens.iter().count()
        );
        *last = hit;
    }
    for (e, _, at, screen_mesh, frame, layers) in &screens {
        let want = hit == Some(e);
        match frame {
            Some(Outline(f)) => {
                if let Ok((mut vis, mut l)) = frames.get_mut(*f) {
                    let v = if want { Visibility::Inherited } else { Visibility::Hidden };
                    if *vis != v {
                        *vis = v;
                    }
                    // On the screen's own layer: the cockpit pass while sealed, the world while open.
                    let screen_layers = layers.cloned().unwrap_or_default();
                    if *l != screen_layers {
                        *l = screen_layers;
                    }
                }
            }
            None if want => {
                let eye_local = at.affine().inverse().transform_point3(eye.translation());
                let corners = meshes.get(&screen_mesh.0).map(positions).unwrap_or_default();
                let Some(mesh) = frame_mesh(&corners, eye_local, OUTLINE_GAP_M, OUTLINE_WIDTH_M, OUTLINE_LIFT_M) else {
                    continue;
                };
                let mat = material
                    .get_or_insert_with(|| {
                        let [r, g, b] = OUTLINE_RGB;
                        materials.add(StandardMaterial {
                            base_color: Color::LinearRgba(LinearRgba::rgb(r, g, b) * OUTLINE_GLOW),
                            unlit: true,
                            cull_mode: None,
                            ..default()
                        })
                    })
                    .clone();
                let f = commands
                    .spawn((
                        Name::new("MFD outline"),
                        OutlineFrame,
                        // It follows its screen's layer itself; the cockpit pass would hide it as hull.
                        crate::mech::OwnLayers,
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(mat),
                        Transform::IDENTITY,
                        Visibility::Inherited,
                        layers.cloned().unwrap_or_default(),
                        bevy::light::NotShadowCaster,
                        ChildOf(e),
                    ))
                    .id();
                commands.entity(e).insert(Outline(f));
            }
            None => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_outline_lies_round_the_screen_on_the_eyes_side_never_on_it() {
        // A 0.30 m screen in the plane z = 0, the eye 0.6 m in front.
        let quad = [
            Vec3::new(-0.15, -0.15, 0.0),
            Vec3::new(0.15, -0.15, 0.0),
            Vec3::new(0.15, 0.15, 0.0),
            Vec3::new(-0.15, 0.15, 0.0),
        ];
        let m = frame_mesh(&quad, Vec3::new(0.0, 0.1, 0.6), 0.008, 0.012, 0.03).unwrap();
        let p = positions(&m);
        assert_eq!(p.len(), 16, "four bars of four corners");
        for q in &p {
            assert!((q.z - 0.03).abs() < 1e-6, "3 cm in front of the screen's face, toward the eye: {q}");
            assert!(
                q.x.abs() >= 0.158 - 1e-6 || q.y.abs() >= 0.158 - 1e-6,
                "outside the screen and its gap: {q}"
            );
            assert!(q.x.abs() <= 0.17 + 1e-6 && q.y.abs() <= 0.17 + 1e-6, "no wider than the bars: {q}");
        }
    }

    #[test]
    fn the_outline_follows_a_tilted_screen_and_faces_the_eye() {
        // A dash screen tilted back 18 degrees, the eye above and behind it on the -z side.
        let tilt = Quat::from_rotation_x(18f32.to_radians());
        let quad: Vec<Vec3> = [(-0.1, -0.1), (0.1, -0.1), (0.1, 0.1), (-0.1, 0.1)]
            .iter()
            .map(|&(x, y)| tilt * Vec3::new(x, y, 0.0))
            .collect();
        let normal = tilt * Vec3::NEG_Z;
        let m = frame_mesh(&quad, Vec3::new(0.0, 0.3, -0.6), 0.008, 0.012, 0.03).unwrap();
        for q in positions(&m) {
            assert!(
                (q.dot(normal) - 0.03).abs() < 1e-5,
                "in the screen's own plane, 3 cm toward the eye: {q}"
            );
        }
    }

    #[test]
    fn the_pointed_screen_is_the_nearest_the_ray_enters() {
        let near = (
            Aabb::from_min_max(Vec3::new(-0.2, -0.2, 0.0), Vec3::new(0.2, 0.2, 0.001)),
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
        );
        let far = (
            Aabb::from_min_max(Vec3::new(-0.2, -0.2, 0.0), Vec3::new(0.2, 0.2, 0.001)),
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -2.0)),
        );
        let (a, b) = (Entity::from_raw_u32(1).unwrap(), Entity::from_raw_u32(2).unwrap());
        let ray = Ray3d::new(Vec3::ZERO, Dir3::NEG_Z);
        let list = [(b, &far.0, &far.1), (a, &near.0, &near.1)];
        assert_eq!(pointed(ray, list.into_iter()), Some(a));
        let away = Ray3d::new(Vec3::ZERO, Dir3::Z);
        assert_eq!(pointed(away, list.into_iter()), None, "nothing behind the eye");
    }
}
