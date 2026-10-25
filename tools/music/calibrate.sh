#!/usr/bin/env bash
# Level every General MIDI instrument the scores use, so a score's gain values
# mean the same thing on every instrument. Each instrument plays one phrase at
# velocity 100 through LMMS at track volume 100; the RMS level is measured and
# music/instruments.json mix.<name>.vol is set so the instrument sits at
# TARGET dBFS (pans are kept). Rerun after changing the soundfont.
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
for i in $(grep -oh 's("gm_[a-z0-9_]*")' "$REPO"/music/*.strudel | sed 's/s("//; s/")//' | sort -u); do
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
python3 - "$TMP/levels.txt" "$REPO/music/instruments.json" "$TARGET" "${1:-}" <<'PY'
import json, sys
levels, path, target, write = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4] == "--write"
inst = json.load(open(path))
print(f"{'instrument':26} {'dBFS@100':>9} {'vol':>5}")
for line in open(levels):
    name, rms = line.split()
    vol = max(30, min(200, round(100 * 10 ** ((target - float(rms)) / 20))))
    print(f"{name:26} {float(rms):9.1f} {vol:5d}")
    inst["mix"].setdefault(name, {})["vol"] = vol
if write:
    inst["mix"] = dict(sorted(inst["mix"].items()))
    with open(path, "w") as f:
        json.dump(inst, f, indent=2)
        f.write("\n")
    print(f"wrote {path}")
PY
