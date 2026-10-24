# Proposal: glass-reflections

## Why

Owner, 2026-10-24, on the building stills: "that glass needs better reflections/materials to be
reflective, cache some cube maps?", with https://polyhaven.com/hdris. The game's curtain walls,
ribbon windows and shopfronts read as clear plastic: the world camera has no environment map, so
the glass has nothing to reflect but the sun's highlight, and its alpha blending scales even that
down to 35 %. A glass tower from the street should mirror the sky and, close up, the street.

## What changes

- **A cached sky** (built): one CC0 Poly Haven HDRI, `kloofendal_48d_partly_cloudy_puresky`, baked
  once by `tools/textures/gen_sky_cube.py` into a 512 px a face cube (sun clamped out, because the
  game's own sun draws the highlight; a ground tone below the horizon), shipped as
  `assets/textures/sky/sky_cube.hdr`. At startup it is filtered on the GPU by Bevy's
  `GeneratedEnvironmentMapLight` on the pilot's eye camera, once, and the result kept: every PBR surface in the world
  reflects that sky and is filled by its light. The flat ambient drops from 900 to 250 cd/m2 to
  make room for it.
- **Mirror-coated tinted glass** (built; owner, 2026-10-24: "usually glass is tinted", "so it
  reflects more light, and interiors appear darker than the world around it", then "glass still
  doesnt look right, too transparent", "it should go harder on the reflections", "in real life,
  thered be less light on the inside of the building, so the reflectivity shuold be way
  stronger"): premultiplied alpha, so the reflection is added whole and only what is behind the
  pane is dimmed (alpha 0.88: the rooms show at 12 %, as a room lit only through its windows
  would); a reflective coating (Bevy's `metallic`, 0.9) over the building's glass tint, about
  45 % head-on and a mirror at grazing angles; roughness 0.02. Plain glass cannot pass 16 %
  head-on in Bevy, hence the coating. One material, tuned in `data/world/sky_light.ron`; each
  building's glass tint (blue, green, bronze, grey) still colours it.
- **Cached street probes** (designed, not built): cube maps of the street itself, captured once
  near the player with the existing face-camera rig and kept as light probes, so glass across the
  street mirrors the buildings opposite, not only the sky. Section 3 of the design.

## Out of scope

Screen-space reflections, ray-traced reflections, the visible sky (the skybox stays the flat sky
colour and fog unless the owner asks: open question GR1), interior mapping on far glass
(`building-interiors`).
