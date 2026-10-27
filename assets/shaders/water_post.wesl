// The water's fullscreen passes (openspec change `water-rendering`) on cameras with `WaterOptics`:
// `fog` (the ground fog, before the transparent pass), and before tonemapping `compose` (the view
// under water: murk by the distance travelled through water, light lost on its way down, the
// wobble, the waterline while straddling, and the distance softened while the glass is wet) and
// `lens` (drops running off the glass after surfacing, refracting the real scene). Linear HDR
// colour, reverse-Z depth (near 1, sky 0).
//
// Provenance: Pale Blue Dot (`RubenTipparach/pale-blue-dot` at `db25fe2`,
// `assets/shaders/water.wgsl`: `blur`, `compose`, `refract_through_drops`, `lens`), Tenebris's
// `composite.fs.glsl` in WGSL. Changes: the sea sphere is the plane `y = surface` under the eye;
// the sealed cabin (seated) is dry; the waterline while straddling is a wavy line across the view
// rather than Pale Blue Dot's sky-only band; the mist and its probe are left out.

import super::water_common::{safe_normal, water_transmittance, water_murk, rd_drops, layer_depth};

// Matches `WaterPostUniform` in `crates/cb-app/src/water_render.rs`; a test pins the size.
struct WaterPost {
    world_from_clip: mat4x4<f32>,
    camera: vec4<f32>,     // xyz the eye, world metres; w seconds
    surface: vec4<f32>,    // x the water surface's height under the eye, m; y the eye's depth below it, m; z the band, m; w submersion 0, 0.5 or 1
    absorption: vec4<f32>, // rgb per metre; w light falloff 0..1
    deep: vec4<f32>,       // rgb the water body's colour (an albedo); w the wobble, uv
    light: vec4<f32>,      // rgb the light reaching the water, times exposure; w wet blur 0..1
    lens: vec4<f32>,       // drop density, refraction, speed, grid size
    lens_fx: vec4<f32>,    // x drips 0..1 (surfacing), y rain on the glass 0..1, z the drops' scale, w the sealed cabin's dry radius m (0 on foot)
    fog_layer: vec4<f32>,  // ground fog: x density at sea level per m, y scale height m, z sea level m, w 1 on
    fog_mist: vec4<f32>,   // x mist strength, y the mist taps' reach m, z a sky ray's length m, w extra extinction per m (rain)
    fog_ambient: vec4<f32>, // rgb the sky's light the fog scatters back; w the phase asymmetry g
    fog_sun: vec4<f32>,    // rgb the sun's light the fog scatters (storm shade in it); w the haze's density at sea level per m
    fog_sun_dir: vec4<f32>, // xyz toward the sun; w the haze's scale height m
    fog_map: vec4<f32>,    // x the mist map's least corner m, y its cell m, z cells a side, w the rain's reach m
    fog_sky_x: vec4<f32>,  // the scattering sky's frame round the eye: xyz its x axis; w the eye's distance from the planet's centre m
    fog_sky_y: vec4<f32>,  // xyz its up; w the planet's radius m
    fog_sky_z: vec4<f32>,  // xyz its back; w 1 with the sky's LUT bound, 0 without (the fog's own colour)
    fog_sky_lut: vec4<f32>, // xy the sky-view LUT's size, texels; z the view's exposure; w how much darker the fog is under the clouds' shade
    glass: vec4<f32>,      // seated: xyz the interior's origin, world m; w window triangles in `panes` (0 on foot)
    glass_look: vec4<f32>, // x metres of pane to one unit of the drop pattern
    panes: array<vec4<f32>, 96>, // the window triangles' corners, world m, three a triangle
}

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> post: WaterPost;
@group(0) @binding(1) var scene_color: texture_2d<f32>;
@if(MULTISAMPLED)
@group(0) @binding(2) var scene_depth: texture_depth_multisampled_2d;
@if(!MULTISAMPLED)
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@group(0) @binding(3) var scene_sampler: sampler;
@group(0) @binding(4) var mist_map: texture_2d<f32>;
@group(0) @binding(5) var sky_view_lut: texture_2d<f32>;

