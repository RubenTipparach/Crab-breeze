//! The command line, and finding the repository's `data/` and `assets/` directories.

use std::path::{Path, PathBuf};

/// How to call the game.
pub const USAGE: &str = "usage: crab-breeze [--seed <n>] [--mech mechs/<name>.ron] [--stills <dir>] [--mission defence] [--showcase world|fight|defence] [--bench world|fight|defence]";

/// Parsed command line.
#[derive(Debug, PartialEq)]
pub struct Args {
    /// World seed.
    pub seed: u64,
    /// Mech data file, relative to `data/`.
    pub mech: String,
    /// Capture instrument output directory (`pilot-boarding` design section 4).
    pub stills: Option<PathBuf>,
    /// Play a mission yourself (`defence`).
    pub mission: Option<String>,
    /// Play a showcase take (`world`, `fight` or `defence`) for a recording.
    pub showcase: Option<String>,
    /// Play a showcase take offscreen and report frame time (perf-bench).
    pub bench: Option<String>,
}

impl Args {
    /// True when this run plays the defence mission (by hand, as a showcase take or a bench).
    pub fn defending(&self) -> bool {
        self.mission.as_deref() == Some("defence")
            || self.showcase.as_deref() == Some("defence")
            || self.bench.as_deref() == Some("defence")
    }

    /// Parses arguments (without the program name).
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut out = Args {
            seed: 42,
            mech: "mechs/placeholder.ron".into(),
            stills: None,
            mission: None,
            showcase: None,
            bench: None,
        };
        let mut it = args;
        while let Some(a) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
            match a.as_str() {
                "--seed" => out.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
                "--mech" => out.mech = value()?,
                "--stills" => out.stills = Some(PathBuf::from(value()?)),
                "--mission" => match it.next().as_deref() {
                    Some("defence") => out.mission = Some("defence".into()),
                    _ => return Err("--mission takes `defence`".into()),
                },
                "--showcase" => out.showcase = Some(take_name(it.next())?),
                "--bench" => out.bench = Some(take_name(it.next())?),
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

/// True when the environment variable `name` is set to anything but `0` (perf-bench switches).
pub fn switched_off(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0")
}

fn take_name(v: Option<String>) -> Result<String, String> {
    match v.as_deref() {
        Some("world") | Some("fight") | Some("defence") => Ok(v.unwrap_or_default()),
        _ => Err("--showcase and --bench take `world`, `fight` or `defence`".into()),
    }
}
