# Design: cockpit-alarms

## Where it lives

- `cb-core::alarms` (new, no engine): `Alarms::step(&AlarmInputs, dt_s) -> AlarmState`, a pure function of the
  player's mech this fixed step. Inputs: each part's armour share and `lost` flag, the boost and skate `heat`
  (0 to 1) and `locked`, every live missile whose `locked_target` is the player (range m, closing speed m/s),
  and this step's hits (part, side, damage). Output per lamp role (`overhead`, `rear`, `footwell`, `sill_l`,
  `sill_r`, `screen`): colour (linear RGB), intensity (lumens) and a spark request list (side, count). Golden-step
  tests read as sentences ("a lost left arm strobes the left sill red for three seconds").
- `cb-app::cockpit_alarms` (new plugin, `Update`, after `PresentSet::Camera`): at `bind_interior` it records the
  interior's `LIGHT_*` node entities by role instead of despawning their lights; it spawns one `PointLight`
  (shadows off, range 2.5 m) on `COCKPIT_LAYER` at each, intensity 0; each frame it writes the `AlarmState` into
  them, and hands spark requests to the particle system (`particles`, the cockpit layer) at the console point on
  the hit's side (`CONSOLE_front`, offset 0.5 m to the side).
- One implementation: the strobe, pulse and beacon are one `wave(kind, t)` function in the core, used by every lamp.

## Numbers (tuning in `data/cockpit_alarms.ron`)

| Alarm | Lamp | Colour | Peak | Rhythm |
| --- | --- | --- | --- | --- |
| Part lost | sill on that side (both for a leg) | red (1.0, 0.08, 0.05) | 120 lm | 4 Hz strobe for 3 s, then 15 lm steady |
| Heat | both sills | orange (1.0, 0.45, 0.10) | 0 lm at heat 0.6, 80 lm at 1.0 | steady ramp; 1.5 Hz pulse while locked |
| Missile lock | overhead | amber (1.0, 0.70, 0.10) | 150 lm | beacon at 1 Hz at 3 s to impact or more, rising to 6 Hz at 0.5 s |
| Hit | all, plus sparks | white-orange (1.0, 0.8, 0.5) | +60 lm flicker for 0.2 s | 20 to 60 sparks, scaled by damage over 10% of the part |

The cabin ambient (`cabin_ambient_cd_m2`) stays as it is; the alarms add to it. Time to impact = range / max(closing
speed, 1 m/s).

## Cost

- Up to 6 point lights on the cockpit pass with shadows off: clustered forward shading on an RTX 3070, a few tenths
  of a millisecond at most; to be measured on the owner's machine (cloud sessions measure nothing, CLAUDE.md
  section 7).
- Sparks: at most 60 a burst, 2 bursts at once, on the existing particle pipeline: no new pipeline, so no new shader
  to prewarm at the loading screen.
- CPU: one pass over the live missiles per fixed step (tens), negligible.

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| A1 | Colour code: red for parts, orange for heat, amber for missiles? | Yes, as in the table |
| A2 | Should the missile lamp also show which side it comes from (left or right sill)? | Yes: the sill on the missile's side flashes with the overhead |
| A3 | An alarm-mute switch on the panel (clickable, bindable, survey C2)? | Yes, it silences the strobe for 10 s and leaves the steady colours |
| A4 | Sparks only from hits on the torso and head, or from any hit? | Torso and head: the cabin is in the chest |
