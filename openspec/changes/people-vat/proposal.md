# Proposal: people-vat

## Why

Owner, 2026-10-26: "VAT - vertex animated texture for humans". The people in the cities are approved stand-ins
(civilians, survey W43): box figures in four fixed poses near the eye, and everyone past 60 m one merged mesh rebuilt
on the CPU thirty times a second. They do not walk, sit, wave or fall; they snap between poses. The cities are meant
to be full of people (up to 2,000 round the pilot), so a skinned, animated body per person is too dear: the render
world is already the bottleneck on per-entity work (`gpu-instancing`).

## What changes

- **A real human body**, built by a committed Blender generator on the humanoid pipeline
  (`blender-humanoid-characters`): MPFB 2 bodies with the 53-bone `game_engine` rig and CC0 MakeHuman clothes, one man
  and one woman, about 1.75 m, with clips from Quaternius' Universal Animation Library (CC0) retargeted onto that rig,
  and a wave authored in the script. Packs pinned by SHA-256 in `tools/deps/character_packs.json`, cached outside the
  repository.
- **A vertex animation texture (VAT) bake**: every clip at its source rate (24 frames a second), every vertex's position and normal per
  frame, for a near level of detail and a crowd level of detail, written as binary files with a manifest of clips.
- **One VAT renderer in Bevy**: `StandardMaterial` extended with a vertex shader that reads the VAT, so people are lit,
  fogged and shadowed like the world. People are packed 32 to a draw entity, their place, clip, phase, blend and
  colours in one storage buffer written each frame; the draw count is fixed whatever the crowd. It replaces both the
  near body pool and the merged crowd mesh, and the running drivers' capsules.
- **Clips follow the simulation**: walking, waiting, standing, sitting, greeting (a wave, then talking), fleeing and
  struck down each pick a clip by a pure rule in `cb-core`; the walk's cadence follows the body's speed; a change of
  clip crossfades.
- **Measured**: a same-session A/B against the stand-ins at the full crowd on a city route.

## Capabilities

- New: `people-vat`.
- Touched: `civilians` (its design's drawing section 2.1 is replaced; the simulation is unchanged), `car-traffic`
  (running drivers drawn as people).

## Out of scope

Faces and speech animation, per-person textures (clothes are coloured by region, not textured; see open questions),
ragdolls, people inside vehicles and cockpits, the pilot's own body.
