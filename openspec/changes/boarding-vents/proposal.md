# Proposal: boarding-vents

## Why

The owner, 2026-10-24: "The cockpit should open up with intricate doors and flaps and puffs of steam and myst
coming out. I want getting in and out of a mech to be a beautiful cinematic event", and 2026-10-25: "bring all other
implemented mechs in game up to the gold standard that is the Foreman ... and a cool boarding animation".

In the game every roster mech, the Foreman included, kneels, opens its chest and its cabin, and lowers the crane and
rope (`pilot-boarding`, the crane from `game-loop`), in silence and without a wisp of steam. The steam and mist are
already designed into the models: every mech glb carries 7 to 10 `VENT_*` empties on its chest seams, hinges and sill
(their kind, stage, rate and direction in each empty's extras, written by `tools/blender/mech_boarding.py`), and
every interior glTF carries its own `VENT_inner_open_*` over the floor and at its seals. The Mechs page plays them.
The game's boarding timelines have a `vents` list per stage, but `tools/mechs/gen_mech_data.py` writes none and
`cb-app` plays none.

## What changes

- **The generator writes the vents**, from the glb's and the interior's own empties, on the page's clock: the chest
  seals hiss as they break, the hinges steam as the armour swings, cold mist rolls over the sill and out of the cabin
  and stays while the chest is open, and the leg rams vent as the mech kneels and stands. Every mech gets them, from
  its own model; nothing is typed by hand.
- **The game plays them** through the one particle path effects already use (`FxStream`, `combat-fx`): two recipes,
  `vent_steam` and `vent_mist`, in `data/fx/fx.ron`, and a `Mist` particle that sinks and spreads where steam rises.
- **Steam only while the hatch moves**: a vent's window on the timeline fires while the clock passes through it, so
  a mech that rests sat and sealed (its clock inside the seals' window) does not hiss forever. A vent marked `hold`
  keeps going while the clock rests in its window: the sill's and the cabin's mist, while the chest stands open.
- **Closing vents too**: the timeline runs backwards to close, and the same windows fire on the way back.

## Out of scope

- Sound (the `audio` change plays its `hatch_*` and `steam` cues on these same boarding stages).
- The chest rams aiming at their tips in the game (the page's `aimRams`).
- The retired Breeze's hand-written timeline keeps its vents; its mist gains `hold` so it behaves as before.
