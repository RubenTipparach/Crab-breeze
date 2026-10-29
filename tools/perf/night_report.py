"""Reports the night's ablation (tools/perf/ablate_night.sh, openspec change `night-frame`): for each variant and round,
the frames of city-stand's look round from the cockpit (its beat 4), their p50, p95 and p99, the share over 16.7 ms,
and the GPU's own time from the run's `perf:` lines in the same beat's time. A measurement instrument.

    python tools/perf/night_report.py [dir] [look beat]   (city-stand's look is beat 4, suburb-stand's 6)
"""

import csv
import re
import statistics
import sys
from pathlib import Path

LOOK_BEAT = "4"


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, int(p / 100 * len(v)))]


def gpu_ms(log: Path):
    text = log.read_text(encoding="utf-8", errors="replace")
    text = re.sub(r"\x1b\[[0-9;]*m", "", text)
    # The cockpit part of the run: after boarding.
    seated = text.split("Board done", 1)[-1]
    g = [float(m) for m in re.findall(r"perf: \d+ fps, [\d.]+ ms, GPU ([\d.]+) ms", seated)]
    return statistics.median(g[1:]) if len(g) > 1 else float("nan")


def main():
    d = Path(sys.argv[1] if len(sys.argv) > 1 else "output/perf/night")
    look = sys.argv[2] if len(sys.argv) > 2 else LOOK_BEAT
    rows = []
    for f in sorted(d.glob("*.csv")):
        with f.open() as fh:
            ms = [float(r["frame_ms"]) for r in csv.DictReader(fh) if r["beat"] == look]
        if not ms:
            continue
        log = f.with_suffix(".log")
        rows.append((f.stem, len(ms), pct(ms, 50), pct(ms, 95), pct(ms, 99), max(ms),
                     sum(m > 16.7 for m in ms) / len(ms), gpu_ms(log) if log.exists() else float("nan")))
    print(f"{'run':18} {'n':>5} {'p50':>7} {'p95':>7} {'p99':>7} {'max':>7} {'>16.7':>7} {'GPU':>6}")
    for r in rows:
        print(f"{r[0]:18} {r[1]:5d} {r[2]:7.2f} {r[3]:7.2f} {r[4]:7.2f} {r[5]:7.1f} {r[6]:7.1%} {r[7]:6.2f}")


if __name__ == "__main__":
    main()
