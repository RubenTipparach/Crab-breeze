# Proposal: an itch.io build and deploy pipeline for Windows, Linux and Mac

## Why

The owner, 2026-11-17: "do you have butler installed/set up on secrets? look at how pale-blue-dot has builds set up for
mac, linux, windows. do that here".

Crab Breeze has no CI (no `.github/`), and a release build cannot be shipped as it is. The game finds `data/` and
`assets/` through `cli::repo_root()`: `CB_ROOT`, else the nearest ancestor of the working directory holding
`data/world/world.ron`, else the checkout it was compiled in (`CARGO_MANIFEST_DIR/../..`). A build made on a CI runner
and double-clicked on a player's machine (working directory `/` on macOS, the shortcut's folder on Windows) finds no
`data/` above its working directory and falls back to the runner's path, which does not exist there.

## What

1. **One manually triggered workflow**, `.github/workflows/deploy-itch.yml`, Pale Blue Dot's
   (`RubenTipparach/Pale-Blue-Dot`, `openspec/changes/itch-deploy`, read 2026-11-17), adapted. `workflow_dispatch` only, so
   it runs on whichever branch is picked in "Run workflow". It builds the game (`cb-app`, the `crab-breeze` binary) with
   `--release` on Windows, Linux and macOS (Apple Silicon) and packages each as the exe, `data/` and `assets/` beside it
   and a `README.txt`. Each package is a workflow artifact. With the `deploy` box ticked (the default), each is pushed to
   itch with butler on the channels `windows`, `linux` and `mac`, with an optional suffix (`-beta`).
2. **The game finds its files beside its exe.** `cli::repo_root()` gains one rule, ahead of the working directory: the
   exe's own folder when it holds `data/world/world.ron`. Every runtime reader of `data/`, `assets/` and `saves/` already
   goes through `repo_root()`, so a shipped game reads its files and keeps its saves beside its exe. A dev build in
   `target/release` has no `data/` beside it, so a dev run is unchanged.

## Not in this change

- Code signing or notarisation of the Mac build, and an Intel Mac build.
- Running the tests in CI (this is a build and deploy pipeline, as Pale Blue Dot's is).
