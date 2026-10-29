// The fragment shader of building meshes drawn with `BuildingMaterial` (openspec changes
// `building-gpu-state`, `ambient-occlusion`): Bevy's own PBR path (bevy_pbr 0.20.0,
// `src/render/pbr.wesl`, cut to what buildings use), except that glass takes no screen-space
// ambient occlusion. Glass is transparent, so it is not in the depth and normal prepass SSAO reads:
// at a window the occlusion of the floors and rooms behind it darkened its sky reflection away, and
// the windows read as see-through with the floor edges showing white (2026-10-24 stills).

import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing};
import bevy_pbr::render::mesh_view_bindings::view;

// 1 for glass: no screen-space ambient occlusion. `FAR_WINDOW` for a house's far window pane, lit at night.
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var<uniform> no_ssao: u32;
// `small_buildings::FAR_WINDOW_FLAG` and `FURNITURE_FLAG` (a test holds them together). The furniture keeps its
// occlusion, and its pieces' own slots do not mark it a damaged building's: its room light is baked dark instead.
const FAR_WINDOW: u32 = 2u;
const FURNITURE: u32 = 4u;
// The windows at night (`building_windows`): the glass no longer glows (owner, 2026-10-29: "I dont want the glass
// itself to glow, I want to see the room lits"); the rooms behind it light instead. y: a house's far window's
// brightness, cd/m2.
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var<uniform> windows: vec4<f32>;
// The lit rooms (`building_state.wesl` decides which): y a lit room's brightness cd/m2 (houses' inside surfaces
// and furniture).
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var<uniform> rooms: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var<uniform> room_colour: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var v = in;
    var damaged = false;
    @if(VERTEX_COLORS) {
        damaged = in.color.a > 1.5 && no_ssao != FURNITURE;
        v.color.a = 1.0;
    }
    var pbr = pbr_input_from_standard_material(v, is_front);
    if no_ssao != 0u && no_ssao != FURNITURE {
        pbr.diffuse_occlusion = vec3<f32>(1.0);
        pbr.specular_occlusion = 1.0;
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
@if(VERTEX_UVS_B) {
    if (!damaged && in.uv_b.y > 0.0) {
        if (no_ssao == FAR_WINDOW) {
            // A house's far window pane (LOD1): its house lit, it glows as the towers' far walls' lit windows do.
            out.color = vec4<f32>(out.color.rgb + room_colour.rgb * in.uv_b.y * windows.y * view.exposure, out.color.a);
        } else {
            out.color = vec4<f32>(out.color.rgb + pbr.material.base_color.rgb * room_colour.rgb * in.uv_b.y * rooms.y * view.exposure, out.color.a);
        }
    }
}
    out.color = main_pass_post_lighting_processing(pbr, out.color);
@if(VERTEX_UVS_B) {
    // A window pane's inner face (its second UV's y -1): clear from inside, at `windows.z`.
    if (no_ssao == 1u && in.uv_b.y < -0.5) {
        out.color = vec4<f32>(out.color.rgb, windows.z);
    }
}
    return out;
}
