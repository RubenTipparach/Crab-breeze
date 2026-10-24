# Proposal: cloud-puffs

## Why

Owner, 2026-10-24: "I also want clouds", "try to keep clouds cheap", "we're on a tight budget, I
just want good looking clouds with correct lighting and dont cost much to render".

The sprite clouds were turned off (coverage 0) because their large alpha-blended billboards,
seven to a puff, cost heavy overdraw ("something is quite heavy on my gpu disable clouds for now").

## What changes

- Each puff of `cb_core::world::clouds` is drawn as one lumpy blob: an icosphere pushed out by a
  stable per-puff noise, its base flattened at the cloud base.
- Every puff of a 2 km cloud tile is merged into one mesh. That is about 30 tile entities within
  the 9 km draw radius, with one material, so Bevy batches them. The wind moves each tile by its
  transform.
- Lighting comes from the scene: the sun's direction and radiance and the ambient light, through
  the water's light helper, so the clouds sit in the same light as the ground and the water.
  - The shader (`assets/shaders/cloud.wesl`) wraps the sun round the blob, darkens the base,
    lights a silver lining toward the sun and adds the sky's ambient.
  - It softens the silhouette by view angle and fades a blob out near the eye, where the existing
    in-cloud fog takes over.
  - Bevy's own fog hazes far clouds.
- One blended layer per blob replaces stacks of sprites.
- `CB_NO_CLOUDS=1` leaves the clouds out, so the benchmark can measure them. Coverage comes back
  on at 0.35.

## Out of scope

Cloud shadows on the ground (a later scrolling texture, one sample a pixel), storms and rain
showers, volumetric ray-marching.
