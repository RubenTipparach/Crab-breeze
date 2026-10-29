# Proposal: cockpit-shake

## Why

The owner, 2026-10-28: "now we need some thruster shakes, impact shakes, skate shakes the cockpit (technically camera)
but you get the idea, I want to the player to feel the weight of this mech when it hits things".

Today the view shakes for one thing only, blasts nearby (`pilot::shake_on_blasts`, `fx.shake` in data): a trauma value
kicked by a blast's energy and distance, decaying over 0.7 s, felt as its square as a three-sine wobble of up to 1.2
degrees on top of the player's view. A mech of 24 m landing from a jump, ramming a building, taking a shell, firing a
cannon, lighting its jets or skating at speed does nothing to the view: it feels weightless.

## What changes

The seated view (the cockpit pass and the world pass alike, from the one eye) shakes from the mech's own body, in three
channels that add on top of the player's view and never into their aim (CLAUDE.md section 8):

- **Kicks** (the trauma that exists, more sources): landing, ramming, being hit, the mech's own guns firing, a melee
  strike landing, the jets lighting.
- **Rumble**, held while it lasts: the jets by their thrust, the skates by their speed and the ground under them.
- **Jolt**, the weight: the eye dips and springs back on a landing or a collision, by how hard, a few centimetres, a
  damped spring.

Footsteps stay off the view (owner, 2026-10-25: "I would remove the footstep motion forom the cokcpit entirely"). Every
number is in data; a settings slider scales all of it, 0 to 1.

Not changed: on foot and driving (those views shake from blasts only, as now); the aim; the law.
