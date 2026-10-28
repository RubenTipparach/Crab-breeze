"""The 60 fps gate (CLAUDE.md section 7): plays a scripted perf route in a release build, on screen, and exits non-zero
if any frame after GAME_READY took longer than 16.7 ms.

The game judges its own frames (`--perf-gate`, `crates/cb-app/src/perf_gate.rs`, the rule in `cb_core::perf`) and
prints one `PERF_GATE pass|fail` line on the way out; this tool launches it, keeps the log, and prints that line with
every over-budget frame and the `hitch:` line that names what the frame spent its time on.

    cargo build --release -p cb-app
    python3 tools/perf/perf_gate.py streaming-route free-roam-fight
    python3 tools/perf/perf_gate.py --bin target/release/crab-breeze --tag new streaming-route

On the Mac run it under the lock, with the display awake:
    tools/perf/mac_lock.sh perf caffeinate -d -u python3 tools/perf/perf_gate.py streaming-route free-roam-fight

Exit status: 0 when every route passed, 1 when a frame went over (or a route did not finish), 2 on a usage error.
Logs go to output/perf/gate-<route>-<tag>.log, the frame file to output/perf/gate-<route>-<tag>-frames.csv.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GATE = re.compile(r"PERF_GATE (pass|fail): (\d+) of (\d+) frames")
OVER = re.compile(r"PERF_GATE over: frame (\d+) after GAME_READY took ([\d.]+) ms")


def run(binary, route, tag, env_extra):
    out = ROOT / "output" / "perf"
    out.mkdir(parents=True, exist_ok=True)
    log = out / f"gate-{route}-{tag}.log"
    env = dict(os.environ)
    # On screen, at the owner's 1080p whatever the display's scale, not waiting for vsync (CLAUDE.md section 7).
    env.setdefault("CB_UNCAPPED", "1")
    env.setdefault("CB_NATIVE_1080", "1")
    env.update(env_extra)
    with open(log, "w") as f:
        code = subprocess.call([str(binary), "--playtest", route, "--perf-gate"], cwd=ROOT, stdout=f, stderr=subprocess.STDOUT, env=env)
    frames = out / f"{route}-frames.csv"
    if frames.exists():
        shutil.move(frames, out / f"gate-{route}-{tag}-frames.csv")
    text = log.read_text(errors="replace")
    gate = GATE.search(text)
    finished = "PLAYTEST_RESULT ok" in text
    print(f"== {route} ({tag}): exit {code}, log {log.relative_to(ROOT)}")
    for line in text.splitlines():
        if GATE.search(line):
            print("  " + line.strip()[:600])
        elif OVER.search(line) or "pipeline late:" in line:
            print("  " + line.strip()[-300:])
    if not finished:
        print("  the route did not finish (no PLAYTEST_RESULT ok): the run is invalid")
    return gate is not None and gate.group(1) == "pass" and finished and code == 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("routes", nargs="+", help="playtest scripts in data/playtests/ (streaming-route, free-roam-fight)")
    ap.add_argument("--bin", default=str(ROOT / "target" / "release" / ("crab-breeze.exe" if os.name == "nt" else "crab-breeze")))
    ap.add_argument("--tag", default="gate")
    ap.add_argument("--env", action="append", default=[], help="NAME=VALUE for the game, repeatable")
    a = ap.parse_args()
    if not Path(a.bin).exists():
        print(f"no binary at {a.bin}: cargo build --release -p cb-app")
        return 2
    env = dict(e.split("=", 1) for e in a.env)
    ok = all([run(a.bin, r, a.tag, env) for r in a.routes])
    print("PASS: every frame after GAME_READY within 16.7 ms" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
