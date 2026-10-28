//! Tiling PBR texture sets read from `assets/textures/` with a full mip chain built at load (Bevy
//! 0.20 makes none for a PNG or JPEG, and a texture tiled every few metres without one shimmers in
//! the distance), sampled with repeat and anisotropic filtering. Used by the ground's CC0 grass
//! (`assets/textures/ground/grass001/SOURCE.md`), the building surfaces and the roads
//! (`assets/textures/road/`).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::path::Path;

/// The levels of a mip chain for a `w` by `h` image, down to one pixel.
pub fn mip_levels(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

/// An RGBA8 image and every mip level under it, each half the last (box filtered), as one buffer
/// in the order wgpu wants (level 0 first).
pub fn mip_chain(base: image::RgbaImage) -> (Vec<u8>, u32) {
    let levels = mip_levels(base.width(), base.height());
    let mut data = base.as_raw().clone();
    let mut level = base;
    for _ in 1..levels {
        let w = (level.width() / 2).max(1);
        let h = (level.height() / 2).max(1);
        level = image::imageops::resize(&level, w, h, image::imageops::FilterType::Triangle);
        data.extend_from_slice(level.as_raw());
    }
    (data, levels)
}

/// Loads `path` (relative to the assets directory) as a tiling texture with its mip chain: sRGB
/// for a colour map, linear for a normal or packed ORM map. `None`, with a warning, if it cannot
/// be read.
pub fn load_tiling(assets_root: &Path, path: &str, srgb: bool) -> Option<Image> {
    let full = assets_root.join(path);
    let decoded = match image::open(&full) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            warn!("textures: cannot read {}: {e}", full.display());
            return None;
        }
    };
    let (w, h) = decoded.dimensions();
    let (data, levels) = mip_chain(decoded);
    // Uninitialised, then the whole mip chain as its data: `Image::new` checks the data against the top level
    // alone, and a debug build stopped on the empty data it was given here.
    let mut img = Image::new_uninit(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(data);
    img.texture_descriptor.mip_level_count = levels;
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    Some(img)
}

/// A folder's packed set (`albedo.jpg`, `normal_gl.png`, `orm.png`, under `assets/`) as a material whose base
/// colour is white, so a mesh's vertex colour tints it. A map that cannot be read is left off, with a warning,
/// and the material falls back to `roughness` and no metal.
pub fn set_material(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    folder: &str,
    roughness: f32,
) -> Handle<StandardMaterial> {
    let root = crate::cli::repo_root().join("assets");
    let mut map = |file: &str, srgb: bool| load_tiling(&root, &format!("{folder}/{file}"), srgb).map(|i| images.add(i));
    let orm = map("orm.png", false);
    materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: map("albedo.jpg", true),
        normal_map_texture: map("normal_gl.png", false),
        occlusion_texture: orm.clone(),
        perceptual_roughness: if orm.is_some() { 1.0 } else { roughness },
        metallic: if orm.is_some() { 1.0 } else { 0.0 },
        metallic_roughness_texture: orm,
        ..default()
    })
}

/// Turns an sRGB RGBA8 colour map (every mip level) into a neutral detail map: each pixel's
/// luminance over the top level's mean, so the mean is 0.5 in linear light and the hue is gone.
/// The ground's biome colour (per vertex, twice as bright) then gives the hue, and the photo only
/// its grain (`island-biomes`: sand and rock no longer take the grass's green).
pub fn neutral_detail(img: &mut Image) {
    let Some(data) = img.data.as_mut() else { return };
    let top = (img.texture_descriptor.size.width * img.texture_descriptor.size.height * 4) as usize;
    let lin = |c: u8| (f32::from(c) / 255.0).powf(2.2);
    let lum = |p: &[u8; 4]| 0.2126 * lin(p[0]) + 0.7152 * lin(p[1]) + 0.0722 * lin(p[2]);
    let n = (top / 4).max(1) as f32;
    let mean = data[..top.min(data.len())].as_chunks::<4>().0.iter().map(lum).sum::<f32>() / n;
    if mean <= 0.0 {
        return;
    }
    for p in data.as_chunks_mut::<4>().0 {
        let v = (0.5 * lum(p) / mean).clamp(0.0, 1.0);
        let g = (v.powf(1.0 / 2.2) * 255.0).round() as u8;
        p[0] = g;
        p[1] = g;
        p[2] = g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_neutral_detail_map_is_grey_with_its_mean_at_half() {
        let mut img = Image::new(
            Extent3d {
                width: 2,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![40, 160, 20, 255, 90, 200, 60, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        neutral_detail(&mut img);
        let d = img.data.unwrap();
        assert_eq!((d[0], d[1]), (d[1], d[2]), "grey");
        let lin = |c: u8| (f32::from(c) / 255.0).powf(2.2);
        let mean = (lin(d[0]) + lin(d[4])) / 2.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
        assert!(d[4] > d[0], "the brighter pixel stays brighter");
    }

    #[test]
    fn a_tiling_texture_loads_with_its_whole_mip_chain() {
        let dir = std::env::temp_dir().join(format!("cb-pbr-set-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 200, 30, 255]))
            .save(dir.join("t.png"))
            .unwrap();
        let img = load_tiling(&dir, "t.png", true).expect("loads");
        assert_eq!(img.texture_descriptor.mip_level_count, 4);
        assert_eq!(img.data.as_ref().map(Vec::len), Some((64 + 16 + 4 + 1) * 4));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_2k_texture_has_twelve_levels_down_to_one_pixel() {
        assert_eq!(mip_levels(2048, 2048), 12);
        assert_eq!(mip_levels(1, 1), 1);
        let (data, levels) = mip_chain(image::RgbaImage::new(8, 4));
        assert_eq!(levels, 4);
        // 8x4 + 4x2 + 2x1 + 1x1 pixels, four bytes each.
        assert_eq!(data.len(), (32 + 8 + 2 + 1) * 4);
    }
}
