# Proposal: audio

## Why

Owner, 2026-10-24: "when firing rocket thrusters there should be some rubling, maybe its time to
search for sfx", with the sources to search: OpenGameArt and Freesound. The game is silent. A 50 t
mech is half sound: the stomp of a foot, the whine of the torso, the roar and rumble of the rockets
under you, the crack of the autocannon, the thump of a shell landing 300 m off.

## What changes

- **One sound plugin** (`cb-app::audio`) on Bevy's audio: sounds are named cues in
  `data/audio/cues.ron` (files, volume in dB, pitch range, world or cockpit, loop), played by name
  with a message, the way effects are recipes (`combat-fx`). Simulation events ask for them: a shot,
  an impact by surface, an explosion by energy, a footfall by surface, the rockets lighting and
  burning, the skates rolling, the hatch's stages, a lock tone, a missile launch.
- **Rocket rumble**: while the rockets burn, a low engine loop in the cockpit rises with the lift,
  a sub-bass rumble under it, and the cockpit's position shakes by a few millimetres with it
  (position only: the view's direction stays the player's, CLAUDE.md section 8; survey G8).
- **Distance and the cockpit**: outside sounds are 3D, delayed by distance at 343 m/s (a shell
  bursting 700 m off is heard 2 s after its flash), low-passed with distance, and muffled through
  the sealed cockpit; cockpit sounds (lock tone, alarms, servos, the rocket rumble) are not.
- **Sources** (`assets/audio/`, each file with its source page, author and licence in
  `assets/audio/SOURCES.md`): CC0 only unless the owner allows more (survey P4). OpenGameArt first
  where it has the sound; Freesound for the rest (it needs the owner's login).

## Out of scope

Music, voice and radio chatter (G4's radio is its own change), a mixer UI, occlusion by buildings.
