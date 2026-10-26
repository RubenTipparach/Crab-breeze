# Proposal: mfd-damage-page

## Why

The owner, 2026-10-26, on the in-game seat stills: "I don't see the mech body on the top left mfd", "Where's is the
mech body damage mfd?", "The one that showed mech damage", and "The two far left mfds are the same. I said to make them
different screens".

The cockpit's design pictures (`tools/textures/gen_cockpit_displays.py`, the stand-in images baked onto the screens)
show the mech's body drawn on an MFD, each part coloured by its damage. The game never had that page: its MECH page is
the legs-against-torso ring and a line of text naming hurt parts. And five readout pages fill six screens, so the dash's
left screen opened on MECH, the same as the hung left one.

## What changes

- **A DAMAGE page**: the mech's body drawn as its parts (head, torso, both arms, both legs), each part's outline and
  fill coloured by the structure it has left (green, amber under 70 %, red under 30 %, dark and crossed out when
  lost), the per cent under each limb, and the worst part's line below. It reads the same numbers the damage
  simulation resolves hits with (`cb_core::mech::damage`): one implementation of how hurt a part is, shared with the
  MECH page's text.
- **Six readouts on six screens, all different at start**: MECH, FLIGHT, MISSION hung above; DAMAGE, TARGET, WEAPONS on
  the dash. DAMAGE is a sixth readout after TARGET, so the others keep their places; the MFD texture grows from seven
  pages to eight (MENU and MAIL move up one).
- **The MENU lists DAMAGE** with the other readouts, so any screen can show it.

## Capabilities

- New: `mfd-damage-page`.
