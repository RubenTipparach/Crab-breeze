// Ground marks (openspec change `ground-marks`): every footprint, skate trail and crater scorch a
// mech leaves, in one draw. One mesh holds a 5 by 5 grid of vertices for every slot of the mark
// ring; a vertex carries only (slot, grid x, grid z), and the vertex shader reads its mark from the
// storage buffer (`cb_core::ground_marks::Mark::pack`), puts each vertex on the ground height
// sampled for it (on the CPU, from the one height query; no terrain noise here) and lifts it a
// little toward the eye. An empty or faded slot collapses to nothing.
//
// The fragment is Mining Mike's dent shader (`godot-sandbox/shaders/ground_dent.gdshader`, branch
// `origin/claude/long-haul-playtest`, commit 74e1412c; the Ravager Fight page's print shader,
// approved 2026-10-22), ported to WGSL: the view ray is marched down the mark's height field
// (parallax), the normal is taken off the field where it lands and lit against the sun with a short
// shadow march, and the result is drawn as a MULTIPLY over what is already there (1 leaves it as
// it is; above 1 lightens, on the HDR target), so the ground's own texture and light show through.
// Crab Breeze adds the surfaces (design section 10): soil is deep, broken into clods and turned dark
// brown, with a lip; paving is a crushed sole broken into slabs, outlined, with long radial cracks
// and a ring of pale dust; a skate line is a groove in soil and a rubber streak on paving.
//
// Bevy binds the view's lights to the fragment stage only: the vertex stage must never read
// `lights` (a test in `cb-app::shader_check` checks every game shader).

import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{view, lights};

// Matches `MarksUniform` in `crates/cb-app/src/ground_marks.rs`.
struct MarksLook {
    reach: vec4<f32>,   // draw within m, lift m, pull toward the eye per m of distance, ambient share
    soil: vec4<f32>,    // rgb pressed soil tint (multiplies); w darkening at the bottom of a print
    paving: vec4<f32>,  // rgb crack colour (multiplies); w dust brightening
    crack: vec4<f32>,   // crack width m, crack reach (sole half lengths), crushed sole, slab size m
}

// One mark, ten rows (`cb_core::ground_marks::ROW_FLOATS`).
struct Mark {
    centre: vec4<f32>,  // xyz middle on the ground, w yaw (local +z along (sin, 0, cos))
    size: vec4<f32>,    // quad half size across m, along m, depth m, lip m
    kind: vec4<f32>,    // shape (0 foot, 1 groove, 2 crater, 3 trail), surface (0 soil, 1 paving), fade, seed
    // The ground's height at each of the 5 by 5 grid's vertices, across then along, relative to the
    // middle (25 floats); then the shape scale (the quad's half size over the shape's, x and y) and
    // a spare.
    h: array<vec4<f32>, 7>,
}

// Vertices a side of a mark's grid (`cb_core::ground_marks::GRID`).
const GRID: u32 = 5u;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> look: MarksLook;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<storage, read> marks: array<Mark>;

struct MarkVertex {
    // x the mark's slot, y and z the vertex's place on the 5 by 5 grid (0..4).
    @location(0) grid: vec3<f32>,
}

struct MarkFragment {
    @builtin(position) position: vec4<f32>,
    // xy the place on the quad, -1..1 across and along; z the distance fade.
    @location(0) local: vec3<f32>,
    @location(1) @interpolate(flat) slot: u32,
    // The view direction in the mark's frame (across, up, along), unscaled.
    @location(2) to_eye: vec3<f32>,
}

const TAU: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;

// The sampled ground height at grid vertex `k` (row-major, across then along) of slot `slot`.
fn grid_height(slot: u32, k: u32) -> f32 {
    return marks[slot].h[k / 4u][k % 4u];
}

// The quad's half size over the shape's own (1 when the shape fills the quad).
fn shape_scale(m: Mark) -> vec2<f32> {
    return max(m.h[6].yz, vec2<f32>(1.0));
}

