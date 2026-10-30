# Design: offshore-limit

## 1. The limit

`cb_core::bounds::offshore(shape, p) -> Offshore { past_m, limit_m }`:

- `past_m = shape.past_land_edge_m(p)`: metres out to sea past the coast (negative on land). Already exists.
- `edge_m`: distance from `p` to the nearest world edge (`world_size_m / 2 - max(|x|, |z|)`).
- The limit is reached when `past_m >= limit_offshore_m` (15,000) **or** `edge_m <= edge_margin_m` (1,000).
- The warning starts at `past_m >= limit_offshore_m - warn_m` (2,000) or `edge_m <= edge_margin_m + warn_m`.
- The HUD shows the smaller of the two distances left.

Tuning in `data/world/bounds.ron` (`limit_offshore_m`, `edge_margin_m`, `warn_m`, `grace_s`).

Measured on the pilot's position in `FixedUpdate`, every step, whatever they ride: mech, car, stolen aircraft or on
foot (a pilot swimming is not a case today). Cost: one height lookup and two compares a step.

## 2. Failing

Once past the limit for `grace_s` (1.0 s, so one step of jitter at the line cannot fail a run), the active mission
fails with `Loss::OutOfBounds`. Free roam has no mission object; `cb_core::missions` (the free-roam board) gets the
same loss and shows the same screen. The fail happens once: the state goes to `Respawning` and the check stops.

## 3. Respawning

`respawn(world, pilot, mech)`:

1. The mission-failed screen shows (`OUT OF BOUNDS`) for 3 s over the frozen view. Look input stays live (CLAUDE.md
   section 8: the player turns the view, nothing else does).
2. Behind the loading screen, the nearest city to the pilot's last position on land is chosen; `air_drop::drop_site`
   picks a clear spot in it for a standing mech.
3. The mech is repaired to full (the career's `call_out` repair), placed standing there, its heading toward the
   city's square; the pilot is seated in it. A stolen aircraft or car is left where it was.
4. `GAME_READY`-style ready check: the world around the spot streams in before the loading screen lifts.

The view starts in the seat facing out of the cockpit (as after boarding), and the mouse is the player's at once.
## 4. As built (2026-10-30)

- The rule is `cb_core::bounds` (`left_m`, `Watch`, `respawn_city`), its numbers `bounds:` in `data/world/world.ron`.
- `cb-app::bounds` reads the pilot's place each fixed step after the mech (the mech while seated or boarding, the
  driven car, else the feet). The warning is a banner over the view in every mode (`RETURN TO COAST n.n KM`) and the
  alarm, at the cockpit speaker when seated, else at the pilot, every 2 s.
- The failure stops free roam's running mission (`FreeRoamRun::stop`) and loses a defence run
  (`Defence::fail(Loss::OutOfBounds)`); the banner reads `OUT OF BOUNDS / MISSION FAILED`.
- **The respawn is on foot, not seated**: after `respawn_after_s` the pilot stands in the nearest city's square and a
  fresh mech is dropped beside them by the mech swap's own path (`SwapMech`, the career's drop search), to walk to
  and board. Putting them straight into the seat would cut past the boarding the law says is walked; O1 below.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| O1 | Should a pilot in a stolen aircraft respawn in their mech, or at the airport they stole it from? | On foot beside a fresh mech in the nearest city, for every pilot (boarding is walked) |
| O2 | Does a respawn cost anything in the career (money, repair bill)? | Nothing for now |
