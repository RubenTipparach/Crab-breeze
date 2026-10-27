// GPU grass (openspec change `gpu-grass`): every blade is built here, in the vertex shader, from
// its index alone. A tile is one mesh instance of a shared blade mesh per level of detail, whose
// vertices carry only (blade index, vertex in blade, level); the tile's corner is its transform,
// and its slot in the tile buffer is its `MeshTag`. The slot holds the ground's height on the
// terrain's lattice and where grass grows, baked on the CPU from the one height query (no terrain
// noise here). Lit by Bevy's PBR lights and fog, shadows received, none cast.
//
// The method is Ghost of Tsushima's (Eric Wohllaib, "Procedural Grass in Ghost of Tsushima", GDC
// 2021) as its open reimplementations carry it (for one, `2Retr0/GodotGrass`), with the practices of
// GPU Gems 2 chapter 1 (Pelzer, "Rendering Countless Blades of Waving Grass"), Orthmann et al. 2008
// and the hexaquo.at grass series: a blade is a strip along a quadratic Bezier curve set by its
// height, lean, droop, stiffness and the wind; Voronoi clumps that share a height, lean, facing and
// hue, some standing tall; layered wind (a broad sway, gust waves rolling across the field and
// lightening the blades they bend, each blade's sway and flutter, all weighted to the tip); fake
// ambient occlusion at the root, deeper in dense tall clumps; edge-on blades turned toward the
// eye; normals rounded across the width; the sun glowing through a blade seen against it; a sheen
// that fades with distance while far normals turn up, so far grass does not alias. Written for Crab
// Breeze, no code copied.

import bevy_pbr::render::mesh_functions::{get_world_from_local, get_tag};
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{view, globals, lights};
import bevy_pbr::render::mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT;
import bevy_pbr::render::shadows::fetch_directional_shadow;
import bevy_pbr::render::pbr_types::{pbr_input_new, STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT};
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing};
import bevy_pbr::render::mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT;

// Matches `GrassUniform` in `crates/cb-app/src/grass.rs`.
struct GrassLook {
    blade: vec4<f32>,    // height min m, height max m, root width m, taper exponent
    bend: vec4<f32>,     // lean max, stiffness 0..1, mid control point height share, edge thickening
    shade: vec4<f32>,    // normal rounding, root light, roughness, dry share
    root: vec4<f32>,     // rgb root colour (linear albedo); w spare
    tip: vec4<f32>,      // rgb tip colour; w spare
    dry: vec4<f32>,      // rgb dry colour; w spare
    wind: vec4<f32>,     // heading x, heading z, speed m/s, steady push share
    gust: vec4<f32>,     // noise cells per m, gust share, sway Hz, sway share
    clump: vec4<f32>,    // droop, tall clump share, tall clump scale, edge shrink
    ao: vec4<f32>,       // dense darkening, AO height share, reflectance, hue variety
    hue: vec4<f32>,      // rgb second green; w tip lightening
    glow: vec4<f32>,     // rgb translucent colour; w translucency
    gloss: vec4<f32>,    // translucency power, far roughness, gloss fade start m, end m
    far: vec4<f32>,      // normals turn up from m, to m; zw spare
    gust2: vec4<f32>,    // gust sharpness, gust brightening, flutter Hz, flutter share
    broad: vec4<f32>,    // broad sway wavelength m, Hz, push share; w spare
    tile: vec4<f32>,     // tile m, blade spacing m, density cell m, terrain cell m
    grid: vec4<u32>,     // blade cells a side, density cells a side, heights a side, words per slot
    fade: vec4<f32>,     // radius m, fade m, clump m, clump strength
    parched: vec4<f32>,  // rgb a parched blade's colour (biome dryness); w spare
    lods: array<vec4<f32>, 4>, // stride, segments, width scale, until m
    next: array<vec4<f32>, 4>, // next level's stride (0: none), its width scale, crossfade m, spare
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> grass: GrassLook;
// Every tile's slot: heights (f32 bits), then density, then dryness, each four cells a word, low byte first.
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<storage, read> tiles: array<u32>;
// Bodies pressing the grass flat: xyz centre (world m), w radius m (0: none).
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<storage, read> pushers: array<vec4<f32>>;

struct GrassVertex {
    @builtin(instance_index) instance_index: u32,
    // x blade index, y vertex in blade, z level of detail (see `cb_core::grass::BladeMesh`).
    @location(0) blade: vec3<f32>,
}

struct GrassFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    // rgb albedo (root darkening and gust lightening in it); a the place along the blade, 0 root
    // to 1 tip.
    @location(2) color: vec4<f32>,
}

