---
name: testing-practices
description: How to test Crab Breeze code - unit tests beside the rule in the core crate, named as sentences, a failing test before every bug fix, golden seeds, data validation, headless render captures, and the full check before a push. Use when adding or changing logic, fixing a bug, adding data, or before committing or pushing.
metadata:
  author: Crab Breeze (Claude Code), after Falchion's testing-practices and Star Crew's verification rules
  version: "1.0"
---

# Testing practices

- **Tests sit with the rule** (`#[cfg(test)] mod tests` in the same file, or `tests/` for a crate's
  public behaviour) and **read as sentences**:
  `a_mech_over_its_heat_limit_shuts_down_within_two_seconds`. One behaviour per test. A
  non-obvious assertion carries a message saying why it matters.
- **`cb-core` has a unit test for every public behaviour.** It is pure, so tests need no mocks
  beyond a seeded RNG and a fixed `dt_s`.
- **Property checks for maths**: symmetry, round trips, conservation (energy in equals energy out
  plus losses), bounds.
- **Golden seeds** for generation and scripted runs (`deterministic-simulation`).
- **A bug fix brings its regression test**, failing first, committed with the fix.
- **Data is validated by loading the real files** (`cb-tools validate data/`), never by a second
  hand-written copy of the expected value.
- **Bevy systems** are tested with a minimal `App` (`MinimalPlugins` plus the plugin under test),
  stepped with `app.update()`, asserting on components and events.
- **What it looks like** is checked by a capture, looked at by eye: headless stills on lavapipe in
  a cloud session, a playtest recording on the owner's machine (`playtest`). A green test is not a
  visual sign-off.
- **Every OpenSpec scenario maps to a test or a playtest beat**, named in its `tasks.md`.

## Before every push

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
openspec validate --all
LC_ALL=C.UTF-8 grep -rnIP '\x{2014}|\x{2013}' --exclude-dir=.git --exclude-dir=.claude --exclude-dir=target --exclude='*.hdr' . && echo FAIL
```

Until the Cargo workspace exists, only the last two apply.
