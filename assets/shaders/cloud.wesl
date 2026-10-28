// The sky's clouds (openspec change `cloud-weather`): every puff of the weather is one billboard
// facing the eye's position and kept upright (`facing`), its edge eaten by noise and lit as a
// sphere by the scene's sun and sky (the light the ground and the water get, through `water_light`
// in Rust); rain shafts are billboards turned about the vertical with streaks falling; cirrus are
// flat streaks high up. All three are items of one mesh, sorted back to front on the CPU, drawn in
// one alpha-blended draw. Each billboard fades where it meets the ground behind it (the depth
// prepass, "soft particles") and near the eye, where the cloud fog takes over; its colour goes toward
// the fog's by the air between it and the eye (`in_the_air`), so a far one sinks into the horizon's haze.
// Linear HDR colour, reverse-Z depth.
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
import super::water_common::{gnoise3, layer_depth};
import super::fog_colour::{FogColour, fog_colour};

// Matches `CloudUniform` in `crates/cb-app/src/clouds_render.rs`.
struct CloudLight {
    sun_dir: vec4<f32>,  // xyz toward the sun; w wrap 0..1
    sun: vec4<f32>,      // rgb the sun's light times exposure; w silver lining strength
    ambient: vec4<f32>,  // rgb the ambient light times exposure; w light scattered inside, a share of the sun
    albedo: vec4<f32>,   // x white albedo, y storm albedo, z white base shade, w storm base shade
    fade: vec4<f32>,     // x fade-in distance from the eye m, y edge erosion, z soft depth m, w the shadow floor (sky fill on the shade)
    fog: vec4<f32>,      // the ground fog: x density at sea level per m (its strength now in it), y scale height m, z sea level m, w unused
    haze: vec4<f32>,     // the aerial haze: x density at sea level per m, y scale height m; zw unused
    // How many times the mist thickens the ground fog's layer toward each of 16 level bearings round the eye (bearing k
    // at k / 16 of a turn from +x toward +z), four to a vec4: `cb_core::world::mist::thickening`, the fog pass's taps.
    thick: array<vec4<f32>, 4>,
    // The fog's colour inputs, as the fog pass has them (`fog_colour.wesl`).
    sky_x: vec4<f32>,
    sky_y: vec4<f32>,
    sky_z: vec4<f32>,
    sky_lut: vec4<f32>,
    fog_ambient: vec4<f32>,
    fog_sun: vec4<f32>,
    fog_sun_dir: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> cloud: CloudLight;
// Bevy's sky-view LUT for the pilot's view, mirrored into this material each frame (`clouds_render::mirror_sky_lut`).
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var sky_view_lut: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var sky_view_sampler: sampler;

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
    // The puff's centre (with the wind's drift), for its facing frame.
    @location(5) @interpolate(flat) centre: vec3<f32>,
}

// A puff's frame: it faces the eye's position and stays upright (owner, 2026-10-25: "cloud
// rotations billboard kinda suck, I think they shoudl do rolling rotations to face you"), so turning
// the head leaves the sky where it is; only moving past a puff turns it. Columns: right, up, and
// back toward the eye. Nothing in it comes from the camera's turn (owner, 2026-10-26: "when I roll
// my camera the cloud billboards roll too", "it should be fixed to not roll"): straight under or
// over a puff, where upright has no meaning, its right is east laid onto its plane.
fn facing(c: vec3<f32>, eye: vec3<f32>) -> mat3x3<f32> {
    let back = normalize(eye - c + vec3<f32>(0.0, 1e-3, 0.0));
    let flat_right = cross(vec3<f32>(0.0, 1.0, 0.0), back);
    var east = vec3<f32>(1.0, 0.0, 0.0) - back * back.x;
    // The same way round as the level right, so blending them never cancels.
    if (dot(east, flat_right) < 0.0) {
        east = -east;
    }
    let right = normalize(mix(east, flat_right, smoothstep(0.02, 0.2, length(flat_right))));
    let up = cross(back, right);
    return mat3x3<f32>(right, up, back);
}

@vertex
fn vertex(in: CloudVertex) -> CloudFragment {
    var out: CloudFragment;
    let drift = get_world_from_local(in.instance_index)[3].xyz;
    let c = in.centre + drift;
    let eye = view.world_position;
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
        let f = facing(c, eye);
        let k = abs(f[2].y);
        wp = c + f[0] * (in.corner.x * in.shape.x) + f[1] * (in.corner.y * mix(in.shape.y, in.shape.x, k * k));
    }
    out.position = position_world_to_clip(wp);
    out.world = wp;
    out.uv = in.corner;
    out.shape = in.shape;
    out.look = in.look;
    out.layer = in.layer;
    out.centre = c;
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

