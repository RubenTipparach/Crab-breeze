#!/bin/bash
# SessionStart hook for Claude Code cloud sessions: make the repository's checks runnable
# (CLAUDE.md section 11). A fresh cloud container lacks the OpenSpec CLI; Rust, python3 and node
# are already there. Adapted from Star Crew Bridge Simulator's hook. Idempotent and synchronous,
# so the checks work from the first prompt.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

OPENSPEC_VERSION="1.14.0"
if ! command -v openspec >/dev/null 2>&1 || [ "$(openspec --version 2>/dev/null)" != "$OPENSPEC_VERSION" ]; then
  npm install -g "@fission-ai/openspec@${OPENSPEC_VERSION}" >/dev/null 2>&1
fi
openspec --version >/dev/null

# The game's build links Wayland, ALSA and udev (Bevy's windowing and audio), and headless stills render through
# Mesa's lavapipe under Xvfb (CLAUDE.md section 9); a fresh container may lack them.
if ! pkg-config --exists wayland-client alsa libudev 2>/dev/null || [ ! -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
  (apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 libxcursor1 libxi6 libxrandr2 \
    libasound2-dev libudev-dev pkg-config mesa-vulkan-drivers libvulkan1 xvfb) >/dev/null 2>&1 || true
fi
