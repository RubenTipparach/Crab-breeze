#!/usr/bin/env bash
# Level every instrument the scores use, so a score's gain values mean the same
# thing on every instrument. Each General MIDI instrument, and each LMMS preset a
# score or instruments.json voices with @voice, plays one phrase at velocity 100
# through LMMS at track volume 100; the RMS level is measured and
# music/instruments.json mix.<name>.vol (soundfont) or preset_vol.<preset>
# (LMMS presets) is set so it sits at TARGET dBFS (pans are kept). Rerun after
# changing the soundfont or adding a voice.
#
# Usage: tools/music/calibrate.sh [--write]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
TARGET=-30
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
ROOT=""; [ "$(id -u)" = 0 ] && ROOT="--allowroot"
: > "$TMP/levels.txt"
VOICED="$(python3 -c 'import json,sys;print(" ".join(json.load(open(sys.argv[1])).get("voices",{})))' "$REPO/music/instruments.json")"
for i in $(grep -oh 's("gm_[a-z0-9_]*")' "$REPO"/music/*.strudel | sed 's/s("//; s/")//' | sort -u); do
  # a GM name with a default voice plays an LMMS preset: it is measured below, as the preset
  case " $VOICED " in *" $i "*) continue ;; esac
  case "$i" in
    *contrabass*|*tuba*|*bass*|*timpani*|*cello*|*trombone*|*taiko*) NOTES="d2 a2 f2 d3" ;;
    *) NOTES="d4 a4 f4 d5" ;;
  esac
  printf '// @title cal\n// @cycles 2\nsetcps(120/60/4)\nnote("[%s]*2").s("%s")\n' "$NOTES" "$i" > "$TMP/$i.strudel"
  # measure at vol 100: a // @mix line overrides the table for this file only
  sed -i "1a // @mix $i vol=100" "$TMP/$i.strudel"
  node "$HERE/strudel_to_lmms.mjs" "$TMP/$i.strudel" "$TMP/$i.mmp" >/dev/null 2>&1
  xvfb-run -a lmms $ROOT render "$TMP/$i.mmp" -o "$TMP/$i.wav" -f wav -s 44100 >/dev/null 2>&1
  RMS="$(ffmpeg -hide_banner -i "$TMP/$i.wav" -af astats=metadata=0 -f null - 2>&1 | grep 'RMS level dB' | tail -1 | awk '{print $NF}')"
  echo "$i $RMS" >> "$TMP/levels.txt"
done
# the LMMS presets: every @voice line in the scores and every default voice
PRESETS="$( (grep -ohE '^// *@(voice|layer) [^ ]* [^ ]*\.xpf' "$REPO"/music/*.strudel | awk '{print $NF}';
  python3 -c 'import json,sys;[print(v) for v in json.load(open(sys.argv[1])).get("voices",{}).values()]' "$REPO/music/instruments.json") | sort -u)"
for p in $PRESETS; do
  case "$p" in *Bass*|*bass*|*Reece*|LB302/*) NOTES="d2 a2 f2 d3" ;; *) NOTES="d4 a4 f4 d5" ;; esac
  id="preset_$(echo "$p" | tr '/.' '__')"
  printf '// @title cal\n// @cycles 2\n// @voice gm_acoustic_piano %s\n// @mix gm_acoustic_piano vol=100\nsetcps(120/60/4)\nnote("[%s]*2").s("gm_acoustic_piano")\n' "$p" "$NOTES" > "$TMP/$id.strudel"
  node "$HERE/strudel_to_lmms.mjs" "$TMP/$id.strudel" "$TMP/$id.mmp" >/dev/null 2>&1
  xvfb-run -a lmms $ROOT render "$TMP/$id.mmp" -o "$TMP/$id.wav" -f wav -s 44100 >/dev/null 2>&1
  RMS="$(ffmpeg -hide_banner -i "$TMP/$id.wav" -af astats=metadata=0 -f null - 2>&1 | grep 'RMS level dB' | tail -1 | awk '{print $NF}')"
  # and its pitch: A3 held for a bar, measured by tools/music/pitch.py
  printf '// @title cal\n// @cycles 2\n// @voice gm_acoustic_piano %s\nsetcps(120/60/4)\nnote("a3@1").s("gm_acoustic_piano")\n' "$p" > "$TMP/${id}_a3.strudel"
  CB_RAW_PITCH=1 node "$HERE/strudel_to_lmms.mjs" "$TMP/${id}_a3.strudel" "$TMP/${id}_a3.mmp" >/dev/null 2>&1
  xvfb-run -a lmms $ROOT render "$TMP/${id}_a3.mmp" -o "$TMP/${id}_a3.wav" -f wav -s 44100 >/dev/null 2>&1
  OFF="$(python3 "$HERE/pitch.py" "$TMP/${id}_a3.wav" 220 | tr ' ' '_')"
  echo "preset:$p $RMS $OFF" >> "$TMP/levels.txt"
done
python3 - "$TMP/levels.txt" "$REPO/music/instruments.json" "$TARGET" "${1:-}" <<'PY'
import json, sys
levels, path, target, write = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4] == "--write"
inst = json.load(open(path))
print(f"{'instrument':40} {'dBFS@100':>9} {'vol':>6}")
for line in open(levels):
    name, rms, *pitch = line.split()
    vol = max(1.0, min(200.0, round(100 * 10 ** ((target - float(rms)) / 20), 1)))
    print(f"{name:40} {float(rms):9.1f} {vol:6.1f}")
    if name.startswith("preset:"):
        key = name[len("preset:"):]
        inst.setdefault("preset_vol", {})[key] = vol
        off = pitch[0] if pitch else "?"
        if off.lstrip("-").isdigit():
            inst.setdefault("preset_offset_st", {})[key] = int(off)
            print(f"{'':40} sounds {int(off):+d} semitones from its key")
        else:
            print(f"{'':40} WARN pitch unclear ({off}): offset left as it was")
    else:
        inst["mix"].setdefault(name, {})["vol"] = vol
if write:
    inst["mix"] = dict(sorted(inst["mix"].items()))
    inst["preset_vol"] = dict(sorted(inst.get("preset_vol", {}).items()))
    inst["preset_offset_st"] = dict(sorted(inst.get("preset_offset_st", {}).items()))
    with open(path, "w") as f:
        json.dump(inst, f, indent=2)
        f.write("\n")
    print(f"wrote {path}")
PY
