# Proposal: targeting

## Why

Owner, 2026-10-24, watching the city defence take: "You need targeting like on ace combat". Ace
Combat (G4) is how a fight reads at speed: every enemy in a box with its distance, the one you are
after marked apart, an arrow at the screen's edge when it is behind you, and a lead marker that
shows where to put the gun so the shell meets it. Today the cockpit has a crosshair and nothing
else, so a raider 700 m off in the trees is a few grey pixels.

## What changes

- **Target boxes** on the screen wall: a bracket round every live enemy mech within 2 000 m, sized
  to it, with its distance and what it carries (`AC`, `ROCKET`). Raiders shelling the city are
  amber; raiders fighting you are red.
- **The selected target**: the enemy nearest the crosshair, or the next one with `Q` (`target_next`,
  bindable). Its box is solid, with its distance, speed and the health of its torso.
- **The lead marker** for the selected target: where the main gun must point for its shell to meet
  the target, from `cb_core::weapons::lead`, the function the AI pilots aim with (CLAUDE.md
  section 6.1: what the cockpit shows is the number the simulation uses). Put the crosshair on it
  and fire.
- **Off-screen arrow**: when the selected target is outside the view, an arrow at the edge points
  the shortest way round to it, with its distance.
- **Missile lock when in range** (owner, 2026-10-24: "Missile lock when in range"): the selected
  target inside missile range (1 200 m) and within 12 degrees of the aim builds a lock over 1.0 s;
  the box closes in and reads `LOCK`, and `IN RANGE` shows before that. Out of range there is no
  lock, however long the aim stays on it.
- **Lock-on missiles** (Ace Combat, G4): `F` (`fire_missile`, bindable) fires a missile from the
  shoulder pod at the locked target, one each 1.2 s. A missile flies at 260 m/s and turns toward
  its target at up to 70 degrees a second (it can be outrun by a hard turn close in), with the
  motor flame and smoke trail of `rockets`. Unlocked, it flies straight like a rocket. It hits
  through the one projectile path and explodes (2.5 MJ).

## Out of scope

The radar display, flares and decoys, ammunition limits, target priority in briefings, sound.
