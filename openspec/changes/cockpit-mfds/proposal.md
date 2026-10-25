# Proposal: cockpit-mfds

## Why

The mech's readouts (leg compass, damage, booster and skate heat, flight, guns, mission, navigation)
are a 2D prototype drawn over the view (`cockpit-hud`). The roster's interiors now carry three MFDs
(`HUD_mfd_0..2`, each with a `_screen` face). The owner (2026-10-24): "make sure we move mech UI stuff
into interactive mfds". CLAUDE.md's law puts the instruments in the cockpit.

## What changes

- The readouts are drawn into one texture, three pages side by side (MECH, FLIGHT, MISSION), by one UI
  camera, and each MFD's screen shows one page.
- Each MFD changes page when the pilot holds free look, looks at it and clicks. The guns do not fire
  while free look is held.
- The crosshair, target boxes, lead ring and edge arrow stay on the view (the screens the pilot looks
  out through).
- A cockpit with no MFDs (the Canopy) keeps the readouts along the bottom of the view.
