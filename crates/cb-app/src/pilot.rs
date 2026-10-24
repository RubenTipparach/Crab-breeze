//! The pilot in the game (`pilot-boarding`): steps the pilot in the fixed step and places the one
//! camera, the pilot's eyes, every frame: the head on foot, the path while boarding or climbing
//! out, and `EYE_pilot` in the seat. There is no other camera (CLAUDE.md, the law).

use crate::input::{InputAccum, StepActions};
use crate::mech::{MechAccessNow, MechRig, socket_now};
use crate::sim::{Data, PresentSet, SimSet};
use crate::world::{Live, Plan, SKY};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use cb_core::mech::PartKind;
use cb_core::pilot::{PilotEvent, PilotMode, PilotState};
use cb_core::world::Scene;

/// The pilot's state at the last two fixed steps, for interpolation.
#[derive(Resource)]
pub struct PilotSim {
    /// The step before.
    pub prev: PilotState,
    /// The latest step.
    pub curr: PilotState,
}

/// Pilot events this fixed step, for the mech's boarding sequence.
#[derive(Resource, Default)]
pub struct PilotEvents(pub Vec<PilotEvent>);

/// The one camera: the pilot's eyes.
#[derive(Component)]
pub struct PilotCamera;

/// Steps the pilot and places the camera.
pub struct PilotPlugin;

impl Plugin for PilotPlugin {
    fn build(&self, app: &mut App) {
        let plan = app.world().resource::<Plan>().clone();
        let s = plan.start;
        let feet = Vec3::new(s.pilot_xz.x, plan.terrain.height_at(s.pilot_xz.x, s.pilot_xz.y), s.pilot_xz.y);
        // Face the mech.
        let to_mech = s.mech_xz - s.pilot_xz;
        let yaw = (-to_mech.x).atan2(-to_mech.y);
        let state = PilotState::standing(feet, yaw);
        app.insert_resource(PilotSim {
            prev: state.clone(),
            curr: state,
        })
        .init_resource::<PilotEvents>()
        .add_systems(Startup, spawn_camera)
        .add_systems(FixedUpdate, step_pilot.in_set(SimSet::Pilot))
        .add_systems(Startup, spawn_prompt)
        .add_systems(
            Update,
            ((place_camera, weather_and_screens).chain(), show_prompt).in_set(PresentSet::Camera),
        );
    }
}

/// The render layer the cockpit is drawn on while the pilot sits sealed in it (cockpit-overlay).
pub const COCKPIT_LAYER: usize = 1;

/// The camera that draws the cockpit after the world, on its own depth (cockpit-overlay): a child
/// of the pilot's eyes with the same lens, so the two views share one transform each frame.
#[derive(Component)]
pub struct CockpitCamera;

fn spawn_camera(mut commands: Commands) {
    let lens = Projection::from(PerspectiveProjection {
        fov: 75f32.to_radians(),
        near: 0.05,
        far: 6000.0,
        ..default()
    });
    let cockpit = commands
        .spawn((
            Name::new("cockpit pass"),
            CockpitCamera,
            Camera3d::default(),
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            lens.clone(),
            bevy::camera::visibility::RenderLayers::layer(COCKPIT_LAYER),
            Msaa::Off,
            bevy::anti_alias::fxaa::Fxaa::default(),
            Transform::default(),
        ))
        .id();
    commands
        .spawn((
            Name::new("pilot's eyes"),
            PilotCamera,
            Camera3d::default(),
            lens,
            DistanceFog {
                color: SKY,
                falloff: FogFalloff::from_visibility_colors(3500.0, Color::srgb(0.45, 0.55, 0.68), Color::srgb(0.80, 0.86, 0.95)),
                ..default()
            },
            Transform::default(),
            ColorGrading::default(),
            // Smoke, dust and fire sorted per pixel, not per quad, so overlapping puffs never pop in
            // front of each other (combat-fx). It needs MSAA off; FXAA smooths the edges instead.
            bevy::core_pipeline::oit::OrderIndependentTransparencySettings {
                sorted_fragment_max_count: 8,
                fragments_per_pixel_average: 6.0,
                alpha_threshold: 0.02,
            },
            Msaa::Off,
            bevy::anti_alias::fxaa::Fxaa::default(),
        ))
        .add_child(cockpit);
}

fn step_pilot(
    mut pilot: ResMut<PilotSim>,
    mut events: ResMut<PilotEvents>,
    actions: Res<StepActions>,
    access: Res<MechAccessNow>,
    data: Res<Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    time: Res<Time<Fixed>>,
) {
    let pilot = &mut *pilot;
    pilot.prev = pilot.curr.clone();
    let ground = Scene {
        plan: &plan,
        dug: &live.dug,
        trees: Some(&live.forest),
        structures: Some(&live.structures),
    };
    let event = pilot
        .curr
        .step(&actions, &data.0.pilot, &ground, access.0.as_ref(), time.delta_secs());
    events.0.extend(event);
    match event {
        Some(PilotEvent::HatchWanted) => info!("pilot: asked the mech to sit and open"),
        Some(PilotEvent::BoardingStarted) => info!("pilot: boarding"),
        Some(PilotEvent::Seated) => info!("pilot: seated at EYE_pilot"),
        Some(PilotEvent::DisembarkStarted) => info!("pilot: climbing out"),
        Some(PilotEvent::Disembarked) => info!("pilot: standing at LADDER_base"),
        None => {}
    }
}

