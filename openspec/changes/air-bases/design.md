# Design: air-bases

## 1. Placement (`cb-core::world`, the world plan)

The plan places `base_count` (5, survey J5) bases after the cities and highways, spread so no two
are within 6 km of each other. A base site needs:
- ground with a slope under 2 degrees over a 700 x 500 m footprint;
- a highway within 3 km, joined to the base by a service road;
- at least 2 km to any city.

Base 0 is the one nearest the island's start point. Placement is seeded and stable, like the cities.
The footprint is flattened the way a city's square is (`ground_m`).

## 2. Layout (one base, metres)

| Piece | Size | Notes |
| --- | --- | --- |
| Apron | 500 x 200 | concrete, 4 landing pads 60 m across, lit rings |
| Hangar bays | 6 bays, each 44 wide x 40 deep, doors 34 tall | a 24 m mech stands inside with the doors shut |
| Repair bay | 50 x 50 x 40 tall | two gantries on rails, arms with welders |
| Control tower | 12 x 12 x 30 | beacon lit |
| Ready room | 20 x 14 x 5 | the mail terminal and the catalogue terminal |
| Fence | perimeter | a gate on the service road |

## 3. Models (CLAUDE.md section 10)

`tools/blender/build_airbase.py` builds the base the CSG way, low-poly block-out first (owner rule),
then detail:
- concrete slabs and corrugated hangar shells;
- roll-up doors as named nodes the game animates (`DOOR_bay_N`);
- gantries as named nodes (`GANTRY_A`, `GANTRY_B`, `ARM_*`);
- terminals as named, hit-testable nodes (`TERM_mail`, `TERM_catalogue`).

Materials are Material Maker graphs (concrete, corrugated steel, hazard paint, apron markings), each a
full PBR set.

| LOD | Range | Triangles a base |
| --- | --- | --- |
| 0 | within 300 m | about 150 000 |
| 1 | to 2 km | 15 000 |
| 2 | beyond 2 km | 1 500 |

Bays are walkable on foot: floors, and stairs to the ready room.

## 4. Rules (`cb-core::bases`, pure)

- **Arrival:** `Arrived(base)` fires when the piloted mech stops on a pad or inside the fence, having
  been airborne or more than 2 km away. The career's current base becomes that base and the save is
  written.
- **Repair** (`Repair::run`, instant: survey J11): when the mech stops in the repair bay, its parts
  are repaired at once, in this order: torso, left leg, right leg, left arm, right arm, head.
  - Cost per part: `price x repair.full_share (0.3) x missing / total structure`, charged as each
    part is repaired.
  - A charge the career refuses stops the repair there, leaving the rest damaged. A lost part is
    refitted at full structure.
- **Buying** (`Catalogue::buy`): charges the design's price, then adds an `OwnedMech` at full
  structure in the lowest free bay of the current base. With no free bay, the purchase is refused
  before anything is charged. Prices are in each mech's data (`price`, provisional on J4).
- **No unlocks** (survey J14): the catalogue shows and sells every design from the start; the price
  is the only limit.
- **Parking:** an owned mech that is not being piloted is parked in its bay:
  - it does not step;
  - it is drawn sitting (the boarding pose);
  - its hatch opens when you walk up to it, as today.

## 5. In the game (`cb-app::bases`)

- **Spawning:** a base's model spawns within 6 km (LOD 2), and its parked mechs within 1.5 km.
- **Parked mechs** spawn as ordinary mechs with no AI and their saved damage. Boarding one makes it
  the piloted mech (`Piloted` moves to it) and changes the career's `piloting`. Boarding has one code
  path (pilot-boarding), with no copy.
- **Repair effects:** the gantries swing in and back out over about 2 s with a burst of weld sparks
  and steam (fx recipes); the repair itself is already done, and the MECH MFD shows the new state and
  what it cost.
- **Terminals:** standing on foot within 1.5 m of a `TERM_*` node and looking at it, the pilot presses
  Use.
  - The terminal's screen shows its page: MAIL from mission-jobs, or the catalogue here.
  - While look is paused, the mouse points on that screen.
  - Escape, or Use again, steps back.
  - The view never moves: the screen is already in front of you.
- **Doors** open when a mech or the pilot comes within 30 m of a bay, and close behind.

## 6. Cost

| Item | Cost |
| --- | --- |
| One base at LOD 0 | about 150 000 triangles, about 60 draw calls, 4 PBR material sets (about 100 MB at 2048 px) |
| Parked mechs | up to 6 times the mech's own cost when you stand in the hangar; drawn at LOD from 300 m |
| Rules | microseconds a step |

Before this is called done, the hangar is measured with all six bays full (perf-measure).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J5 | How many bases | 5 | decided (survey J5) |
| J11 | How repairs take time | Instant | decided (survey J11) |
| J13 | How you use the terminals | Walk up, press Use, the mouse points on the screen in front of you | decided (survey J13) |
| J4 | Prices | Mechs 300 000-1 200 000, full repair up to 30 % of the price | decided (survey J4) |
| J14 | Do won jobs unlock mechs, or only money | Money only: every mech is for sale from the start | decided (survey J14) |
