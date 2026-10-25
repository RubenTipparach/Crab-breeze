# Proposal: weapon-kinds

## Why

Owner, 2026-10-25: "other mechs will have more advacned weapons like shotguns, rail guns (with a
scope), multi lock on missiels, lasers, plasma cannon (shoots plasmaballs)". Today every mech fires
the same three things (`weapons`, `rockets`, `targeting`): a 35 mm autocannon, a 120 mm cannon and a
shoulder missile. The combat block-outs already hold shotguns and railguns in their hands and on
their backs (`tools/blender/build_mech_blockouts.py`: Hatchet, Brigand, Rampart, Scuttler), but the
simulation cannot fire them.

This change adds four weapon kinds to the one projectile path. The railgun's scope is
`railgun-scope` and the multi-lock missiles are `multi-lock`, each its own change.

## What changes

What the pilot gets, then how:

- **Shotgun**: one pull throws a cone of 12 pellets. Every pellet lands inside 100 m, a third at
  200 m, about one at 400 m. Each pellet is an ordinary projectile.
- **Railgun**: a 1.0 s charge, then one slug at 2 400 m/s that beats almost any armour. Nearly flat
  out to 2 km. Heavy on heat; one shot every 4 s.
- **Laser**: hold the trigger and a beam reaches 1 500 m in no time at all, burning whatever it
  touches for as long as it stays on. No lead needed. It heats fast and shuts down when hot.
- **Plasma cannon**: a slow glowing ball (180 m/s) that flies dead straight and bursts with a big
  splash, on whatever it meets or at 650 m. Easy to see coming and dodge at range; deadly close.
- **Weapon heat**: the railgun, the laser and the plasma cannon share a weapon heat bank that empties
  as they fire and refills over time. Full heat locks them until it cools. It is the same budget the
  boosters already use (`boost-and-skate`), one implementation, with its own numbers.
- **In data, on any mech**: `WeaponSpec` gains a `kind` (`shell`, `rocket`, `pellets`, `slug`,
  `beam`, `plasma`) and the fields that kind needs. The existing `rocket: bool` becomes `kind:
  rocket`. The player flies whatever their mech carries; enemies use the same weapons through the
  same `fire` actions.
- **A fourth fire group** for back units (`fire_back`, G and Mouse4 by default, bindable): arms on the
  mouse buttons, the missile on F, back weapons on G.
- **The cockpit shows the simulation's numbers**: the weapons MFD page lists each weapon with ready,
  charging or locked, the railgun's charge and the heat bank, read from the same state the shots use.

## Out of scope

The scope (`railgun-scope`), multi-lock (`multi-lock`), ammunition counts, swords and melee, shields,
the weapons' sounds (named here for `audio`, built there) and models (the block-outs already carry
them; detailed models come with the mechs).
