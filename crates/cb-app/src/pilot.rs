//! The pilot in the game (`pilot-boarding`): steps the pilot in the fixed step and places the one
//! camera, the pilot's eyes, every frame: the head on foot, the path while boarding or climbing
//! out, and `EYE_pilot` in the seat. There is no other camera (CLAUDE.md, the law).

use crate::input::{InputAccum, StepActions};
use crate::mech::{MechAccessNow, MechRig};
use crate::sim::{Data, PresentSet, SimSet};
use crate::state::GameApp;
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
        .init_resource::<Shake>()
        .add_world_spawn((cpu_light_clustering, spawn_camera))
        .add_game_systems(FixedUpdate, step_pilot.in_set(SimSet::Pilot))
        .add_message::<ShakeKick>()
        .add_game_systems(FixedUpdate, body_kicks.after(SimSet::Mech))
        .add_world_spawn(spawn_prompt)
        .add_game_systems(
            Update,
            (
                (
                    shake_on_blasts,
                    place_camera,
                    crate::water_render::track_eye,
                    crate::ground_fog::follow_eye_weather,
                    weather_and_screens,
                )
                    .chain(),
                show_prompt,
                ease_fov,
            )
                .in_set(PresentSet::Camera),
        );
    }
}

/// The render layer the cockpit is drawn on while the pilot sits sealed in it (cockpit-overlay).
pub const COCKPIT_LAYER: usize = 1;

/// The camera that draws the cockpit after the world, on its own depth (cockpit-overlay): a child
/// of the pilot's eyes with the same lens, so the two views share one transform each frame.
#[derive(Component)]
pub struct CockpitCamera;

/// Lights are clustered on the CPU. Bevy 0.20's GPU clustering ignores render layers, so every gun
/// flash and blast outside lit the cockpit pass too and the cabin strobed with the guns (owner,
/// 2026-10-24: "Strobing appears to be from guns"). CPU clustering keeps each light to its layer.
fn cpu_light_clustering(mut settings: ResMut<bevy::light::cluster::GlobalClusterSettings>) {
    settings.gpu_clustering = None;
}

/// The view widens as the pilot sits and narrows as they stand (openspec change `seated-fov`; owner, 2026-10-26:
/// "normal fps fov is 75, it grows to 95 when inside the mech"): the one system that sets the pilot camera's and the
/// cockpit pass's field of view, from the player's seated setting and the data's on-foot view, eased
/// (`cb_core::pilot::fov_blend`, `fov_deg`). Only the field of view changes, never where the view points.
fn ease_fov(
    pilot: Res<PilotSim>,
    data: Res<Data>,
    settings: Res<crate::settings::Settings>,
    time: Res<Time>,
    mut blend: Local<f32>,
    mut lenses: Query<&mut Projection, Or<(With<PilotCamera>, With<CockpitCamera>)>>,
) {
    let cfg = &data.0.pilot;
    // only the mech's seat widens it: driving a car keeps the on-foot view, third person included (owner,
    // 2026-10-26: "car driving should remain at 75")
    *blend = cb_core::pilot::fov_blend(*blend, pilot.curr.mode.wide_view(), time.delta_secs(), cfg.fov_change_s);
    for mut lens in &mut lenses {
        if let Projection::Perspective(p) = lens.as_mut() {
            let fov = cb_core::pilot::fov_deg(cfg, settings.0.fov_deg, *blend, p.aspect_ratio).to_radians();
            if (p.fov - fov).abs() > 1e-5 {
                p.fov = fov;
            }
        }
    }
}

