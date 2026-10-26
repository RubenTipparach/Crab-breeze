"""Reads a free-roam performance run's frame log (output/perf/<script>-frames.csv, written by the scripted pilot's
`log_frames`) and says where the frame rate left the owner's 60-120 fps band (owner, 2026-10-26: "ensure it always
stays withing the 60-120 fps range. If it drops below raise some flags and study the conditions that cause it").

Prints the whole run's p50/p95/p99 frame times, each beat's, the share of frames over 16.7 ms (under 60 fps), every
hitch over 33 ms with what the mech was doing, and slow stretches grouped by place on a 500 m grid. Writes the per
second series as JSON beside the CSV for the report page's frame graph.

    python tools/perf/free_roam_report.py output/perf/free-roam-perf-frames.csv
"""

import csv
import json
import sys
from collections import defaultdict
from pathlib import Path

SLOW_MS = 1000.0 / 60.0
HITCH_MS = 1000.0 / 30.0
GRID_M = 500.0


def pct(xs, p):
    if not xs:
        return float("nan")
    s = sorted(xs)
    return s[min(len(s) - 1, int(p / 100.0 * len(s)))]


def main(path):
    rows = []
    with open(path, newline="") as f:
        for r in csv.DictReader(f):
            try:
                rows.append({k: (v if k == "mode" else float(v)) for k, v in r.items()})
            except ValueError:
                continue
    if not rows:
        print("no frames")
        return
    # The first 3 s after the take starts hold shader and pipeline compiles; reported apart.
    t0 = rows[0]["real_s"]
    warm = [r for r in rows if r["real_s"] - t0 < 3.0]
    rows = [r for r in rows if r["real_s"] - t0 >= 3.0]
    ms = [r["frame_ms"] for r in rows]
    dur = rows[-1]["real_s"] - rows[0]["real_s"]
    print(f"{len(rows)} frames over {dur / 60:.1f} min (and {len(warm)} warm-up frames, worst {max((r['frame_ms'] for r in warm), default=0):.1f} ms)")
    print(f"frame ms p50 {pct(ms, 50):.2f}  p95 {pct(ms, 95):.2f}  p99 {pct(ms, 99):.2f}  max {max(ms):.1f}")
    print(f"under 60 fps: {sum(m > SLOW_MS for m in ms) / len(ms) * 100:.2f}% of frames;  over 120 fps: {sum(m < 1000 / 120 for m in ms) / len(ms) * 100:.1f}%")

    print("\nby beat: frames, p50, p95, p99, % under 60 fps, entities max")
    beats = defaultdict(list)
    for r in rows:
        beats[int(r["beat"])].append(r)
    for b, rs in sorted(beats.items()):
        m = [r["frame_ms"] for r in rs]
        print(f"  beat {b:2d}: {len(m):6d}  {pct(m, 50):6.2f} {pct(m, 95):6.2f} {pct(m, 99):6.2f}  {sum(x > SLOW_MS for x in m) / len(m) * 100:5.1f}%  {max(r['entities'] for r in rs):.0f}")

    print("\nby speed (seated): p50, p95, % under 60")
    bands = [(0, 2), (2, 15), (15, 40), (40, 1e9)]
    for lo, hi in bands:
        m = [r["frame_ms"] for r in rows if r["mode"] == "seated" and lo <= r["speed_m_s"] < hi]
        if m:
            print(f"  {lo:>3}-{hi if hi < 1e9 else 'up':>3} m/s: {len(m):6d}  {pct(m, 50):6.2f} {pct(m, 95):6.2f}  {sum(x > SLOW_MS for x in m) / len(m) * 100:5.1f}%")

    print("\nby enemies alive: p50, p95, % under 60")
    en = defaultdict(list)
    for r in rows:
        en[min(int(r["enemies"]), 6)].append(r["frame_ms"])
    for k, m in sorted(en.items()):
        print(f"  {k}{'+' if k == 6 else ' '}: {len(m):6d}  {pct(m, 50):6.2f} {pct(m, 95):6.2f}  {sum(x > SLOW_MS for x in m) / len(m) * 100:5.1f}%")

    print(f"\nhitches over {HITCH_MS:.1f} ms:")
    hitches = [r for r in rows if r["frame_ms"] > HITCH_MS]
    for r in hitches[:60]:
        print(f"  t {r['real_s'] - t0:7.1f} s  {r['frame_ms']:6.1f} ms  beat {int(r['beat'])}  at ({r['x']:.0f}, {r['z']:.0f}) y {r['y']:.0f}  {r['speed_m_s']:.0f} m/s  {r['mode']}  enemies {int(r['enemies'])}  entities {int(r['entities'])}")
    if len(hitches) > 60:
        print(f"  ... {len(hitches) - 60} more")

    print(f"\nslowest places ({GRID_M:.0f} m cells, at least 200 frames): p95, % under 60")
    cells = defaultdict(list)
    for r in rows:
        cells[(int(r["x"] // GRID_M), int(r["z"] // GRID_M))].append(r["frame_ms"])
    ranked = sorted(((pct(m, 95), k, m) for k, m in cells.items() if len(m) >= 200), reverse=True)
    for p95, (cx, cz), m in ranked[:12]:
        print(f"  cell ({cx * GRID_M:.0f}, {cz * GRID_M:.0f}): {len(m):6d} frames  p95 {p95:6.2f}  {sum(x > SLOW_MS for x in m) / len(m) * 100:5.1f}%")

    # Per second: fps, worst frame, and context, for the frame graph.
    series = []
    sec = None
    for r in rows:
        s = int(r["real_s"] - t0)
        if sec is None or sec["s"] != s:
            sec = {"s": s, "n": 0, "worst": 0.0, "sum": 0.0, "beat": int(r["beat"]), "x": r["x"], "z": r["z"], "speed": r["speed_m_s"], "enemies": int(r["enemies"])}
            series.append(sec)
        sec["n"] += 1
        sec["sum"] += r["frame_ms"]
        sec["worst"] = max(sec["worst"], r["frame_ms"])
    out = [{"s": x["s"], "fps": round(1000.0 * x["n"] / x["sum"], 1), "worst_ms": round(x["worst"], 1), "beat": x["beat"], "x": round(x["x"]), "z": round(x["z"]), "speed": round(x["speed"], 1), "enemies": x["enemies"]} for x in series if x["sum"] > 0]
    js = Path(path).with_suffix(".seconds.json")
    js.write_text(json.dumps(out))
    print(f"\nper-second series: {js}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "output/perf/free-roam-perf-frames.csv")
