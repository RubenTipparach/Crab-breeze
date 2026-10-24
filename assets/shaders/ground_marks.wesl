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
// it is), so the ground's own texture and light show through. Crab Breeze adds the surfaces: soil
// is tinted dark and keeps its lip; paving gets radial cracks, a crushed rim, dust and, under a
// skate, rubber streaks instead.

import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{view, lights};

// Matches `MarksUniform` in `crates/cb-app/src/ground_marks.rs`.
struct MarksLook {
    reach: vec4<f32>,   // draw within m, lift m, pull toward the eye per m of distance, ambient share
    soil: vec4<f32>,    // rgb pressed soil tint (multiplies); w darkening at the bottom of a print
    paving: vec4<f32>,  // rgb crack colour (multiplies); w dust brightening
    crack: vec4<f32>,   // crack width m, crack reach (share of the print's length), scuff darkening, spare
}

// One mark, ten rows (`cb_core::ground_marks::ROW_FLOATS`).
struct Mark {
    centre: vec4<f32>,  // xyz middle on the ground, w yaw (local +z along (sin, 0, cos))
    size: vec4<f32>,    // quad half size across m, along m, depth m, lip m
    kind: vec4<f32>,    // shape (0 foot, 1 groove, 2 crater, 3 trail), surface (0 soil, 1 paving), fade, seed
    // The ground's height at each of the 5 by 5 grid's vertices, across then along, relative to the
    // middle; 25 used, 3 spare.
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
    // The view direction and the sun in the mark's frame (across, up, along), unscaled.
    @location(2) to_eye: vec3<f32>,
    @location(3) sun: vec3<f32>,
}

const TAU: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;

