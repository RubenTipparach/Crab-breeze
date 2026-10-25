# Proposal: cockpit-parity

## Why

The owner, 2026-10-25: "bring all other implemented mechs in game up to the gold standard that is the Foreman,
rendered cockpits with baked lighting, fully functioning MFDs, and a cool boarding animation".

The eight roster mechs sit in four interiors. From the seat (`docs/screenshots/cockpit-smear-fix/`):

- **Panorama** (Foreman, Driller, Picket) is the standard: three MFDs hung from the front screen's top bar, a dash
  under the screen with three more, a glare shield, side consoles, all baked at 4096 px and lit by its own lamps.
  Since `weapon-select` all six screens are live (MECH, FLIGHT, MISSION above; MECH, TARGET, WEAPONS below).
- **Titan** (Roustabout, Sapper) has the same six screens and dash. It is at the standard.
- **Excavator cab** (Ironhorse, Mule) has no dash at all (its layout asks only for the top MFDs), so it has three
  live screens, and the rail between its two screen rows runs at 1.115 m, 8.5 cm under the eye: a beam straight
  across the middle of the view, against the owner's rule of no frame member within about 40 degrees of straight
  ahead.
- **Canopy** (Harrow) has the dash's three screens but no hanging MFDs, so the game drew the pages as a strip along
  the bottom of the view until `weapon-select` bound the dash. Nothing of the cabin shows from the seat in its
  still: the dash sits under the bottom of the view.

## What changes

- **The Cab gets the Panorama's dash** (`dash_unit`, the same three MFDs, glare shield and brackets) riding its
  ramp, so it folds down with the front and becomes the step out, as the Titan's does. Its rows move so the seam
  rail sits at 0.97 m, behind the glare shield's lip seen from the eye.
- **The Canopy gets the three hanging MFDs**, hung above its low front screen on the canopy's front slope (its
  screen row ends at 1.45 m, so MFDs hung below the bar as in the Panorama would cover the screen at eye height).
- **Both are rebuilt and baked** at the Panorama's settings (4096 px atlases, 16 contact occlusion samples,
  denoised), with the same lamps.
- **Every interior has the same six screens**: `HUD_mfd_0..2_screen` and `SCREEN_left/center/right_screen`. A
  check opens every interior a mech's data names and refuses one missing a screen, `EYE_pilot`, `SEAT_pilot` or
  its lamps, so a cockpit can never again ship with fewer readouts than the Foreman's.
- The fallback strip along the bottom of the view stays for a mech with no interior (the stand-in), and only for it.

## Out of scope

- The Panorama and the Titan: no change.
- The chests: the envelopes, the opening mechanisms and the riding parts are unchanged, so no mech is rebuilt.
- New pages: the six screens show the pages that exist (`cockpit-mfds`, `weapon-select`).
