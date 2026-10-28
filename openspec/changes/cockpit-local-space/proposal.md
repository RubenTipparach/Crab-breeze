# Proposal: cockpit-local-space

## Why

The owner, 2026-10-28: "is the cockpit drawn in game space? is that why it shakes due to imprecision", "is cockpit
drawn in local camera space?", "also is cockpit rotation smoothed? to avoid stutters and jitters?", then "fix it, also
we'll need to make sure it is smoothly rotating (probably measure camera rotation a swell".

Two causes, read in the code:

1. **Precision.** The cabin is the mech's own meshes, drawn by the cockpit camera where the mech stands in the world,
   thousands of metres from the origin: 32-bit floats there step 0.5 mm (4 to 8 km) or 1 mm (8 to 16 km). The eye is
   placed by its own chain of transforms (`mech::socket_steady`), the cabin by Bevy's transform tree, and the GPU
   rounds the cabin's world positions again: each lands on its own grid. A panel 0.6 m from the eye moving 0.5 to
   1 mm moves about 1 to 2 pixels at 1080p and 75 degrees: a shimmer of the whole cabin, worse farther from the
   map's centre.
2. **The seated look steps.** Seated, the view is the mech's heading blended between fixed steps plus the pilot's look
   relative to the nose (`seated_rel_rad`) from the last step alone. The mech turns toward the view, so each step the
   nose gains what the relative look loses; drawn, the heading glides and the relative look jumps, and the view lags
   by up to one step's turn and snaps back (about 0.7 degrees a step at 45 degrees a second and 64 steps a second):
   the view, and the cabin against it, saw-tooth while the mech turns.

## What changes

- **The cockpit pass draws in the mech's own frame.** While seated, the cabin's meshes and lamps are drawn by
  stand-ins sharing their meshes and materials, placed by each part's chain of local transforms up to the mech's
  root, never through a world position; the cockpit camera sits in the same frame at the seat's eye, turned as the
  view is. Every number in the pass is a few metres, wherever the mech stands. The world pass is unchanged.
- **The seated view is blended as the cabin is**: the look's own direction between the two steps, plus the mouse not
  yet stepped, so the view and the cabin both glide.
- **The view's turn is measured**: each frame's drawn yaw and the cockpit's, logged by an instrument, so a saw-tooth
  is a number, not an impression.

Not changed: what the pilot sees (the same cabin, the same view direction), the law (first person, the player's own
look; nothing turns the view but the player and what carries it).
