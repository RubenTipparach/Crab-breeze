// Mining Mike's mesh nuke, as a reference for the particle one (`nuke_mesh.rs`, crab-airship section 11): his
// `nuke_cloud.gdshader` (godot-sandbox `godot-sandbox/resources/shaders/nuke_cloud.gdshader` at a5c3ec5a) ported to
// WESL, one material per part: `heat` 0 -> 1 cools the surface from white-hot through yellow, orange and red into lit
// smoke, crevices and undersides last; `dissolve` 0 -> 1 eats it away in noisy holes with an ember rim; time rolls the
// billows upward. The smoke is lit by the sun and the sky as the ground is; the fire as bright as a lit white surface
// times his push. Opaque with discard.

import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::globals;

// Matches `NukeMeshLook` in `nuke_mesh.rs`.
struct NukeMeshLook {
    drive: vec4<f32>, // x heat, y dissolve, z heat offset, w dissolve offset
    shape: vec4<f32>, // x noise cells per mesh unit, y billow (mesh units), z roll cells a second, w fire brightness
    light: vec4<f32>, // xyz toward the sun, w heat variance
    sun: vec4<f32>,   // rgb the sun's light over pi, times exposure
    amb: vec4<f32>,   // rgb the sky's light, times exposure
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> look: NukeMeshLook;

const COL_WHITE: vec3<f32> = vec3<f32>(1.0, 0.97, 0.86);
const COL_YELLOW: vec3<f32> = vec3<f32>(1.0, 0.80, 0.30);
const COL_ORANGE: vec3<f32> = vec3<f32>(1.0, 0.42, 0.08);
const COL_RED: vec3<f32> = vec3<f32>(0.62, 0.11, 0.05);
const COL_SMOKE_DARK: vec3<f32> = vec3<f32>(0.20, 0.17, 0.16);
const COL_SMOKE_LIT: vec3<f32> = vec3<f32>(0.62, 0.55, 0.50);
const COL_EMBER: vec3<f32> = vec3<f32>(1.0, 0.45, 0.12);
const UNDER_GLOW: f32 = 0.3;
const EMBER_WIDTH: f32 = 0.05;

struct NukeVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct NukeFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) obj: vec3<f32>,
    @location(1) wn: vec3<f32>,
    @location(2) noise: f32,
}

fn hash31(q: vec3<f32>) -> f32 {
    var p = fract(q * 0.3183099 + 0.1);
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

fn vnoise(x: vec3<f32>) -> f32 {
    let i = floor(x);
    var f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(mix(hash31(i + vec3<f32>(0.0, 0.0, 0.0)), hash31(i + vec3<f32>(1.0, 0.0, 0.0)), f.x),
            mix(hash31(i + vec3<f32>(0.0, 1.0, 0.0)), hash31(i + vec3<f32>(1.0, 1.0, 0.0)), f.x), f.y),
        mix(mix(hash31(i + vec3<f32>(0.0, 0.0, 1.0)), hash31(i + vec3<f32>(1.0, 0.0, 1.0)), f.x),
            mix(hash31(i + vec3<f32>(0.0, 1.0, 1.0)), hash31(i + vec3<f32>(1.0, 1.0, 1.0)), f.x), f.y),
        f.z);
}

fn fbm(q: vec3<f32>) -> f32 {
    var p = q;
    var sum = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 3; i++) {
        sum += amp * vnoise(p);
        p = p * 2.03 + vec3<f32>(1.7, 9.2, 3.1);
        amp *= 0.5;
    }
    return sum / 0.875;
}

@vertex
fn vertex(in: NukeVertex) -> NukeFragment {
    var out: NukeFragment;
    let world_from_local = get_world_from_local(in.instance_index);
    let n = fbm(in.position * look.shape.x + vec3<f32>(0.0, -globals.time * look.shape.z, 0.0));
    let local = in.position + in.normal * (n - 0.5) * 2.0 * look.shape.y;
    out.position = position_world_to_clip((world_from_local * vec4<f32>(local, 1.0)).xyz);
    out.obj = in.position;
    out.noise = n;
    out.wn = normalize((world_from_local * vec4<f32>(in.normal, 0.0)).xyz);
    return out;
}

@fragment
fn fragment(in: NukeFragment) -> @location(0) vec4<f32> {
    let d = clamp(look.drive.y + look.drive.w, 0.0, 1.0);
    let hole = fbm(in.obj * look.shape.x * 2.3 + vec3<f32>(5.1, 2.7, 8.3));
    let thr = select(-1.0, mix(0.18, 0.86, d), d > 0.0);
    if hole < thr {
        discard;
    }
    let under = clamp(-in.wn.y, 0.0, 1.0);
    let h = clamp(look.drive.x + look.drive.z + (in.noise - 0.5) * 2.0 * look.light.w - under * UNDER_GLOW, 0.0, 1.0);
    var fire = mix(COL_WHITE, COL_YELLOW, smoothstep(0.0, 0.15, h));
    fire = mix(fire, COL_ORANGE, smoothstep(0.15, 0.35, h));
    fire = mix(fire, COL_RED, smoothstep(0.35, 0.55, h));
    let lit = clamp(dot(in.wn, normalize(look.light.xyz)) * 0.5 + 0.5, 0.0, 1.0);
    let smoke = mix(COL_SMOKE_DARK, COL_SMOKE_LIT, lit) * (look.sun.rgb * lit + look.amb.rgb);
    let white = dot(look.sun.rgb + look.amb.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let glow = mix(look.shape.w, 1.0, smoothstep(0.3, 0.55, h));
    var col = mix(fire * glow * white, smoke, smoothstep(0.45, 0.78, h));
    if thr > 0.0 && hole < thr + EMBER_WIDTH {
        col = COL_EMBER * look.shape.w * white;
    }
    return vec4<f32>(col, 1.0);
}
