# Crab Breeze music: the motifs and the rules of the score

Read this before writing, arranging or rendering any music for the game. It holds the musical
world, every motif with its notes and what it means, and the cues built from them so far. The
method (Strudel score, multitrack MIDI, LMMS headless with real instruments) is the
`music-authoring` skill. The canonical text of every motif is the `MOTIF LIBRARY` block of
`music/motifs.strudel`; the notes below are that block written out for reading.

Listen to everything on the Crab Breeze Music page (the link is in `docs/design/README.md`).

## The musical world

- **Setting and sound** (owner, 2026-10-25: "music sucks, electronic, scifi futuristic please!",
  replacing the first orchestral-rock round). Alternate-history 1980s industrial mechs (survey G1)
  defending towns from alien crabs, fought Ace Combat style (survey G4). The score is
  **electronic and futuristic**: supersaw and trance leads, pumping pads, sixteenth-note
  arpeggios, analog and reese basses, an acid bassline for the crabs, synth-stab claws, 808/909
  drums and breakbeats, echoes written as notes and reverb on the leads and pads. The town keeps a
  little warmth (a glassy bell, a DX electric piano: calm synthwave). Nothing orchestral.
- **The voices** are LMMS synths, chosen per role in `music/instruments.json` `voices`: lead
  `gm_lead_2_sawtooth` (SuperSawLead), second lead `gm_lead_5_charang` (AnalogTimes), pad
  `gm_pad_poly` (TINTNpad), dark pad `gm_pad_choir` (CryingPads), strings `gm_synth_strings_1`
  (PowerStrings), stabs `gm_synth_brass_1` (Jupiter), pluck and arpeggio `gm_lead_1_square`
  (PluckArpeggio), bass `gm_synth_bass_1` (FuzzyAnalogBass), reese `gm_synth_bass_2` (DirtyReece),
  acid `gm_lead_8_bass_lead` (LB302 AcidLead), bell `gm_fx_crystal` (AnalogBell), swells
  `gm_fx_sci_fi` (Supernova); drums the soundfont's 808/909 kit (`@drumkit 25`). A score names the
  role by its GM name, so strudel.cc still plays a rough preview.
- **Home key: D minor**, natural minor. **The personality is the lift**: B flat, C, then **D
  major** (bVI, bVII, I with the major third). Every big moment arrives by it, and the F sharp of
  that last chord is the score's signature note: the minor world breaks into major when the Breeze
  wins.
- **The spine** is 16 bars. Pass A ends on A major (the pull home, with the C sharp); pass B
  climbs G minor, B flat, C into the lift:

  ```
  pass A:  Dm | Bb | F | C | Dm | Bb | Gm | A
  pass B:  Dm | Bb | F | C | Gm | Bb | C  | D (major)
  ```

- **The stride is the mech's gait: 3 + 3 + 2 eighths** ("stomp, stomp, step-step"), onsets on
  beat 1, the "and" of 2, and beat 4. The Breeze's melody rides it, the bass and the kick drum
  play it, and a cue that wants the mech in it uses it.
- **The crabs are the opposite**: chromatic (D against E flat), the tritone (D against A flat),
  and a scuttling rhythm of sixteenths grouped 3 + 3 + 3 + 3 + 2 + 2, many legs moving sideways.
- **Tempi**: 128 BPM for the title, 140 to 150 BPM for combat, 92 on foot, 72 for the boarding
  cinematic. One cycle of Strudel is one 4/4 bar (`setcps(BPM/60/4)`).

## The motifs

Notes are scientific pitch (C4 is middle C). Durations are in eighths unless marked: `@3` is a
dotted quarter, `@2` a quarter, `@8` a whole bar.

### 1. The Breeze: the mech, the hero (16 bars)

The main theme. It starts on a rising fifth (D to A) that sags a step and pushes back up, in the
stride rhythm, so it walks like the machine. Pass A climbs to E and pulls home through C sharp;
pass B climbs through the lift and ends singing **F sharp to A** over D major.

```
pass A  Dm  D4@3 A4@3 G4 A4   | Bb  Bb4@3 A4@3 F4 G4  | F   A4@3 C5@3 Bb4 A4 | C  G4@6 F4 E4
        Dm  D4@3 A4@3 G4 A4   | Bb  Bb4@3 D5@3 C5 Bb4 | Gm  D5@3 G5@3 F5 E5  | A  E5@6 C#5 A4
pass B  Dm  D4@3 A4@3 G4 A4   | Bb  Bb4@3 A4@3 F4 G4  | F   A4@3 C5@3 D5 E5  | C  F5@3 E5@3 D5 C5
        Gm  D5@3 Bb4@3 C5 D5  | Bb  F5@3 D5@3 E5 F5   | C   G5@3 E5@3 F5 G5  | D  F#5@3 A5@5
```

Use it: when the mech is the subject. A trance lead states it plainly; supersaws sing it in full; its head
alone (`D A G A`) is enough to say "the Breeze" anywhere. In D major (the fanfare) it is victory.

### 2. Call Sign: the radio (2 bars)

"CB" in Morse code (`-.-.  -...`) as a rhythm of sixteenths: C on A5, B on D5. A dah is three
sixteenths, a dit one, with Morse's own gaps.

```
A5@3 . A5 . A5@3 . A5 . . .  D5@3 . D5 . D5 . D5 . . . . . . . . .
```

Use it: briefings, radio traffic, the mech's systems waking, the end of a cue that hands over to a
mission. An echoing pluck makes it a radio; a glassy bell makes it instruments lighting up. The
lock-on tone in `openspec/changes/audio` can take its rhythm, so the cockpit and the score speak
the same language.

