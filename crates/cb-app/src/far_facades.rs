//! The far facades' maps (openspec change `building-type-lods`): every facade kind and main colour's
//! ground and typical storey tiles, and the roof, as layers of three small texture arrays (colour,
//! normal, occlusion-roughness-metal). Rendered once at startup from `cb_core::world::far_facade`'s
//! texels with the very maps the near skin uses (the colour layers `building_arrays` just built), so
//! a far facade cannot disagree with its near one, and nothing is baked to disk to go stale.
//! The far material that wears them and the far meshes are in `buildings` and `world`.

use crate::building_arrays::{Arrays, array};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use cb_core::world::dressing::{DressingConfig, Mat};
use cb_core::world::far_facade::{self, Band, TILE_PX, Texel};
use image::{Rgba, RgbaImage};

/// The roof's tint over its map (the dressing's roof tone).
const ROOF_TONE: f32 = 0.9;
/// Far glass is darker than its near pane: it shows the sky's reflection, not the room behind.
const GLASS_SHADE: f32 = 0.6;

fn to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// A material's perceptual roughness and metalness far off.
fn roughness_metal(mat: Mat) -> (f32, f32) {
    match mat {
        Mat::Brick => (0.9, 0.0),
        Mat::Concrete => (0.85, 0.0),
        Mat::Roof => (0.9, 0.0),
        Mat::Wood => (0.75, 0.0),
        Mat::Trim => (0.55, 0.0),
        Mat::Glass => (0.1, 0.0),
    }
}

/// The colour of a texel: its material's map where it is on the facade, times its colour.
fn colour_of(t: &Texel, skins: &[RgbaImage], skin_layer: &impl Fn(Mat) -> (u32, f32)) -> [u8; 4] {
    if t.mat == Mat::Glass {
        let c = t.color.map(|v| to_srgb(v * GLASS_SHADE));
        return [c[0], c[1], c[2], 255];
    }
    let (layer, tile_m) = skin_layer(t.mat);
    let base = skins.get(layer as usize).map_or([1.0; 3], |img| {
        let (w, h) = img.dimensions();
        let u = (t.at[0] / tile_m).rem_euclid(1.0);
        let v = (1.0 - t.at[1] / tile_m).rem_euclid(1.0);
        let p = img.get_pixel(((u * w as f32) as u32).min(w - 1), ((v * h as f32) as u32).min(h - 1));
        [to_linear(p[0]), to_linear(p[1]), to_linear(p[2])]
    });
    [
        to_srgb(base[0] * t.color[0]),
        to_srgb(base[1] * t.color[1]),
        to_srgb(base[2] * t.color[2]),
        255,
    ]
}

/// One tile's three maps from its texels (`TILE_PX` square, top row first).
fn tile_maps(texels: &[Option<Texel>], skins: &[RgbaImage], skin_layer: &impl Fn(Mat) -> (u32, f32)) -> [RgbaImage; 3] {
    let n = TILE_PX;
    let px_m = far_facade::SAMPLE_CELL_M.x / n as f32;
    let depth = |c: i32, r: i32| -> f32 {
        let (c, r) = (c.clamp(0, n as i32 - 1) as u32, r.clamp(0, n as i32 - 1) as u32);
        texels[(r * n + c) as usize].map_or(0.0, |t| t.depth_m)
    };
    let (mut albedo, mut normal, mut orm) = (RgbaImage::new(n, n), RgbaImage::new(n, n), RgbaImage::new(n, n));
    for r in 0..n {
        for c in 0..n {
            let t = texels[(r * n + c) as usize];
            let (ci, ri) = (c as i32, r as i32);
            // relief from the depth steps: height is minus depth; rows run down, so up is -row
            let dh_dx = -(depth(ci + 1, ri) - depth(ci - 1, ri)) / (2.0 * px_m);
            let dh_dy = -(depth(ci, ri - 1) - depth(ci, ri + 1)) / (2.0 * px_m);
            let nv = Vec3::new(-dh_dx.clamp(-3.0, 3.0) * 0.5, -dh_dy.clamp(-3.0, 3.0) * 0.5, 1.0).normalize();
            // occlusion where a pixel sits deeper than its neighbourhood (a recessed window, a reveal)
            let mut sum = 0.0;
            for dr in -2..=2 {
                for dc in -2..=2 {
                    sum += depth(ci + dc, ri + dr);
                }
            }
            let ao = 1.0 - ((depth(ci, ri) - sum / 25.0) * 2.0).clamp(0.0, 0.5);
            let (rough, metal) = t.map_or((0.8, 0.0), |t| roughness_metal(t.mat));
            let a = t.map_or([128, 128, 128, 255], |t| colour_of(&t, skins, skin_layer));
            albedo.put_pixel(c, r, Rgba(a));
            normal.put_pixel(
                c,
                r,
                Rgba([
                    ((nv.x * 0.5 + 0.5) * 255.0).round() as u8,
                    ((nv.y * 0.5 + 0.5) * 255.0).round() as u8,
                    ((nv.z * 0.5 + 0.5) * 255.0).round() as u8,
                    255,
                ]),
            );
            orm.put_pixel(c, r, Rgba([(ao * 255.0) as u8, (rough * 255.0) as u8, (metal * 255.0) as u8, 255]));
        }
    }
    [albedo, normal, orm]
}

