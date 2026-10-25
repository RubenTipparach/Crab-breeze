# Tasks: audio

- [x] 1.1 A first sound set: 36 cues synthesised by `tools/audio/gen_sfx.py` into `assets/audio/sfx/` (listed in its `SOURCES.md`), on the Crab Breeze Sounds page
- [x] 1.1b Replace placeholders with recorded CC0 where one beats them: 30 of 36 cut from CC0 recordings by `tools/audio/cut_cc0.py` (spec `tools/audio/cc0_cues.json`), the six cockpit tones stay synthesised; approved by the owner (2026-10-25: "all audio and sfx approved")
- [x] 1.1c Punch sounds (`punch_swing`, `punch_hit_mech`, `punch_hit_building`), cut from CC0, ready for when melee is designed
- [ ] 1.2 `data/audio/cues.ron`, loaded and validated (tests: `every_cue_the_game_asks_for_is_in_the_file`, `a_cue_without_its_file_stops_startup`, `every_sound_effect_is_placed_in_3d`, `a_placed_cue_with_a_stereo_file_is_refused`)
- [ ] 1.3 Speaker and reactor empties in the cockpit generator: `SND_warning` and `SND_radio` on `CONSOLE_front`, `SND_reactor` behind the seat; `cb-tools validate` checks every `Emitter` node exists in the glb
- [ ] 2.1 The audio plugin: cue messages, 3D sound, distance delay and muffling, voice cap
- [ ] 2.2 Rocket loop, sub rumble and the position-only cockpit shake
- [ ] 2.3 Guns, impacts, explosions, footfalls, skates, lock tones, the hatch
- [ ] 3.1 Playtest with sound in the share copy (P2)
