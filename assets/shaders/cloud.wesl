// The sky's clouds (openspec change `cloud-weather`): every puff of the weather is one camera-facing
// billboard, its edge eaten by noise and lit as a sphere by the scene's sun and sky (the light the
// ground and the water get, through `water_light` in Rust); rain shafts are billboards turned about
// the vertical with streaks falling; cirrus are flat streaks high up. All three are items of one
// mesh, sorted back to front on the CPU, drawn in one alpha-blended draw. Each billboard fades
// where it meets the ground behind it (the depth prepass, "soft particles") and near the eye, where
// the cloud fog takes over; Bevy's fog hazes far ones. Linear HDR colour, reverse-Z depth.
//
// The look is the island mockup's (`docs/mockups/island.html`, `spriteMat`, `shaftMat`,
// `cirrusMat`), ported to WESL, with the scene's light in place of its fixed colours.
//
// Lighting of a puff, per pixel:
// - the sun wrapped round the sphere (cumulus scatter light deep into their shade side);
// - darker toward its system's base, lighter at the tops; storm grey puffs darker still;
// - each puff darker toward its own underside and near the system's base, which is cut flat (a
//   cumulus has a flat, shadowed base at the condensation level);
// - light scattered many times inside the cloud, so a puff with the sun behind it is grey, not black;
// - a silver lining where the eye looks toward the sun through a thin edge (forward scattering);
// - the ambient sky from above.
//
// Bevy binds the view's lights to the fragment stage only: the vertex stage never reads `lights`.

import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{view, globals};
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;
@if(DISTANCE_FOG)
import bevy_pbr::render::{mesh_view_bindings::fog, pbr_functions::apply_fog};

// Matches `CloudUniform` in `crates/cb-app/src/clouds_render.rs`.
struct CloudLight {
    sun_dir: vec4<f32>,  // xyz toward the sun; w wrap 0..1
    sun: vec4<f32>,      // rgb the sun's light times exposure; w silver lining strength
    ambient: vec4<f32>,  // rgb the ambient light times exposure; w light scattered inside, a share of the sun
    albedo: vec4<f32>,   // x white albedo, y storm albedo, z white base shade, w storm base shade
    fade: vec4<f32>,     // x fade-in distance from the eye m, y edge erosion, z soft depth m, w unused
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> cloud: CloudLight;

// What a billboard is (`look.x` of a vertex): the puff kinds of `cb_core::world::clouds::CloudKind`,
// then a rain shaft and a cirrus streak.
const KIND_CAP: f32 = 5.0;
const KIND_SHAFT: f32 = 7.0;
const KIND_CIRRUS: f32 = 8.0;

struct CloudVertex {
    @builtin(instance_index) instance_index: u32,
    // The item's centre at time zero (the transform carries the wind).
    @location(0) centre: vec3<f32>,
    // The quad's corner, -1..1.
    @location(1) corner: vec2<f32>,
    // A puff: rx across, ry up, seed, unused. A shaft: radius, height, seed, unused. Cirrus:
    // length, width, seed, heading (radians from +x toward +z).
    @location(2) shape: vec4<f32>,
    // x kind, y darkness 0..1, z opacity 0..1, w unused.
    @location(3) look: vec4<f32>,
    // x the base of its system, y its top (metres).
    @location(4) layer: vec2<f32>,
}

struct CloudFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) shape: vec4<f32>,
    @location(3) @interpolate(flat) look: vec4<f32>,
    @location(4) @interpolate(flat) layer: vec2<f32>,
}

