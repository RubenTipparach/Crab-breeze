//! Mining Mike's mesh nuke, beside the particle one in the nuke test scene as its reference (owner, 2026-10-30: "Place
//! mesh nuke side by side of this nuke"; godot-sandbox `scenes/nuke_mushroom.tscn` and
//! `resources/shaders/nuke_cloud.gdshader` at a5c3ec5a): his stem, cap, ring rolling under the cap and dust skirt at
//! his sizes, under a root scaled to the blast, his shader (`nuke_cloud.wesl`) cooling and dissolving them. It plays
//! the same timeline the particle nuke does (`nuke::pose`, `nuke::timeline`, `nuke::heat`), so the two stay in step.
//! Only the test scene sets one off (`MeshNuke`); the game's blasts are the particle nuke.

use crate::nuke::{AUTHORED_HEAD_M, heat, pose, timeline};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

/// Set off the mesh nuke at ground point `.0`.
#[derive(Message, Clone, Copy, Debug)]
pub struct MeshNuke(pub Vec3);

/// The look's uniform; `NukeMeshLook` in `nuke_cloud.wesl`.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
struct NukeMeshLook {
    /// x heat, y dissolve, z heat offset, w dissolve offset.
    drive: Vec4,
    /// x noise cells per mesh unit, y billow (mesh units), z roll (cells a second), w fire brightness.
    shape: Vec4,
    /// xyz toward the sun, w heat variance.
    light: Vec4,
    /// rgb the sun's light over pi, times exposure (`water_light`).
    sun: Vec4,
    /// rgb the sky's light, times exposure.
    amb: Vec4,
}

/// His cloud material, one per part for its heat and dissolve offsets.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct NukeMeshMaterial {
    #[uniform(0)]
    look: NukeMeshLook,
}

const SHADER: &str = "shaders/nuke_cloud.wesl";

impl Material for NukeMeshMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    // Opaque with discard (the holes): no prepass, so the holes are not filled in the depth; no shadows.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
        ])?];
        Ok(())
    }
}

/// Which posed node of his scene an entity is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Node {
    Skirt,
    StemPivot,
    Head,
}

/// A mesh nuke playing: seconds since, its posed nodes and its materials.
#[derive(Component)]
struct MeshCloud {
    t_s: f32,
    nodes: Vec<(Entity, Node)>,
    materials: Vec<Handle<NukeMeshMaterial>>,
}

/// The mesh nuke.
pub struct NukeMeshPlugin;

impl Plugin for NukeMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MeshNuke>()
            .add_plugins(MaterialPlugin::<NukeMeshMaterial>::default())
            .add_game_systems(Update, (spawn, animate).chain().before(PresentSet::Pose));
    }
}

