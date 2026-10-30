// The main-pass vertex shader of building meshes (openspec change `building-gpu-state`): Bevy 0.20's
// `bevy_pbr::render::mesh` vertex function (bevy_pbr 0.20.0, `src/render/mesh.wesl`) cut to what
// building meshes use (no skinning, no morph targets), plus the element state: a vertex whose
// element is down is moved outside the view, and the colour's alpha (the slot) goes back to 1.
// The fragment is Bevy's PBR, unchanged.

import bevy_pbr::render::forward_io::{Vertex, VertexOutput, decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import super::building_slot::{dark, fallen, GONE};

// The lit rooms at night (openspec change `night-voxel-light`, `cb_core::night_light`): x the share of rooms lit
// now, y a lit room's brightness cd/m2, z the glass's night share, w an always-on room's gain. Bound by both building
// materials.
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var<uniform> rooms: vec4<f32>;

// `cb_core::night_light::LIGHT_SCALE`, `ALWAYS_ON` and `ALWAYS_LIT` (a test holds them together).
const LIGHT_SCALE: f32 = 4.0;
const ALWAYS_ON: f32 = 4094.0;
const ALWAYS_LIT: f32 = 4095.0;

// A vertex's room light, unpacked from its second UV's `y` (`cb_core::night_light::pack_room`: the room's index
// in the whole part, its light in the fraction), and nothing if its room is dark now: a room is lit when its
// hash, of its index and the mesh's world origin (so two houses of one kit light different rooms), falls under
// the share lit now. A site's room (`ALWAYS_LIT`) comes on with the night glass and stays on all night; a tower's
// cab (`ALWAYS_ON`) is on day and night at `rooms.w`. Decided here, per vertex, because an index cannot be blended
// across a triangle.
fn room_light(packed: f32, origin: vec3<f32>) -> f32 {
    // A window pane's inner face carries -1 (`small_buildings::light_glass_sides`): passed on as it is.
    if (packed < 1.0) {
        return min(packed, 0.0);
    }
    let cell = floor(packed);
    let light = (packed - cell) * LIGHT_SCALE;
    if (cell >= ALWAYS_LIT) {
        return light * rooms.z;
    }
    if (cell >= ALWAYS_ON) {
        return light * rooms.w;
    }
    if (rooms.x <= 0.0) {
        return 0.0;
    }
    let h = fract(sin(dot(vec3<f32>(cell * 0.618, origin.x * 0.013, origin.z * 0.017), vec3<f32>(127.1, 311.7, 74.7))) * 43758.547);
    return select(0.0, light, h < rooms.x);
}

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
@if(VERTEX_UVS_B) {
    out.uv_b = vec2<f32>(vertex.uv_b.x, room_light(vertex.uv_b.y, world_from_local[3].xyz));
}

@if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex_in.instance_index);

@if(VERTEX_COLORS) {
    if fallen(vertex.color) {
        out.position = GONE;
    }
    // The alpha (the slot) goes back to 1, or to 2 for a vertex of a building whose windows have gone
    // out (a quarter-wrecked frame keeps them: `structure.lights_out_share`): the glass shader turns its
    // windows out by it (day-night-cycle) and puts it back to 1.
    out.color = vec4<f32>(vertex.color.rgb, select(1.0, 2.0, dark(vertex.color)));
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);

    return out;
}
