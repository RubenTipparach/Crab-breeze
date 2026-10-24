"""Turn a `perf_suite.py` run into one self-contained HTML report.

Ported from Pale Blue Dot's `tools/perf_report_html.py` (RubenTipparach/pale-blue-dot, revision
db25fe2); the cloud-GPU panel became the render passes' CPU time, and a table of where the render
time goes was added.

    python3 tools/perf/perf_report_html.py output/perf/2026-10-24-island \\
        --notes docs/benchmarks/2026-10-24-island/report.md \\
        --out docs/benchmarks/2026-10-24-island/report.html

Reads the run directory's `suite.json` and each run's frame-log CSV, and writes
a page with the per-scenario tables, the frame-time and render-CPU traces of
every run (binned to 0.1 s: the bin's worst frame and its median, so a spike
stays visible), the render passes, and the findings and limits sections of the notes Markdown.
Everything is inlined: the page opens from disk or as a published artifact.
"""
from __future__ import annotations

import argparse
import html
import json
import math
import re
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))  # perf_suite.py beside this file
from perf_suite import FLOOR_MS, WANT_MS, load_frames  # noqa: E402

BIN_S = 0.1
SKIP_S = 10.0


def median(values):
    v = [x for x in values if x is not None and not math.isnan(x)]
    return statistics.median(v) if v else None


def series(csv_path: Path, skip_s: float):
    rows = [r for r in load_frames(csv_path) if r["t"] >= skip_s]
    if not rows:
        return None
    t0 = rows[0]["t"]
    bins: dict[int, list[dict]] = {}
    for r in rows:
        bins.setdefault(int((r["t"] - t0) / BIN_S), []).append(r)
    out = {"t": [], "max": [], "p50": [], "gpu": []}
    for k in sorted(bins):
        b = bins[k]
        out["t"].append(round(k * BIN_S, 2))
        out["max"].append(round(max(r["ms"] for r in b), 2))
        out["p50"].append(round(statistics.median(r["ms"] for r in b), 2))
        g = median([r["render_cpu"] for r in b])
        out["gpu"].append(round(max(g, 0.0), 3) if g is not None else None)
    return out


def md_inline(text: str) -> str:
    text = html.escape(text)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)
    return text


def md_section(markdown: str, heading: str) -> str:
    """The list under `## heading`, as HTML (numbered or bulleted items)."""
    m = re.search(rf"^## {re.escape(heading)}\n(.*?)(?=^## |\Z)", markdown, re.S | re.M)
    if not m:
        return ""
    items, ordered = [], False
    for line in m[1].splitlines():
        start = re.match(r"^(\d+\.|-) (.*)", line)
        if start:
            ordered = start[1] != "-"
            items.append(start[2])
        elif line.startswith("  ") and items:
            items[-1] += " " + line.strip()
    tag = "ol" if ordered else "ul"
    return f"<{tag}>" + "".join(f"<li>{md_inline(i)}</li>" for i in items) + f"</{tag}>"


def summarise(results, scenario, variant, key="frame_ms"):
    rs = [r for r in results if r["scenario"] == scenario and r["variant"] == variant and r["valid"]]
    src = rs
    get = lambda sub, k: median([s.get(sub, {}).get(k, math.nan) for s in src])
    mean = get("frame_ms", "mean")
    return {
        "runs": f"{len(rs)}/{len([r for r in results if r['scenario'] == scenario and r['variant'] == variant])}",
        "fps": 1000 / mean if mean else None,
        **{k: get("frame_ms", k) for k in ("p50", "p95", "p99", "p999", "max")},
        "gpu_total": get("gpu_total_ms", "p50"),
        "gpu_clouds": get("render_cpu_ms", "p50"),
        "gpu_clouds95": get("render_cpu_ms", "p95"),
        "entities": get("entities", "p50"),
        "memory": median([s.get("memory_mb", {}).get("peak", math.nan) for s in src]),
    }