fn load_depth(uv: vec2<f32>) -> f32 {
    let size = vec2<i32>(textureDimensions(scene_depth));
    let coord = clamp(vec2<i32>(uv * vec2<f32>(size)), vec2<i32>(0), size - 1);
    return textureLoad(scene_depth, coord, 0);
}

fn sample_scene(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(scene_color, scene_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0).rgb;
}

fn world_at(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let h = post.world_from_clip * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    return h.xyz / max(abs(h.w), 1e-8) * sign(h.w);
}

// Reverse-Z depth of a point far down the frustum: the direction of a pixel never comes from a sky
// pixel's (absent) depth. Pale Blue Dot's `VIEW_RAY_DEPTH`.
const VIEW_RAY_DEPTH: f32 = 1.0e-4;

fn view_direction(uv: vec2<f32>) -> vec3<f32> {
    return safe_normal(world_at(uv, VIEW_RAY_DEPTH) - post.camera.xyz);
}

// While straddling, the share of the view under water along `direction`: a wavy waterline across
// the view that rises as the eye sinks through the band.
fn waterline_wet(direction: vec3<f32>, uv: vec2<f32>) -> f32 {
    let band = max(post.surface.z, 1e-4);
    let sink = clamp(post.surface.y / band, -1.0, 1.0);
    let t = post.camera.w;
    let wave = 0.012 * sin(uv.x * 23.0 + t * 2.1) + 0.006 * sin(uv.x * 51.0 - t * 3.3);
    return 1.0 - smoothstep(-0.015, 0.015, direction.y - 0.15 * sink - wave);
}

// How much of a pixel `distance` metres away is outside the sealed cabin (1 on foot).
fn beyond_cabin(distance: f32, sky: bool) -> f32 {
    let cabin = post.lens_fx.w;
    if (cabin <= 0.0 || sky) {
        return 1.0;
    }
    return smoothstep(cabin, cabin + 0.3, distance);
}

// 17-tap two-ring blur, radius growing with focus.
fn blur(uv: vec2<f32>, focus: f32) -> vec3<f32> {
    let r = focus * 0.0016;
    var s = sample_scene(uv);
    var w = 1.0;
    let taps = array<vec2<f32>, 8>(vec2(r, 0.), vec2(0., r), vec2(r, r) * 0.707, vec2(r, -r) * 0.707,
        vec2(r * 2.2, 0.), vec2(0., r * 2.2), vec2(r * 2.2, r * 2.2) * 0.707, vec2(r * 2.2, -r * 2.2) * 0.707);
    for (var i = 0u; i < 8u; i++) {
        let d = taps[i];
        s += sample_scene(uv + d);
        s += sample_scene(uv - d);
        w += 2.0;
    }
    return s / w;
}

// The mist (0..1) at `xz` from the baked map (`cb_core::world::mist::MistMap`).
fn mist_at(xz: vec2<f32>) -> f32 {
    let uv = (xz - vec2<f32>(post.fog_map.x)) / (post.fog_map.y * max(post.fog_map.z, 1.0));
    return textureSampleLevel(mist_map, scene_sampler, uv, 0.0).r;
}

// The ground fog's optical depth along a ray from `origin` going `dir` for `len` metres: the layer's
// exact integral (`cb_core::world::mist::optical_depth`, written with both ends' exponentials),
// thickened by the mist read at four points along the ray, plus the rain's haze.
fn fog_depth(origin: vec3<f32>, dir: vec3<f32>, len: f32) -> f32 {
    let sea = post.fog_layer.z;
    let reach = min(len, post.fog_mist.y);
    var mist = 0.0;
    for (var k = 0u; k < 4u; k++) {
        let f = array<f32, 4>(0.1, 0.3, 0.6, 1.0)[k];
        mist += mist_at((origin + dir * (reach * f)).xz);
    }
    let thick = 1.0 + post.fog_mist.x * mist * 0.25;
    let layer = layer_depth(post.fog_layer.x, post.fog_layer.y, sea, origin.y, dir.y, len);
    // The aerial haze (ground-fog design section 2a): thin and deep, not thickened by mist.
    let haze = layer_depth(post.fog_sun.w, post.fog_sun_dir.w, sea, origin.y, dir.y, len);
    // The rain's haze only as far as the rain reaches (`cb_core::world::mist::rain_depth`).
    return thick * max(layer, 0.0) + max(haze, 0.0) + post.fog_mist.w * min(len, post.fog_map.w);
}

