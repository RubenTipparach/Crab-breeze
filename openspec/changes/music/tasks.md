# Tasks: music

- [x] 0.1 Motifs, rules and cue list (`docs/design/music.md`); scores in `music/`; the pipeline (`tools/music`, skill `music-authoring`); seven cues rendered to `assets/audio/music/`
- [x] 0.2 The owner listened: electronic and futuristic, not orchestral (A1); then moody cyberpunk, an edgy rebel, not heroic; Main Street deleted. Motifs rebuilt (the Rebel riff and hook, a spine that never resolves to major), every cue rewritten
- [x] 0.3 The owner approved the music (2026-10-25: "music is all good", "all audio and sfx approved")
- [x] 1.1 `data/audio/music.ron`, loaded and validated; `cb-tools validate` checks each `bpm` against its score (tests: `the_shipped_cue_file_loads`, `the_score_tempo_is_read_from_setcps`, `every_shipped_sound_effect_is_mono_and_every_file_is_there`)
- [x] 1.2 `music_state` in `cb-core` (tests: `calm_in_the_seat_is_silent`, `on_foot_is_silent_even_under_fire`, `combat_holds_thirty_seconds_after_the_last_shot`, `a_set_piece_beats_combat_and_a_win_beats_everything`, `a_change_lands_on_the_next_bar_line`)
- [ ] 2.1 Measure a bar-aligned switch on Bevy audio on the owner's machine (Bevy audio is used; switches land on the frame after the bar line, about 16 ms late); move to bevy_kira_audio only if it is audibly off
- [x] 2.3 Survey A2 decided (owner, 2026-10-26: "Approve all"): the switch and fade rules in `cb-core` (`plan_switch`, `fade_s`, `fade_progress`; tests `every_state_but_silence_has_its_own_cue`, `a_switch_waits_for_the_bar_line_of_what_plays`, `from_silence_a_cue_starts_at_once`, `a_state_that_flips_back_before_the_bar_cancels_its_switch`, `a_waiting_switch_keeps_its_bar_line`, `a_crossfade_takes_one_bar_of_the_new_cue_and_silence_two_of_the_old`); the title cue on the main menu and the loading screen; the set piece is the air fortress; a fortress brought down is a win
- [x] 2.2 The music plugin (`cb-app/src/music.rs`): bar-aligned crossfades, calm fading to silence over 2 bars, stingers that play once, its volume from data (built 2026-10-25; not yet heard in a playtest)
- [ ] 3.1 Playtest with sound in the share copy: title, walk, boarding, a fight, the win