// The mark's frame: across, up (the ground's normal over the mark, from the grid's middle row and
// column) and along.
fn frame(slot: u32, m: Mark) -> mat3x3<f32> {
    let s = sin(m.centre.w);
    let c = cos(m.centre.w);
    let across0 = vec3<f32>(c, 0.0, -s);
    let along0 = vec3<f32>(s, 0.0, c);
    let mid = GRID / 2u;
    let dx = (grid_height(slot, mid * GRID + GRID - 1u) - grid_height(slot, mid * GRID)) / (2.0 * max(m.size.x, 1e-3));
    let dz = (grid_height(slot, (GRID - 1u) * GRID + mid) - grid_height(slot, mid)) / (2.0 * max(m.size.y, 1e-3));
    let across = normalize(across0 + vec3<f32>(0.0, dx, 0.0));
    let along = normalize(along0 + vec3<f32>(0.0, dz, 0.0));
    let up = normalize(cross(along, across));
    return mat3x3<f32>(across, up, along);
}

fn sd_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// A mech's sole, in Mining Mike's units (the quad 1.55 times the sole): wider at the toe (+y),
// rounded, its front split into three toes by two notches.
fn sd_foot(p: vec2<f32>) -> f32 {
    let w = mix(0.40, 0.56, smoothstep(-0.55, 0.25, p.y));
    let body = sd_box(p, vec2<f32>(w, 0.62), 0.2);
    let notch = sd_box(vec2<f32>(abs(p.x) - 0.19, p.y - 0.58), vec2<f32>(0.025, 0.2), 0.02);
    return max(body, -notch);
}

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise1(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash21(vec2<f32>(i, 0.0)), hash21(vec2<f32>(i + 1.0, 0.0)), u);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// The mark's height field (Mining Mike's `H`), at `uv` on the quad: a pit `depth` deep inside the
// shape, a lip `lip` high just outside it; tread bars in a footprint; a ragged bowl for a crater.
// `k` is the shape scale; `clods` (soil) breaks the lip and the bottom into lumps.
fn field(uv: vec2<f32>, shape: f32, depth: f32, lip: f32, k: vec2<f32>, clods: f32, seed: f32) -> f32 {
    let p = uv * k;
    var d: f32;
    var tread = 0.0;
    if shape < 0.5 {
        d = sd_foot(p);
        tread = smoothstep(0.62, 0.8, abs(fract(p.y * 2.6 + 0.25) - 0.5) * 2.0) * step(-0.45, p.y);
    } else if shape < 1.5 {
        d = sd_box(p, vec2<f32>(0.34, 0.82), 0.33);
    } else if shape > 2.5 {
        // A skate's line: a straight channel the whole length of its quad, which it fills end to
        // end, so the next segment carries one channel on with no wall and no doubled darkening.
        d = abs(p.x) - 0.34;
    } else {
        let a = atan2(p.y, p.x);
        let rough = 0.06 * sin(a * 7.0) + 0.04 * sin(a * 13.0 + 1.3);
        let r = length(p) / 0.6;
        d = length(p) - 0.6 * (1.0 + rough);
        let bowl = -depth * (1.0 - r * r) * step(r, 1.0 + rough);
        let g = (d - 0.08) / 0.12;
        return mix(bowl, -depth * smoothstep(0.08, -0.1, d), 0.25) + lip * 1.6 * exp(-g * g);
    }
    let inside = smoothstep(0.1, -0.12, d);
    var pit = -depth * inside * (1.0 - 0.3 * tread);
    let g = (d - 0.12) / 0.11;
    var rim = lip * exp(-g * g);
    if clods > 0.5 {
        // Broken soil: the lip heaped in lumps, the bottom uneven.
        let n = noise2(p * 7.0 + seed * 31.0);
        rim *= 0.45 + 1.1 * n;
        pit += depth * 0.22 * (noise2(p * 11.0 + seed * 17.0) - 0.5) * inside;
    }
    return pit + rim;
}