// The scattering sky's colour along `dir` at or above the horizon, as Bevy's sky pass reads it
// from the sky-view LUT (`bevy_pbr` atmosphere `sample_sky_view_lut`, its uv mapping written out
// here), times the exposure. A ray below the horizon takes the horizon's colour just above it.
fn sky_horizon_colour(dir: vec3<f32>) -> vec3<f32> {
    let d = vec3<f32>(dot(dir, post.fog_sky_x.xyz), dot(dir, post.fog_sky_y.xyz), dot(dir, post.fog_sky_z.xyz));
    let r = post.fog_sky_x.w;
    let inner = post.fog_sky_y.w;
    let horizon_zenith = 3.14159265 - acos(clamp(sqrt(max(r * r - inner * inner, 0.0)) / r, -1.0, 1.0));
    // Half a degree above the horizon at the lowest: the LUT's horizon texel row, not the ground's.
    let view_zenith = min(acos(clamp(d.y, -1.0, 1.0)), horizon_zenith - 0.009);
    let l = view_zenith - horizon_zenith;
    let v = 0.5 + 0.5 * sign(l) * sqrt(abs(l) / 1.57079633);
    let u = atan2(d.x, -d.z) * 0.15915494 + 0.5;
    let size = max(post.fog_sky_lut.xy, vec2<f32>(1.0));
    let uv = (vec2<f32>(u, v) + 0.5 / size) * (size / (size + 1.0));
    return textureSampleLevel(sky_view_lut, scene_sampler, uv, 0.0).rgb * post.fog_sky_lut.z;
}

// The fog's colour seen along `dir`. With the scattering sky it is the sky's own colour toward the
// horizon that way, so the far land and sea fade into the sky at the horizon in one colour, as they
// do from the air (owner, 2026-10-26: "the distant ground is grey as well, it should match the fog
// color the horizon color should be uniform", "we need to assume distant horizons are ocean+fog
// color"), darkened under the clouds' shade. Without it: the sky's light scattered back, and the
// sun's scattered forward (Henyey-Greenstein), so it glows toward the sun.
fn fog_colour(dir: vec3<f32>) -> vec3<f32> {
    if (post.fog_sky_z.w > 0.5) {
        return sky_horizon_colour(dir) * (1.0 - clamp(post.fog_sky_lut.w, 0.0, 1.0));
    }
    let g = post.fog_ambient.w;
    let c = dot(dir, normalize(post.fog_sun_dir.xyz));
    let phase = (1.0 - g * g) / (12.566371 * pow(max(1.0 + g * g - 2.0 * g * c, 1e-4), 1.5));
    return post.fog_ambient.rgb + post.fog_sun.rgb * phase;
}

// Ground fog in the air, from the cabin's wall (seated) to the pixel or out to the sky's length.
// Its own pass between the opaque (and transmissive) pass and the transparent one, so the clouds,
// which write no depth, are drawn over the fogged ground and sky and carry only their own fog: fogged
// afterwards by what lay behind them, they wore the band of thick fog where the far sea meets the
// sky as a line across them (owner, 2026-10-26: "notice the horizon line showing throuhg the
// clouds?"). Under water the compose pass's murk is the only fog.
@fragment
fn fog(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = sample_scene(uv);
    let depth = load_depth(uv);
    let sky = depth <= 1e-7;
    let eye = post.camera.xyz;
    let direction = view_direction(uv);
    var view_dist = 1.0e5;
    if (!sky) {
        view_dist = length(world_at(uv, depth) - eye);
    }
    let start = post.lens_fx.w;
    let end = select(view_dist, post.fog_mist.z, sky);
    let len = end - start;
    if (len > 0.0) {
        let t = exp(-fog_depth(eye + direction * start, direction, len));
        color = mix(fog_colour(direction), color, t);
    }
    return vec4<f32>(color, 1.0);
}

