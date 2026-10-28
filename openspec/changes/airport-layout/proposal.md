# Airport layout: bases that read as airfields

## Why

Owner, 2026-10-28: "We should also improve airports. They don't look much like airports", with four references
(`docs/design/references/airports/`): an RAF airfield labelled part by part, the 1952 master plan of Nellis Air Force
Base, and two civil airport layout plans.

Each of the five air bases (`air-bases`) is drawn today as one concrete slab with three mech hangars, three yellow
pad outlines and a box for a control tower, and beside it a runway (survey W45) joined by one short taxiway. What the
references share, and our bases lack, falls into three groups:

1. **An airport is laid out round its runway.** A parallel taxiway runs its whole length, joined to it by links at
   the ends and in the middle, with a holding position painted on every link. The apron opens off the taxiway.
2. **A runway is marked to be read from the air.** Its designators (the heading in tens of degrees), threshold bars,
   touchdown zone and aiming point marks, a centre line and edge lines. Yellow chevrons on the blast pads beyond the
   thresholds. Approach lights out past each end, and PAPI lights beside the aiming point.
3. **The site is buildings with jobs.** A terminal with boarding bridges and aircraft on its stands, a control tower
   with a glazed cab, a fire station, a fuel farm, a car park, a helicopter stand, a windsock, and a fence round it
   all. The mech hangars stay: they are what the base is for in this game.

## What changes

- **`cb_core::world::airfield`**, a pure layout of each base's airfield, from its site and runway: the parallel
  taxiway and its links, holding positions, stands, and every building's footprint and kind. One rule, tested:
  everything inside the airfield, nothing overlapping a hangar, the tower, a pad or another building, every stand
  clear, and the taxiway clear of the runway.
- **The strip between the site and the runway** (`runway_gap_m`, now 100 m so a taxiway fits with clearance) is
  flattened with them and checked dry and clear when the runway is placed.
- **`air_bases.rs` draws the layout:**
  - the runway's full markings, approach lights and PAPI;
  - the taxiways with their yellow centre lines and holding positions;
  - the apron's stand lines;
  - the buildings as block-outs (owner rule: low poly first), and the civil aircraft's stand-ins (`air_traffic`,
    approved 2026-10-26) parked on the stands;
  - the windsock and the fence.
- **Stills:** `CB_STILLS_FX=base` tours the nearest base from four sides and from the ground.

## What does not change

Where the bases are, their hangars, terminals, pads and tower position, and repair and rearm, all stay as
`air-bases` has them. Harbours are a separate change (`harbours`), already built.
