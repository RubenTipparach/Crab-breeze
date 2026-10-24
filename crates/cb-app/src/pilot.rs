//! The pilot in the game (`pilot-boarding`): steps the pilot in the fixed step and places the one
//! camera, the pilot's eyes, every frame: the head on foot, the path while boarding or climbing
//! out, and `EYE_pilot` in the seat. There is no other camera (CLAUDE.md, the law).

use crate::input::{InputAccum, StepActions};
use crate::mech::{MechAccessNow, MechRig, socket_now};
use crate::sim::{Data, PresentSet, SimSet};
use crate::world::{Plan, SKY};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use cb_core::pilot::{PilotEvent, PilotMode, PilotState};

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
        .add_systems(Update, place_camera.in_set(PresentSet::Camera));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("pilot's eyes"),
        PilotCamera,
        Camera3d::default(),
        Projection::from(PerspectiveProjection {
            fov: 75f32.to_radians(),
            near: 0.05,
            far: 6000.0,
            ..default()
        }),
        DistanceFog {
            color: SKY,
            falloff: FogFalloff::from_visibility_colors(3500.0, Color::srgb(0.45, 0.55, 0.68), Color::srgb(0.80, 0.86, 0.95)),
            ..default()
        },
        Transform::default(),
    ));
}

fn step_pilot(
    mut pilot: ResMut<PilotSim>,
    mut events: ResMut<PilotEvents>,
    actions: Res<StepActions>,
    access: Res<MechAccessNow>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Fixed>>,
) {
    let pilot = &mut *pilot;
    pilot.prev = pilot.curr.clone();
    let ground: &cb_core::world::WorldPlan = &plan;
    let event = pilot
        .curr
        .step(&actions, &data.0.pilot, ground, access.0.as_ref(), time.delta_secs());
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
    fixed: Res<Time<Fixed>>,
    data: Res<Data>,
    rigs: Query<&MechRig>,
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
        // EYE_pilot looks along its +z; a camera looks along its -z.
        Some(Transform::from_translation(eye.translation).with_rotation(eye.rotation * Quat::from_rotation_y(std::f32::consts::PI)))
    };
    let walking_view = || {
        let pitch_limit = cfg.pitch_limit_deg.to_radians();
        view(
            curr.yaw_rad - pending.x.to_radians(),
            (curr.pitch_rad + pending.y.to_radians()).clamp(-pitch_limit, pitch_limit),
        )
    };
    let eye = prev.eye(cfg).lerp(curr.eye(cfg), alpha);
    **camera = match &curr.mode {
        PilotMode::OnFoot | PilotMode::Disembarking(_) => Transform::from_translation(eye).with_rotation(walking_view()),
        PilotMode::Boarding(path) => {
            // On the last leg, into the seat, the view turns to the seat's forward.
            let mut rot = walking_view();
            if let Some(s) = seat() {
                let t = path.last_leg_progress();
                rot = rot.slerp(s.rotation, t * t * (3.0 - 2.0 * t));
            }
            Transform::from_translation(eye).with_rotation(rot)
        }
        PilotMode::Seated => match seat() {
            Some(s) => {
                let yaw = curr.head_yaw_rad;
                let pitch = curr.head_pitch_rad;
                s.with_rotation(s.rotation * view(yaw, pitch))
            }
            None => Transform::from_translation(eye).with_rotation(walking_view()),
        },
    };
}
