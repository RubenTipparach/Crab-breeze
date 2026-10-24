# Proposal: boost-and-skate

## Why

Owner, 2026-10-24: "some rockets to jump and some skates to slide around ... boosters last for
about 20 seconds, skates last for about 40." Survey M3 (heat limits flight, fuel ignored for now),
M4 (wheels in the feet, Armored Core style) and M5 (a limited mode the pilot turns on) are decided.

After the first playtest the owner split the two (2026-10-24): "shift always activates skate, so
lateral movement never eats up rockets, rockets is always vertical stabilizers, but wasd normally
doesnt eat up skates. make rocket TWR 2x stronger, skate speed 4x faster, good acceleration curve,
just takes time to build speed."

## What changes

- **Rockets are vertical only.** Hold Space and they lift (thrust doubled: 56 m/s2, a thrust to
  weight of 5.7, climbing at up to 60 m/s); let go and the mech falls. They never push sideways.
  20 s of burn from full; recharge 8 s from empty after a 1.5 s pause; empty, they lock until 30 %
  has come back. Ctrl dives: a faster fall from the air (`water-skimming` gives Ctrl its water use).
- **Skates are all horizontal movement that costs anything.** Shift toggles them, on the ground or
  in the air. On the ground the wheels drive along the legs' heading up to 120 m/s (four times the
  first cut) on an acceleration curve that is strong from a stand and fades toward top speed (about
  5 s to 55 m/s, 15 s to 110 m/s), with low side grip so corners drift. In the air, with skates on,
  WASD pushes the mech sideways, forward or back relative to the view on the same curve and the same
  budget. 40 s of use; recharge 10 s after a 1 s pause; empty, the mech drops back to walking.
- WASD without skates is walking and costs nothing; in the air without skates the mech coasts.
- One velocity for the mech in every mode, so walking, skating and flying carry momentum into each
  other.

## Out of scope

Fuel (M3: ignored for now), wall-running. Skating on water is `water-skimming`.
