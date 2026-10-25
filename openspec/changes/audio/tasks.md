# Tasks: audio

- [x] 1.1 A first sound set: 36 cues synthesised by `tools/audio/gen_sfx.py` into `assets/audio/sfx/` (listed in its `SOURCES.md`), on the Crab Breeze Sounds page
- [x] 1.1b Replace placeholders with recorded CC0 where one beats them: 30 of 36 cut from CC0 recordings by `tools/audio/cut_cc0.py` (spec `tools/audio/cc0_cues.json`), the six cockpit tones stay synthesised; approved by the owner (2026-10-25: "all audio and sfx approved")
- [x] 1.1c Punch sounds (`punch_swing`, `punch_hit_mech`, `punch_hit_building`), cut from CC0, ready for when melee is designed
- [x] 1.2 `data/audio/cues.ron`, loaded and validated (tests: `every_cue_the_game_asks_for_is_in_the_file`, `a_cue_without_its_file_stops_startup`, `every_sound_effect_is_placed_in_3d`, `a_placed_cue_with_a_stereo_file_is_refused`)
- [x] 1.3 Cockpit speakers as offsets from `EYE_pilot` in `cues.ron` (`warning`, `radio`, `console`, `reactor`); named empties in the cockpit models can replace them later
- [x] 1.4 Muffled and far copies of every placed cue, rendered by `tools/audio/render_sfx.sh`; `cb-tools validate` checks every cue file exists, every placed one is mono and its copies are there (test `every_shipped_sound_effect_is_mono_and_every_file_is_there`)
- [x] 2.1 The audio plugin (`cb-app/src/audio.rs`): `Sound` and `Loop` messages, 3D sound on the pilot's head, distance delay, the seal, the voice cap (built 2026-10-25; not yet heard in a playtest)
- [x] 2.2 Rocket loop and sub rumble at the jets, air skates as a low rumble (built 2026-10-25; not yet heard in a playtest); the cockpit shake from `RocketSound::shake_m` is not wired yet
- [x] 2.3 Guns, impacts, explosions, footfalls, splashes, skates, the torso servo, lock tones, heat and damage alarms, the hatch stages, systems boot, control clicks, the reactor, the radio on a defence wave (built 2026-10-25; not yet heard in a playtest); shotgun, railgun, laser, plasma, punches and radar have no weapon or system yet, so their cues wait
- [ ] 3.1 Playtest with sound in the share copy (P2)
