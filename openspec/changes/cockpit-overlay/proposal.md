# Proposal: cockpit-overlay

## Why

Owner, 2026-10-24: "make sure trees and explosion, water and geometry never clips through the
cockpit, might be as easy as when player is in mech, cockpit sorting order always on top", and
"also to avoid jitter it draws in camera space?". From the seat, a falling tree, a fireball, smoke
or a river's surface can poke through the cockpit frame, because the cockpit is ordinary geometry
in the world's depth buffer.

## What changes

- **The cockpit draws last, on its own depth**: while the pilot is seated, every mesh of the pilot's
  own mech is on render layer 1, drawn by a second camera after the world with a fresh depth
  buffer. Nothing in the world can draw over or through it; the cockpit's own parts still sort among
  themselves.
- **No jitter**: the cockpit camera is a child of the pilot's camera with the same lens, so both
  views share one transform each frame (the eye is placed from this frame's pose, `socket_now`), and
  the cockpit is drawn from exactly the eye the world is drawn from.
- **Lit on its own** (superseded by `cockpit-lighting`, owner: "cockpit lighting is independent of
  outside world"): the cockpit pass has only the cabin's ambient and the screens' glow.
- On foot the pilot's mech is on the world layer like everything else.

## Out of scope

Shadows cast by the world onto the cockpit (it is sealed: the outside reaches the pilot through
the screens), the cockpit reflecting effect lights.
