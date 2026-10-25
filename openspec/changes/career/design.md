# Design: career

## 1. State (`cb-core::career`, pure)

```text
Career {
  name: String,                 // the save's name
  money: i64,                   // whole credits; never below 0
  mechs: Vec<OwnedMech>,        // in purchase order; stable ids
  piloting: Option<MechKey>,    // the mech the pilot is in or last boarded
  job: Option<JobKey>,          // the accepted job (mission-jobs)
  done: Vec<JobRecord>,         // finished jobs, newest last
  ledger: Vec<Entry>,           // every credit and charge, newest last
  seed: u64,                    // forks the job generator's randomness
}
OwnedMech { key: MechKey, design: String /* data/mechs/<id>.ron */, base: u32, bay: u32,
            structure: Vec<f32> /* per part, as Damage */, lost: Vec<bool> }
Entry { day_s: f64, amount: i64, what: String }
```

Money is an integer number of credits so sums never drift. `Damage` already holds structure and lost
parts per part (`mech-damage`); an owned mech stores exactly that, so a mech spawned from the career
is spawned with its damage and a mech that lands is written back from its damage. One shape, no copy.

## 2. The ledger

`Career::pay(amount, what)` and `Career::charge(amount, what) -> Result<(), Short>`:
- a charge larger than the money changes nothing and returns how much is missing;
- non-positive or overflowing amounts are refused;
- both append an `Entry`, so the MAIL page can show a statement.

Repairs (air-bases), purchases (air-bases) and job pay (mission-jobs) all call these. No other code
changes `money`.

## 3. Saving (`cb-app::career`)

- RON, `saves/<name>.ron` under the repository root in development (beside the game when shipped).
- Written on: job paid, repair or purchase done, landing at a base, Quit. Written to a temporary file
  and renamed, so a crash mid-write keeps the last good save.
- Loaded on Continue (title menu). A file that fails to load is reported with its path and field and
  the title menu stays open; it is never silently replaced.
- Versioned (`version: 1`); an older version is upgraded field by field or refused with a message.

## 4. Numbers (data, `data/career/career.ron`, provisional on survey J4)

| Item | Value |
|---|---|
| Starting money | 200 000 credits |
| Starting mech | Foreman, home base bay 1 |
| Job pay | 40 000-150 000 (mission-jobs) |
| Mech prices | 300 000-1 200 000 (air-bases) |
| Full repair | up to 30 % of the mech's price (air-bases) |

## 5. Cost

No rendering. The career is a few kilobytes; a save is under 50 kB and written a handful of times a
session, off the main thread.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J4 | Money: start, pay and prices | 200 000 to start, jobs 40 000-150 000, mechs 300 000-1 200 000 | provisional (survey J4) |
| J3 | What losing your mech costs | Recovered to the nearest base, a full repair bill, the job's pay lost | provisional (survey J3) |
