# Tasks: music

- [x] 0.1 Motifs, rules and cue list (`docs/design/music.md`); scores in `music/`; the pipeline (`tools/music`, skill `music-authoring`); seven cues rendered to `assets/audio/music/`
- [x] 0.2 The owner listened: electronic and futuristic, not orchestral (A1); then moody cyberpunk, an edgy rebel, not heroic; Main Street deleted. Motifs rebuilt (the Rebel riff and hook, a spine that never resolves to major), every cue rewritten
- [ ] 0.3 The owner listens to the electronic round and answers A2 and A3
- [ ] 1.1 `data/audio/music.ron`, loaded and validated; `cb-tools validate` checks each `bpm` against its score (tests: `a_state_without_a_cue_stops_startup`, `music_bpm_matches_the_score`)
- [ ] 1.2 `music_state` in `cb-core` (tests: `combat_wins_over_boarding`, `on_foot_is_silent`, `the_set_piece_wins_over_combat`)
- [ ] 2.1 Measure a bar-aligned switch on Bevy audio; choose Bevy audio or bevy_kira_audio and record it in design section 4
- [ ] 2.2 The music plugin: bar-aligned crossfades, stingers, loops, ducking, its volume setting
- [ ] 3.1 Playtest with sound in the share copy: title, walk, boarding, a fight, the win
