#!/usr/bin/env bash
# Renders the houses' surface graphs (tools/materials/ptex/house_*.ptex, written by build_house_ptex.py) with Material
# Maker, headless (the material-maker-headless skill), at 2048 px into tools/materials/raw/houses/ (gitignored), then
# writes assets/textures/houses/<generator>/ and docs/screenshots/house_materials/sheet.png (postprocess.py --houses).
# Set up Material Maker first: eval "$(bash .claude/skills/material-maker-headless/scripts/setup.sh | grep ^export)".
# Pass generator names (brick siding ...) to rebuild only those graphs; the post-process always rewrites every set.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/materials/build_house_ptex.py "$@"
graphs=()
if [ $# -gt 0 ]; then
  for g in "$@"; do graphs+=("tools/materials/ptex/house_$g.ptex"); done
else
  rm -rf tools/materials/raw/houses
  graphs=(tools/materials/ptex/house_*.ptex)
fi
MM_TIMEOUT="${MM_TIMEOUT:-3600}" bash .claude/skills/material-maker-headless/scripts/render.sh \
  -o tools/materials/raw/houses --size 2048 "${graphs[@]}"
python3 tools/materials/postprocess.py tools/materials/raw/houses --houses
