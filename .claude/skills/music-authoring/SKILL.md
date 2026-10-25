---
name: music-authoring
description: Write, arrange, render and mix Crab Breeze's music the repository's way, after Tom Lander's pipeline. Motifs and themes are written down first as notes (docs/design/music.md, the MOTIF LIBRARY block of music/motifs.strudel); a cue is a Strudel score (plays at strudel.cc) whose layers name General MIDI instruments or synths; tools/music exports it to a multitrack MIDI file and an LMMS project, LMMS renders it headless with real instruments (the GeneralUser GS soundfont, LMMS's own synths) into the final comp, and the result is loudness-normalised to OGG for the game and MP3 to share. Use whenever making or changing music, a motif, a theme, a leitmotif, a cue, a stinger, a soundtrack, a MIDI file, an LMMS project or a soundfont, mixing or levelling instruments, or asking how the score works ("make music", "write a motif", "new theme", "combat music", "render the soundtrack", "export MIDI", "LMMS", "soundfont", "the mix is muddy", "make the trumpet louder").
metadata:
  author: Crab Breeze (Claude Code), after Tom Lander's audio.md and tools/strudel
  version: "1.0"
---

# Music authoring

From Tom Lander (`audio.md` and `tools/strudel/` at `dea3d50`, 2026-10-25), rebuilt for Crab
Breeze; provenance in `.claude/skills/PROVENANCE.md`. Tom Lander's flow was: motifs transcribed
into a reference file, arranged in Strudel, rendered by a small Node synth, and for orchestral
work each score copied its patterns into its own MIDI exporter. Here the instrument rides on the
note (`s("gm_trumpet")`), so **one exporter serves every score**, and LMMS renders the final comp.

## The pipeline

```
notes on paper           docs/design/music.md + the MOTIF LIBRARY block of music/motifs.strudel
   |
Strudel score            music/<cue>.strudel   (plays as written at https://strudel.cc)
   |  tools/music/strudel_to_midi.mjs        tools/music/strudel_to_lmms.mjs
   v                                          v
multitrack MIDI          music/<cue>.mid      LMMS project  music/<cue>.mmp
(any DAW)                                     |  lmms render (headless, xvfb)
                                              v
                                   final comp -> loudnorm -16 LUFS
                                              |-> assets/audio/music/<cue>.ogg  (the game's copy)
                                              '-> output/music/<cue>.mp3        (share copy)
```

`tools/music/render.sh music/<cue>.strudel` runs the whole chain. `--preview` renders the MIDI
through fluidsynth instead (seconds, rougher, for comparing arrangements).

## 1. Motifs first, in notes

Before any score, write the motif down: its notes and rhythm, the chords under it, what it means
and where it plays. That goes into `docs/design/music.md` and, as Strudel, into the `MOTIF
LIBRARY` block of `music/motifs.strudel` (the motif sheet, which plays every motif in turn). Read
`docs/design/music.md` before writing anything: it holds the key (D minor, the lift into D major),
the spine, the stride rhythm, the crab material and the rules for new music.

Ask the owner about direction with the survey, never in chat (CLAUDE.md section 8). Build on the
recommendation while a question is open.

## 2. A cue is a Strudel score

A score is one self-contained `.strudel` file, so it can be pasted into strudel.cc and heard. Its
header:

```js
// @title Crab Breeze: Sortie          the title in the file's tags
// @cycles 64                          bars to render = the sum of the bars in arrange()
// @reverb 0.35                        the soundfont's reverb for this cue (0..1)
// @mix gm_trumpet vol=190             this cue's comp: LMMS track volume (%) and pan
// @mix supersaw vol=30 pan=-20
```

then `setcps(BPM/60/4)` (one cycle is one 4/4 bar), the `MOTIF LIBRARY` block copied word for
word, the sections as `stack(...)`, and the form as `arrange([bars, section], ...)`.

**Every layer names its instrument with `s(...)`**, which is how the exporters split the score
into tracks:

