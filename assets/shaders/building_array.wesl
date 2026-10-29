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
import bevy_pbr::render::mesh_view_bindings::{view, globals};
import super::building_windows::window_light;

@group(constants::MATERIAL_BIND_GROUP) @binding(101) var albedo_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var array_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var orm_array: texture_2d_array<f32>;
// The windows at night (`building_windows`), lit on the far and massing walls only (zero on the near
// walls, whose windows are glass): x the share lit now, y brightness cd/m2, z storey m, w cell m.
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var<uniform> windows: vec4<f32>;
// The lit rooms (`building_state.wesl` decides which; this adds their light): y a lit room's brightness cd/m2.
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var<uniform> rooms: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var<uniform> room_colour: vec4<f32>;

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
@if(VERTEX_UVS_B) {
    // A lit room's own light on its floor, walls, ceiling and furniture (`cb_core::night_light`), from the
    // vertex shader: nothing by day or in a dark room.
    if (in.uv_b.y > 0.0) {
        out.color = vec4<f32>(out.color.rgb + pbr.material.base_color.rgb * room_colour.rgb * in.uv_b.y * rooms.y * view.exposure, out.color.a);
    }
}
    if (windows.x > 0.0) {
        out.color = vec4<f32>(out.color.rgb + window_light(in.world_position.xyz, normalize(in.world_normal), globals.time, windows, true) * view.exposure, out.color.a);
    }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
