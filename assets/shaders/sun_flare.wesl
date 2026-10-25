// The sun's lens flare (openspec change `sun-flare`), drawn additively into the pilot's eyes'
// linear HDR image after the bloom and before tonemapping, and the cockpit's share of it.
//
// `flare_vertex` / `flare_fragment`: a fullscreen triangle. The vertex shader works out once, per
// vertex, how much of the sun is seen: the world's depth sampled at 17 taps round the sun
// (`tap_visibility`), times the share the cockpit's frame left clear last frame (`cockpit_seen`, a
// 1x1 texture), times what the CPU decided (on screen, cloud and water at the eye). The fragment
// shader draws every element as a function of the pixel, the sun's screen position and the centre:
// a warm core and glow, a starburst of spikes and fine rays, an anamorphic streak, a halo ring and a
// chain of tinted ghosts (discs and the aperture's hexagons) with chromatic fringes on the line from
// the sun through the centre.
//
// `taps_vertex` / `taps_fragment`: run on the cockpit camera into the 1x1 texture, the same taps
// against the cockpit's own depth (its frame and pillars; the screens are open).
//
// Positions are in screen heights: the origin at the centre, y up, one screen height is 1.

// Matches `SunFlareUniform` in `crates/cb-app/src/sun_flare.rs`; a test pins the size.
struct SunFlare {
    sun: vec4<f32>,         // xy the sun, screen heights; z the aspect (width over height); w visibility from the CPU, 0..1
    light: vec4<f32>,       // rgb the flare's strength (the sun's exposed light times the gain); w the starburst's turn, radians
    core: vec4<f32>,        // rgb; w radius
    glow: vec4<f32>,        // rgb; w half-strength radius
    spikes: vec4<f32>,      // rgb; w length
    spike_shape: vec4<f32>, // x count, y sharpness, z fringe, w ray count
    rays: vec4<f32>,        // x strength, y length, z glow power, w ghost count
    streak: vec4<f32>,      // rgb; w length
    halo: vec4<f32>,        // rgb; w radius
    shape: vec4<f32>,       // x streak half thickness, y halo half width, z halo fringe, w unused
    ghost_look: vec4<f32>,  // x fringe, y rim, z hexagon turn (radians), w unused
    ghosts: array<vec4<f32>, 8>,        // x offset (0 sun, 1 centre), y radius, z softness, w 1 hexagon or 0 disc
    ghost_colours: array<vec4<f32>, 8>, // rgb
    taps: array<vec4<f32>, 17>,         // xy uv, z weight, w 1 on screen
}

const TAPS: u32 = 17u;
// Reverse-Z: the sky's depth (`cb_core::sun_flare::SKY_DEPTH`).
const SKY_DEPTH: f32 = 1.0e-7;
// Red, green, blue spread for the chromatic fringes: red drawn larger, blue smaller.
const FRINGE: vec3<f32> = vec3<f32>(1.0, 0.0, -1.0);

@group(0) @binding(0) var<uniform> flare: SunFlare;
@if(MULTISAMPLED)
@group(0) @binding(1) var scene_depth: texture_depth_multisampled_2d;
@if(!MULTISAMPLED)
@group(0) @binding(1) var scene_depth: texture_depth_2d;
@group(0) @binding(2) var cockpit_seen: texture_2d<f32>;

struct FlareVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) seen: f32,
}

fn load_depth(uv: vec2<f32>) -> f32 {
    let size = vec2<i32>(textureDimensions(scene_depth));
    let coord = clamp(vec2<i32>(uv * vec2<f32>(size)), vec2<i32>(0), size - 1);
    return textureLoad(scene_depth, coord, 0);
}

// The share of the sun the depth leaves clear: `cb_core::sun_flare::depth_visibility` on the GPU.
fn tap_visibility() -> f32 {
    var clear = 0.0;
    var total = 0.0;
    for (var i = 0u; i < TAPS; i++) {
        let tap = flare.taps[i];
        let w = max(tap.z, 0.0);
        total += w;
        if (tap.w < 0.5 || load_depth(tap.xy) <= SKY_DEPTH) {
            clear += w;
        }
    }
    return select(1.0, clamp(clear / total, 0.0, 1.0), total > 0.0);
}

// Bevy's fullscreen triangle.
fn fullscreen(index: u32) -> FlareVertex {
    var out: FlareVertex;
    let uv = vec2<f32>(f32(index >> 1u), f32(index & 1u)) * 2.0;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    out.seen = 0.0;
    return out;
}

@vertex
fn flare_vertex(@builtin(vertex_index) index: u32) -> FlareVertex {
    var out = fullscreen(index);
    let cockpit = clamp(textureLoad(cockpit_seen, vec2<i32>(0), 0).r, 0.0, 1.0);
    out.seen = flare.sun.w * cockpit * tap_visibility();
    return out;
}

