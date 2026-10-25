//! The water shaders compile, and the Rust uniforms match them (openspec change
//! `water-rendering`). The shaders are compiled the way Bevy 0.20 compiles them
//! (`crate::shader_check`), so a broken shader fails `cargo test`, not the game. The surface
//! shader imports Bevy's own `bevy_pbr` modules, which are read from Bevy's sources as Cargo has
//! them.

use super::*;
use crate::shader_check::{compile, compile_with, game_only, struct_size, with_bevy};

#[test]
fn every_shared_water_function_compiles() {
    // Whole, so a function only the surface uses is checked even where Bevy's sources are absent.
    compile_with(&game_only(), "water_common", &[], false);
}

#[test]
fn the_post_shader_compiles_with_and_without_msaa() {
    for multisampled in [false, true] {
        compile(&game_only(), "water_post", &[("MULTISAMPLED", multisampled)]);
    }
}

#[test]
fn the_post_uniform_matches_the_wesl_struct() {
    let module = compile(&game_only(), "water_post", &[("MULTISAMPLED", true)]);
    assert_eq!(
        u64::from(struct_size(&module, "WaterPost")),
        WaterPostUniform::min_size().get(),
        "the Rust uniform must be the WESL struct's size"
    );
}

#[test]
fn the_surface_shader_compiles_against_bevy_pbr() {
    let Some(resolver) = with_bevy() else {
        eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
        return;
    };
    // The main pass's transmissive pipeline as the pilot camera has it (HDR, MSAA, depth prepass,
    // transmission, distance fog), with and without a river's vertex colour, and the fallbacks.
    let base = [
        ("VERTEX_POSITIONS", true),
        ("VERTEX_NORMALS", true),
        ("VERTEX_UVS_A", true),
        ("CLUSTERED_FORWARD_STORAGE_BUFFER", true),
    ];
    for (msaa, prepass, transmission, fog, colors) in [
        (true, true, true, true, false),
        (true, true, true, true, true),
        (false, true, true, true, true),
        (false, false, false, false, false),
    ] {
        let mut flags = base.to_vec();
        flags.extend([
            ("MULTISAMPLED", msaa),
            ("DEPTH_PREPASS", prepass),
            ("VIEW_TRANSMISSION_TEXTURE", transmission),
            ("DISTANCE_FOG", fog),
            ("VERTEX_COLORS", colors),
        ]);
        let module = compile(&resolver, "water_surface", &flags);
        assert_eq!(
            u64::from(struct_size(&module, "WaterSurface")),
            WaterSurfaceUniform::min_size().get(),
            "the Rust uniform must be the WESL struct's size"
        );
    }
}

#[test]
fn the_water_surface_is_never_displaced() {
    // Bevy's own mesh vertex shader: the surface is drawn where the mesh is, at the simulation's
    // `surface_m`; the swell and ripples are in the fragment's normal only.
    assert!(matches!(WaterMaterial::vertex_shader(), ShaderRef::Default));
    assert!(matches!(WaterMaterial::fragment_shader(), ShaderRef::Path(_)));
}

#[test]
fn a_dry_eye_runs_no_water_pass() {
    let dry = WaterEye::default();
    assert_eq!(passes_needed(&dry, 0.0, false), (false, false));
}

#[test]
fn ground_fog_runs_the_compose_pass_on_a_dry_eye() {
    let dry = WaterEye::default();
    assert_eq!(passes_needed(&dry, 0.0, true), (true, false));
}

#[test]
fn under_water_composes_but_puts_no_drops_on_the_glass() {
    let under = WaterEye {
        submersion: 1.0,
        ..default()
    };
    assert_eq!(passes_needed(&under, 0.0, false), (true, false));
}

#[test]
fn a_wet_glass_runs_both_passes() {
    let surfacing = WaterEye { drips: 0.6, ..default() };
    assert_eq!(passes_needed(&surfacing, 0.0, false), (true, true));
}

#[test]
fn the_water_body_is_lit_like_the_ground() {
    let ambient = GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 100.0,
        ..default()
    };
    // No sun: only the ambient light, times exposure.
    let light = water_light(None, &ambient, 0.5);
    assert!((light.body - Vec3::splat(50.0)).length() < 1e-3, "{:?}", light.body);
    // A sun straight overhead adds its light over pi.
    let sun = DirectionalLight {
        color: Color::WHITE,
        illuminance: std::f32::consts::PI,
        ..default()
    };
    let overhead = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)));
    let lit = water_light(Some((&sun, &overhead)), &ambient, 1.0);
    assert!(lit.sun_dir.y > 0.999, "the sun is overhead: {:?}", lit.sun_dir);
    assert!((lit.body - Vec3::splat(101.0)).length() < 1e-3, "{:?}", lit.body);
}

#[test]
fn only_a_wet_eye_sees_water_from_below() {
    // The surface shader takes "seen from below" only when the eye is in the water (`deep.w`): a
    // dry eye in the lowlands looking up at a river on the range sees it from above, not as murk.
    let look = WaterLook::default();
    let light = WaterLight::default();
    let dry = WaterSurfaceUniform::new(&look, 0.0, &light, Vec3::new(0.0, 20.0, 0.0), 0.0, 0.0);
    let under = WaterSurfaceUniform::new(&look, 0.0, &light, Vec3::new(0.0, -3.0, 0.0), 0.0, 1.0);
    assert_eq!(dry.deep.w, 0.0);
    assert_eq!(under.deep.w, 1.0);
    let source = std::fs::read_to_string(crate::shader_check::shader_dir().join("water_surface.wesl")).unwrap();
    assert!(
        source.contains("let below = in_water &&"),
        "the shader gates `below` on the eye being wet"
    );
}
