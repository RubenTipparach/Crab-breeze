//! The command line, and finding the repository's `data/` and `assets/` directories.

use std::path::{Path, PathBuf};

/// How to call the game.
pub const USAGE: &str = "usage: crab-breeze [--seed <n>] [--mech mechs/<name>.ron] [--stills <dir>] [--showcase]";

/// Parsed command line.
#[derive(Debug, PartialEq)]
pub struct Args {
    /// World seed.
    pub seed: u64,
    /// Mech data file, relative to `data/`.
    pub mech: String,
    /// Capture instrument output directory (`pilot-boarding` design section 4).
    pub stills: Option<PathBuf>,
    /// Play the showcase take for a recording.
    pub showcase: bool,
}

impl Args {
    /// Parses arguments (without the program name).
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut out = Args {
            seed: 42,
            mech: "mechs/placeholder.ron".into(),
            stills: None,
            showcase: false,
        };
        let mut it = args;
        while let Some(a) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
            match a.as_str() {
                "--seed" => out.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
                "--mech" => out.mech = value()?,
                "--stills" => out.stills = Some(PathBuf::from(value()?)),
                "--showcase" => out.showcase = true,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(out)
    }
}

/// The repository root: `CB_ROOT` if set, else the nearest ancestor of the working directory with
/// `data/world/world.ron`, else the one this binary was built from.
pub fn repo_root() -> PathBuf {
    if let Ok(r) = std::env::var("CB_ROOT") {
        return PathBuf::from(r);
    }
    let marker = Path::new("data/world/world.ron");
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            if dir.join(marker).exists() {
                return dir.to_path_buf();
            }
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
