//! The time of day in the game (openspec change `day-night-cycle`): runs the world's clock in the
//! fixed step and lights the world from `cb_core::day_night::lighting` each frame. It moves the sun
//! and the moon (two directional lights; the one that is up casts the shadows) and sets the
//! exposure, the flat ambient and the sky's light. Everything that wants "the light on the world"
//! (the water, the clouds, the fog) takes the brighter body through `key_light`, so they never
//! disagree with what lights the ground. Its own module because no other capability owns the hour.
//!
//! `CB_HOUR=<h>` holds the clock at hour `h` of the second day (stills and benchmarks).

use crate::sim::{Data, PresentSet, SimSet};
use crate::state::GameApp;
use crate::world::Sun;
use bevy::light::{CascadeShadowConfigBuilder, EnvironmentMapLight};
use bevy::prelude::*;
use cb_core::day_night::{Lighting, ShadowBody, WorldClock, lighting};

/// The world's clock.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Clock(pub WorldClock);

/// This frame's light of the hour.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Light(pub Lighting);

/// The moon's light.
#[derive(Component)]
pub struct Moon;

/// A body in the sky that lights the world (the sun or the moon): how much of its direct light
/// reaches the ground now, 0..1. Bevy dims a directional light below the horizon itself on its own
/// materials; the game's own shaders take this instead.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SkyBody {
    /// The share of its light that reaches the ground.
    pub visible: f32,
}

/// The light on the world from the sky: whichever body gives more, its illuminance scaled by how
/// much of it reaches the ground. `None` before the lights exist.
pub fn key_light<'a>(
    bodies: impl Iterator<Item = (&'a DirectionalLight, &'a GlobalTransform, &'a SkyBody)>,
) -> Option<(DirectionalLight, GlobalTransform)> {
    bodies
        .map(|(l, t, b)| {
            let mut lit = *l;
            lit.illuminance *= b.visible.clamp(0.0, 1.0);
            (lit, *t)
        })
        .max_by(|a, b| a.0.illuminance.total_cmp(&b.0.illuminance))
}

/// The day's flat ambient, cd/m2 (the sky light's, scaled for the scattering sky's exposure): the one
/// place it is worked out (`world` inserts it, the hour scales it).
pub fn day_ambient_cd_m2(data: &cb_core::GameData) -> f32 {
    crate::sky_light::ambient_cd_m2(&data.sky_light) * crate::world::exposure_scale(&data.world.view)
}

/// The day's sky light intensity (`sky_light`'s environment map), the one place it is worked out.
pub fn day_sky_intensity(data: &cb_core::GameData) -> f32 {
    data.sky_light.sky_cd_m2 * crate::world::exposure_scale(&data.world.view)
}

/// Runs the hour.
pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        let data = app.world().resource::<Data>().0.clone();
        let clock = Clock(WorldClock::start(&data.day_night));
        let light = Light(lighting(&data.day_night, &clock.0, data.world.view.exposure_ev100));
        app.insert_resource(clock)
            .insert_resource(light)
            .add_world_spawn((restart_clock, spawn_moon))
            .add_game_systems(FixedUpdate, tick.in_set(SimSet::Input))
            .add_game_systems(Update, light_the_world.in_set(PresentSet::Pose));
    }
}

/// The hour `CB_HOUR` holds the clock at, if set.
fn held_hour() -> Option<f32> {
    std::env::var("CB_HOUR")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|h| h.is_finite())
}

/// A new world begins at the clock's start.
fn restart_clock(data: Res<Data>, mut clock: ResMut<Clock>) {
    clock.0 = WorldClock::start(&data.0.day_night);
}

