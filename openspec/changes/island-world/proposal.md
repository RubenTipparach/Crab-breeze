# Proposal: island-world

## Why

Owner, 2026-10-24: "the map needs to be 10x bigger, make terrain like an island surrounded by
water, use floating precision if precision is an issue". The mech now skates at up to 120 m/s and
boosts: it crosses the old 4,096 m world in about half a minute. Then, while this change was being
built: "make sure to lower draw calls, batch trees together, use that sweet sweet ecs framework",
after "something is quite heavy on my gpu".

## What changes

- **The world is ten times the side: 40,960 m square, an island in the sea.** Land fills the
  middle (about 680 km2 of it, forty times the whole old world), its coast pushed in and out by
  noise into bays and headlands, falling over a 3 km shelf of beaches and shallows to a sea floor
  29 to 47 m down: deep enough to sink a 12 m mech, shallow enough for it to walk back ashore. The
  sea goes on past the world's edge to the horizon.
- **Heights are asked for, not stored.** A sample of the ground is a pure function of where it is;
  the 26 million heights a 40 km world at 8 m would store (105 MB) are never made. `height_at` is
  still the one ground query, for the drawn ground at every level of detail, the colliders, the
  pilot and the mechs.
- **The ground is drawn as a quadtree**: 512 m chunks at full detail near the eye, nodes up to 8 km
  across at step 16 far away, every node one mesh of 64 x 64 quads with a skirt. About 90 meshes
  for the whole island (instead of 6,400 chunks), built off the main thread as the eye moves.
  Colliders only near the eye and the loose physics bodies.
- **More of everything, bounded by the player, not the island**: 24 cities and 23 highways (68 km of
  road), 6 rivers, about 6,100 buildings. Trees grow by tile only within 1.5 km of the pilot and
  every mech, and are drawn within 1.2 km of the eye: about 14,000 trees kept and 9,000 drawn,
  against 41,571 in the old 4 km world. Every tree of a species shares one mesh, and all trees share
  one material, so each species is one instanced draw.
- **The sea** is one sheet that follows the eye out to the 40 km far plane; the sea floor beyond the
  world's edge is drawn flat. `water_at` reports the sea wherever the ground is below sea level,
  past the edge too.
- **The view reaches 40 km** (the far plane only culls: Bevy's projection is reverse-Z infinite)
  with 9 km of clear-air visibility.
- **The game starts in the capital**, the largest city, placed within 2.5 km of the island's centre.
- **No floating origin.** Measured: within the island f32 positions cost at most 0.6 px of cockpit
  shimmer at 1080p (design section 5).

## Out of scope

The sea's look (another change ports Pale Blue Dot's water and underwater rendering), swimming and
sinking physics (the mech locomotion change), clouds (off for now: their own change redoes them
cheaply), roads and buildings at a distance (no levels of detail for them yet), and a playtest.
