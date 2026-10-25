# Proposal: cloud-rain

## Why

Owner, 2026-10-24: "then I want you to implmeent rain drops on glass as I fly through the clouds",
"like in pale blue dot repo".

## What changes

- The game already has Pale Blue Dot's drops on the glass. `water-rendering` ported the lens pass,
  with the "Heartfelt" drop field through Tenebris. It has a rain channel, `lens_fx.y`, fed from a
  fixed `lens_rain` (0).
- Flying through a cloud now wets that glass. `water_look::cloud_glass` raises the rain toward
  `cloud_rain` (0.85) x the cloud's density at the eye over `cloud_wet_s` (1.2 s), and clears it
  over `cloud_dry_s` (4 s) once the eye is out.
- Seated, the drops sit on the external camera lenses, so they show on the cockpit's screen wall.
- One drop effect for water and cloud alike (CLAUDE.md 6.1).

## Out of scope

Drops streaking with airspeed, rain showers under clouds, wipers.
