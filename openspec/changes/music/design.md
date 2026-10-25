# Design: music

## 1. Data

`data/audio/music.ron` maps each game state to a cue:

```ron
(
    cues: {
        "title":   (file: "audio/music/main_theme.ogg", bpm: 132, beats_per_bar: 4, looping: true),
        "on_foot": (file: "audio/music/on_foot.ogg",    bpm: 92,  beats_per_bar: 4, looping: true),
        "boarding":(file: "audio/music/boarding.ogg",   bpm: 72,  beats_per_bar: 4, looping: false),
        "combat":  (file: "audio/music/sortie.ogg",     bpm: 150, beats_per_bar: 4, looping: true),
        "set_piece":(file: "audio/music/big_boy.ogg",   bpm: 140, beats_per_bar: 4, looping: true),
        "won":     (file: "audio/music/mission_accomplished.ogg", bpm: 104, beats_per_bar: 4, looping: false),
    },
    crossfade_bars: 1.0,
    duck_db: -6.0,
    volume_db: -4.0,
)
```

A state the game enters that the file lacks, a missing file or a non-finite number stops startup
with its path (CLAUDE.md 6.4). `cb-tools validate` checks that every `bpm` matches the `setcps` of its
score, `music/<cue>.strudel`, so the data and the score cannot disagree.

## 2. Choosing the cue

The state is derived, not set by hand: the title screen; on foot (the pilot is not seated);
boarding (the boarding sequence is running); combat (any enemy within 2 km has the mech as a
target, or a shot landed within 30 s); set piece (Big Boy is alive in the wave); won (the mission
result is a win). Higher entries in this list lose to lower ones. The rule is a pure function in
`cb-core` (`music_state(&World) -> MusicState`) with tests; `cb-app` plays what it returns.

## 3. Bar-aligned crossfades

On a state change at time `t` into a cue playing since `t0` at `bpm`, the switch happens at the
next bar line `t0 + ceil((t - t0) / bar) * bar`, `bar = 60 / bpm * beats_per_bar` s, and the old
cue fades out while the new one fades in over `crossfade_bars` bars of the new cue. A state that
flips back before the switch cancels it. Stingers start on the next bar and return to the derived
state when they end.

## 4. Engine

Bevy 0.20's audio (rodio) can loop and set volume but has no sample-accurate scheduling or
tweens. Measure first: if a bar-aligned switch with Bevy's `AudioSink` lands within 20 ms of the
bar, use it; otherwise use `bevy_kira_audio` 0.27 (supports Bevy 0.20, kira clocks and tweens;
`docs/analysis/audio-sources.md`). The choice is recorded here with the measurement before code.

## 5. Cost

Two OGG streams decoding at once during a crossfade: well under 1 % of a core. Memory: streamed,
not loaded whole; the seven cues are 11 MB on disk.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| A1 | The score's direction | Electronic and futuristic | decided (owner, 2026-10-25: "electronic, scifi futuristic please!") |
| A2 | Adaptive music | One cue per state with bar-aligned crossfades first; stems later if wanted | provisional (survey A2) |
| A3 | Music in the cockpit | The score plays over everything (not through the radio) | provisional (survey A3) |
