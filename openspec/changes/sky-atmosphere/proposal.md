# Proposal: sky-atmosphere

## Why

Owner, 2026-10-24: "Sky needs better sky shaders it's all uniform in color." The sky is a flat
clear colour (`SKY`), and the distance fog fades everything toward the same flat blue, which also
washes out the ground's colours.

## What changes

- Bevy 0.20's physically based atmosphere (`Atmosphere::earth`, Bruneton-style lookup tables) draws
  the sky on the pilot's camera: deep blue overhead, paler toward the horizon, warmer around the
  sun, with a sun disk; and its aerial perspective hazes distant ground by real scattering instead
  of a flat fog colour.
- The sun becomes raw sunlight (`lux::RAW_SUNLIGHT`) filtered by the atmosphere, with the camera's
  exposure set to match (data: `world.view.exposure_ev100`); ambient light is scaled by the same
  exposure change so shadows keep their weight.
- The flat distance fog stays only for what it is still needed for (inside cloud and under water),
  thinned in clear air.
- `CB_NO_SKY=1` leaves the old flat sky, so the suite can measure the atmosphere's cost.

## Out of scope

Time of day, volumetric clouds, the cockpit's own lighting (its camera keeps its own exposure).
