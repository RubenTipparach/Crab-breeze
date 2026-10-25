// The water surface (openspec change `water-rendering`): the one material every river, lake and
// the ocean is drawn with. A Bevy material in the transmissive phase: it reads the opaque scene
// behind it (`view_transmission_texture`) and the ground's depth (the depth prepass, which the
// water is not in), and writes an opaque colour. Linear HDR colour, reverse-Z depth.
//
// Provenance: Pale Blue Dot (`RubenTipparach/pale-blue-dot` at `db25fe2`,
// `assets/shaders/water.wgsl` `fragment`, the cap), Tenebris's `water.fs.glsl` in WGSL. Changes:
// the planet's radial is world up; the cap's own pass and cell buffer become a mesh material; the
// swell (Pale Blue Dot's `sea.wgsl`) is three components in the normal, never in the height, so
// the surface stays at the simulation's `surface_m`; flow is a two-phase flow map from the mesh's
// vertex colour; foam gains a shore term; day and night are dropped; Bevy's own fog fogs it; the
// distance murk under water is left to `water_post.wesl`'s compose.

import bevy_pbr::render::mesh_view_bindings::view;
@if(VIEW_TRANSMISSION_TEXTURE)
import bevy_pbr::render::mesh_view_bindings::{view_transmission_texture, view_transmission_sampler};
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;
@if(DISTANCE_FOG)
import bevy_pbr::render::{mesh_view_bindings::fog, pbr_functions::apply_fog};
import super::water_common::{safe_normal, fbm3, water_transmittance, water_murk};

// Matches `WaterSurfaceUniform` in `crates/cb-app/src/water_render.rs`.
struct WaterSurface {
    ripple: vec4<f32>,      // noise cells per metre, speed, steepness, slope cap
    motion: vec4<f32>,      // scaled seconds, detail fade, flow period s, real seconds
    refraction: vec4<f32>,  // amount, max uv, max path m, light falloff 0..1
    absorption: vec4<f32>,  // rgb per metre; w the least reflection at a grazing angle
    deep: vec4<f32>,        // rgb the water body's colour (an albedo); w the eye's submersion (0 dry, 1 under)
    light: vec4<f32>,       // rgb the light reaching the water, times exposure; w spare
    sky_horizon: vec4<f32>, // rgb the sky at the horizon (the game's sky), linear; w spare
    sky_zenith: vec4<f32>,  // rgb the sky straight up, linear; w spare
    foam_color: vec4<f32>,  // rgb (an albedo); w intensity
    foam_crest: vec4<f32>,  // lo, hi, weight; w shore foam depth m
    foam_slope: vec4<f32>,  // lo, hi, weight; w shore foam weight
    sun_dir: vec4<f32>,     // xyz toward the sun; w glint intensity
    sun_light: vec4<f32>,   // rgb the sun's light times exposure; w glint power
    sun_tint: vec4<f32>,    // rgb the glint's tint; w spare
    cabin: vec4<f32>,       // xyz the eye; w the sealed cabin's dry radius m (0: none)
    swell: array<vec4<f32>, 3>, // heading x, heading z, wavenumber rad/m, height m
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> water: WaterSurface;

// The mesh vertex shader's output, the fields the water reads (bevy_pbr `forward_io::VertexOutput`
// has fixed locations).
struct WaterFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @if(VERTEX_COLORS)
    @location(5) color: vec4<f32>,
}

// The ripples at `xz` (world metres): x their height, yz their slope in x and z.
fn ripples(xz: vec2<f32>, t: f32, detail: f32) -> vec3<f32> {
    let p = vec3<f32>(xz.x, 0.0, xz.y) * water.ripple.x;
    let h = fbm3(p, t);
    let hx = fbm3(p + vec3<f32>(0.08, 0.0, 0.0), t);
    let hz = fbm3(p + vec3<f32>(0.0, 0.0, 0.08), t);
    return vec3<f32>(h, hx - h, hz - h) * vec3<f32>(detail, 12.5 * detail, 12.5 * detail);
}

// The swell's slope at `xz`: deep-water waves (omega squared = g k), each faded out once a pixel
// spans a quarter of its wavelength so the far sea does not sparkle.
fn swell_slope(xz: vec2<f32>, seconds: f32, metres_per_pixel: f32) -> vec2<f32> {
    var g = vec2<f32>(0.0);
    for (var i = 0u; i < 3u; i++) {
        let w = water.swell[i];
        let k = max(w.z, 1e-4);
        let wavelength = 6.2831853 / k;
        let fade = 1.0 - smoothstep(0.25, 0.5, metres_per_pixel / wavelength);
        let phase = dot(w.xy, xz) * k - sqrt(9.81 * k) * seconds;
        g += w.xy * (k * w.w * cos(phase) * fade);
    }
    return g;
}

