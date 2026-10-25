# Crab Breeze music: the motifs and the rules of the score

Read this before writing, arranging or rendering any music for the game. It holds the musical
world, every motif with its notes and what it means, and the cues built from them so far. The
method (Strudel score, multitrack MIDI, LMMS headless with real instruments) is the
`music-authoring` skill. The canonical text of every motif is the `MOTIF LIBRARY` block of
`music/motifs.strudel`; the notes below are that block written out for reading.

Listen to everything on the Crab Breeze Music page (the link is in `docs/design/README.md`).

## The musical world

- **Mood: an edgy rebel, not a hero** (owner, 2026-10-25, three rounds: "music sucks, electronic,
  scifi futuristic please!", then "Theme sucks..I want some thing moody,more cyber punk themed",
  then "It's too cherry and heroic. I want to feel like an edgy rebel"). Moody, dark cyberpunk:
  rain on the city, a snarling riff, a melody that sinks instead of soaring, half-time drums with a
  big snare. **Nothing cheery, nothing heroic, nothing orchestral, no triumphant major chord.**
  The town theme (Main Street) is deleted ("Main street sucks delete it").
- **The voices** are LMMS synths, chosen per role in `music/instruments.json` `voices`: smoky lead
  `gm_lead_5_charang` (Erazzor), big lead `gm_lead_2_sawtooth` (SuperSawLead), gritty riff
  `gm_lead_7_fifths` (SawReso), dark pad `gm_pad_choir` (CryingPads), pad `gm_pad_poly`
  (TINTNpad), Vangelis brass pad `gm_synth_brass_1` (Jupiter), pluck and arpeggio
  `gm_lead_1_square` (PluckArpeggio), reese `gm_synth_bass_2` (DirtyReece), bass
  `gm_synth_bass_1` (FuzzyAnalogBass), acid `gm_lead_8_bass_lead` (LB302), sick bell
  `gm_fx_crystal` (Monstro ScaryBell), swells `gm_fx_sci_fi` (Supernova); rain `gm_fx_rain` from
  the soundfont; drums the soundfont's 808/909 kit (`@drumkit 25`) with reverb on the snare
  (`@space drums`). A score names the role by its GM name, so strudel.cc still plays a preview.
- **Home key: D minor, with the flat two.** E flat against D is the edge: the hook leans on it, the
  spine moves to an E flat chord, the crabs grind on it.
- **The spine** is 8 bars that never resolve to major. The last chord is A minor (the fifth made
  minor), so every loop falls back into D minor without a victory:

  ```
  Dm(add9) | Dm(add9) | Eb(maj7) | Eb(maj7) | Bb(maj7) | C(sus) | Asus4 | Am
  ```

- **The stride is the mech's gait: 3 + 3 + 2 eighths** ("stomp, stomp, step-step"), onsets on
  beat 1, the "and" of 2, and beat 4.
- **The crabs** are chromatic (D against E flat), the tritone (D against A flat), and a scuttling
  rhythm of sixteenths grouped 3 + 3 + 3 + 3 + 2 + 2.
- **Tempi**: 96 BPM half-time for the title and the getaway, 140 to 150 BPM for the fights, 72 for
  boarding. One cycle of Strudel is one 4/4 bar (`setcps(BPM/60/4)`).

## The motifs

Notes are scientific pitch (C4 is middle C). Durations are in eighths unless marked: `@3` is a
dotted quarter, `@2` a quarter, `@8` a whole bar; riffs are in sixteenths, `.` a rest.

### 1. The Rebel: the mech and its pilot

**The riff** (2 bars of sixteenths, low): D with the flat three, the flat five (A flat) and the
flat seven, snarling and syncopated. A gritty saw an octave up, a reese at pitch.

```
D2 . D2 F2 . D2 G2 . Ab2 . G2 F2 . D2 . .
D2 . D2 F2 . D2 C3 . Bb2 . Ab2 G2 F2 . D2 .
```

**The hook** (8 bars over the spine): it leans on the flat two in its first bar and sinks, ending
on A without resolving.

```
Dm   D5@3 Eb5 D5@2 C5@2 | Dm   A4@6 .@2          | Eb    Bb4@3 C5 Bb4@2 G4@2 | Eb  Eb5@6 .@2
Bb   D5@3 C5 Bb4@2 A4@2 | C    G4@3 A4 C5@2 E5@2 | Asus4 D5@6 C5@2           | Am  A4@8
```

Use them: whenever the mech is the subject. The riff alone says "the rebel is here"; the hook is
the theme. On a smoky lead it broods; on supersaws over the riff it fights.

### 2. Call Sign: the radio

"CB" in Morse code (`-.-.  -...`) as a rhythm of sixteenths: C on A5, B on D5. A dah is three
sixteenths, a dit one, with Morse's own gaps.