// The sampled ground height at grid vertex `k` (row-major, across then along) of slot `slot`.
fn grid_height(slot: u32, k: u32) -> f32 {
    return marks[slot].h[k / 4u][k % 4u];
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

// A mech's sole: wider at the toe (+y), rounded.
fn sd_foot(p: vec2<f32>) -> f32 {
    let w = mix(0.40, 0.56, smoothstep(-0.55, 0.25, p.y));
    return sd_box(p, vec2<f32>(w, 0.62), 0.2);
}

// The mark's height field on the quad (Mining Mike's `H`): a pit `depth` deep inside the shape, a
// lip `lip` high just outside it; tread bars in a footprint; a ragged bowl for a crater.
fn field(p: vec2<f32>, shape: f32, depth: f32, lip: f32) -> f32 {
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
    let pit = -depth * smoothstep(0.1, -0.12, d) * (1.0 - 0.3 * tread);
    let g = (d - 0.12) / 0.11;
    let rim = lip * exp(-g * g);
    return pit + rim;
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

// Paving under a foot: cracks running out from the print, some long, some short, wandering; and a
// crushed rim round the sole. 0 nothing, 1 a crack. `q` in metres from the middle, `len` the print's
// half length in metres.
fn cracks(q: vec2<f32>, len: f32, seed: f32, outline: f32) -> f32 {
    let r = length(q);
    let a = atan2(q.y, q.x);
    let n = 9.0;
    let wander = a + 0.22 * (noise1(r * 2.2 + seed * 37.0) - 0.5) + 0.08 * sin(r * 7.0 + seed * 11.0);
    let x = wander * n / TAU + seed * 5.0;
    let cell = floor(x);
    let to_line_m = (0.5 - abs(fract(x) - 0.5)) * TAU * r / n;
    let reach = len * look.crack.y * (0.45 + 0.75 * hash21(vec2<f32>(cell, seed * 91.0)));
    let w = look.crack.x * (1.0 - 0.6 * smoothstep(0.0, reach, r));
    let radial = (1.0 - smoothstep(w * 0.4, w, to_line_m))
        * smoothstep(len * 0.35, len * 0.6, r)
        * (1.0 - smoothstep(reach * 0.7, reach, r))
        * step(0.3, hash21(vec2<f32>(cell + 17.0, seed * 53.0)));
    // The crushed rim: a broken line hugging the sole's outline.
    let broken = step(0.35, noise1(a * 6.0 + seed * 23.0));
    let rim = (1.0 - smoothstep(0.0, 0.03, abs(outline - 0.02))) * broken;
    return max(radial, rim);
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
    // The sun is read in the fragment stage: Bevy's view lights are bound for fragments only.
    out.sun = vec3<f32>(0.0, 1.0, 0.0);
    return out;
}

@fragment
fn fragment(in: MarkFragment) -> @location(0) vec4<f32> {
    let m = marks[in.slot];
    var sun_dir = vec3<f32>(0.35, 0.8, 0.45);
    if lights.n_directional_lights > 0u {
        sun_dir = lights.directional_lights[0].direction_to_light;
    }
    let sun = transpose(frame(in.slot, m)) * normalize(sun_dir);
    let shape = m.kind.x;
    let paved = m.kind.y > 0.5;
    let fade = m.kind.z;
    let seed = m.kind.w;
    let depth = m.size.z;
    let lip = m.size.w;
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
        let top = lip;
        let dh = (depth + lip) / f32(steps);
        let u0 = in.local.xy - per * top;
        var h_prev = top;
        var f_prev = field(u0, shape, depth, lip) - top;
        for (var i = 1; i <= steps; i++) {
            let hr = top - f32(i) * dh;
            let u = u0 + per * (top - hr);
            let f = field(u, shape, depth, lip) - hr;
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
    let h = field(uv, shape, depth, lip);
    let e = 0.012;
    let hx = (field(uv + vec2<f32>(e, 0.0), shape, depth, lip) - field(uv - vec2<f32>(e, 0.0), shape, depth, lip)) / (2.0 * e * scl.x);
    let hz = (field(uv + vec2<f32>(0.0, e), shape, depth, lip) - field(uv - vec2<f32>(0.0, e), shape, depth, lip)) / (2.0 * e * scl.y);
    let n = normalize(vec3<f32>(-hx, 1.0, -hz));
    let L = normalize(sun);
    var shadow = 1.0;
    if L.y > 0.05 && depth + lip > 0.005 {
        let per_s = L.xz / L.y / scl;
        for (var i = 1; i <= 10; i++) {
            let t = f32(i) * (depth + lip) * 0.11;
            if field(uv + per_s * t, shape, depth, lip) > h + t {
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
    if shape > 2.5 {
        ratio *= mix(1.0, 0.62, smoothstep(0.02, -0.08, abs(uv.x) - 0.34));
    } else if shape > 1.5 {
        ratio *= mix(1.0, 0.45, smoothstep(0.5, 0.05, length(uv)));
    } else if shape > 0.5 {
        ratio *= mix(1.0, 0.62, smoothstep(0.02, -0.08, sd_box(uv, vec2<f32>(0.34, 0.82), 0.33)));
    }
    var col = vec3<f32>(ratio);
    if paved {
        // Paving: no soil tint. A scuff inside the sole, cracks and a crushed rim round a print or
        // a crater, rubber streaks along a skate's line, and pale dust round it all.
        let q = uv * scl;
        if shape > 2.5 {
            let along_m = uv.y * scl.y;
            let streak = smoothstep(0.34, 0.18, abs(uv.x)) * (0.55 + 0.45 * noise1(along_m * 1.7 + seed * 40.0));
            let scuff = smoothstep(0.6, 0.3, abs(uv.x)) * noise1(along_m * 0.6 + uv.x * 9.0 + seed * 13.0);
            col *= mix(vec3<f32>(1.0), look.paving.rgb, max(streak * 0.9, scuff * 0.35));
        } else {
            var outline = sd_foot(uv) * scl.y;
            var len = 0.62 * scl.y;
            if shape > 1.5 {
                outline = (length(uv) - 0.6) * scl.y;
                len = 0.6 * scl.y;
            }
            let inside = smoothstep(0.02, -0.06, outline);
            col *= mix(1.0, look.crack.z, inside * (0.6 + 0.4 * noise1(q.y * 3.0 + q.x * 5.0 + seed * 7.0)));
            col *= mix(vec3<f32>(1.0), look.paving.rgb, cracks(q, len, seed, outline));
            let dust = smoothstep(0.0, 0.25, outline) * (1.0 - smoothstep(0.3, 1.1, outline));
            col *= 1.0 + (look.paving.w - 1.0) * dust * (0.5 + 0.5 * noise1(atan2(q.y, q.x) * 4.0 + seed * 3.0));
        }
    } else {
        // Soil: the pressed bottom turns dark and wet-looking brown; the lip keeps the ground's
        // colour, lit by its slope.
        let tint = mix(vec3<f32>(1.0), look.soil.rgb, smoothstep(0.05, 0.6, pressed));
        col *= tint * (1.0 - look.soil.w * pressed * pressed);
    }
    let k = clamp(fade * edge * in.local.z, 0.0, 1.0);
    return vec4<f32>(mix(vec3<f32>(1.0), col, k), 1.0);
}
