//! Tiling PBR texture sets read from `assets/textures/` with a full mip chain built at load (Bevy
//! 0.20 makes none for a PNG or JPEG, and a texture tiled every few metres without one shimmers in
//! the distance), sampled with repeat and anisotropic filtering. Used by the ground's CC0 grass
//! (`assets/textures/ground/grass001/SOURCE.md`).

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

#[cfg(test)]
mod tests {
    use super::*;

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
