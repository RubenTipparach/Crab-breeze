"""Compares the fight ablation runs (tools/perf/ablate_fight.sh): for each variant, the drive to the job (beat 4) and
the fight (beat 5) of every round, frame ms p50/p95/p99 and the share of frames under 60 fps, then the variant's
median over its rounds. A difference smaller than the spread between a variant's rounds is not a result.

    python tools/perf/ablate_report.py output/perf/ablate
"""

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

SLOW_MS = 1000.0 / 60.0


def pct(xs, p):
    s = sorted(xs)
    return s[min(len(s) - 1, int(p / 100.0 * len(s)))] if s else float("nan")


def stats(ms):
    if not ms:
        return None
    return (pct(ms, 50), pct(ms, 95), pct(ms, 99), sum(m > SLOW_MS for m in ms) / len(ms) * 100, len(ms))


def main(folder):
    runs = defaultdict(list)
    for f in sorted(Path(folder).glob("*.csv")):
        variant, _, rnd = f.stem.rpartition("-")
        beats = defaultdict(list)
        with open(f, newline="") as fh:
            rows = list(csv.DictReader(fh))
        if not rows:
            continue
        t0 = float(rows[0]["real_s"])
        for r in rows:
            if float(r["real_s"]) - t0 < 3.0:
                continue
            beats[int(r["beat"])].append(float(r["frame_ms"]))
        runs[variant].append((rnd, stats(beats.get(4, [])), stats(beats.get(5, []))))
    for label, idx in (("drive to the job (beat 4)", 1), ("the fight (beat 5)", 2)):
        print(f"\n{label}: per round p50 / p95 / p99 ms, % under 60 fps, frames")
        for v, rs in runs.items():
            cells = [r[idx] for r in rs if r[idx]]
            line = "   ".join(f"{s[0]:5.2f} {s[1]:6.2f} {s[2]:6.2f} {s[3]:5.1f}% {s[4]:5d}" for s in cells)
            med = statistics.median(s[0] for s in cells) if cells else float("nan")
            med95 = statistics.median(s[1] for s in cells) if cells else float("nan")
            print(f"  {v:18s} {line}   | median p50 {med:5.2f}  p95 {med95:6.2f}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "output/perf/ablate")
