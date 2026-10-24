# Proposal: cockpit-hud

## Why

Owner, 2026-10-24: "I need to be able to see which way the legs are turning here, so some quick
rapid ui prototyping in the cockpit is needed, and show the heat on my boosters, and heat on my
skates, how much time I have left to use each feature, and how long it takes to recharge."

## What changes

A 2D HUD prototype over the cockpit view (it moves onto the cockpit's own screens with
`mech-boarding`'s all-view screen later; CLAUDE.md wants instruments in the cockpit, this is the
owner's rapid prototype): a crosshair; a leg compass (the legs' heading against the torso, with the
turn direction and speed); booster and skate gauges (heat, seconds left, seconds to recharge,
locked); throttle; speed; altitude; mode; guns ready and reloading; part damage. Every number shown
is the simulation's own (CLAUDE.md section 6.1).

## Out of scope

The cockpit's 3D instruments and screens, radar, lock-on.
