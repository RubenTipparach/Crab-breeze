# Proposal: mech-boarding

## Why

The owner, 2026-10-24: "The cockpit should be enclosed fully. We can view outside via screens like
in gundam. For the mockups have the mech sit down on the ground so that pilots can enter/exit. The
cockpit should open up with intricate doors and flaps and puffs of steam and myst coming out. I
want getting in and out of a mech to be a beautiful cinematic event."

`pilot-boarding` carries the pilot up a lift into an open cockpit. This change replaces the lift
with the mech itself: it sits down, opens its chest in stages with steam and mist, the pilot walks
up the stair it lowers, sits, and watches the doors seal and the screens come alive. The law holds
throughout: the camera is the pilot's eyes; the cinematic is what they see, not a cut.

## What changes

- The cockpit is a sealed pod. Above the consoles its inner wall is one panoramic screen (the
  all-view screen, Gundam's all-view monitor) showing the mech's cameras, with the HUD drawn on it.
- The mech sits down to board: legs fold, hips to the ground, torso bowed, so the hatch sill comes
  down to about 3 m.
- The hatch opens in stages, each a named node the game animates: latches retract, the outer
  armour opens as a clamshell (upper visor up, lower plate down) with side flaps swinging out,
  blast shutters retract, a stair unfolds to the ground, the front console swings up to clear the
  way. Steam vents at every seam and hinge; cold mist pours out and rolls down the stair.
- Boarding: walk to the stair, climb it, step in, turn, sit. The doors close in reverse, the cabin
  pressurises (a hiss, a last puff), and the all-view screen powers up tile by tile.
- Disembarking reverses it: the screens go dark, the doors open, the pilot climbs out and the mech
  can stand.
- The timeline lives in data (`data/mechs/<mech>/boarding.ron`): each stage's nodes, angles,
  start times and durations, vents and their bursts. The mockup's timeline is the first draft.

## Out of scope

The pilot's body and hands, sound, and the simulation behind the start-up checks (reactor, power):
their own changes. Exterior light on the opened hatch uses the game's lighting as it is.