// Voronoi cells of size 1 at `p`: x the distance to the nearest seam (F2 - F1, halved), y the cell's
// own random number.
fn slabs(p: vec2<f32>, seed: f32) -> vec2<f32> {
    let i = floor(p);
    var f1 = 9.0;
    var f2 = 9.0;
    var id = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let c = i + vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(hash21(c + seed * 13.0), hash21(c + seed * 29.0 + 7.0));
            let dist = length(c + o - p);
            if dist < f1 {
                f2 = f1;
                f1 = dist;
                id = hash21(c + 3.7);
            } else if dist < f2 {
                f2 = dist;
            }
        }
    }
    return vec2<f32>((f2 - f1) * 0.5, id);
}

// Radial cracks running out from a print on paving, some long, some short, wandering: 0 nothing,
// 1 a crack. `q` in metres from the middle, `len` the sole's half length in metres.
fn cracks(q: vec2<f32>, len: f32, seed: f32) -> f32 {
    let r = length(q);
    let a = atan2(q.y, q.x);
    let n = 9.0;
    let wander = a + 0.25 * (noise1(r * 2.2 + seed * 37.0) - 0.5) + 0.09 * sin(r * 6.0 + seed * 11.0);
    let x = wander * n / TAU + seed * 5.0;
    let cell = floor(x);
    let to_line_m = (0.5 - abs(fract(x) - 0.5)) * TAU * r / n;
    let reach = len * look.crack.y * (0.55 + 0.5 * hash21(vec2<f32>(cell, seed * 91.0)));
    let w = look.crack.x * (1.0 - 0.65 * smoothstep(len * 0.5, reach, r));
    return (1.0 - smoothstep(w * 0.35, w, to_line_m))
        * smoothstep(len * 0.4, len * 0.6, r)
        * (1.0 - smoothstep(reach * 0.75, reach, r))
        * step(0.25, hash21(vec2<f32>(cell + 17.0, seed * 53.0)));
}

@vertex
fn vertex(in: MarkVertex) -> MarkFragment {
    var out: MarkFragment;
    let slot = u32(in.grid.x);
    let m = marks[slot];
    let gi = min(u32(in.grid.y), GRID - 1u);
    let gj = min(u32(in.grid.z), GRID - 1u);
    let p = vec2<f32>(f32(gi), f32(gj)) * (2.0 / f32(GRID - 1u)) - 1.0;
    let eye = view.world_position.xyz;
    let reach = look.reach.x;
    let gone = m.kind.z <= 0.0 || length(eye - m.centre.xyz) > reach + 2.0 * max(m.size.x, m.size.y);
    if gone {
        // Outside the clip volume: the triangles have no area and are clipped.
        out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0);
        return out;
    }
    let f = frame(slot, m);
    let s = sin(m.centre.w);
    let c = cos(m.centre.w);
    let flat_across = vec3<f32>(c, 0.0, -s);
    let flat_along = vec3<f32>(s, 0.0, c);
    var world = m.centre.xyz + flat_across * (p.x * m.size.x) + flat_along * (p.y * m.size.y);
    world.y = m.centre.y + grid_height(slot, gj * GRID + gi) + look.reach.y;
    // Pulled toward the eye a little more the farther it is, so a mark never shares the ground's
    // depth (no z-fighting, CLAUDE.md section 10), at most half a metre.
    let to_eye = eye - world;
    let d = length(to_eye);
    world += to_eye / max(d, 1e-3) * min(d * look.reach.z, 0.5);
    out.position = position_world_to_clip(world);
    out.local = vec3<f32>(p, 1.0 - smoothstep(reach * 0.8, reach, d));
    out.slot = slot;
    // Into the mark's frame (the inverse of an orthonormal frame is its transpose).
    out.to_eye = transpose(f) * (eye - world);
    return out;
}