@fragment
fn compose(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let depth = load_depth(uv);
    let sky = depth <= 1e-7;
    let eye = post.camera.xyz;
    let direction = view_direction(uv);
    let surface = post.surface.x;
    let under = post.surface.w > 0.75;
    let straddle = post.surface.w > 0.25 && !under;
    // How far the eye is below the surface (0 above it).
    let cam_gap = max(0.0, surface - eye.y);
    var view_dist = 1.0e5;
    var point = eye + direction * view_dist;
    if (!sky) {
        point = world_at(uv, depth);
        view_dist = length(point - eye);
    }
    // Wet: the whole view under water, the part below the waterline while straddling, nothing in
    // air (the surface material shows what is under the water from above). The sealed cabin is
    // dry.
    var wet = select(select(0.0, waterline_wet(direction, uv), straddle), 1.0, under);
    wet *= beyond_cabin(view_dist, sky);
    var color = sample_scene(uv);
    if (wet > 0.0) {
        if (post.deep.w > 0.0) {
            let time = post.camera.w;
            let wobble = vec2<f32>(sin(time * 1.3 + uv.y * 32.0) + 0.5 * sin(time * 0.7 + uv.x * 47.0),
                cos(time * 0.9 + uv.x * 28.0) + 0.5 * cos(time * 1.6 + uv.y * 41.0));
            color = sample_scene(uv + wobble * post.deep.w * wet);
        }
        // Water travelled: to the pixel, starting at the cabin wall when seated; a sky pixel under
        // water is the exit through the surface, or murk all the way for a ray that never rises.
        var travel = max(view_dist - post.lens_fx.w, 0.0);
        if (sky) {
            travel = select(1.0e5, cam_gap / direction.y, direction.y > 0.0008 && cam_gap > 0.0);
        }
        let absorption = post.absorption.rgb;
        // Light reaching a drowned surface crossed the water above it first.
        var lit = color;
        if (!sky) {
            lit *= water_transmittance(absorption * post.absorption.w, surface - point.y);
        }
        let deep = water_murk(post.deep.rgb, post.light.rgb, absorption, cam_gap);
        color = mix(color, mix(deep, lit, water_transmittance(absorption, travel)), wet);
    }
    // Depth blur while the glass is wet: the distance softens, the foreground stays crisp.
    let dz_amt = max(post.lens_fx.y, post.lens_fx.x);
    if (dz_amt > 0.001 && !under) {
        let dz = smoothstep(28.0, 190.0, view_dist) * clamp(dz_amt, 0.0, 1.0);
        if (dz > 0.002) {
            color = mix(color, blur(uv, dz * 3.5), dz * clamp(post.light.w, 0.0, 1.0));
        }
    }
    return vec4<f32>(color, 1.0);
}

// The drops at `drop_uv` (the reference's y-up frame), refracting the scene at `uv` (Bevy's y-down
// frame): the refracted colour and the drops' coverage.
fn refract_through_drops(uv: vec2<f32>, drop_uv: vec2<f32>, t: f32, l0: f32, l1: f32, l2: f32) -> vec4<f32> {
    let density = post.lens.x;
    let size = post.lens.w;
    let c = rd_drops(drop_uv, t, l0, l1, l2, density, size);
    let e = vec2<f32>(0.001, 0.0);
    let cx = rd_drops(drop_uv + e, t, l0, l1, l2, density, size).x;
    let cy = rd_drops(drop_uv + e.yx, t, l0, l1, l2, density, size).x;
    // A gradient in the y-up drop frame, negated in y for the y-down sample. Divided by the drops'
    // scale, so each drop still bends the same share of its own width however small it is drawn.
    let n = vec2<f32>(cx - c.x, -(cy - c.x));
    let bend = post.lens.y / max(post.lens_fx.z, 0.05);
    let refracted = sample_scene(uv - n * bend);
    return vec4<f32>(refracted, clamp(c.x, 0.0, 1.0));
}

