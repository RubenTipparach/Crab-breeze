---
name: bevy-architecture
description: Decide where Crab Breeze code goes and how a Bevy feature is structured - the workspace's crates (an engine-independent simulation core, the Bevy game, tools), plugins per capability, system sets and schedules, fixed-step simulation, components and resources, events and observers, assets, and the extension points. Use before starting any new feature or system (mech locomotion, cockpit, weapons, damage, buildings, crowds, cities, UI), when adding a crate or plugin, or when asking "where does this go" or "how should this be structured in Bevy".
metadata:
  author: Crab Breeze (Claude Code), after Falchion's modular-game-systems, Pale Blue Dot's core/app split and Star Crew's simulation core
  version: "1.0"
---

# Bevy architecture

The stack is Rust with Bevy (owner, 2026-10-23: "our tech stack should be bevy"). The exact Bevy
version is pinned in the workspace `Cargo.toml` (`bevy = "=0.20.0"` when the workspace is
created; Pale Blue Dot pins `=0.18.1` and is the reference for a Bevy workspace done this way).
Read the Bevy release notes and migration guide for the pinned version before writing code; never
trust remembered APIs across Bevy versions.

## Crates (dependencies point downward only)

```
crates/cb-app/     the game: Bevy app, plugins, rendering, input, audio, UI, the cockpit
crates/cb-core/    pure simulation: mech systems, ballistics, damage, heat, power, stable ids
crates/cb-tools/   command-line tools: data validation, asset checks, playtest reports
data/              tuning and content in RON, with units, validated on load
assets/            glb models, textures, sounds, fonts (what Bevy's AssetServer loads)
tools/             Blender, Material Maker and capture scripts (Python, shell)
```

- **`cb-core` has no Bevy rendering, windowing, audio or asset dependency** (Pale Blue Dot and Star
  Crew). It may use `bevy_ecs` and `bevy_math` only if the design says so; otherwise it is plain
  Rust. It runs headless and is tested without a GPU.
- **`cb-app` adapts core state into Bevy**: components that mirror core state, systems that call
  core functions, and everything a player sees and hears. The core never calls the renderer.
- **The simulation is authoritative.** The cockpit reads the simulation; it never keeps its own
  numbers. One code path for single player and any future multiplayer.

## Plugins and schedules

- **One plugin per capability**, in its own module: `mech::LocomotionPlugin`,
  `cockpit::InstrumentsPlugin`, `weapons::BallisticsPlugin`, `city::StreamingPlugin`. A plugin
  group per area wires them together; `main.rs` is the composition root and holds no logic.
- **Simulation runs in `FixedUpdate`** at a fixed rate in seconds (the rate is data). Rendering,
  camera, input sampling and UI run in `Update`. Visual interpolation between fixed steps is the
  render side's job.
- **Order with system sets**, named by phase (`SimSet::Input`, `SimSet::Locomotion`,
  `SimSet::Weapons`, `SimSet::Damage`), configured once, never with ad hoc `.after()` chains
  across plugins.
- **Events and observers for cross-capability messages** (a hit, a limb destroyed, the pilot
  boarding). A plugin never reaches into another plugin's components to change them.
- **Input is actions, not keys.** Systems read action state ("throttle", "torso twist", "fire
  group 1"); a binding table in data maps keyboard, mouse, pad and HOTAS to actions. The scripted
  pilot (`playtest`) writes the same actions.

## The player is always first person

- **One camera, the pilot's eyes.** On foot it is the body's head. Boarding moves the same camera
  into the cockpit seat's `EYE_pilot` empty (`blender-hard-surface`), in one continuous motion.
- **The cockpit is real geometry in the world**, parented to the mech's torso, rendered with the
  world. Instruments are textures or meshes driven by simulation state, and screens inside the
  cockpit render to textures. A full-screen 2D overlay is never the cockpit.
- **Head motion is simulated**: the cockpit sways with the mech's gait and recoil as a computed
  effect, with an accessibility setting to damp it.

## Extension points (prefer these over new code paths)

| To add | Do this |
| --- | --- |
| a mech chassis | a data entry in `data/mechs/` plus its glb; no code unless a new part kind |
| a weapon | a data entry; a new firing behaviour is one trait `impl`, registered once |
| a cockpit instrument | a component reading one simulation value, and its mesh or screen in the cockpit glb |
| a building or city block | a glb from `blender-csg-levels` and a placement entry in the city's data |
| a person | a row in the character table (`blender-humanoid-characters`) |
| a playtest | a script in `data/playtests/` (`playtest`) |

## Feature checklist

1. Write the OpenSpec change first (`/opsx:propose`): proposal, design with numbers and costs,
   spec deltas, tasks. Stop there until asked to build.
2. Rules in `cb-core`, tuning in `data/`, visuals and input in `cb-app`.
3. Unit tests for the core (`testing-practices`).
4. Wire the plugin into its group.
5. Play it (`playtest`), and measure it if it can move frame time (`perf-measure`).
