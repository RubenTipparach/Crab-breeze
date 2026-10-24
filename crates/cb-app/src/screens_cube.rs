//! The screens' cube map (openspec change `cockpit-lighting` task 2.1; owner, 2026-10-24: "it should
//! be lit up by the screens perhaps a cube map made from the screens"). Six small cameras on the
//! world layer, one drawn a frame in turn, render the world round the pilot's eye into six face
//! images; a render-world system copies them into one cube texture each frame; Bevy's
//! `GeneratedEnvironmentMapLight` on the cockpit pass filters that into the cabin's light. A fireball
//! ahead washes the panels from the front; a cloud turns the cabin grey. The cockpit pass lights
//! nothing from the world directly (it sees only layer 1), so this is the one way the outside reaches
//! the cabin, as it is through the screens.

use crate::pilot::{CockpitCamera, PilotCamera};
use crate::sim::{Data, PresentSet};
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    CommandEncoderDescriptor, Extent3d, Origin3d, TexelCopyTextureInfo, TextureAspect, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::texture::GpuImage;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

/// The six faces and the cube they are copied into.
#[derive(Resource, Clone)]
struct ScreensCube {
    faces: [Handle<Image>; 6],
    cube: Handle<Image>,
    px: u32,
}

/// A face camera, by face index (+X, -X, +Y, -Y, +Z, -Z).
#[derive(Component)]
struct FaceCamera(usize);

/// Draws the screens' cube map and lights the cabin with it.
pub struct ScreensCubePlugin;

impl Plugin for ScreensCubePlugin {
    fn build(&self, app: &mut App) {
        if app.world().resource::<Data>().0.cockpit_light.screens_cube_px == 0 {
            return;
        }
        app.add_systems(PostStartup, setup)
            .add_systems(Update, turn_faces.after(PresentSet::Camera));
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .add_systems(ExtractSchedule, extract)
                .add_systems(Render, copy_faces.in_set(RenderSystems::PrepareResources));
        }
    }
}

/// Where each face looks and which way is up on it.
const FACES: [(Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::Y),
    (Vec3::NEG_X, Vec3::Y),
    (Vec3::Y, Vec3::NEG_Z),
    (Vec3::NEG_Y, Vec3::Z),
    (Vec3::Z, Vec3::Y),
    (Vec3::NEG_Z, Vec3::Y),
];

fn setup(mut commands: Commands, data: Res<Data>, mut images: ResMut<Assets<Image>>, cockpit: Single<Entity, With<CockpitCamera>>) {
    let light = &data.0.cockpit_light;
    let px = light.screens_cube_px;
    let format = TextureFormat::Rgba16Float;
    let faces: [Handle<Image>; 6] = std::array::from_fn(|_| images.add(Image::new_target_texture(px, px, format, None)));
    let mut cube = Image::new_fill(
        Extent3d {
            width: px,
            height: px,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        &[0; 8],
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    cube.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;
    cube.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    let cube = images.add(cube);
    for (i, face) in faces.iter().enumerate() {
        commands.spawn((
            Name::new(format!("screens cube face {i}")),
            FaceCamera(i),
            Camera3d::default(),
            Camera {
                order: -10 - i as isize,
                is_active: false,
                ..default()
            },
            RenderTarget::Image(face.clone().into()),
            Projection::from(PerspectiveProjection {
                fov: std::f32::consts::FRAC_PI_2,
                aspect_ratio: 1.0,
                near: 0.5,
                far: 3000.0,
                ..default()
            }),
            Transform::default(),
        ));
    }
    commands.entity(*cockpit).insert(GeneratedEnvironmentMapLight {
        environment_map: cube.clone(),
        intensity: light.screens_cube_cd_m2,
        rotation: Quat::IDENTITY,
        affects_lightmapped_mesh_diffuse: true,
    });
    commands.insert_resource(ScreensCube { faces, cube, px });
}

/// One face a frame, in turn, drawn from where the pilot's eye is now.
fn turn_faces(
    mut frame: Local<usize>,
    eye: Single<&GlobalTransform, With<PilotCamera>>,
    mut faces: Query<(&FaceCamera, &mut Camera, &mut Transform)>,
) {
    *frame = (*frame + 1) % 6;
    let at = eye.translation();
    for (f, mut cam, mut t) in &mut faces {
        cam.is_active = f.0 == *frame;
        let (dir, up) = FACES[f.0];
        *t = Transform::from_translation(at).looking_to(dir, up);
    }
}

fn extract(mut commands: Commands, cube: Extract<Option<Res<ScreensCube>>>) {
    if let Some(c) = cube.as_deref() {
        commands.insert_resource(c.clone());
    }
}

/// Copies the six faces into the cube's six layers (the newest face is a frame old by then).
fn copy_faces(cube: Option<Res<ScreensCube>>, images: Res<RenderAssets<GpuImage>>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    let Some(c) = cube else { return };
    let Some(dst) = images.get(&c.cube) else { return };
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("screens cube faces"),
    });
    for (i, face) in c.faces.iter().enumerate() {
        let Some(src) = images.get(face) else { continue };
        encoder.copy_texture_to_texture(
            src.texture.as_image_copy(),
            TexelCopyTextureInfo {
                texture: &dst.texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: i as u32 },
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: c.px,
                height: c.px,
                depth_or_array_layers: 1,
            },
        );
    }
    queue.submit([encoder.finish()]);
}