@vertex
fn vertex(in: CloudVertex) -> CloudFragment {
    var out: CloudFragment;
    let drift = get_world_from_local(in.instance_index)[3].xyz;
    let c = in.centre + drift;
    let eye = view.world_position;
    let right = view.world_from_view[0].xyz;
    let up = view.world_from_view[1].xyz;
    let kind = in.look.x;
    var wp: vec3<f32>;
    if kind > KIND_CIRRUS - 0.5 {
        // A flat streak along its heading.
        let along = vec3<f32>(cos(in.shape.w), 0.0, sin(in.shape.w));
        let side = vec3<f32>(-along.z, 0.0, along.x);
        wp = c + along * (in.corner.x * 0.5 * in.shape.x) + side * (in.corner.y * 0.5 * in.shape.y);
    } else if kind > KIND_SHAFT - 0.5 {
        // Turned about the vertical to face the eye, from the ground up to its top.
        var to = eye - c;
        to.y = 0.0;
        let facing = normalize(to + vec3<f32>(1e-3, 0.0, 0.0));
        let across = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), facing));
        wp = c + across * (in.corner.x * in.shape.x) + vec3<f32>(0.0, in.corner.y * 0.5 * in.shape.y, 0.0);
    } else {
        // A flat puff seen from above is round: its height on screen grows toward its width.
        let k = abs(normalize(eye - c).y);
        wp = c + right * (in.corner.x * in.shape.x) + up * (in.corner.y * mix(in.shape.y, in.shape.x, k * k));
    }
    out.position = position_world_to_clip(wp);
    out.world = wp;
    out.uv = in.corner;
    out.shape = in.shape;
    out.look = in.look;
    out.layer = in.layer;
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q += dot(q, q + 45.32);
    return fract(q.x * q.y);
}

