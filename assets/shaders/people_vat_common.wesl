// The people's vertex animation (openspec change `people-vat`, design section 4), shared by the main and
// prepass vertex shaders (`people_vat`, `people_vat_prepass`). A level's draw mesh is 32 copies of one body;
// each vertex carries its vertex number and copy in its UV, and its shade, colour region and hair style in its
// colour (r, g, b). The person drawn by a copy is `people[tag + copy]` (the chunk entity's `MeshTag` is its first
// slot). Each texel of the level's texture is one vertex in one frame: the position quantised to 16 bits a
// component over `look.bounds`, and the octahedral normal, 8 bits a component, in the fourth.

import bevy_pbr::render::mesh_functions::get_tag;

// `PeopleLook` in `people_vat.rs`, field for field.
struct PeopleLook {
    bounds_min: vec4<f32>,
    bounds_max: vec4<f32>,
    // skin 0-7, hair 8-15, tops 16-31, bottoms 32-43, shoes 44-51, the eyes' whites 52 (linear RGB)
    palette: array<vec4<f32>, 56>,
}

// `PersonGpu` in `people_vat.rs`, field for field.
struct Person {
    // feet (x, y, z) and heading (radians, 0 facing -z, positive turning left)
    place: vec4<f32>,
    // scale, the frame of the clip playing, the frame of the clip fading out, the playing clip's weight
    frames: vec4<f32>,
    // the playing clip (first row | frames << 16), the fading clip (the same), the palette, the flags
    clips: vec4<u32>,
}

const FLAG_SHOWN: u32 = 1u;
const FLAG_NOW_LOOPS: u32 = 2u;
const FLAG_PREV: u32 = 4u;
const FLAG_PREV_LOOPS: u32 = 8u;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> look: PeopleLook;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var vat: texture_2d<u32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var<storage, read> people: array<Person>;

// A clip position outside the view volume: a triangle whose corners all land here draws nothing.
const GONE: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);

struct Posed {
    shown: bool,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    color: vec4<f32>,
}

fn decode_normal(w: u32) -> vec3<f32> {
    let e = vec2<f32>(f32(w & 255u), f32(w >> 8u)) / 255.0 * 2.0 - 1.0;
    var n = vec3<f32>(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
    if n.z < 0.0 {
        let s = select(vec2<f32>(-1.0), vec2<f32>(1.0), n.xy >= vec2<f32>(0.0));
        n = vec3<f32>((1.0 - abs(n.yx)) * s, n.z);
    }
    return normalize(n);
}

// One vertex in one row: position (person space, metres) in xyz, and the normal.
fn texel(v: u32, row: u32) -> mat2x3<f32> {
    let q = textureLoad(vat, vec2<u32>(v, row), 0);
    let p = look.bounds_min.xyz + vec3<f32>(q.xyz) / 65535.0 * (look.bounds_max.xyz - look.bounds_min.xyz);
    return mat2x3<f32>(p, decode_normal(q.w));
}

// A clip's pose at a fraction of a frame: the two frames either side, mixed.
fn sample_clip(v: u32, clip: u32, frame: f32, loops: bool) -> mat2x3<f32> {
    let first = clip & 0xffffu;
    let count = clip >> 16u;
    let f0 = u32(floor(frame)) % count;
    var f1 = f0 + 1u;
    if f1 >= count {
        f1 = select(count - 1u, 0u, loops);
    }
    let a = texel(v, first + f0);
    let b = texel(v, first + f1);
    let t = fract(frame);
    return mat2x3<f32>(mix(a[0], b[0], t), mix(a[1], b[1], t));
}

fn palette_index(region: u32, packed: u32) -> u32 {
    switch region {
        case 0u: { return packed & 7u; }
        case 1u: { return 16u + ((packed >> 6u) & 15u); }
        case 2u: { return 32u + min((packed >> 10u) & 15u, 11u); }
        case 3u: { return 44u + ((packed >> 14u) & 7u); }
        case 5u: { return 52u; }
        default: { return 8u + ((packed >> 3u) & 7u); }
    }
}

// The vertex `uv.x` of copy `uv.y` posed for its person: the clip playing crossfaded with the one fading
// out, scaled to the person's height, turned to their heading and set on their feet.
fn pose(instance_index: u32, uv: vec2<f32>, color: vec4<f32>) -> Posed {
    var out: Posed;
    let slot = get_tag(instance_index) + u32(uv.y + 0.5);
    let p = people[slot];
    out.shown = (p.clips.w & FLAG_SHOWN) != 0u;
    if !out.shown {
        return out;
    }
    // a hair style other than the person's own (bits 17-19 of the palette word; 0 bald) is not drawn
    let style = u32(color.b + 0.5);
    if style != 0u && style != ((p.clips.z >> 17u) & 7u) {
        out.shown = false;
        return out;
    }
    let v = u32(uv.x + 0.5);
    var pn = sample_clip(v, p.clips.x, p.frames.y, (p.clips.w & FLAG_NOW_LOOPS) != 0u);
    if (p.clips.w & FLAG_PREV) != 0u {
        let pp = sample_clip(v, p.clips.y, p.frames.z, (p.clips.w & FLAG_PREV_LOOPS) != 0u);
        pn = mat2x3<f32>(mix(pp[0], pn[0], p.frames.w), mix(pp[1], pn[1], p.frames.w));
    }
    let c = cos(p.place.w);
    let s = sin(p.place.w);
    let turn = mat3x3<f32>(vec3<f32>(c, 0.0, -s), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(s, 0.0, c));
    out.world_position = p.place.xyz + turn * (pn[0] * p.frames.x);
    out.world_normal = normalize(turn * pn[1]);
    let region = u32(color.g + 0.5);
    out.color = vec4<f32>(look.palette[palette_index(region, p.clips.z)].rgb * color.r, 1.0);
    return out;
}
