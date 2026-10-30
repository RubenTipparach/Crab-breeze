//! Waypoint navigation in the game (openspec change `waypoint-nav`; owner, 2026-10-30: "we need to add a waypoint nav
//! system in the pause menu. Add a map and let me plop waypoints down to navigate to"): the island's map image, the
//! pause menu's MAP page (zoom, pan, put down and take off waypoints, the route's list), the marker over the active
//! waypoint seated and on foot, and the chime when it is reached. The route and its rule are `cb_core::nav`'s and the
//! image `cb_core::world::map::paint`'s; this adapts them to Bevy.

use crate::menu::{DIM, INK, Menu};
use crate::sim::Data;
use crate::state::GameApp;
use crate::world::Plan;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use bevy::ui::RelativeCursorPosition;
use cb_core::nav::{Route, Step, distance_label};

/// The waypoints' colour on the map and over the world.
const WAYPOINT: Color = Color::srgb(0.45, 0.9, 1.0);
/// The mission's mark, gold as on the page the owner approved.
const MISSION: Color = Color::srgb(1.0, 0.82, 0.25);
/// The map's side on the page, pixels.
const MAP_PX: f32 = 760.0;

pub struct NavPlugin;

impl Plugin for NavPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavRoute>()
            .init_resource::<MapImage>()
            .init_resource::<MapView>()
            .add_world_spawn(spawn_marker)
            .add_game_systems(
                Update,
                (paint_map, map_input, draw_page, step_route, draw_marker)
                    .chain()
                    .in_set(crate::sim::PresentSet::Camera),
            );
    }
}

/// The pilot's route (`cb_core::nav::Route`).
#[derive(Resource, Default)]
pub struct NavRoute(pub Route);

/// The island's map image, painted once on the pool when the world is there.
#[derive(Resource, Default)]
struct MapImage {
    handle: Option<Handle<Image>>,
    task: Option<Task<Vec<[u8; 4]>>>,
}

/// What the MAP page shows: the zoom, the image point at the page's middle (0..1 a side), and a press in progress.
#[derive(Resource)]
struct MapView {
    zoom: f32,
    centre: Vec2,
    press: Option<Press>,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            centre: Vec2::splat(0.5),
            press: None,
        }
    }
}

/// A mouse button held on the map: when, where (the node's -0.5..0.5), the centre then, and whether it has panned.
#[derive(Clone, Copy)]
struct Press {
    button: MouseButton,
    at_s: f64,
    from: Vec2,
    centre: Vec2,
    panned: bool,
}

/// The map's image node.
#[derive(Component)]
struct MapBox;

/// The marks over the map (labels, waypoints, the pilot, the mission), rebuilt while the page is open.
#[derive(Component)]
struct MapMarks;

/// The route's list beside the map.
#[derive(Component)]
struct MapList;

/// The marker over the active waypoint, seated and on foot.
#[derive(Component)]
struct WaypointMarker;

/// The MAP page's own button.
#[derive(Component, Clone, Copy)]
struct ClearRoute;

/// The MAP page's body under the page row: the map, and the route's list with CLEAR and BACK. Its contents are kept by
/// `draw_page` while the page is open.
pub(crate) fn map_page(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        column_gap: px(20.0),
        align_items: AlignItems::FlexStart,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            MapBox,
            Node {
                width: px(MAP_PX),
                height: px(MAP_PX),
                border: UiRect::all(px(2.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode::default(),
            RelativeCursorPosition::default(),
            BorderColor::all(DIM),
            BackgroundColor(Color::srgb(0.02, 0.06, 0.05)),
        ))
        .with_children(|b| {
            b.spawn((
                MapMarks,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
            ));
        });
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
            width: px(340.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn(crate::menu::label("MAP", 40.0, INK));
            col.spawn((MapList, crate::menu::label("", 22.0, INK)));
            crate::menu::button_frame(col, "Clear", ClearRoute, 200.0)
                .observe(|_: On<PointerClick>, mut route: ResMut<NavRoute>| route.0.clear());
            crate::menu::button_sized(col, "Back", crate::menu::MenuButton::Back, 200.0);
            col.spawn(crate::menu::label(
                "CLICK  waypoint\nRIGHT CLICK  remove\nWHEEL  zoom\nDRAG  pan",
                18.0,
                DIM,
            ));
        });
    });
}

