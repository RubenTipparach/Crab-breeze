# Proposal: music

## Why

Owner, 2026-10-25: "we need audio, read tom lander repo for how music is authored, make a skill of
that for our repo ... and make me some awesome motifs". The game is silent, and `audio` (sound
effects) leaves music out of scope. An Ace Combat style game (survey G4) lives on its score: a
theme for the rebel in the cockpit, the enemy's music when the crabs close in, a grim getaway
when the city is saved (the owner wants an edgy rebel, not a hero: `docs/design/music.md`).

The score's motifs, its first seven cues and the pipeline that makes them now exist
(`docs/design/music.md`, `music/`, the `music-authoring` skill, `assets/audio/music/*.ogg`). This
change is how the game plays them.

## What changes

- **One music plugin** (`cb-app::music`) plays one cue at a time from `assets/audio/music/`, chosen
  by the game's state: title, boarding, combat, the set piece, mission won; on foot is silent. The states and
  their cues live in `data/audio/music.ron`, not in code.
- **Crossfades on the bar.** A state change waits for the current cue's next bar line (the cue's
  tempo is in the data) and crossfades over one bar, so the music never lurches; stingers (mission
  accomplished) play once and hand back.
- **Loops at the seams.** Looping cues (title, combat) loop at their last bar; the scores already
  end where they begin (the spine ends on A minor and falls back to D minor).
- **Music under the cockpit.** The score is not in the world: it is not delayed, placed or muffled
  like `audio`'s world sounds, and it ducks by 6 dB under the radio and alarms.
- **Volume** is its own channel in the settings, with sound effects separate.

## Out of scope

Adaptive layers (stems that rise with the fight: survey A2 decides), music heard through the
cockpit's radio (A3), voice and radio chatter, a music player menu.
