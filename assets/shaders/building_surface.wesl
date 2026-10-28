// The fragment shader of building meshes drawn with `BuildingMaterial` (openspec changes
// `building-gpu-state`, `ambient-occlusion`): Bevy's own PBR path (bevy_pbr 0.20.0,
// `src/render/pbr.wesl`, cut to what buildings use), except that glass takes no screen-space
// ambient occlusion. Glass is transparent, so it is not in the depth and normal prepass SSAO reads:
// at a window the occlusion of the floors and rooms behind it darkened its sky reflection away, and
// the windows read as see-through with the floor edges showing white (2026-10-24 stills).

import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing};
import bevy_pbr::render::mesh_view_bindings::{view, globals};

// 1 for glass: no screen-space ambient occlusion.
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var<uniform> no_ssao: u32;
// The windows at night (`cb_core::day_night::WindowLights`): x the share lit now, y a lit window's
// brightness cd/m2, z a storey's height m, w a window cell's width m.
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var<uniform> windows: vec4<f32>;

fn window_hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.547);
}

// A lit window's light at this fragment of glass, cd/m2, or none: each window is a cell of its facade
// one storey high and `windows.w` wide, lit when its own hash falls under the share lit now (owner,
// 2026-10-28: "buildings also need some sort of cell lighting. If the building is damaged all of its
// lights turn off"). Mostly warm rooms, some office tubes, a few televisions.
fn window_light(world: vec3<f32>, normal: vec3<f32>, seconds: f32) -> vec3<f32> {
    let along_z = abs(normal.x) > abs(normal.z);
    let t = select(world.x, world.z, along_z);
    let side = select(sign(normal.z), sign(normal.x), along_z);
    let cell = vec3<f32>(floor(t / max(windows.w, 0.1)), floor(world.y / max(windows.z, 0.5)), side + select(0.0, 7.0, along_z));
    let h = window_hash(cell);
    if (h >= windows.x) {
        return vec3<f32>(0.0);
    }
    let kind = window_hash(cell + vec3<f32>(17.0, 3.0, 5.0));
    var colour = vec3<f32>(1.0, 0.72, 0.42);
    if (kind > 0.95) {
        colour = vec3<f32>(0.45, 0.6, 1.0) * (0.7 + 0.3 * sin(seconds * 9.0 + h * 40.0));
    } else if (kind > 0.7) {
        colour = vec3<f32>(0.85, 0.95, 1.0);
    }
    return colour * windows.y * (0.6 + 0.4 * window_hash(cell + vec3<f32>(5.0, 11.0, 2.0)));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var v = in;
    var damaged = false;
    @if(VERTEX_COLORS) {
        damaged = in.color.a > 1.5;
        v.color.a = 1.0;
    }
    var pbr = pbr_input_from_standard_material(v, is_front);
    if no_ssao != 0u {
        pbr.diffuse_occlusion = vec3<f32>(1.0);
        pbr.specular_occlusion = 1.0;
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    if (no_ssao != 0u && !damaged && windows.x > 0.0) {
        out.color = vec4<f32>(out.color.rgb + window_light(in.world_position.xyz, normalize(in.world_normal), globals.time) * view.exposure, max(out.color.a, 0.85));
    }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