@fragment
fn fragment(in: MarkFragment) -> @location(0) vec4<f32> {
    let m = marks[in.slot];
    // The sun, read here: Bevy's view lights are bound for fragments only.
    var sun_dir = vec3<f32>(0.35, 0.8, 0.45);
    if lights.n_directional_lights > 0u {
        sun_dir = lights.directional_lights[0].direction_to_light;
    }
    let L = normalize(transpose(frame(in.slot, m)) * normalize(sun_dir));
    let shape = m.kind.x;
    let paved = m.kind.y > 0.5;
    let fade = m.kind.z;
    let seed = m.kind.w;
    let depth = m.size.z;
    let lip = m.size.w;
    let ks = shape_scale(m);
    let clods = select(1.0, 0.0, paved);
    let scl = max(m.size.xy, vec2<f32>(1e-3));
    // The view ray in quad units across and along, metres up.
    let v = vec3<f32>(in.to_eye.x / scl.x, in.to_eye.y, in.to_eye.z / scl.y);
    let V = normalize(v);
    var uv = in.local.xy;
    let eye_m = length(in.to_eye);
    if V.y > 0.04 && depth + lip > 0.005 {
        // Parallax: march the view ray down from the lip's top to where it meets the field; fewer
        // steps far away, where the relief is a few pixels.
        let steps = i32(mix(28.0, 8.0, smoothstep(10.0, 120.0, eye_m)));
        let per = -V.xz / V.y;
        let top = lip * select(1.0, 1.55, clods > 0.5);
        let dh = (depth * 1.15 + top) / f32(steps);
        let u0 = in.local.xy - per * top;
        var h_prev = top;
        var f_prev = field(u0, shape, depth, lip, ks, clods, seed) - top;
        for (var i = 1; i <= steps; i++) {
            let hr = top - f32(i) * dh;
            let u = u0 + per * (top - hr);
            let f = field(u, shape, depth, lip, ks, clods, seed) - hr;
            if f >= 0.0 {
                let k = f_prev / (f_prev - f);
                uv = u0 + per * (top - mix(h_prev, hr, k));
                break;
            }
            h_prev = hr;
            f_prev = f;
            uv = u;
        }
    }
    var edge_d = max(abs(uv.x), abs(uv.y));
    if shape > 2.5 {
        edge_d = abs(uv.x);
    }
    let edge = smoothstep(1.0, 0.82, edge_d);
    let h = field(uv, shape, depth, lip, ks, clods, seed);
    let e = 0.012;
    let hx = (field(uv + vec2<f32>(e, 0.0), shape, depth, lip, ks, clods, seed)
        - field(uv - vec2<f32>(e, 0.0), shape, depth, lip, ks, clods, seed)) / (2.0 * e * scl.x);
    let hz = (field(uv + vec2<f32>(0.0, e), shape, depth, lip, ks, clods, seed)
        - field(uv - vec2<f32>(0.0, e), shape, depth, lip, ks, clods, seed)) / (2.0 * e * scl.y);
    let n = normalize(vec3<f32>(-hx, 1.0, -hz));
    var shadow = 1.0;
    if L.y > 0.05 && depth + lip > 0.005 {
        let per_s = L.xz / L.y / scl;
        for (var i = 1; i <= 10; i++) {
            let t = f32(i) * (depth + lip) * 0.11;
            if field(uv + per_s * t, shape, depth, lip, ks, clods, seed) > h + t {
                shadow = 0.45;
                break;
            }
        }
    }
    let ambient = look.reach.w;
    let lit = ambient + max(dot(n, L), 0.0) * shadow;
    let flat0 = ambient + max(L.y, 0.0);
    let pressed = clamp(-h / max(depth, 0.01), 0.0, 1.0);
    var ratio = lit / max(flat0, 1e-3) * (1.0 - 0.28 * pressed);
    let p = uv * ks;
    if shape > 2.5 {
        ratio *= mix(1.0, 0.62, smoothstep(0.02, -0.08, abs(p.x) - 0.34));
    } else if shape > 1.5 {
        ratio *= mix(1.0, 0.45, smoothstep(0.5, 0.05, length(p)));
    } else if shape > 0.5 {
        ratio *= mix(1.0, 0.62, smoothstep(0.02, -0.08, sd_box(p, vec2<f32>(0.34, 0.82), 0.33)));
    }
    var col = vec3<f32>(ratio);
    // Metres from the middle, and metres per unit of the shape.
    let q = uv * scl;
    let unit_m = scl.y / ks.y;
    if paved {
        if shape > 2.5 {
            // A skate's rubber on paving: a dark streak down the line, a fainter scuff round it.
            let along_m = uv.y * scl.y;
            let streak = smoothstep(0.34, 0.16, abs(p.x)) * (0.7 + 0.3 * noise1(along_m * 1.7 + seed * 40.0));
            let scuff = smoothstep(0.75, 0.3, abs(p.x)) * noise1(along_m * 0.6 + p.x * 9.0 + seed * 13.0);
            col *= mix(vec3<f32>(1.0), look.paving.rgb * 1.3, max(streak, scuff * 0.4));
        } else {
            var outline = sd_foot(p) * unit_m;
            var len = 0.62 * unit_m;
            if shape > 1.5 {
                outline = (length(p) - 0.6) * unit_m;
                len = 0.6 * unit_m;
            }
            let inside = smoothstep(0.03, -0.05, outline);
            // A ring of pale concrete dust round it, broken up.
            let dust = smoothstep(-0.05, 0.2, outline) * (1.0 - smoothstep(0.45, 1.05, outline))
                * (0.45 + 0.55 * noise2(q * 2.5 + seed * 9.0));
            col *= 1.0 + (look.paving.w - 1.0) * dust;
            // The crushed sole: darker, dusty specks, broken into slabs with dark seams.
            let speck = step(0.82, hash21(floor(q * 22.0) + seed * 5.0));
            let slab = slabs(q / look.crack.w, seed);
            let seam = 1.0 - smoothstep(0.035, 0.09, slab.x * look.crack.w);
            col *= mix(1.0, look.crack.z * (0.8 + 0.4 * slab.y) * (1.0 + 0.6 * speck), inside);
            col *= mix(vec3<f32>(1.0), look.paving.rgb, seam * inside);
            // The sole's outline, toes and all, crushed into a dark edge.
            let rim = 1.0 - smoothstep(0.0, 0.06, abs(outline - 0.015));
            col *= mix(vec3<f32>(1.0), look.paving.rgb, rim * (0.65 + 0.35 * noise1(atan2(q.y, q.x) * 9.0 + seed * 23.0)));
            // Cracks running out well past the sole.
            col *= mix(vec3<f32>(1.0), look.paving.rgb, cracks(q, len, seed));
        }
    } else {
        // Soil: the pressed bottom turns dark brown (it turns the green-tinted meadow ground brown
        // too); the clods vary; the lip is turned-up soil, a little lighter brown.
        let soilness = smoothstep(0.03, 0.4, pressed);
        let heaped = clamp(h / max(lip, 0.01), 0.0, 1.0);
        let lumps = 0.8 + 0.4 * noise2(q * 4.0 + seed * 7.0);
        let tint = mix(vec3<f32>(1.0), look.soil.rgb, max(soilness, heaped * 0.55));
        col *= tint * (1.0 - look.soil.w * pressed) * mix(1.0, lumps, max(soilness, heaped));
    }
    let k = clamp(fade * edge * in.local.z, 0.0, 1.0);
    return vec4<f32>(mix(vec3<f32>(1.0), col, k), 1.0);
}
