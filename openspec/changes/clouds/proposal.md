# Proposal: clouds

## Why

Owner, 2026-10-24: "Cloud system: need something cheap and fast that I can fly into. No need for
rain right now, but will need to borrow the rain drops on camera from Pale Blue Dot." Jet flight
(M3) takes the mech into the sky; Ace Combat's sky is full of clouds you fly through.

## What changes

- A cloud layer from the seed: cumulus clusters between `cloud_base_m` and `cloud_top_m` (data,
  default 600-1100 m), drawn as soft lit billboard puffs (instanced, a few hundred clusters of
  about 20 puffs), with soft-particle depth fade so they never cut hard edges into terrain or
  mechs.
- Flying into a cloud: each cluster is a set of ellipsoids; when the camera (the sensor head) is
  inside one, the view fogs to white in proportion to depth inside (a fog volume around the camera,
  cheap), wisps stream past, and the screens show it. Clouds also cast soft shadows on the ground
  through a scrolling shadow texture.
- Wind moves the layer slowly; deterministic from the seed and time.
- Camera raindrops (later, when rain arrives): port Pale Blue Dot's droplets-on-the-lens effect
  onto the all-view screen's camera, and use it now for spray when wading (`terrain-water`). Needs
  Pale Blue Dot's source; this session cannot reach that repository, so the owner or a session
  with access brings it in.

## Out of scope

Rain and storms, volumetric ray-marched clouds (too costly for the target), weather changes.