fn view(yaw: f32, pitch: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}

fn place_camera(
    pilot: Res<PilotSim>,
    acc: Res<InputAccum>,
    access: Res<MechAccessNow>,
    fixed: Res<Time<Fixed>>,
    data: Res<Data>,
    rigs: Query<&MechRig, With<crate::mech::Piloted>>,
    transforms: Query<&Transform, Without<PilotCamera>>,
    parents: Query<&ChildOf>,
    mut camera: Single<&mut Transform, With<PilotCamera>>,
) {
    let cfg = &data.0.pilot;
    let alpha = fixed.overstep_fraction();
    let (prev, curr) = (&pilot.prev, &pilot.curr);
    // Mouse look not yet taken by a fixed step is shown now, so looking never waits for a step.
    let pending = acc.look_deg;
    let seat = || {
        let rig = rigs.iter().next()?;
        let eye = socket_now(rig, "EYE_pilot", &transforms, &parents)?;
        Some(eye)
    };
    let walking_view = || {
        let pitch_limit = cfg.pitch_limit_deg.to_radians();
        view(
            curr.yaw_rad - pending.x.to_radians(),
            (curr.pitch_rad + pending.y.to_radians()).clamp(-pitch_limit, pitch_limit),
        )
    };
    // The view's direction is the player's, always: only the mouse turns it. Seated, the eyes ride
    // at EYE_pilot; boarding and climbing out, they ride the path.
    let eye = match &curr.mode {
        PilotMode::Seated => seat().map_or(prev.eye(cfg).lerp(curr.eye(cfg), alpha), |s| s.translation),
        _ => prev.eye(cfg).lerp(curr.eye(cfg), alpha),
    };
    let rotation = match &curr.mode {
        // Stepping in (the owner's design): look input is paused; the view turns from where the
        // player was looking to face out of the seat over the climb.
        PilotMode::Boarding(path) => {
            let k = (path.travelled_m / path.length_m().max(0.01)).clamp(0.0, 1.0);
            let k = k * k * (3.0 - 2.0 * k);
            let seat_yaw = access.0.as_ref().map_or(curr.yaw_rad, |a| a.seat_yaw_rad);
            view(curr.yaw_rad, curr.pitch_rad).slerp(view(seat_yaw, 0.0), k)
        }
        _ => walking_view(),
    };
    **camera = Transform::from_translation(eye).with_rotation(rotation);
}

/// The step-in prompt.
#[derive(Component)]
struct Prompt;

fn spawn_prompt(mut commands: Commands) {
    commands.spawn((
        Prompt,
        Text::new(""),
        TextFont::from_font_size(28.0),
        TextColor(Color::srgb(0.95, 0.95, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            bottom: percent(30.0),
            width: percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center),
    ));
}

/// Shows "E: step into the cockpit" when the kneeling mech is ready and the pilot is near.
fn show_prompt(pilot: Res<PilotSim>, access: Res<MechAccessNow>, data: Res<Data>, mut prompt: Single<&mut Text, With<Prompt>>) {
    let p = &pilot.curr;
    let ready = access.0.as_ref().is_some_and(|a| a.hatch_open && p.can_board(a, &data.0.pilot));
    let want = if ready { "[E]  Step into the cockpit" } else { "" };
    if prompt.0 != want {
        prompt.0 = want.into();
    }
}

/// Clear-air visibility.
const CLEAR_VISIBILITY_M: f32 = 3500.0;

/// What the pilot sees through: in a cloud the view fogs by the cloud density at the eye (the one
/// cloud query); seated with the sensor head shot off, the view is the backup cameras' (washed out,
/// darker, tinted) until the all-view screen change draws them properly.
fn weather_and_screens(
    plan: Res<Plan>,
    time: Res<Time>,
    pilot: Res<PilotSim>,
    mechs: Query<(&crate::mech::MechSim, &crate::mech::MechSpec), With<crate::mech::Piloted>>,
    mut camera: Single<(&GlobalTransform, &mut DistanceFog, &mut ColorGrading), With<PilotCamera>>,
) {
    let (eye, fog, grading) = &mut *camera;
    let density = plan.clouds.density_at(eye.translation(), time.elapsed_secs());
    let vis = CLEAR_VISIBILITY_M + (25.0 - CLEAR_VISIBILITY_M) * density.sqrt();
    let inside = density.sqrt();
    fog.color = SKY.mix(&Color::srgb(0.9, 0.92, 0.95), inside);
    fog.falloff = FogFalloff::from_visibility_colors(
        vis,
        Color::srgb(0.45, 0.55, 0.68).mix(&Color::srgb(0.85, 0.87, 0.9), inside),
        Color::srgb(0.80, 0.86, 0.95).mix(&Color::srgb(0.95, 0.95, 0.97), inside),
    );
    let head_lost = pilot.curr.mode == PilotMode::Seated && mechs.iter().any(|(m, spec)| m.damage.is_lost(&spec.parts, PartKind::Head));
    let (sat, exp, tint) = if head_lost { (0.25, -0.6, 0.15) } else { (1.0, 0.0, 0.0) };
    if grading.global.post_saturation != sat {
        grading.global.post_saturation = sat;
        grading.global.exposure = exp;
        grading.global.tint = tint;
    }
}
