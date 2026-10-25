# Design: weapon-select

## 1. Stations (`cb-core::weapons`, pure)

- `next_station(specs, current, step, usable) -> usize`: the next station `step` away (+1 or -1),
  wrapping, skipping any `usable(i)` refuses (a lost arm's gun); the current one if none other is
  usable.
- `station_pulls(specs, selected, primary, secondary, missile) -> Vec<bool>`: which weapons are pulled
  this step: the selected one on the primary trigger, every group-1 weapon on the secondary, every
  group-2 one on the missile key.
- `group_pulls(specs, [bool; 3]) -> Vec<bool>`: the old rule, every weapon of each pulled group, for
  the enemy AI.
- `Guns::trigger` takes the per-weapon pulls. One trigger rule; the two callers only say what is
  pulled.
- A mech starts with its first primary-group weapon selected.

## 2. Input

`Actions::weapon_cycle` (-1, 0, +1 this step), latched like `target_next`. The binding
`weapon_cycle` takes key names (each steps forward) and `MouseWheel` (each notch steps, up back, down
on); default `["MouseWheel", "BracketRight"]`. The Controls page lists it.

## 3. The screens (cockpit-mfds)

- The MFD texture holds five pages, 2560 x 512: MECH, FLIGHT, MISSION, WEAPONS, TARGET.
- Besides `HUD_mfd_N_screen`, the dash's `SCREEN_left_screen`, `SCREEN_center_screen` and
  `SCREEN_right_screen` meshes are bound by exact name, starting on MECH, TARGET and WEAPONS. Their
  baked pictures go; every screen is live.
- **WEAPONS:** up to six rows, `STA n  NAME  STATE`, the selected row in an amber box, a lost one red.
- **TARGET:** `TARGET` and the name (a mech's design name, a vehicle's kind), `1240 M  BRG 045`, the lock
  (`LOCK` or `LOCKING`), and its health: a mech's torso structure, a vehicle's hit points, as a
  share. `NO TARGET` with none.
- The FLIGHT page's gun line shows the selected station instead of every gun.

## 4. Cost

Two more 512 x 512 pages in the one MFD texture (2560 x 512 instead of 1536 x 512, about 2.6 MB more)
drawn by the same camera; a few UI nodes updated a frame. Measured with the MFD camera in the perf
suite (cockpit-mfds 3.1).

## Open questions

None: the owner asked for each of these in words.
