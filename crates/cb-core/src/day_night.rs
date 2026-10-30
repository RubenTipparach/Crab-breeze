//! The time of day (openspec change `day-night-cycle`; owner, 2026-10-27: "24 mins for day time, 24
//! minutes for night"): the world's one clock, the sun's and the moon's paths across the sky, the
//! moon's phases, and the light and exposure each hour gives. Everything that changes with the hour
//! (the lights, the exposure, the morning mist, the ambience) reads it here, so no two parts of the
//! game disagree on the time. Pure; `cb-app` steps the clock and lights the world from `lighting`.
//!
//! Directions are in the world's frame: +x east, -z north, +y up.

use crate::data::{Validate, within};
use crate::world::terrain::smoothstep;
use glam::Vec3;
use serde::Deserialize;

/// How the hour runs (survey N7: an Options setting, and a mission may set its own hour).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum ClockMode {
    /// The hour runs.
    #[default]
    Cycle,
    /// Held at noon.
    Day,
    /// Held at midnight.
    Night,
}

/// `data/world/day_night.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DayNightConfig {
    /// The hour a world begins at (survey N1: 07:00, in the morning mist).
    pub start_h: f32,
    /// Game hours a real second: 1/120, so the twelve hours of day take 24 minutes and the twelve
    /// of night 24.
    pub hours_per_s: f32,
    /// Whether the hour runs or is held (survey N7).
    pub mode: ClockMode,
    /// The world's latitude, degrees north: the sun peaks at `90 - latitude` degrees at noon.
    pub latitude_deg: f32,
    /// The moon's phase on the first night, 0 new, 0.5 full (survey N2: real phases; the first
    /// nights begin full so a new player's first night can be seen).
    pub moon_start_phase: f32,
    /// Days from one new moon to the next.
    pub moon_period_days: f32,
    /// The exposure at night, EV100 (the day's is `view.exposure_ev100`).
    pub night_ev100: f32,
    /// Sun elevations, degrees, over which night's exposure and light give way to day's.
    pub dusk_deg: [f32; 2],
    /// How bright the flat ambient is at night on screen, a share of the day's.
    pub night_ambient_share: f32,
    /// How bright the sky's light (its reflections and fill) is at night on screen, a share of
    /// the day's.
    pub night_sky_light_share: f32,
    /// How bright a full moon's light is on screen, a share of the sun's at noon.
    pub moon_full_share: f32,
    /// The moon's shadow cascades (openspec change `night-frame`, NF2): a low moon's cascade boxes run far back toward
    /// it and take in every caster along the way, so it casts in fewer and nearer than the sun.
    pub moon_cascades: u32,
    /// ...out to this far from the eye, metres.
    pub moon_shadow_far_m: f32,
    /// A body's direct light fades in over this many degrees above the horizon (the disc rising
    /// and the air thinning toward it), degrees.
    pub horizon_fade_deg: f32,
    /// The stars: how bright the brightest is on screen against the night sky, 0..1 of white.
    pub star_brightness: f32,
    /// Sun elevations, degrees, over which the stars come out (fully out at the first).
    pub stars_deg: [f32; 2],
    /// The share of the sky's cells holding a star, 0..1.
    pub star_share: f32,
    /// The sky's own brightness on screen once the stars are fully out, against its brightness as the atmosphere
    /// draws it at the night's exposure, 0..1: a darker night sky the stars stand out on (owner, 2026-10-30: "make
    /// night sky darker and have stars out").
    pub night_sky_dim: f32,
    /// The moon's disc across, degrees (the real moon is 0.52; drawn a little larger to read).
    pub moon_disc_deg: f32,
    /// The lit face of a full moon on screen, against white (1).
    pub moon_disc_brightness: f32,
    /// The street lamps (`crate::world::lamps`).
    pub lamps: crate::world::lamps::LampConfig,
    /// The buildings' lit windows.
    pub windows: WindowLights,
    /// The lit rooms behind the windows and the glass at night (`crate::night_light`).
    pub rooms: crate::night_light::RoomLights,
    /// The pilot's own faint light on foot at night.
    pub self_light: SelfLight,
}

