#!/bin/sh
# Old build against new on the free-roam fight, interleaved over $2 rounds, vsync off, one game at a time: the
# binaries are output/perf/bin/old.exe and new.exe; each run's frame log and log land in output/perf/$1/. A
# measurement instrument (perf-measure).
cd "$(dirname "$0")/../.."
out="output/perf/$1"
mkdir -p "$out"
for r in $(seq 1 "${2:-3}"); do
  for v in old new; do
    rm -f output/perf/free-roam-fight-frames.csv
    env CB_UNCAPPED=1 RUST_LOG=info "./output/perf/bin/$v.exe" --playtest free-roam-fight > "$out/$v-$r.log" 2>&1
    cp output/perf/free-roam-fight-frames.csv "$out/$v-$r.csv" 2>/dev/null
    echo "$v round $r: $(grep -ao 'PLAYTEST_RESULT[^\r]*' "$out/$v-$r.log" | head -1)"
  done
done
echo ab done
