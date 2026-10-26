//! The cockpit's MFDs (openspec changes `cockpit-mfds`, `weapon-select`, `cockpit-parity`): one UI camera
//! draws the mech's readouts (`hud`'s MFD pages) into one texture five pages wide, and each of the
//! cockpit's six screens (`HUD_mfd_N_screen` hung above the front screen, `SCREEN_left/center/right_screen`
//! on the dash) shows one page of it. With free look held, the screen under the view's centre is outlined, and a
//! click on a readout opens that screen's MENU (`mfd_apps`, openspec change `mfd-menus`); on MENU or MAIL the click
//! presses the row under the view's centre, found on the page itself (`ray_uv`), and BACK gives the screen back the
//! page it showed before. Letting go of free look, or leaving the seat, ends MFD focus and puts every screen back on
//! its default page (survey C14, `cb_core::mfd_focus`). A mech with no interior (the stand-in) keeps the pages along
//! the bottom of the view.

use crate::hud::{MFD_PAGE_PX, MFD_PAGES as PAGES, MFD_READOUTS, MfdRoot};
use crate::input::InputAccum;
use crate::mfd_apps::{Item, MAIL_PAGE, MENU_PAGE, Offer, Pointed, item_at, offers};
use crate::pilot::{CockpitCamera, PilotCamera, PilotSim};
use crate::state::GameApp;
use bevy::camera::RenderTarget;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiTargetCamera;
use cb_core::mfd_focus::{ScreenPages, focus_left, focused};
use cb_core::pilot::PilotMode;

/// The dash's lower screens, bound by exact name (they were baked pictures), and the page each starts on: DAMAGE,
/// TARGET, WEAPONS, so with the hung screens' MECH, FLIGHT and MISSION the six start on six different pages (owner,
/// 2026-10-26: "The two far left mfds are the same. I said to make them different screens").
const DASH_SCREENS: [(&str, usize); 3] = [("SCREEN_left_screen", 5), ("SCREEN_center_screen", 4), ("SCREEN_right_screen", 3)];

/// The hung screens' pages at start: `HUD_mfd_N_screen` shows page N.
const HUNG_PAGES: [usize; 3] = [0, 1, 2];

/// The MFD texture and the camera that draws it.
#[derive(Resource)]
struct MfdTarget {
    image: Handle<Image>,
    camera: Entity,
}

/// A screen showing a page (0..PAGES) of the MFD texture through `material`. `pages` holds the page it is built
/// with, the page it shows, and the readout it showed when its menu was opened, which BACK gives back (owner,
/// 2026-10-25: "clicking back should return to mech view and restore the mfd to its original screen"); all of it
/// lasts only while MFD focus does (survey C14).
#[derive(Component)]
pub(crate) struct MfdScreen {
    /// The page it shows and the one it is built with.
    pub(crate) pages: ScreenPages,
    material: Handle<StandardMaterial>,
}

/// Draws the MFDs.
pub struct MfdPlugin;

