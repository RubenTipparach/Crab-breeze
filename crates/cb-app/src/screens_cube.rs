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
use crate::state::GameApp;
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
    /// The face drawn the frame before, to copy into the cube this frame (none: nothing to copy).
    fresh: Option<usize>,
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
        app.add_after_world_spawn(setup)
            .add_game_systems(Update, turn_faces.after(PresentSet::Camera));
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
    // Each face is drawn into, then copied out of into the cube.
    let faces: [Handle<Image>; 6] = std::array::from_fn(|_| {
        let mut face = Image::new_target_texture(px, px, format, None);
        face.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        images.add(face)
    });
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
                // Near surroundings and the sky light the cabin; far geometry would only cost the
                // CPU a visit to every mesh in the world for each face.
                far: 400.0,
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
    commands.insert_resource(ScreensCube {
        faces,
        cube,
        px,
        fresh: None,
    });
}

/// One face at a time, `6 * screens_cube_hz` faces a second, drawn from where the pilot's eye is
/// now; no face on the frames between.
fn turn_faces(
    mut next: Local<usize>,
    mut due_s: Local<f32>,
    mut drawn: Local<Option<usize>>,
    mut cube: ResMut<ScreensCube>,
    time: Res<Time>,
    data: Res<Data>,
    eye: Single<&GlobalTransform, With<PilotCamera>>,
    mut faces: Query<(&FaceCamera, &mut Camera, &mut Transform)>,
) {
    // The face drawn last frame is in its image now: that one, and only it, goes into the cube.
    cube.fresh = drawn.take();
    *due_s -= time.delta_secs();
    let draw = *due_s <= 0.0;
    if draw {
        *due_s += 1.0 / (6.0 * data.0.cockpit_light.screens_cube_hz);
        *due_s = due_s.max(0.0);
        *next = (*next + 1) % 6;
        *drawn = Some(*next);
    }
    let at = eye.translation();
    for (f, mut cam, mut t) in &mut faces {
        cam.is_active = draw && f.0 == *next;
        let (dir, up) = FACES[f.0];
        *t = Transform::from_translation(at).looking_to(dir, up);
    }
}

fn extract(mut commands: Commands, cube: Extract<Option<Res<ScreensCube>>>) {
    if let Some(c) = cube.as_deref() {
        commands.insert_resource(c.clone());
    }
}

/// Copies the face drawn last frame into its layer of the cube; nothing on frames with none.
fn copy_faces(cube: Option<Res<ScreensCube>>, images: Res<RenderAssets<GpuImage>>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    let Some(c) = cube else { return };
    let Some(i) = c.fresh else { return };
    let (Some(dst), Some(src)) = (images.get(&c.cube), images.get(&c.faces[i])) else {
        return;
    };
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("screens cube faces"),
    });
    {
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