/// The pilot's own light on foot (owner, 2026-10-29: "the player needs a light, so like when I walk in to a an area, I
/// can still see a little bit. just a dim blue subtle self light like in dark souls"): a dim blue point light just
/// above their head, no shadows, that comes up as the day goes, so a dark room or a moonless street keeps its floor
/// and walls a few metres round them. Off seated or driving: the cockpit and the car have their own.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SelfLight {
    /// Its light at full night, lumens (at night's exposure, `night_ev100`).
    pub lumens: f32,
    /// Its colour, sRGB 0..1.
    pub colour: [f32; 3],
    /// How far it reaches, metres.
    pub range_m: f32,
    /// How far above the eye it hangs, metres.
    pub above_eye_m: f32,
}

impl Default for SelfLight {
    fn default() -> Self {
        Self {
            lumens: 40.0,
            colour: [0.55, 0.7, 1.0],
            range_m: 8.0,
            above_eye_m: 0.3,
        }
    }
}

impl SelfLight {
    /// Its light now, lumens: full at night, none by day, fading through dusk with `Lighting::day`.
    pub fn lumens_at(&self, light: &Lighting) -> f32 {
        self.lumens * (1.0 - light.day).clamp(0.0, 1.0)
    }

    fn check(&self) -> Result<(), String> {
        within("self_light.lumens", self.lumens, 0.0, 10_000.0)?;
        for c in self.colour {
            within("self_light.colour", c, 0.0, 1.0)?;
        }
        within("self_light.range_m", self.range_m, 0.1, 100.0)?;
        within("self_light.above_eye_m", self.above_eye_m, -2.0, 2.0)
    }
}

/// The buildings' windows at night (owner, 2026-10-28: "buildings also need some sort of cell
/// lighting. If the building is damaged all of its lights turn off"): each window a cell of its
/// facade, one storey high and `column_m` wide, lit when its own hash falls under the share of the
/// hour. A damaged building's windows are never lit (the glass shader knows a damaged building by its
/// frame's element slots).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WindowLights {
    /// The share of windows lit through the day: (hour, share) points, hours rising from 0 to 24,
    /// straight lines between and round midnight.
    pub lit_by_hour: Vec<[f32; 2]>,
    /// A window cell's width along its facade, metres (one storey high).
    pub column_m: f32,
    /// A lit window's brightness, cd/m2 (times the exposure on screen).
    pub cd_m2: f32,
}

impl Default for WindowLights {
    fn default() -> Self {
        Self {
            lit_by_hour: vec![
                [0.0, 0.35],
                [3.0, 0.12],
                [5.5, 0.12],
                [7.0, 0.0],
                [17.5, 0.0],
                [20.0, 0.65],
                [24.0, 0.35],
            ],
            column_m: 2.5,
            cd_m2: 3.0,
        }
    }
}

impl WindowLights {
    /// The share of windows lit at `hour`, 0..1.
    pub fn lit_share(&self, hour: f32) -> f32 {
        let h = hour.rem_euclid(24.0);
        let pts = &self.lit_by_hour;
        for w in pts.windows(2) {
            let ([h0, s0], [h1, s1]) = (w[0], w[1]);
            if h >= h0 && h <= h1 {
                let t = if h1 > h0 { (h - h0) / (h1 - h0) } else { 0.0 };
                return (s0 + (s1 - s0) * t).clamp(0.0, 1.0);
            }
        }
        pts.first().map_or(0.0, |p| p[1].clamp(0.0, 1.0))
    }

