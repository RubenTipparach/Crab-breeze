# Proposal: save-game

## Why

The owner, 2026-10-25: "where is the persistence? if I click continue, it should load the game state where I was at
last", "continue will have two options, careeer or free roam". A career saves only its books (money, mechs, the job
board) and only when a job ends; free roam saves nothing. Continue started back at the start, on foot, with a fresh
mech.

## What changes

- **What a save holds** (`Place`):
  - where the pilot is, where they look, and whether they are in the seat;
  - where the mech is and its heading;
  - every part's structure and the parts lost;
  - the missiles left.
- **Free roam** saves to `saves/free_roam.ron`, with the world seed and mech data file it was played with.
- **A career** carries its `Place` in `saves/career.ron` with its books.
- **When:** every 30 s of play, when the pause menu opens, when the game quits, and when a job ends. Each write goes to
  a temporary file first, then replaces the save, so a crash never leaves a broken one.
- **Continue** (`--play free-continue`, `--play career-continue`) loads the world as usual, then puts the pilot and
  the mech back where the save says, with the damage and missiles it had.
- **Not yet:** the world's own damage (craters, fallen buildings, felled trees) and the jobs' enemies in the field. A
  later change.
