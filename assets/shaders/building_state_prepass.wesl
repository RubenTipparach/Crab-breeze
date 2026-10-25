// The prepass vertex shader of building meshes (openspec change `building-gpu-state`), used for
// the depth and normal prepass and the shadow maps: Bevy 0.20's `bevy_pbr::prepass` vertex function
// (bevy_pbr 0.20.0, `src/prepass/prepass.wesl`) cut to what building meshes use (no skinning, no
// morph targets; buildings do not move, so last frame's position is this frame's), plus the element
// state, as `building_state`.

import bevy_pbr::prepass::io::{Vertex, VertexOutput, decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import super::building_slot::{fallen, GONE};

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);

@if(VERTEX_COLORS) {
    if fallen(vertex.color) {
        out.position = GONE;
    }
    out.color = vec4<f32>(vertex.color.rgb, 1.0);
}

@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
@if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
@if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex_in.instance_index);
}

@if(MOTION_VECTOR_PREPASS)
    out.previous_world_position = out.world_position;

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);

    return out;
}
