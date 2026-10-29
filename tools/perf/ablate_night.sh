#!/bin/sh
# The night's suspects (openspec change `night-frame`, design section 2): city-stand at noon, at midnight, and at
# midnight with each suspect switched off alone, interleaved (A..F, then F..A) over $1 rounds, one game at a time, in
# its window, vsync off, game audio muted. Keeps each run's frame log and log as output/perf/night/<variant>-<round>.*.
# A measurement instrument. Usage: tools/perf/ablate_night.sh [rounds] [exe]
cd "$(dirname "$0")/../.."
rounds=${1:-2}
exe=${2:-./target/release/crab-breeze.exe}
mkdir -p output/perf/night
variants="noon midnight no-lamps no-windows no-shadows no-fx-lights"
reverse="no-fx-lights no-shadows no-windows no-lamps midnight noon"
env_of() {
  case "$1" in
    noon) echo "CB_HOUR=12" ;;
    midnight) echo "CB_HOUR=0" ;;
    no-lamps) echo "CB_HOUR=0 CB_NO_LAMPS=1" ;;
    no-windows) echo "CB_HOUR=0 CB_NO_WINDOW_LIGHTS=1" ;;
    no-shadows) echo "CB_HOUR=0 CB_NO_SHADOWS=1" ;;
    no-fx-lights) echo "CB_HOUR=0 CB_NO_FX_LIGHTS=1" ;;
  esac
}
for r in $(seq 1 "$rounds"); do
  order=$variants
  [ $((r % 2)) -eq 0 ] && order=$reverse
  for v in $order; do
    rm -f output/perf/city-stand-frames.csv
    env CB_UNCAPPED=1 CB_NATIVE_1080=1 CB_MUTE=1 $(env_of "$v") "$exe" --playtest city-stand > "output/perf/night/$v-$r.log" 2>&1
    cp output/perf/city-stand-frames.csv "output/perf/night/$v-$r.csv" 2>/dev/null
    echo "$v round $r: $(grep -ao 'PLAYTEST_RESULT[^\r]*' "output/perf/night/$v-$r.log" | head -1)"
  done
done
echo night ablation done
