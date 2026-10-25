# Design: boarding-vents

## 1. Where the vents are

Two sets of empties, both written by the Blender generators and both already shipped:

| Set | Written by | Names | Extras |
| --- | --- | --- | --- |
| The chest's | `tools/blender/mech_boarding.py` | `VENT_seam_*` on the armour's seams, `VENT_open_*` at the hinges (Foreman, Ironhorse), `VENT_sill_*` along the sill | `kind` (steam, mist), `stage` (seam, open, sill), `rate` (1.0 to 2.0), `dir` |
| The cabin's | `tools/blender/build_cockpit.py` | `VENT_inner_open_*`: five over the floor (mist), the rest at the opening part's seals (steam) | the same keys, stage `inner_open` |

Seven to ten chest vents a mech, six to nine in its cabin. `dir` is the way the puff is thrown, in the empty's
parent's frame; every vent empty is unrotated, so that is also its own frame.

## 2. The timeline: the page's clock, written by the generator

`tools/mechs/gen_mech_data.py` already writes each mech's boarding on the Mechs page's clock: the sit over 0 to
3.0 s, the chest's tracks from `CHEST_AT` = 3.2 s, `ready_s` 0.3 s after the last track ends. Call the chest's
tracks' end `E` (the latest `t0 + d`, 2.9 s on the Foreman). It adds vent stages on the same clock, the page's own
windows (`gen_mech_designs.py`, `vent`):

| Stage | Window (s) | Vents | Kind | Rate a vent (a second) | Holds |
| --- | --- | --- | --- | --- | --- |
| The legs' rams work | 0 to 3.0 (the sit) | `shin_L/R`, `hock_L/R` | steam | 30 | no |
| The seals break | 3.1 to 4.4 | `VENT_seam_*` | steam | 30 x `rate` | no |
| The hinges steam | 3.5 to 3.2 + E | `VENT_open_*` | steam | 30 x `rate` | no |
| Mist over the sill | 3.2 + 0.6 E to `ready_s` | `VENT_sill_*` | mist | 6.3 x `rate` | yes |
| The cabin breathes out | 3.2 + 0.8 E to `ready_s` | `VENT_inner_open_*` | each its own | mist 6.3, steam 30, x `rate` | mist only |

Each vent is listed by its exact node name with its own rate and direction, read from the model; nothing is typed by
hand, and a vent the model drops disappears from the data at the next generation. The legs' direction is the page's
(out to the side, a little up and forward: (+-1, 0.4, 0.3) normalised, in the mech's frame) turned into each bone's
own rest frame by the generator.

## 3. The data: two new fields on a vent

`cb_core::boarding::Vent` gains:

- `dir: [f32; 3]`: the throw direction in the node's own frame (default (0, 0, 1), the old "along +z"). Validated
  finite and not zero.
- `hold: bool` (default false): keeps emitting while the clock rests inside the window.

and one rule in `BoardingTimeline`:

```rust
/// A vent emits while the clock moves through its window, or rests in it if it holds.
pub fn vent_emits(&self, stage: &Stage, vent: &Vent, prev_t_s: f32, t_s: f32) -> bool {
    self.vent_active(stage, vent, t_s) && (vent.hold || prev_t_s != t_s)
}
```

Why the clock must move: a mech that sits and seals rests at `sealed_sitting_s`, which is inside the seals' window
(3.1 to 4.4 s); without the rule it would hiss for as long as the pilot sits. The page uses the same rule ("moving").

**Stages that only vent.** `sealed_sitting_s` is "sat, before any stage begins" and today takes the first stage's
start. A vent stage at 0 s (the legs) would make it 0. It becomes the first stage that moves something (has motions),
clamped to the sit's end, which is what it always meant. The Foreman's stays 3.55 s.

The retired Breeze's hand-written timeline gains `hold: true` on its two mist vents (they ran 10 s and 8 s past an
8 s `ready_s`: held while open), so it plays as before.

## 4. Playing them: one particle path

`cb-app::vents` (`VentsPlugin`), presentation only:

- **Binding, once.** When the piloted mech's rig and interior are bound (the interior is hung under the `cockpit`
  bone and its riding parts on their chest joints, so every vent node is under the mech's root), each vent's node is
  found by name under the root. Each gets a child entity with the existing `FxStream` (`combat-fx`'s continuous
  emitter: one `bevy_hanabi` effect per burst layer, switched on and off) and a `VentEmitter { stage, vent }`. A node
  the model lacks is logged once and skipped, as hatch motions are.
- **Each frame**, `on = timeline.vent_emits(stage, vent, hatch_prev.t_s, hatch.t_s)` from the mech's two fixed-step
  boarding clocks (`MechSim`).
- **Rate and direction**: `FxStream` gains `density` (a multiplier on the recipe's stream rate, 1.0 for the jets and
  fires that use it now); a vent's density is its `rate_per_s` over the recipe's own rate. The stream is turned so
  the recipe's throw axis lies along the vent's `dir`. The look (size, speed, life, colour) is the recipe's: one
  place for what boarding steam looks like.
- **Recipes** in `data/fx/fx.ron`, added to `cb_core::fx::RECIPES` so the data check demands them:
  `vent_steam` (Steam, 0.35 m puffs growing as they rise, 3 m/s, 1.4 s) and `vent_mist` (a new particle kind, `Mist`:
  pale, slow, sinking at 0.4 m/s2 and spreading, 0.8 m, 4 s), streamed at 30 and 6 a second.
- The vents' `speed_m_s` in data is the recipe's speed times the stream's scale; the generator writes the recipe's
  speeds, so the page and the game agree.

Nothing here decides an outcome: the boarding clock is the simulation's, the vents only show it.

## 5. Cost

Particles alive at once, the Foreman (the most vents): the seals, 5 vents x 30 a second x 1.4 s = 210 during the
1.3 s window; the hinges, 2 x 42 x 1.4 = 118; the legs, 4 x 30 x 1.4 = 168 while kneeling; the mist, 3 sill + 5 cabin
vents x 6.3 a second x 4 s = 200 while open. At most about 500, against the thousands an explosion starts. 21 emitter
entities (one effect each, batched by `bevy_hanabi` per recipe), idle except in their windows. CPU: one comparison a
vent a frame. Cloud sessions cannot measure frame time; the owner's machine measures the boarding.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M12 | Should a mech vent when an enemy pilot boards or leaves it (none do today) | No: only the player boards | provisional |