    fn check(&self) -> Result<(), String> {
        if self.lit_by_hour.len() < 2 {
            return Err("`windows.lit_by_hour` needs at least two points".into());
        }
        for w in self.lit_by_hour.windows(2) {
            if w[1][0] < w[0][0] {
                return Err(format!("`windows.lit_by_hour` hours must rise, got {:?} then {:?}", w[0], w[1]));
            }
        }
        for p in &self.lit_by_hour {
            within("windows.lit_by_hour hour", p[0], 0.0, 24.0)?;
            within("windows.lit_by_hour share", p[1], 0.0, 1.0)?;
        }
        within("windows.column_m", self.column_m, 0.5, 20.0)?;
        within("windows.cd_m2", self.cd_m2, 0.0, 1.0e5)
    }
}

impl Default for DayNightConfig {
    fn default() -> Self {
        Self {
            start_h: 7.0,
            hours_per_s: 1.0 / 120.0,
            mode: ClockMode::Cycle,
            latitude_deg: 30.0,
            moon_start_phase: 0.5,
            moon_period_days: 29.53,
            night_ev100: 0.4,
            dusk_deg: [-8.0, 6.0],
            night_ambient_share: 0.10,
            night_sky_light_share: 0.06,
            moon_full_share: 0.35,
            moon_cascades: 2,
            moon_shadow_far_m: 250.0,
            horizon_fade_deg: 2.5,
            star_brightness: 0.8,
            stars_deg: [-14.0, -4.0],
            star_share: 0.045,
            night_sky_dim: 1.0,
            moon_disc_deg: 0.8,
            moon_disc_brightness: 1.6,
            lamps: crate::world::lamps::LampConfig::default(),
            windows: WindowLights::default(),
            rooms: crate::night_light::RoomLights::default(),
            self_light: SelfLight::default(),
        }
    }
}

impl Validate for DayNightConfig {
    fn validate(&self) -> Result<(), String> {
        within("start_h", self.start_h, 0.0, 24.0)?;
        within("hours_per_s", self.hours_per_s, 0.0, 10.0)?;
        within("latitude_deg", self.latitude_deg, -80.0, 80.0)?;
        within("moon_start_phase", self.moon_start_phase, 0.0, 1.0)?;
        within("moon_period_days", self.moon_period_days, 1.0, 100.0)?;
        within("night_ev100", self.night_ev100, -6.0, 20.0)?;
        ordered("dusk_deg", self.dusk_deg)?;
        within("night_ambient_share", self.night_ambient_share, 0.0, 1.0)?;
        within("night_sky_light_share", self.night_sky_light_share, 0.0, 1.0)?;
        within("moon_full_share", self.moon_full_share, 0.0, 1.0)?;
        within("moon_cascades", self.moon_cascades as f32, 1.0, 4.0)?;
        within("moon_shadow_far_m", self.moon_shadow_far_m, 10.0, 5000.0)?;
        within("horizon_fade_deg", self.horizon_fade_deg, 0.01, 20.0)?;
        within("star_brightness", self.star_brightness, 0.0, 10.0)?;
        within("star_share", self.star_share, 0.0, 1.0)?;
        within("night_sky_dim", self.night_sky_dim, 0.01, 1.0)?;
        within("moon_disc_deg", self.moon_disc_deg, 0.05, 10.0)?;
        within("moon_disc_brightness", self.moon_disc_brightness, 0.0, 20.0)?;
        self.lamps.validate()?;
        self.windows.check()?;
        self.rooms.check()?;
        self.self_light.check()?;
        ordered("stars_deg", self.stars_deg)
    }
}

/// A range of elevations, degrees: both ends finite, the first below the second.
fn ordered(field: &str, r: [f32; 2]) -> Result<(), String> {
    within(field, r[0], -90.0, 90.0)?;
    within(field, r[1], -90.0, 90.0)?;
    if r[0] < r[1] {
        Ok(())
    } else {
        Err(format!("`{field}` must run from low to high, got {r:?}"))
    }
}

/// The world's clock: game hours since midnight of the first day (f64, so a long session keeps
/// its seconds). Advanced in the fixed step, so the same steps give the same hour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldClock {
    /// Game hours since the first midnight.
    pub hours: f64,
    /// Held where it is (free roam's pause menu: "freeze ToD"; owner, 2026-10-28).
    pub frozen: bool,
}

