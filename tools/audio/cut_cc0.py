#!/usr/bin/env python3
"""Cut Crab Breeze's recorded sound effects from CC0 and public domain originals.

tools/audio/cc0_cues.json says, for each recorded cue, which original files to layer and how
(start, length, which shot of a multi-shot take, pitch as a tape-speed ratio, gain, steep lowpass
and highpass, fades, delay), and may add a synthesised layer from tools/audio/gen_sfx.py where a
recording lacks weight. This script applies it over the synthesised set gen_sfx.py wrote: the cue's
WAV in --out is replaced, its sfx.json entry updated, and assets/audio/sfx/SOURCES.md rewritten
with one row per cue naming every original it is cut from.

The originals are fetched by tools/audio/fetch_cc0.py into --src and are not committed; the cuts
are. A cue whose originals are missing is left synthesised and reported.

Usage: python3 tools/audio/cut_cc0.py [--src DIR] [--out DIR] [cue ...]
"""
import argparse
import json
import os
import subprocess
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import gen_sfx as g  # noqa: E402  (the shared filters, writer and synthesised cues)

SR = g.SR


def find(src_root: str, ref: str) -> str:
    """'pack:file' resolved to a path: the file is searched for anywhere under the pack's folder,
    so a pack's own zip layout does not leak into the spec."""
    pack, _, name = ref.partition(":")
    base = os.path.join(src_root, pack)
    for root, _, files in os.walk(base):
        if name in files:
            return os.path.join(root, name)
    raise FileNotFoundError(f"{ref} (looked under {base})")


def load(path: str, channels: int = 1) -> np.ndarray:
    """The file as float samples, shape (channels, n): a mono file is copied to both sides."""
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-ac", str(channels), "-ar", str(SR), "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(raw, np.float32).astype(np.float64).reshape(-1, channels).T


def shot_start(x: np.ndarray, k: int, gap_s: float = 0.25) -> float:
    """Where the k-th shot (from 1) of a multi-shot take begins: a rise above a fifth of the take's
    peak after at least gap_s below it."""
    env = np.convolve(np.abs(x), np.ones(64) / 64, "same")
    hot = env > env.max() * 0.2
    starts, last = [], -10 * SR
    for i in np.flatnonzero(hot):
        if i - last > gap_s * SR:
            starts.append(i)
        last = i
    if k > len(starts):
        raise ValueError(f"asked for shot {k}, the take has {len(starts)}")
    return max(0, starts[k - 1] - int(0.005 * SR)) / SR


def varispeed(x: np.ndarray, ratio: float) -> np.ndarray:
    """Tape-style pitch: ratio below 1 is lower and longer, as a bigger machine would sound."""
    if ratio == 1:
        return x
    n = int(x.shape[-1] / ratio)
    return np.stack([np.interp(np.arange(n) * ratio, np.arange(x.shape[-1]), c) for c in x])


def fades(x: np.ndarray, fin: float, fout: float) -> np.ndarray:
    x = x.copy()
    n = x.shape[-1]
    a, b = min(n, int(fin * SR)), min(n, int(fout * SR))
    if a:
        x[..., :a] *= np.linspace(0, 1, a)
    if b:
        x[..., -b:] *= np.linspace(1, 0, b) ** 2
    return x


def per_channel(fn, x: np.ndarray, *a, **k) -> np.ndarray:
    return np.stack([fn(c, *a, **k) for c in x])


def fit(x: np.ndarray, n: int) -> np.ndarray:
    return np.pad(x, ((0, 0), (0, max(0, n - x.shape[-1]))))[:, :n]


def layer(spec: dict, src_root: str, channels: int) -> np.ndarray:
    """One layer as (channels, n). pan (-1 left to 1 right, equal power) places it in a stereo cue."""
    if "synth" in spec:
        group, what, fn, _ = g.SOUNDS[spec["synth"]]
        x = np.repeat(fn(g.rng_for(spec["synth"]))[None, :], channels, axis=0)
    else:
        x = load(find(src_root, spec["src"]), channels)
        start = shot_start(x.mean(axis=0), spec["shot"]) + spec.get("start", 0.0) if "shot" in spec else spec.get("start", 0.0)
        x = x[:, int(start * SR):]
        x = varispeed(x, spec.get("pitch", 1.0))
        if spec.get("repeat", 1) > 1:
            x = np.tile(x, (1, spec["repeat"]))
    if "dur" in spec:
        x = x[:, : int(spec["dur"] * SR)]
    if "hp" in spec:
        x = per_channel(g.hp, x, spec["hp"])
    if "lp" in spec:
        x = per_channel(g.lp, x, spec["lp"], order=spec.get("lp_order", 4))
    x = fades(x, spec.get("fadein", 0.003), spec.get("fadeout", 0.01))
    x = x / (np.max(np.abs(x)) or 1.0) * 10 ** (spec.get("gain", 0.0) / 20)
    if channels == 2 and "pan" in spec:
        a = (spec["pan"] + 1) * np.pi / 4
        x = x * np.array([[np.cos(a)], [np.sin(a)]]) * np.sqrt(2)
    return np.concatenate([np.zeros((channels, int(spec.get("delay", 0.0) * SR))), x], axis=1)


