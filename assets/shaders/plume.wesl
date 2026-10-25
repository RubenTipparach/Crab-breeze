// Exhaust plume (openspec change `rocket-plumes`; owner, 2026-10-25: "rocket plumes arent convincing right now"): drawn
// additively on the plume mesh (`plumes.rs`), which runs from the nozzle (uv.y 0) to the far end (uv.y 1). White hot at
// the nozzle through yellow to orange, broken by noise scrolling down the flame, brightest where the surface faces the
// eye so the edges are soft, with shock diamonds along a jet's first stretch. Unlit: the flame is its own light.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::{view, globals};

// Matches `PlumeUniform` in `crates/cb-app/src/plumes.rs`.
struct PlumeLook {
    core: vec4<f32>,    // rgb at the nozzle, w overall brightness
    mid: vec4<f32>,     // rgb a third of the way, w scroll speed in lengths a second
    edge: vec4<f32>,    // rgb at the far end, w noise scale across
    shock: vec4<f32>,   // x diamonds along the length, y their strength, z how far along they run (share), w unused
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> look: PlumeLook;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let v = in.uv.y;
    let u = in.uv.x;
    let t = globals.time * look.mid.w;
    // Two octaves of noise running down the flame, wrapping around it.
    let p = vec2<f32>(u * look.edge.w, v * 6.0 - t * 6.0);
    let n = 0.65 * noise(p) + 0.35 * noise(p * 2.3 + vec2<f32>(17.0, 3.0));
    var colour = mix(look.core.rgb, look.mid.rgb, smoothstep(0.0, 0.33, v));
    colour = mix(colour, look.edge.rgb, smoothstep(0.33, 1.0, v));
    // Soft at the silhouette: brightest where the surface looks at the eye.
    let to_eye = normalize(view.world_position.xyz - in.world_position.xyz);
    let facing = pow(abs(dot(normalize(in.world_normal), to_eye)), 1.4);
    let shock = 1.0 + look.shock.y * max(0.0, cos(6.2831853 * v * look.shock.x)) * (1.0 - smoothstep(0.0, look.shock.z, v));
    let fade = pow(1.0 - v, 1.3) * smoothstep(0.0, 0.04, v);
    let k = look.core.w * fade * facing * shock * mix(0.45, 1.25, n);
    return vec4<f32>(colour * k, 1.0);
}
