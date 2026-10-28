// The main-pass vertex shader of people (openspec change `people-vat`): Bevy 0.20's mesh vertex function
// (`bevy_pbr::render::mesh`) replaced by the vertex animation texture's pose (`people_vat_common`). The
// fragment is Bevy's PBR, unchanged; the vertex colour carries the person's palette colour times the shade.

import bevy_pbr::render::forward_io::{Vertex, VertexOutput};
import bevy_pbr::render::view_transformations::position_world_to_clip;
import super::people_vat_common::{pose, GONE};

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
@if(VERTEX_UVS_A && VERTEX_COLORS) {
    let posed = pose(vertex_in.instance_index, vertex_in.uv, vertex_in.color);
    out.world_position = vec4<f32>(posed.world_position, 1.0);
    out.world_normal = posed.world_normal;
    out.position = select(GONE, position_world_to_clip(posed.world_position), posed.shown);
    out.color = posed.color;
    out.uv = vertex_in.uv;
}
@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    return out;
}
