//! Stand-in textures, generated at start (owner, 2026-10-24: "generate some basic textures for road
//! street, and mech metal, grass trees and stuff"). CLAUDE.md section 10 makes Material Maker graphs
//! the one source of surface looks; these are placeholders until the art side's PBR sets land
//! (openspec change `terrain-materials`), and are replaced, not extended, when they do.
//! Every texture tiles; colours are near white so a material's base colour tints them. The roads and
//! pavements left this file for CC0 sets (`road-surfaces`, `assets/textures/road/`), the buildings'
//! brick, concrete, roof and timber for Material Maker sets (`building-materials`).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The stand-in textures.
#[derive(Resource, Clone)]
pub struct Textures {
    /// Grass, about 4 m a tile.
    pub grass: Handle<Image>,
    /// A building facade: one window bay 4 m wide and one storey (3.5 m) tall a tile.
    pub facade: Handle<Image>,
    /// Painted armour plate with panel seams and rivets, one tile a box face.
    pub metal: Handle<Image>,
    /// Bark and leaves (trees).
    pub foliage: Handle<Image>,
}

const N: usize = 256;

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Tiling value noise at `cells` cells across the texture.
fn noise(u: f32, v: f32, cells: i32, seed: u32) -> f32 {
    let (x, y) = (u * cells as f32, v * cells as f32);
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let h = |i: i32, j: i32| hash(i.rem_euclid(cells), j.rem_euclid(cells), seed);
    let a = h(xi, yi) + (h(xi + 1, yi) - h(xi, yi)) * s(fx);
    let b = h(xi, yi + 1) + (h(xi + 1, yi + 1) - h(xi, yi + 1)) * s(fx);
    a + (b - a) * s(fy)
}

fn fbm(u: f32, v: f32, base: i32, seed: u32) -> f32 {
    (noise(u, v, base, seed) * 0.5 + noise(u, v, base * 2, seed + 1) * 0.3 + noise(u, v, base * 4, seed + 2) * 0.2).clamp(0.0, 1.0)
}

fn image(images: &mut Assets<Image>, f: impl Fn(f32, f32) -> [f32; 3]) -> Handle<Image> {
    let mut data = Vec::with_capacity(N * N * 4);
    for j in 0..N {
        for i in 0..N {
            let c = f(i as f32 / N as f32, j as f32 / N as f32);
            data.extend(c.iter().map(|x| (x.clamp(0.0, 1.0) * 255.0) as u8));
            data.push(255);
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    images.add(img)
}

/// Generates the textures.
pub struct TexturesPlugin;

impl Plugin for TexturesPlugin {
    fn build(&self, app: &mut App) {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let grass = image(&mut images, |u, v| {
            let g = fbm(u, v, 8, 1);
            let blade = noise(u * 7.0, v, 64, 2);
            let dry = fbm(u, v, 3, 3);
            let k = 0.75 + 0.35 * g - 0.15 * blade;
            [k * (0.95 + 0.2 * dry), k, k * (0.85 - 0.1 * dry)]
        });
        let facade = image(&mut images, |u, v| {
            // One window bay: wall, a sill band, a window with a frame.
            let window = (0.18..0.82).contains(&u) && (0.25..0.8).contains(&v);
            let frame = window
                && ((0.18..0.21).contains(&u) || (0.79..0.82).contains(&u) || (0.25..0.28).contains(&v) || (0.49..0.52).contains(&u));
            let wall = 0.85 + 0.1 * fbm(u, v, 8, 8);
            if frame {
                [0.35, 0.35, 0.37]
            } else if window {
                let sky = 0.25 + 0.25 * (1.0 - v) + 0.1 * noise(u, v, 4, 9);
                [sky * 0.8, sky * 0.9, sky * 1.15]
            } else if (0.82..0.86).contains(&v) {
                [wall * 0.75, wall * 0.75, wall * 0.75]
            } else {
                [wall, wall * 0.98, wall * 0.95]
            }
        });
        let metal = image(&mut images, |u, v| {
            // Painted plate: two panels a face, seams, rivets along them, wear at the edges.
            let seam = !(0.012..=0.988).contains(&u) || !(0.012..=0.988).contains(&v) || (0.495..0.505).contains(&v);
            let near_seam = |t: f32| (t - (t * 2.0).round() / 2.0).abs() < 0.03;
            let rivet =
                (near_seam(u) || near_seam(v)) && ((u * 16.0).fract() - 0.5).abs() < 0.12 && ((v * 16.0).fract() - 0.5).abs() < 0.12;
            let wear = fbm(u, v, 10, 10);
            let k = 0.82 + 0.1 * wear - if seam { 0.45 } else { 0.0 } + if rivet { 0.12 } else { 0.0 };
            let chipped = wear > 0.78 && (!(0.08..=0.92).contains(&u) || !(0.08..=0.92).contains(&v));
            if chipped { [0.6, 0.6, 0.62] } else { [k, k, k] }
        });
        let foliage = image(&mut images, |u, v| {
            let leaf = fbm(u, v, 24, 11);
            let k = 0.6 + 0.5 * leaf;
            [k, k, k]
        });
        app.insert_resource(Textures {
            grass,
            facade,
            metal,
            foliage,
        });
    }
}
