//! The water shaders compile, and the Rust uniforms match them (openspec change
//! `water-rendering`). The shaders are compiled the way Bevy 0.20 compiles them (the `wesl`
//! compiler with Bevy's options, then naga's full validation), so a broken shader fails
//! `cargo test`, not the game. The surface shader imports Bevy's own `bevy_pbr` modules, which
//! are read from Bevy's sources as Cargo has them.

use super::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wesl::syntax::{ModulePath, PathOrigin};

/// The repository's `assets/shaders`.
fn shader_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/shaders")
}

/// Resolves the game's shaders (`/shaders/<name>`, as Bevy names an asset WESL module), Bevy's
/// packages (`bevy_pbr::...`, from their sources) and Bevy's `constants` module.
struct Resolver {
    packages: HashMap<String, PathBuf>,
    constants: String,
}

impl wesl::Resolver for Resolver {
    // A package imported from inside another is named `outer/inner`; it is `inner` (as Bevy's own
    // `ShaderResolver` has it).
    fn canonical_path(&self, path: &ModulePath) -> ModulePath {
        match &path.origin {
            PathOrigin::Package(p) if p.contains('/') => ModulePath {
                origin: PathOrigin::Package(p.rsplit('/').next().unwrap_or(p).to_string()),
                components: path.components.clone(),
            },
            _ => path.clone(),
        }
    }

    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, wesl::error::ResolveError> {
        let path = &self.canonical_path(path);
        let missing = |why: &str| wesl::error::ResolveError::ModuleNotFound(path.clone(), why.to_string());
        let file = match &path.origin {
            PathOrigin::Package(p) if p == "constants" && path.components.is_empty() => {
                return Ok(Cow::Borrowed(self.constants.as_str()));
            }
            PathOrigin::Package(p) => self
                .packages
                .get(p)
                .ok_or_else(|| missing("package not found"))?
                .join("src")
                .join(path.components.join("/")),
            PathOrigin::Absolute if path.components.first().map(String::as_str) == Some("shaders") => {
                shader_dir().join(path.components[1..].join("/"))
            }
            _ => return Err(missing("unexpected module path")),
        };
        std::fs::read_to_string(file.with_extension("wesl"))
            .map(Cow::Owned)
            .map_err(|e| missing(&e.to_string()))
    }
}

/// Compiles the game shader `name` (in `assets/shaders`) with these feature flags, as Bevy's shader
/// cache does, then validates it with naga.
fn compile(resolver: &Resolver, name: &str, flags: &[(&str, bool)]) -> naga::Module {
    compile_with(resolver, name, flags, true)
}

/// `compile`; `strip` false keeps every declaration, used by an entry point or not.
fn compile_with(resolver: &Resolver, name: &str, flags: &[(&str, bool)], strip: bool) -> naga::Module {
    let mut options = wesl::CompileOptions {
        imports: true,
        condcomp: true,
        visibility: false,
        strip,
        ..Default::default()
    };
    for (flag, on) in flags {
        options.features.flags.insert((*flag).to_string(), (*on).into());
    }
    let main = ModulePath {
        origin: PathOrigin::Absolute,
        components: vec!["shaders".into(), name.into()],
    };
    let wgsl = wesl::compile(&main, &options, resolver)
        .unwrap_or_else(|e| panic!("{name} {flags:?} did not compile:\n{}", e.diagnostic().render_plain()))
        .to_string();
    let module =
        naga::front::wgsl::parse_str(&wgsl).unwrap_or_else(|e| panic!("{name} {flags:?} did not parse:\n{}", e.emit_to_string(&wgsl)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name} {flags:?} did not validate:\n{}", e.emit_to_string(&wgsl)));
    module
}

/// The size in bytes of the struct named `name` in a compiled module.
fn struct_size(module: &naga::Module, name: &str) -> u32 {
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    let (handle, _) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no struct {name}"));
    layouter[handle].size
}

fn game_only() -> Resolver {
    Resolver {
        packages: HashMap::new(),
        constants: String::new(),
    }
}

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

/// Bevy 0.20's crate source directories by package name, as Cargo unpacked them into its registry
/// (`$CARGO_HOME/registry/src/<index>/bevy_*-0.20.0`; the workspace pins `bevy = "=0.20.0"`).
/// `None` when they are not there (a vendored or remote build).
fn bevy_sources() -> Option<HashMap<String, PathBuf>> {
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))?;
    let mut packages = HashMap::new();
    for index in std::fs::read_dir(home.join("registry/src")).ok()?.flatten() {
        for krate in std::fs::read_dir(index.path()).into_iter().flatten().flatten() {
            let folder = krate.file_name().to_string_lossy().into_owned();
            if let Some((name, "0.20.0")) = folder.rsplit_once('-')
                && name.starts_with("bevy_")
            {
                packages.insert(name.to_string(), krate.path());
            }
        }
    }
    packages.contains_key("bevy_pbr").then_some(packages)
}

#[test]
fn the_surface_shader_compiles_against_bevy_pbr() {
    let Some(packages) = bevy_sources() else {
        eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
        return;
    };
    // Bevy's integer shader defs become the `constants` module; these are the ones the imported
    // modules read.
    let constants = [
        ("MATERIAL_BIND_GROUP", 3),
        ("MAX_DIRECTIONAL_LIGHTS", 10),
        ("MAX_CASCADES_PER_LIGHT", 4),
        ("MAX_RECT_LIGHTS", 16),
        ("PER_OBJECT_BUFFER_BATCH_SIZE", 64),
        ("TONEMAPPING_LUT_TEXTURE_BINDING_INDEX", 18),
        ("TONEMAPPING_LUT_SAMPLER_BINDING_INDEX", 19),
        ("SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS", 8),
        ("SORTED_FRAGMENT_MAX_COUNT", 8),
        ("SLICE_COUNT", 4),
        ("SAMPLES_PER_SLICE_SIDE", 2),
    ]
    .iter()
    .map(|(k, v)| format!("const {k} = {v}u;\n"))
    .collect();
    let resolver = Resolver { packages, constants };
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
    assert_eq!(passes_needed(&dry, 0.0), (false, false));
}

#[test]
fn under_water_composes_but_puts_no_drops_on_the_glass() {
    let under = WaterEye {
        submersion: 1.0,
        ..default()
    };
    assert_eq!(passes_needed(&under, 0.0), (true, false));
}

#[test]
fn a_wet_glass_runs_both_passes() {
    let surfacing = WaterEye { drips: 0.6, ..default() };
    assert_eq!(passes_needed(&surfacing, 0.0), (true, true));
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
