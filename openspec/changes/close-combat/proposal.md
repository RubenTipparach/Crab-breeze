# Proposal: close-combat

## Why

The owner (2026-10-24): "Close combat is automatically toggled on if I'm within melee range of a target
enemy... tab to toggle it off... or toggle it on when I'm not near anybody and just want to hit stuff".
The mech has guns, rockets and missiles but no way to fight hand to hand.

## What changes

- **Close combat, a mode:**
  - It switches on when the targeted hostile comes within melee range (`melee.range_m`, 30 m from
    the pilot's eye). An enemy that is near but not targeted never switches it on (owner,
    2026-10-24: "only hostiles targeted and gets close does this auto melee").
  - Tab toggles it. Tab while it is on turns it off and holds it off until that enemy leaves range.
    Tab with nobody near turns it on.
- **Strikes:** in close combat the fire controls swing the arms instead of firing the guns. Left mouse
  strikes with the left arm, right mouse with the right.
  - Each strike has a wind-up, a hit that sweeps the arm's reach ahead, and a recovery.
  - A strike damages mechs and vehicles through the same damage rules as a shot, and breaks building
    cells and trees in its sweep.
  - A lost arm cannot strike.
- **Display:** the FLIGHT MFD shows `CLOSE COMBAT`.
- **The scripted pilot** switches the same way, so a take shows close fights.
