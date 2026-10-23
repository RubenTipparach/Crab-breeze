# Proposal: playtest-pipeline

## Why

The owner, 2026-10-23: "the most important thing is playtesting, obs recording and uploading to
Google drive." CLAUDE.md section 9 makes a playtest the proof that a feature is done. Before the
game has a mech, it needs the means to play itself the way a person does, print when to record,
and hand the take to OBS and Google Drive. Built first, every later feature is proven the same way
from its first day.

## What changes

- A `--playtest <name>` flag on the game that loads a scripted scene and drives the pilot through
  beats read from `data/playtests/<name>.ron`, through the same input actions a player sends.
- Output lines `PLAYTEST_READY`, `PLAYTEST_START`, `PLAYTEST_BEAT`, `PLAYTEST_RESULT`,
  `PLAYTEST_END` on stdout, and a JSON report written beside the run.
- A wrapper, `tools/playtest.py`, that runs a script, records it with `obs-record`, makes the share
  copy, copies it to the Drive for Desktop folder and writes `docs/playtests/<date>-<name>.md`.
- Headless stills on lavapipe for cloud sessions (`--playtest <name> --stills`).

## Out of scope

The mech, the cockpit and the city themselves. The first script, `boarding`, runs in a grey test
yard with a stand-in mech, so the pipeline is proven before the art exists.
