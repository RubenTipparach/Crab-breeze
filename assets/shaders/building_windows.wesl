// The buildings' lit windows at night (openspec change `day-night-cycle`; owner, 2026-10-28:
// "buildings also need some sort of cell lighting. If the building is damaged all of its lights turn
// off"). The one rule both building shaders draw by: each window is a cell of its facade one storey
// high and `w.w` wide, lit when its own hash falls under the share lit now (`w.x`, from
// `cb_core::day_night::WindowLights::lit_share`); mostly warm rooms, some office tubes, a few
// televisions. The near glass (`building_surface`) is lit across its whole cell; a far or massing wall
// (`building_array`) only in a window inside each cell (`inset`), the wall between left dark.
// `w`: x the share lit now, y a lit window's brightness cd/m2, z a storey's height m, w a cell's width m.

fn window_hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.547);
}

// A lit window's light at a fragment of a facade at `world` facing `normal`, cd/m2, or none.
fn window_light(world: vec3<f32>, normal: vec3<f32>, seconds: f32, w: vec4<f32>, inset: bool) -> vec3<f32> {
    if (w.x <= 0.0 || abs(normal.y) > 0.5) {
        return vec3<f32>(0.0);
    }
    let along_z = abs(normal.x) > abs(normal.z);
    let t = select(world.x, world.z, along_z);
    let side = select(sign(normal.z), sign(normal.x), along_z);
    let col = t / max(w.w, 0.1);
    let row = world.y / max(w.z, 0.5);
    if (inset) {
        // The window inside its cell: 70 % of its width, from a fifth to four fifths of its height.
        let f = vec2<f32>(fract(col), fract(row));
        if (abs(f.x - 0.5) > 0.35 || f.y < 0.2 || f.y > 0.8) {
            return vec3<f32>(0.0);
        }
    }
    let cell = vec3<f32>(floor(col), floor(row), side + select(0.0, 7.0, along_z));
    let h = window_hash(cell);
    if (h >= w.x) {
        return vec3<f32>(0.0);
    }
    let kind = window_hash(cell + vec3<f32>(17.0, 3.0, 5.0));
    var colour = vec3<f32>(1.0, 0.72, 0.42);
    if (kind > 0.95) {
        colour = vec3<f32>(0.45, 0.6, 1.0) * (0.7 + 0.3 * sin(seconds * 9.0 + h * 40.0));
    } else if (kind > 0.7) {
        colour = vec3<f32>(0.85, 0.95, 1.0);
    }
    return colour * w.y * (0.6 + 0.4 * window_hash(cell + vec3<f32>(5.0, 11.0, 2.0)));
}
