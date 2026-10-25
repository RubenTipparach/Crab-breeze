#!/usr/bin/env bash
# Run Crab Breeze. Builds the release game if needed, then starts it.
#
#   ./run.sh                      play (seed 42, the stand-in mech)
#   ./run.sh --mech mechs/foreman.ron   the detailed mechs (mech-roster): foreman, ironhorse, harrow,
#                                       roustabout, driller, mule, picket, sapper
#   ./run.sh --seed 7
#   ./run.sh --showcase fight     watch the scripted fight
#   ./run.sh --showcase world     watch the scripted world tour
#   ./run.sh --start fx           every effect set out below the overlook in front of you, on a loop
#
# Controls (Esc opens the menu; its Controls page lists every key from data/input/bindings.ron):
# WASD walk / throttle and turn, mouse look / aim the torso, E board and climb out, Shift run,
# X throttle to zero, hold Left Ctrl to free look (and click an MFD to change its page).
# In the mech: hold Space for the rockets, Shift toggles the skates, C dives or sinks, left mouse
# autocannon, right mouse cannon, F missile at the lock, Q next target, Tab close combat (on by
# itself when the targeted hostile is within 30 m: the mouse buttons strike with the arms),
# F5-F8 shoot off a part, F9 drop a shell.
set -euo pipefail
cd "$(dirname "$0")"
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found: install Rust from https://rustup.rs" >&2
  exit 1
fi
cargo build --release -p cb-app
exec ./target/release/crab-breeze "$@"
