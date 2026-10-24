# Design: city-defence

All numbers are the defaults in `data/missions/defence.ron`; the code holds none (section 6.4).

## 1. The cities (measured)

`cb-tools cities 42` (a measurement instrument, CLAUDE.md section 4), on the default seed:

| Wave | City | Centre (x, z) | Half size | Buildings | Cells | From the previous city |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 | (281, 230) | 474 m | 461 | 89 255 | the pilot starts 51 m from its centre |
| 2 | 1 | (-1044, 303) | 466 m | 365 | 70 637 | 1 327 m |
| 3 | 2 | (-1020, -1260) | 338 m | 243 | 37 744 | 1 563 m |

A cell is 6 m x 6 m x one storey (`building-destruction`). At the mech's top walking speed
(7 m/s) the 1.3-1.6 km between cities is 3-4 minutes; on jets (60 m/s, limited by heat) under a
minute of flying in hops.

## 2. The rules (`cb-core::mission::Defence`)

A state machine stepped every fixed step with what the simulation knows: the clock, how many of
the wave's raiders are alive, each city's damage, and whether the player's mech is destroyed.

| Phase | Ends when | Then |
| --- | --- | --- |
| `Briefing` | the pilot is seated | `Lull` for wave 1 |
| `Lull(w)` | `lull_s` (20 s) has passed | `Wave(w)`: its 10 raiders appear, its clock starts |
| `Wave(w)` | all its raiders are destroyed | `Lull(w + 1)`, or `Won` after the last wave |
| `Wave(w)` | its city's damage passes `lose_damage` (0.5) | `Lost(CityFell)` |
| `Wave(w)` | `wave_time_s` (600 s) has passed | `Lost(OutOfTime)` |
| any | the player's mech is destroyed | `Lost(PilotDown)` |

`city_damage(city) = 1 - standing cells / cells at the start`, over the city's buildings, from
`Structures`: the one number the rule and the cockpit both read. A city that is not under attack
can still take damage (stray shells); only the attacked city's damage decides a loss, but every
city's damage is logged.

Tests read as sentences: `clearing_a_wave_starts_the_next_after_the_lull`,
`a_city_past_half_damage_loses_the_mission`, `a_wave_not_cleared_in_ten_minutes_loses`,
`clearing_all_three_waves_wins`, `city_damage_counts_cells_not_buildings`,
`spawn_points_are_the_same_for_the_same_seed`.

## 3. Where raiders appear

On an arc of `spawn_arc_deg` (100 degrees) at `spawn_distance_m` (450 m) beyond the city square's
corner radius, centred on the bearing away from the previous city (wave 1: a bearing from the
seed fork `defence/0`), spaced evenly with a seeded jitter of up to 30 m. A point in water or
outside the world's 1900 m half-width is pulled toward the city until it is dry and inside. Each
raider faces the city centre. Seeded and stable: the same seed gives the same points.

## 4. Raiders (`cb-core::mission::raider_goal`)

Every think (0.25 s) each raider picks one of two goals, a pure function of what it can see:

- **Fight the pilot** if it is one of the `max_on_player` (3) raiders nearest the player and
  within `engage_player_m` (300 m), or if it was hit in the last `grudge_s` (8 s) and the player
  is within 700 m. The goal is the player's mech, exactly as in `enemy-mechs`.
- **Shell the city** otherwise: the goal is the facade point of the nearest standing building of
  the attacked city (its footprint's nearest point to the raider, 2 m out from the wall), with zero
  velocity. It fires only inside its burst window: `siege_burst_s` (1.0 s) out of every
  `siege_burst_s + siege_pause_s` (6.0 s), offset per raider by its index so a wave does not fire
  in step. A facade point is clear to aim at when nothing stands between the raider's eye and a
  point 9.8 m up the wall (the AI's existing line check).

The brain itself (orbit, evade, reposition, retreat, hop) is unchanged: one implementation, a
different target.

### Loadouts (cycled through the wave)

| Raider | Share | Weapon | Armour |
| --- | --- | --- | --- |
| autocannon raider | 7 in 10 | 25 mm autocannon, 0.2 s, 55 mm, 6 damage, 0.3 MJ | 55 % of the stand-in's |
| rocket raider | 3 in 10 | 90 mm rocket pod, 240 m/s, 1.6 s, 120 mm, 30 damage, 2 MJ (`rockets`) | 55 % |

### How fast an undefended city falls (estimate, to be measured)

A 35 % storey gives way under everything above it, so most cells fall in collapses. An average
building in city 0 has 194 cells over about 13 storeys (15 cells a storey); shelling at 9.8 m
(storey 3) takes about 10 hits to drop 165 cells, about 16 cells a hit. The design target is that
an undefended wave takes its city past 50 % in about 6 minutes, so the clock and the city both
press: 44 600 cells of city 0 in 360 s is 124 cells/s for 10 raiders, 0.8 hits a second each, which
the 1 s burst in 6 s gives at 5 rounds a second with about 1 in 2 landing. Every raider the pilot
destroys slows the city's fall. The first playtest logs every city's damage every 10 s (`MISSION
status`), and the burst numbers are retuned from that log, not from this estimate.

## 5. The scripted pilot (`--showcase defence`)

It plays fair, through the player's actions only. Beats:

| Beat | Does |
| --- | --- |
| `load` | waits 5 s for shaders |
| `look` | looks along the street |
| `board` | walks to the stair, presses Use, is carried in, is seated (the one shared boarding beat) |
| `travel` | heads for the attacked city's spawn side: on jets in hops while their heat is under 0.85, walking between hops; within 500 m of the nearest raider, on to `fight` |
| `fight` | the enemies' brain on the nearest live raider of the wave, its orders as actions (as in the city fight take); when the wave is cleared, `travel` to the next |
| `end` | prints `PLAYTEST_RESULT win` or `PLAYTEST_RESULT lose <why>`, then `PLAYTEST_END` |

## 6. The cockpit readout

A line at the top of the screen wall: `WAVE 2/3   CITY 1  DAMAGE 23 %   RAIDERS 6/10   7:41`.
It turns amber past 35 % damage or under 2 minutes, red past 45 % or under 1 minute. It reads
`Defence` and `city_damage` (section 6.1). The win and loss lines say why.

## 7. Cost

| Item | Numbers |
| --- | --- |
| Mechs | up to 10 live raiders and 20 wrecks; the stand-in is 39 boxes, so about 1 200 mesh entities, batched |
| AI | 10 brains at 4 thinks a second; a think is a few line checks and a scan of the attacked city's buildings (243-461), microseconds |
| Shots | at most about 60 in flight (bursts, rockets) |
| City damage | summed per city when a building changes (not every step): under 0.1 ms |
| Frame | measured by `--bench defence` on the RTX 3070 machine, per beat (`perf-measure`); the target is p95 8.3 ms, p99 16.7 ms at 1920 x 1080 |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G9 | What "city damaged" counts | The share of the city's building structure destroyed (cells), so a fallen tower counts more than a shed | provisional (survey G9) |
| G10 | When a wave's 10 minutes start | When the wave appears, so travel between cities is part of the race | provisional (survey G10) |
| G12 | How many raiders turn to fight the pilot at once | 3, the rest keep shelling the city | provisional (survey G12) |
| G11 | Who carries rockets first | 3 of every 10 raiders | provisional (survey G11) |