impl WorldClock {
    /// A world beginning at `cfg.start_h` of the first day.
    pub fn start(cfg: &DayNightConfig) -> Self {
        Self {
            hours: f64::from(cfg.start_h),
            frozen: false,
        }
    }

    /// Runs the clock `dt_s` real seconds (held by `cfg.mode`).
    pub fn step(&mut self, cfg: &DayNightConfig, dt_s: f32) {
        if !self.frozen && cfg.mode == ClockMode::Cycle && dt_s.is_finite() && dt_s > 0.0 {
            self.hours += f64::from(cfg.hours_per_s) * f64::from(dt_s);
        }
    }

    /// The hour of the day, 0..24 (noon when held at day, midnight at night).
    pub fn hour(&self, cfg: &DayNightConfig) -> f32 {
        match cfg.mode {
            ClockMode::Cycle => self.hours.rem_euclid(24.0) as f32,
            ClockMode::Day => 12.0,
            ClockMode::Night => 0.0,
        }
    }

    /// Sets the hour of the day to `hour` (0..24), on the same day (free roam's pause menu: "a toggle
    /// to pick time of day"; owner, 2026-10-28). A non-finite hour changes nothing.
    pub fn set_hour(&mut self, hour: f32) {
        if hour.is_finite() {
            self.hours = (self.hours / 24.0).floor() * 24.0 + f64::from(hour.rem_euclid(24.0));
        }
    }

    /// Moves the clock `hours` on (or back, never before the first midnight).
    pub fn shift(&mut self, hours: f32) {
        if hours.is_finite() {
            self.hours = (self.hours + f64::from(hours)).max(0.0);
        }
    }

    /// Whole days since the first midnight.
    pub fn day(&self) -> u32 {
        (self.hours / 24.0).floor().max(0.0) as u32
    }

    /// The moon's phase now: 0 new, 0.5 full, advancing through `moon_period_days`.
    pub fn moon_phase(&self, cfg: &DayNightConfig) -> f32 {
        let days = (self.hours - f64::from(cfg.start_h)) / 24.0;
        (f64::from(cfg.moon_start_phase) + days / f64::from(cfg.moon_period_days)).rem_euclid(1.0) as f32
    }
}

/// The direction toward a body on the celestial equator (the sun at the equinox) whose hour angle
/// is `hour_angle_deg` (0 at its highest, due south; negative before), from `latitude_deg`.
fn equatorial_dir(hour_angle_deg: f32, latitude_deg: f32) -> Vec3 {
    let h = hour_angle_deg.to_radians();
    let lat = latitude_deg.to_radians();
    // East, north and up, then into the world's frame (north is -z).
    let east = -h.sin();
    let north = -lat.sin() * h.cos();
    let up = lat.cos() * h.cos();
    Vec3::new(east, up, -north).normalize()
}

/// The direction toward the sun at `hour` (0..24): it rises due east at 06:00, stands highest due
/// south at 12:00 and sets due west at 18:00.
pub fn sun_dir(hour: f32, latitude_deg: f32) -> Vec3 {
    equatorial_dir((hour - 12.0) * 15.0, latitude_deg)
}

/// The direction toward the moon at `hour` and `phase`: it trails the sun by its phase of a day,
/// so a new moon rides with the sun and a full one opposite it, rising at sunset.
pub fn moon_dir(hour: f32, phase: f32, latitude_deg: f32) -> Vec3 {
    equatorial_dir((hour - 12.0) * 15.0 - phase * 360.0, latitude_deg)
}

/// The share of the moon's face that is lit at `phase`, 0 new to 1 full.
pub fn lit_fraction(phase: f32) -> f32 {
    (1.0 - (phase * std::f32::consts::TAU).cos()) * 0.5
}

/// A direction's elevation above the horizon, degrees.
pub fn elevation_deg(dir: Vec3) -> f32 {
    dir.y.clamp(-1.0, 1.0).asin().to_degrees()
}

