// The fragment shader of building meshes drawn with `BuildingMaterial` (openspec changes
// `building-gpu-state`, `ambient-occlusion`): Bevy's own PBR path (bevy_pbr 0.20.0,
// `src/render/pbr.wesl`, cut to what buildings use), except that glass takes no screen-space
// ambient occlusion. Glass is transparent, so it is not in the depth and normal prepass SSAO reads:
// at a window the occlusion of the floors and rooms behind it darkened its sky reflection away, and
// the windows read as see-through with the floor edges showing white (2026-10-24 stills).

import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing};

// 1 for glass: no screen-space ambient occlusion.
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var<uniform> no_ssao: u32;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);
    if no_ssao != 0u {
        pbr.diffuse_occlusion = vec3<f32>(1.0);
        pbr.specular_occlusion = 1.0;
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
