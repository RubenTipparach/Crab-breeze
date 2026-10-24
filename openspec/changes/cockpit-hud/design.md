# Design: cockpit-hud

Bevy UI, one plugin (`cb-app::hud`), visible only seated. Reads `MechSim` (state, damage, guns) and
the mech's data. Leg compass: a ring with a fixed torso marker at the top and a leg arrow rotated
by minus the torso twist, so it points where the legs walk relative to the view. Gauges: a bar per
budget, red when locked, with `12.4 s left` or `recharge 6.0 s`. Cost: a few dozen UI nodes.
