#!/usr/bin/env bash
# Build Crab Breeze's sound effects: tools/audio/gen_sfx.py synthesises every cue into output/sfx with
# a manifest, tools/audio/cut_cc0.py replaces the recorded cues with cuts from CC0 and public domain
# originals (fetched once by tools/audio/fetch_cc0.py into $SFX_SRC, default /opt/sfx-src), then
# each becomes the game's copy (assets/audio/sfx/<cue>.ogg, Vorbis q5, mono) and a share copy for the Mech Sounds page (output/sfx/<cue>.mp3).
# Usage: tools/audio/render_sfx.sh [cue ...]      (no cues: all of them)
# Needs: python3 with numpy, ffmpeg, curl, unzip, 7z (the first fetch only).
set -euo pipefail
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$REPO/output/sfx"
OGG="$REPO/assets/audio/sfx"
mkdir -p "$OUT" "$OGG"
if [ "$#" -gt 0 ]; then
  python3 "$REPO/tools/audio/gen_sfx.py" --out "$OUT" --only "$@"
else
  python3 "$REPO/tools/audio/gen_sfx.py" --out "$OUT"
fi
SRC="${SFX_SRC:-/opt/sfx-src}"
[ -f "$SRC/licences.json" ] || python3 "$REPO/tools/audio/fetch_cc0.py" --dest "$SRC"
python3 "$REPO/tools/audio/cut_cc0.py" --src "$SRC" --out "$OUT" "$@"
for wav in "$OUT"/*.wav; do
  name="$(basename "$wav" .wav)"
  if [ "$#" -gt 0 ] && [[ ! " $* " == *" $name "* ]]; then continue; fi
  ffmpeg -hide_banner -loglevel error -y -i "$wav" -c:a libvorbis -q:a 5 "$OGG/$name.ogg"
  ffmpeg -hide_banner -loglevel error -y -i "$wav" -c:a libmp3lame -b:a 160k "$OUT/$name.mp3"
done
echo "wrote $(ls "$OGG"/*.ogg | wc -l) sounds to assets/audio/sfx"
