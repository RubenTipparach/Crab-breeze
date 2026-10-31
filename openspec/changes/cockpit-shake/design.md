# Design: cockpit-shake

## 1. The model

One `Shake` (today trauma and a clock) gains a rumble and a jolt:

- **Trauma** `T` in 0..1, kicked by events, falling by `1 / decay_s` a second (as now). Felt as `T^2`.
- **Rumble** `R` in 0..1, set each frame from what the mech is doing (the larger of the jets' and the skates'), eased
  toward its target over `rumble_ease_s` so it swells and fades, never steps.
- **Jolt** `y` (metres, the eye's dip) and its speed, a damped spring: a kick sets the speed down by `v`, the spring
  (`jolt_hz`, `jolt_damping`) brings it back. Clamped to `jolt_max_m`.

Drawn (in `place_camera`, after the view is decided and never written into the aim):

- rotation: the three sines of today at amplitude `max_deg * T^2`, plus a faster, finer buzz (`rumble_hz`) at
  `rumble_deg * R`;
- position: the eye lowered by `y` (seated only; the cockpit camera, in the mech's frame, takes the same dip, so the
  cabin and the world move together, as they would through the mech's own cameras).

Everything multiplied by the player's `camera_shake` setting (0 to 1, default 1).

## 2. What kicks it (seated, the pilot's own mech)

| Event | Kick to trauma | Jolt | From |
| --- | --- | --- | --- |
| Landing | `land_per_m_s * v_down` above `land_from_m_s` | `v_down * jolt_per_m_s` | the mech's step: airborne, then grounded, with its vertical speed |
| Ramming | `ram_per_m_s * dv` above `ram_from_m_s` | `dv * jolt_per_m_s` | the speed lost in one step beyond what the brakes take (a wall, a building, a car) |
| Hit | `hit_per_mm * penetration + hit_per_mj * blast` | none | combat's hit on the piloted mech |
| Own gun or missile | the firing rumble held `fire_hold_s` at `fire_rumble`; a heavy round `heavy_kick`, missile `missile_kick` | heavy `heavy_jolt_m_s` | each round or missile fired (owner: "firing guns should also cause some vibrations", "and missiles") |
| Melee | `melee_kick` | `melee_jolt_m_s` | a strike landing |
| Jets lighting | `ignite_kick` | none | the jets turning on |
| Blast nearby | as now | none | `blast::Explosion` |

Rumble targets: jets `jets_rumble * thrust share`; skates `skate_rumble * min(1, speed / skate_full_m_s) * surface`
(`surface`: 1 on dirt and grass, `concrete_share` on paved ground).

The rules (kick from an event, rumble from a state, the spring) are pure functions in `cb_core` with their tests; the
app reads the mech's state and messages and draws.

## 3. Numbers to start (data `fx.shake`, provisional; tuned by the owner in play)

`max_deg` 1.2 (0.6 since 2026-10-30: every amplitude halved, owner: "thats a bit much rumbling, change it to 50% of that overall as the new base"; `rumble_deg` 0.125, `jolt_per_m_s` 0.06, `jolt_max_m` 0.125), `decay_s` 0.7 (as now); `land_from_m_s` 3, `land_per_m_s` 0.06; `ram_from_m_s` 2, `ram_per_m_s` 0.08;
`hit_per_mj` 0.08, `hit_per_mm` 0.0015; `fire_rumble` 0.6, `fire_hold_s` 0.15, `heavy_kick` 0.4,
`heavy_jolt_m_s` 0.2, `missile_kick` 0.3 (a recoil kick by penetration, felt as the square of the trauma, was too
small to feel: an autocannon round 0.04, a few hundredths of a degree); `melee_kick` 0.45, `melee_jolt_m_s` 0.6;
`ignite_kick` 0.15; `rumble_deg` 0.25, `rumble_hz` 23, `rumble_ease_s` 0.3, `jets_rumble` 0.7, `skate_rumble` 0.5,
`skate_full_m_s` 30, `paved_share` 0.5; `jolt_per_m_s` 0.12, `jolt_max_m` 0.25, `jolt_hz` 1.6, `jolt_damping` 0.45
(0.04 and 2.2 Hz dipped a 12 m/s landing 2 cm, too little to feel; this is about 9 cm).

Every number is in `data/fx/fx.ron` under `shake`, and any of them may be 0, which turns its part off (owner,
2026-10-28: "make sure theyre all numbers we can tweak later", "and that 0 works peroperly"): a 0 decay or spring
rate means no trauma or no dip rather than a division by zero, a 0 ease means at once, a 0 damping rings. A test sets
every one to 0 on the shipped file and checks it loads, nothing shakes and nothing goes non-finite.

## 4. Cost

A few floats a frame; no draw. The cockpit pass already redraws the cabin every frame.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| CS1 | Do footsteps stay off the view, now there is a jolt for the weight? | Answered (owner, 2026-10-29: "can you add some shakes from footsteps back in?", "similar to the landing shake, but like 50% that strength"): each footstep kicks half a 10 m/s landing (`step_as_landing_m_s`, `step_share`), 0.21 of trauma and about a 4 cm dip. The stride's bob and sway stay off. |
| CS2 | A camera-shake slider in the settings? | Yes, 0 to 1, default 1: some players are sick with shake. |