/// The moon: a directional light like the sun's, with the same cascades, unshadowed until the sun
/// sets. No disc of Bevy's: the fog pass draws it with its phase.
fn spawn_moon(mut commands: Commands, data: Res<Data>) {
    let shade = &data.0.world.sun_shade;
    commands.spawn((
        Name::new("moon"),
        Moon,
        SkyBody::default(),
        DirectionalLight {
            illuminance: 0.0,
            color: Color::srgb(0.78, 0.84, 1.0),
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        CascadeShadowConfigBuilder {
            num_cascades: shade.live_cascades as usize,
            first_cascade_far_bound: shade.live_first_cascade_m,
            maximum_distance: shade.live_far_m,
            ..default()
        }
        .build(),
    ));
}

/// Runs the clock one fixed step.
pub(crate) fn tick(data: Res<Data>, time: Res<Time<Fixed>>, mut clock: ResMut<Clock>) {
    clock.0.step(&data.0.day_night, time.delta_secs());
    if let Some(h) = held_hour() {
        clock.0.hours = 24.0 + f64::from(h.rem_euclid(24.0));
    }
}

/// Lights the world for the hour: the sun and the moon, their shadows, the exposure, the ambient
/// and the sky's light.
#[allow(clippy::type_complexity)]
fn light_the_world(
    data: Res<Data>,
    clock: Res<Clock>,
    settings: Option<Res<crate::settings::Settings>>,
    mut light: ResMut<Light>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform, &mut SkyBody), (With<Sun>, Without<Moon>)>,
    mut moons: Query<(&mut DirectionalLight, &mut Transform, &mut SkyBody), (With<Moon>, Without<Sun>)>,
    mut camera: Query<(Option<&mut bevy::camera::Exposure>, Option<&mut EnvironmentMapLight>), With<crate::pilot::PilotCamera>>,
) {
    let d = &data.0;
    let day_ev100 = d.world.view.exposure_ev100;
    let l = lighting(&d.day_night, &clock.0, day_ev100);
    light.0 = l;
    let shadows_allowed = settings.is_none_or(|s| s.0.shadows) && !crate::cli::switched_off("CB_NO_SHADOWS");
    // A directional light shines along its -z: it looks away from the body.
    let facing = |toward: Vec3| Quat::from_rotation_arc(Vec3::Z, toward.normalize_or(Vec3::Y));
    for (mut sun, mut transform, mut body) in &mut suns {
        transform.rotation = facing(l.sun_dir);
        body.visible = l.sun_visible;
        let cast = shadows_allowed && l.shadows == ShadowBody::Sun;
        if sun.shadow_maps_enabled != cast {
            sun.shadow_maps_enabled = cast;
        }
    }
    for (mut moon, mut transform, mut body) in &mut moons {
        transform.rotation = facing(l.moon_dir);
        body.visible = (l.moon_share / d.day_night.moon_full_share.max(1e-6)).clamp(0.0, 1.0);
        // On screen, `moon_share` of the noon sun: the sun's raw light through the exposure's drop.
        moon.illuminance = light_consts::lux::RAW_SUNLIGHT * cb_core::day_night::screen_share(l.moon_share, l.ev100, day_ev100);
        let cast = shadows_allowed && l.shadows == ShadowBody::Moon;
        if moon.shadow_maps_enabled != cast {
            moon.shadow_maps_enabled = cast;
        }
    }
    ambient.brightness = day_ambient_cd_m2(d) * l.ambient_scale;
    for (exposure, sky) in &mut camera {
        if let Some(mut e) = exposure {
            e.ev100 = l.ev100;
        }
        if let Some(mut sky) = sky {
            sky.intensity = day_sky_intensity(d) * l.sky_light_scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_light_is_the_brighter_body_as_much_as_reaches_the_ground() {
        let t = GlobalTransform::IDENTITY;
        let sun = DirectionalLight {
            illuminance: 100.0,
            ..default()
        };
        let moon = DirectionalLight {
            illuminance: 2.0,
            ..default()
        };
        let (down, up) = (SkyBody { visible: 0.0 }, SkyBody { visible: 1.0 });
        let key = key_light([(&sun, &t, &down), (&moon, &t, &up)].into_iter()).unwrap();
        assert_eq!(key.0.illuminance, 2.0, "the sun below the horizon gives way to the moon");
        let half = SkyBody { visible: 0.5 };
        let key = key_light([(&sun, &t, &half), (&moon, &t, &up)].into_iter()).unwrap();
        assert_eq!(key.0.illuminance, 50.0);
    }
}