const TAU: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;

// PCG hash (Jarzynski and Olano, "Hash Functions for GPU Rendering", JCGT 2020).
fn pcg(x: u32) -> u32 {
    let s = x * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn hash2(c: vec2<u32>, salt: u32) -> u32 {
    return pcg(c.x ^ pcg(c.y ^ pcg(salt)));
}

fn unit(h: u32) -> f32 {
    return f32(h >> 8u) * (1.0 / 16777216.0);
}

// Four uniform randoms in 0..1 for cell `c`.
fn rand4(c: vec2<u32>, salt: u32) -> vec4<f32> {
    let a = hash2(c, salt);
    let b = pcg(a);
    let d = pcg(b);
    let e = pcg(d);
    return vec4<f32>(unit(a), unit(b), unit(d), unit(e));
}

fn cell_of(p: vec2<f32>) -> vec2<u32> {
    return bitcast<vec2<u32>>(vec2<i32>(floor(p)));
}

// Smooth value noise in 0..1.
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    let c = cell_of(i);
    let a = unit(hash2(c, 11u));
    let b = unit(hash2(c + vec2<u32>(1u, 0u), 11u));
    let d = unit(hash2(c + vec2<u32>(0u, 1u), 11u));
    let e = unit(hash2(c + vec2<u32>(1u, 1u), 11u));
    return mix(mix(a, b, u.x), mix(d, e, u.x), u.y);
}

// The Voronoi clump holding `p` (world metres): its cell (for its randoms) and how far `p` is
// toward its edge, 0 at its seed to 1 at about its edge.
struct Clump {
    cell: vec2<u32>,
    edge: f32,
}

fn clump_at(p: vec2<f32>) -> Clump {
    let g = p / grass.fade.z;
    let base = floor(g);
    var best = 1e9;
    var cell = cell_of(base);
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let c = base + vec2<f32>(f32(i), f32(j));
            let seed = c + rand4(cell_of(c), 2u).xy;
            let d = distance(g, seed);
            if d < best {
                best = d;
                cell = cell_of(c);
            }
        }
    }
    return Clump(cell, clamp(best / 0.8, 0.0, 1.0));
}

// The ground's height at `local` (metres from the tile's corner): the terrain's lattice read on the
// same two triangles a cell as every ground mesh (`cb_core::world::terrain::cell_height`).
fn tile_height(base: u32, local: vec2<f32>) -> f32 {
    let n = grass.grid.z;
    let g = local / grass.tile.w;
    let i = min(vec2<u32>(max(floor(g), vec2<f32>(0.0))), vec2<u32>(n - 2u));
    let f = g - vec2<f32>(i);
    let at = base + i.y * n + i.x;
    let h00 = bitcast<f32>(tiles[at]);
    let h11 = bitcast<f32>(tiles[at + n + 1u]);
    if f.x >= f.y {
        let h10 = bitcast<f32>(tiles[at + 1u]);
        return h00 + f.x * (h10 - h00) + f.y * (h11 - h10);
    }
    let h01 = bitcast<f32>(tiles[at + n]);
    return h00 + f.y * (h01 - h00) + f.x * (h11 - h01);
}

// Where grass grows at `local`, 0..1.
fn tile_density(base: u32, local: vec2<f32>) -> f32 {
    let d = grass.grid.y;
    let c = min(vec2<u32>(max(floor(local / grass.tile.z), vec2<f32>(0.0))), vec2<u32>(d - 1u));
    let index = c.y * d + c.x;
    let word = tiles[base + grass.grid.z * grass.grid.z + index / 4u];
    return f32((word >> ((index % 4u) * 8u)) & 255u) * (1.0 / 255.0);
}

