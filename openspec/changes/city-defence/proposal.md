# Proposal: city-defence

## Why

Owner, 2026-10-23: "design a benchmark scenario of 3 waves of attacking mech across 3 cities, your
job is to eliminate all enemies (10 in each wave) before each city takes too much damage, if city
is damaged more than 50% you lose", then "each wave must be cleared in 10 minutes" and "record,
upload to gdrive when you have a winning run".

It is the first mission with a win and a loss, and the heaviest scene the game can make today: up
to 10 enemy mechs firing into a city at once, buildings falling, the player's guns, rockets and
explosions, so it doubles as the benchmark scene CLAUDE.md section 7 asks for ("the densest
scene"). It is also the first taste of Ace Combat's set pieces (G4): a clear goal, a clock and a
city at stake.

## What changes

- **The mission** (`cb-core::mission`, pure and tested): three waves, one per city. Wave 1 attacks
  the city the pilot starts in, wave 2 the next largest, wave 3 the third. Each wave is 10 enemy
  mechs that appear outside the city and march on it. The pilot wins by destroying all 30. The
  pilot loses if a city's damage passes 50 %, if a wave is not cleared within 10 minutes of
  appearing, or if their own mech is destroyed.
- **City damage** is one number per city, the share of its buildings' structure destroyed
  (destroyed cells over all cells, so a fallen tower counts for more than a shed). The cockpit
  shows the same number the rule checks (section 6.1).
- **Raiders**: the enemies' own brain (`enemy-mechs`), given a new goal. Most of a wave shells the
  city in bursts; the three nearest the player within 300 m turn to fight them, and any raider hit
  turns on the pilot for a while. 3 of every 10 carry a rocket pod (`rockets`).
- **The cockpit readout**: wave, time left, the city's damage, enemies left.
- **Ways to run it**: `./run.sh --mission defence` to play it; `--showcase defence` for the
  scripted pilot's take (it boards, travels between cities on foot and on jets, and fights each
  wave with the same brain the enemies use, through the player's actions); `--bench defence` for
  frame time by beat. The take prints `PLAYTEST_START`, `PLAYTEST_RESULT win|lose ...` and
  `PLAYTEST_END` for the recorder.
- **A measurement tool**, `cb-tools cities [seed]`: each city's size, buildings and cells.

## Out of scope

Briefings, radio, a mission menu, allies, difficulty settings, repairs between waves, saving.
