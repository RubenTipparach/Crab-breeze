"""Sums a Bevy `trace_chrome` trace's spans (begin/end pairs per thread) over its last N bytes: each span name's total,
count, mean and worst, ranked by total, and the main frame's count, so per-frame costs can be read off. A measurement
instrument for traces too big to load whole (gigabytes).

    python tools/perf/trace_spans.py <trace.json> [last_gb]
"""

import os
import re
import sys
from collections import defaultdict

EV = re.compile(rb'"name":"((?:[^"\\]|\\.)*)","ph":"([BE])","pid":\d+,"tid":(\d+),"ts":([\d.]+)')


def main(path, last_gb=5.0):
    size = os.path.getsize(path)
    start = max(0, size - int(last_gb * 1e9))
    stacks = defaultdict(list)
    total = defaultdict(float)
    count = defaultdict(int)
    worst = defaultdict(float)
    t_first = t_last = None
    with open(path, "rb") as f:
        f.seek(start)
        if start:
            f.readline()
        for line in f:
            m = EV.search(line)
            if not m:
                continue
            name, ph, tid, ts = m.group(1), m.group(2), m.group(3), float(m.group(4))
            t_first = ts if t_first is None else t_first
            t_last = ts
            st = stacks[tid]
            if ph == b"B":
                st.append((name, ts))
            else:
                # Pop to the matching begin (a span cut at the window's start has none).
                for i in range(len(st) - 1, -1, -1):
                    if st[i][0] == name:
                        d = ts - st[i][1]
                        del st[i:]
                        total[name] += d
                        count[name] += 1
                        worst[name] = max(worst[name], d)
                        break
    secs = (t_last - t_first) / 1e6
    frames = max((count[n] for n in count if n.startswith(b"frame")), default=0)
    print(f"window {secs:.1f} s, {frames} frames ({frames / secs:.1f} fps traced)")
    for n in sorted(total, key=total.get, reverse=True)[:400]:
        name = n.decode(errors="replace")[:90]
        print(f"{total[n] / 1e3 / max(frames, 1):8.2f} ms/frame  {count[n]:8d}x  mean {total[n] / count[n] / 1e3:7.3f} ms  worst {worst[n] / 1e3:8.1f} ms  {name}")


if __name__ == "__main__":
    main(sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 5.0)
