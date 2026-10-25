//! Stand-in textures, generated at start (owner, 2026-10-24: "generate some basic textures for road
//! street, and mech metal, grass trees and stuff"). CLAUDE.md section 10 makes Material Maker graphs
//! the one source of surface looks; these are placeholders until the art side's PBR sets land
//! (openspec change `terrain-materials`), and are replaced, not extended, when they do.
//! Every texture tiles; colours are near white so a material's base colour tints them.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The stand-in textures.
#[derive(Resource, Clone)]
pub struct Textures {
    /// Grass, about 4 m a tile.
    pub grass: Handle<Image>,
    /// Asphalt, 4 m a tile.
    pub asphalt: Handle<Image>,
    /// Concrete paving slabs, 4 m a tile.
    pub concrete: Handle<Image>,
    /// A building facade: one window bay 4 m wide and one storey (3.5 m) tall a tile.
    pub facade: Handle<Image>,
    /// Painted armour plate with panel seams and rivets, one tile a box face.
    pub metal: Handle<Image>,
    /// Bark and leaves (trees).
    pub foliage: Handle<Image>,
    /// Building materials (`building-generation`), each a colour map and a normal map, near white
    /// so the piece's colour tints them; stand-ins for Material Maker graphs (survey W14).
    pub building: BuildingTextures,
}

/// A colour map and its normal map.
#[derive(Clone)]
pub struct Pbr {
    /// Base colour, sRGB.
    pub color: Handle<Image>,
    /// Tangent-space normal, linear.
    pub normal: Handle<Image>,
}