fn world_at(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let h = view.world_from_clip * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    return h.xyz / max(abs(h.w), 1e-8) * sign(h.w);
}

// The ground's depth at a pixel (the depth prepass, without the water), 0 where it is sky or no
// prepass exists.
fn ground_depth(pixel: vec2<f32>) -> f32 {
    var depth = 0.0;
    @if(DEPTH_PREPASS) {
        depth = prepass_depth(vec4<f32>(pixel, 0.0, 0.0), 0u);
    }
    return depth;
}

// The opaque scene behind the water at `uv`, or the deep colour when there is no copy of it.
fn scene_behind(uv: vec2<f32>, fallback: vec3<f32>) -> vec3<f32> {
    var scene = fallback;
    @if(VIEW_TRANSMISSION_TEXTURE) {
        scene = textureSampleLevel(view_transmission_texture, view_transmission_sampler, uv, 0.0).rgb;
    }
    return scene;
}

@fragment
fn fragment(in: WaterFragment, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let p = in.world_position.xyz;
    let xz = p.xz;
    // Footprints first, in uniform control flow.
    let noise_footprint = length(fwidth(vec3<f32>(xz.x, 0.0, xz.y) * water.ripple.x));
    let metres_per_pixel = length(fwidth(xz));
    // The sealed cabin is dry: no water surface inside it.
    if (water.cabin.w > 0.0 && distance(p, water.cabin.xyz) < water.cabin.w) {
        discard;
    }
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let eye = view.world_position;
    let seconds = water.motion.w;
    let t = water.motion.x * water.ripple.y;
    // Not Tenebris's: once the fbm's features fall under a pixel its gradient is noise, and Fresnel
    // turns that into sparkle across the far water. Fade by the footprint (Pale Blue Dot).
    let detail = 1.0 / (1.0 + noise_footprint * max(water.motion.y, 0.0));

    // Flow: rivers carry it in vertex colour (rg, metres a second); lakes and the sea are still.
    var flow = vec2<f32>(0.0);
    @if(VERTEX_COLORS) {
        flow = in.color.rg;
    }
    var r: vec3<f32>;
    if (dot(flow, flow) > 1e-8) {
        // Two phases half a period apart, cross-faded so neither is seen as it resets: the noise
        // moves with the water and never stretches at a bend.
        let period = max(water.motion.z, 0.1);
        let ph0 = fract(seconds / period);
        let ph1 = fract(seconds / period + 0.5);
        let a = ripples(xz - flow * (ph0 * period), t, detail);
        let b = ripples(xz - flow * (ph1 * period) + vec2<f32>(17.3, 9.1), t, detail);
        r = mix(a, b, abs(1.0 - 2.0 * ph0));
    } else {
        r = ripples(xz, t, detail);
    }
    let height = r.x;
    let gradient = r.yz;
    // Slope cap before the gradient bends the normal; foam reads the raw slope.
    let raw_slope = length(gradient);
    let capped = gradient * min(1.0, max(water.ripple.w, 0.0) / max(raw_slope, 1e-6));
    let swell = swell_slope(xz, seconds, metres_per_pixel);
    let bend = swell + capped * water.ripple.z;
    let normal = safe_normal(up - vec3<f32>(bend.x, 0.0, bend.y));

    // Where this pixel is on screen, and the ground under the water there.
    var size = view.viewport.zw;
    @if(VIEW_TRANSMISSION_TEXTURE) {
        size = vec2<f32>(textureDimensions(view_transmission_texture));
    }
    let uv = in.position.xy / size;
    let own_ground = ground_depth(in.position.xy);
    // Refraction: shift the sample by the ripples' slope on screen; reject a sample across the sky
    // boundary or of something in front of the water.
    let screen_gradient = (view.clip_from_world * vec4<f32>(bend.x, 0.0, bend.y, 0.0)).xy;
    var offset = screen_gradient * vec2<f32>(1.0, -1.0) * water.refraction.x;
    offset *= min(1.0, max(water.refraction.y, 0.0) / max(length(offset), 1e-6));
    var refracted_uv = clamp(uv + offset, vec2<f32>(0.0), vec2<f32>(1.0));
    var refracted_ground = ground_depth(refracted_uv * size);
    if ((own_ground <= 1e-7) != (refracted_ground <= 1e-7) || refracted_ground > in.position.z + 1e-6) {
        refracted_uv = uv;
        refracted_ground = own_ground;
    }
    let deep_lit = water.deep.rgb * water.light.rgb;
    let scene = scene_behind(refracted_uv, deep_lit);
    let to_eye = eye - p;
    let eye_distance = length(to_eye);
    let look = safe_normal(to_eye);
    let absorption = max(water.absorption.rgb, vec3<f32>(0.0));

    // Seen from below: only by an eye in the water (`deep.w`, its submersion), and then decided by
    // where the eye is, not by the polygon's winding (nothing is culled); `front` only breaks the
    // tie for an eye exactly on the sheet. A dry eye lower than the water (a river or a lake up on
    // the range seen from the lowlands) sees it from above: taking that for "below" drew every
    // high river and lake as black murk (owner, 2026-10-25: "the rivers have missing geometry").
    // Snell's window: the sky straight up, the murk at the eye's depth outside it. The distance
    // murk is compose's.
    let in_water = water.deep.w > 0.25;
    let below = in_water && (eye.y < p.y || (eye.y == p.y && !front));
    if (below) {
        let deep = water_murk(water.deep.rgb, water.light.rgb, absorption, p.y - eye.y);
        let window = smoothstep(0.55, 0.75, dot(-look, normal));
        return vec4<f32>(mix(deep, scene, window), 1.0);
    }

    // Seen from above. The path the light behind took through the water, capped.
    var path = max(water.refraction.z, 0.0);
    var scene_lit = scene;
    if (refracted_ground > 1e-7) {
        let behind = world_at(refracted_uv, refracted_ground);
        path = min(path, max(length(behind - eye) - eye_distance, 0.0));
        // The light reaching the drowned ground crossed the water above it first.
        scene_lit *= water_transmittance(absorption * water.refraction.w, p.y - behind.y);
    }
    let transmitted = mix(deep_lit, scene_lit, water_transmittance(absorption, path));
    // The sky mirrored: Schlick's Fresnel, floored at a grazing angle.
    let reflected_ray = reflect(-look, normal);
    let reflection_height = clamp(reflected_ray.y, 0.0, 1.0);
    let fresnel = (0.02 + 0.98 * pow(1.0 - max(dot(look, normal), 0.0), 5.0))
        * mix(clamp(water.absorption.w, 0.0, 1.0), 1.0, reflection_height);
    let sky = mix(water.sky_horizon.rgb, water.sky_zenith.rgb, reflection_height);
    // Foam on crests, steep ripples and along the shore (the water under this pixel is shallow).
    let crest = smoothstep(water.foam_crest.x, max(water.foam_crest.y, water.foam_crest.x + 1e-4), height) * water.foam_crest.z;
    let steep = smoothstep(water.foam_slope.x, max(water.foam_slope.y, water.foam_slope.x + 1e-4), raw_slope) * water.foam_slope.z;
    var shore = 0.0;
    if (own_ground > 1e-7 && water.foam_crest.w > 0.0) {
        let water_depth = max(p.y - world_at(uv, own_ground).y, 0.0);
        let near = 1.0 - smoothstep(0.0, water.foam_crest.w, water_depth);
        shore = near * smoothstep(-0.25, 0.25, height + 0.5 * near - 0.25) * water.foam_slope.w;
    }
    let foam = clamp(max(max(crest, steep) * water.foam_color.w, shore), 0.0, 1.0);
    var color = mix(mix(transmitted, sky, fresnel), water.foam_color.rgb * water.light.rgb, foam);
    // The sun's glint.
    let sun = safe_normal(water.sun_dir.xyz);
    let half_vector = safe_normal(look + sun);
    color += water.sun_tint.rgb * water.sun_light.rgb
        * pow(max(dot(normal, half_vector), 0.0), max(water.sun_light.w, 1.0)) * water.sun_dir.w;
    var out = vec4<f32>(color, 1.0);
    // Bevy's own fog, the camera's one fog in air, so the water fogs with everything else.
    @if(DISTANCE_FOG) {
        out = apply_fog(fog, out, p, eye, in.position.xy);
    }
    return out;
}
