# Design: rubble

## 1. Deposit

- On an island's landing, `volume = sum of its element volumes x rubble_fines` (0.5).
- It spreads over cells within `r = min(14 m, 2 m + 0.9 m x sqrt(pieces))`, weighted
  `exp(-d^2 / (0.35 r^2))`, normalised, and added as height (`volume x weight / cell area`).
- **Repose**: up to 12 passes over the touched cells plus a 2 m border. Where a cell stands more than
  `repose_slope x spacing` above a neighbour (0.75, about 37 degrees), a quarter of the excess moves
  across. It stops when nothing moves.
- **Pool overflow**: a piece that can't get a slot goes straight into the heap at its position.

## 2. One layer with craters

- Rubble adds positive deltas to the `terrain-deformation` tiles (64 m, 0.5 m spacing). Craters add
  negative ones.
- The clamp (`max_dig_m` below the original ground) is unchanged, and rubble may stand up to
  `max_rubble_m` (data, 18 m) above it.
- The tile remesh, `height_at`, the pilot's feet, the mech's gait and the Avian ground colliders all
  read the one layer (section 6.1: one implementation).

## 3. Looks

- **Material**: `terrain-materials` gains a `rubble` layer weighted by rubble depth (full over
  0.3 m), and the brick or concrete share follows the building it came from.
- **Instanced dressing**: about 1 mesh per 2 m^2 of heap over 0.3 m, picked from 12 kit pieces by the
  cell hash, sunk 30 % into the surface. One draw per piece type.
- **Dust**: a sprite cloud sized by the island, growing over 6 s. Under 0.3 ms of overdraw at the
  target (measured).

## 4. Cost

From the mockup (a browser; not a game measurement):

- The warehouse collapse made 645-719 m3 of heap.
- About 5 000 resting pieces in one instanced draw.
- One heap mesh of 22 k vertices updated every 0.2 s.

Expected in the game:

- A deposit touches at most 4 tiles. The repose pass is under 0.2 ms, plus the existing async tile
  remesh.
- Memory comes from the deformation tiles already budgeted (66 kB a tile).
- Pieces: 9 000 instances at 64 bytes is 0.6 MB.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W13 | How long does rubble last? | all mission: piles you can climb; loose pieces past the budget settle into the pile | provisional (survey W13) |
