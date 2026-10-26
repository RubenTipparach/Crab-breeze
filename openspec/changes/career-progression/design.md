# Design: career-progression

## 1. Data

- `data/jobs/jobs.ron`: each job kind gains `tiers: [(tier, mechs, pay)]`; the generator picks a tier per offer.
- Each mech's data gains `tier` (1 to 4) and `price` (with `mech-shop`).
- `data/career/career.ron`: `standing_per_job`, `standing_lost`, `pay_bonus_per_standing`.

## 2. Rules (`cb-core::jobs`, pure)

- An offer of tier `t` is made only when the career owns a mech with `tier >= t`.
- Pay is the tier's pay times (1 + the standing bonus at the offering base), rounded to 10 credits.
- Standing per base is kept in the career (`Career.standing`, one number per base).

## 3. Cost

Nothing per frame; a few numbers in the save.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CP1 | How many tiers | four | provisional |
| CP2 | Does standing ever close a base to the pilot | no, it only changes pay | provisional |
