// The element state of a building (openspec change `building-gpu-state`): which of its frame's
// elements stand, one word each, rewritten from the CPU when the building changes. Bit 1 of a word: the
// element stands; bit 2: the building's windows have gone out (`building_state::standing_words`). Every vertex of
// a building mesh carries its element's slot in its colour's alpha: 0 always shown, n > 0 element
// n - 1. Shared by the main and prepass vertex shaders (`building_state`, `building_state_prepass`).

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<storage, read> standing: array<u32>;

// True when the vertex's element has fallen or broken (its triangles are dropped).
fn fallen(color: vec4<f32>) -> bool {
    let slot = u32(round(color.a));
    if slot == 0u {
        return false;
    }
    let id = slot - 1u;
    return id >= arrayLength(&standing) || (standing[id] & 1u) == 0u;
}

// True when the vertex's building has had its windows go out (enough of it has fallen).
fn dark(color: vec4<f32>) -> bool {
    let slot = u32(round(color.a));
    if slot == 0u {
        return false;
    }
    let id = slot - 1u;
    return id < arrayLength(&standing) && (standing[id] & 2u) != 0u;
}

// A clip position outside the view volume: a triangle whose three corners all land here
// rasterises nothing, in any pass.
const GONE: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
