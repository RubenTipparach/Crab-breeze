"""Performance suite: play the benchmark takes and compare builds or variants.

Ported from Pale Blue Dot's `tools/perf_suite.py` (RubenTipparach/pale-blue-dot, revision
db25fe2), which follows godot-sandbox's battle bench: fixed scenarios from a fresh start, a warm-up
that is thrown away, a check that the run really did its hard part, one JSON file per run, and
variants interleaved A, B, then B, A so drift over the sitting falls on every variant alike.
CLAUDE.md section 7 and the `perf-measure` skill are the rules.

Crab Breeze's scenarios are its `--bench` takes (openspec change `perf-bench`): the game plays the
showcase take with the pilot's camera rendering offscreen at 1920 x 1080, so no frame waits for a
display. Each run writes a per-frame CSV (`CB_FRAME_LOG`), and the suite samples the process's
memory every 0.25 s.

Scenarios (each ends itself):
    world   the world take: walk to the mech, board, the street, rockets and skates, the river,
            the wood, shells, damage
    fight   the city fight: the player's mech against the raiders
    defence the combat test from the desktop session (`--showcase defence`): defend a city against
            waves of enemies, three fights

Variants are `name|exe|ENV=1;ENV2=2|root` (all but the name optional). `root` is the checkout
whose `data/` and `assets/` that exe reads (`CB_ROOT`): an old build runs with its own data. For
example:
    python3 tools/perf/perf_suite.py --out output/perf/today
    python3 tools/perf/perf_suite.py --scenarios world --repeats 3 \\
        --variant "current" --variant "no-shadows||CB_NO_SHADOWS=1"
    python3 tools/perf/perf_suite.py \\
        --variant "before|/path/old-target/release/crab-breeze||/path/old-checkout" --variant "after"

Budgets are the owner's (CLAUDE.md section 7): p95 within 8.3 ms (120 fps), p99 within 16.7 ms
(60 fps). Run from the repository root.
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path

SCENARIOS = {"world": ["--bench", "world"], "fight": ["--bench", "fight"], "defence": ["--bench", "defence"]}
DONE = "BENCH all"
WANT_MS = 1000 / 120
FLOOR_MS = 1000 / 60
ANSI = re.compile(r"\x1b\[[0-9;]*m")
# Anything that would share the machine with a run. In Pale Blue Dot a build in the background once
# made a walk read 37 ms at p50 that reads a fraction of that alone.
BUSY = ["crab-breeze", "cargo", "rustc"]
# Idle, these cost little; recording, they cost a lot. Warn rather than refuse.
WARN = ["obs"]


def pct(sorted_values: list[float], p: float) -> float:
    if not sorted_values:
        return math.nan
    return sorted_values[min(len(sorted_values) - 1, int((len(sorted_values) - 1) * p))]


def stats(values: list[float]) -> dict:
    v = sorted(x for x in values if not math.isnan(x))
    if not v:
        return {}
    return {
        "n": len(v),
        "mean": sum(v) / len(v),
        "p50": pct(v, 0.5),
        "p95": pct(v, 0.95),
        "p99": pct(v, 0.99),
        "p999": pct(v, 0.999),
        "max": v[-1],
        "over_want": sum(x > WANT_MS for x in v),
        "over_floor": sum(x > FLOOR_MS for x in v),
    }


def num(value: str | None) -> float:
    try:
        return float(value) if value not in (None, "") else math.nan
    except ValueError:
        return math.nan


def load_frames(path: Path) -> list[dict]:
    rows = []
    if not path.exists():
        return rows
    with open(path, newline="") as f:
        for r in csv.DictReader(f):
            t, ms = num(r.get("since_start_s")), num(r.get("wall_ms"))
            if math.isnan(t) or math.isnan(ms):
                continue  # a killed run's last row can be cut short
            rows.append({
                "t": t,
                "ms": ms,
                "beat": num(r.get("beat")),
                "entities": num(r.get("entities")),
                "render_cpu": num(r.get("render_cpu_ms")),
                "gpu_total": num(r.get("gpu_total_ms")),
            })
    return rows


def parse_variant(spec: str, default_exe: str) -> dict:
    parts = spec.split("|")
    env = {}
    if len(parts) > 2 and parts[2]:
        for pair in parts[2].split(";"):
            k, _, v = pair.partition("=")
            env[k.strip()] = v.strip()
    exe = parts[1] if len(parts) > 1 and parts[1] else default_exe
    root = parts[3] if len(parts) > 3 and parts[3] else None
    return {"name": parts[0], "exe": str(Path(exe).resolve()), "env": env, "root": root}


def running(name: str) -> bool:
    return subprocess.run(["pgrep", "-x", name], capture_output=True).returncode == 0


def rss_mb(pid: int) -> float:
    out = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    return int(out) / 1024 if out.isdigit() else math.nan


def run_once(scenario: str, variant: dict, index: int, a) -> dict:
    tag = f"{scenario}_{variant['name']}_{index}"
    csv_path = (a.out / f"{tag}.csv").resolve()
    log_path = a.out / f"{tag}.log"
    cmd = [variant["exe"], *SCENARIOS[scenario]]
    env = {**os.environ, **variant["env"], "CB_FRAME_LOG": str(csv_path), "CB_BENCH_NAME": f"suite-{tag}"}
    if variant.get("root"):
        env["CB_ROOT"] = variant["root"]
    print(f"[{dt.datetime.now():%H:%M:%S}] {tag}: {' '.join(cmd)} {variant['env'] or ''}", flush=True)
    started = time.time()
    memory = []
    with open(log_path, "w", encoding="utf-8", errors="replace") as log:
        proc = subprocess.Popen(cmd, stdout=log, stderr=subprocess.STDOUT, env=env, cwd=variant.get("root") or None)
        timed_out = False
        while proc.poll() is None:
            memory.append(rss_mb(proc.pid))
            if time.time() - started > a.timeout:
                proc.kill()
                timed_out = True
                break
            time.sleep(0.25)
        code = proc.wait()
    text = ANSI.sub("", log_path.read_text(encoding="utf-8", errors="replace"))
    rows = [r for r in load_frames(csv_path) if r["t"] >= a.skip_s]

    problems = []
    notes = []
    if timed_out:
        problems.append(f"timed out after {a.timeout} s")
    elif code != 0:
        problems.append(f"exit code {code}")
    if DONE not in text:
        problems.append(f"no `{DONE}` line")
    # A build from before the frame log has only its summary line (its own 10 s warm-up already
    # thrown away): totals, no traces.
    summary = re.search(r"BENCH all\s+frames=\s*(\d+) p50=\s*([\d.]+) p95=\s*([\d.]+) p99=\s*([\d.]+) max=\s*([\d.]+)", text)
    if not rows and summary:
        notes.append("no frame log (a build from before it): totals from its BENCH line")
    elif len(rows) < 300:
        problems.append(f"only {len(rows)} frames after the warm-up")
    passes = {m[1]: {"cpu_ms": float(m[2]), "gpu_ms": float(m[3])}
              for m in re.finditer(r"BENCHPASS (\S+) cpu_ms=([\d.]+) gpu_ms=([\d.]+)", text)}
    memory = [m for m in memory if not math.isnan(m)]
    result = {
        "scenario": scenario, "variant": variant["name"], "exe": variant["exe"], "env": variant["env"],
        "index": index, "seconds": round(time.time() - started, 1), "csv": str(csv_path), "log": str(log_path),
        "frame_ms": stats([r["ms"] for r in rows]),
        "render_cpu_ms": stats([r["render_cpu"] for r in rows]),
        "gpu_total_ms": stats([r["gpu_total"] for r in rows]),
        "entities": stats([r["entities"] for r in rows]),
        "memory_mb": {"peak": max(memory) if memory else math.nan,
                      "median": sorted(memory)[len(memory) // 2] if memory else math.nan},
        "passes": passes,
        "notes": notes,
    }
    if notes and summary:
        n, p50, p95, p99, mx = int(summary.group(1)), *map(float, summary.group(2, 3, 4, 5))
        result["frame_ms"] = {"n": n, "mean": p50, "p50": p50, "p95": p95, "p99": p99, "p999": math.nan, "max": mx}
    adapter = re.search(r'AdapterInfo \{ name: "([^"]+)".*?backend: (\w+)', text)
    if adapter:
        result["adapter"] = f"{adapter[1]} ({adapter[2]})"
    result["gpu_preprocessing"] = "GPU preprocessing is fully supported" in text
    result["valid"] = not problems
    result["problems"] = problems
    (a.out / f"{tag}.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    fm = result["frame_ms"]
    print(f"    {'VALID' if result['valid'] else 'INVALID ' + '; '.join(problems)}: "
          f"p50 {fm.get('p50', math.nan):.2f} p95 {fm.get('p95', math.nan):.2f} "
          f"p99 {fm.get('p99', math.nan):.2f} max {fm.get('max', math.nan):.1f} ms, "
          f"render CPU p50 {result['render_cpu_ms'].get('p50', math.nan):.2f} ms, "
          f"peak memory {result['memory_mb']['peak']:.0f} MB", flush=True)
    return result


def median(values: list[float]) -> float:
    v = sorted(x for x in values if not math.isnan(x))
    return v[len(v) // 2] if v else math.nan


def spread(values: list[float]) -> float:
    v = [x for x in values if not math.isnan(x)]
    return max(v) - min(v) if len(v) > 1 else math.nan


def fmt(x: float, want: float | None = None) -> str:
    if math.isnan(x):
        return "-"
    mark = " **over**" if want is not None and x > want else ""
    return f"{x:.2f}{mark}"


def report(results: list[dict], a, meta: dict) -> str:
    lines = [f"# Performance suite, {meta['when']}", ""]
    lines.append(f"- revision: `{meta['revision']}`{' (uncommitted changes)' if meta['dirty'] else ''}")
    lines.append(f"- adapter: {meta.get('adapter', 'unknown')}; offscreen 1920 x 1080, uncapped (`--bench`)")
    lines.append(f"- first {a.skip_s} s of each run thrown away; {a.repeats} run(s) per variant, interleaved")
    lines.append(f"- budgets: p95 within {WANT_MS:.1f} ms (120 fps), p99 within {FLOOR_MS:.1f} ms (60 fps); "
                 "**over** marks a miss. Each cell is the median over the valid runs; spread is the "
                 "largest minus the smallest p95 between them.")
    lines.append("")
    for scenario in a.scenarios:
        lines.append(f"## {scenario}")
        lines.append("")
        lines.append("| variant | valid runs | fps | p50 | p95 | p95 spread | p99 | p99.9 | max | render CPU p50 | GPU p50 | entities | peak memory MB |")
        lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
        for variant in a.variant_names:
            rs = [r for r in results if r["scenario"] == scenario and r["variant"] == variant]
            ok = [r for r in rs if r["valid"]]
            m = lambda key, sub="frame_ms", src=ok: median([r[sub].get(key, math.nan) for r in src])
            fps = 1000 / m("mean") if ok else math.nan
            lines.append(
                f"| {variant} | {len(ok)}/{len(rs)} | {fmt(fps)} | {fmt(m('p50'))} | {fmt(m('p95'), WANT_MS)} | "
                f"{fmt(spread([r['frame_ms'].get('p95', math.nan) for r in ok]))} | "
                f"{fmt(m('p99'), FLOOR_MS)} | {fmt(m('p999'))} | {fmt(m('max'))} | {fmt(m('p50', 'render_cpu_ms'))} | "
                f"{fmt(m('p50', 'gpu_total_ms'))} | {fmt(m('p50', 'entities'))} | {fmt(m('peak', 'memory_mb'))} |")
        for r in results:
            if r["scenario"] == scenario and r.get("notes"):
                lines.append(f"- {r['variant']} run {r['index']}: {'; '.join(r['notes'])} (its fps column is 1000 / p50)")
        bad = [r for r in results if r["scenario"] == scenario and not r["valid"]]
        for r in bad:
            lines.append(f"- invalid: {r['variant']} run {r['index']}: {'; '.join(r['problems'])}")
        # Where the render time goes, for each variant (means over the run, median over the runs).
        lines.append("")
        lines.append("Render passes, mean CPU ms per frame (GPU ms where the adapter reports it):")
        lines.append("")
        names = sorted({p for r in results if r["scenario"] == scenario for p in r.get("passes", {})})
        if names:
            lines.append("| pass | " + " | ".join(a.variant_names) + " |")
            lines.append("| --- | " + " | ".join("---:" for _ in a.variant_names) + " |")
            rows = []
            for name in names:
                cells = []
                for variant in a.variant_names:
                    ok = [r for r in results if r["scenario"] == scenario and r["variant"] == variant and r["valid"]]
                    cpu = median([r["passes"].get(name, {}).get("cpu_ms", math.nan) for r in ok])
                    gpu = median([r["passes"].get(name, {}).get("gpu_ms", math.nan) for r in ok])
                    cells.append((cpu, gpu))
                rows.append((max((c[0] for c in cells if not math.isnan(c[0])), default=0), name, cells))
            for _, name, cells in sorted(rows, reverse=True)[:15]:
                lines.append(f"| {name} | " + " | ".join(
                    fmt(c) + (f" ({g:.2f})" if g and not math.isnan(g) else "") for c, g in cells) + " |")
        lines.append(f"\n![{scenario}]({scenario}.png)\n")
    return "\n".join(lines)


def plot(results: list[dict], a) -> None:
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not installed: no graphs (the HTML report draws its own)")
        return
    colours = ["#3465a4", "#cc0000", "#4e9a06", "#75507b", "#c4a000", "#555753"]
    for scenario in a.scenarios:
        rs = [r for r in results if r["scenario"] == scenario]
        if not rs:
            continue
        fig, (ax, ax2) = plt.subplots(2, 1, figsize=(16, 8), sharex=True, gridspec_kw={"height_ratios": [3, 2]})
        for r in rs:
            c = colours[a.variant_names.index(r["variant"]) % len(colours)]
            rows = [x for x in load_frames(Path(r["csv"])) if x["t"] >= a.skip_s]
            t0 = rows[0]["t"] if rows else 0
            label = f"{r['variant']} #{r['index']}" + ("" if r["valid"] else " (invalid)")
            ax.plot([x["t"] - t0 for x in rows], [x["ms"] for x in rows], lw=0.5, color=c, alpha=0.7, label=label)
            ax2.plot([x["t"] - t0 for x in rows], [x["render_cpu"] for x in rows], lw=0.6, color=c, alpha=0.7)
        ax.axhline(WANT_MS, color="#4e9a06", ls="--", lw=0.8, label="120 fps")
        ax.axhline(FLOOR_MS, color="#cc0000", ls="--", lw=0.8, label="60 fps")
        ax.set_ylim(0, 40)
        ax.set_ylabel("frame wall time, ms")
        ax.set_title(f"{scenario}: frame time per run")
        ax.legend(loc="upper right", fontsize=8)
        ax2.set_ylabel("render passes CPU, ms")
        ax2.set_xlabel("seconds after the warm-up")
        fig.tight_layout()
        fig.savefig(a.out / f"{scenario}.png", dpi=100)
        plt.close(fig)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--scenarios", default="world,fight,defence")
    ap.add_argument("--variant", action="append", help="name|exe|ENV=1;ENV2=2|root (repeatable)")
    ap.add_argument("--exe", default="target/release/crab-breeze", help="the exe of a variant that names none")
    ap.add_argument("--repeats", type=int, default=2)
    ap.add_argument("--skip-s", type=float, default=10.0)
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--out", type=Path, default=Path("output/perf") / dt.datetime.now().strftime("%Y%m%d-%H%M"))
    a = ap.parse_args()
    a.scenarios = [s.strip() for s in a.scenarios.split(",") if s.strip()]
    for s in a.scenarios:
        if s not in SCENARIOS:
            raise SystemExit(f"unknown scenario {s}; known: {', '.join(SCENARIOS)}")
    variants = [parse_variant(v, a.exe) for v in (a.variant or ["current"])]
    a.variant_names = [v["name"] for v in variants]
    for v in variants:
        if not Path(v["exe"]).exists():
            raise SystemExit(f"{v['name']}: no exe at {v['exe']}")
    busy = [name for name in BUSY if running(name)]
    if busy:
        raise SystemExit(f"{', '.join(busy)} running: close it, it would share the machine with the runs")
    for name in WARN:
        if running(name):
            print(f"warning: {name} is running; make sure it is idle (not recording)", flush=True)
    a.out.mkdir(parents=True, exist_ok=True)

    meta = {
        "when": dt.datetime.now().strftime("%Y-%m-%d %H:%M"),
        "revision": subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip(),
        "dirty": bool(subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"],
                                     capture_output=True, text=True).stdout.strip()),
        "variants": variants,
        "args": sys.argv[1:],
        "machine": subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip(),
    }
    results = []
    for scenario in a.scenarios:
        for index in range(a.repeats):
            # A, B, then B, A: each variant goes first as often as it goes last.
            order = variants if index % 2 == 0 else variants[::-1]
            for variant in order:
                results.append(run_once(scenario, variant, index, a))
    adapters = {r["adapter"] for r in results if "adapter" in r}
    if adapters:
        meta["adapter"] = ", ".join(sorted(adapters))
    meta["gpu_preprocessing"] = sorted({r["gpu_preprocessing"] for r in results})
    (a.out / "suite.json").write_text(json.dumps({"meta": meta, "results": results}, indent=2), encoding="utf-8")
    plot(results, a)
    text = report(results, a, meta)
    (a.out / "report.md").write_text(text + "\n", encoding="utf-8")
    print()
    print(text)
    print(f"\nreport: {a.out / 'report.md'}")


if __name__ == "__main__":
    main()
