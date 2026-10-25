# Design: audio

## 1. Cues

`data/audio/cues.ron`: each cue has `files`, `volume_db`, `pitch: (lo, hi)`, `space: World | Cockpit`,
`loop` and `max_voices`. A cue picks one of its files at random (presentation randomness, not the
simulation's) and a pitch in its range. A missing file, or a cue the game asks for that the file
lacks, stops startup with its path.

| Cue | Asked by | Space |
| --- | --- | --- |
| `autocannon`, `cannon`, `rocket_launch`, `missile_launch` | a gun firing (`mech::step_mech`) | World (the player's own: Cockpit) |
| `impact_mech`, `impact_building`, `impact_ground`, `impact_water` | `combat::fly_shots` | World |
| `explosion_small`, `explosion_large` | `blast::explode` (under or over 3 MJ) | World |
| `foot_stomp`, `foot_splash` | footfalls | World (own: Cockpit, a muffled thump) |
| `rocket_loop`, `rocket_rumble` | rockets burning, volume and pitch on the lift axis | Cockpit |
| `skate_roll` | skating with the wheels on the ground, pitch on speed (section 2) | Cockpit |
| `lock_tone`, `lock_on` | `targeting` (building, locked) | Cockpit |
| `hatch_*`, `steam` | the boarding stages | World |
| `collapse` | a building storey giving way | World |

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
passed toward 2 kHz at 1 km. Heard from the seat, World sounds are 6 dB down and low passed at
4 kHz (the sealed cockpit). At most 32 voices; the quietest is dropped.

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

## 5. Cost

Bevy audio (rodio) on its own thread; 32 voices of 48 kHz stereo is well under 1 % of a core. Files
are OGG Vorbis, about 20 MB for the first set.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| P4 | Sound licences | CC0 only; CC-BY allowed with a credits screen when a needed sound has no CC0 match | provisional (survey P4) |
| G8 | The cockpit shake | position only, millimetres, with the rockets and near blasts | provisional (survey G8) |
| P2 | Audio in the share copy | yes, now that the game has sound | provisional (survey P2) |
