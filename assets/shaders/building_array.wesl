// The opaque building material's fragment shader (openspec change `building-texture-array`): every
// opaque building look is a layer of three texture arrays, the vertex's second UV holding its
// layer, so a building's opaque parts are one mesh and one draw. It reads the layer's colour
// (tinted by the vertex colour), normal map and occlusion-roughness-metal map into Bevy's PBR input
// (which already holds the view, fog, shadow flags and SSAO, from
// `pbr_input_from_standard_material`) and hands it to Bevy's lighting, as `StandardMaterial` would.
// The vertex shaders are `building_state` and `building_state_prepass`.

import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace, apply_normal_mapping};

@group(constants::MATERIAL_BIND_GROUP) @binding(101) var albedo_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var array_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var orm_array: texture_2d_array<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);

    var layer = 0;
@if(VERTEX_UVS_B)
    layer = i32(round(in.uv_b.x));
    var uv = vec2<f32>(0.0);
@if(VERTEX_UVS_A)
    uv = in.uv;
    var tint = vec3<f32>(1.0);
@if(VERTEX_COLORS)
    tint = in.color.rgb;

    let albedo = textureSample(albedo_array, array_sampler, uv, layer);
    pbr.material.base_color = vec4<f32>(albedo.rgb * tint, 1.0);
    let orm = textureSample(orm_array, array_sampler, uv, layer);
    pbr.material.perceptual_roughness = orm.g;
    pbr.material.metallic = orm.b;
    pbr.diffuse_occlusion *= orm.r;
@if(VERTEX_TANGENTS) {
    let nt = textureSample(normal_array, array_sampler, uv, layer).rgb;
    let tbn = calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
    pbr.N = apply_normal_mapping(pbr.material.flags, tbn, false, is_front, nt);
}

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
