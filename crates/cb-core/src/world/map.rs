//! The island as one map image (openspec change `waypoint-nav`, design section 1): land shaded by height and lit from
//! the north-west, water darkened by its depth, the highways, the cities' streets, and the air bases', the airport's
//! and the seaport's footprints, drawn once when the world loads. Pure, so it is tested headless and the game only
//! uploads what it returns; the colours are `data/hud/nav.ron`'s, after the MAP page the owner approved.

use super::WorldPlan;
use crate::nav::NavConfig;
use glam::Vec2;

/// The world square the image covers: its least corner (x, z) and its side, metres.
pub fn extent(plan: &WorldPlan) -> (Vec2, f32) {
    let t = &plan.terrain;
    (Vec2::splat(t.origin_m), t.cell_m * t.n as f32)
}

/// The island painted into a `px` by `px` RGBA image, row by row from the least z (north at the top).
pub fn paint(plan: &WorldPlan, cfg: &NavConfig, px: u32) -> Vec<[u8; 4]> {
    let n = px.max(1) as usize;
    let (lo, side) = extent(plan);
    let step = side / n as f32;
    let at = |i: usize, j: usize| lo + Vec2::new((i as f32 + 0.5) * step, (j as f32 + 0.5) * step);
    // One height per pixel, so the hill shade reads its neighbours without asking the terrain again.
    let heights: Vec<f32> = (0..n * n)
        .map(|k| {
            let p = at(k % n, k / n);
            plan.terrain.height_at(p.x, p.y)
        })
        .collect();
    let lerp = |a: [u8; 3], b: [u8; 3], t: f32| {
        let t = t.clamp(0.0, 1.0);
        [0, 1, 2].map(|c| f32::from(a[c]) + (f32::from(b[c]) - f32::from(a[c])) * t)
    };
    let mut out = vec![[0u8; 4]; n * n];
    for j in 0..n {
        for i in 0..n {
            let k = j * n + i;
            let g = heights[k];
            let water = plan.waters.water_at(at(i, j), g).filter(|w| w.depth_m > 0.0);
            let rgb = if let Some(w) = water {
                lerp(cfg.water_shallow_rgb, cfg.water_deep_rgb, w.depth_m / cfg.water_deep_m)
            } else {
                // Lit from the north-west: a slope rising toward +x and +z is in its own shade.
                let hx = heights[j * n + (i + 1).min(n - 1)] - heights[j * n + i.saturating_sub(1)];
                let hz = heights[(j + 1).min(n - 1) * n + i] - heights[j.saturating_sub(1) * n + i];
                let shade = (1.0 - (hx + hz) / (step * 4.4)).clamp(0.45, 1.25);
                let sea = plan.waters.sea_level_m;
                lerp(cfg.land_low_rgb, cfg.land_high_rgb, (g - sea) / cfg.land_high_m).map(|c| c * shade)
            };
            out[k] = [rgb[0], rgb[1], rgb[2], 255.0].map(|c| c.round().clamp(0.0, 255.0) as u8);
        }
    }
    let to_px = |p: Vec2| (p - lo) / step;
    let mut put = |x: i64, y: i64, c: [u8; 3]| {
        if x >= 0 && y >= 0 && (x as usize) < n && (y as usize) < n {
            out[y as usize * n + x as usize] = [c[0], c[1], c[2], 255];
        }
    };
    // The cities' streets, filled.
    for city in &plan.cities {
        for s in &city.streets {
            let (a, b) = (to_px(s.rect.min), to_px(s.rect.max));
            for y in a.y.floor() as i64..=b.y.floor() as i64 {
                for x in a.x.floor() as i64..=b.x.floor() as i64 {
                    put(x, y, cfg.street_rgb);
                }
            }
        }
    }
    // The highways, `highway_px` wide along their centre lines.
    let r = (cfg.highway_px * 0.5).max(0.5);
    for h in &plan.highways {
        for w in h.path.windows(2) {
            let (a, b) = (to_px(w[0]), to_px(w[1]));
            let steps = (a.distance(b).ceil() as usize).max(1);
            for s in 0..=steps {
                let c = a.lerp(b, s as f32 / steps as f32);
                let span = r.ceil() as i64;
                for dy in -span..=span {
                    for dx in -span..=span {
                        if Vec2::new(dx as f32, dy as f32).length() <= r {
                            put(c.x.floor() as i64 + dx, c.y.floor() as i64 + dy, cfg.highway_rgb);
                        }
                    }
                }
            }
        }
    }
    // The air bases and the sites, outlined.
    let mut outline = |min: Vec2, max: Vec2, c: [u8; 3]| {
        let (a, b) = (to_px(min), to_px(max));
        let (x0, y0, x1, y1) = (a.x.floor() as i64, a.y.floor() as i64, b.x.floor() as i64, b.y.floor() as i64);
        for x in x0..=x1 {
            put(x, y0, c);
            put(x, y1, c);
        }
        for y in y0..=y1 {
            put(x0, y, c);
            put(x1, y, c);
        }
    };
    for b in &plan.bases {
        outline(b.centre - b.half_m, b.centre + b.half_m, cfg.base_rgb);
    }
    for s in &plan.sites {
        let f = s.footprint();
        outline(f.min, f.max, cfg.site_rgb);
    }
    out
}

/// Where a world point (x, z) falls on the image, 0..1 a side.
pub fn to_uv(plan: &WorldPlan, p: Vec2) -> Vec2 {
    let (lo, side) = extent(plan);
    (p - lo) / side
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{WorldConfig, generate};

    fn plan() -> WorldPlan {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let cfg: WorldConfig = crate::data::load_ron(&root.join("world/world.ron")).unwrap();
        generate(42, &cfg)
    }

    #[test]
    fn the_map_shows_land_at_the_start_sea_at_the_edge_and_a_highway_on_its_line() {
        let plan = plan();
        let cfg = NavConfig::default();
        let px = 256u32;
        let img = paint(&plan, &cfg, px);
        assert_eq!(img.len(), (px * px) as usize);
        let pixel = |p: Vec2| {
            let uv = to_uv(&plan, p) * px as f32;
            img[uv.y as usize * px as usize + uv.x as usize]
        };
        // The start stands on land: green over blue.
        let s = pixel(plan.start.mech_xz);
        assert!(s[1] > s[2], "the start reads as land: {s:?}");
        // The island's corner is open sea: blue over green, the deep water's colour.
        let (lo, _) = extent(&plan);
        let c = pixel(lo + Vec2::splat(10.0));
        assert_eq!(&c[..3], &cfg.water_deep_rgb, "the corner is deep sea");
        // A highway's middle point is drawn in the highway's colour.
        let h = &plan.highways[0];
        let mid = h.path[h.path.len() / 2];
        assert_eq!(&pixel(mid)[..3], &cfg.highway_rgb);
        // Every pixel is opaque.
        assert!(img.iter().all(|p| p[3] == 255));
    }
}
