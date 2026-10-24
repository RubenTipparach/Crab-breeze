# Proposal: water-rendering

## Why

Owner, 2026-10-24: "need to borrow water rendering, underwater rendering principals from pale blue
dot so that water game look good, same with the water effects on glass when emerging from
underwater." The mech is about to skim the water on skates, sink, and walk on the sea floor
(`terrain-water`, survey W3), and the world is becoming a 40 km island in an ocean. The pilot's eye
(the one camera, the law) will go under water and come back up. Today the water is a flat
`StandardMaterial` sheet with a scrolled sine-wave normal map: no depth colour, no refraction, no
foam, nothing under water, nothing on the glass.

Pale Blue Dot (the owner's Bevy 0.18.1 game, revision `db25fe2`) already solved this, porting
Tenebris's water term for term: `crates/pbd-app/src/planet_water.rs`, `assets/shaders/water.wgsl`,
`assets/config/water.ron`. This change ports its principles and as much of its code as fits a flat
world and Bevy 0.20. It also settles survey T4 (Pale Blue Dot's source is now in reach).

## What changes

What the player sees:

- **Water that reads as water.** Every water surface (rivers, lakes and the ocean) uses one water
  material: rippling, flowing normals (Pale Blue Dot's gradient-noise fbm, carried along the river
  by a two-phase flow map), a long swell in the normals, the sky mirrored by Schlick's Fresnel, the
  ground under the water refracted and absorbed by colour (clear in the shallows, dark blue-green in
  the deep), foam on crests, steep ripples and along the shore, and a sun glint.
- **Under water.** When the pilot's eye is below the surface, the view fogs toward a murk colour
  that darkens with depth, red going first; the image wobbles; the surface seen from below shows
  Snell's window (the sky straight up, a mirror of murk outside it). Inside a sealed cockpit, the
  cabin stays dry: only what the screens show past the hull is murky.
- **Water on the glass when surfacing.** When the eye comes up out of the water, drops and rivulets
  run down the view for a few seconds, refracting the real scene; straddling the surface keeps the
  glass wet; the distance softens while it is wet. Raindrops on the glass are ported and wired but
  gated off until weather exists.

How it works: a `WaterMaterial` (Bevy `Material`, transmissive phase) replaces the water's
`StandardMaterial`; two fullscreen passes (compose, lens) run before tonemapping on cameras that
carry the `WaterOptics` marker; the main world decides once a frame how wet the eye is from the one
water query (`Ground::water_at`) and the pure state machine in `cb_core::water_look`. Tuning lives
in `data/world/water_look.ron`. The pilot camera becomes HDR and gains a depth prepass and
screen-space transmission, which the material needs.

## Out of scope

- Displaced wave geometry: the surface stays flat so the simulation's `surface_m` is the surface the
  mech skates on (CLAUDE.md 6.1); the swell is in the normals only.
- Pale Blue Dot's cloud march, rain volume, weather overlay and the lens mist probe (they need its
  cloud and weather maps, which Crab Breeze does not have).
- Rain ripples on the water surface (no rain yet).
- Ocean geometry and island terrain (the `island` work owns them).