def passes(results, scenario, variants):
    """Per render pass, the median over the valid runs of its mean CPU ms, per variant."""
    names = sorted({p for r in results if r["scenario"] == scenario for p in r.get("passes", {})})
    rows = []
    for name in names:
        cells = []
        for v in variants:
            ok = [r for r in results if r["scenario"] == scenario and r["variant"] == v and r["valid"]]
            cells.append(median([r["passes"].get(name, {}).get("cpu_ms", math.nan) for r in ok]))
        rows.append({"name": name, "cpu": cells})
    rows.sort(key=lambda r: -max((c for c in r["cpu"] if c is not None), default=0))
    return rows[:15]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run_dir", type=Path)
    ap.add_argument("--notes", type=Path, help="Markdown with ## Findings and ## Limits")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--title", default="Crab Breeze frame budget")
    ap.add_argument("--eyebrow", default="Performance suite")
    ap.add_argument("--headline", help="the variant the summary tiles show (default: the first)")
    a = ap.parse_args()

    suite = json.loads((a.run_dir / "suite.json").read_text(encoding="utf-8"))
    meta, results = suite["meta"], suite["results"]
    scenarios = list(dict.fromkeys(r["scenario"] for r in results))
    variants = list(dict.fromkeys(r["variant"] for r in results))
    notes = a.notes.read_text(encoding="utf-8") if a.notes else ""

    headline = a.headline or variants[0]
    if headline not in variants:
        raise SystemExit(f"--headline {headline} is not one of {variants}")
    # The intro: the notes' first paragraph under their title.
    intro = ""
    m = re.search(r"^# .*?\n\n(.*?)\n\n", notes, re.S | re.M)
    if m:
        intro = md_inline(" ".join(m[1].split()))
    data = {"want": WANT_MS, "floor": FLOOR_MS, "variants": variants, "headline": headline,
            "scenarios": {}}
    for s in scenarios:
        runs = []
        for r in results:
            if r["scenario"] != s:
                continue
            csv_path = a.run_dir / Path(r["csv"]).name
            ser = series(csv_path, SKIP_S)
            if ser:
                runs.append({"variant": r["variant"], "index": r["index"], "valid": r["valid"], **ser})
        rows = [{"variant": v, **summarise(results, s, v)} for v in variants]
        data["scenarios"][s] = {"runs": runs, "rows": rows, "leg": [], "passes": passes(results, s, variants)}

    variant_env = {v["name"]: v.get("env") or {} for v in meta.get("variants", [])}
    page = TEMPLATE
    for key, value in {
        "TITLE": html.escape(a.title),
        "EYEBROW": html.escape(a.eyebrow),
        "INTRO": intro,
        "WHEN": html.escape(meta["when"]),
        "REVISION": html.escape(meta["revision"] + (" + uncommitted changes" if meta.get("dirty") else "")),
        "ADAPTER": html.escape(meta.get("adapter", "unknown GPU")),
        "FINDINGS": md_section(notes, "Findings"),
        "LIMITS": md_section(notes, "Limits"),
        "COMMAND": html.escape("python3 tools/perf/perf_suite.py " + " ".join(
            f'"{x}"' if " " in x or "|" in x else x for x in meta.get("args", []))),
        "VARIANTS": html.escape(json.dumps(variant_env)),
        "DATA": json.dumps(data, separators=(",", ":")).replace("</", "<\\/"),
    }.items():
        page = page.replace(f"%%{key}%%", value)
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(page, encoding="utf-8")
    print(f"wrote {a.out} ({a.out.stat().st_size / 1024:.0f} KB)")