impl Plugin for MfdPlugin {
    fn build(&self, app: &mut App) {
        app.add_world_spawn(setup).add_game_systems(
            Update,
            (bind_screens, place_pages, point, press, leave_focus, outline, shoot).chain(),
        );
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
                let Ok(n) = rest.parse::<usize>() else { continue };
                HUNG_PAGES.get(n).copied().unwrap_or(n)
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
            commands.entity(m).insert((
                MeshMaterial3d(material.clone()),
                MfdScreen {
                    pages: ScreenPages::new(page),
                    material,
                },
            ));
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

/// Where the view's centre falls on the MFDs this frame, seated with free look held: the screen (`pointed`), the page
/// it shows, and the row or button there (`ray_uv` on the screen's own mesh, then `mfd_apps::item_at`).
#[allow(clippy::type_complexity)]
fn point(
    actions: Res<crate::input::StepActions>,
    pilot: Res<PilotSim>,
    eye: Single<&GlobalTransform, With<PilotCamera>>,
    screens: Query<(Entity, &MfdScreen, &Aabb, &GlobalTransform, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
    mut out: ResMut<Pointed>,
) {
    let mut now = Pointed::default();
    if actions.free_look && pilot.curr.mode == PilotMode::Seated {
        let ray = Ray3d::new(eye.translation(), eye.forward());
        if let Some(e) = pointed(ray, screens.iter().map(|(e, _, aabb, at, _)| (e, aabb, at)))
            && let Ok((_, screen, _, at, mesh)) = screens.get(e)
        {
            now.screen = Some(e);
            now.page = Some(screen.pages.page);
            let inv = at.affine().inverse();
            let uv = meshes.get(&mesh.0).and_then(|m| {
                ray_uv(
                    inv.transform_point3(ray.origin),
                    inv.transform_vector3(*ray.direction),
                    &positions(m),
                    &uvs(m),
                    &indices(m),
                )
            });
            now.at = uv.map(|uv| uv * MFD_PAGE_PX);
            now.item = now.at.and_then(|p| item_at(screen.pages.page, p));
        }
    }
    if *out != now {
        *out = now;
    }
}

/// Free look and fire pressed on a screen: a readout opens the screen's MENU (remembering the readout); on MENU or
/// MAIL the pointed row acts: a readout or MAIL shows that page; a slot's main button ACCEPTs a career's job (its
/// events go to the career, as the menu's Accept sends them) or STARTs or STOPs a free-roam mission (through
/// `FreeRoamRun`, as the menu's buttons do); DECLINE turns a job down; BACK gives the screen back its readout.
#[allow(clippy::too_many_arguments)]
fn press(
    mut acc: ResMut<InputAccum>,
    pointed: Res<Pointed>,
    mut screens: Query<&mut MfdScreen>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut career: Option<ResMut<crate::career::CareerRun>>,
    (mut free, data): (Option<ResMut<crate::missions::FreeRoamRun>>, Res<crate::sim::Data>),
    mut sounds: MessageWriter<crate::audio::Sound>,
) {
    let back = std::mem::take(&mut acc.mfd_back);
    if !std::mem::take(&mut acc.mfd_press) && !back {
        return;
    }
    let Some(mut screen) = pointed.screen.and_then(|e| screens.get_mut(e).ok()) else {
        return;
    };
    let page = screen.pages.page;
    let item = pointed.item;
    let open = offers(career.as_deref().map(|c| &c.career.board), free.as_deref().map(|f| &f.free));
    let next = match item {
        // Escape is BACK wherever the view points on the page (`turn_page`).
        _ if back || page < MFD_READOUTS || matches!(item, Some(Item::Back | Item::Readout(_) | Item::Mail)) => {
            let (next, home) = turn_page(page, screen.pages.home, item, back);
            screen.pages.home = home;
            next
        }
        Some(Item::Accept(k)) => {
            match open.get(k) {
                Some(Offer::Job(id)) => {
                    if let Some(run) = career.as_mut() {
                        let events = run.career.board.accept(*id);
                        run.pending.extend(events);
                        info!("MFD: job {id} accepted on the MAIL page");
                        for m in run.career.board.inbox.iter_mut().filter(|m| m.job == Some(*id)) {
                            m.read = true;
                        }
                        sounds.write(crate::audio::Sound::speaker("mfd_click"));
                    }
                }
                Some(Offer::Mission(i)) => {
                    if let (Some(f), Some(m)) = (free.as_mut(), data.0.missions.mail.get(*i)) {
                        if f.free.active == Some(m.mission) {
                            info!("MFD: mission {:?} stopped on the MAIL page", m.mission);
                            f.stop();
                        } else {
                            info!("MFD: mission {:?} started on the MAIL page", m.mission);
                            f.start(m.mission);
                        }
                        sounds.write(crate::audio::Sound::speaker("mfd_click"));
                    }
                }
                None => {}
            }
            None
        }
        Some(Item::Decline(k)) => {
            if let (Some(Offer::Job(id)), Some(run)) = (open.get(k), career.as_mut())
                && run.career.board.decline(*id)
            {
                info!("MFD: job {id} declined on the MAIL page");
                sounds.write(crate::audio::Sound::speaker("mfd_click"));
            }
            None
        }
        _ => None,
    };
    if let Some(p) = next {
        sounds.write(crate::audio::Sound::speaker("mfd_click"));
        screen.pages.page = p.min(PAGES - 1);
        if let Some(mut m) = materials.get_mut(&screen.material) {
            m.uv_transform = page_uv(screen.pages.page);
        }
    }
}

/// MFD focus ended this frame (free look let go, or the pilot out of the seat): every screen goes back to its default
/// page and forgets its menu (owner, 2026-10-26, survey C14: "mfds return to default as soon as you click back on
/// them, leaving mfd focus"). Nothing is kept between focuses, so nothing is kept between sorties either.
fn leave_focus(
    actions: Res<crate::input::StepActions>,
    pilot: Res<PilotSim>,
    mut screens: Query<&mut MfdScreen>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut was: Local<bool>,
) {
    let now = focused(pilot.curr.mode == PilotMode::Seated, actions.free_look);
    if focus_left(*was, now) {
        for mut screen in &mut screens {
            if screen.pages.leave_focus()
                && let Some(mut m) = materials.get_mut(&screen.material)
            {
                m.uv_transform = page_uv(screen.pages.page);
            }
        }
    }
    *was = now;
}

/// Which page a screen turns to, and the readout it remembers (`home`), when it is pressed on `item` while showing
/// `page`, or BACK'd by Escape (`escape`): a readout opens the MENU and remembers itself; BACK gives the remembered
/// readout back (the MECH page when none is remembered); a readout row shows that readout and forgets the old one;
/// MAIL keeps it. Escape on a readout does nothing. None: the page stays.
fn turn_page(page: usize, home: Option<usize>, item: Option<Item>, escape: bool) -> (Option<usize>, Option<usize>) {
    if page < MFD_READOUTS {
        return if escape { (None, home) } else { (Some(MENU_PAGE), Some(page)) };
    }
    match if escape { Some(Item::Back) } else { item } {
        Some(Item::Back) => (Some(home.unwrap_or(0)), None),
        Some(Item::Readout(i)) => (Some(i), None),
        Some(Item::Mail) => (Some(MAIL_PAGE), home),
        _ => (None, home),
    }
}

/// Where a ray (origin `o`, direction `d`, in the mesh's frame) first crosses the triangles `idx` of `pos`, as the
/// mesh's own UV there (`uv`, barycentric): on a screen, the point on its page, 0 to 1 each way from the top left.
fn ray_uv(o: Vec3, d: Vec3, pos: &[Vec3], uv: &[Vec2], idx: &[u32]) -> Option<Vec2> {
    let mut best: Option<(f32, Vec2)> = None;
    for t in idx.as_chunks::<3>().0 {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        let (Some(&p0), Some(&p1), Some(&p2)) = (pos.get(a), pos.get(b), pos.get(c)) else {
            continue;
        };
        let (Some(&u0), Some(&u1), Some(&u2)) = (uv.get(a), uv.get(b), uv.get(c)) else {
            continue;
        };
        // Moller-Trumbore, both faces.
        let (e1, e2) = (p1 - p0, p2 - p0);
        let h = d.cross(e2);
        let det = e1.dot(h);
        if det.abs() < 1e-9 {
            continue;
        }
        let s = o - p0;
        let bu = s.dot(h) / det;
        let q = s.cross(e1);
        let bv = d.dot(q) / det;
        let dist = e2.dot(q) / det;
        if !(0.0..=1.0).contains(&bu) || bv < 0.0 || bu + bv > 1.0 || dist < 0.0 {
            continue;
        }
        if best.is_none_or(|(bd, _)| dist < bd) {
            best = Some((dist, u0 * (1.0 - bu - bv) + u1 * bu + u2 * bv));
        }
    }
    best.map(|(_, p)| p)
}

/// A mesh's first UVs.
fn uvs(mesh: &Mesh) -> Vec<Vec2> {
    match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(p)) => p.iter().map(|a| Vec2::from_array(*a)).collect(),
        _ => Vec::new(),
    }
}

/// A mesh's triangle indices (0, 1, 2, ... when it has none).
fn indices(mesh: &Mesh) -> Vec<u32> {
    match mesh.indices() {
        Some(i) => i.iter().map(|v| v as u32).collect(),
        None => (0..mesh.count_vertices() as u32).collect(),
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
    #[test]
    fn the_six_screens_start_on_six_different_readouts() {
        let mut pages: Vec<usize> = super::HUNG_PAGES.to_vec();
        pages.extend(super::DASH_SCREENS.iter().map(|(_, p)| *p));
        let mut sorted = pages.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 6, "{pages:?}");
        assert!(pages.iter().all(|p| *p < crate::hud::MFD_READOUTS), "{pages:?}");
    }

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

    /// A 2 m quad in z = 0 whose UVs run 0 to 1 from its top left, as a screen's do.
    fn quad() -> (Vec<Vec3>, Vec<Vec2>, Vec<u32>) {
        (
            vec![
                Vec3::new(-1.0, -1.0, 0.0),
                Vec3::new(1.0, -1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(-1.0, 1.0, 0.0),
            ],
            vec![Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, 0.0)],
            vec![0, 1, 2, 0, 2, 3],
        )
    }

    /// One screen's pages as the pilot clicks through them: (page, remembered readout).
    fn click(state: (usize, Option<usize>), item: Option<Item>, escape: bool) -> (usize, Option<usize>) {
        let (next, home) = turn_page(state.0, state.1, item, escape);
        (next.unwrap_or(state.0), home)
    }

    #[test]
    fn leaving_the_mail_by_back_gives_the_screen_its_own_page_again() {
        // MECH (0): click opens the MENU, MAIL, then BACK.
        let s = click((0, None), None, false);
        assert_eq!(s, (MENU_PAGE, Some(0)));
        let s = click(s, Some(Item::Mail), false);
        assert_eq!(s, (MAIL_PAGE, Some(0)));
        assert_eq!(click(s, Some(Item::Back), false), (0, None));
    }

    #[test]
    fn escape_on_a_menu_is_back_and_on_a_readout_does_nothing() {
        let s = click((4, None), None, false);
        assert_eq!(click(s, None, true), (4, None), "Escape anywhere on the MENU: TARGET again");
        assert_eq!(click((4, None), None, true), (4, None), "Escape on a readout: nothing");
    }

    #[test]
    fn picking_a_readout_makes_it_the_screens_page() {
        let s = click((4, None), None, false);
        let s = click(s, Some(Item::Readout(1)), false);
        assert_eq!(s, (1, None));
        assert_eq!(
            click(click(s, None, false), Some(Item::Back), false),
            (1, None),
            "BACK now gives FLIGHT"
        );
    }

    #[test]
    fn a_ray_through_a_screens_middle_lands_mid_page() {
        let (p, uv, i) = quad();
        let hit = ray_uv(Vec3::new(0.0, 0.0, 2.0), Vec3::NEG_Z, &p, &uv, &i).unwrap();
        assert!((hit - Vec2::splat(0.5)).length() < 1e-5, "{hit}");
    }

    #[test]
    fn a_ray_near_a_screens_top_left_lands_near_the_pages_top_left() {
        let (p, uv, i) = quad();
        let hit = ray_uv(Vec3::new(-0.9, 0.9, 2.0), Vec3::NEG_Z, &p, &uv, &i).unwrap();
        assert!((hit - Vec2::new(0.05, 0.05)).length() < 1e-5, "{hit}");
        assert!(
            ray_uv(Vec3::new(1.5, 0.0, 2.0), Vec3::NEG_Z, &p, &uv, &i).is_none(),
            "off the screen: nothing"
        );
        assert!(
            ray_uv(Vec3::new(0.0, 0.0, 2.0), Vec3::Z, &p, &uv, &i).is_none(),
            "looking away: nothing"
        );
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
