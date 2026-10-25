# Proposal: career

## Why

The owner (2026-10-24): "The player will need to open emails and accept jobs... air bases allow the
player to fix their mechs, buy new mechs and get into different mechs." Jobs pay, repairs and mechs
cost, and the mechs you own keep their damage between fights. None of that exists: today every run
starts fresh with one mech. A career is the state those features share, so it is its own change.

## What changes

- **The career:** the pilot's money, the mechs they own (which design, which air base and bay it is
  parked in, its damage part by part), the job taken (if any) and the jobs done.
- **Saved to disk** as `saves/<name>.ron` beside the game, written when something worth keeping
  happens (a job paid, a repair or purchase made, landing at a base) and when you quit. Continue on
  the title menu loads the latest one.
- **Money moves in one place:** every payment and charge (job pay, repair, purchase) goes through
  one ledger function that refuses a charge you cannot afford and records what it was for.
- **New career:** starts at the home air base with one Foreman in bay 1 and starting money from data.
