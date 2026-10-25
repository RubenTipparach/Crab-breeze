#!/usr/bin/env bash
# Render a Crab Breeze score: .strudel -> .mid + .mmp (beside the score) -> LMMS
# headless -> loudness-normalised .ogg (the game's copy, assets/audio/music/) and
# an .mp3 share copy (output/music/). The skill music-authoring explains each step.
#
# Usage: tools/music/render.sh music/<name>.strudel [--preview]
#   --preview  render the .mid through fluidsynth instead of LMMS (seconds, rough)
#
# Needs: node 20+, `npm install` in tools/music, and
#   apt-get install --no-install-recommends lmms fluidsynth fluid-soundfont-gm ffmpeg xvfb
# Adapted from Tom Lander tools/strudel/render_midi.sh and audio.md (dea3d50).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
SCORE="$(realpath "${1:?usage: render.sh music/<name>.strudel [--preview]}")"
MODE="${2:-lmms}"
NAME="$(basename "$SCORE" .strudel)"
DIR="$(dirname "$SCORE")"
OGG_DIR="$REPO/assets/audio/music"
MP3_DIR="$REPO/output/music"
mkdir -p "$OGG_DIR" "$MP3_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

[ -d "$HERE/node_modules/@strudel/core" ] || (cd "$HERE" && npm install --silent)
[ -f /opt/soundfonts/GeneralUser-GS.sf2 ] || "$HERE/fetch_soundfonts.sh"
node "$HERE/strudel_to_midi.mjs" "$SCORE" "$DIR/$NAME.mid" 2>/dev/null
node "$HERE/strudel_to_lmms.mjs" "$SCORE" "$DIR/$NAME.mmp" 2>/dev/null | tail -1

SF2="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["soundfont"])' "$REPO/music/instruments.json")"
if [ "$MODE" = "--preview" ]; then
  fluidsynth -ni -g 0.6 -F "$TMP/raw.wav" -r 44100 "$SF2" "$DIR/$NAME.mid" >/dev/null 2>&1
  OUT_MP3="$MP3_DIR/${NAME}_preview.mp3"
else
  # LMMS refuses root without --allowroot and always wants a display
  ROOT=""; [ "$(id -u)" = 0 ] && ROOT="--allowroot"
  xvfb-run -a lmms $ROOT render "$DIR/$NAME.mmp" -o "$TMP/raw.wav" -f wav -s 44100 -b 320 -i sincbest >/dev/null 2>&1
  OUT_MP3="$MP3_DIR/$NAME.mp3"
fi
[ -s "$TMP/raw.wav" ] || { echo "render produced no audio" >&2; exit 1; }

# clipping in the raw mix means the comp is too hot: lower vol in the score's @mix lines
python3 "$HERE/check_mix.py" "$TMP/raw.wav"

# one loudness for every cue: -16 LUFS integrated, -1.5 dBTP (two-pass loudnorm)
STATS="$(ffmpeg -hide_banner -nostats -i "$TMP/raw.wav" -af loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json -f null - 2>&1 | sed -n '/^{/,/^}/p')"
get() { python3 -c 'import json,sys;print(json.loads(sys.stdin.read())[sys.argv[1]])' "$1" <<<"$STATS"; }
AF="loudnorm=I=-16:TP=-1.5:LRA=11:measured_I=$(get input_i):measured_TP=$(get input_tp):measured_LRA=$(get input_lra):measured_thresh=$(get input_thresh):offset=$(get target_offset):linear=true"
ffmpeg -hide_banner -loglevel error -y -i "$TMP/raw.wav" -af "$AF" -ar 44100 "$TMP/norm.wav"
TITLE="$(sed -n 's#^// *@title *##p' "$SCORE" | head -1)"
ffmpeg -hide_banner -loglevel error -y -i "$TMP/norm.wav" -c:a libmp3lame -b:a 192k \
  -metadata title="$TITLE" -metadata artist="Crab Breeze OST" "$OUT_MP3"
if [ "$MODE" != "--preview" ]; then
  ffmpeg -hide_banner -loglevel error -y -i "$TMP/norm.wav" -c:a libvorbis -q:a 6 \
    -metadata title="$TITLE" -metadata artist="Crab Breeze OST" "$OGG_DIR/$NAME.ogg"
  echo "wrote $OGG_DIR/$NAME.ogg"
fi
python3 "$HERE/check_mix.py" "$TMP/norm.wav" --profile
echo "wrote $OUT_MP3"
