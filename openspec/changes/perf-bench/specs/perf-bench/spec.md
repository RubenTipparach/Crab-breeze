## ADDED Requirements

### Requirement: Frame cost by beat
The benchmark SHALL play the scripted take rendering to an offscreen 1920 x 1080 target and report frame time percentiles per beat.

#### Scenario: A bench run
- **WHEN** `crab-breeze --bench` runs to the end of the take
- **THEN** it prints a `BENCH` line for every beat with p50, p95, p99 and max frame time in milliseconds
