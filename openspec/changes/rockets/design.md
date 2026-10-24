# Design: rockets

| Weapon | Speed | Interval | Penetration | Damage | Blast |
| --- | --- | --- | --- | --- | --- |
| 90 mm rocket pod | 240 m/s | 1.6 s | 120 mm | 30 | 2 MJ |

At 240 m/s a rocket fired from 400 m takes 1.7 s to arrive: time for a pilot who sees the motor
flame to step aside at 7 m/s (12 m, more than a mech's 3 m body radius).

- `Projectile` gains `rocket: bool`, copied from its weapon when fired. In `fly`, a rocket keeps
  `vel` unchanged; a shell keeps `v.y -= g dt`.
- `lead(from, target, vel, speed, drop)`: the drop term `0.5 g t^2` only when `drop` is true. The
  AI passes `!rocket` for its main weapon.
- Cost: nothing measurable (one branch per projectile per step).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G11 | Who carries rockets first | 3 of every 10 enemies in the defence waves carry a rocket pod; the player's lock-on missiles come with radar (G4) | provisional (survey G11) |