| `s(...)` | Plays as | In strudel.cc |
| --- | --- | --- |
| `gm_<name>` (Strudel's General MIDI names, e.g. `gm_french_horn`, `gm_string_ensemble_1`, `gm_distortion_guitar`, `gm_choir_aahs`, `gm_timpani`; the table is `GM_NAMES` in `tools/music/score_lib.mjs`) | That GM program, through the soundfont, on its own MIDI channel and LMMS track | The same GM sound |
| `bd sd hh oh cr rd rim ht mt lt ft tb sh ...` (`DRUM_MAP`) | The GM drum kit, channel 10 | Strudel's drum samples |
| `sawtooth square triangle sine supersaw` | The LMMS preset `music/instruments.json` gives it (TripleOscillator), with a GM stand-in in the MIDI file | Strudel's synths |

Anything else stops the export with its name: a dropped part would ship a cue missing a voice.
Loudness of a note is `gain` (and `velocity`): MIDI velocity = 100 x velocity x gain, so keep
leads near 1.0 to 1.1 and accompaniment at 0.4 to 0.8. The same instrument in two layers is one
track (strings low and high merge); give a layer its own instrument to give it its own fader.

Strudel details that matter here: `@3` weights a step (a dotted quarter in an eighth grid), `~` is
a rest and takes a weight too (`~@3`), `<a b c>` is one per bar, `!2` repeats, `/2` stretches a
pattern over two bars, `.early(12)` starts a 16-bar motif at its bar 13, `.add(note(12))` and
`.sub(note(12))` move it by octaves, `.struct(STRIDE)` re-rhythms chords, and a mini-notation
string in `gain(...)` gives per-step accents. Check new syntax on a two-bar test before a long
score: `node tools/music/strudel_to_midi.mjs test.strudel /tmp/t.mid` prints every part and its
note count.

## 3. Render and look

```sh
tools/music/render.sh music/<cue>.strudel
node tools/music/check_motifs.mjs          # every score carries the motif library unchanged
```

`render.sh` writes the `.mid` and `.mmp` beside the score, renders with LMMS, refuses a raw mix
that clips (`check_mix.py`), normalises to -16 LUFS integrated, -1.5 dBTP, and prints the loudness
per 5 s. Then look before handing it over:

- **The profile** has the cue's shape (a quiet intro, loud peaks, a falling ending). A flat or
  empty one means a wrong `@cycles`, a section that never sounds, or an unknown instrument.
- **A spectrogram** shows the lead line above the accompaniment:
  `ffmpeg -i output/music/<cue>.mp3 -lavfi "showspectrumpic=s=1600x400:fscale=log:stop=5000" spec.png`.
  If the melody's contour is not visible, the comp buries it (step 4).
- To hear one instrument alone, set `muted="1"` on the other `<track>`s of a copy of the `.mmp`
  and render it; `astats` gives its level.

A render that passes these is checked, not approved: only the owner's ears approve music. Say so.

## 4. The final comp in LMMS

LMMS is where the instruments and the mix are chosen. The project `strudel_to_lmms.mjs` writes has
one instrument track per part:

- **GM parts and drums** play through LMMS's Sf2Player on the soundfont in
  `music/instruments.json` (GeneralUser GS 2.0.3, fetched by `tools/music/fetch_soundfonts.sh` to
  `/opt/soundfonts`, pinned by commit and SHA-256), with its reverb set by `@reverb`.
- **Synth parts** play an LMMS preset (`synths.<name>.lmms_preset`, a path under
  `/usr/share/lmms/presets`: TripleOscillator, LB302, Monstro, Kicker ...). To give the score a
  new synth voice, add a Strudel synth name to `synths` with its preset and GM stand-in.
- **Levels**: `music/instruments.json` `mix.<instrument>.vol` is **calibrated**, not tuned:
  `tools/music/calibrate.sh --write` plays one phrase on every GM instrument the scores use at
  volume 100, measures it and sets each volume so all sit at -30 dBFS. Rerun it after changing the
  soundfont. A cue's balance is then its own `@mix` lines (lift the lead, trim the bass) and its
  gains. Never hand-edit the `.mmp` and keep it: it is regenerated from the score.
- To mix by hand in the LMMS GUI (on a desktop), open the `.mmp`, then carry the volumes you chose
  back into the score's `@mix` lines.

Headless facts (from Tom Lander): LMMS refuses root without `--allowroot`, always wants a display
(`xvfb-run -a`), and cannot import MIDI headless (so the project is written directly). It renders
to the last note: give a cue a held final chord rather than trailing silence.

## 5. Where things go and what to commit

| Path | What | Commit |
| --- | --- | --- |
| `music/<cue>.strudel` | The score, the source of truth | yes |
| `music/<cue>.mid`, `music/<cue>.mmp` | Exports, regenerated by `render.sh` | yes (for a DAW on another machine) |
| `music/instruments.json` | Soundfont, synth presets, calibrated levels | yes |
| `assets/audio/music/<cue>.ogg` | The game's copy, -16 LUFS | yes |
| `output/music/<cue>.mp3` | Share copy | no (`/output` is ignored); send it or publish it on the music page |

After a render worth hearing, republish the Crab Breeze Music page (link in
`docs/design/README.md`) with the new MP3s and end the reply with its link (CLAUDE.md section 8).
Music is not wired into the game until `openspec/changes/music` is applied.

## Setting up a session

```sh
apt-get install -y --no-install-recommends lmms fluidsynth fluid-soundfont-gm ffmpeg xvfb
(cd tools/music && npm install)       # @strudel/core, mini, transpiler; postinstall patches kabelsalat
tools/music/fetch_soundfonts.sh        # GeneralUser GS into /opt/soundfonts
```

About two minutes in a cloud session. Rendering needs no GPU, so a cloud session can make and
check music; only listening is the owner's.

## What is not here yet, and where to get more

- **Sample-based orchestra (SFZ)**: VSCO 2 Community Edition (CC0) and others are SFZ, which LMMS
  1.2 cannot load. Either convert to sf2 with Polyphone or render those stems with sfizz; neither
  is set up. `docs/analysis/audio-sources.md` lists the candidates and their licences.
- **ZynAddSubFX** ships inside LMMS and makes far richer pads and leads than TripleOscillator; its
  presets would need their XML embedded in the project. A good next step for the 80s colours.
- **Tempo changes inside a cue**: one tempo per score. Split a cue whose tempo changes.
