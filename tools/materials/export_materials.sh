#!/usr/bin/env bash
# Renders every graph in tools/materials/ptex with Material Maker, headless (the material-maker-headless skill), into
# tools/materials/raw/ (gitignored), then runs the post-process into assets/textures/fx/ and the contact sheet.
# openspec changes `blast-scorch` and `fx-sprites`.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/materials/build_ptex.py
rm -rf tools/materials/raw
MM_TIMEOUT="${MM_TIMEOUT:-2400}" bash .claude/skills/material-maker-headless/scripts/render.sh \
  -o tools/materials/raw --size 2048 tools/materials/ptex/*.ptex
python3 tools/materials/postprocess.py tools/materials/raw
