//! The command line, and finding the repository's `data/` and `assets/` directories.

use std::path::{Path, PathBuf};

/// How to call the game.
/// The mech flown when none is named.
pub const DEFAULT_MECH: &str = "mechs/foreman.ron";

pub const USAGE: &str = "usage: crab-breeze [--seed <n>] [--mech mechs/<name>.ron] [--stills <dir>] [--mission defence|fortress] [--showcase world|fight|defence|marks|sun|demolition|trail] [--bench world|fight|defence|marks|sun|demolition] [--start meadow|overlook|base] [--play free|free-continue|career-new|career-continue] [--playtest <name> [--run <i>]] [--perf-gate]";

/// Parsed command line.
#[derive(Debug, PartialEq)]
pub struct Args {
    /// World seed.
    pub seed: u64,
    /// Mech data file, relative to `data/`.
    pub mech: String,
    /// Capture instrument output directory (`pilot-boarding` design section 4).
    pub stills: Option<PathBuf>,
    /// Play a mission yourself (`defence`; `fortress`: free roam with the Stratostacean running).
    pub mission: Option<String>,
    /// Play a showcase take (`world`, `fight`, `defence`, `marks` or `sun`) for a recording.
    pub showcase: Option<String>,
    /// Play a showcase take offscreen and report frame time (perf-bench).
    pub bench: Option<String>,
    /// The mode the menu started this run in (the main menu): `free`, `free-continue`, `career-new` or
    /// `career-continue`.
    pub play: Option<String>,
    /// Start somewhere other than the city: `meadow`, the nearest open grassland (gpu-grass); `fx`, the same meadow
    /// with every effect set out in front of the pilot (`fx_gallery`).
    pub start: Option<String>,
    /// Play a playtest script, `data/playtests/<name>.ron` (playtest-pipeline): its run sets `--play`.
    pub playtest: Option<String>,
    /// Which of the script's runs this launch plays.
    pub run: usize,
    /// `--perf-gate`: judge every frame after `GAME_READY` against 16.7 ms (`cb_core::perf`) and exit non-zero when
    /// any was over (CLAUDE.md section 7).
    pub perf_gate: bool,
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
            playtest: None,
            run: 0,
            perf_gate: false,
        };
        let mut it = args;
        while let Some(a) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
            match a.as_str() {
                "--seed" => out.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
                "--mech" => out.mech = value()?,
                "--stills" => out.stills = Some(PathBuf::from(value()?)),
                "--mission" => match it.next().as_deref() {
                    Some(m @ ("defence" | "fortress")) => out.mission = Some(m.into()),
                    _ => return Err("--mission takes `defence` or `fortress`".into()),
                },
                "--showcase" => out.showcase = Some(take_name(it.next())?),
                "--bench" => out.bench = Some(take_name(it.next())?),
                "--play" => match it.next().as_deref() {
                    Some(m @ ("free" | "free-continue" | "career-new" | "career-continue")) => out.play = Some(m.into()),
                    _ => return Err("--play takes `free`, `free-continue`, `career-new` or `career-continue`".into()),
                },
                "--playtest" => out.playtest = Some(value()?),
                "--run" => out.run = value()?.parse().map_err(|e| format!("--run: {e}"))?,
                "--perf-gate" => out.perf_gate = true,
                "--start" => match it.next().as_deref() {
                    Some("meadow") => out.start = Some("meadow".into()),
                    Some("fx") => out.start = Some("fx".into()),
                    Some("overlook") => out.start = Some("overlook".into()),
                    Some("base") => out.start = Some("base".into()),
                    _ => return Err("--start takes `meadow`, `fx`, `overlook` or `base`".into()),
                },
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(out)
    }
}

/// The file that marks a root holding the game's `data/` (and `assets/` beside it).
const ROOT_MARKER: &str = "data/world/world.ron";

/// The repository root: `CB_ROOT` if set; else the executable's own folder when the game is shipped with its files
/// beside it (`shipped_in`, openspec change `itch-deploy`); else the nearest ancestor of the working directory with
/// `data/world/world.ron`; else the one this binary was built from.
pub fn repo_root() -> PathBuf {
    if let Ok(r) = std::env::var("CB_ROOT") {
        return PathBuf::from(r);
    }
    if let Some(dir) = shipped_in(std::env::current_exe().ok().as_deref()) {
        return dir;
    }
    let marker = Path::new(ROOT_MARKER);
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            if dir.join(marker).exists() {
                return dir.to_path_buf();
            }
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The folder of the executable `exe` when it holds the game's files (`data/world/world.ron`): a shipped build,
/// started from anywhere, reads its own files and keeps its saves there. A dev build in `target/` has none beside it.
fn shipped_in(exe: Option<&Path>) -> Option<PathBuf> {
    let dir = exe?.parent()?;
    dir.join(ROOT_MARKER).is_file().then(|| dir.to_path_buf())
}

/// True when the environment variable `name` is set to anything but `0` (perf-bench switches).
pub fn switched_off(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0")
}

fn take_name(v: Option<String>) -> Result<String, String> {
    match v.as_deref() {
        Some("world") | Some("fight") | Some("defence") | Some("marks") | Some("sun") | Some("demolition") | Some("trail")
        | Some("people") => Ok(v.unwrap_or_default()),
        _ => Err("--showcase and --bench take `world`, `fight`, `defence`, `marks`, `sun`, `demolition`, `trail` or `people`".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shipped_game_reads_the_files_beside_its_exe_and_a_dev_build_does_not() {
        let tmp = std::env::temp_dir().join(format!("cb-shipped-{}", std::process::id()));
        let shipped = tmp.join("crab-breeze");
        std::fs::create_dir_all(shipped.join("data/world")).unwrap();
        std::fs::write(shipped.join(ROOT_MARKER), "()").unwrap();
        let dev = tmp.join("target/release");
        std::fs::create_dir_all(&dev).unwrap();
        assert_eq!(
            shipped_in(Some(&shipped.join("crab-breeze.exe"))),
            Some(shipped.clone()),
            "beside the exe"
        );
        assert_eq!(
            shipped_in(Some(&dev.join("crab-breeze"))),
            None,
            "a dev build has no data beside it"
        );
        assert_eq!(shipped_in(None), None);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn perf_gate_is_switched_on_by_its_flag() {
        let args = Args::parse(["--playtest", "streaming-route", "--perf-gate"].into_iter().map(String::from)).unwrap();
        assert!(args.perf_gate);
        assert!(!Args::parse(std::iter::empty()).unwrap().perf_gate);
    }

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