// Value noise, 0..1.
fn vn(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Three octaves of value noise, 0..1.
fn fbm2(p: vec2<f32>) -> f32 {
    return (0.5 * vn(p) + 0.25 * vn(p * 2.03 + 17.0) + 0.125 * vn(p * 4.07 + 31.0)) / 0.875;
}

// How much of the billboard shows in front of the ground behind it: 1 in open sky, fading to 0
// over `soft_m` metres in front of the ground (reverse-Z, infinite far plane: depth = near / z).
fn soft_edge(pixel: vec4<f32>) -> f32 {
    var soft = 1.0;
    @if(DEPTH_PREPASS) {
        let scene = prepass_depth(pixel, 0u);
        if scene > 0.0 {
            let near = view.clip_from_view[3][2];
            let gap = near / scene - near / max(pixel.z, 1e-9);
            soft = smoothstep(0.0, cloud.fade.z, gap);
        }
    }
    return soft;
}

fn puff(in: CloudFragment, look: vec3<f32>, dist: f32) -> vec4<f32> {
    let p = in.uv;
    let d = length(p);
    let seed = in.shape.z;
    let dark = in.look.y;
    // Billows: the edge eaten by two octaves of noise in the sprite's plane (caps stay smooth).
    let n = fbm2(p * 2.2 + seed * 13.1 + globals.time * 0.004);
    let n2 = vn(p * 7.0 + seed * 5.0);
    let erode = select(cloud.fade.y, 0.22, abs(in.look.x - KIND_CAP) < 0.5);
    let dens = smoothstep(1.0, 0.45, d + (n - 0.5) * erode + (n2 - 0.5) * 0.2);
    let right = view.world_from_view[0].xyz;
    let up = view.world_from_view[1].xyz;
    let back = view.world_from_view[2].xyz;
    let z = sqrt(max(0.0, 1.0 - d * d));
    let nrm = normalize(right * p.x + up * p.y + back * (z + 0.15) + 0.5 * (vec3<f32>(n, n2, n) - 0.5));
    let sun = normalize(cloud.sun_dir.xyz);
    let wrap = cloud.sun_dir.w;
    let diffuse = clamp((dot(nrm, sun) + wrap) / (1.0 + wrap), 0.0, 1.0);
    // Height through the puff's system: storm bases go slate grey, tops stay white.
    let h = clamp((in.world.y - in.layer.x) / max(in.layer.y - in.layer.x, 1.0), 0.0, 1.0);
    let base_shade = mix(cloud.albedo.z, cloud.albedo.w, dark);
    // Self-shadow: the puff's own underside (its lower half on screen, nudged by the billows) and
    // the last few hundred metres above its system's base.
    let under = smoothstep(-0.95, 0.55, p.y + (n - 0.5) * 0.4);
    let above_base = smoothstep(0.0, 350.0, in.world.y - in.layer.x);
    let shadow = mix(0.35, 1.0, under) * mix(0.55, 1.0, above_base);
    let body = mix(1.0 - base_shade, 1.0, smoothstep(0.0, 0.7, h)) * shadow;
    let silver = pow(max(dot(look, sun), 0.0), 6.0) * (1.0 - z) * (1.0 - z) * cloud.sun.w;
    let scattered = cloud.ambient.w * body;
    let albedo = mix(cloud.albedo.x, cloud.albedo.y, dark);
    let lit = cloud.sun.rgb * (albedo / 3.14159265) * (diffuse * body + silver + scattered)
        + cloud.ambient.rgb * albedo * mix(0.45, 1.0, h) * mix(1.0, 0.6, dark) * mix(0.5, 1.0, shadow);
    let near = smoothstep(cloud.fade.x * 0.3, cloud.fade.x, dist);
    // The flat base: seen from the side, nothing below the system's base. Seen from under the base
    // looking up, the base is the flat underside the eye sees, so nothing is cut: cutting the flat
    // billboard there sliced storm puffs into straight-edged blocks and hid the lowest ones
    // (owner, 2026-10-25: "why clouds are all rough like this?").
    let eye_y = view.world_position.y;
    let from_below = smoothstep(in.layer.x - 30.0, in.layer.x - 250.0, eye_y) * smoothstep(0.15, 0.5, look.y);
    let floor = mix(smoothstep(in.layer.x - 20.0, in.layer.x + 50.0, in.world.y), 1.0, from_below);
    return vec4<f32>(lit, dens * in.look.z * near * floor);
}

fn shaft(in: CloudFragment) -> vec4<f32> {
    let x = in.uv.x;
    let y = in.uv.y * 0.5 + 0.5;
    let t = globals.time;
    let seed = in.shape.z;
    let streak = 0.6 * vn(vec2<f32>(x * 22.0 + seed, in.world.y / 90.0 + t * 2.2))
        + 0.4 * vn(vec2<f32>(x * 60.0, in.world.y / 40.0 + t * 3.1));
    let body = pow(max(0.0, 1.0 - x * x), 1.5) * (0.55 + 0.45 * vn(vec2<f32>(x * 3.0 + t * 0.02, seed)));
    let a = 0.75 * body * (0.5 + 0.5 * streak) * smoothstep(0.0, 0.12, y) * smoothstep(1.0, 0.8, y);
    let col = cloud.ambient.rgb * vec3<f32>(0.5, 0.53, 0.58) * 0.8 + cloud.sun.rgb * (0.03 / 3.14159265);
    return vec4<f32>(col, a);
}

fn cirrus(in: CloudFragment, look: vec3<f32>) -> vec4<f32> {
    let q = in.uv;
    let u = q.x * 0.5 + 0.5;
    let v = q.y * 0.5 + 0.5;
    let seed = in.shape.z;
    // Seen edge-on from far away a flat streak is a scratch: fade it out at grazing angles.
    let graze = smoothstep(0.15, 0.45, abs(look.y));
    let shape = smoothstep(1.0, 0.2, abs(q.y) + 0.35 * vn(vec2<f32>(u * 6.0 + seed, 2.0))) * smoothstep(1.0, 0.4, abs(q.x));
    let streak = 0.6 * vn(vec2<f32>(u * 5.0 + seed, v * 4.0)) + 0.4 * vn(vec2<f32>(u * 14.0, v * 9.0 + seed));
    let a = 0.16 * shape * smoothstep(0.35, 0.85, streak) * graze;
    let col = cloud.sun.rgb * (0.8 / 3.14159265) + cloud.ambient.rgb * 0.9;
    return vec4<f32>(col, a);
}

@fragment
fn fragment(in: CloudFragment) -> @location(0) vec4<f32> {
    let eye = view.world_position;
    let to_frag = in.world - eye;
    let dist = max(length(to_frag), 1e-3);
    let look = to_frag / dist;
    let kind = in.look.x;
    var out: vec4<f32>;
    if kind > KIND_CIRRUS - 0.5 {
        out = cirrus(in, look);
    } else if kind > KIND_SHAFT - 0.5 {
        out = shaft(in);
    } else {
        if length(in.uv) > 1.0 {
            discard;
        }
        out = puff(in, look, dist);
    }
    out.a *= soft_edge(in.position);
    if out.a < 0.004 {
        discard;
    }
    @if(DISTANCE_FOG) {
        out = apply_fog(fog, out, in.world, eye, in.position.xy);
    }
    return out;
}
