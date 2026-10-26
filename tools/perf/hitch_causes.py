"""Reads a run's log for its `hitch:` lines (perf_hud: every frame over 16.7 ms, with the main world's share and each
timed system's milliseconds in it; a `--features hitch_spans` build times every system, main and render world) and
says what the slow frames spent their time on.

Prints how many frames were over 16.7 ms, in how many frames the main world alone took over 16.7 ms, the systems
most often the largest cost of a slow frame, and for every system that took 2 ms or more in any slow frame: how many
slow frames it did so in, its mean and its worst.

    python3 tools/perf/hitch_causes.py output/perf/streaming-route.log [more logs...]
"""

import re
import sys
from collections import defaultdict

LINE = re.compile(r"hitch: frame (\d+) ([\d.]+) ms, main ([\d.]+) ms \| ?([^|]*)(?:\| ?(.*))?$")
PART = re.compile(r"(\S+) ([\d.]+)$")
BIG_MS = 2.0
# Spans that hold other timed spans (a schedule run inside one system): left out of "largest cost", so the system
# inside that took the time is named instead.
CONTAINERS = ("run_render_schedule", "run_fixed_main_schedule")


def read(path):
    out = []
    with open(path, errors="replace") as f:
        for line in f:
            m = LINE.search(line.rstrip())
            if not m:
                continue
            parts = []
            for p in filter(None, (s.strip() for s in m.group(4).split(","))):
                q = PART.match(p)
                if q:
                    parts.append((q.group(1), float(q.group(2))))
            counts = []
            for p in filter(None, (s.strip() for s in (m.group(5) or "").split(","))):
                q = PART.match(p)
                if q:
                    counts.append((q.group(1), float(q.group(2))))
            out.append((int(m.group(1)), float(m.group(2)), float(m.group(3)), parts, counts))
    return out


def main(paths):
    for path in paths:
        rows = read(path)
        print(f"== {path}")
        if not rows:
            print("no hitch lines")
            continue
        ms = [r[1] for r in rows]
        over = sum(1 for m in ms if m > 1000.0 / 60.0)
        main_over = sum(1 for r in rows if r[2] > 1000.0 / 60.0)
        # With pipelined rendering a main-world spike shows in the NEXT frame's length, so it is logged on a line of
        # its own (main over the line, length not yet); both kinds are read here.
        print(f"{len(rows)} lines; {over} frames over 16.7 ms (worst {max(ms):.1f} ms); main world over 16.7 ms in {main_over}")
        for lo, hi in [(16.7, 25), (25, 33.3), (33.3, 50), (50, 1e9)]:
            n = sum(1 for m in ms if lo < m <= hi)
            print(f"  {lo:5.1f} to {hi if hi < 1e9 else 'inf':>5} ms: {n}")
        top = defaultdict(int)
        big = defaultdict(list)
        counted = defaultdict(list)
        for _, _, _, parts, counts in rows:
            for name, n in counts:
                counted[name].append(n)
            # Guards nest (a `slow` system and the span of the same system): the largest named cost leads.
            leaves = [p for p in parts if not p[0].endswith(CONTAINERS)]
            if leaves:
                top[leaves[0][0]] += 1
            for name, v in parts:
                if v >= BIG_MS:
                    big[name].append(v)
        print("largest cost in the slow frame (count):")
        for name, n in sorted(top.items(), key=lambda x: -x[1])[:15]:
            print(f"  {n:5d}  {name}")
        print(f"systems at {BIG_MS} ms or more in a slow frame: frames, mean ms, worst ms")
        for name, vs in sorted(big.items(), key=lambda x: -sum(x[1]))[:25]:
            print(f"  {len(vs):5d}  {sum(vs) / len(vs):7.1f}  {max(vs):7.1f}  {name}")
        if counted:
            print("entities moved or spawned in slow frames (hitch_spans builds): frames, mean, most")
            for name, ns in sorted(counted.items(), key=lambda x: -sum(x[1]))[:20]:
                print(f"  {len(ns):5d}  {sum(ns) / len(ns):8.0f}  {max(ns):8.0f}  {name}")


if __name__ == "__main__":
    main(sys.argv[1:])

