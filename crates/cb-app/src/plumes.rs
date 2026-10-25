//! Exhaust plumes (openspec change `rocket-plumes`; owner, 2026-10-25: "rocket plumes arent convincing right now"): one
//! flame-shaped mesh and two materials (a rocket's, a jet's with shock diamonds), drawn additively with
//! `assets/shaders/plume.wesl`. `fx.rs` hangs a plume behind every rocket in flight from its motor pool; `mech.rs` hangs
//! one under each jet nozzle, shown while the jets fire. A plume's length and width are its transform's scale, so every
//! plume of a kind shares its material and batches. Presentation only.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

/// The plume shader, loaded from `assets/`.
const SHADER: &str = "shaders/plume.wesl";

/// `PlumeLook` in `plume.wesl`, field for field.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct PlumeUniform {
    /// rgb at the nozzle, w overall brightness.
    pub core: Vec4,
    /// rgb a third of the way along, w the noise's scroll in lengths a second.
    pub mid: Vec4,
    /// rgb at the far end, w the noise's scale around the flame.
    pub edge: Vec4,
    /// x shock diamonds along the length, y their strength, z the share of the length they run.
    pub shock: Vec4,
}

/// An exhaust flame, drawn additively.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PlumeMaterial {
    /// The look.
    #[uniform(0)]
    pub look: PlumeUniform,
}

impl Material for PlumeMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Seen from inside as well as out (a plume pointing at the eye).
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// The plume mesh and the two looks.
#[derive(Resource, Clone)]
pub struct PlumeAssets {
    /// The flame, 1 m long along +Z from the nozzle at the origin, 1 m across at its widest.
    pub mesh: Handle<Mesh>,
    /// A rocket motor's.
    pub rocket: Handle<PlumeMaterial>,
    /// A mech's jet's, with shock diamonds.
    pub jet: Handle<PlumeMaterial>,
}

/// The plumes' assets.
pub struct PlumesPlugin;

impl Plugin for PlumesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PlumeMaterial>::default())
            .add_systems(PreStartup, setup);
    }
}

/// The flame's radius at `t` along it (0 the nozzle, 1 the far end), for a widest radius of 0.5: it swells out of the
/// nozzle, then narrows to a point.
pub fn profile(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    0.5 * (0.35 + 0.65 * (std::f32::consts::FRAC_PI_2 * (t * 3.0).min(1.0)).sin()) * (1.0 - t).powf(0.55)
}

/// The flame mesh: a lathe of `rings` rings of `sides` vertices along +Z, uv.x around, uv.y along.
pub fn flame_mesh(rings: usize, sides: usize) -> Mesh {
    let (mut pos, mut nrm, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for i in 0..=rings {
        let t = i as f32 / rings as f32;
        let r = profile(t);
        let slope = (profile((t + 0.01).min(1.0)) - profile((t - 0.01).max(0.0))) / 0.02;
        for j in 0..=sides {
            let a = j as f32 / sides as f32 * std::f32::consts::TAU;
            let (s, c) = a.sin_cos();
            pos.push([r * c, r * s, t]);
            nrm.push(Vec3::new(c, s, -slope).normalize().to_array());
            uv.push([j as f32 / sides as f32, t]);
        }
    }
    let w = (sides + 1) as u32;
    for i in 0..rings as u32 {
        for j in 0..sides as u32 {
            let (a, b) = (i * w + j, (i + 1) * w + j);
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(idx))
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<PlumeMaterial>>) {
    let rgb = |r: f32, g: f32, b: f32, w: f32| Vec4::new(r, g, b, w);
    let rocket = materials.add(PlumeMaterial {
        look: PlumeUniform {
            core: rgb(1.0, 0.95, 0.85, 14.0),
            mid: rgb(1.0, 0.72, 0.3, 3.0),
            edge: rgb(1.0, 0.32, 0.07, 9.0),
            shock: rgb(0.0, 0.0, 0.4, 0.0),
        },
    });
    let jet = materials.add(PlumeMaterial {
        look: PlumeUniform {
            core: rgb(0.85, 0.9, 1.0, 10.0),
            mid: rgb(1.0, 0.75, 0.45, 2.2),
            edge: rgb(1.0, 0.4, 0.12, 7.0),
            shock: rgb(5.0, 0.9, 0.45, 0.0),
        },
    });
    commands.insert_resource(PlumeAssets {
        mesh: meshes.add(flame_mesh(16, 16)),
        rocket,
        jet,
    });
}

/// A plume's transform: from the nozzle at `at`, pointing along `dir`, `length_m` long and `width_m` across.
pub fn placed(at: Vec3, dir: Vec3, length_m: f32, width_m: f32) -> Transform {
    Transform::from_translation(at)
        .with_rotation(Quat::from_rotation_arc(Vec3::Z, dir.normalize_or(Vec3::NEG_Z)))
        .with_scale(Vec3::new(width_m, width_m, length_m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plume_swells_from_the_nozzle_and_ends_in_a_point() {
        assert!(profile(0.0) > 0.1, "open at the nozzle");
        assert!(profile(0.3) > profile(0.0), "swells");
        assert!(profile(1.0) < 1e-4, "ends in a point");
        assert!((0..=100).all(|i| profile(i as f32 / 100.0) <= 0.5 + 1e-6));
    }

    #[test]
    fn the_plume_shader_compiles_against_bevy_pbr_and_its_uniform_matches() {
        use crate::shader_check::{compile, struct_size, with_bevy};
        let Some(resolver) = with_bevy() else {
            eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
            return;
        };
        let flags = [("VERTEX_POSITIONS", true), ("VERTEX_NORMALS", true), ("VERTEX_UVS_A", true)];
        let module = compile(&resolver, "plume", &flags);
        assert_eq!(u64::from(struct_size(&module, "PlumeLook")), PlumeUniform::min_size().get());
    }

    #[test]
    fn a_plume_points_along_its_direction_at_its_length() {
        let t = placed(Vec3::ZERO, Vec3::NEG_Y, 6.0, 1.5);
        let tip = t.transform_point(Vec3::Z);
        assert!((tip - Vec3::new(0.0, -6.0, 0.0)).length() < 1e-4, "{tip}");
    }
}
