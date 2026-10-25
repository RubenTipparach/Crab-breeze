# Proposal: ground-fog

## Why

Owner, 2026-10-25, while the weather was being built: "think about some cheap volumetric
ground/distance fog too". Today the air between the eye and the ground is the atmosphere's aerial
perspective (`sky-atmosphere`), which only hazes the far distance, plus a flat fog used inside
cloud and under water. Nothing lies low: valleys, lakes and the coast are as clear at dawn as at
noon, the crater has no mist in it, the rain under a storm has no grey curtain round you, and a
mech walking out of a river valley never parts a fog bank.

## What changes (designed, not built)

- **Height fog, worked out exactly along each view ray, not marched.** Fog that is thick near the
  ground and thins with height (an exponential layer) has a closed-form integral along a straight
  ray, so each pixel's fog costs a handful of instructions, whatever the distance. It is added to
  the existing full-screen water `compose` pass, which already fogs each pixel by the medium it
  crossed (the water): one pass for everything between the eye and a pixel, not a second one.
- **Mist where mist forms**: a coarse mist map over the island, baked once from the world's own
  rules (low ground relative to its surroundings, lakes, rivers, wetland, the coast, the crater
  floor), scales the layer's density; the pass reads it at a few points along the ray. Valleys and
  the crater fill, ridges stand out above it.
- **Rain haze under storms**: inside and near a rain shaft (`cloud-weather`) the fog thickens and
  greys, so standing under a storm shrinks the world to a few kilometres.
- **Lit by the scene**: the fog's colour is the sky's ambient plus the sun scattered forward (bright
  toward the sun, a glow round it), darkened under a storm by the same shade the sun carries.
- **Time and weather set its strength** in data: a morning mist preset and a clear afternoon one,
  picked per mission.
- `CB_NO_GROUND_FOG=1` switches it off for the benchmark.

## Out of scope

Bevy's ray-marched `VolumetricFog` everywhere (weighed in the design: far more expensive); light
shafts through the fog; fog that moves round a walking mech (later, if a playtest asks for it);
local fog volumes (steam when the cockpit opens, waterfall spray) which are their own effects.
