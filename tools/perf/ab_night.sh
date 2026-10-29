#!/bin/sh
# Old build against new at one hour on a scripted spot (openspec change `night-frame`), interleaved over $1 rounds
# (old, new, then new, old), one game at a time, in its window, vsync off, game audio muted. Each build reads its own
# checkout's data and assets (`CB_ROOT`): the old one runs from a worktree of its commit. Keeps each run's frame log and
# log as output/perf/night/ab-<script>/<old|new>-<round>.*. A measurement instrument.
# Usage: tools/perf/ab_night.sh rounds old.exe old-root new.exe [script] [hour]
cd "$(dirname "$0")/../.."
here=$(pwd)
rounds=$1
old=$2
old_root=$3
new=$4
script=${5:-suburb-stand}
hour=${6:-0}
out=output/perf/night/ab-$script
# One game at a time: a game already running (a batch stopped from outside leaves its loop and game behind) spoils
# every frame here, so refuse to start.
if tasklist 2>/dev/null | grep -qiE "^(night|crab-breeze|plain)[^ ]*\.exe"; then
  echo "a game is already running: not starting"
  exit 1
fi
mkdir -p "$out"
run() {
  name=$1; exe=$2; root=$3
  rm -f "$root/output/perf/$script-frames.csv"
  env CB_ROOT="$root" CB_UNCAPPED=1 CB_NATIVE_1080=1 CB_MUTE=1 CB_HOUR="$hour" "$exe" --playtest "$script" > "$out/$name-$r.log" 2>&1
  cp "$root/output/perf/$script-frames.csv" "$out/$name-$r.csv" 2>/dev/null
  echo "$name round $r: $(grep -ao 'PLAYTEST_RESULT[^\r]*' "$out/$name-$r.log" | head -1)"
}
for r in $(seq 1 "$rounds"); do
  if [ $((r % 2)) -eq 1 ]; then
    run old "$old" "$old_root"; run new "$new" "$here"
  else
    run new "$new" "$here"; run old "$old" "$old_root"
  fi
done
echo night ab done
