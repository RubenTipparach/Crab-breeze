# Design: enemy-mechs

The AI runs in the fixed step before the mechs move; it reads only what a pilot could know (the
target's position and velocity) and writes `MechControls` plus fire. Skill sets aim noise
(degrees), reaction time and preferred range. Lead: the intercept of a projectile at muzzle speed
with a target moving at constant velocity. Cost: microseconds per AI.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G6 | First fight | two enemy stand-ins in the city, cannons and autocannons | provisional (survey G6) |

## As built (2026-10-24)

- Two enemies from `data/world/encounters.ron`, the stand-in in a rust tint with only the autocannon. No line-of-fire check yet (they shoot through each other).
