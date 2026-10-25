// Smoke puffs (openspec change `smoke-puffs`): one alpha-blended layer per lumpy blob, lit by the
// scene's sun and ambient light (the same light the ground and the water get, through `water_light`
// in Rust), fogged by Bevy's own fog. Linear HDR colour. Each puff's own colour is its albedo
// (`SMOKE`), its height share is its normal's (darker underneath), and its own opacity scales the
// alpha.
//
// Provenance: the cloud-blob shader of `cloud-puffs` (`assets/shaders/cloud.wesl` at main 7808fa6),
// which drew the clouds until they became billboards (`cloud-weather`); smoke kept the blob look.
//
// Lighting, per pixel:
// - the sun wrapped round the blob;
// - darker toward its underside, lighter on top;
// - a silver lining where the eye looks toward the sun through a thin edge (forward scattering);
// - the ambient sky from above;
// - the silhouette softened by the view angle (no hard polygon edge), and the blob faded out near
//   the eye.

import bevy_pbr::render::mesh_view_bindings::view;
@if(DISTANCE_FOG)
import bevy_pbr::render::{mesh_view_bindings::fog, pbr_functions::apply_fog};

// Matches `SmokeLight` in `crates/cb-app/src/smoke_render.rs`.
struct CloudLight {
    sun_dir: vec4<f32>,  // xyz toward the sun; w wrap 0..1
    sun: vec4<f32>,      // rgb the sun's light times exposure; w silver lining strength
    ambient: vec4<f32>,  // rgb the ambient light times exposure; w albedo
    layer: vec4<f32>,    // x cloud base m, y cloud top m, z base darkening 0..1
    fade: vec4<f32>,     // x fade-in distance m, y edge softness, z clear-air visibility m
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> cloud: CloudLight;
// A smoke puff's colour (rgb, linear) and opacity (a); `SmokeMaterial::tint` in Rust.
@if(SMOKE)
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> tint: vec4<f32>;

// The mesh vertex shader's output, the fields used (bevy_pbr `forward_io::VertexOutput` locations).
struct CloudFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
}

@fragment
fn fragment(in: CloudFragment) -> @location(0) vec4<f32> {
    let p = in.world_position.xyz;
    let eye = view.world_position;
    let to_frag = p - eye;
    let dist = max(length(to_frag), 1e-3);
    let look = to_frag / dist;
    let n = normalize(in.world_normal);
    let sun = normalize(cloud.sun_dir.xyz);
    let wrap = cloud.sun_dir.w;

    // The sun wrapped round the blob, the base darker than the top.
    let diffuse = clamp((dot(n, sun) + wrap) / (1.0 + wrap), 0.0, 1.0);
    var h = clamp((p.y - cloud.layer.x) / max(cloud.layer.y - cloud.layer.x, 1.0), 0.0, 1.0);
    var albedo = vec3<f32>(cloud.ambient.w);
    var opacity = 1.0;
    @if(SMOKE) {
        h = n.y * 0.5 + 0.5;
        albedo = tint.rgb;
        opacity = tint.a;
    }
    let body = mix(1.0 - cloud.layer.z, 1.0, smoothstep(0.0, 0.8, h));
    // Edges facing across the view are thin: the silver lining toward the sun lights them.
    let facing = abs(dot(n, look));
    let edge = 1.0 - facing;
    let toward_sun = pow(max(dot(look, sun), 0.0), 6.0);
    let silver = toward_sun * edge * edge * cloud.sun.w;

    let lit = cloud.sun.rgb * (albedo / 3.14159265) * (diffuse * body + silver)
        + cloud.ambient.rgb * albedo * mix(0.55, 1.0, h);

    // Soft silhouette, and a fade near the eye (inside a cloud the fog does the work).
    let soft = smoothstep(0.0, max(cloud.fade.y, 1e-3), facing);
    let near = smoothstep(cloud.fade.x * 0.4, cloud.fade.x, dist);
    var out = vec4<f32>(lit, soft * near * opacity);
    @if(DISTANCE_FOG) {
        out = apply_fog(fog, out, p, eye, in.position.xy);
    }
    return out;
}
