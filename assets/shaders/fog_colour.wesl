// The fog's colour along a view direction (openspec change `ground-fog`, design section 2b): the one copy, imported by
// the ground fog pass (`water_post.wesl`), which mixes the land, the sea and the sky toward it, and by the clouds
// (`cloud.wesl`), which take the same aerial perspective (owner, 2026-10-27: "cloud fog blend should be a thing"). With
// Bevy's scattering sky it is the sky's own colour toward the horizon that way, read from the sky-view LUT, so far land,
// sea and cloud meet the sky in one colour. Each caller binds the LUT and its sampler itself and hands them in.

// The fog's colour inputs; Rust fills both callers' fields from one place (`water_render::FogTint` and `SkyFrame`).
struct FogColour {
    sky_x: vec4<f32>,   // the scattering sky's frame round the eye: xyz its x axis; w the eye's distance from the planet's centre m
    sky_y: vec4<f32>,   // xyz its up; w the planet's radius m
    sky_z: vec4<f32>,   // xyz its back; w 1 with the sky's LUT bound, 0 without (the fog's own colour)
    sky_lut: vec4<f32>, // xy the sky-view LUT's size, texels; z the view's exposure; w how much darker the fog is under the clouds' shade
    ambient: vec4<f32>, // rgb the sky's light the fog scatters back; w the phase asymmetry g
    sun: vec4<f32>,     // rgb the sun's light the fog scatters (storm shade in it); w unused here
    sun_dir: vec4<f32>, // xyz toward the sun; w unused here
    sun_body: vec4<f32>, // xyz toward the sun itself, whose glow the sky-view LUT holds (`sun_dir` is the key light's, the moon's at night); w unused
    glow: vec4<f32>,    // x, y the distance (m) over which the glow toward the sun comes in (`FogConfig::sun_glow_m`); zw unused
}

// The scattering sky's colour along `d`, a direction in the sky's own frame (`FogColour`'s axes), as Bevy's sky pass
// reads it from the sky-view LUT (`bevy_pbr` atmosphere `sample_sky_view_lut`, its uv mapping written out here), times
// the exposure.
fn sky_lut_colour(lut: texture_2d<f32>, lut_sampler: sampler, c: FogColour, d: vec3<f32>) -> vec3<f32> {
    let horizon_zenith = 3.14159265 - acos(sky_limb_dip_sin(c));
    let view_zenith = acos(clamp(d.y, -1.0, 1.0));
    let l = view_zenith - horizon_zenith;
    let v = 0.5 + 0.5 * sign(l) * sqrt(abs(l) / 1.57079633);
    let u = atan2(d.x, -d.z) * 0.15915494 + 0.5;
    let size = max(c.sky_lut.xy, vec2<f32>(1.0));
    let uv = (vec2<f32>(u, v) + 0.5 / size) * (size / (size + 1.0));
    return textureSampleLevel(lut, lut_sampler, uv, 0.0).rgb * c.sky_lut.z;
}

// The sine of the dip of the planet's limb below level at the eye (the horizon lies a little below level, the more so
// the higher the eye).
fn sky_limb_dip_sin(c: FogColour) -> f32 {
    let r = c.sky_x.w;
    let inner = c.sky_y.w;
    return clamp(sqrt(max(r * r - inner * inner, 0.0)) / r, -1.0, 1.0);
}

// `dir` in the sky's own frame.
fn sky_frame(c: FogColour, dir: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(dir, c.sky_x.xyz), dot(dir, c.sky_y.xyz), dot(dir, c.sky_z.xyz));
}

// How much of the glow toward the sun the fog over a sight line `len` metres long carries, 0..1 (`FogConfig::sun_glow_m`).
// The sky-view LUT is the light scattered along a ray through the whole atmosphere: toward a low sun its forward
// scattering makes the horizon hundreds of times brighter than across from it. A sight line a few hundred metres long
// through the ground fog scatters nothing like that, and taking the LUT's colour for it painted that glow over houses
// and trees a hundred metres off at dawn and dusk (owner, 2026-10-30: "dusk and dawn shouldnt be drawing atmosphere
// infront of everything like this").
fn sun_glow_share(c: FogColour, len: f32) -> f32 {
    return smoothstep(c.glow.x, max(c.glow.y, c.glow.x + 1.0), len);
}

