# Proposal: rain

## Why

The owner, 2026-10-25: "can we also implment actual rain particles?" Today rain is only distant shafts under the
storm cells (`cloud-weather`) and drops on the glass inside a cloud (`cloud-rain`). Under a storm, nothing falls
around the pilot.

## What changes

- **Falling rain around the pilot.** Inside a storm's rain shaft, streaks fall all around the eye, slanted by the
  wind, thicker toward the shaft's middle. They fade in at the shaft's edge, so walking or flying in or out of the
  storm is gradual.
- **Splashes.** Drops hitting the ground, roads, roofs and water make small splashes and rings near the eye.
- **Wet glass.** Under the rain, the drops on the cockpit's screens and on the pilot's view on foot come from the
  same rain amount, through the existing lens pass (`lens_rain`), with no second drop effect.
- **Sound.** The ambience's rain bed (`amb_rain`, already in `data/audio/ambience.ron`) rises with the same amount.
- **A free-roam switch.** The pause menu's Options gets Weather: Storms (rain under the storm cells only, the
  default) or Rain everywhere, so the rain can be seen without flying to a storm.

## Out of scope

Puddles and wet-ground shading, rain on the mech's own armour, wipers, snow.
