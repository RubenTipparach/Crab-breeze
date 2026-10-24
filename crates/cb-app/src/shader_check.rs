//! Compiling the game's shaders in `cargo test` the way Bevy 0.20 compiles them (the `wesl`
//! compiler with Bevy's options, then naga's full validation), so a broken shader fails the tests,
//! not the game. Shared by every custom material's tests (openspec changes `water-rendering` and
//! `gpu-grass`); a shader that imports Bevy's own `bevy_pbr` modules reads them from Bevy's sources
//! as Cargo has them.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wesl::syntax::{ModulePath, PathOrigin};

/// The repository's `assets/shaders`.
pub fn shader_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/shaders")
}

/// Resolves the game's shaders (`/shaders/<name>`, as Bevy names an asset WESL module), Bevy's
/// packages (`bevy_pbr::...`, from their sources) and Bevy's `constants` module.
pub struct Resolver {
    /// Bevy's crates by package name.
    pub packages: HashMap<String, PathBuf>,
    /// Bevy's integer shader defs as the `constants` module.
    pub constants: String,
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
pub fn compile(resolver: &Resolver, name: &str, flags: &[(&str, bool)]) -> naga::Module {
    compile_with(resolver, name, flags, true)
}

/// `compile`; `strip` false keeps every declaration, used by an entry point or not.
pub fn compile_with(resolver: &Resolver, name: &str, flags: &[(&str, bool)], strip: bool) -> naga::Module {
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
pub fn struct_size(module: &naga::Module, name: &str) -> u32 {
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    let (handle, _) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no struct {name}"));
    layouter[handle].size
}

/// Resolves the game's own shaders only.
pub fn game_only() -> Resolver {
    Resolver {
        packages: HashMap::new(),
        constants: String::new(),
    }
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

/// A resolver for the game's shaders and Bevy 0.20's (`None` when Bevy's sources are not in the
/// Cargo registry; the test then says it skipped).
pub fn with_bevy() -> Option<Resolver> {
    let packages = bevy_sources()?;
    // Bevy's integer shader defs become the `constants` module; these are the ones the imported
    // modules read.
    let constants: String = [
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
    Some(Resolver { packages, constants })
}