// The sky's colour along `dir` with no glow toward the sun: the sky at the same bearing turned out to at least a right
// angle from the sun's, and never below the horizon. Across from the sun the LUT holds the plain air's colour; never
// below the horizon so it does not jump from the horizon's colour to the ground's at the level line (that jump drew a
// hard line across the land with the haze above it only).
fn sky_plain_colour(lut: texture_2d<f32>, lut_sampler: sampler, c: FogColour, dir: vec3<f32>) -> vec3<f32> {
    let d = sky_frame(c, dir);
    let s = sky_frame(c, c.sun_body.xyz);
    var bearing = d.xz;
    let sun_bearing = s.xz;
    if (dot(sun_bearing, sun_bearing) > 1e-8) {
        let toward = normalize(sun_bearing);
        let across = vec2<f32>(-toward.y, toward.x);
        let side = select(-1.0, 1.0, dot(bearing, across) >= 0.0);
        if (dot(bearing, toward) > 0.0) {
            bearing = across * side;
        }
    }
    bearing = select(normalize(bearing), vec2<f32>(1.0, 0.0), dot(bearing, bearing) < 1e-8);
    // Just above the planet's limb (0.01 rad), or the ray's own height if it looks higher.
    let y = max(d.y, -sky_limb_dip_sin(c) + 0.01);
    let level = sqrt(max(1.0 - y * y, 0.0));
    return sky_lut_colour(lut, lut_sampler, c, vec3<f32>(bearing.x * level, y, bearing.y * level));
}

// The scattering sky's colour along `dir`. A ray below the horizon takes the LUT's own light along a downward ray: at
// twilight the horizon toward the sun glows far brighter than the air between the eye and the ground, and taking the
// horizon's colour for every downward ray turned the haze over the land red-hot (day-night-cycle, 2026-10-28).
fn sky_horizon_colour(lut: texture_2d<f32>, lut_sampler: sampler, c: FogColour, dir: vec3<f32>) -> vec3<f32> {
    return sky_lut_colour(lut, lut_sampler, c, sky_frame(c, dir));
}

// The fog's colour seen along `dir` over a sight line `len` metres long. With the scattering sky it is the sky's own
// colour toward the horizon that way, so the far land and sea fade into the sky at the horizon in one colour, as they
// do from the air (owner, 2026-10-26: "the distant ground is grey as well, it should match the fog color the horizon
// color should be uniform", "we need to assume distant horizons are ocean+fog color"), darkened under the clouds'
// shade; nearer than `FogConfig::sun_glow_m` the glow toward the sun leaves it (`sun_glow_share`). Without the sky: the
// sky's light scattered back, and the sun's scattered forward (Henyey-Greenstein), so it glows toward the sun, with the
// same share of the glow by distance.
fn fog_colour(lut: texture_2d<f32>, lut_sampler: sampler, c: FogColour, dir: vec3<f32>, len: f32) -> vec3<f32> {
    let glow = sun_glow_share(c, len);
    if (c.sky_z.w > 0.5) {
        var colour = sky_horizon_colour(lut, lut_sampler, c, dir);
        // The plain colour only ever takes the glow away, never adds light: a downward ray at dusk reads the dark
        // air below the horizon, while the plain colour is the sky just above it, the brightest band at twilight,
        // and taking it turned the far city into a pale wall at 19:30 (2026-10-30).
        if (glow < 1.0) {
            colour = mix(min(sky_plain_colour(lut, lut_sampler, c, dir), colour), colour, glow);
        }
        return colour * (1.0 - clamp(c.sky_lut.w, 0.0, 1.0));
    }
    let g = c.ambient.w;
    let along = dot(dir, normalize(c.sun_dir.xyz));
    let cosine = mix(min(along, 0.0), along, glow);
    let phase = (1.0 - g * g) / (12.566371 * pow(max(1.0 + g * g - 2.0 * g * cosine, 1e-4), 1.5));
    return c.ambient.rgb + c.sun.rgb * phase;
}
