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
  rain on the city, a snarling riff, a hook built like a classical sentence, hard 90s drums. **Nothing cheery, nothing heroic, nothing orchestral, no triumphant major chord.**
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
- **Tempi**: 100 BPM big beat for the title and the getaway, 140 to 150 BPM for the fights, 72 for
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

**The hook** (8 bars over the spine) is a sentence (see "Writing a melody" below). In
sixteenths: `@2` an eighth, `@3` a dotted eighth, `@4` a quarter, `@8` a half.

```
basic idea      Dm   A4@2 A4@3 C5@3 D5@4 C5@2 A4@2   | Dm  G4@3 F4@3 E4@2 D4@8
again, up       Eb   Bb4@2 Bb4@3 D5@3 Eb5@4 D5@2 Bb4@2 | Eb  Ab4@3 G4@3 F4@2 Eb4@8
continuation    Bb   D5@2 D5@3 F5@3 G5@4 F5@2 D5@2   | C   E5@2 E5@3 G5@3 Bb5@4 A5@2 G5@2
high point, out Asus4 D6@6 C6@2 Bb5@4 A5@4           | Am  A5@3 G5@3 E5@2 A4@8
```

One rhythm (a syncopated figure: onsets on sixteenths 1, 3, 6, 9, 13, 15) runs through bars 1, 3,
5 and 6; the tails of bars 2, 4 and 8 walk in the mech's stride (3 + 3 + 2) and land on a long note.
Bar 3 is bar 1 moved up to the E flat chord, the edge. Bars 5 and 6 climb a step at a time to the
single high point, D6, at bar 7 (three quarters of the way through), and the tail falls a fifth to
A4, the note bar 1 starts on, so the hook loops without a seam.

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

No parade. The hook's figure on B flat and C, its stride tail on D minor, then D held.

```
Bb  F5@2 F5@3 D5@3 Bb4@4 D5@2 F5@2 | C  G5@2 G5@3 E5@3 C5@4 E5@2 G5@2 | Dm  A5@3 G5@3 F5@2 E5@8 | Dm  D5@16
```

## Writing a melody

The owner's note on the first rebel hook: "the melody is just garbage. Look into music theory and
how to compose good melodies." What the theory says, and the rules every melody here follows:

- **Build from one short idea.** A melody is a 1 to 2 bar basic idea (a rhythm and a contour) that is
  repeated and developed, not a new shape in every bar. Unity comes from the repetition, interest
  from what changes ([Point Blank](https://www.pointblankmusicschool.com/blog/creating-melodic-hooks-that-stick/),
  [Mogul](https://www.usemogul.com/post/how-to-compose-a-melody-a-step-by-step-guide)).
- **Use a classical sentence for an 8-bar theme**: the basic idea (2 bars), the idea again, varied or
  moved to the next chord (2 bars), then a continuation that fragments it into shorter units and
  speeds up toward a cadence (4 bars)
  ([Fundamentals, Function, and Form](https://milnepublishing.geneseo.edu/fundamentals-function-form/chapter/35-sentences-and-periods/),
  [Analysing Classical Sonatas](https://www.piano-composer-teacher-london.co.uk/post/analysing-classical-sonatas-the-sentence)).
  The rebel hook is one.
- **Rhythm first.** The rhythm is what people remember: fix one rhythmic figure and keep it; let
  syncopation (notes ahead of the beat) give the edge
  ([Futureproof](https://www.futureproofmusicschool.com/blog/how-to-write-a-hook-that-sticks)).
- **Repeat, changing one thing at a time**: the pitch level, the ending note or the rhythm of the
  last beat. Too much repetition bores, too much change confuses
  ([Wisseloord](https://wisseloord.org/?p=54948)).
- **One high point.** The highest note appears once, about two thirds to three quarters of the way
  through, and the line falls away from it (counterpoint's classic shape).
- **Mostly steps, few leaps.** After a leap, move by step the other way (gap fill). Keep the range
  near an octave and a half so a lead can carry it.
- **Strong beats on chord tones**, tension notes (the flat two, the ninth) on weak beats or held for
  effect; tension then release ([Mogul](https://www.usemogul.com/post/how-to-compose-a-melody-a-step-by-step-guide)).
- **Hum it.** If the line does not work hummed with no production, it will not work on a supersaw
  ([Futureproof](https://www.futureproofmusicschool.com/blog/how-to-write-a-hook-that-sticks)).

## The drums: 90s weight

The owner: "The beats are too soft to be edm..we need really strong percussion like edm from the
90s". Measured before the fix, the drums sat 11 dB under the music; after it, level with it (-20.6
against -21.0 dBFS RMS in the main theme's hook). How 90s dance drums are made, and how it is done
here ([MusicRadar, rave breaks](https://www.musicradar.com/tuition/tech/how-to-build-a-layered-90s-style-rave-breakbeat-636596),
[MusicRadar, 909 processing](https://www.musicradar.com/tuition/tech/how-to-process-roland-tr-909-drum-sounds-632930),
[Attack, 909 layering](https://www.attackmagazine.com/technique/beat-dissected/raw-french-house/)):

- **Layer the kick and the snare**: the soundfont's 909 kit plus an LMMS Kicker kick under every `bd`
  and a Kicker clap under every `cp` (`@layer bd Kicker/KickPower.xpf`).
- **Squash and drive the drum bus**: a compressor with a slow enough attack to keep the hit, then
  saturation (`@fx drums comp+drive`); a gentle glue compressor on the master so the music ducks
  under each kick (`@fx master glue`).
- **Play them hard** (gains 1.1 to 1.2) and keep the reverb on them short (`@space drums 0.08`).
- **Three kits, written once per score** (`BIGBEAT`, `FLOOR`, `RAVE`): a Chemical Brothers style
  big-beat break with ghost snares and open hats (90 to 110 BPM, the title and the getaway), a 909
  four-on-the-floor with off-beat open hats (125 to 145 BPM, the crabs), and a chopped Prodigy style
  rave break (140 to 160 BPM, the fights). Snare rolls lead into drops.

## The cues

| Cue | Score | Where it plays | Tempo, length | Built from |
| --- | --- | --- | --- | --- |
| The motifs | `music/motifs.strudel` | Reference only: every motif in turn | 110 BPM, 45 bars, 1:38 | All |
| Main Theme | `music/main_theme.strudel` | Title screen | 100 BPM, 52 bars, 2:07 | Rain and the call sign; the riff over a kick-and-hat build; the hook on a smoky lead over a big-beat break; a breakdown with a snare roll; the hook on supersaws over the riff and the full break; the riff fading |
| Boarding | `music/boarding.strudel` | The boarding cinematic | 72 BPM, 14 bars, 0:53 | Hatch over a drone with steam and swells, the call sign, the hook beginning |
| Sortie | `music/sortie.strudel` | Combat; loops | 150 BPM, 64 bars, 1:44 | Call Sign, the riff over breakbeats, Scuttle, Claw, Siren, the hook on supersaws |
| Big Boy | `music/big_boy.strudel` | The siege crab set piece (survey G14) | 140 BPM, 48 bars, 1:24 | Scuttle as a march and on acid, Claw, Siren, the rebel's riff and hook |
| Getaway | `music/mission_accomplished.strudel` | After a won mission | 100 BPM, 6 bars, 0:17 | Getaway on a big-beat break, the rain |

There is no music on foot for now: Main Street was deleted, and walking plays the world's sound.

Each score's `.mid` (for any DAW) and `.mmp` (the LMMS project) sit beside it in `music/`; the
game's copies are `assets/audio/music/<cue>.ogg`, brought to -16 LUFS by a plain gain and a peak
limiter, so each keeps its build from quiet to loud. How they play in the game (which cue when,
crossfades, loops) is `openspec/changes/music`, not built yet.

## Rules for new music

- **Build from the motifs.** A new cue quotes at least one motif, in the key and harmony above,
  and copies the `MOTIF LIBRARY` block word for word (`node tools/music/check_motifs.mjs`). A new
  motif goes into `music/motifs.strudel` and this file first, then into cues.
- **Write melodies by the rules above** ("Writing a melody"): one idea, a sentence, one high point.
- **Drums at the front** ("The drums"): layered, compressed, driven, level with the music.
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