// How parched the grass is at `local`, 0..1 (the biome's dryness: beige on sand, green in a meadow).
fn tile_dryness(base: u32, local: vec2<f32>) -> f32 {
    let d = grass.grid.y;
    let c = min(vec2<u32>(max(floor(local / grass.tile.z), vec2<f32>(0.0))), vec2<u32>(d - 1u));
    let index = c.y * d + c.x;
    let word = tiles[base + grass.grid.z * grass.grid.z + (d * d + 3u) / 4u + index / 4u];
    return f32((word >> ((index % 4u) * 8u)) & 255u) * (1.0 / 255.0);
}

@vertex
fn vertex(in: GrassVertex) -> GrassFragment {
    var out: GrassFragment;
    let blade = u32(in.blade.x);
    let k = u32(in.blade.y);
    let lod = min(u32(in.blade.z), 3u);
    let level = grass.lods[lod];
    let next = grass.next[lod];
    let stride = max(u32(level.x), 1u);
    let segments = max(u32(level.y), 1u);

    // The blade's cell in the tile, at the nearest level's spacing.
    let per_row = (grass.grid.x + stride - 1u) / stride;
    let cell = vec2<u32>(blade % per_row, blade / per_row) * stride;
    let world_from_local = get_world_from_local(in.instance_index);
    let corner = world_from_local[3].xz;
    let base = get_tag(in.instance_index) * grass.grid.w;
    let spacing = grass.tile.y;
    // The cell counted from the world's origin: the same blade whatever tile or level draws it.
    let world_cell = bitcast<vec2<u32>>(vec2<i32>(round(corner / spacing)) + vec2<i32>(cell));
    let r = rand4(world_cell, 1u);
    let local = (vec2<f32>(cell) + r.xy) * spacing;
    let root_xz = corner + local;
    let root = vec3<f32>(root_xz.x, tile_height(base, local), root_xz.y);
    let eye = view.world_position.xyz;
    let distance = length(eye - root);

    // How much of the blade stands: where grass grows (a blade stands if its roll is under the
    // density), shrinking away toward the radius, and, for the blades the next level drops,
    // toward this level's boundary.
    let density = tile_density(base, local);
    var stand = select(0.0, 1.0, r.z < density);
    stand *= 1.0 - smoothstep(grass.fade.x - grass.fade.y, grass.fade.x, distance);
    let next_stride = u32(next.x);
    let toward = smoothstep(level.w - next.z, level.w, distance);
    var width_scale = level.z;
    if next_stride > 0u {
        let dropped = (cell.x % next_stride) != 0u || (cell.y % next_stride) != 0u;
        if dropped {
            stand *= 1.0 - toward;
        } else {
            width_scale = mix(level.z, next.y, toward);
        }
    }

    // The blade's Voronoi clump: it shares a height, lean, facing and hue, some clumps stand tall,
    // and a clump is a mound (its edge blades shorter).
    let clump = clump_at(root_xz);
    let ch = rand4(clump.cell, 4u);
    let ch2 = rand4(clump.cell, 5u);
    let own = rand4(world_cell, 3u);
    let cs = grass.fade.w;
    let tall = select(1.0, grass.clump.z, ch2.x < grass.clump.y);
    let mound = 1.0 - grass.clump.w * clump.edge * clump.edge;
    let tallest = grass.blade.y * grass.clump.z;
    let height = mix(grass.blade.x, grass.blade.y, mix(own.x, ch.x, cs)) * tall * mound * stand;
    if height < 0.005 {
        // Nothing stands: every vertex of the blade at one point outside the view, so its
        // triangles have no area and are clipped.
        out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0);
        return out;
    }
    let tallness = clamp(height / tallest, 0.0, 1.0);
    let own_dir = vec2<f32>(cos(own.y * TAU), sin(own.y * TAU));
    let clump_dir = vec2<f32>(cos(ch.y * TAU), sin(ch.y * TAU));
    let facing_xz = normalize(mix(own_dir, clump_dir, cs) + vec2<f32>(1e-4, 0.0));
    let facing = vec3<f32>(facing_xz.x, 0.0, facing_xz.y);
    let across = vec3<f32>(-facing.z, 0.0, facing.x);

    // Layered wind, as horizontal shares of the height at the tip: a steady push; a broad sway
    // across the field; gust waves, noise scrolled with the wind and sharpened into bands; each
    // blade's own sway. Then the lean (shared with the clump) and the droop of a tall blade, and
    // pressed flat away from any body standing on it.
    let wind_dir = normalize(grass.wind.xy + vec2<f32>(1e-6, 0.0));
    let t = globals.time;
    let gust_noise = value_noise(root_xz * grass.gust.x - wind_dir * (grass.wind.z * t * grass.gust.x));
    let gust = pow(gust_noise, grass.gust2.x);
    let push = grass.wind.w * (1.0 - grass.gust.y + 2.0 * grass.gust.y * gust);
    let broad = sin(dot(root_xz, wind_dir) * TAU / grass.broad.x - t * TAU * grass.broad.y) * grass.broad.z;
    let sway = sin(t * TAU * grass.gust.z + own.z * TAU) * grass.gust.w;
    let lean = mix(own.w, ch.z, cs) * grass.bend.x + grass.clump.x * tallness;
    var tip = facing.xz * lean + wind_dir * (push + broad + sway);
    var flat = 0.0;
    for (var i = 0u; i < arrayLength(&pushers); i++) {
        let p = pushers[i];
        if p.w > 0.0 {
            let d = root - p.xyz;
            let reach = 1.0 - length(d) / p.w;
            if reach > 0.0 {
                let away = normalize(d.xz + vec2<f32>(1e-4, 0.0));
                let press = smoothstep(0.0, 0.35, reach);
                tip = mix(tip, away * 0.97, press);
                flat = max(flat, press);
            }
        }
    }
    let bent = min(length(tip), 0.97);
    let tip_dir = normalize(vec3<f32>(tip.x, 0.0, tip.y) + vec3<f32>(0.0, 1e-4, 0.0));
    let tip_flat = vec3<f32>(tip_dir.x, 0.0, tip_dir.z) * bent;
    let p2 = (tip_flat + vec3<f32>(0.0, sqrt(1.0 - bent * bent), 0.0)) * height;
    // The middle control point: straight up from the root for a stiff blade (it bends near the
    // tip, a tall one drooping over), toward the tip for a soft one (it bends from the root).
    let p1 = vec3<f32>(0.0, grass.bend.z * height * (1.0 - 0.6 * flat), 0.0) + tip_flat * height * (1.0 - grass.bend.y) * 0.5;

    // This vertex: its place along the blade and its side.
    var along = 1.0;
    var side = 0.0;
    if k < 2u * segments {
        along = f32(k / 2u) / f32(segments);
        side = select(-1.0, 1.0, (k & 1u) == 1u);
    }
    let a = 1.0 - along;
    var curve = 2.0 * a * along * p1 + along * along * p2;
    let tangent = normalize(2.0 * a * p1 + 2.0 * along * (p2 - p1) + vec3<f32>(0.0, 1e-5, 0.0));

    // An edge-on blade turns toward the eye a little, so it never thins to nothing.
    let to_eye = normalize(eye.xz - root_xz + vec2<f32>(1e-4, 0.0));
    let edge_on = 1.0 - abs(dot(facing.xz, to_eye));
    let square = vec3<f32>(-to_eye.y, 0.0, to_eye.x) * sign(dot(vec2<f32>(-to_eye.y, to_eye.x), across.xz) + 1e-6);
    let width_dir = normalize(mix(across, square, edge_on * grass.bend.w));
    // Flutter: a quick shiver across the blade, growing toward the tip.
    let flutter = sin(t * TAU * grass.gust2.z + own.x * TAU + along * 2.5) * grass.gust2.w * height * along * along * (1.0 - flat);
    curve += width_dir * flutter;
    let width = grass.blade.z * width_scale * (1.0 - pow(along, grass.blade.w));
    let world = root + curve + width_dir * (side * width * 0.5);

    // The normal: the strip's, tilted across its width so it shades like a curved leaf.
    let flat_normal = normalize(cross(width_dir, tangent));
    let normal = normalize(flat_normal + width_dir * (side * grass.shade.x));

    // Colour: root to tip, each clump's hue moved toward the second green, straw-coloured dry
    // blades scattered through, tips lighter; darker at the root where blades shade each other
    // (more in dense, tall clumps); lighter where a gust bends the blades over.
    let tip_green = mix(grass.tip.rgb, grass.hue.rgb, ch.w * grass.ao.w);
    let green = mix(grass.root.rgb, tip_green, pow(along, 0.8));
    let dry = select(0.0, 1.0, own.z < grass.shade.w);
    var colour = mix(green, mix(grass.root.rgb, grass.dry.rgb, sqrt(along)), dry * 0.9);
    // Where the biome is dry (sand, desert, savanna) the whole tuft is parched beige, root a shade darker.
    let parched = tile_dryness(base, local);
    colour = mix(colour, grass.parched.rgb * mix(0.55, 1.0, along), parched);
    colour = mix(colour, vec3<f32>(1.0), grass.hue.w * along * along);
    let root_floor = grass.shade.y * (1.0 - grass.ao.x * density * tallness);
    let light = mix(root_floor, 1.0, smoothstep(0.0, grass.ao.y, along));
    let bright = 1.0 + grass.gust2.y * gust * along;

    out.world_position = vec4<f32>(world, 1.0);
    out.position = position_world_to_clip(world);
    out.world_normal = normal;
    out.color = vec4<f32>(colour * light * bright, along);
    return out;
}

