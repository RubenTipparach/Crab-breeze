#!/bin/sh
# The free-roam fight with the game's info logging on and off (RUST_LOG=warn), interleaved over $1 rounds, vsync off:
# does writing log lines cost the frame? Frame logs land in output/perf/logtest/<variant>-<round>.csv. A measurement
# instrument.
cd "$(dirname "$0")/../.."
for r in $(seq 1 "${1:-2}"); do
  for v in info warn; do
    rm -f output/perf/free-roam-fight-frames.csv
    env CB_UNCAPPED=1 RUST_LOG=$v ./target/release/crab-breeze.exe --playtest free-roam-fight > "output/perf/logtest/$v-$r.log" 2>&1
    cp output/perf/free-roam-fight-frames.csv "output/perf/logtest/$v-$r.csv" 2>/dev/null
    echo "$v round $r: $(grep -ao 'PLAYTEST_RESULT[^\r]*' "output/perf/logtest/$v-$r.log" | head -1)"
  done
done
echo log test done