/// Paints the island once, on the pool, and keeps the image (`cb_core::world::map::paint`).
fn paint_map(mut map: ResMut<MapImage>, plan: Option<Res<Plan>>, data: Res<Data>, mut images: ResMut<Assets<Image>>) {
    if map.handle.is_some() {
        return;
    }
    let px = data.0.nav.image_px;
    if let Some(task) = map.task.as_mut() {
        if let Some(pixels) = check_ready(task) {
            let mut img = Image::new(
                Extent3d {
                    width: px,
                    height: px,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels.into_iter().flatten().collect(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
            map.handle = Some(images.add(img));
            map.task = None;
        }
        return;
    }
    let Some(plan) = plan else { return };
    let (plan, cfg) = (plan.0.clone(), data.0.nav.clone());
    map.task = Some(AsyncComputeTaskPool::get().spawn(async move { cb_core::world::map::paint(&plan, &cfg, px) }));
}

/// The view's centre kept so the image fills the page at this zoom.
fn clamp_centre(centre: Vec2, zoom: f32) -> Vec2 {
    let half = 0.5 / zoom;
    centre.clamp(Vec2::splat(half), Vec2::splat(1.0 - half))
}

/// The world point (x, z) under a node position (-0.5..0.5 a side).
fn world_at(view: &MapView, plan: &Plan, at: Vec2) -> Vec2 {
    let (lo, side) = cb_core::world::map::extent(plan);
    lo + (view.centre + at / view.zoom) * side
}

/// Mouse on the MAP page: the wheel zooms round the pointer, a quick left click puts down a waypoint, a held left or
/// a middle drag pans, a right click takes off the nearest waypoint within `pick_px`.
#[allow(clippy::too_many_arguments)]
fn map_input(
    menu: Res<Menu>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Real>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut view: ResMut<MapView>,
    mut route: ResMut<NavRoute>,
    cursor: Query<&RelativeCursorPosition, With<MapBox>>,
) {
    let scrolled: f32 = wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y,
            MouseScrollUnit::Pixel => w.y / 40.0,
        })
        .sum();
    if *menu != Menu::Map {
        view.press = None;
        return;
    }
    let cfg = &data.0.nav;
    let Ok(rel) = cursor.single() else { return };
    let at = rel.normalized.filter(|_| rel.cursor_over);
    let now = time.elapsed_secs_f64();
    if let Some(at) = at
        && scrolled != 0.0
    {
        let under = view.centre + at / view.zoom;
        let zoom = (view.zoom * 1.25f32.powf(scrolled)).clamp(cfg.zoom[0], cfg.zoom[1]);
        view.zoom = zoom;
        view.centre = clamp_centre(under - at / zoom, zoom);
    }
    if let Some(at) = at {
        for b in [MouseButton::Left, MouseButton::Middle] {
            if buttons.just_pressed(b) {
                view.press = Some(Press {
                    button: b,
                    at_s: now,
                    from: at,
                    centre: view.centre,
                    panned: b == MouseButton::Middle,
                });
            }
        }
        if buttons.just_pressed(MouseButton::Right) {
            let (_, side) = cb_core::world::map::extent(&plan);
            let within_m = cfg.pick_px / MAP_PX * side / view.zoom;
            route.0.remove_near(world_at(&view, &plan, at), within_m);
        }
    }
    let Some(press) = view.press else { return };
    let pos = at.or(rel.normalized).unwrap_or(press.from);
    let held = now - press.at_s > f64::from(cfg.hold_pan_s);
    if buttons.pressed(press.button) {
        if press.panned || held {
            let zoom = view.zoom;
            view.centre = clamp_centre(press.centre - (pos - press.from) / zoom, zoom);
            view.press = Some(Press { panned: true, ..press });
        }
        return;
    }
    // Let go: a quick left click that never panned puts down a waypoint where it was let go.
    view.press = None;
    if press.button == MouseButton::Left && !press.panned && !held && at.is_some() {
        let p = world_at(&view, &plan, pos);
        route.0.add(p, cfg.max_waypoints);
    }
}

/// One mark on the map: text centred on a point of the node, in pixels from its top left.
fn mark(p: &mut ChildSpawnerCommands, at: Vec2, text: &str, size: f32, colour: Color, back: bool) {
    let w = text.chars().count() as f32 * size * 0.62 + 8.0;
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x - w * 0.5),
            top: px(at.y - size * 0.7),
            width: px(w),
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(if back { Color::srgba(0.0, 0.0, 0.0, 0.6) } else { Color::NONE }),
    ))
    .with_children(|m| {
        m.spawn((Text::new(text), TextFont::from_font_size(size), TextColor(colour)));
    });
}

