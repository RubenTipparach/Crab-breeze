// A street lamp's glow seen from afar (openspec change `day-night-cycle`, design section 5, tier 3):
// each lamp a quad around its head, sized on screen to cover at least `glow.y` pixels, drawn adding
// its light, so a city at night reads as a grid of lights from the air where the lit heads are under
// a pixel. It fades in from `glow.z` metres (nearer, the head and its real light carry the lamp) and
// with the lamps' switch-on share (`glow.x`). Matches `LampGlow` in `crates/cb-app/src/lamps.rs`.

import bevy_pbr::render::mesh_view_bindings::view;

struct LampGlow {
    colour: vec4<f32>, // rgb the glow at its heart, on screen; w unused
    glow: vec4<f32>,   // x the lamps' switch-on share; y the least size, pixels; z fully in from this far, m; w its size in the world, m
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> lamp: LampGlow;

struct GlowVertex {
    @location(0) head: vec3<f32>,
    @location(1) corner: vec2<f32>,
}

struct GlowFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) corner: vec2<f32>,
    @location(1) @interpolate(flat) strength: f32,
}

@vertex
fn vertex(in: GlowVertex) -> GlowFragment {
    var out: GlowFragment;
    let clip = view.clip_from_world * vec4<f32>(in.head, 1.0);
    let distance = length(in.head - view.world_position);
    // Its size on screen: its world size or the least size, whichever is larger, in pixels.
    let size = view.viewport.zw;
    let world_px = lamp.glow.w / max(clip.w, 1e-3) * view.clip_from_view[1][1] * size.y * 0.5;
    let px = max(world_px, lamp.glow.y);
    out.position = clip + vec4<f32>(in.corner * px / size * 2.0 * clip.w, 0.0, 0.0);
    out.corner = in.corner;
    out.strength = lamp.glow.x * smoothstep(lamp.glow.z * 0.5, lamp.glow.z, distance);
    return out;
}

@fragment
fn fragment(in: GlowFragment) -> @location(0) vec4<f32> {
    let d2 = dot(in.corner, in.corner);
    if (d2 > 1.0 || in.strength <= 0.0) {
        discard;
    }
    let g = exp(-d2 * 4.0) * in.strength;
    return vec4<f32>(lamp.colour.rgb * g, 0.0);
}
