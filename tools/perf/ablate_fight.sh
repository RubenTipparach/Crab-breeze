#!/bin/sh
# Runs the free-roam-fight script once per variant, each switch alone, interleaved over $1 rounds, one game at a time,
# vsync off; keeps each run's frame log as output/perf/ablate/<variant>-<round>.csv. A measurement instrument.
cd "$(dirname "$0")/../.."
rounds=${1:-2}
variants="base CB_NO_PARTICLES CB_NO_SHADOWS CB_NO_BUILDINGS CB_NO_CLOUDS CB_NO_FX_LIGHTS"
for r in $(seq 1 "$rounds"); do
  for v in $variants; do
    rm -f output/perf/free-roam-fight-frames.csv
    if [ "$v" = base ]; then
      CB_UNCAPPED=1 ./target/release/crab-breeze.exe --playtest free-roam-fight > "output/perf/ablate/$v-$r.log" 2>&1
    else
      env CB_UNCAPPED=1 "$v=1" ./target/release/crab-breeze.exe --playtest free-roam-fight > "output/perf/ablate/$v-$r.log" 2>&1
    fi
    cp output/perf/free-roam-fight-frames.csv "output/perf/ablate/$v-$r.csv" 2>/dev/null
    echo "$v round $r: $(grep -o 'PLAYTEST_RESULT.*' "output/perf/ablate/$v-$r.log" | head -1)"
  done
done
echo ablation done