/// The page while it is open: the image at the view's zoom and centre, the marks over it, and the route's list.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_page(
    mut commands: Commands,
    menu: Res<Menu>,
    map: Res<MapImage>,
    view: Res<MapView>,
    route: Res<NavRoute>,
    (plan, data, sim, mech): (
        Res<Plan>,
        Res<Data>,
        Res<crate::pilot::PilotSim>,
        Query<&crate::mech::MechSim, With<crate::mech::Piloted>>,
    ),
    camera: Single<&GlobalTransform, With<crate::pilot::PilotCamera>>,
    (career, free, fortress, defence): (
        Option<Res<crate::career::CareerRun>>,
        Option<Res<crate::missions::FreeRoamRun>>,
        Option<Res<crate::fortress::FortressRun>>,
        Option<Res<crate::defence::DefenceRun>>,
    ),
    mut image: Query<&mut ImageNode, With<MapBox>>,
    marks: Query<Entity, With<MapMarks>>,
    mut list: Query<&mut Text, With<MapList>>,
) {
    if *menu != Menu::Map {
        return;
    }
    let cfg = &data.0.nav;
    let Ok(mut img) = image.single_mut() else { return };
    let px_side = cfg.image_px as f32;
    if let Some(h) = &map.handle
        && img.image != *h
    {
        img.image = h.clone();
    }
    let half = 0.5 / view.zoom;
    let lo = view.centre - Vec2::splat(half);
    img.rect = Some(Rect::from_corners(lo * px_side, (view.centre + Vec2::splat(half)) * px_side));
    // Where a world point (x, z) falls on the page, pixels from its top left; None off the page.
    let on_page = |p: Vec2| {
        let uv = cb_core::world::map::to_uv(&plan, p);
        let at = (uv - lo) * view.zoom * MAP_PX;
        (at.x >= 0.0 && at.y >= 0.0 && at.x <= MAP_PX && at.y <= MAP_PX).then_some(at)
    };
    let pilot = crate::air_drop::pilot_xz(&sim, mech.iter().next());
    let goal = crate::hud::objective(
        career.as_deref(),
        free.as_deref(),
        fortress.as_deref(),
        defence.as_deref(),
        &data,
        &plan,
    );
    let Ok(marks) = marks.single() else { return };
    commands.entity(marks).despawn_related::<Children>();
    commands.entity(marks).with_children(|m| {
        let label = view.zoom >= 1.5;
        for c in &plan.cities {
            if let Some(at) = on_page(c.centre) {
                mark(m, at, &format!("CITY {}", c.id + 1), if label { 16.0 } else { 13.0 }, INK, true);
            }
        }
        for s in &plan.sites {
            let name = match s.template.kind {
                cb_core::world::sites::SiteKind::Airport => "AIRPORT",
                cb_core::world::sites::SiteKind::Seaport => "SEAPORT",
            };
            if let Some(at) = on_page(s.footprint().centre()) {
                mark(m, at, name, 14.0, Color::srgb(1.0, 0.78, 0.3), true);
            }
        }
        if label {
            for b in &plan.bases {
                if let Some(at) = on_page(b.centre) {
                    mark(m, at, "BASE", 13.0, DIM, true);
                }
            }
        }
        for (i, p) in route.0.points().iter().enumerate() {
            if let Some(at) = on_page(*p) {
                mark(m, at, &format!("<{}>", i + 1), 20.0, WAYPOINT, true);
            }
        }
        if let Some((s, _)) = &goal
            && let Some(at) = on_page(Vec2::new(s.x, s.z))
        {
            mark(m, at, "<M>", 20.0, MISSION, true);
        }
        if let Some(at) = on_page(pilot) {
            // The pilot's arrow, turned to the way the view looks (north up).
            let f = camera.forward();
            let turn = Vec2::new(f.x, f.z).normalize_or(Vec2::NEG_Y);
            m.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(at.x - 10.0),
                    top: px(at.y - 16.0),
                    ..default()
                },
                Text::new("^"),
                TextFont::from_font_size(30.0),
                TextColor(Color::WHITE),
                UiTransform {
                    rotation: Rot2::radians(turn.x.atan2(-turn.y)),
                    ..default()
                },
            ));
        }
    });
    if let Ok(mut text) = list.single_mut() {
        let mut s = format!("ROUTE {}/{}\n", route.0.points().len(), cfg.max_waypoints);
        let mut from = pilot;
        for (i, p) in route.0.points().iter().enumerate() {
            s.push_str(&format!("{}  {}\n", i + 1, distance_label(from.distance(*p))));
            from = *p;
        }
        if !route.0.points().is_empty() {
            s.push_str(&format!("TOTAL {}\n", distance_label(route.0.length_from(pilot))));
        }
        if let Some((g, name)) = &goal {
            s.push_str(&format!("\nM  {name}  {}\n", distance_label(pilot.distance(Vec2::new(g.x, g.z)))));
        }
        s.push_str(&format!("\nZOOM {:.1}x", view.zoom));
        if text.0 != s {
            text.0 = s;
        }
    }
}