// Gradient noise, 0..1, on a plane `z` through the 3D lattice. Billows use it rather than value
// noise: value noise is a grid of hashed squares, smoothed only at their corners, and close up a puff
// filled with it read as blocks (owner, 2026-10-26: "close up clouds are still pixelated, the look
// like jpeg compression"). `gnoise3` has a standard deviation of about 0.28 (one octave of the 0.166
// measured for `fbm3`), so 0.65 gives the 0.18 spread the value noise had.
fn gn(p: vec2<f32>, z: f32) -> f32 {
    return clamp(0.5 + 0.65 * gnoise3(vec3<f32>(p.x, z, p.y)), 0.0, 1.0);
}

// Billows, 0..1: three octaves of gradient noise, and a fourth, finer one where the puff is close
// enough for it to cover more than a few pixels (`fine`, 0..1), so it never shimmers far off.
fn fbm2(p: vec2<f32>, z: f32, fine: f32) -> f32 {
    let base = 0.5 * gn(p, z) + 0.25 * gn(p * 2.03 + 17.0, z + 3.1) + 0.125 * gn(p * 4.07 + 31.0, z + 7.3);
    return (base + 0.0625 * fine * gn(p * 8.3 + 5.0, z + 11.9)) / (0.875 + 0.0625 * fine);
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

// `fine` (0..1): how much of the finest billow octave this pixel can show (see `fragment`).
// `sky`: the sky's colour along the view (the fog's, `fog_colour`), which lights the shaded side from below and round.
fn puff(in: CloudFragment, look: vec3<f32>, dist: f32, fine: f32, sky: vec3<f32>) -> vec4<f32> {
    let p = in.uv;
    let d = length(p);
    let seed = in.shape.z;
    let dark = in.look.y;
    // Billows: the edge eaten by two octaves of noise in the sprite's plane (caps stay smooth).
    let n = fbm2(p * 2.2, seed * 13.1 + globals.time * 0.004, fine);
    let n2 = gn(p * 7.0, seed * 5.0 + 40.0);
    let erode = select(cloud.fade.y, 0.22, abs(in.look.x - KIND_CAP) < 0.5);
    // Faded out before the billboard's circle whatever the noise says, so no puff shows the hard arc
    // where the circle is cut (seen in the storm stills from below).
    let dens = smoothstep(1.0, 0.45, d + (n - 0.5) * erode + (n2 - 0.5) * 0.2) * smoothstep(1.0, 0.8, d);
    let f = facing(in.centre, view.world_position);
    let right = f[0];
    let up = f[1];
    let back = f[2];
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
    // The sky's fill on the shade (`look_shadow_floor`): the sky and the haze round a cloud light its underside and its
    // shaded flank, so a puff's shade stays a soft grey and a storm's base, lower in albedo, darker but never black
    // (owner, 2026-10-27: "Can we tint the dark parts of clouds softer?"). Only where the sun does not reach.
    let shade = 1.0 - clamp(diffuse * body, 0.0, 1.0);
    let lit = cloud.sun.rgb * (albedo / 3.14159265) * (diffuse * body + silver + scattered)
        + cloud.ambient.rgb * albedo * mix(0.45, 1.0, h) * mix(1.0, 0.6, dark) * mix(0.5, 1.0, shadow)
        + sky * albedo * cloud.fade.w * shade;
    let near = smoothstep(cloud.fade.x * 0.3, cloud.fade.x, dist);
    // The flat base: nothing below the system's base, cut on the height of the puff's own round
    // surface at this pixel (as a sphere's would be) over a band a third of its height deep, never
    // on the flat billboard. A flat height cut each billboard along a straight line at its own angle,
    // and a storm's base, full of low puffs, broke into straight-edged shards (owner, 2026-10-25:
    // "why clouds are all rough like this?"; 2026-10-26: "its also really blocky up close"). Seen
    // from under the base looking up, the base is the flat underside the eye sees: nothing is cut.
    let surface_y = in.centre.y + in.shape.x * right.y * p.x + in.shape.y * (up.y * p.y + back.y * z);
    let band = max(in.shape.y * 0.35, 20.0);
    let eye_y = view.world_position.y;
    let from_below = smoothstep(in.layer.x - 30.0, in.layer.x - 250.0, eye_y) * smoothstep(0.15, 0.5, look.y);
    let floor = mix(smoothstep(in.layer.x - band, in.layer.x + band * 0.5, surface_y), 1.0, from_below);
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
    // From inside the shaft or looking steeply up, a sheet is seen nearly edge-on: a flat grey wedge
    // with straight edges that swings round as the eye moves under it (owner, 2026-10-26: "its also
    // really blocky up close"). There the rain's haze and the drops on the glass carry the rain, so
    // the sheet fades out within the shaft's radius of its axis and as the view climbs past 35 degrees.
    let eye = view.world_position;
    let off_axis = length(eye.xz - in.centre.xz);
    let inside = smoothstep(0.6 * in.shape.x, 1.6 * in.shape.x, off_axis);
    let steep = smoothstep(0.75, 0.55, abs(normalize(in.world - eye).y));
    let a = 0.75 * body * (0.5 + 0.5 * streak) * smoothstep(0.0, 0.12, y) * smoothstep(1.0, 0.8, y) * inside * steep;
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

// The mist's thickening of the ground fog's layer toward `look`'s bearing, between the two nearest of the 16 (`thick`).
fn thickening(look: vec3<f32>) -> f32 {
    let k = fract(atan2(look.z, look.x) * 0.15915494 + 1.0) * 16.0;
    let i = u32(floor(k)) % 16u;
    let j = (i + 1u) % 16u;
    let a = cloud.thick[i / 4u][i % 4u];
    let b = cloud.thick[j / 4u][j % 4u];
    return mix(a, b, fract(k));
}

// The share of a billboard's light that crosses the air from it to the eye: the ground fog's layer, thickened by the mist
// round the eye toward it as the fog pass thickens the land under it (`fog_depth` in `water_post.wesl`), and the aerial
// haze, the same optical depth the fog pass gives the ground and the sky behind it. The rain's haze is left out: it
// reaches only 1.5 km, so all of it lies under the storm's own base, and tinting by it washed the base, its shape and
// its dark flat underside, into one flat grey (stills of 2026-10-27, `docs/screenshots/2026-10-27-cloud-fog-blend-*`).
// Fading by it instead opened the base onto the blue sky behind it. Four exponentials, no march (CLAUDE.md section 7).
fn air_between(eye: vec3<f32>, look: vec3<f32>, dist: f32) -> f32 {
    let layer = layer_depth(cloud.fog.x, cloud.fog.y, cloud.fog.z, eye.y, look.y, dist) * thickening(look);
    let haze = layer_depth(cloud.haze.x, cloud.haze.y, cloud.fog.z, eye.y, look.y, dist);
    return exp(-(max(layer, 0.0) + max(haze, 0.0)));
}

// Aerial perspective on a cloud (owner, 2026-10-27: "cloud fog blend should be a thing"): its colour mixed toward the
// fog's colour along the view by the air it crosses, as the fog pass mixes the land behind it, so a far cloud sinks
// into the horizon's haze in the horizon's colour and a near one, or one seen from above the layer, stays crisp. Its
// alpha is kept: the in-scattered light is in front of the cloud whatever lies behind it, and a faded billboard showed
// the sky through a far cloud rather than the haze over it.
fn in_the_air(c: vec3<f32>, fog_c: vec3<f32>, eye: vec3<f32>, look: vec3<f32>, dist: f32) -> vec3<f32> {
    return mix(fog_c, c, air_between(eye, look, dist));
}

// The fog's colour along `look` (`fog_colour.wesl`, as the fog pass has it): the cloud's one texture read.
fn fog_along(look: vec3<f32>) -> vec3<f32> {
    let f = FogColour(cloud.sky_x, cloud.sky_y, cloud.sky_z, cloud.sky_lut, cloud.fog_ambient, cloud.fog_sun, cloud.fog_sun_dir);
    return fog_colour(sky_view_lut, sky_view_sampler, f, look);
}

@fragment
fn fragment(in: CloudFragment) -> @location(0) vec4<f32> {
    let eye = view.world_position;
    let to_frag = in.world - eye;
    let dist = max(length(to_frag), 1e-3);
    let look = to_frag / dist;
    let kind = in.look.x;
    // The finest billow octave only where a puff's unit spans 600 pixels or more (its features 70 px
    // across); taken here, before any branch, where screen derivatives are defined.
    let fine = smoothstep(1.0 / 300.0, 1.0 / 600.0, length(fwidth(in.uv)));
    let fog_c = fog_along(look);
    var out: vec4<f32>;
    if kind > KIND_CIRRUS - 0.5 {
        out = cirrus(in, look);
    } else if kind > KIND_SHAFT - 0.5 {
        out = shaft(in);
    } else {
        if length(in.uv) > 1.0 {
            discard;
        }
        out = puff(in, look, dist, fine, fog_c);
    }
    out.a *= soft_edge(in.position);
    if out.a < 0.004 {
        discard;
    }
    out = vec4<f32>(in_the_air(out.rgb, fog_c, eye, look, dist), out.a);
    @if(DISTANCE_FOG) {
        out = apply_fog(fog, out, in.world, eye, in.position.xy);
    }
    return out;
}
