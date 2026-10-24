# Proposal: jet-flight

## Why

Owner, 2026-10-24: "Cloud system: need something cheap and fast that I can fly into." Flying into
clouds needs flight, which survey M3 decided (limited by heat; fuel ignored in early testing) but
no change had built. This is the first, simple flight model, written up after it was built to show
the clouds in the 2026-10-24 showcase take (CLAUDE.md section 4 asks for the write-up first; this
records the deviation).

## What changes

- `LocomotionMode::Jet` is built. Mode 3 (or the scripted pilot) switches to it; Space lifts.
- Vertical: jet thrust against gravity, climb capped, a heat budget that cuts the jets when full and
  relights them when cooled. Horizontal: the throttle lever drives a faster air speed.
- Touching down ends a flight; the mech walks on. Roofs are ground: a mech can land on a building.
- Jet flame and smoke at `JET_L` and `JET_R` while lifting.

## Out of scope

Fuel (survey M3: ignored for now), aerodynamics, stalls, flight assists (G5), the cockpit's heat
gauge (the number exists: `MechState::jet_heat`).