/// Takes off the active waypoint once the pilot reaches it, with a chime (`cb_core::nav::Route::step`).
fn step_route(
    menu: Res<Menu>,
    data: Res<Data>,
    (sim, mech): (
        Res<crate::pilot::PilotSim>,
        Query<&crate::mech::MechSim, With<crate::mech::Piloted>>,
    ),
    mut route: ResMut<NavRoute>,
    mut sounds: MessageWriter<crate::audio::Sound>,
) {
    if menu.is_open() {
        return;
    }
    let pilot = crate::air_drop::pilot_xz(&sim, mech.iter().next());
    if matches!(route.0.step(pilot, data.0.nav.reach_m), Step::Reached | Step::Arrived) {
        sounds.write(crate::audio::Sound::speaker("nav_reached"));
    }
}

fn spawn_marker(mut commands: Commands) {
    commands.spawn((
        WaypointMarker,
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        (Text::new(""), TextFont::from_font_size(22.0), TextColor(WAYPOINT)),
        Visibility::Hidden,
    ));
}

/// The active waypoint's marker over the world, seated and on foot: a diamond, its number and its distance, held at
/// the screen's edge with an arrow when it is out of view (the job marker's own `pin` and `edge_arrow`).
fn draw_marker(
    menu: Res<Menu>,
    data: Res<Data>,
    plan: Res<Plan>,
    route: Res<NavRoute>,
    camera: Single<(&Camera, &GlobalTransform), With<crate::pilot::PilotCamera>>,
    mut marker: Single<(&mut Node, &mut Text, &mut Visibility), With<WaypointMarker>>,
) {
    let (node, text, vis) = &mut *marker;
    let Some(p) = route.0.active().filter(|_| !menu.is_open()) else {
        **vis = Visibility::Hidden;
        return;
    };
    let (cam, at) = *camera;
    let world = Vec3::new(p.x, plan.terrain.height_at(p.x, p.y) + data.0.nav.marker_height_m, p.y);
    let Some((pos, seen)) = crate::hud::pin(cam, at, world, 40.0) else {
        **vis = Visibility::Hidden;
        return;
    };
    let size = cam.logical_viewport_size().unwrap_or(Vec2::ONE);
    let eye = at.translation();
    let d = Vec2::new(eye.x, eye.z).distance(p);
    let arrow = if seen { "<> " } else { crate::hud::edge_arrow(pos, size) };
    node.left = px((pos.x - 40.0).clamp(0.0, size.x - 200.0));
    node.top = px((pos.y - 14.0).clamp(0.0, size.y - 30.0));
    let s = format!("{arrow}WP1 {}", distance_label(d));
    if text.0 != s {
        text.0 = s;
    }
    **vis = Visibility::Inherited;
}
