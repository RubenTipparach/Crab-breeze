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
| Own gun | `recoil_per_mm * penetration` | none | each round fired |
| Melee | `melee_kick` | `melee_jolt_m_s` | a strike landing |
| Jets lighting | `ignite_kick` | none | the jets turning on |
| Blast nearby | as now | none | `blast::Explosion` |

Rumble targets: jets `jets_rumble * thrust share`; skates `skate_rumble * min(1, speed / skate_full_m_s) * surface`
(`surface`: 1 on dirt and grass, `concrete_share` on paved ground).

The rules (kick from an event, rumble from a state, the spring) are pure functions in `cb_core` with their tests; the
app reads the mech's state and messages and draws.

## 3. Numbers to start (data `fx.shake`, provisional; tuned by the owner in play)

`max_deg` 1.2, `decay_s` 0.7 (as now); `land_from_m_s` 3, `land_per_m_s` 0.06; `ram_from_m_s` 2, `ram_per_m_s` 0.08;
`hit_per_mj` 0.08, `hit_per_mm` 0.0015; `recoil_per_mm` 0.0006; `melee_kick` 0.45, `melee_jolt_m_s` 0.6;
`ignite_kick` 0.15; `rumble_deg` 0.25, `rumble_hz` 23, `rumble_ease_s` 0.3, `jets_rumble` 0.7, `skate_rumble` 0.5,
`skate_full_m_s` 30, `paved_share` 0.5; `jolt_per_m_s` 0.12, `jolt_max_m` 0.25, `jolt_hz` 1.6, `jolt_damping` 0.45
(0.04 and 2.2 Hz dipped a 12 m/s landing 2 cm, too little to feel; this is about 9 cm).

## 4. Cost

A few floats a frame; no draw. The cockpit pass already redraws the cabin every frame.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| CS1 | Do footsteps stay off the view, now there is a jolt for the weight? | Yes, the owner's 2026-10-25 rule; a landing and a ram carry the weight. |
| CS2 | A camera-shake slider in the settings? | Yes, 0 to 1, default 1: some players are sick with shake. |
