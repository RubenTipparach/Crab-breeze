//! The opaque building looks as layers of three texture arrays (openspec change
//! `building-texture-array`): colour, normal and packed occlusion-roughness-metal, one layer per
//! look, so a building's opaque parts are one mesh drawn with one material. Built at startup from
//! the same maps the separate materials used: the CC0 surface sets (`assets/textures/building/`),
//! the generated skin maps (`textures`), flat layers for the plain trim and facade panels. Every
//! layer is `SIZE` square with a full mip chain.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
use image::RgbaImage;

/// Every layer's side, pixels (the CC0 sets' size; generated maps are resized to it).
pub const SIZE: u32 = 1024;

/// Where one layer's maps come from.
pub enum Source {
    /// A packed CC0 set: `albedo.jpg`, `normal_gl.png`, `orm.png` in this folder under `assets/`.
    Set(String),
    /// Generated maps (`textures`): colour, a normal map or none (flat), and one roughness.
    Generated {
        /// The colour map.
        color: Handle<Image>,
        /// The normal map, if it has one.
        normal: Option<Handle<Image>>,
        /// Its perceptual roughness.
        roughness: f32,
    },
    /// White, flat, one roughness (plain trim).
    Flat {
        /// Its perceptual roughness.
        roughness: f32,
    },
}

/// The three arrays.
pub struct Arrays {
    /// Colour, sRGB.
    pub albedo: Handle<Image>,
    /// Tangent-space normal, OpenGL convention, linear.
    pub normal: Handle<Image>,
    /// R occlusion, G roughness, B metal, linear.
    pub orm: Handle<Image>,
}

/// A flat map of one colour at `SIZE`.
fn flat(rgba: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(SIZE, SIZE, image::Rgba(rgba))
}

/// An image at `SIZE` from an `Assets<Image>` entry (RGBA8, any size), or `fallback`.
fn from_asset(images: &Assets<Image>, h: &Handle<Image>, fallback: [u8; 4]) -> RgbaImage {
    images
        .get(h)
        .and_then(|i| {
            let (w, ht) = (i.width(), i.height());
            let data = i.data.as_ref()?;
            let raw = RgbaImage::from_raw(w, ht, data[..(w * ht * 4) as usize].to_vec())?;
            Some(image::imageops::resize(&raw, SIZE, SIZE, image::imageops::FilterType::Triangle))
        })
        .unwrap_or_else(|| flat(fallback))
}

/// An image at `SIZE` from a file under `root`, or `fallback`.
fn from_file(root: &std::path::Path, path: &str, fallback: [u8; 4]) -> RgbaImage {
    match image::open(root.join(path)) {
        Ok(i) => {
            let i = i.to_rgba8();
            if i.dimensions() == (SIZE, SIZE) {
                i
            } else {
                image::imageops::resize(&i, SIZE, SIZE, image::imageops::FilterType::Triangle)
            }
        }
        Err(e) => {
            warn!("building arrays: cannot read {}: {e}", root.join(path).display());
            flat(fallback)
        }
    }
}

/// One array image from its layers, each with its mip chain (layer-major, as wgpu wants).
pub fn array(layers: Vec<RgbaImage>, format: TextureFormat) -> Image {
    let n = layers.len() as u32;
    let (width, height) = layers.first().map_or((SIZE, SIZE), |l| l.dimensions());
    let mut data = Vec::new();
    let mut levels = 1;
    for l in layers {
        let (d, lv) = crate::pbr_set::mip_chain(l);
        data.extend(d);
        levels = lv;
    }
    let mut img = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: n,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(data);
    img.texture_descriptor.mip_level_count = levels;
    img.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    img
}

/// The ORM value of a plain layer: no occlusion, `roughness`, dielectric.
fn orm_of(roughness: f32) -> [u8; 4] {
    [255, (roughness.clamp(0.0, 1.0) * 255.0).round() as u8, 0, 255]
}

/// Builds the arrays from `sources`, in layer order, and keeps the colour layers on the CPU for whoever renders
/// from them next (the far facades, `far_facades`).
pub fn build_keeping(images: &mut Assets<Image>, sources: &[Source]) -> (Arrays, Vec<RgbaImage>) {
    let root = crate::cli::repo_root().join("assets");
    const FLAT_NORMAL: [u8; 4] = [128, 128, 255, 255];
    let (mut albedo, mut normal, mut orm) = (Vec::new(), Vec::new(), Vec::new());
    for s in sources {
        match s {
            Source::Set(dir) => {
                albedo.push(from_file(&root, &format!("{dir}/albedo.jpg"), [204; 4]));
                normal.push(from_file(&root, &format!("{dir}/normal_gl.png"), FLAT_NORMAL));
                orm.push(from_file(&root, &format!("{dir}/orm.png"), orm_of(0.85)));
            }
            Source::Generated {
                color,
                normal: n,
                roughness,
            } => {
                albedo.push(from_asset(images, color, [255; 4]));
                normal.push(n.as_ref().map_or_else(|| flat(FLAT_NORMAL), |n| from_asset(images, n, FLAT_NORMAL)));
                orm.push(flat(orm_of(*roughness)));
            }
            Source::Flat { roughness } => {
                albedo.push(flat([255; 4]));
                normal.push(flat(FLAT_NORMAL));
                orm.push(flat(orm_of(*roughness)));
            }
        }
    }
    let kept = albedo.clone();
    (
        Arrays {
            albedo: images.add(array(albedo, TextureFormat::Rgba8UnormSrgb)),
            normal: images.add(array(normal, TextureFormat::Rgba8Unorm)),
            orm: images.add(array(orm, TextureFormat::Rgba8Unorm)),
        },
        kept,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_array_holds_every_layer_with_its_mips() {
        let img = array(vec![flat([1, 2, 3, 4]), flat([5, 6, 7, 8])], TextureFormat::Rgba8Unorm);
        assert_eq!(img.texture_descriptor.size.depth_or_array_layers, 2);
        let levels = crate::pbr_set::mip_levels(SIZE, SIZE);
        assert_eq!(img.texture_descriptor.mip_level_count, levels);
        let per_layer: usize = (0..levels).map(|l| ((SIZE >> l).max(1) as usize).pow(2) * 4).sum();
        let data = img.data.as_ref().unwrap();
        assert_eq!(data.len(), per_layer * 2);
        assert_eq!(
            &data[per_layer..per_layer + 4],
            &[5, 6, 7, 8],
            "layer 1 starts after layer 0's whole chain"
        );
    }

    #[test]
    fn a_plain_layer_keeps_its_roughness() {
        assert_eq!(orm_of(0.6), [255, 153, 0, 255]);
    }
}
