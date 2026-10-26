# Far buildings, every type its own look: cloud stills, 2026-10-26

**This is not a playtest.** These are headless stills from a cloud session: Mesa's software Vulkan
(lavapipe) under Xvfb, 4 CPU cores, no GPU, frames of about 1.7 s, debug build. A playtest belongs on
the owner's machine (`./run.sh`, then skate toward a city).

- Change: `openspec/changes/building-type-lods`.
- Command: `CB_STILLS_POSTER=1 CB_FAR_SHEET=<file.png> VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
  xvfb-run -a -s "-screen 0 1920x1080x24" ./target/debug/crab-breeze --start overlook --stills <dir>`, seed 42.
- Before: main at `e5d1e24`, the same command.
- Stills: `docs/screenshots/building-lods/`.

## What the stills show

- **Before** (`overlook-before.jpg`): from the hill, most of the city past 250 m is near-black boxes in
  one shared window grid; only a few read as buildings at all.
- **After** (`overlook-after.jpg`, `skyline-before-after.jpg`): glass curtain-wall towers in their
  tints, pale ribbon and punched concrete offices, brick walk-ups with their cornices, and parapets
  and water tanks on the roofs (the massing, from each building's own dressing).
- **The tiles** (`far-facades-sheet.jpg`): per kind a row, per main colour a ground storey and a
  typical storey: walk-up (shopfront, sash windows; 6 bricks), ribbon office and punched office (4
  concretes each), tower (curtain wall; 4 glasses), warehouse (loading door, steel sash; 6 bricks).

## What these stills cannot show

- The swap at 250 m and 1.5 km while moving (task 3.3, a playtest), and what it costs: a cloud
  session measures nothing (task 3.2, on the owner's machine).
- A tower's ground storey shows its typical curtain wall: the sampled bay is not the lobby door.

## Found on the way

- `fx.rs` despawned a point light twice when the light budget evicted it in the same frame its fade
  ended, a warning on every run whose warm-up frames ran long; both now use `try_despawn`.
- `mech::locomotion::tests::the_torso_eases_into_a_twist_and_brakes_onto_its_aim` fails on main as
  well ("no jump in rate"); not touched here.

## Checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | all pass but the locomotion test above, which fails on main too |
| `cargo run -p cb-tools -- validate data/` | passes |
| `openspec validate --all` | 101 passed |
