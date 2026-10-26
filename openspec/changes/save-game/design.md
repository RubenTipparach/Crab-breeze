# Design: save-game

## 1. The record (`cb-core::save`, pure)

```rust
pub struct Place {
    pub pilot_feet: [f32; 3], pub pilot_yaw_rad: f32, pub pilot_pitch_rad: f32, pub seated: bool,
    pub mech_pos: [f32; 3], pub mech_heading_rad: f32,
    pub structure: Vec<f32>, pub lost: Vec<bool>, pub rounds_left: Vec<Option<u32>>,
}
pub struct FreeRoamSave { pub version: u32, pub seed: u64, pub mech: String, pub place: Place }
```

`Career` gains `place: Option<Place>` (`#[serde(default)]`, so older saves load with none). Every number is checked
finite on load; a save that fails to parse or check is set aside as `<name>.bad` and the game starts fresh, saying so
in the log.

## 2. Saving (`cb-app::save_game`)

A system in `Update` builds a `Place` from the pilot and the piloted mech and writes it every 30 s of real time, when
the pause menu opens and on `AppExit`. It saves only in a player's free roam or career (`--play`), never in a
scripted take. A career's `Place` goes into the career and is written with it, through the career's one writer.
Nothing is saved while the pilot is mid-boarding or climbing out (the save waits for the next moment on foot or
seated).

## 3. Restoring

Once the piloted mech exists after `GameReady`, a one-shot system applies the save. It does so only when the seed and
mech file match; otherwise it says so and starts fresh.

- **Mech:** `prev` and `curr` move to the saved position and heading, the damage is copied in, and lost parts go
  into `cuts` so the rig cuts them off as a hit would.
- **Missiles:** `rounds_left` is copied in.
- **Pilot:** seated, the pilot goes straight into the seat (the hatch at its default, standing and sealed). On foot,
  the pilot stands at the saved feet, looking where they looked.

## 4. Cost

A save is a few hundred bytes, written in under a millisecond, every 30 s.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| SG1 | Save the world's damage too | later, its own change | provisional |
