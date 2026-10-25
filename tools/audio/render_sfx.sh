#!/usr/bin/env bash
# Build Crab Breeze's sound effects: tools/audio/gen_sfx.py synthesises every cue into output/sfx with
# a manifest, tools/audio/cut_cc0.py replaces the recorded cues with cuts from CC0 and public domain
# originals (fetched once by tools/audio/fetch_cc0.py into $SFX_SRC, default /opt/sfx-src), then
# each becomes the game's copy (assets/audio/sfx/<cue>.ogg, Vorbis q5, mono; ambience beds amb_* in
# assets/audio/ambience, stereo, q4) and a share copy for the Mech Sounds page (output/sfx/<cue>.mp3).
# Usage: tools/audio/render_sfx.sh [cue ...]      (no cues: all of them)
# Needs: python3 with numpy, ffmpeg, curl, unzip, 7z (the first fetch only).
set -euo pipefail
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$REPO/output/sfx"
OGG="$REPO/assets/audio/sfx"
AMB="$REPO/assets/audio/ambience"
mkdir -p "$OUT" "$OGG" "$AMB"
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
  if [[ "$name" == amb_* ]]; then
    # ambience beds: stereo, non-spatial, in their own folder; a lighter share copy for the page
    ffmpeg -hide_banner -loglevel error -y -i "$wav" -c:a libvorbis -q:a 4 "$AMB/$name.ogg"
    ffmpeg -hide_banner -loglevel error -y -i "$wav" -c:a libmp3lame -b:a 96k "$OUT/$name.mp3"
  else
    ffmpeg -hide_banner -loglevel error -y -i "$wav" -c:a libvorbis -q:a 5 "$OGG/$name.ogg"
    ffmpeg -hide_banner -loglevel error -y -i "$wav" -c:a libmp3lame -b:a 160k "$OUT/$name.mp3"
  fi
done
echo "wrote $(ls "$OGG"/*.ogg | wc -l) sounds to assets/audio/sfx, $(ls "$AMB"/*.ogg 2>/dev/null | wc -l) beds to assets/audio/ambience"
