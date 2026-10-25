//! The island seen from above (`cb-tools map`), for judging the generated world by eye: the ground
//! coloured by biome and shaded by slope, the sea and lakes by depth, rivers by
//! width, waterfalls as red dots, city squares and highways. Reads the plan through the game's
//! own queries (`height_at`, `water_at`), so it draws what the game walks on.

use cb_core::world::{WorldConfig, WorldPlan};
use glam::{Vec2, Vec3};

/// Map colours of the biomes, in `BIOMES` order: snow, rock, montane forest, temperate, savanna,
/// desert, beach, wetland.
const BIOME_COLOURS: [[f32; 3]; 8] = [
    [0.95, 0.96, 0.98],
    [0.5, 0.48, 0.46],
    [0.2, 0.34, 0.22],
    [0.38, 0.54, 0.27],
    [0.66, 0.6, 0.34],
    [0.86, 0.73, 0.5],
    [0.9, 0.84, 0.64],
    [0.3, 0.42, 0.3],
];

/// The plan drawn as a binary PPM of `px` x `px` pixels covering `view` (centre, span in metres; a
/// span of zero is the whole world).
pub fn draw(plan: &WorldPlan, cfg: &WorldConfig, px: usize, view: (Vec2, f32)) -> Vec<u8> {
    let size = if view.1 > 0.0 { view.1 } else { cfg.world_size_m };
    let corner = view.0 - Vec2::splat(size * 0.5);
    let m_per_px = size / px as f32;
    let at = |ix: usize, iz: usize| corner + Vec2::new(ix as f32 + 0.5, iz as f32 + 0.5) * m_per_px;
    let mut img = vec![Vec3::ZERO; px * px];
    for iz in 0..px {
        for ix in 0..px {
            let p = at(ix, iz);
            let h = plan.terrain.height_at(p.x, p.y);
            let e = m_per_px;
            let gx = plan.terrain.height_at(p.x + e, p.y) - h;
            let gz = plan.terrain.height_at(p.x, p.y + e) - h;
            let n = Vec3::new(-gx, e, -gz).normalize();
            let light = n.dot(Vec3::new(-0.5, 0.75, -0.45).normalize()).clamp(0.0, 1.0) * 0.6 + 0.45;
            let slope = Vec2::new(gx, gz).length() / e;
            let w = plan.biomes.weights(p, h, slope);
            let mut c: Vec3 = w.iter().zip(BIOME_COLOURS).map(|(k, c)| Vec3::from(c) * *k).sum();
            c *= light;
            if let Some(w) = plan.waters.water_at(p, h) {
                let deep = (w.depth_m / 30.0).clamp(0.0, 1.0);
                c = Vec3::new(0.32, 0.6, 0.72).lerp(Vec3::new(0.06, 0.2, 0.4), deep);
            }
            img[iz * px + ix] = c;
        }
    }
    let mut dot = |p: Vec2, r: i32, c: Vec3| {
        let cx = ((p.x - corner.x) / m_per_px).floor() as i32;
        let cz = ((p.y - corner.y) / m_per_px).floor() as i32;
        for dz in -r..=r {
            for dx in -r..=r {
                let (x, z) = (cx + dx, cz + dz);
                if x >= 0 && z >= 0 && (x as usize) < px && (z as usize) < px && dx * dx + dz * dz <= r * r {
                    img[z as usize * px + x as usize] = c;
                }
            }
        }
    };
    for h in &plan.highways {
        let steps = (h.length_m() / (m_per_px * 0.5)).ceil() as usize;
        for i in 0..=steps {
            dot(h.point_at(i as f32 / steps as f32), 0, Vec3::new(0.15, 0.13, 0.12));
        }
    }
    for c in &plan.cities {
        let r = c.rect();
        let steps = (r.size().x / m_per_px).ceil() as usize;
        for i in 0..=steps {
            for j in 0..=steps {
                let q = r.min + r.size() * Vec2::new(i as f32, j as f32) / steps as f32;
                dot(q, 0, Vec3::new(0.7, 0.68, 0.66));
            }
        }
    }
    for r in &plan.waters.rivers {
        for (p, hw) in r.path.iter().zip(&r.half_width_m) {
            dot(*p, (*hw / m_per_px).round() as i32, Vec3::new(0.15, 0.4, 0.85));
        }
    }
    for f in &plan.waters.falls {
        dot(plan.waters.rivers[f.river].path[f.lip], 2, Vec3::new(0.95, 0.15, 0.1));
    }
    let mut out = format!("P6\n{px} {px}\n255\n").into_bytes();
    for c in img {
        out.extend(c.to_array().map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8));
    }
    out
}