// Where the view along `dir` crosses the nearest window pane: xy the point in the pane's own plane
// (metres from the interior's origin, x across, y up the pane, so drops run down it), z 1 on a pane
// and 0 through none.
fn pane_uv(dir: vec3<f32>) -> vec3<f32> {
    let eye = post.camera.xyz;
    let count = min(u32(post.glass.w + 0.5), 32u);
    var best = 1.0e9;
    var out = vec3<f32>(0.0);
    for (var i = 0u; i < count; i++) {
        let a = post.panes[i * 3u].xyz;
        let b = post.panes[i * 3u + 1u].xyz;
        let c = post.panes[i * 3u + 2u].xyz;
        let n = cross(b - a, c - a);
        let facing = dot(n, dir);
        if (abs(facing) < 1e-9) {
            continue;
        }
        let t = dot(n, a - eye) / facing;
        if (t <= 0.0 || t >= best) {
            continue;
        }
        let p = eye + dir * t;
        // Inside: on the same side of all three edges.
        let s0 = dot(cross(b - a, p - a), n);
        let s1 = dot(cross(c - b, p - b), n);
        let s2 = dot(cross(a - c, p - c), n);
        if (!((s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0))) {
            continue;
        }
        best = t;
        // The pane's frame from its plane alone, so the two triangles of a quad (and every pane in
        // one plane) share it: across is level, up runs up the pane.
        let nn = normalize(n);
        var across = cross(vec3<f32>(0.0, 1.0, 0.0), nn);
        if (length(across) < 0.1) {
            across = cross(vec3<f32>(1.0, 0.0, 0.0), nn);
        }
        across = normalize(across);
        var up = cross(nn, across);
        if (up.y < 0.0) {
            up = -up;
        }
        let q = p - post.glass.xyz;
        out = vec3<f32>(dot(q, across), dot(q, up), 1.0);
    }
    return out;
}

@fragment
fn lens(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = sample_scene(uv);
    let under = post.surface.w > 0.75;
    if (under) {
        return vec4<f32>(color, 1.0);
    }
    let depth = load_depth(uv);
    let sky = depth <= 1e-7;
    var view_dist = 1.0e5;
    if (!sky) {
        view_dist = length(world_at(uv, depth) - post.camera.xyz);
    }
    // Straddling, drops bead only on the view above the waterline; seated, only on what the
    // screens show past the hull (the drops are on the external camera lenses).
    var glass = beyond_cabin(view_dist, sky);
    if (post.surface.w > 0.25) {
        glass *= 1.0 - waterline_wet(view_direction(uv), uv);
    }
    // The drop math is the reference's, whose full-screen uv is y-up; Bevy's is y-down.
    let size = vec2<f32>(textureDimensions(scene_color));
    var drop_uv = vec2<f32>(uv.x - 0.5, 0.5 - uv.y);
    drop_uv.x *= max(size.x / max(size.y, 1.0), 0.1);
    // Seated, the drops are on the window panes, not on the eye: laid in the plane of the pane the
    // view crosses, so they stay on the glass as the head turns (owner, 2026-10-25: "the rain drops
    // on the cocpit are not projected to the panes of window correctly, they are projected into
    // camera space"). A pixel through no pane gets none.
    if (post.glass.w > 0.5) {
        let on_pane = pane_uv(view_direction(uv));
        glass *= on_pane.z;
        drop_uv = on_pane.xy / max(post.glass_look.x, 0.01);
    }
    drop_uv *= max(post.lens_fx.z, 0.05);
    let t = post.camera.w * 0.2 * post.lens.z;
    // Rain on the glass (gated off until weather exists).
    let rain = clamp(post.lens_fx.y, 0.0, 1.0);
    if (rain > 0.001) {
        let s = smoothstep(-0.5, 1.0, rain) * 2.0;
        let l1 = smoothstep(0.25, 0.75, rain);
        let l2 = smoothstep(0.0, 0.5, rain);
        let drops = refract_through_drops(uv, drop_uv, t, s, l1, l2);
        color = mix(color, drops.rgb, drops.a * rain * glass);
    }
    // Surfacing: the water runs off.
    let emerge = clamp(post.lens_fx.x, 0.0, 1.0);
    if (emerge > 0.001) {
        let r = refract_through_drops(uv, drop_uv, t, 0.0, 1.0, 0.6);
        color = mix(color, r.rgb, r.a * emerge * glass);
    }
    return vec4<f32>(color, 1.0);
}
