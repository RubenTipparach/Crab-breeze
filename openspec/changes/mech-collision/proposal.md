# Proposal: mech-collision

## Why

The owner, 2026-10-26, with a still of the pilot standing inside a kneeling mech's hull: "mech body needs collision so
players cant walk inside their geometry like this". On foot the pilot is pushed out of buildings, rocks and trees, but
a mech is nothing to the walk: the pilot can walk through its feet and shins and stand inside its parts, and the view
then shows the inside of its plating. A mech 24 m tall that the pilot walks through is not there.

## What changes

- **A mech is solid to the pilot on foot**: its feet, legs, hull and arms, posed as they stand now (walking, standing,
  kneeling to be boarded, or a wreck), push the pilot out the way a wall does. The pilot can walk under a thigh or
  between the legs where there is room, and up to the rope's foot to board; they cannot walk into the plating.
- **Each mech's solid shape is measured off its glb**: `tools/mechs/gen_mech_data.py` fills each bone's meshes into
  boxes in that bone's frame and writes them into `data/mechs/<id>.ron` (`body`). Nothing is typed by hand, and
  `cb-tools validate` refuses a file left stale by a rebuild.
- **One push-out for every wall**: the pilot's walk asks the scene (`world::Scene`) for walls as now; the scene also
  holds every nearby mech's boxes, posed by the bones the game already draws, the way it holds the Hermit's deck.
- **The rope's foot is clear**: in the kneel, the pilot's body at the rope's foot touches no box, checked by the
  generator for every roster mech.

## Not in this change

- The mech pushing or hurting the pilot when it walks into them (the pilot is moved out of the boxes, never crushed).
- Standing on a mech (its toes, a kneeling thigh) or climbing it: the boxes are walls, not floors.
- Mech against mech, and the mech against the pilot: unchanged.

## Capabilities

- New: `mech-collision`.
