// The prepass vertex shader of people (openspec change `people-vat`), for the depth and normal prepass and
// the shadow maps: Bevy 0.20's prepass vertex function replaced by the vertex animation texture's pose
// (`people_vat_common`). Motion vectors take this frame's position as last frame's.

import bevy_pbr::prepass::io::{Vertex, VertexOutput};
import bevy_pbr::render::view_transformations::position_world_to_clip;
import super::people_vat_common::{pose, GONE};

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
@if(VERTEX_UVS_A && VERTEX_COLORS) {
    let posed = pose(vertex_in.instance_index, vertex_in.uv, vertex_in.color);
    out.world_position = vec4<f32>(posed.world_position, 1.0);
    out.position = select(GONE, position_world_to_clip(posed.world_position), posed.shown);
    out.color = posed.color;
@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS)
    out.world_normal = posed.world_normal;
    out.uv = vertex_in.uv;
}
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}
@if(MOTION_VECTOR_PREPASS)
    out.previous_world_position = out.world_position;
@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    return out;
}
