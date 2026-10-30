# City benchmark, RTX 3070

Published: https://claude.ai/artifact/LaCXXniGEcVRPWCtTvmhAw (a dashboard; private until shared from its page).

The night of 2026-10-27 to 28 on the owner's desktop: the capital before and after `city-frame`, what the GPU was
drawing with each part switched off, the video-memory spill, the flights across the map and the twenty houses brought
down. The write-up behind it is `docs/benchmarks/2026-10-28-city/report.md`, every run in `runs.txt` there.

- `index.html`: the dashboard's page, as stored in its `files/index.html`.
- The eight CSV files: its data, each attached to the dashboard as a fixed dataset (`city`, `parts`, `vram`, `images`,
  `flight`, `suburb`, `night_suburb`, `night_moon`), taken from the runs' frame logs, the probe's census and Windows' GPU
  memory counters.
- 2026-10-29: the "At night" section (`night_suburb.csv`, `night_moon.csv`, from `output/perf/night/suburb-stand` and
  `output/perf/night/ab-suburb-stand`; openspec change `night-frame`) and the Next list brought up to date.

Made by the desktop session; republished here whenever the page or its data change.