@vertex
fn taps_vertex(@builtin(vertex_index) index: u32) -> FlareVertex {
    var out = fullscreen(index);
    out.seen = tap_visibility();
    return out;
}

@fragment
fn taps_fragment(in: FlareVertex) -> @location(0) vec4<f32> {
    return vec4<f32>(in.seen, 0.0, 0.0, 1.0);
}

fn rotate(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

// A ghost's shape distance, 1 at its edge: a disc, or a hexagon by its inner radius.
fn ghost_distance(q: vec2<f32>, radius: f32, hexagon: bool) -> f32 {
    if (!hexagon) {
        return length(q) / radius;
    }
    let a = abs(rotate(q, flare.ghost_look.z));
    return max(a.x * 0.8660254 + a.y * 0.5, a.y) / radius;
}

// A soft-edged body with a brighter rim, from the shape distance (1 at the edge).
fn ghost_body(d: vec3<f32>, softness: f32) -> vec3<f32> {
    let body = 1.0 - smoothstep(vec3<f32>(1.0 - max(softness, 1.0e-3)), vec3<f32>(1.0), d);
    // Squared by hand: pow() of a negative base (inside the ghost, d < 0.96) is undefined in WGSL and
    // came out NaN on NVIDIA under Vulkan/DX12, drawing every ghost as a black shape.
    let r = (d - vec3<f32>(0.96)) / 0.05;
    let rim = exp(-(r * r)) * flare.ghost_look.y;
    return body * (0.55 + 0.45 * d * d) + rim * body;
}

// A cheap hash for the fine rays' lengths.
fn hash(n: f32) -> f32 {
    return fract(sin(n * 12.9898) * 43758.547);
}

fn flare_at(uv: vec2<f32>) -> vec3<f32> {
    let aspect = flare.sun.z;
    let p = vec2<f32>((uv.x - 0.5) * aspect, 0.5 - uv.y);
    let sun = flare.sun.xy;
    let d = p - sun;
    let r = length(d);
    var c = vec3<f32>(0.0);

    // The core and the glow.
    let core_r = flare.core.w;
    // The disk itself: a solid circle with a soft rim (owner: "sun should be a sizable circle like
    // in real life"), and a little bleed past it.
    c += flare.core.rgb * (1.0 - smoothstep(core_r * 0.82, core_r, r));
    c += flare.core.rgb * 0.25 * exp(-(r * r) / (4.0 * core_r * core_r));
    let g = r / flare.glow.w;
    c += flare.glow.rgb * pow(1.0 / (1.0 + g * g), flare.rays.z);

    // The starburst: blade spikes, red reaching a little further than blue, and fine rays of
    // uneven length between them, all turning with the view.
    let angle = atan2(d.y, d.x) + flare.light.w;
    let spike = pow(abs(cos(angle * flare.spike_shape.x * 0.5)), flare.spike_shape.y);
    let reach = flare.spikes.w * (vec3<f32>(1.0) + flare.spike_shape.z * FRINGE);
    c += flare.spikes.rgb * spike * exp(-vec3<f32>(r) / reach);
    let rays = flare.spike_shape.w;
    if (rays > 0.5) {
        let k = angle / 6.2831853 * rays;
        let ray = pow(abs(cos(k * 3.1415927)), flare.spike_shape.y * 0.25);
        // The ray's own index, wrapped so the ray across the angle's seam (straight left) is one ray.
        let index = floor(k + 0.5);
        let len = flare.rays.y * (0.45 + 0.55 * hash(index - rays * floor(index / rays)));
        c += flare.spikes.rgb * flare.rays.x * ray * exp(-r / len);
    }

    // The anamorphic streak.
    let t = d.y / flare.shape.x;
    c += flare.streak.rgb * exp(-t * t) * exp(-abs(d.x) / flare.streak.w);

    // The halo ring, red outside and blue inside.
    let ring = flare.halo.w * (vec3<f32>(1.0) + flare.shape.z * FRINGE);
    let ring_t = (vec3<f32>(r) - ring) / flare.shape.y;
    c += flare.halo.rgb * exp(-ring_t * ring_t);

    // The ghosts, on the line from the sun through the centre.
    let count = u32(flare.rays.w);
    let spread = vec3<f32>(1.0) + flare.ghost_look.x * FRINGE;
    for (var i = 0u; i < count; i++) {
        let ghost = flare.ghosts[i];
        let q = p - sun * (1.0 - ghost.x);
        // Skip the pixels well outside it.
        if (length(q) > ghost.y * 1.4 * spread.x) {
            continue;
        }
        let d0 = ghost_distance(q, ghost.y, ghost.w > 0.5);
        c += flare.ghost_colours[i].rgb * ghost_body(vec3<f32>(d0) / spread, ghost.z);
    }
    return c * flare.light.rgb;
}

@fragment
fn flare_fragment(in: FlareVertex) -> @location(0) vec4<f32> {
    if (in.seen <= 1.0e-4) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(flare_at(in.uv) * in.seen, 0.0);
}
