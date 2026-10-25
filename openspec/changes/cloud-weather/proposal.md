# Proposal: cloud-weather

## Why

Owner, 2026-10-24, after the lit blob clouds (`cloud-puffs`) were built and shot: "I prefer clouds to
be billboards or a mix of some cheap raymarch", then "I also want more varied clouds. Like make it
look like there's actual weather".

Today every cloud is the same: scattered cumulus at 600-1,100 m, about 35 % cover, evenly over the
island and the sea. Nothing says where the weather is coming from, and the blobs show their facets
up close.

## What changes

- **Billboards.** Each puff is drawn as one camera-facing quad, its edge eaten by noise and lit as
  a sphere (the `cloud-puffs` shading: sun wrap, dark base, silver lining, sky ambient). One quad
  per puff, sorted back to front, replaces the blob meshes. The old sprite clouds were turned off
  for overdraw because they drew seven large quads a puff; this draws one, the size of the puff,
  and discards outside its eroded edge.
- **Weather systems.** The puffs are laid out by a set of rules, each a kind of weather, seeded:
  - fair-weather cumulus over the land, thick over wet ground, thin over the desert;
  - storm cells (cumulonimbus): towers from 700 m to 6.5-10.5 km, an anvil spread downwind, slate
    grey bases, ragged scud below, a rain shaft under each, and darker ground and sea beneath;
  - a stratocumulus deck over the sea to the north-west at 700-1,000 m;
  - orographic cloud banked against the range's windward flank at 900-1,700 m;
  - lenticular caps over the peaks above 1,900 m;
  - cirrus streaks at 8-9 km.
- **The wind** blows from the west at 6 m/s, as today: the west is wet, the east in the rain
  shadow (`island-biomes`), and every system drifts with it.
- **Rain.** Under a storm, in its shaft, the glass takes rain as it does in cloud (`cloud-rain`).
- **Cheap raymarch, later and only if needed:** the storm towers are the one place billboards may
  not hold up close; a short raymarch of the tower's ellipsoids at quarter resolution is the
  fallback, weighed in the design and asked in the survey.

The island mockup (`docs/mockups/island.html`) draws all of this as the reference for the look.

## Out of scope

Weather that changes during a mission (forms, grows, dies), lightning and thunder, wind gusts on
the mech, radar and sensors reading the weather, and true cloud shadows on the ground beyond the
storm cells' darkening (`cloud-puffs` W24 / survey I7).
