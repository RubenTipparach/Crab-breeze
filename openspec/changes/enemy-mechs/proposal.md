# Proposal: enemy-mechs

## Why

"Epic mech fights" need an opponent. An enemy is the same mech simulation driven by an AI pilot
that writes the same controls a player does (one implementation: no enemy-only movement or damage).

## What changes

- Any number of mechs, each with its own state, damage and rig; the player's is the one with a pilot.
- An AI pilot (`cb-core::ai`): find the player, close to a preferred range while keeping moving,
  twist the torso onto the target, lead it, fire when the aim error is small, use jets to hop when
  hit hard, walk around buildings it cannot shoot through. Seeded and deterministic.
- Enemies are listed in data (`data/world/encounters.ron`): chassis, spawn, skill.

## Out of scope

Squads, cover planning, missions and briefings (G4), infantry and vehicles.
