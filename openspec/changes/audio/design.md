# Design: audio

## 1. Cues

`data/audio/cues.ron`: each cue has `files`, `volume_db`, `pitch: (lo, hi)`, `space`, `loop` and
`max_voices`. A cue picks one of its files at random (presentation randomness, not the
simulation's) and a pitch in its range. A missing file, or a cue the game asks for that the file
lacks, stops startup with its path.

**Every sound is 3D except ambience, music and UI** (owner, 2026-10-25: "verify all sounds are 3d
based (except ambience and music and ui)"). `space` is one of:

- `Emitter(node)`: placed in 3D at a named node and moving with it, heard from the listener at
  `EYE_pilot` (the pilot's head, on foot or in the seat) with distance, direction and the head's
  turn. The player's own mech is no exception: its guns fire from `MUZZLE_*` beside and below the
  cockpit, its jets roar from behind, and a cockpit tone comes from its speaker, so turning the
  head moves every one of them.
- `At(point)`: the same, at a world point that does not move (an impact, a blast, a collapse).
- `Ambience`, `Music`, `Ui`: not placed; stereo, played as they are (`ambience`, `music`, menus).

| Cue | Asked by | Space |
| --- | --- | --- |
| `autocannon`, `cannon`, `shotgun`, `rail_charge`, `rail_fire`, `laser_loop`, `laser_stop`, `plasma_fire` | a gun firing (`mech::step_mech`, `weapon-kinds`) | `Emitter(MUZZLE_L / MUZZLE_R)` of the gun's mount |
| `rocket_launch`, `missile_launch` | a launcher firing | `Emitter` at the launcher's muzzle |
| `impact_mech`, `impact_building`, `impact_ground`, `impact_water`, `plasma_burst` | `combat::fly_shots` | `At` the hit |
| `explosion_small`, `explosion_large` | `blast::explode` (under or over 3 MJ) | `At` the blast |
| `foot_stomp`, `foot_splash` | footfalls | `Emitter(foot_L / foot_R)`, the foot that landed |
| `skate_roll` | skating with the wheels on the ground, pitch on speed (section 2b) | `Emitter(WHEEL_L)` and `Emitter(WHEEL_R)`, one voice each |
| `rocket_loop`, `rocket_rumble`, `jet_ignite` | rockets burning, volume and pitch on the lift axis | `Emitter(JET_L)` and `Emitter(JET_R)` |
| `servo_torso`, `hydraulic` | the torso turning, a leg's ram | `Emitter` at the torso ring, at the leg's knee |
| `punch_swing`, `punch_hit_mech`, `punch_hit_building` | a punch (melee is not designed yet: `weapon-kinds` leaves it out; the cue names are reserved) | `Emitter(hand_L / hand_R)` for the swing, `At` the contact for a hit |
| `lock_tone`, `lock_on`, `heat_lock`, `alarm_warning`, `radar_ping`, `systems_boot`, `radio_squelch` | `targeting`, `cockpit-alarms`, the radio, the boot | `Emitter(SND_warning)` or `Emitter(SND_radio)`: new speaker empties on `CONSOLE_front` (a task for the cockpit generator) |
| `mfd_click` | a cockpit control pressed | `Emitter` at the pressed control's own node (every control is a named node, CLAUDE.md 10) |
| `reactor_hum` | the reactor running | `Emitter(SND_reactor)`: a new empty behind the seat |
| `hatch_*` | the boarding stages | `Emitter(HATCH_entry)` and the hatch parts that move |
| `collapse` | a building storey giving way | `At` the storey |

**The check.** `cb-tools validate` loads every cue's files and refuses an `Emitter` or `At` cue
whose file is not mono (a stereo file cannot be placed; every file in `assets/audio/sfx` is mono
today, 39 of 39) and a cue whose `space` names a node the mech's or cockpit's glb lacks. Only
`Ambience`, `Music` and `Ui` may be stereo. A test reads `cues.ron` and fails on any cue that is
neither placed nor one of those three, so a new cue cannot slip in flat.

## 2. Rocket rumble

While `MechState::boosting`: `rocket_loop` at `-12 + 10 * lift` dB and `0.9 + 0.2 * lift` pitch,
`rocket_rumble` (a 30-60 Hz loop) at `-8 + 6 * lift` dB, and the cockpit set's transform offset by
a shake of `0.004 * lift` m at 18-25 Hz (smoothed noise). The camera rides the cockpit, so the view
shakes in position only; its direction is never touched.

## 2b. Skates: wheels on the ground, a low rumble in the air

(owner, 2026-10-25: "skate roll is only on the ground right? other wise its a low rocket rumble?")

- **On the ground, skates on:** `skate_roll` at `-14 + 10 * s` dB and `0.8 + 0.4 * s` pitch, where
  `s = speed / 120 m/s` (the skates' top speed, `boost-and-skate`). Silent at a standstill.
- **Feet leave the ground:** `skate_roll` fades out over 0.2 s; it never plays in the air. It fades
  back in over 0.1 s on landing, after the landing's own thump.
- **In the air, skates on, a direction held** (the skates thrusting along the view,
  `boost-and-skate`): the same `rocket_rumble` and `rocket_loop` the lift rockets use, lower, as
  small thrusters: `rocket_rumble` at `-14 + 6 * k` dB and `rocket_loop` at `-22 + 8 * k` dB and
  `0.8 + 0.1 * k` pitch, where `k` is the skate thrust as a share of its full value. No shake.
- **Rockets and air skates together:** one voice per cue at the louder of the two levels, never two
  copies of the same loop.

The thrust shares (`lift`, `k`) and ground contact are read from the simulation's state, the numbers
the cockpit's own gauges show (CLAUDE.md 6.1), not guessed from the sound side.

## 3. Distance

A World sound starts after `distance / 343` s, falls off as `1 / distance` beyond 5 m, and is low
passed toward 2 kHz at 1 km. Heard from the seat, a sound from outside the cockpit (anything
but the speaker, reactor and control emitters inside the sealed cabin, and the ambience beds the
same way) is 6 dB down and low passed at 4 kHz (the sealed cockpit). At most 32 voices; the quietest is dropped.

## 4. Sources

`assets/audio/SOURCES.md` lists every file: the cue, the source URL, the author, the licence and the
edits (trim, loop point, normalised to -1 dBFS peak). Candidates found on OpenGameArt (CC0):
[Mech Stomp / Step Sound](https://opengameart.org/content/mech-stomp-step-sound),
[Metal footsteps on concrete](https://opengameart.org/content/metal-footsteps-on-concrete),
[25 CC0 bang / firework SFX](https://opengameart.org/content/25-cc0-bang-firework-sfx),
[Explosions](https://opengameart.org/content/explosions-4),
[Boost or Launch or Thruster Sound Effect](https://opengameart.org/content/boost-or-launch-or-thruster-sound-effect),
[Mechanical Sounds](https://opengameart.org/content/mechanical-sounds),
[100 CC0 SFX #2](https://opengameart.org/content/100-cc0-sfx-2). The gaps (a rocket engine loop, a
sub rumble, a heavy autocannon, a lock tone) come from Freesound's CC0 filter.

## 4b. Engine for 3D (decided, built 2026-10-25)

Bevy 0.20's own audio (rodio), read from the pinned source: a spatial voice is panned between two
ears with rodio's falloff of `min(1, 1 / d^2)` on amplitude, -40 dB at only 10 m. So every voice
carries a spatial scale of `1e-4`: 4 km becomes 0.4, inside rodio's unit radius where its falloff is
1, while its panning (which depends only on the difference of the two ears' distances) still
works. The level is then this design's law (`cb_core::audio::distance_gain`), set as the voice's
volume every frame. rodio has no per-voice filter, so the low passes are files: every placed cue
has a muffled copy (4 kHz, heard through the seal) and a far copy (2 kHz, past 600 m) rendered by
`tools/audio/render_sfx.sh` into `audio/sfx/muffled/` and `audio/sfx/far/`, chosen when the sound
starts. If panning alone does not put a sound behind the pilot convincingly in a playtest, the
candidates are `oddio` and Steam Audio (HRTF).

**Most sounds ride messages the game already sends.** `fx_sounds` in `cues.ron` maps an effect
recipe to its sound (a muzzle flash to its gun, an impact to its surface's hit, a foot strike or a
landing to a stomp, foot spray to a splash, a wall smash, a failing member, collapse dust, a lost
part), so an effect and its sound are asked for by one message. Explosions sound from the
`Explosion` message, large at 3 MJ and over. A sound that starts within 15 m of a mech rides it, so
a muzzle blast or a footstep is not left behind a mech skating at 100 m/s.

**Cockpit speakers are data**, offsets from the seat's `EYE_pilot` socket in `cues.ron`
(`speakers`): a speaker voice rides that socket, so it stays put while the head turns. Named
speaker empties in the cockpit models can replace the offsets later without changing a cue.

## 5. Cost

Bevy audio (rodio) on its own thread; 32 voices of 48 kHz stereo is well under 1 % of a core. Files
are OGG Vorbis, about 20 MB for the first set.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| P4 | Sound licences | CC0 only; CC-BY allowed with a credits screen when a needed sound has no CC0 match | provisional (survey P4) |
| G8 | The cockpit shake | position only, millimetres, with the rockets and near blasts | provisional (survey G8) |
| P2 | Audio in the share copy | yes, now that the game has sound | provisional (survey P2) |
