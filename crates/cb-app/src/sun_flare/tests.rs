//! The flare's shader compiles the way Bevy 0.20 compiles it, its uniform matches the Rust one,
//! and the CPU decides when the flare is drawn (openspec change `sun-flare`).

use super::*;
use crate::shader_check::{compile, game_only, struct_size};
use cb_core::sun_flare::SunLook;

fn look() -> FlareLook {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/sun_flare.ron");
    cb_core::data::load_ron::<SunLook>(&path).unwrap().flare
}

#[test]
fn the_flare_shader_compiles_with_and_without_msaa() {
    for multisampled in [false, true] {
        let module = compile(&game_only(), "sun_flare", &[("MULTISAMPLED", multisampled)]);
        for entry in ["flare_vertex", "flare_fragment", "taps_vertex", "taps_fragment"] {
            assert!(module.entry_points.iter().any(|e| e.name == entry), "no entry point {entry}");
        }
    }
}

#[test]
fn the_flare_uniform_matches_the_wesl_struct() {
    let module = compile(&game_only(), "sun_flare", &[("MULTISAMPLED", false)]);
    assert_eq!(
        u64::from(struct_size(&module, "SunFlare")),
        SunFlareUniform::min_size().get(),
        "the Rust uniform must be the WESL struct's size"
    );
}

#[test]
fn the_shader_has_room_for_every_tap_and_ghost() {
    let source = std::fs::read_to_string(crate::shader_check::shader_dir().join("sun_flare.wesl")).unwrap();
    assert!(source.contains(&format!("array<vec4<f32>, {MAX_GHOSTS}>")));
    assert!(source.contains(&format!("array<vec4<f32>, {OCCLUSION_TAPS}>")));
    assert!(source.contains(&format!("const TAPS: u32 = {OCCLUSION_TAPS}u;")));
    // The sky test is the core's.
    assert!(source.contains("const SKY_DEPTH: f32 = 1.0e-7;"));
    assert_eq!(cb_core::sun_flare::SKY_DEPTH, 1.0e-7);
}

fn eye(toward_sun: Vec3, medium: f32) -> FlareView {
    FlareView {
        role: Role::Eye,
        medium,
        light: Vec3::splat(0.9),
        toward_sun,
    }
}

fn lens() -> Mat4 {
    let f = 1.0 / 37.5f32.to_radians().tan();
    Mat4::from_cols(
        Vec4::new(f * 9.0 / 16.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, f, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 0.0, -1.0),
        Vec4::new(0.0, 0.0, 0.05, 0.0),
    )
}

#[test]
fn a_sun_ahead_is_drawn_and_one_behind_is_not() {
    let (ndc, visibility) = placed(&eye(Vec3::new(0.1, 0.2, -1.0), 1.0), lens(), 0.3).unwrap();
    assert!(ndc.length() < 0.5 && visibility == 1.0);
    assert!(placed(&eye(Vec3::new(0.1, 0.2, 1.0), 1.0), lens(), 0.3).is_none());
    // Far off to the side: past the margin.
    assert!(placed(&eye(Vec3::new(1.0, 0.0, -0.2), 1.0), lens(), 0.3).is_none());
}

#[test]
fn under_water_or_in_cloud_nothing_is_drawn_but_the_cockpit_still_tests() {
    assert!(placed(&eye(Vec3::NEG_Z, 0.0), lens(), 0.3).is_none());
    let cockpit = FlareView {
        role: Role::Cockpit,
        light: Vec3::ZERO,
        ..eye(Vec3::NEG_Z, 1.0)
    };
    assert!(placed(&cockpit, lens(), 0.3).is_some());
    // A dark sun draws nothing on the eyes.
    let dark = FlareView {
        light: Vec3::ZERO,
        ..eye(Vec3::NEG_Z, 1.0)
    };
    assert!(placed(&dark, lens(), 0.3).is_none());
}

#[test]
fn the_uniform_carries_the_look_and_the_sun() {
    let look = look();
    let u = SunFlareUniform::new(&look, Vec2::new(0.5, -0.25), 2.0, Vec3::ONE, 0.7);
    assert_eq!(u.sun, Vec4::new(0.5, -0.125, 2.0, 0.7));
    assert_eq!(u.rays.w, look.ghosts.len() as f32);
    assert_eq!(u.ghosts[0].x, look.ghosts[0].offset);
    assert_eq!(u.taps[0].z, 1.0);
}

#[test]
fn the_bloom_is_the_data_s() {
    let b = bloom_from(&BloomLook::default());
    assert_eq!(b.intensity, BloomLook::default().intensity);
    assert_eq!(b.composite_mode, BloomCompositeMode::EnergyConserving);
    let additive = BloomLook {
        additive: true,
        threshold: 1.5,
        ..default()
    };
    let b = bloom_from(&additive);
    assert_eq!(b.composite_mode, BloomCompositeMode::Additive);
    assert_eq!(b.prefilter.threshold, 1.5);
}

#[test]
fn the_cockpit_s_share_starts_clear() {
    // 1.0 as a half float.
    assert_eq!(half_one(), [0x00, 0x3C]);
}
