# Proposal: water-skimming

## Why

Owner, 2026-10-24: "when on water you dont sink if you have any skate power. this should let me
skim along the water surface ... if skate runs out and rocket runs out (rocket is used as backup on
water if skate runs out) on the water, the mech sinks and systems recharge. CTRL can be used to
force a sink, or be used to descend faster from the air." The island world (`island-world`) puts
ocean all round the land, so water is now somewhere the pilot goes, not a wall.

## What changes

- **Skimming.** At the water's surface, with skate power left, the mech rides the surface on its
  wheels and does not sink. On deep water the skates switch on by themselves.
- **Rockets as the backup.** On deep water with the skates empty, the rockets fire by themselves to
  hold the mech at the surface, on their own budget; the legs paddle it along at walking speed.
- **Sinking.** With both empty, or with Ctrl held, the mech sinks (4 m/s, water braking motion at 30 m/s2) and walks the bottom at
  wading speed while both budgets recharge. Space lifts it back up through the water (10 m/s) once
  the rockets relight; at the surface, with skate power, it skims again.
- Deep water is no longer a wall.

## Out of scope

Buoyancy by hull volume, waves that move the mech, water damage to systems.