### 3. Main Street: the town and its people (8 bars, F major)

A hymn in the relative major, plain and singable, with none of the stride in it: people, not
machines.

```
F   C5@3 A4 G4@2 F4@2 | Bb  D5@3 C5 Bb4@4      | Dm  A4@3 G4 F4@2 D4@2 | C   E4@2 G4@2 C5@4
F   C5@3 A4 G4@2 F4@2 | Bb  D5@3 F5 E5@2 D5@2  | Gm C  C5@2 Bb4@2 A4@2 G4@2 | F   F4@8
```

Use it: on foot, in town, before the alarm; briefings about a city; the breakdown of the title
theme. Defeat (a city lost) can take it into D minor on a lone bell.

### 4. The crabs: Scuttle, Claw and Siren

- **Scuttle** (2 bars of sixteenths, grouped 3 + 3 + 3 + 3 + 2 + 2, accents on each group):
  ```
  D2 Eb2 D2 | D2 Eb2 D2 | Ab1 A1 Ab1 | D2 Eb2 D2 | F2 E2 | Eb2 D2
  D2 Eb2 D2 | D2 Eb2 D2 | Ab1 A1 Ab1 | D2 Eb2 D2 | Bb1 A1 | Ab1 G1
  ```
  An acid bassline over a reese. The crabs on the move.
- **Claw**: the tritone D4 + A flat 4, stabbed on the scuttle's accents (sixteenths 1, 7, 13, 15).
  Synth stabs.
- **Siren**: A4, A flat 4, G4, F sharp 4, one bar each: the crabs' wail, a slow chromatic fall.
  A dark pad.

Use them: whenever the crabs are near. In the fight the Breeze takes the scuttle's rhythm and
plays it on the spine's chords: the mech beating the crabs at their own game.

### 5. The Hatch: boarding (10 bars)

Stacked fourths, climbing a step for each stage of the chest opening (two bars a stage), then a
quartal chord that resolves into **D major** as the pilot sits:

```
D3 G3 C4 F4 (x2) | E3 A3 D4 G4 (x2) | F3 Bb3 E4 A4 (x2) | G3 C4 F4 Bb4 (x2) | A3 D4 G4 C5 | D3 A3 D4 F#4
```

Use it: the boarding cinematic (`openspec/changes/mech-boarding`), one stage per hatch stage, a
burst of steam on each. Seated, the Breeze's last four bars (the lift) follow.

### 6. Fanfare: mission accomplished (6 bars)

The lift with the Breeze's head turned major:

```
Bb  D5@3 F5@3 D5 F5 | C  E5@3 G5@3 E5 G5 | D  D5@3 A5@3 G5 A5 | D  F#5@8 | G  B5@3 A5@3 F#5 G5 | D  A5@8
```

## The cues

| Cue | Score | Where it plays | Tempo, length | Built from |
| --- | --- | --- | --- | --- |
| The motifs | `music/motifs.strudel` | Reference only: every motif in turn | 120 BPM, 59 bars, 1:58 | All |
| Main Theme | `music/main_theme.strudel` | Title screen | 128 BPM, 48 bars, 1:34 | Call Sign on a pluck, Breeze on a trance lead, a Main Street breakdown and build, the drop on supersaws, the lift |
| Main Street | `music/on_foot.strudel` | On foot in town; loops | 92 BPM, 32 bars, 1:26 | Main Street on a bell and a pad lead, Call Sign on a radio |
| Boarding | `music/boarding.strudel` | The boarding cinematic | 72 BPM, 14 bars, 0:53 | Hatch on an echoing pluck with steam and swells, Call Sign, the Breeze's lift |
| Sortie | `music/sortie.strudel` | Combat; loops (ends on A) | 150 BPM, 64 bars, 1:44 | Call Sign, Breeze on supersaws over breakbeats, Scuttle on acid, Claw, Siren |
| Big Boy | `music/big_boy.strudel` | The siege crab set piece (survey G14) | 140 BPM, 48 bars, 1:24 | Scuttle as a reese march and on acid, Claw, Siren, Breeze |
| Mission Accomplished | `music/mission_accomplished.strudel` | After a won mission | 104 BPM, 8 bars, 0:21 | Fanfare on supersaws |

Each score's `.mid` (for any DAW) and `.mmp` (the LMMS project) sit beside it in `music/`; the
game's copies are `assets/audio/music/<cue>.ogg`, loudness-normalised to -16 LUFS. How they play in
the game (which cue when, crossfades, loops, layers) is `openspec/changes/music`, not built yet.

## Rules for new music

- **Build from the motifs.** A new cue quotes at least one motif, in the key and harmony above,
  and copies the `MOTIF LIBRARY` block word for word (`node tools/music/check_motifs.mjs`). A new
  motif goes into `music/motifs.strudel` and this file first, then into cues.
- **The Breeze is never the crabs' and the crabs are never the Breeze's**: no chromatic grind in
  the hero theme, no lift in a crab cue unless the Breeze is winning.
- **First person holds** (CLAUDE.md, the law): music never cuts the view or the input. A stinger
  plays over the game; it does not stop it.
- **Mix for the cockpit.** Leads in the middle (the supersaw, the trance lead), no sub-bass mud under the rocket
  rumble and footfalls (`openspec/changes/audio`): keep the lowest parts on the bass and the kick, and check a cue under the game's sound once both exist.
- **Know what is proven.** A render is checked by numbers (no clipping, the loudness profile has
  the cue's shape) and by a spectrogram that shows the lead line; neither replaces the owner's
  ears. Say so when handing music over.
