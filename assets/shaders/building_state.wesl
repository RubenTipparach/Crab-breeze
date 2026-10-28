// The main-pass vertex shader of building meshes (openspec change `building-gpu-state`): Bevy 0.20's
// `bevy_pbr::render::mesh` vertex function (bevy_pbr 0.20.0, `src/render/mesh.wesl`) cut to what
// building meshes use (no skinning, no morph targets), plus the element state: a vertex whose
// element is down is moved outside the view, and the colour's alpha (the slot) goes back to 1.
// The fragment is Bevy's PBR, unchanged.

import bevy_pbr::render::forward_io::{Vertex, VertexOutput, decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import super::building_slot::{fallen, GONE};

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

@if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex_in.instance_index);

@if(VERTEX_COLORS) {
    if fallen(vertex.color) {
        out.position = GONE;
    }
    // The alpha (the slot) goes back to 1, or to 2 for a vertex of a damaged building's frame (its
    // slots are its own, intact buildings carry 0): the glass shader turns a damaged building's
    // windows out by it (day-night-cycle) and puts it back to 1.
    out.color = vec4<f32>(vertex.color.rgb, select(1.0, 2.0, round(vertex.color.a) > 0.5));
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);

    return out;
}
