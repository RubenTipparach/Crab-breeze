#!/usr/bin/env bash
# Fetch the soundfont every Crab Breeze score plays through: GeneralUser GS 2.0.3
# by S. Christian Collins (licence: free for music creation, private or
# commercial; documentation/LICENSE.txt in its repository). Pinned to a commit and
# checked by SHA-256, so every render uses the same samples. About 32 MB, into
# /opt/soundfonts (not the repository: the licence asks for no hotlinking and the
# file is large). music/instruments.json names the path.
set -euo pipefail
REV=684543d5e5efaef08d02be50dcda8d552478fa60
SHA=9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe
DIR=/opt/soundfonts
URL=https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/$REV
mkdir -p "$DIR"
if ! echo "$SHA  $DIR/GeneralUser-GS.sf2" | sha256sum -c --quiet 2>/dev/null; then
  curl -sSfL -o "$DIR/GeneralUser-GS.sf2" "$URL/GeneralUser-GS.sf2"
  curl -sSfL -o "$DIR/GeneralUser-GS-LICENSE.txt" "$URL/documentation/LICENSE.txt"
  echo "$SHA  $DIR/GeneralUser-GS.sf2" | sha256sum -c --quiet
fi
echo "GeneralUser GS ready in $DIR"