@fragment
fn fragment(in: GrassFragment, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let to_eye = view.world_position.xyz - in.world_position.xyz;
    let distance = length(to_eye);
    let V = to_eye / max(distance, 1e-4);
    var pbr = pbr_input_new();
    pbr.material.base_color = vec4<f32>(in.color.rgb, 1.0);
    // A soft sheen near, faded to matte far away so thin blades do not sparkle.
    pbr.material.perceptual_roughness = mix(grass.shade.z, grass.gloss.y, smoothstep(grass.gloss.z, grass.gloss.w, distance));
    pbr.material.reflectance = vec3<f32>(grass.ao.z);
    pbr.material.flags |= STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    // Receives shadows, casts none (no shadow pass draws grass).
    pbr.flags = MESH_FLAGS_SHADOW_RECEIVER_BIT;
    pbr.frag_coord = in.position;
    pbr.world_position = in.world_position;
    // Both faces are drawn: the back face's normal is the front's turned round. Far away the
    // normal turns up, so far grass shades like the ground under it and does not alias.
    let facing_normal = normalize(select(-in.world_normal, in.world_normal, is_front));
    let n = normalize(mix(facing_normal, vec3<f32>(0.0, 1.0, 0.0), smoothstep(grass.far.x, grass.far.y, distance)));
    pbr.world_normal = n;
    pbr.N = n;
    pbr.V = V;
    var lit = apply_pbr_lighting(pbr);

    // Translucency: the sun glows through a blade seen against it, yellow-green, more toward the
    // thin tip, shadowed where the blade is in shadow.
    if lights.n_directional_lights > 0u {
        let sun = lights.directional_lights[0];
        let L = sun.direction_to_light;
        let behind = pow(clamp(dot(-V, L), 0.0, 1.0), grass.gloss.x);
        var shadow = 1.0;
        if (sun.flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            let view_z = dot(vec4<f32>(
                view.view_from_world[0].z,
                view.view_from_world[1].z,
                view.view_from_world[2].z,
                view.view_from_world[3].z
            ), in.world_position);
            shadow = fetch_directional_shadow(0u, in.world_position, n, view_z, in.position.xy);
        }
        let through = grass.glow.rgb * sun.color.rgb * (behind * grass.glow.w * shadow * (0.3 + 0.7 * in.color.a) / PI);
        lit = vec4<f32>(lit.rgb + through, lit.a);
    }
    return main_pass_post_lighting_processing(pbr, lit);
}