/// How much of a body's direct light reaches the ground at `elevation_deg`: none below the
/// horizon, all from `fade_deg` up.
pub fn above_horizon(elevation_deg: f32, fade_deg: f32) -> f32 {
    smoothstep(-0.5, fade_deg, elevation_deg)
}

/// Which body's light casts the shadows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowBody {
    /// The sun, while it is up.
    Sun,
    /// The moon, while the sun is down and the moon is up and lit.
    Moon,
    /// Neither: a night with the moon down or new.
    None,
}

/// The light of the hour, as everything that lights the world reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    /// The hour, 0..24.
    pub hour: f32,
    /// Toward the sun.
    pub sun_dir: Vec3,
    /// Toward the moon.
    pub moon_dir: Vec3,
    /// The moon's phase, 0 new, 0.5 full.
    pub moon_phase: f32,
    /// How much of the sun's direct light reaches the ground, 0..1.
    pub sun_visible: f32,
    /// The moon's light as a share of the sun's noon light on screen (its phase, its height).
    pub moon_share: f32,
    /// 0 at night, 1 by day: what the exposure and the sky's light follow.
    pub day: f32,
    /// The exposure, EV100.
    pub ev100: f32,
    /// What the flat ambient is multiplied by, against the day's (its share on screen, through the
    /// exposure).
    pub ambient_scale: f32,
    /// What the sky's light is multiplied by, against the day's.
    pub sky_light_scale: f32,
    /// How far the stars are out, 0..1.
    pub stars: f32,
    /// Whose shadows are cast.
    pub shadows: ShadowBody,
}

/// What a light set for the day must be multiplied by to show `share` of its day brightness on
/// screen at exposure `ev100` (`day_ev100` the day's): the exposure is darker by `day - now` stops,
/// so the light is dimmer by as many.
pub fn screen_share(share: f32, ev100: f32, day_ev100: f32) -> f32 {
    share * 2f32.powf(ev100 - day_ev100)
}