/// The building materials' maps.
#[derive(Clone)]
pub struct BuildingTextures {
    /// Stretcher-bond brick, 215 x 65 mm with 10 mm joints, a 2.4 m tile.
    pub brick: Pbr,
    /// Board-formed concrete, 150 mm boards, tie holes on 600 mm, a 2.4 m tile.
    pub concrete: Pbr,
    /// Roofing felt with gravel, a 2 m tile.
    pub roof: Pbr,
    /// Painted timber boards, a 1 m tile.
    pub wood: Pbr,
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

/// A colour map and a normal map from `f(u, v) -> (colour, height 0..1)` at `n` px a side; the
/// normal comes from the height's slope (`relief` is how steep a full step of height looks).
fn pbr(images: &mut Assets<Image>, n: usize, relief: f32, f: impl Fn(f32, f32) -> ([f32; 3], f32)) -> Pbr {
    let mut col = Vec::with_capacity(n * n * 4);
    let mut h = vec![0.0f32; n * n];
    for j in 0..n {
        for i in 0..n {
            let (c, hh) = f(i as f32 / n as f32, j as f32 / n as f32);
            col.extend(c.iter().map(|x| (x.clamp(0.0, 1.0) * 255.0) as u8));
            col.push(255);
            h[j * n + i] = hh;
        }
    }
    let mut nor = Vec::with_capacity(n * n * 4);
    for j in 0..n {
        for i in 0..n {
            let at = |a: usize, b: usize| h[(b % n) * n + (a % n)];
            let dx = (at(i + n - 1, j) - at(i + 1, j)) * relief;
            let dy = (at(i, j + n - 1) - at(i, j + 1)) * relief;
            let v = Vec3::new(dx, dy, 1.0).normalize();
            nor.extend([v.x, v.y, v.z].iter().map(|x| ((x * 0.5 + 0.5) * 255.0) as u8));
            nor.push(255);
        }
    }
    let make = |data: Vec<u8>, format: TextureFormat| {
        let mut img = Image::new(
            Extent3d {
                width: n as u32,
                height: n as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            format,
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
        img
    };
    Pbr {
        color: images.add(make(col, TextureFormat::Rgba8UnormSrgb)),
        normal: images.add(make(nor, TextureFormat::Rgba8Unorm)),
    }
}

/// The building materials (the mockup's canvases, at their real sizes).
fn building_textures(images: &mut Assets<Image>) -> BuildingTextures {
    // Brick: 32 courses of 75 mm and 9.3 bricks of 225 mm in a 2.4 m tile, half-bond.
    let brick = pbr(images, 512, 6.0, |u, v| {
        let (courses, per) = (32.0, 2.4 / 0.225);
        let row = (v * courses).floor();
        let x = u * per + if row as i32 % 2 == 1 { 0.5 } else { 0.0 };
        let (fx, fy) = (x.fract(), (v * courses).fract());
        let joint = fx < 0.045 || fy < 0.13;
        let id = hash(x.floor() as i32, row as i32, 21);
        if joint {
            let m = 0.78 + 0.08 * noise(u, v, 64, 22);
            ([m, m * 0.97, m * 0.92], 0.0)
        } else {
            let k = 0.70 + 0.30 * id - 0.08 * fbm(u, v, 48, 23) - if id > 0.94 { 0.2 } else { 0.0 };
            ([k, k * 0.96, k * 0.93], 0.8 + 0.2 * noise(u, v, 96, 24))
        }
    });
    // Board-formed concrete: boards every 150 mm, tie holes on a 600 mm grid.
    let concrete = pbr(images, 512, 3.0, |u, v| {
        let board = (v * 16.0).floor();
        let grain = noise(u * 0.4, v * 16.0, 64, 31 + board as u32) * 0.06;
        let seam = (v * 16.0).fract() < 0.04;
        let (hx, hy) = ((u * 4.0).fract() - 0.5, (v * 4.0).fract() - 0.5);
        let hole = hx * hx + hy * hy < 0.0016;
        let k = 0.82 + 0.1 * fbm(u, v, 12, 32) - grain - if seam { 0.12 } else { 0.0 } - if hole { 0.35 } else { 0.0 };
        ([k, k * 0.99, k * 0.96], if seam || hole { 0.0 } else { 0.6 + grain })
    });
    // Roofing felt and gravel.
    let roof = pbr(images, 256, 4.0, |u, v| {
        let g = noise(u, v, 128, 41);
        let k = 0.38 + 0.25 * g + 0.08 * fbm(u, v, 8, 42);
        ([k, k * 0.98, k * 0.95], g)
    });
    // Painted timber boards, 120 mm wide.
    let wood = pbr(images, 256, 4.0, |u, v| {
        let b = (u * 8.0).floor();
        let grain = noise(u * 8.0, v * 2.0, 32, 51 + b as u32);
        let gap = (u * 8.0).fract() < 0.05;
        let k = 0.78 + 0.15 * grain - if gap { 0.35 } else { 0.0 };
        ([k, k * 0.95, k * 0.9], if gap { 0.0 } else { 0.5 + 0.3 * grain })
    });
    BuildingTextures {
        brick,
        concrete,
        roof,
        wood,
    }
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
        let asphalt = image(&mut images, |u, v| {
            let k = 0.8
                + 0.15 * fbm(u, v, 16, 4)
                + if hash((u * 256.0) as i32, (v * 256.0) as i32, 5) > 0.93 {
                    0.15
                } else {
                    0.0
                };
            let crack = (fbm(u, v, 4, 6) - 0.5).abs() < 0.008;
            let k = if crack { k * 0.6 } else { k };
            [k, k, k * 1.02]
        });
        let concrete = image(&mut images, |u, v| {
            // Four slabs a tile, with joints.
            let joint = (u * 2.0).fract() < 0.02 || (v * 2.0).fract() < 0.02;
            let k = 0.82 + 0.12 * fbm(u, v, 12, 7) - if joint { 0.25 } else { 0.0 };
            [k, k * 0.99, k * 0.96]
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
        let building = building_textures(&mut images);
        app.insert_resource(Textures {
            grass,
            asphalt,
            concrete,
            facade,
            metal,
            foliage,
            building,
        });
    }
}
