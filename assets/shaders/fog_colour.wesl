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
}

// The scattering sky's colour along `dir` at or above the horizon, as Bevy's sky pass reads it from the sky-view LUT
// (`bevy_pbr` atmosphere `sample_sky_view_lut`, its uv mapping written out here), times the exposure. A ray below the
// horizon takes the horizon's colour just above it.
fn sky_horizon_colour(lut: texture_2d<f32>, lut_sampler: sampler, c: FogColour, dir: vec3<f32>) -> vec3<f32> {
    let d = vec3<f32>(dot(dir, c.sky_x.xyz), dot(dir, c.sky_y.xyz), dot(dir, c.sky_z.xyz));
    let r = c.sky_x.w;
    let inner = c.sky_y.w;
    let horizon_zenith = 3.14159265 - acos(clamp(sqrt(max(r * r - inner * inner, 0.0)) / r, -1.0, 1.0));
    // Half a degree above the horizon at the lowest: the LUT's horizon texel row, not the ground's.
    let view_zenith = min(acos(clamp(d.y, -1.0, 1.0)), horizon_zenith - 0.009);
    let l = view_zenith - horizon_zenith;
    let v = 0.5 + 0.5 * sign(l) * sqrt(abs(l) / 1.57079633);
    let u = atan2(d.x, -d.z) * 0.15915494 + 0.5;
    let size = max(c.sky_lut.xy, vec2<f32>(1.0));
    let uv = (vec2<f32>(u, v) + 0.5 / size) * (size / (size + 1.0));
    return textureSampleLevel(lut, lut_sampler, uv, 0.0).rgb * c.sky_lut.z;
}

// The fog's colour seen along `dir`. With the scattering sky it is the sky's own colour toward the horizon that way, so
// the far land and sea fade into the sky at the horizon in one colour, as they do from the air (owner, 2026-10-26: "the
// distant ground is grey as well, it should match the fog color the horizon color should be uniform", "we need to
// assume distant horizons are ocean+fog color"), darkened under the clouds' shade. Without it: the sky's light scattered
// back, and the sun's scattered forward (Henyey-Greenstein), so it glows toward the sun.
fn fog_colour(lut: texture_2d<f32>, lut_sampler: sampler, c: FogColour, dir: vec3<f32>) -> vec3<f32> {
    if (c.sky_z.w > 0.5) {
        return sky_horizon_colour(lut, lut_sampler, c, dir) * (1.0 - clamp(c.sky_lut.w, 0.0, 1.0));
    }
    let g = c.ambient.w;
    let cosine = dot(dir, normalize(c.sun_dir.xyz));
    let phase = (1.0 - g * g) / (12.566371 * pow(max(1.0 + g * g - 2.0 * g * cosine, 1e-4), 1.5));
    return c.ambient.rgb + c.sun.rgb * phase;
}
