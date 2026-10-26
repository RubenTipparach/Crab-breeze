//! The command line, and finding the repository's `data/` and `assets/` directories.

use std::path::{Path, PathBuf};

/// How to call the game.
/// The mech flown when none is named.
pub const DEFAULT_MECH: &str = "mechs/foreman.ron";

pub const USAGE: &str = "usage: crab-breeze [--seed <n>] [--mech mechs/<name>.ron] [--stills <dir>] [--mission defence] [--showcase world|fight|defence|marks|sun|demolition|trail] [--bench world|fight|defence|marks|sun|demolition] [--start meadow|overlook] [--play free|free-continue|career-new|career-continue]";

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
    /// Play a showcase take (`world`, `fight`, `defence`, `marks` or `sun`) for a recording.
    pub showcase: Option<String>,
    /// Play a showcase take offscreen and report frame time (perf-bench).
    pub bench: Option<String>,
    /// The mode the menu started this run in (`launcher`): `free`, `free-continue`, `career-new` or
    /// `career-continue`.
    pub play: Option<String>,
    /// Start somewhere other than the city: `meadow`, the nearest open grassland (gpu-grass); `fx`, the same meadow
    /// with every effect set out in front of the pilot (`fx_gallery`).
    pub start: Option<String>,
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
            // The Foreman by default (owner, 2026-10-25: "wrong cockpit, use forman"), so plain runs,
            // showcases and benchmarks fly the mech and cockpit the game is built around.
            // `CB_MECH=mechs/<name>.ron` picks another without an argument (the perf suite's
            // variants set only the environment); `--mech` still wins.
            mech: std::env::var("CB_MECH").unwrap_or_else(|_| DEFAULT_MECH.into()),
            stills: None,
            mission: None,
            showcase: None,
            bench: None,
            play: None,
            start: None,
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
                "--play" => match it.next().as_deref() {
                    Some(m @ ("free" | "free-continue" | "career-new" | "career-continue")) => out.play = Some(m.into()),
                    _ => return Err("--play takes `free`, `free-continue`, `career-new` or `career-continue`".into()),
                },
                "--start" => match it.next().as_deref() {
                    Some("meadow") => out.start = Some("meadow".into()),
                    Some("fx") => out.start = Some("fx".into()),
                    Some("overlook") => out.start = Some("overlook".into()),
                    _ => return Err("--start takes `meadow`, `fx` or `overlook`".into()),
                },
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
        Some("world") | Some("fight") | Some("defence") | Some("marks") | Some("sun") | Some("demolition") | Some("trail") => {
            Ok(v.unwrap_or_default())
        }
        _ => Err("--showcase and --bench take `world`, `fight`, `defence`, `marks`, `sun`, `demolition` or `trail`".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_foreman_is_flown_when_no_mech_is_named() {
        if std::env::var("CB_MECH").is_ok() {
            return; // a run that names its mech in the environment
        }
        let args = Args::parse(std::iter::empty()).unwrap();
        assert_eq!(args.mech, "mechs/foreman.ron");
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data").join(&args.mech);
        assert!(file.exists(), "{} is in data/", args.mech);
    }
}