TEMPLATE = r"""<title>%%TITLE%%</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Archivo:wdth,wght@75..100,500..800&family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
:root{
  --bg:#eef2f5; --panel:#ffffff; --ink:#122030; --muted:#566676; --rule:#d3dbe2;
  --on:#2a6fb4; --off:#c0692c; --want:#2f8a4a; --floor:#c23a31; --leg:rgba(42,111,180,.10);
  --grid:#e2e8ee; --chip:#e4ebf1;
  color-scheme:light;
}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){
  --bg:#0c1319; --panel:#131d26; --ink:#dde7ef; --muted:#8fa1b1; --rule:#26343f;
  --on:#6fa9e6; --off:#e89457; --want:#5cc27a; --floor:#f06a5f; --leg:rgba(111,169,230,.12);
  --grid:#1d2a34; --chip:#1c2934; color-scheme:dark;}}
:root[data-theme="dark"]{
  --bg:#0c1319; --panel:#131d26; --ink:#dde7ef; --muted:#8fa1b1; --rule:#26343f;
  --on:#6fa9e6; --off:#e89457; --want:#5cc27a; --floor:#f06a5f; --leg:rgba(111,169,230,.12);
  --grid:#1d2a34; --chip:#1c2934; color-scheme:dark;}
body{background:var(--bg);color:var(--ink);font:15px/1.6 "IBM Plex Sans",system-ui,sans-serif;padding-inline:16px;padding-block:28px 64px}
.wrap{max-width:1080px;margin:0 auto;display:grid;gap:40px}
h1,h2,h3{font-family:Archivo,"Arial Narrow",system-ui,sans-serif;font-stretch:80%;text-wrap:balance;margin:0;line-height:1.1}
h1{font-size:clamp(30px,5vw,46px);font-weight:800;letter-spacing:-.01em}
h2{font-size:26px;font-weight:700}
h3{font-size:15px;font-weight:600;font-stretch:100%;color:var(--muted);text-transform:uppercase;letter-spacing:.08em}
p{margin:0;max-width:68ch}
code,.mono{font-family:"IBM Plex Mono",ui-monospace,monospace;font-size:.88em}
code{background:var(--chip);padding:1px 5px;border-radius:4px}
header{display:grid;gap:14px}
.meta{display:flex;flex-wrap:wrap;gap:6px 18px;color:var(--muted);font-size:13px}
.meta span b{color:var(--ink);font-weight:500}
.strip{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px}
.tile{background:var(--panel);border:1px solid var(--rule);border-radius:8px;padding:14px 16px;display:grid;gap:4px}
.tile .name{font-size:12px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted)}
.tile .big{font-family:Archivo,sans-serif;font-stretch:80%;font-size:34px;font-weight:700;font-variant-numeric:tabular-nums;line-height:1}
.tile .big small{font-size:15px;color:var(--muted);font-weight:500;margin-left:4px}
.tile .sub{font-size:13px;color:var(--muted);font-variant-numeric:tabular-nums}
.pill{display:inline-block;font-size:11px;font-weight:600;letter-spacing:.04em;padding:2px 8px;border-radius:99px;text-transform:uppercase;justify-self:start}
.pill.ok{background:color-mix(in srgb,var(--want) 18%,transparent);color:var(--want)}
.pill.miss{background:color-mix(in srgb,var(--floor) 16%,transparent);color:var(--floor)}
section{display:grid;gap:16px}
ol,ul{margin:0;padding-left:22px;display:grid;gap:8px;max-width:76ch}
.scen{background:var(--panel);border:1px solid var(--rule);border-radius:10px;padding:20px;display:grid;gap:16px}
.scen header{display:flex;flex-wrap:wrap;align-items:baseline;gap:8px 16px}
.scen header p{color:var(--muted);font-size:14px}
.tablewrap{overflow-x:auto}
table{border-collapse:collapse;width:100%;font-size:13.5px;font-variant-numeric:tabular-nums}
th,td{padding:6px 10px;text-align:right;border-bottom:1px solid var(--rule);white-space:nowrap}
th:first-child,td:first-child{text-align:left}
th{font-weight:600;color:var(--muted);font-size:12px}
td.over{color:var(--floor);font-weight:600}
.swatch{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:6px;vertical-align:0}
.caption{font-size:12.5px;color:var(--muted)}
.chart{width:100%;height:auto;display:block}
.chart text{fill:var(--muted);font:11px "IBM Plex Mono",monospace}
.legend{display:flex;flex-wrap:wrap;gap:4px 16px;font-size:12.5px;color:var(--muted)}
.legend i{display:inline-block;width:18px;height:0;border-top:2px solid;margin-right:6px;vertical-align:3px}
pre{background:var(--panel);border:1px solid var(--rule);border-radius:8px;padding:12px 14px;overflow-x:auto;margin:0;font:13px/1.5 "IBM Plex Mono",monospace}
.foot{display:grid;gap:12px}
</style>

<div class="wrap">
<header>
  <h3>%%EYEBROW%%</h3>
  <h1>%%TITLE%%</h1>
  <p>%%INTRO%%</p>
  <p>The owner's targets are 120 fps (8.3 ms) at the 95th percentile and never below 60 fps (16.7 ms) at the 99th.</p>
  <div class="meta">
    <span>Run <b>%%WHEN%%</b></span><span>Build <b class="mono">%%REVISION%%</b></span>
    <span>GPU <b>%%ADAPTER%%</b></span><span>View <b>offscreen 1920 × 1080, uncapped</b></span>
  </div>
</header>

<div class="strip" id="strip"></div>

<section>
  <h2>Findings</h2>
  %%FINDINGS%%
</section>

<div id="scenarios" style="display:grid;gap:24px"></div>

<section class="foot">
  <h2>How this was measured</h2>
  %%LIMITS%%
  <p>Each run starts the game fresh, plays the scripted take with the pilot's view rendered offscreen, and throws away its first 10 s. A run that crashed, timed out or never finished its take is marked invalid and left out. Traces are binned to 0.1 s: the faint line is each bin's worst frame, the solid line its median. Reproduce with:</p>
  <pre>%%COMMAND%%</pre>
</section>
</div>

<script>
const DATA=%%DATA%%;
const VARIANT_ENV=JSON.parse(`%%VARIANTS%%`.replace(/&quot;/g,'"'));
const NAMES={world:"The world take",fight:"The city fight",defence:"The city defence"};
const BLURB={
  world:"`--bench world`: walk to the mech and board, stride the street, rockets and skates, the river, the wood, shells and damage.",
  fight:"`--bench fight`: the player's mech against the raiders in a city, guns, debris and collapsing buildings.",
  defence:"`--bench defence`: the combat test, defending a city against waves of enemies in three fights."};
const PALETTE=["var(--on)","var(--off)","var(--want)","#8a63c8","#b8952a"];
const colour=v=>PALETTE[Math.max(0,DATA.variants.indexOf(v))%PALETTE.length];
const f=(x,d=2)=>x==null||isNaN(x)?"-":Number(x).toFixed(d);
const esc=s=>String(s).replace(/[&<>]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;"}[c]));
const md=s=>esc(s).replace(/`([^`]+)`/g,"<code>$1</code>");

// Summary strip: the headline variant per scenario.
const strip=document.getElementById("strip");
for(const [s,sc] of Object.entries(DATA.scenarios)){
  const r=sc.rows.find(x=>x.variant===DATA.headline)||sc.rows[0];
  if(r.fps==null) continue;
  const ok=r.p95<=DATA.want && r.p99<=DATA.floor;
  strip.insertAdjacentHTML("beforeend",`<div class="tile">
    <span class="name">${esc(NAMES[s]||s)} · ${esc(DATA.headline)}</span>
    <span class="big">${f(r.fps,0)}<small>fps</small></span>
    <span class="sub">p95 ${f(r.p95,1)} ms · p99 ${f(r.p99,1)} ms</span>
    <span class="pill ${ok?"ok":"miss"}">${ok?"meets 120 fps":"misses 120 fps at p95"}</span></div>`);
}

function table(rows,leg){
  const head=`<tr><th>Variant</th>${leg?"":"<th>Valid runs</th>"}<th>fps</th><th>p50 ms</th><th>p95</th><th>p99</th>${leg?"":"<th>p99.9</th>"}<th>max</th><th>GPU frame p50</th><th>render CPU p50 / p95</th>${leg?"":"<th>entities</th><th>peak MB</th>"}</tr>`;
  const body=rows.map(r=>`<tr><td><span class="swatch" style="background:${colour(r.variant)}"></span>${esc(r.variant)}${Object.keys(VARIANT_ENV[r.variant]||{}).length?` <span class="caption mono">${esc(Object.entries(VARIANT_ENV[r.variant]).map(([k,v])=>k+"="+v).join(" "))}</span>`:""}</td>
    ${leg?"":`<td>${r.runs}</td>`}<td>${f(r.fps,0)}</td><td>${f(r.p50)}</td>
    <td class="${r.p95>DATA.want?"over":""}">${f(r.p95)}</td><td class="${r.p99>DATA.floor?"over":""}">${f(r.p99)}</td>
    ${leg?"":`<td>${f(r.p999)}</td>`}<td>${f(r.max,1)}</td><td>${f(r.gpu_total)}</td><td>${f(r.gpu_clouds)} / ${f(r.gpu_clouds95)}</td>${leg?"":`<td>${f(r.entities,0)}</td><td>${f(r.memory,0)}</td>`}</tr>`).join("");
  return `<div class="tablewrap"><table>${head}${body}</table></div>`;
}

function chart(sc){
  const W=1000,H=300,G=120,L=44,R=10,T=10,B=26,gap=34;
  const tmax=Math.max(...sc.runs.map(r=>r.t[r.t.length-1]||0));
  const top=40, gtop=Math.max(1,...sc.runs.flatMap(r=>r.gpu.filter(x=>x!=null)))*1.1;
  const x=t=>L+(W-L-R)*t/tmax, y=ms=>T+(H-T-B)*(1-Math.min(ms,top)/top);
  const gy0=H+gap, gy=v=>gy0+G*(1-v/gtop);
  let s=`<svg class="chart" viewBox="0 0 ${W} ${H+gap+G+B}" role="img" aria-label="frame time traces">`;
  const leg=sc.runs.find(r=>r.leg);
  if(leg) s+=`<rect x="${x(leg.leg[0])}" y="${T}" width="${x(leg.leg[1])-x(leg.leg[0])}" height="${H-T-B+gap+G}" fill="var(--leg)"/>`+
    `<text x="${x(leg.leg[0])+6}" y="${T+12}">cloud leg</text>`;
  for(const ms of [0,10,20,30,40]) s+=`<line x1="${L}" x2="${W-R}" y1="${y(ms)}" y2="${y(ms)}" stroke="var(--grid)"/><text x="${L-6}" y="${y(ms)+4}" text-anchor="end">${ms}</text>`;
  for(let t=0;t<=tmax;t+=10) s+=`<text x="${x(t)}" y="${gy0+G+18}" text-anchor="middle">${t}s</text>`;
  s+=`<line x1="${L}" x2="${W-R}" y1="${y(DATA.want)}" y2="${y(DATA.want)}" stroke="var(--want)" stroke-dasharray="5 4"/>`;
  s+=`<line x1="${L}" x2="${W-R}" y1="${y(DATA.floor)}" y2="${y(DATA.floor)}" stroke="var(--floor)" stroke-dasharray="5 4"/>`;
  s+=`<text x="${W-R-4}" y="${y(DATA.want)-4}" text-anchor="end" style="fill:var(--want)">120 fps</text><text x="${W-R-4}" y="${y(DATA.floor)-4}" text-anchor="end" style="fill:var(--floor)">60 fps</text>`;
  s+=`<text x="${L-6}" y="${T-0}" text-anchor="end">ms</text>`;
  const path=(ts,vs,fy)=>ts.map((t,i)=>vs[i]==null?"":`${x(t).toFixed(1)},${fy(vs[i]).toFixed(1)}`).filter(Boolean).join(" ");
  for(const r of sc.runs){
    const c=colour(r.variant), dash=r.index%2?' stroke-dasharray="4 2"':"";
    s+=`<polyline points="${path(r.t,r.max,y)}" fill="none" stroke="${c}" stroke-opacity=".35" stroke-width="1"${dash}/>`;
    s+=`<polyline points="${path(r.t,r.p50,y)}" fill="none" stroke="${c}" stroke-width="1.6"${dash}/>`;
  }
  for(const v of [0,gtop/2]) s+=`<line x1="${L}" x2="${W-R}" y1="${gy(v)}" y2="${gy(v)}" stroke="var(--grid)"/><text x="${L-6}" y="${gy(v)+4}" text-anchor="end">${v.toFixed(1)}</text>`;
  s+=`<text x="${L}" y="${gy0-8}">render passes, CPU ms (Metal reports no GPU time)</text>`;
  for(const r of sc.runs)
    s+=`<polyline points="${path(r.t,r.gpu,gy)}" fill="none" stroke="${colour(r.variant)}" stroke-width="1.4"${r.index%2?' stroke-dasharray="4 2"':""}/>`;
  return s+"</svg>";
}

const root=document.getElementById("scenarios");
for(const [s,sc] of Object.entries(DATA.scenarios)){
  root.insertAdjacentHTML("beforeend",`<article class="scen">
    <header><h2>${esc(NAMES[s]||s)}</h2><span class="mono caption">${esc(s)}</span><p>${md(BLURB[s]||"")}</p></header>
    ${table(sc.rows,false)}
    ${sc.passes&&sc.passes.length?`<h3>Where the render time goes (CPU ms per frame, by pass)</h3><div class="tablewrap"><table><tr><th>Pass</th>${DATA.variants.map(v=>`<th>${esc(v)}</th>`).join("")}</tr>${sc.passes.map(p=>`<tr><td class="mono">${esc(p.name)}</td>${p.cpu.map(c=>`<td>${f(c)}</td>`).join("")}</tr>`).join("")}</table></div>`:""}
    <div class="legend">${DATA.variants.map(v=>`<span><i style="border-color:${colour(v)}"></i>${esc(v)}</span>`).join("")}<span><i style="border-color:var(--muted);border-top-style:dashed"></i>second run</span><span>faint: worst frame per 0.1 s · solid: median</span></div>
    ${chart(sc)}
  </article>`);
}
</script>
"""

if __name__ == "__main__":
    main()
