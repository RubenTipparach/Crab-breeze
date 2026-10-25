# Proposal: multi-lock

## Why

Owner, 2026-10-25: "multi lock on missiels". Ace Combat's special weapons (G4) fire at several
targets at once: sweep the aim across a group, each one boxes in turn, release and a missile goes to
each. Crab waves come in groups of ten (`city-defence`, `crab-enemies`), and Big Boy has several
weak points (`WEAK_*`), so a multi-lock pod is the answer to a swarm and to the set piece.

## What changes

- **A multi-lock pod** (six cells): hold `fire_missile` (F) and sweep the aim. Every target inside
  the pod's range (1 500 m) and its wide cone (25 degrees) gains a lock in turn, one every 0.35 s,
  nearest the aim first, up to six. With fewer targets than cells, the locks go round again, so a
  lone target can take all six. Release F and one missile flies to each lock, 0.08 s apart.
- **Locks can be lost**: a target that leaves the range, or the cone by half as much again (37.5
  degrees), loses its locks before release.
- **The cockpit counts them**: each locked target's box shows its count (`x2`), the reticle fills a
  ring of six pips, and the pod reads RELOAD for its 8 s after a salvo.
- **One lock rule for every missile**: the lock's numbers move from `data/hud/targeting.ron` into each
  missile's `seeker` in its `WeaponSpec`. Today's shoulder missile becomes a seeker with one lock
  (1 200 m, 12 degrees, 1.0 s), so single and multi lock are one implementation with different data.
- **Big Boy's weak points** (`WEAK_*`) are targets for locks, each its own.
- **Enemies use it** through the same F action.

## Out of scope

Flares and decoys, missile ammunition counts, a separate special weapon selector (the pod is the
mech's missile group).
