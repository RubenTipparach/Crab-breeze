# Proposal: skim-depth

## Why

Owner, 2026-10-24: "If water isn't deep the mech can just wade or walk in the water. The skis only
kicking if you're above water deeper than 50% of the mech".

The skates skimmed only past `max_wade_depth_m`, 13-18 m on the 24 m mechs (54-75 % of their
height). A mech wading out from the shore with its skates on never came up to skim: its feet
stayed on the bed.

## What changes

- The skates ride water only deeper than `skim_depth_share` (0.5) x the mech's height.
  Shallower water is waded or walked, skates or not.
- A mech wading with its skates on into water that deep rises to the surface at `skim_rise_m_s`
  (6 m/s), not a snap, and skims.
- Without skate power nothing changes: it wades to `max_wade_depth_m`, then sinks and walks the
  bottom, or the rockets hold it up.
