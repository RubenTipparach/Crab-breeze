# Proposal: sun-flare

## Why

Owner, 2026-10-24: "do we have a sun? and epic lensflare?". Since `sky-atmosphere` there is a sun
disk in the scattering sky, but nothing glows: the view has no bloom, so the sun is a small hard
white dot, and there is no lens flare at all. The pilot looks out through camera screens (on foot,
through their own eyes); a sim cockpit seen through cameras is exactly where a cinematic flare
belongs.

## What changes

- **Bloom** on the pilot's eyes camera (Bevy's `Bloom`, energy-conserving, its settings in
  `data/world/sun_flare.ron`): the sun disk and the bright sky glow softly; sunlit ground barely
  softens. The cockpit camera does not bloom, so the cabin and its panels stay crisp; Bevy UI is
  drawn after the post-process passes and is never bloomed.
- **A lens flare**, our own post pass on the pilot's eyes, drawn into the linear HDR image after the
  bloom and before the water's passes and tonemapping. Every element is a function of the sun's
  screen position and the screen centre: a warm core and glow at the sun, a starburst of spikes and
  fine rays that wheels slowly as the view swings, a thin cool anamorphic streak, a faint halo ring,
  and a chain of ghosts (discs and the aperture's hexagons, cool tints, chromatic fringes) on the
  line from the sun through the centre. Every colour and size is in data.
- **It fades when it should**: when a building, the mech, terrain or the cockpit's frame hides the
  sun (17 depth taps round the sun, on the world's depth and on the cockpit's own), as the sun
  leaves the view (with a short margin so the ghosts linger), and in cloud or under water at the
  eye. Its strength follows the sun's light and the camera's exposure.
- It only adds light: it never moves or turns the view (CLAUDE.md section 8).
- `CB_NO_FLARE=1` leaves the flare out and `CB_NO_BLOOM=1` the bloom, so the suite can measure
  them.

## Out of scope

Lens dirt and scratches on the screens, a flare from other lights (explosions, muzzle flash,
searchlights), the atmosphere's reddening of a low sun in the flare's colour, auto exposure.