fn spawn(
    mut asked: MessageReader<MeshNuke>,
    mut commands: Commands,
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<NukeMeshMaterial>>,
) {
    let Some(n) = data.0.fortress.cfg.crash.nuke else {
        asked.clear();
        return;
    };
    for MeshNuke(at) in asked.read() {
        let mut look = |heat_offset: f32, dissolve_offset: f32| {
            materials.add(NukeMeshMaterial {
                look: NukeMeshLook {
                    drive: Vec4::new(0.0, 0.0, heat_offset, dissolve_offset),
                    shape: Vec4::new(0.035, 10.0, 0.35, 3.0),
                    ..default()
                },
            })
        };
        let (skirt_m, stem_m, cap_m, ring_m) = (look(0.35, 0.25), look(0.12, 0.1), look(0.0, 0.0), look(0.05, 0.05));
        let skirt = meshes.add(Torus::new(40.0, 140.0).mesh().minor_resolution(24).major_resolution(64));
        let stem = meshes.add(
            ConicalFrustum {
                radius_top: 22.0,
                radius_bottom: 40.0,
                height: 240.0,
            }
            .mesh()
            .resolution(48)
            .segments(24),
        );
        let cap = meshes.add(Sphere::new(90.0).mesh().uv(64, 40));
        let ring = meshes.add(Torus::new(40.0, 115.0).mesh().minor_resolution(24).major_resolution(64));
        let root = commands
            .spawn((
                Name::new("mesh nuke"),
                Transform::from_translation(*at).with_scale(Vec3::splat(n.cap_height_m / AUTHORED_HEAD_M)),
                Visibility::default(),
            ))
            .id();
        let mut node = |name: &str, drawn: Option<(Handle<Mesh>, Handle<NukeMeshMaterial>)>, t: Transform, parent: Entity| {
            let mut e = commands.spawn((Name::new(name.to_string()), t, Visibility::default(), ChildOf(parent)));
            if let Some((mesh, mat)) = drawn {
                e.insert((Mesh3d(mesh), MeshMaterial3d(mat), NotShadowCaster, NoFrustumCulling));
            }
            e.id()
        };
        let skirt_e = node("skirt", Some((skirt, skirt_m.clone())), Transform::from_xyz(0.0, 8.0, 0.0), root);
        let pivot = node("stem pivot", None, Transform::default(), root);
        node("stem", Some((stem, stem_m.clone())), Transform::from_xyz(0.0, 120.0, 0.0), pivot);
        let head = node("head", None, Transform::from_xyz(0.0, 30.0, 0.0), root);
        node(
            "cap",
            Some((cap, cap_m.clone())),
            Transform::from_xyz(0.0, 10.0, 0.0).with_scale(Vec3::new(1.0, 110.0 / 180.0, 1.0)),
            head,
        );
        node("cap ring", Some((ring, ring_m.clone())), Transform::from_xyz(0.0, -25.0, 0.0), head);
        commands.entity(root).insert(MeshCloud {
            t_s: 0.0,
            nodes: vec![(skirt_e, Node::Skirt), (pivot, Node::StemPivot), (head, Node::Head)],
            materials: vec![skirt_m, stem_m, cap_m, ring_m],
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn animate(
    mut commands: Commands,
    mut clouds: Query<(Entity, &mut MeshCloud)>,
    mut transforms: Query<&mut Transform>,
    mut materials: ResMut<Assets<NukeMeshMaterial>>,
    data: Res<Data>,
    time: Res<Time>,
    ambient: Res<GlobalAmbientLight>,
    sun: Query<(&DirectionalLight, &GlobalTransform, &crate::day_night::SkyBody)>,
    camera: Query<Option<&bevy::camera::Exposure>, With<crate::pilot::PilotCamera>>,
) {
    let Some(n) = data.0.fortress.cfg.crash.nuke else { return };
    let exposure = camera.iter().next().flatten().copied().unwrap_or_default().exposure();
    let key = crate::day_night::key_light(sun.iter());
    let light = crate::water_render::water_light(key.as_ref().map(|(l, t)| (l, t)), &ambient, exposure);
    let sun_pi = light.sun / std::f32::consts::PI;
    let amb = (light.body - sun_pi).max(Vec3::ZERO);
    for (e, mut c) in &mut clouds {
        c.t_s += time.delta_secs();
        if c.t_s > n.cloud_s {
            commands.entity(e).despawn();
            continue;
        }
        let u = timeline(c.t_s, n.cloud_s);
        let p = pose(u);
        for (node, kind) in &c.nodes {
            if let Ok(mut t) = transforms.get_mut(*node) {
                match kind {
                    Node::Skirt => t.scale = Vec3::new(p.skirt.x, p.skirt.y, p.skirt.x),
                    Node::StemPivot => t.scale = Vec3::new(p.stem.x, p.stem.y, p.stem.x),
                    Node::Head => {
                        t.translation.y = p.head_y;
                        t.scale = Vec3::splat(p.head_scale);
                    }
                }
            }
        }
        let h = heat(u);
        for m in &c.materials {
            if let Some(mut m) = materials.get_mut(m) {
                m.look.drive.x = h;
                m.look.drive.y = p.dissolve;
                m.look.light = light.sun_dir.extend(0.35);
                m.look.sun = sun_pi.extend(0.0);
                m.look.amb = amb.extend(0.0);
            }
        }
    }
}
