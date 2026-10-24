# Proposal: cockpit-lighting

## Why

Owner, 2026-10-24: "muzzle flash from the mech should not appear in cockpit, in fact cockpit lighting
is independent of outside world" and "it should be lit up by the screens perhaps a cube map made from
the screens". The cockpit is sealed (CLAUDE.md section 1): no daylight, no flash of a gun or a blast
reaches the pilot except through the screens. Today the sun lights the cockpit pass, and the world's
lights could too.

## What changes

- **The cockpit pass has its own lights only**: the sun, the explosion and muzzle flashes and every
  other world light stay on the world layer. The cockpit pass gets its own dim ambient (the cabin's
  lamps) and the screens' glow.
- **Screen glow, first step**: a soft light in front of each screen (front, left, right), on the
  cockpit layer, in the screens' phosphor tint.
- **Screen glow, the real thing (the owner's cube map)**: a small cubemap of what the screens show,
  rendered from the eye on the world layer (64 px a face, one face a frame, so each face is six
  frames old at most), filtered on the GPU by Bevy's `GeneratedEnvironmentMapLight` and given to the
  cockpit pass as its environment light. A fireball on the front screen then washes the panels
  orange from the front; flying into a cloud turns the cabin grey; night darkens it.
- The screens' brightness is the one number: the same view the pilot sees drives the light (CLAUDE.md
  section 6.1).

## Out of scope

Baked cabin lamps (`light-baking`), instrument backlights (the cockpit change), the HUD on the
screens.
