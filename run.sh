#!/usr/bin/env bash
# Run Crab Breeze. Builds the release game if needed, then starts it.
#
#   ./run.sh                      play (seed 42, the stand-in mech)
#   ./run.sh --mech mechs/foreman.ron   the detailed mechs (mech-roster): foreman, ironhorse, harrow,
#                                       roustabout, driller, mule, picket, sapper
#   ./run.sh --seed 7
#   ./run.sh --showcase fight     watch the scripted fight
#   ./run.sh --showcase world     watch the scripted world tour
#
# Controls: WASD walk / throttle and turn, mouse look / aim the torso, E board and climb out,
# Shift run, X throttle to zero, Alt free look, 1 walk.
# In the mech: hold Space for the rockets (up only), Shift toggles the skates (wheels on the ground
# and the water, WASD thrust in the air), hold Ctrl to dive from the air or sink on water,
# left mouse autocannon, right mouse cannon, F5-F8 shoot off a part, F9 drop a shell,
# Esc release the mouse (click to grab it again).
set -euo pipefail
cd "$(dirname "$0")"
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found: install Rust from https://rustup.rs" >&2
  exit 1
fi
cargo build --release -p cb-app
exec ./target/release/crab-breeze "$@"
