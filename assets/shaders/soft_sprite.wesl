// Soft smoke sprites (openspec change `particle-depth`): the blended smoke, trail, dust, steam and mist
// particles as camera-facing quads of one mesh, sorted back to front on the CPU each frame
// (`crates/cb-app/src/soft_sprites.rs`), each a cell of its kind's Material Maker sheet times its colour
// over life (as bevy_hanabi drew them: `ImageSampleMapping::Modulate`). Each fades where it meets the
// world behind it (the depth prepass, "soft particles", as the clouds do in `cloud.wesl`) and near the
// eye; Bevy's fog hazes far ones; the result is written premultiplied (`AlphaMode::Premultiplied`).
// Unlit: the sheets carry their own light from above. Linear HDR colour, reverse-Z depth.

import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::view;
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;
@if(DISTANCE_FOG)
import bevy_pbr::render::{mesh_view_bindings::fog, pbr_functions::apply_fog};

// Matches `SoftLook` in `soft_sprites.rs`.
struct SoftLook {
    fade: vec4<f32>, // x soft depth against the world m, y fade-in distance from the eye m
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> look: SoftLook;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var smoke_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var smoke_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var dust_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var dust_sampler: sampler;

// Cells across a sheet (`particles::GRID`).
const GRID: f32 = 4.0;

struct SoftVertex {
    // The sprite's centre.
    @location(0) centre: vec3<f32>,
    // The quad's corner, -1..1.
    @location(1) corner: vec2<f32>,
    // x full size m, y spin radians, z sheet cell, w sheet (0 smoke, 1 dust).
    @location(2) shape: vec4<f32>,
    // Colour over life, linear, alpha in w.
    @location(3) colour: vec4<f32>,
}

struct SoftFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) sheet: f32,
    @location(3) colour: vec4<f32>,
}

@vertex
fn vertex(in: SoftVertex) -> SoftFragment {
    var out: SoftFragment;
    let right = view.world_from_view[0].xyz;
    let up = view.world_from_view[1].xyz;
    let s = sin(in.shape.y);
    let c = cos(in.shape.y);
    let k = vec2<f32>(in.corner.x * c - in.corner.y * s, in.corner.x * s + in.corner.y * c);
    let wp = in.centre + (right * k.x + up * k.y) * (0.5 * in.shape.x);
    out.position = position_world_to_clip(wp);
    out.world = wp;
    let cell = in.shape.z;
    let col = cell - floor(cell / GRID) * GRID;
    let row = floor(cell / GRID);
    let local = vec2<f32>(in.corner.x * 0.5 + 0.5, 0.5 - in.corner.y * 0.5);
    out.uv = (vec2<f32>(col, row) + local) / GRID;
    out.sheet = in.shape.w;
    out.colour = in.colour;
    return out;
}

// How much of the sprite shows in front of the world behind it: 1 in open air, fading to 0 over
// `look.fade.x` metres in front of a surface (reverse-Z, infinite far plane: depth = near / z).
fn soft_edge(pixel: vec4<f32>) -> f32 {
    var soft = 1.0;
    @if(DEPTH_PREPASS) {
        let scene = prepass_depth(pixel, 0u);
        if scene > 0.0 {
            let near = view.clip_from_view[3][2];
            let gap = near / scene - near / max(pixel.z, 1e-9);
            soft = smoothstep(0.0, look.fade.x, gap);
        }
    }
    return soft;
}

@fragment
fn fragment(in: SoftFragment) -> @location(0) vec4<f32> {
    let a = textureSample(smoke_tex, smoke_sampler, in.uv);
    let b = textureSample(dust_tex, dust_sampler, in.uv);
    let tex = select(a, b, in.sheet > 0.5);
    var out = tex * in.colour;
    let eye = view.world_position;
    let near = smoothstep(0.0, look.fade.y, distance(in.world, eye));
    out.a *= soft_edge(in.position) * near;
    if out.a < 0.003 {
        discard;
    }
    @if(DISTANCE_FOG) {
        out = apply_fog(fog, out, in.world, eye, in.position.xy);
    }
    // Premultiplied: the colour carries its own coverage, so a sprite's soft rim blends into what is
    // behind it with no dark or bright fringe, and filtered texels at the edge do not bleed colour.
    return vec4<f32>(out.rgb * out.a, out.a);
}
