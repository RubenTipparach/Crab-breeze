//! Imported models drawn one-sided (openspec change `city-frame` section 7). Blender's glTF exporter writes a material
//! double-sided unless its Backface Culling box is ticked, and the generators left it off: every mech, cockpit, boat
//! and piece of furniture came in with both faces of every triangle drawn and cast into each shadow cascade (1 139 of
//! the 1 723 shipped glTF materials). Their meshes are closed solids, so the back faces are never seen. Each opaque
//! material read from a glTF file is turned to cull its back faces as it arrives; every model sharing it follows.
//! See-through and cut-out materials (glass, decals) keep both sides: they are often single cards seen from behind.
//! `CB_DOUBLE_SIDED=1` leaves the imports as they are (an instrument, to measure it).
//!
//! `tools/build/check_winding.py` keeps them honest: a face wound the wrong way vanishes drawn one-sided, so the
//! models are fixed at their generators (the far hip roofs and the lift's arms were, 2026-10-28).

use bevy::prelude::*;
use bevy::render::render_resource::Face;

/// Turns imported opaque materials one-sided.
pub struct OneSidedPlugin;

impl Plugin for OneSidedPlugin {
    fn build(&self, app: &mut App) {
        if !crate::cli::switched_off("CB_DOUBLE_SIDED") {
            app.add_systems(PreUpdate, one_sided);
        }
    }
}

/// Whether a material read from a glTF file is turned one-sided: opaque and drawn both sides.
fn turns(m: &StandardMaterial) -> bool {
    m.alpha_mode == AlphaMode::Opaque && (m.double_sided || m.cull_mode.is_none())
}

fn one_sided(
    mut added: MessageReader<AssetEvent<StandardMaterial>>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: Local<u32>,
) {
    let before = *count;
    for ev in added.read() {
        let AssetEvent::Added { id } = ev else { continue };
        let from_gltf = assets
            .get_path(*id)
            .is_some_and(|p| matches!(p.path().extension().and_then(|e| e.to_str()), Some("glb" | "gltf")));
        if !from_gltf || !materials.get(*id).is_some_and(turns) {
            continue;
        }
        if let Some(mut m) = materials.get_mut(*id) {
            m.double_sided = false;
            m.cull_mode = Some(Face::Back);
            *count += 1;
        }
    }
    if *count > before {
        info!("one-sided: {} imported materials cull their back faces", *count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opaque_double_sided_import_turns_and_glass_and_decals_do_not() {
        let opaque = StandardMaterial {
            double_sided: true,
            cull_mode: None,
            ..default()
        };
        assert!(turns(&opaque));
        let glass = StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..opaque.clone()
        };
        assert!(!turns(&glass));
        let decal = StandardMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            ..opaque.clone()
        };
        assert!(!turns(&decal));
        assert!(!turns(&StandardMaterial::default()), "one-sided already");
    }
}
