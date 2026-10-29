"""Reads a game log run with CB_VIEW_TRACE=1 (openspec change `cockpit-local-space` section 3) and says whether the
seated view turned smoothly.

    python tools/perf/view_trace.py output/perf/city-stand.log

For each frame it takes the change in drawn yaw since the last frame, degrees per second. A view that glides keeps that
rate close to its neighbours'; a saw-tooth (the view a step behind, then snapped on) shows as frames turning backwards
against the turn, or a rate that swings far from the frames around it. Only frames where the view is turning (above
5 degrees a second) count.
"""

import re
import sys

LINE = re.compile(r"VIEW_TRACE t=([\d.]+) yaw=(-?[\d.]+) heading=(-?[\d.]+|NaN) alpha=([\d.]+)")


def wrap(d):
    return (d + 180.0) % 360.0 - 180.0


def main(path):
    rows = [tuple(float(x) for x in m.groups()) for m in map(LINE.search, open(path, encoding="utf-8", errors="replace")) if m]
    if len(rows) < 3:
        print("no VIEW_TRACE lines: run with CB_VIEW_TRACE=1, seated")
        return 1
    rates = []
    for (t0, y0, _, _), (t1, y1, _, _) in zip(rows, rows[1:]):
        dt = t1 - t0
        if dt > 0:
            rates.append(wrap(y1 - y0) / dt)
    turning = backwards = jumps = 0
    for prev, now, nxt in zip(rates, rates[1:], rates[2:]):
        around = (prev + nxt) / 2.0
        if abs(around) < 5.0:
            continue
        turning += 1
        if now * around < 0:
            backwards += 1
        elif abs(now - around) > 0.5 * abs(around):
            jumps += 1
    print(f"{len(rows)} seated frames, {turning} turning")
    print(f"  turned backwards against the turn: {backwards}")
    print(f"  rate off its neighbours' by more than half: {jumps}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
