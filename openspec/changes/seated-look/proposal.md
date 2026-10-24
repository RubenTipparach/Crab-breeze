# Proposal: seated-look

## Why

Seated, the pilot's view turns freely all the way round while the mech's nose stays put. Turning
the view fast left the mech, its torso and the view pointing three different ways ("you can't turn
and cause the mech's rotation to be confused"), and the scripted pilot's view spun wildly chasing
targets behind it. The owner (2026-10-24): "The mouse should be confined to 180 degrees of the front
of the mech (90 each side) such that its rotation then becomes limited by turning speed of the
mech", "look turn should not be able to look directly behind the pilot. Free look would allow that
when the look is no longer coupled to the mech".

## What changes

- **Coupled look (seated, the default):** the view stays within 90 degrees of the mech's nose. Mouse
  input past the edge is held at it, and while the view is at the edge the mech turns toward it at
  its own turn rate, so the view comes round no faster than the mech can turn.
- **Free look (held, Left Alt by default):** the view is not coupled: it can look anywhere, behind
  the pilot included, and the mech does not turn for it.
- **Leaving free look** with the view past 90 degrees: the view stays where it is (only the player
  turns the view, CLAUDE.md section 8) and the mech turns toward it at its turn rate until the view is
  inside the limit again.
- On foot, boarding and climbing out are unchanged.
- The scripted pilot sends the same look input, so it is held to the same limit.
