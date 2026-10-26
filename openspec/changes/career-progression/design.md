# Design: career-progression

## 1. Data

- `data/jobs/jobs.ron`: each job kind gains `tiers: [(tier, mechs, pay)]`; the generator picks a tier per offer.
- Each mech's data gains `tier` (1 to 4) and `price` (with `mech-shop`).
- `data/career/career.ron`: `standing_per_job`, `standing_lost`, `pay_bonus_per_standing`.

## 2. Rules (`cb-core::jobs`, pure)

- An offer of tier `t` is made only when the career owns a mech with `tier >= t`.
- Pay is the tier's pay times (1 + the standing bonus at the offering base), rounded to 10 credits.
- Standing per base is kept in the career (`Career.standing`, one number per base).

## 3. Decided while building (2026-10-25)

- An offer's tier is drawn at even odds among the tiers open for its kind; the Destroy kind has no tier 1.
- The offering base is the base nearest the job's site; its standing sets the bonus.
- When a purchase opens a tier, the board offers one job of that tier at once (beyond `open_offers`) and mails
  "Tier n contracts open", so the new tier is seen without waiting for an offer slot.
- Standing is 0 to `standing_max` (10), one point a job won, one lost for a failed or abandoned one, 5 % pay a point.

## 4. Cost

Nothing per frame; a few numbers in the save.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CP1 | How many tiers | four | provisional |
| CP2 | Does standing ever close a base to the pilot | no, it only changes pay | provisional |