fn spawn_camera(mut commands: Commands, data: Res<Data>) {
    let light = &data.0.cockpit_light;
    // Bevy's perspective is reverse-Z with an infinite far plane, so `far` only culls: seeing 40 km
    // costs no depth precision (island-world design section 5).
    let view = &data.0.world.view;
    let srgb = |c: [f32; 3]| Color::srgb(c[0], c[1], c[2]);
    let lens = Projection::from(PerspectiveProjection {
        // On foot at the start; `ease_fov` owns it from then on (openspec change `seated-fov`).
        fov: cb_core::pilot::fov_deg(&data.0.pilot, data.0.pilot.view_fov_deg, 0.0, 16.0 / 9.0).to_radians(),
        near: 0.05,
        far: view.far_m,
        ..default()
    });
    let cockpit = commands
        .spawn((
            Name::new("cockpit pass"),
            CockpitCamera,
            Camera3d::default(),
            Camera {
                order: 1,
                // `CB_NO_COCKPIT_PASS=1`: the second camera off (perf-bench switch, to measure it).
                is_active: !crate::cli::switched_off("CB_NO_COCKPIT_PASS"),
                clear_color: ClearColorConfig::None,
                ..default()
            },
            lens.clone(),
            bevy::camera::visibility::RenderLayers::layer(COCKPIT_LAYER),
            // It draws over the eye camera's frame without clearing, so the two must share one
            // target texture: the same HDR (the water makes the eye camera HDR) and no MSAA. Left
            // LDR, it drew into a texture of its own that never cleared: the HUD smeared, the
            // loading screen and an old prompt stuck, and the world vanished behind it.
            bevy::camera::Hdr,
            Msaa::Off,
            bevy::anti_alias::fxaa::Fxaa::default(),
            Transform::default(),
            // The sealed cabin's own light (cockpit-lighting): its lamps, not the sky. The sun and
            // every flash outside are on the world layer and never reach this pass.
            AmbientLight {
                color: srgb(light.cabin_ambient_colour),
                brightness: light.cabin_ambient_cd_m2,
                ..default()
            },
        ))
        .with_children(|eye| {
            // The screens' glow, as one directional light kept to the cockpit layer (cb-core
            // `screen_glow`).
            if let Some((travel, lux)) = light.screen_glow() {
                eye.spawn((
                    DirectionalLight {
                        illuminance: lux,
                        color: srgb(light.screen_tint),
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::default().looking_to(travel, Vec3::Y),
                    bevy::camera::visibility::RenderLayers::layer(COCKPIT_LAYER),
                ));
            }
        })
        .id();
    let mut camera = commands.spawn((
        Name::new("pilot's eyes"),
        PilotCamera,
        Camera3d::default(),
        lens,
        DistanceFog {
            color: SKY,
            falloff: FogFalloff::from_visibility_colors(
                view.clear_visibility_m,
                Color::srgb(0.45, 0.55, 0.68),
                Color::srgb(0.80, 0.86, 0.95),
            ),
            ..default()
        },
        Transform::default(),
        ColorGrading::default(),
        Msaa::Off,
        bevy::anti_alias::fxaa::Fxaa::default(),
    ));
    camera.add_child(cockpit);
    // Screen-space ambient occlusion on the world (ambient-occlusion): creases and contact points
    // darken in indirect light. The cockpit keeps its baked occlusion. `CB_NO_SSAO=1` or a 0 radius
    // switches it off.
    // GPU occlusion culling (occlusion-culling): meshes hidden behind what was drawn last frame are
    // skipped on the GPU. It needs the depth prepass. `CB_NO_OCCLUSION=1` switches it off.
    if view.occlusion_culling && !crate::cli::switched_off("CB_NO_OCCLUSION") {
        camera.insert((
            bevy::core_pipeline::prepass::DepthPrepass,
            bevy::render::occlusion_culling::OcclusionCulling,
        ));
    }
    if view.ssao_radius_m > 0.0 && !crate::cli::switched_off("CB_NO_SSAO") {
        use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel as Q};
        use cb_core::world::config::SsaoQuality;
        camera.insert(ScreenSpaceAmbientOcclusion {
            quality_level: match view.ssao_quality {
                SsaoQuality::Low => Q::Low,
                SsaoQuality::Medium => Q::Medium,
                SsaoQuality::High => Q::High,
                SsaoQuality::Ultra => Q::Ultra,
            },
            radius: view.ssao_radius_m,
            ..default()
        });
    }
    // The scattering sky (sky-atmosphere): Bevy's atmosphere draws the sky and hazes the distance,
    // under raw sunlight, with the exposure to match. `CB_NO_SKY=1` keeps the old flat sky.
    if crate::world::scattering_sky() {
        camera.insert((
            bevy::pbr::AtmosphereSettings::default(),
            bevy::camera::Exposure {
                ev100: view.exposure_ev100,
            },
            // ACES keeps a sunlit scene's colour where Bevy's default tonemapper washes it toward
            // grey (owner: "Grass needs a richer green. It's too faded right now").
            bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        ));
    }
    // Sees water the pilot's way: under it, and with drops on the glass after surfacing
    // (openspec change `water-rendering`; brings HDR, a depth prepass and transmission).
    // `CB_NO_WATER_FX=1` leaves them off, to compare frame time or if the passes misbehave.
    if !crate::cli::switched_off("CB_NO_WATER_FX") {
        camera.insert(crate::water_render::WaterOptics);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn step_pilot(
    mut pilot: ResMut<PilotSim>,
    mut events: ResMut<PilotEvents>,
    actions: Res<StepActions>,
    access: Res<MechAccessNow>,
    car: Res<crate::traffic::CarAccessNow>,
    choice: Res<crate::interact::UseChoice>,
    data: Res<Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    time: Res<Time<Fixed>>,
    interiors: Res<crate::walk_in::InteriorColliders>,
) {
    let pilot = &mut *pilot;
    pilot.prev = pilot.curr.clone();
    let ground = Scene {
        plan: &plan,
        dug: &live.dug,
        trees: Some(&live.forest),
        structures: Some(&live.structures),
        platforms: &live.platforms,
        interiors: Some(&interiors.0),
    };
    // Use reaches only the interaction selected (`interact`, survey W40): a car's door or getting out, or the mech.
    // A car first (car-driving): Use at a car's door gets in; in a car, the mech's step has nothing to do.
    use cb_core::interact::Interaction;
    let dt = time.delta_secs();
    let mut car_a = actions.0;
    car_a.use_pressed = matches!(choice.now, Some(Interaction::GetIn | Interaction::GetOut));
    let car_event = pilot.curr.step_car(&car_a, &data.0.pilot, &ground, car.0.as_ref(), dt);
    events.0.extend(car_event);
    let mut a = actions.0;
    a.use_pressed = choice.now == Some(Interaction::Board) && car_event.is_none();
    let event = if pilot.curr.mode.in_car() || car_event.is_some() {
        None
    } else {
        pilot.curr.step(&a, &data.0.pilot, &ground, access.0.as_ref(), dt)
    };
    events.0.extend(event);
    match event {
        Some(PilotEvent::HatchWanted) => info!("pilot: asked the mech to sit and open"),
        Some(PilotEvent::BoardingStarted) => info!("pilot: boarding"),
        Some(PilotEvent::Seated) => info!("pilot: seated at EYE_pilot"),
        Some(PilotEvent::DisembarkStarted) => info!("pilot: climbing out"),
        Some(PilotEvent::Disembarked) => info!("pilot: standing at LADDER_base"),
        Some(PilotEvent::Ejected) => info!("pilot: ejected from the destroyed mech"),
        Some(e) => info!("pilot: {e:?}"),
        None => {}
    }
}

fn view(yaw: f32, pitch: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}

/// The view's shake (openspec change `cockpit-shake`): the trauma kicked by blasts and the mech's own body (0..1), the
/// rumble of its jets and skates (0..1), the eye's jolt, the clock, and the player's strength setting.
#[derive(Resource, Default)]
pub(crate) struct Shake {
    trauma: f32,
    rumble: f32,
    /// The firing rumble is held until this clock time, seconds.
    fire_until_s: f32,
    jolt: cb_core::fx::Jolt,
    t_s: f32,
    strength: f32,
}

impl Shake {
    /// How far the eye is dipped this frame, metres (the cockpit camera takes the same dip).
    pub(crate) fn dip_m(&self) -> f32 {
        self.jolt.y * self.strength
    }
}

/// A kick to the view from the pilot's own mech (a landing, a ram, a hit, a punch, its guns, its jets lighting).
#[derive(Message, Clone, Copy, Debug)]
pub struct ShakeKick(pub cb_core::fx::Kick);

/// Kicks the shake from blasts by how near and big they are, from the mech's body (`ShakeKick`) and from hits on the
/// piloted mech; eases the rumble toward what the jets and skates ask; springs the jolt back. It all dies away by
/// itself.
#[allow(clippy::too_many_arguments)]
fn shake_on_blasts(
    mut blasts: MessageReader<crate::blast::Explosion>,
    mut kicks: MessageReader<ShakeKick>,
    mut hits: MessageReader<crate::battle_fx::MechHit>,
    data: Res<Data>,
    time: Res<Time>,
    camera: Single<&Transform, With<PilotCamera>>,
    piloted: Query<(&crate::mech::MechSim, &crate::mech::MechId), With<crate::mech::Piloted>>,
    (pilot, plan, settings): (Res<PilotSim>, Res<Plan>, Option<Res<crate::settings::Settings>>),
    mut shake: ResMut<Shake>,
) {
    let cfg = &data.0.fx.shake;
    let dt = time.delta_secs();
    shake.t_s += dt;
    shake.strength = settings.map_or(1.0, |s| s.0.camera_shake);
    shake.trauma = cfg.decay(shake.trauma, dt);
    let add = |k: cb_core::fx::Kick, shake: &mut Shake| {
        shake.trauma = (shake.trauma + k.trauma).min(1.0);
        shake.jolt.kick(k.jolt_m_s);
        if k.fire_s > 0.0 {
            shake.fire_until_s = shake.fire_until_s.max(shake.t_s + k.fire_s);
        }
    };
    for b in blasts.read() {
        let kick = cfg.kick(b.energy_mj, b.at.distance(camera.translation));
        shake.trauma = (shake.trauma + kick).min(1.0);
    }
    let seated = pilot.curr.mode == PilotMode::Seated;
    let me = piloted.iter().next();
    for k in kicks.read() {
        if seated {
            add(k.0, &mut shake);
        }
    }
    for h in hits.read() {
        if seated && me.is_some_and(|(_, id)| id.0 == h.mech) {
            add(cfg.hit(h.penetration_mm, h.blast_mj), &mut shake);
        }
    }
    // The rumble the body asks for, eased so it swells and fades.
    let want = match me {
        Some((sim, _)) if seated => {
            let s = &sim.curr;
            let paved = plan.0.surface_at(Vec2::new(s.pos.x, s.pos.z)) == cb_core::world::Surface::Concrete;
            cfg.rumble(s.boosting, s.skates_on && !s.in_air, s.speed_m_s, paved)
        }
        _ => 0.0,
    };
    // Firing holds the rumble at least at `fire_rumble` (owner: "firing guns should also cause some vibrations").
    let want = if shake.t_s < shake.fire_until_s {
        want.max(cfg.fire_rumble)
    } else {
        want
    };
    shake.rumble = cfg.ease_rumble(shake.rumble, want, dt);
    let jolt_cfg = cfg.clone();
    shake.jolt.step(&jolt_cfg, dt);
}

/// The pilot's own mech kicks the view in the step it lands, rams, or lights its jets (openspec change
/// `cockpit-shake`): read from its state before and after the step, so no event is missed between frames.
fn body_kicks(data: Res<Data>, piloted: Query<&crate::mech::MechSim, With<crate::mech::Piloted>>, mut kicks: MessageWriter<ShakeKick>) {
    let cfg = &data.0.fx.shake;
    let Some(sim) = piloted.iter().next() else { return };
    let (a, b) = (&sim.prev, &sim.curr);
    if a.in_air && !b.in_air {
        let k = cfg.landing(-a.vel_y_m_s);
        if k.trauma > 0.0 {
            kicks.write(ShakeKick(k));
        }
    }
    // Speed lost in one step beyond any brake: the mech hit something.
    let lost = (a.vel - b.vel).length();
    if !b.in_air && b.vel.length() < a.vel.length() {
        let k = cfg.ram(lost);
        if k.trauma > 0.0 {
            kicks.write(ShakeKick(k));
        }
    }
    if b.boosting && !a.boosting {
        kicks.write(ShakeKick(cfg.ignite()));
    }
}

/// The chase camera's eased state between frames (openspec change `car-chase-camera`).
#[derive(Default)]
struct Chase {
    /// 0 at the pilot's eyes, 1 at the chase place: eased in on getting in, out on getting out.
    blend: f32,
    /// The orbit's point, lagging the car.
    pivot: Vec3,
    /// The car's heading the view turns with, lagging the car's own.
    heading_rad: f32,
    /// The vehicle it last followed, so a new one starts the lags afresh.
    car: Option<(cb_core::pilot::Vehicle, u32)>,
    /// The last chase place, held if the car is lost mid-blend (a wreck cleared), so the view never cuts.
    last: Option<(Vec3, Quat)>,
}

/// Where the chase camera is this frame, or `None` when no vehicle is in hand: behind and above the car or the stolen
/// aircraft the pilot is in (or getting into or out of), looking along the player's view from its nose, pulled in out
/// of buildings and the ground. One camera for both (`fly-stolen-aircraft`); rules in `cb_core::pilot::chase`.
#[allow(clippy::too_many_arguments)]
fn chase_view(
    chase: &mut Chase,
    cfg: &cb_core::pilot::PilotConfig,
    curr: &PilotState,
    pending: Vec2,
    car: Option<&cb_core::pilot::CarAccess>,
    traffic: Option<&crate::traffic::TrafficSim>,
    flown: Option<&crate::stolen_aircraft::FlownAircraft>,
    data: &Data,
    plan: &Plan,
    alpha: f32,
    dt_s: f32,
) -> Option<(Vec3, Quat)> {
    use cb_core::pilot::Vehicle;
    use cb_core::pilot::chase::{ease, ease_angle, view_rotation};
    let cam = &cfg.car_camera;
    let car = car?;
    // where the vehicle stands (its middle on the ground, or under it in the air), its heading, length and height
    let (pos, heading, length_m, height_m) = match car.vehicle {
        Vehicle::Car => {
            let c = traffic?.traffic.get(car.id)?;
            let (pos, heading) = traffic?.drawn_pose(c, alpha);
            let spec = &data.0.traffic.kinds[c.kind as usize];
            (pos, heading, spec.size_m[2], spec.size_m[1])
        }
        Vehicle::Aircraft => {
            let f = flown.filter(|f| f.id == car.id)?;
            let spec = data.0.air_traffic.kinds.get(&f.kind)?;
            let (pos, heading) = f.drawn(alpha);
            (
                pos - Vec3::Y * cb_core::air_traffic::rest_m(spec),
                heading,
                spec.length_m,
                spec.height_m,
            )
        }
    };
    let target = cam.pivot(pos, height_m);
    if chase.car != Some((car.vehicle, car.id)) {
        chase.car = Some((car.vehicle, car.id));
        chase.pivot = target;
        chase.heading_rad = heading;
    }
    chase.pivot = ease(chase.pivot, target, dt_s, cam.follow_lag_s);
    chase.heading_rad = ease_angle(chase.heading_rad, heading, dt_s, cam.heading_lag_s);
    // the player's own look from the car's nose, with the mouse not yet stepped (CLAUDE.md section 8)
    let yaw = chase.heading_rad + curr.seated_rel_rad - pending.x.to_radians();
    let pitch = cam.pitch(cam.clamp_player_pitch(curr.pitch_rad + pending.y.to_radians()));
    let wanted = cam.wanted_eye(chase.pivot, yaw, pitch, cam.distance_for(length_m));
    // out of the ground (with a hand's width to spare) and out of the buildings
    let eye = cam.clip(chase.pivot, wanted, 0.25, |q| plan.solid_at(q - Vec3::Y * 0.3));
    Some((eye, view_rotation(yaw, pitch)))
}

fn place_camera(
    shake: Res<Shake>,
    pilot: Res<PilotSim>,
    acc: Res<InputAccum>,
    access: Res<MechAccessNow>,
    fixed: Res<Time<Fixed>>,
    data: Res<Data>,
    rigs: Query<&MechRig, With<crate::mech::Piloted>>,
    sims: Query<&crate::mech::MechSim, With<crate::mech::Piloted>>,
    transforms: Query<&Transform, Without<PilotCamera>>,
    parents: Query<&ChildOf>,
    mut camera: Single<&mut Transform, With<PilotCamera>>,
    (car, traffic, plan, time, flown): (
        Res<crate::traffic::CarAccessNow>,
        Option<Res<crate::traffic::TrafficSim>>,
        Res<Plan>,
        Res<Time>,
        Res<crate::stolen_aircraft::Flown>,
    ),
    mut chase: Local<Chase>,
) {
    let cfg = &data.0.pilot;
    let alpha = fixed.overstep_fraction();
    let (prev, curr) = (&pilot.prev, &pilot.curr);
    // Mouse look not yet taken by a fixed step is shown now, so looking never waits for a step.
    let pending = acc.look_deg;
    let seat = || {
        let rig = rigs.iter().next()?;
        // Level with the stride: no footstep bob or sway in the cockpit (owner, 2026-10-25).
        let eye = crate::mech::socket_steady(rig, "EYE_pilot", &transforms, &parents)?;
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
        // driving, the eyes ride EYE_driver as the car moved between the two steps
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
        // Seated, the view is the mech's heading, drawn between steps as the cockpit is, plus the
        // pilot's own look from its nose with the mouse not yet stepped. Turning only once a fixed
        // step while the cockpit glided between steps, it jittered against the cabin (owner,
        // 2026-10-25: "the motion of the mech turning vs mouselook ... it jitters alot").
        // The look from the nose is blended between steps as the nose is (`cb_core::pilot::seated_yaw`): taken from
        // the last step alone, the view fell a step's turn behind and snapped on while the mech turned (owner,
        // 2026-10-28: "make sure it is smoothly rotating").
        PilotMode::Seated => match sims.iter().next() {
            Some(sim) => {
                let pitch_limit = cfg.pitch_limit_deg.to_radians();
                let yaw = cb_core::pilot::seated_yaw(
                    [sim.prev.heading_rad, sim.curr.heading_rad],
                    [prev.seated_rel_rad, curr.seated_rel_rad],
                    alpha,
                );
                view(
                    yaw - pending.x.to_radians(),
                    (curr.pitch_rad + pending.y.to_radians()).clamp(-pitch_limit, pitch_limit),
                )
            }
            None => walking_view(),
        },
        // Driving, the view turns with the car between steps and the mouse looks round from its nose; the
        // car only carries the eyes, the player turns the view (CLAUDE.md section 8).
        PilotMode::Driving if prev.mode == PilotMode::Driving => {
            let pitch_limit = cfg.pitch_limit_deg.to_radians();
            let turned = (curr.yaw_rad - prev.yaw_rad + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            view(
                prev.yaw_rad + turned * alpha - pending.x.to_radians(),
                (curr.pitch_rad + pending.y.to_radians()).clamp(-pitch_limit, pitch_limit),
            )
        }
        _ => walking_view(),
    };
    // Driving a car or flying a stolen aircraft, the view is third person (the exceptions to THE LAW; owner,
    // 2026-10-26: "also driving should be in 3rd person", and 2026-10-30 for aircraft): the chase camera eases out of the pilot's eyes on getting in and back into them
    // on getting out, with no cut (openspec change `car-chase-camera`).
    let dt = time.delta_secs();
    chase.blend = cb_core::pilot::fov_blend(chase.blend, curr.mode == PilotMode::Driving, dt, cfg.car_camera.blend_s);
    let (eye, rotation) = if chase.blend > 0.0 {
        let at = chase_view(
            &mut chase,
            cfg,
            curr,
            pending,
            car.0.as_ref(),
            traffic.as_deref(),
            flown.0.as_ref(),
            &data,
            &plan,
            alpha,
            dt,
        );
        chase.last = at.or(chase.last);
        match chase.last {
            Some((at, rot)) => cb_core::pilot::chase::blend_view(eye, rotation, at, rot, chase.blend),
            None => (eye, rotation),
        }
    } else {
        chase.car = None;
        chase.last = None;
        (eye, rotation)
    };
    // The shake rides on top of the player's view and is never written into their aim: the look
    // they steer is unchanged (CLAUDE.md section 8). Felt as the square of the trauma, so small
    // blasts barely register; three unrelated sines so it never settles into a rhythm.
    let sc = &data.0.fx.shake;
    let amp = sc.max_deg.to_radians() * shake.trauma * shake.trauma * shake.strength;
    let t = shake.t_s;
    // The jets' and skates' rumble: a finer, faster buzz (openspec change `cockpit-shake`).
    let buzz = sc.rumble_deg.to_radians() * shake.rumble * shake.strength;
    let w = std::f32::consts::TAU * sc.rumble_hz;
    let seated = curr.mode == PilotMode::Seated;
    // The jolt: the eye dips and springs back on a footstep, a landing, a ram or a punch, seated only.
    let eye = if seated { eye - Vec3::Y * shake.dip_m() } else { eye };
    let shake = Quat::from_euler(
        EulerRot::YXZ,
        amp * (t * 37.0).sin() * (t * 5.3).cos() + buzz * (t * w).sin(),
        amp * (t * 41.0 + 1.3).sin() + buzz * (t * w * 1.37 + 0.7).sin(),
        amp * 0.5 * (t * 29.0 + 2.1).sin() + buzz * 0.5 * (t * w * 0.71 + 1.9).sin(),
    );
    // The stride's bob and sway stay off the view (owner, 2026-10-25); a footstep reaches it only as a kick to this
    // shake (owner, 2026-10-29: "similar to the landing shake, but like 50% that strength").
    **camera = Transform::from_translation(eye).with_rotation(rotation * shake);
    // Measurement instrument (`cockpit-local-space` section 3): each frame's drawn yaw and the mech's drawn heading,
    // degrees, with the step fraction, for `tools/perf/view_trace.py` to find a saw-tooth in.
    if crate::cli::switched_off("CB_VIEW_TRACE") && curr.mode == PilotMode::Seated {
        let (yaw, _, _) = rotation.to_euler(EulerRot::YXZ);
        let heading = sims.iter().next().map_or(f32::NAN, |s| {
            cb_core::pilot::blend_rad(s.prev.heading_rad, s.curr.heading_rad, alpha)
        });
        info!(
            "VIEW_TRACE t={:.4} yaw={:.4} heading={:.4} alpha={alpha:.3}",
            time.elapsed_secs_f64(),
            yaw.to_degrees(),
            heading.to_degrees()
        );
    }
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

/// The one prompt for Use (survey W40): what is in reach, from the one selector (`interact::UseChoice`). One option
/// reads as it always has ("[E]  Step into the cockpit", "[E]  Get in", "[E]  Use the terminal..."); several are
/// listed with the selected one marked, and the mouse wheel moves the mark.
fn show_prompt(choice: Res<crate::interact::UseChoice>, data: Res<Data>, mut prompt: Single<&mut Text, With<Prompt>>) {
    let b = &data.0.bindings;
    let label = |keys: &[String]| keys.first().map_or_else(|| "?".to_string(), |k| cb_core::input::key_label(k));
    let want = choice.selector.lines(&label(&b.use_key), &label(&b.option_cycle)).join("\n");
    if prompt.0 != want {
        prompt.0 = want;
    }
}

/// What the pilot sees through, the one place that decides the camera's fog: in a cloud the view
/// fogs by the cloud density at the eye (the one cloud query); under water the air's fog stands
/// back and the water's own fog (`water_render`'s compose pass, which murks every pixel by the
/// water it crossed) is the fog; seated with the sensor head shot off, the view is the backup
/// cameras' (washed out, darker, tinted) until the all-view screen change draws them properly.
fn weather_and_screens(
    data: Res<Data>,
    weather: Res<crate::ground_fog::EyeWeather>,
    pilot: Res<PilotSim>,
    mechs: Query<(&crate::mech::MechSim, &crate::mech::MechSpec), With<crate::mech::Piloted>>,
    mut camera: Single<(&mut DistanceFog, &mut ColorGrading, Option<&crate::water_render::WaterEye>), With<PilotCamera>>,
) {
    let (fog, grading, water) = &mut *camera;
    // Under water there is no cloud at the eye; the water fogs the view.
    let density = if water.is_some_and(|w| w.under_water()) {
        0.0
    } else {
        weather.cloud
    };
    let clear = data.0.world.view.clear_visibility_m;
    // Under the scattering sky, clear air has no flat fog at all: the atmosphere's aerial
    // perspective does the hazing, and a flat fog would cover the sky (sky-atmosphere).
    let clear = if crate::world::scattering_sky() { 1.0e9 } else { clear };
    let vis = clear + (25.0f32.min(clear) - clear) * density.sqrt();
    let inside = density.sqrt();
    fog.color = SKY.mix(&Color::srgb(0.9, 0.92, 0.95), inside);
    // Under the scattering sky the atmosphere hazes the distance itself, and a flat fog would paint
    // over the whole sky (it reaches full strength at infinite distance): in clear air the fog's
    // blend factor (its alpha) is zero, and it comes in only inside a cloud (sky-atmosphere).
    if crate::world::scattering_sky() {
        fog.color.set_alpha(inside);
    }
    fog.falloff = FogFalloff::from_visibility_colors(
        vis,
        Color::srgb(0.45, 0.55, 0.68).mix(&Color::srgb(0.85, 0.87, 0.9), inside),
        Color::srgb(0.80, 0.86, 0.95).mix(&Color::srgb(0.95, 0.95, 0.97), inside),
    );
    let head_lost = pilot.curr.mode == PilotMode::Seated && mechs.iter().any(|(m, spec)| m.damage.is_lost(&spec.parts, PartKind::Head));
    // The world's colour grade from data; a lost head washes it out on top (backup cameras).
    let view = &data.0.world.view;
    let (sat, exp, tint) = if head_lost { (0.25, -0.6, 0.15) } else { (1.0, 0.0, 0.0) };
    let sat = sat * view.grade_saturation;
    if grading.global.post_saturation != sat || grading.midtones.contrast != view.grade_contrast {
        grading.global.post_saturation = sat;
        grading.global.exposure = exp;
        grading.global.tint = tint;
        grading.global.temperature = view.grade_temperature;
        let g = &mut **grading;
        for section in [&mut g.shadows, &mut g.midtones, &mut g.highlights] {
            section.contrast = view.grade_contrast;
        }
    }
}
