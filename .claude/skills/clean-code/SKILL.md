---
name: clean-code
description: Write and review Crab Breeze Rust the repository's way - one implementation of every rule, SOLID applied to Rust and Bevy (small plugins and systems, traits as narrow roles, registries over match arms, dependencies passed in), tuning in validated data with units, documented contracts, guarded edges, no warnings. Use when writing a new module, system, plugin or component, adding a feature to an existing system, refactoring, or reviewing a diff ("clean this up", "review this", "where should this go", "is this SOLID", "refactor").
metadata:
  author: Crab Breeze (Claude Code), after Falchion's solid-principles, Star Crew's and Pale Blue Dot's code rules, and Mining the Sky's "never rebuild"
  version: "1.0"
---

# Clean code in Crab Breeze

Crab Breeze will grow for a long time: more mechs, more weapons, more cities, more people. Write
code so that extension means **adding** a module, not editing ten. CLAUDE.md section 6 holds the
rules; this is the working checklist.

## One implementation of every rule

The owner's highest-priority rule across Star Crew, Undercity and Mining the Sky.
- **Search first.** Before writing a generator, a system, a UI panel or a math helper, find the
  one that exists. Extend or parameterise it: an argument, a variant, a strategy.
- **Never rebuild an existing feature from scratch.** If the existing code cannot be adapted, say
  why before replacing it.
- **What the cockpit shows is computed by the code that resolves it.** A heat gauge reads the heat
  model; a weapon's lead indicator calls the ballistics the shot uses. A preview that disagrees
  with the outcome is the bug this rule prevents.
- **Report duplication** when you find it. Never add a third copy.

## SOLID, in Rust and Bevy

- **Single responsibility.** One reason to change per module. A plugin owns one capability
  (`MechLocomotionPlugin`, `CockpitInstrumentsPlugin`). A system does one job; if its doc needs
  "and", split it. Simulation rules live in the core crate, never in a rendering module.
- **Open/closed.** Extend through registries, traits and data, not `match` arms spread over the
  codebase. A new weapon is a data entry plus, if it needs new behaviour, one `impl` of the weapon
  trait registered in one place. Adding a variant must not touch the variants that work.
- **Liskov substitution.** Every `impl` of a trait honours the whole contract: no panicking
  `unimplemented!()`, no hidden preconditions ("only works above 10 m/s").
- **Interface segregation.** Small traits and small components. A system's query asks for the
  components it reads and writes, nothing more (`Query<(&Heat, &mut Coolant)>`, not the whole mech).
- **Dependency inversion.** Domain code takes what it needs as parameters: the seeded RNG, the
  simulation time step, the data tables. Concrete wiring happens in the app's composition root
  (`main.rs` and the plugin groups), never deep inside a system.

## Conventions

- **Document the contract.** Every file opens with a `//!` comment saying what it owns and why it
  lives there. Every public item has a doc comment. Units go in the name or the doc
  (`mass_kg`, `stride_m`, `torque_n_m`).
- **No inline tuning.** Tuning values live in `data/` (RON), with the unit in the key, validated on
  load. A missing field takes the code default; a present zero is zero; an unknown key is an error.
  A data file that fails to load stops startup with its path and field.
- **Guard the edges.** Reject non-finite numbers where they enter (data, input, physics results,
  network, saves). Check indices. An operation that does not fit a capacity changes nothing.
- **Warnings are errors.** `cargo fmt --all -- --check` and
  `cargo clippy --all-targets -- -D warnings` pass before every push.
- **Borrowed code keeps its provenance**: source repository and revision beside it.
- **Writing style**: no em dashes or en dashes anywhere (CLAUDE.md section 5).

## Review checklist

- [ ] Does this already exist somewhere? Was it extended rather than copied?
- [ ] Does the core crate stay free of rendering, windowing, audio and asset-loading code?
- [ ] Can a new variant be added by data or registration, without editing a `match` elsewhere?
- [ ] Are tuning numbers in data, with units?
- [ ] Is every public item documented, every file headed with its contract?
- [ ] Does every rule have a test that reads as a sentence (`testing-practices`)?
- [ ] Is anything random seeded and anything ordered stable (`deterministic-simulation`)?
