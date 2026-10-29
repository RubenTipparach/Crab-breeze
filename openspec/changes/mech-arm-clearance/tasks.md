# Tasks: mech-arm-clearance

- [x] 1.1 Measure every detailed mech's arms against its legs through the kneel, triangle against triangle
- [x] 1.2 A kneel only arm offset in the pose layer: Foreman, Harrow, Roustabout and Mule clear, nothing else changed
- [x] 1.3 The Mechs page shows it (published 2026-10-29; the earlier model and always-on versions undone there first)
- [ ] 1.4 Owner approves the kneel
- [ ] 1.5 The game's kneel carries the offset: `SitPose` gains `arm_out_deg`, `arm_forward_deg`, `elbow_deg`, written by
      `tools/mechs/gen_mech_data.py` into each `boarding.ron`, applied by the leg solver's sit blend
- [ ] 1.6 Playtest: each of the four kneels to be boarded, arms clear of the thighs, seen from the walk up