/// The light at `clock`'s hour; `day_ev100` is the day's exposure (`view.exposure_ev100`).
pub fn lighting(cfg: &DayNightConfig, clock: &WorldClock, day_ev100: f32) -> Lighting {
    let hour = clock.hour(cfg);
    let phase = clock.moon_phase(cfg);
    let sun = sun_dir(hour, cfg.latitude_deg);
    let moon = moon_dir(hour, phase, cfg.latitude_deg);
    let sun_deg = elevation_deg(sun);
    let moon_deg = elevation_deg(moon);
    let day = smoothstep(cfg.dusk_deg[0], cfg.dusk_deg[1], sun_deg);
    let ev100 = cfg.night_ev100 + (day_ev100 - cfg.night_ev100) * day;
    let ambient_share = cfg.night_ambient_share + (1.0 - cfg.night_ambient_share) * day;
    let sky_share = cfg.night_sky_light_share + (1.0 - cfg.night_sky_light_share) * day;
    let sun_visible = above_horizon(sun_deg, cfg.horizon_fade_deg);
    let moon_share = cfg.moon_full_share * lit_fraction(phase) * above_horizon(moon_deg, cfg.horizon_fade_deg);
    let shadows = if sun_deg > 0.0 {
        ShadowBody::Sun
    } else if moon_share > 0.01 * cfg.moon_full_share {
        ShadowBody::Moon
    } else {
        ShadowBody::None
    };
    Lighting {
        hour,
        sun_dir: sun,
        moon_dir: moon,
        moon_phase: phase,
        sun_visible,
        moon_share,
        day,
        ev100,
        ambient_scale: screen_share(ambient_share, ev100, day_ev100),
        sky_light_scale: screen_share(sky_share, ev100, day_ev100),
        stars: 1.0 - smoothstep(cfg.stars_deg[0], cfg.stars_deg[1], sun_deg),
        shadows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pilot's own light is full at midnight and out at noon (owner, 2026-10-29: "a dim blue subtle self light").
    #[test]
    fn the_pilots_own_light_is_on_at_night_and_out_by_day() {
        let cfg = DayNightConfig::default();
        let at = |h: f64| {
            let clock = WorldClock {
                hours: 24.0 + h,
                ..WorldClock::start(&cfg)
            };
            cfg.self_light.lumens_at(&lighting(&cfg, &clock, 13.4))
        };
        assert!((at(0.0) - cfg.self_light.lumens).abs() < 1e-3, "midnight: full");
        assert_eq!(at(12.0), 0.0, "noon: out");
    }

    fn close(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn an_hour_takes_two_minutes_and_day_and_night_take_24_minutes_each() {
        let cfg = DayNightConfig::default();
        let mut clock = WorldClock::start(&cfg);
        let steps = 48 * 60 * 60; // 48 minutes at 60 steps a second
        for _ in 0..steps {
            clock.step(&cfg, 1.0 / 60.0);
        }
        // To within a second of game time (the step is an f32 sixtieth of a second).
        assert!(
            (clock.hours - (f64::from(cfg.start_h) + 24.0)).abs() < 1.0 / 3600.0,
            "{}",
            clock.hours
        );
    }

    #[test]
    fn the_same_steps_give_the_same_hour() {
        let cfg = DayNightConfig::default();
        let (mut a, mut b) = (WorldClock::start(&cfg), WorldClock::start(&cfg));
        for i in 0..5000 {
            let dt = 1.0 / 60.0 + (i % 3) as f32 * 1e-4;
            a.step(&cfg, dt);
            b.step(&cfg, dt);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn the_sun_rises_due_east_at_six_stands_sixty_degrees_up_due_south_at_noon_and_sets_due_west_at_six() {
        let lat = 30.0;
        let rise = sun_dir(6.0, lat);
        assert!(close(rise.y, 0.0, 1e-5) && close(rise.x, 1.0, 1e-5), "{rise}");
        let noon = sun_dir(12.0, lat);
        assert!(close(elevation_deg(noon), 60.0, 1e-3), "{noon}");
        assert!(noon.z > 0.0 && close(noon.x, 0.0, 1e-5), "south is +z: {noon}");
        let set = sun_dir(18.0, lat);
        assert!(close(set.y, 0.0, 1e-5) && close(set.x, -1.0, 1e-5), "{set}");
        assert!(sun_dir(0.0, lat).y < -0.8, "midnight: below");
    }

    #[test]
    fn a_full_moon_rides_opposite_the_sun_and_a_new_one_with_it() {
        let lat = 30.0;
        for hour in [0.0, 5.0, 12.0, 21.0] {
            let sun = sun_dir(hour, lat);
            assert!(moon_dir(hour, 0.5, lat).dot(sun) < -0.999, "full at {hour}");
            assert!(moon_dir(hour, 0.0, lat).dot(sun) > 0.999, "new at {hour}");
        }
        assert!(close(lit_fraction(0.5), 1.0, 1e-6) && close(lit_fraction(0.0), 0.0, 1e-6));
    }

    #[test]
    fn the_moon_goes_through_its_phases_in_its_period() {
        let cfg = DayNightConfig::default();
        let mut clock = WorldClock::start(&cfg);
        let first = clock.moon_phase(&cfg);
        assert!(close(first, cfg.moon_start_phase, 1e-6));
        clock.hours += f64::from(cfg.moon_period_days) * 24.0 / 2.0;
        assert!(
            close(clock.moon_phase(&cfg), (cfg.moon_start_phase + 0.5) % 1.0, 1e-4),
            "half a period: new"
        );
    }

    #[test]
    fn noon_is_lit_by_the_sun_at_the_day_exposure_and_a_full_moon_midnight_by_the_moon() {
        let cfg = DayNightConfig::default();
        let mut clock = WorldClock::start(&cfg);
        clock.hours = 12.0;
        let noon = lighting(&cfg, &clock, 13.4);
        assert_eq!(noon.shadows, ShadowBody::Sun);
        assert!(close(noon.ev100, 13.4, 1e-4) && close(noon.ambient_scale, 1.0, 1e-4));
        assert!(close(noon.stars, 0.0, 1e-6));
        clock.hours = 24.0; // the first midnight: the moon nearly full
        let midnight = lighting(&cfg, &clock, 13.4);
        assert_eq!(midnight.shadows, ShadowBody::Moon);
        assert!(close(midnight.ev100, cfg.night_ev100, 1e-4) && close(midnight.stars, 1.0, 1e-6));
        assert!(midnight.sun_visible == 0.0 && midnight.moon_share > 0.9 * cfg.moon_full_share);
        // On screen the night's ambient is its share of the day's.
        let on_screen = midnight.ambient_scale * 2f32.powf(13.4 - midnight.ev100);
        assert!(close(on_screen, cfg.night_ambient_share, 1e-5));
    }

    #[test]
    fn a_new_moon_night_casts_no_shadows() {
        let cfg = DayNightConfig {
            moon_start_phase: 0.0,
            ..DayNightConfig::default()
        };
        let clock = WorldClock { hours: 0.5, frozen: false };
        let l = lighting(&cfg, &clock, 13.4);
        assert_eq!(l.shadows, ShadowBody::None);
        assert!(l.moon_share < 1e-3);
    }

    #[test]
    fn windows_light_through_the_evening_and_go_out_by_morning() {
        let w = WindowLights::default();
        assert_eq!(w.lit_share(12.0), 0.0, "none by day");
        assert!((w.lit_share(20.0) - 0.65).abs() < 1e-6, "most at 20:00");
        assert!((w.lit_share(0.0) - 0.35).abs() < 1e-6 && (w.lit_share(24.0) - 0.35).abs() < 1e-6);
        assert!((w.lit_share(4.0) - 0.12).abs() < 1e-6, "a few all night");
        assert!(w.lit_share(18.75) > 0.0 && w.lit_share(18.75) < 0.65, "coming on at dusk");
    }

    #[test]
    fn a_frozen_clock_holds_its_hour_and_the_hour_can_be_picked() {
        let cfg = DayNightConfig::default();
        let mut clock = WorldClock::start(&cfg);
        clock.hours = 24.0 * 3.0 + 9.5;
        clock.frozen = true;
        clock.step(&cfg, 600.0);
        assert_eq!(clock.hour(&cfg), 9.5, "frozen");
        clock.set_hour(20.0);
        assert_eq!((clock.hour(&cfg), clock.day()), (20.0, 3), "picked on the same day");
        clock.shift(5.0);
        assert_eq!(
            (clock.hour(&cfg), clock.day()),
            (1.0, 4),
            "an hour on past midnight is the next day"
        );
        clock.set_hour(f32::NAN);
        assert_eq!(clock.hour(&cfg), 1.0);
        clock.frozen = false;
        clock.step(&cfg, 120.0);
        assert!((clock.hour(&cfg) - 2.0).abs() < 1e-4, "running again");
    }

    #[test]
    fn a_held_clock_stands_at_noon_or_midnight() {
        let mut cfg = DayNightConfig {
            mode: ClockMode::Day,
            ..DayNightConfig::default()
        };
        let mut clock = WorldClock::start(&cfg);
        clock.step(&cfg, 600.0);
        assert_eq!(clock.hour(&cfg), 12.0);
        cfg.mode = ClockMode::Night;
        assert_eq!(clock.hour(&cfg), 0.0);
    }

    #[test]
    fn the_shipped_day_night_data_loads_and_is_the_owners_cycle() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/day_night.ron");
        let cfg: DayNightConfig = crate::data::load_ron(&path).expect("day_night.ron");
        assert_eq!(cfg.start_h, 7.0, "survey N1");
        assert!(close(cfg.hours_per_s * 120.0, 1.0, 1e-4), "an hour a 120 s");
    }
}
