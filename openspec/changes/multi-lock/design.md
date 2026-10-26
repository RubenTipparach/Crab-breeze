# Design: multi-lock

## 1. The seeker is the missile's data

`WeaponSpec` gains `seeker: Option<Seeker>` and loses `seeker_turn_deg_s` (folded in):

```rust
pub struct Seeker {
    pub max_locks: u8,       // 1 for the shoulder missile, 6 for the pod
    pub lock_s: f32,         // time to add one lock
    pub range_m: f32,
    pub cone_deg: f32,       // a lock builds only inside this cone
    pub turn_deg_s: f32,     // how hard the missile turns toward its lock
}
```

`TargetingConfig` keeps the boxes and the pick (`max_range_m`, `pick_cone_deg`, `box_px`) and loses
`lock_range_m`, `lock_cone_deg` and `lock_s`, which move to the shoulder missile's seeker with the
same values (1 200 m, 12 degrees, 1.0 s). Data is migrated in the same commit.

| | Shoulder missile (today) | Multi-lock pod |
| --- | --- | --- |
| `max_locks` | 1 | 6 |
| `lock_s` | 1.0 s | 0.35 s a lock |
| `range_m` | 1 200 m | 1 500 m |
| `cone_deg` | 12 | 25 |
| `turn_deg_s` | 70 | 60 |
| Speed | 260 m/s | 240 m/s |
| Damage, penetration, blast | 45, 150 mm, 2.5 MJ | 30, 100 mm, 1.5 MJ each |
| Interval | 1.2 s | 8.0 s after a salvo |

## 2. Locks

`targeting::Lock` becomes `Locks { held: Vec<u32>, building_s: f32 }` (target ids, a target listed
once per lock). Each fixed step while the missile group is held:

1. Drop locks on targets now out of `range_m` or beyond `cone_deg * 1.5` from the aim.
2. If `held.len() < max_locks`, the candidate is the target inside range and cone with the fewest
   locks, nearest the aim on a tie, then lowest id (stable order, CLAUDE.md 6.3). Add `dt` to
   `building_s`; at `lock_s` push the candidate and reset.
3. On release (the group's press ends), fire one missile per held lock, `0.08 s` apart in lock order,
   each homing with `weapons::home` at `turn_deg_s`; clear the locks; start the interval.

With `max_locks: 1` this is today's behaviour exactly: one target, `lock_s` to lock, fire on F. The
difference: today F fires on press with the lock already built; the new rule builds while held and
fires on release. For a one-lock seeker the press also fires at once if a lock is already held, so
the shoulder missile plays as it does now.

Six locks take 2.1 s of sweeping. Against Big Boy, each `WEAK_*` point is listed as a target with
its own id (`crab-enemies`).

## 3. Cockpit

The selected-target box and every other box show `xN` for N locks; the reticle shows a ring of
`max_locks` pips filled by `held.len()` and a sweep arc for `building_s / lock_s`; `IN RANGE` and
`LOCK` keep their meaning for a one-lock seeker. All drawn from `Locks`, the state the missiles fire
from (CLAUDE.md 6.1). Sound cues for `audio`: `lock_tick` per lock (the Call Sign rhythm on the
last, `docs/design/music.md`), `salvo`.

## 4. AI

An AI pilot with a pod holds F while two or more targets are inside its cone, and releases at three
locks or after 3 s; with one target, at all six. Same `Locks`, same action.

## 5. Cost

Per step per mech holding the group: one pass over the visible targets (at most a few dozen) for
range and angle. Six missiles in flight follow the existing missile path. Negligible.

## Open questions

None new: what a pod is carried by is survey G22 (`weapon-kinds`), decided by the owner on 2026-10-26: anyone
can carry every weapon, on both sides, multi-lock pods included.

## Built, and the calls made on the way

Built 2026-10-26 with the advanced weapons, on the owner's "Approve all" and "Go and implement"; the owner said
"Use recommended for survey" for any choice on the way, and these were taken:

- **One lock rule, data left where it is.** `Seeker` is on `WeaponSpec` (`seeker: Some((max_locks, lock_s,
  range_m, cone_deg, turn_deg_s))`) and `Seeker::reaches` is the one test of what a lock may build on, used by the
  pod's `Locks` and by the shoulder missile's `Lock`. The shoulder missile's numbers stay in
  `data/hud/targeting.ron` and are read as a one-lock seeker (`TargetingConfig::seeker`), rather than moved into
  every mech's weapon data, so the eight generated mech files and other branches' data are untouched. The test
  `the_shoulder_missile_keeps_its_lock_numbers` pins them. `seeker_turn_deg_s` still loads; `turn_deg_s()` reads
  either.
- **Fog (merged with ground-fog, survey F2).** `Seeker::reaches` and `Seeker::keeps` take the fog toward the
  target (`fog_scale`, carried on each `Target`), so fog shortens a pod's range and slows its locks exactly as it
  does the shoulder missile's; an enemy pod locking on the pilot reads the same fog along the same line
  (`targeting::fog_reach` in `cb-app`).
- **Locks live on the mech's guns** (`Guns::locks`): the player's build in `targeting` while F is held, an AI's in
  its own step; the release and the salvo are one path (`Guns::release`, `Guns::take_queued`, 0.08 s apart, the
  interval from the last missile). A burst (`signature-weapons`) uses the same queue.
- **Who carries one** (G22): the hangar sells a six-cell pod for the missile station; the raiders' railgun Brigand
  and the first fight's Tyrant carry one.
- **Cockpit**: each locked box reads `xN`; the WEAPONS page reads `LOCKS n/6`. The pip ring and sweep arc on the
  reticle are not built yet.
- **Big Boy's weak points** as separate lock targets are not built: `crab-enemies` has no `WEAK_*` targets in the
  game yet.
