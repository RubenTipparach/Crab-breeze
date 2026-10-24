# Design: cargo-workspace

## 1. Layout

```
Cargo.toml            workspace, pinned versions, lints, profiles
crates/cb-core/       pure simulation: glam, serde, ron. No Bevy.
crates/cb-app/        the game: Bevy 0.20.0, binary `crab-breeze`
crates/cb-tools/      `cb-tools validate <dir>` loads every data file through cb-core
data/                 RON tuning, units in every key
```

`cb-core` uses `glam` 0.33 directly, the same version Bevy 0.20 re-exports as `bevy::math`, so a
core `Vec3` is a Bevy `Vec3` with no conversion. It does not depend on any Bevy crate.

## 2. Seeded randomness

`SimRng` is SplitMix64 (public domain, Steele, Lea and Flood 2014): 64 bits of state, one
multiply-xorshift per draw, identical on every platform. `fork(label)` derives a child stream from
the parent's seed and an FNV-1a hash of the label, without advancing the parent, so adding a new
subsystem never shifts another subsystem's draws (`deterministic-simulation` rule 3).

## 3. Data loading

`cb_core::data::load_ron::<T>(path)` reads a file, parses it with `ron`, and runs `Validate` on the
result. Types derive `Deserialize` with `#[serde(deny_unknown_fields, default)]`: a missing field
takes the code default, a present zero is zero, an unknown key is an error. `Validate` rejects
non-finite numbers and out-of-range values with the field name. The error names the path.

`cb-tools validate data/` loads every known file through the same function. The game calls the same
function at startup and stops with the error if a file fails.

## 4. Profiles

`dev` builds dependencies at `opt-level = 3` so the game is playable while iterating; `release` is
for measurements (`perf-measure`).

## 5. Cost

No runtime cost. A clean release build of Bevy on the owner's Mac takes a few minutes once.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T1 | Bevy version | `=0.20.0` | provisional (survey T1) |