def cut(cue: dict, src_root: str) -> np.ndarray:
    """The cue as (channels, n): stereo for an ambience bed (cue "stereo": true), mono otherwise."""
    ch = 2 if cue.get("stereo") else 1
    parts = [layer(s, src_root, ch) for s in cue["layers"]]
    n = cue.get("len") and int(cue["len"] * SR) or max(p.shape[-1] for p in parts)
    x = sum(fit(p, n) for p in parts)
    if "drive" in cue:
        x = g.shape(x / (np.max(np.abs(x)) or 1.0), cue["drive"])
    x = per_channel(g.hp, x, cue.get("hp", 20))
    if cue.get("loop"):
        x = per_channel(g.loopable, x, cue.get("loop_fade", 0.3))
    else:
        x = fades(x, 0.0, cue.get("tail_fade", 0.05))
    return x


def credit(ref: str, licences: dict) -> str:
    pack, _, name = ref.partition(":")
    info = licences.get(pack, {})
    who = f" ({info['author']})" if info.get("author") else ""
    title = pack.split("/", 1)[1]
    lic = "public domain" if any("publicdomain" in s for s in info.get("licences", [])) else "CC0"
    return f"[{title}]({info.get('url', '')}){who} `{name}`, {lic}"


def rows_for(names, cues: dict, licences: dict) -> list:
    rows = []
    for name in names:
        if name in cues and cues[name].get("_cut"):
            srcs = []
            for s in cues[name]["layers"]:
                srcs.append(f"our `{s['synth']}` synthesis" if "synth" in s else credit(s["src"], licences))
            edits = cues[name].get("edits", "trimmed, layered, filtered, levelled")
            rows.append(f"| {name} | {'; '.join(dict.fromkeys(srcs))} | {edits} |")
        else:
            rows.append(f"| {name} | `tools/audio/gen_sfx.py` (synthesised, ours) | none |")
    return rows


HEADER = """Recorded cues are cut from CC0 and public domain originals by `tools/audio/cut_cc0.py` to the spec in
`tools/audio/cc0_cues.json` (originals fetched by `tools/audio/fetch_cc0.py`, which refuses a pack
whose page does not say CC0 or public domain); the rest are synthesised by `tools/audio/gen_sfx.py`.
Rebuild all of them with `tools/audio/render_sfx.sh`. This file is written by `cut_cc0.py`: edit the
spec, not this table. CC0 asks for no credit; the people who recorded the originals are named anyway.
Candidates and what was looked at: `docs/analysis/audio-sources.md`.

| Cue | Made from | Edits |
| --- | --- | --- |
"""


def write_sources(cues: dict, licences: dict) -> None:
    named = {n for n in cues if not n.startswith("_")}
    amb = sorted(n for n in named if cues[n].get("group") == "Ambience")
    sfx = sorted((set(g.SOUNDS) | named) - set(amb))
    for title, names, folder in (("Sound effect sources", sfx, "sfx"), ("Ambience sources", amb, "ambience")):
        os.makedirs(os.path.join(REPO, "assets/audio", folder), exist_ok=True)
        with open(os.path.join(REPO, "assets/audio", folder, "SOURCES.md"), "w") as f:
            f.write(f"# {title}\n\n" + HEADER + "\n".join(rows_for(names, cues, licences)) + "\n")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--src", default="/opt/sfx-src")
    ap.add_argument("--out", default=os.path.join(REPO, "output/sfx"))
    ap.add_argument("cues", nargs="*")
    args = ap.parse_args()
    with open(os.path.join(HERE, "cc0_cues.json")) as f:
        cues = json.load(f)
    lic_path = os.path.join(args.src, "licences.json")
    licences = json.load(open(lic_path)) if os.path.exists(lic_path) else {}
    man_path = os.path.join(args.out, "sfx.json")
    manifest = {m["name"]: m for m in json.load(open(man_path))}
    status = 0
    for name, cue in cues.items():
        if name.startswith("_") or (args.cues and name not in args.cues):
            continue
        if name not in g.SOUNDS and not ("group" in cue and "what" in cue):
            print(f"{name}: not a cue gen_sfx.py knows, and it names no group and what", file=sys.stderr)
            status = 1
            continue
        try:
            x = cut(cue, args.src)
        except (FileNotFoundError, ValueError, subprocess.CalledProcessError) as e:
            print(f"{name:18} left synthesised: {e}")
            continue
        if not np.all(np.isfinite(x)):
            print(f"{name}: produced a non-finite sample", file=sys.stderr)
            return 1
        cue["_cut"] = True
        known = g.SOUNDS.get(name)
        peak_db = cue.get("peak_db", known[3] if known else -1.0)
        g.write_wav(os.path.join(args.out, f"{name}.wav"), x if x.shape[0] == 2 else x[0], peak_db,
                    loop=bool(cue.get("loop")))
        mono = np.abs(x).max(axis=0)
        peaks = mono[: len(mono) // 120 * 120].reshape(120, -1).max(axis=1)
        m = manifest.setdefault(name, {"name": name})
        m.update({"group": cue.get("group", known[0] if known else "Other"),
                  "what": cue.get("what", known[1] if known else ""), "seconds": round(x.shape[-1] / SR, 2),
                  "loop": bool(cue.get("loop")), "peaks": [round(float(p), 3) for p in peaks / (peaks.max() or 1)],
                  "recorded": True, "stereo": x.shape[0] == 2})
        print(f"{name:18} {x.shape[-1] / SR:4.1f} s  recorded{' stereo' if x.shape[0] == 2 else ''}")
    # cues listed but not cut this run keep a row only if cut earlier: rebuild every row from disk state
    for name, cue in cues.items():
        if not name.startswith("_") and manifest.get(name, {}).get("recorded"):
            cue["_cut"] = True
    with open(man_path, "w") as f:
        json.dump(list(manifest.values()), f)
    write_sources(cues, licences)
    return status


if __name__ == "__main__":
    sys.exit(main())