```
A5@3 . A5 . A5@3 . A5 . . .  D5@3 . D5 . D5 . D5 . . . . . . . . .
```

Use it: briefings, radio traffic, the mech's systems waking. An echoing pluck makes it a radio; a
sick bell makes it the cockpit lighting up. The boot-up sound effect beeps it, and the lock-on tone
in `openspec/changes/audio` can take its rhythm.

### 3. The crabs: Scuttle, Claw and Siren

- **Scuttle** (2 bars of sixteenths, grouped 3 + 3 + 3 + 3 + 2 + 2, accents on each group):
  ```
  D2 Eb2 D2 | D2 Eb2 D2 | Ab1 A1 Ab1 | D2 Eb2 D2 | F2 E2 | Eb2 D2
  D2 Eb2 D2 | D2 Eb2 D2 | Ab1 A1 Ab1 | D2 Eb2 D2 | Bb1 A1 | Ab1 G1
  ```
  An acid bassline over a reese. The crabs on the move.
- **Claw**: the tritone D4 + A flat 4, stabbed on the scuttle's accents (sixteenths 1, 7, 13, 15).
- **Siren**: A4, A flat 4, G4, F sharp 4, one bar each: the crabs' wail, on a dark pad.

Use them: whenever the crabs are near. In the fight the rebel takes the scuttle's rhythm into its
own bass.

### 4. The Hatch: boarding (10 bars)

Stacked fourths, climbing a step for each stage of the chest opening (two bars a stage), then
landing on a dark D minor (add 9) as the pilot sits:

```
D3 G3 C4 F4 (x2) | E3 A3 D4 G4 (x2) | F3 Bb3 E4 A4 (x2) | G3 C4 F4 Bb4 (x2) | A3 D4 G4 C5 | D3 A3 E4 F4
```

### 5. Getaway: a mission done (4 bars)

No parade. B flat, C, then D minor held: the hook's edge in four grim, defiant bars.

```
Bb  F5@3 Eb5 D5@2 Bb4@2 | C  C5@3 D5 E5@2 G5@2 | Dm  F5@3 E5 D5@2 A4@2 | Dm  D5@8
```

## The cues

| Cue | Score | Where it plays | Tempo, length | Built from |
| --- | --- | --- | --- | --- |
| The motifs | `music/motifs.strudel` | Reference only: every motif in turn | 110 BPM, 45 bars, 1:38 | All |
| Main Theme | `music/main_theme.strudel` | Title screen | 96 BPM, 52 bars, 2:15 | Rain and the call sign; the riff; the hook on a smoky lead with a brass pad; a breakdown in the rain; the hook on supersaws over the riff; the riff fading |
| Boarding | `music/boarding.strudel` | The boarding cinematic | 72 BPM, 14 bars, 0:53 | Hatch over a drone with steam and swells, the call sign, the hook beginning |
| Sortie | `music/sortie.strudel` | Combat; loops | 150 BPM, 64 bars, 1:44 | Call Sign, the riff over breakbeats, Scuttle, Claw, Siren, the hook on supersaws |
| Big Boy | `music/big_boy.strudel` | The siege crab set piece (survey G14) | 140 BPM, 48 bars, 1:24 | Scuttle as a march and on acid, Claw, Siren, the rebel's riff and hook |
| Getaway | `music/mission_accomplished.strudel` | After a won mission | 96 BPM, 6 bars, 0:18 | Getaway, the rain |

There is no music on foot for now: Main Street was deleted, and walking plays the world's sound.

Each score's `.mid` (for any DAW) and `.mmp` (the LMMS project) sit beside it in `music/`; the
game's copies are `assets/audio/music/<cue>.ogg`, brought to -16 LUFS by a plain gain and a peak
limiter, so each keeps its build from quiet to loud. How they play in the game (which cue when,
crossfades, loops) is `openspec/changes/music`, not built yet.

## Rules for new music

- **Build from the motifs.** A new cue quotes at least one motif, in the key and harmony above,
  and copies the `MOTIF LIBRARY` block word for word (`node tools/music/check_motifs.mjs`). A new
  motif goes into `music/motifs.strudel` and this file first, then into cues.
- **Stay the rebel.** Minor and modal, the flat two, chords that do not resolve home in triumph,
  half-time weight. If a cue sounds like a victory parade or a happy town, it is wrong.
- **First person holds** (CLAUDE.md, the law): music never cuts the view or the input. A stinger
  plays over the game; it does not stop it.
- **Mix for the cockpit.** Leads in the middle, no sub-bass mud under the rocket rumble and
  footfalls (`openspec/changes/audio`): the lowest parts are the reese and the kick, and a cue is
  checked under the game's sound once both exist.
- **Know what is proven.** A render is checked by numbers (no clipping, the loudness profile has
  the cue's shape) and by a spectrogram that shows the lead line; neither replaces the owner's
  ears. Say so when handing music over.