/// Every far tile and the roof, in `far_facade` layer order. `skins` are the near skin's colour
/// layers (`building_arrays::build_keeping`), and `skin_layer` gives a material's layer in them and
/// its tile size, m.
pub fn build(images: &mut Assets<Image>, skins: &[RgbaImage], skin_layer: impl Fn(Mat) -> (u32, f32), cfg: &DressingConfig) -> Arrays {
    let (mut albedo, mut normal, mut orm) = (Vec::new(), Vec::new(), Vec::new());
    for v in far_facade::variants() {
        for band in [Band::Ground, Band::Typical] {
            let [a, n, o] = tile_maps(&far_facade::tile(v, band, cfg), skins, &skin_layer);
            albedo.push(a);
            normal.push(n);
            orm.push(o);
        }
    }
    // the roof: the near roof map, small, in its tone
    let (roof, _) = skin_layer(Mat::Roof);
    let small = skins.get(roof as usize).map_or_else(
        || RgbaImage::from_pixel(TILE_PX, TILE_PX, Rgba([120, 120, 120, 255])),
        |img| image::imageops::resize(img, TILE_PX, TILE_PX, image::imageops::FilterType::Triangle),
    );
    let mut roof_img = small.clone();
    for p in roof_img.pixels_mut() {
        for ch in 0..3 {
            p[ch] = to_srgb(to_linear(p[ch]) * ROOF_TONE);
        }
    }
    albedo.push(roof_img);
    normal.push(RgbaImage::from_pixel(TILE_PX, TILE_PX, Rgba([128, 128, 255, 255])));
    orm.push(RgbaImage::from_pixel(TILE_PX, TILE_PX, Rgba([255, 230, 0, 255])));
    debug_assert_eq!(albedo.len() as u32, far_facade::layer_count());
    // A contact sheet (a measurement instrument, CLAUDE.md section 4): `CB_FAR_SHEET=<file.png>`
    // writes every variant's ground and typical tiles side by side, one row per kind.
    if let Ok(path) = std::env::var("CB_FAR_SHEET") {
        write_sheet(&albedo, &path);
    }
    Arrays {
        albedo: images.add(array(albedo, TextureFormat::Rgba8UnormSrgb)),
        normal: images.add(array(normal, TextureFormat::Rgba8Unorm)),
        orm: images.add(array(orm, TextureFormat::Rgba8Unorm)),
    }
}

/// Writes the colour tiles as a sheet: per kind a row, per colour a ground and a typical tile, 4x.
fn write_sheet(albedo: &[RgbaImage], path: &str) {
    let all = far_facade::variants();
    let cols = cb_core::world::far_facade::FacadeKind::ALL
        .iter()
        .map(|k| all.iter().filter(|v| v.kind == *k).count())
        .max()
        .unwrap_or(1) as u32;
    let (t, z) = (TILE_PX, 4);
    let mut sheet = RgbaImage::from_pixel(cols * 2 * t * z + cols * 8, 5 * t * z + 40, Rgba([30, 30, 30, 255]));
    for (i, v) in all.iter().enumerate() {
        let row = far_facade::FacadeKind::ALL.iter().position(|k| *k == v.kind).unwrap_or(0) as u32;
        for band in 0..2u32 {
            let tile = image::imageops::resize(&albedo[i * 2 + band as usize], t * z, t * z, image::imageops::FilterType::Nearest);
            let x = (v.colour as u32 * 2 + band) * t * z + v.colour as u32 * 8;
            image::imageops::overlay(&mut sheet, &tile, x as i64, (row * (t * z + 8)) as i64);
        }
    }
    match sheet.save(path) {
        Ok(()) => info!("far facades: contact sheet {path}"),
        Err(e) => warn!("far facades: cannot write {path}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_conversions_round_trip() {
        for c in [0u8, 1, 50, 128, 200, 255] {
            assert_eq!(to_srgb(to_linear(c)), c);
        }
    }

    #[test]
    fn every_far_layer_is_built() {
        let mut images = Assets::<Image>::default();
        let skins = vec![RgbaImage::from_pixel(8, 8, Rgba([200, 150, 120, 255])); 16];
        let a = build(&mut images, &skins, |_| (0, 2.4), &DressingConfig::default());
        let img = images.get(&a.albedo).unwrap();
        assert_eq!(img.texture_descriptor.size.depth_or_array_layers, far_facade::layer_count());
        assert_eq!(img.texture_descriptor.size.width, TILE_PX);
    }
}
