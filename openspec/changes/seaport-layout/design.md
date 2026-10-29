# Design: seaport-layout

## 0. What is there today

`harbours` (built) places a harbour by each coastal city, at most four: a 40 m apron along the shore, a 30 m pier on
piles out to 10 m of water, a 220 m head with three luffing cranes, three 50 x 24 x 12 m warehouses and a 16 x 12 x 9 m
harbour office whose terminal opens the jobs board (`data/world/ships.ron`, `cb_core::ships::harbour`). Nothing is dug
or flattened (HB3). Ships moor at the head, ride at anchor in the roads and fish off the harbour.

## 1. The port (the mockup, `docs/mockups/seaport.html`)

Metres; the shore along z = 0, land to -z, sea to +z; still water y = 0, decks at 2.5 (the harbours' `deck_m`), the
seabed at -14 inside the breakwaters.

| Part | Size | Why it is there |
|---|---|---|
| Breakwaters | 2 rubble mounds, 2.1 km each, 50 m at the seabed, 12 m crest 6 m up, crown wall seaward | calm water for every berth |
| Entrance | 190 m between the heads, red light west and green east (IALA A, entering northward) | the one way in |
| Channel | red and green buoys every 300 m out to 1.7 km, leading lights 18 m and 32 m high ashore | ships line up from the anchorage |
| Container terminal | 480 m quay, 160 m deep, 3 ship-to-shore gantries (30 m rail gauge, 42 m girder, 70 m A-frame, 55 m outreach), 15 stack blocks of 6 x 6 stacks 1-4 high, straddle carriers | the box trade, the most valuable target |
| General cargo pier | 300 x 80 m finger pier, a 240 m transit shed, 4 luffing cranes (today's model), rail along both faces | break-bulk, a freighter each side |
| Ferry terminal | 160 x 120 m quay, a 14 m linkspan to the stern door, terminal, 6 marshalling lanes | the ferry, stern-in |
| Port control | harbour office with the jobs terminal (as today), a 38 m control tower with a glazed cab, pilot station, 2 tugs | who runs the port, and where the pilot gets work |
| Tanker jetty | 420 m trestle on piles, a 120 x 20 m head with 4 loading arms, pipe rack; the 150 m tanker alongside in 14 m | oil kept away from the rest |
| Tank farm | 6 tanks 44 m across and 18 m high in bunds | what the jetty fills |
| Dry dock | 200 x 40 m, floor at -11, caisson gate, keel blocks, a goliath crane 60 m over it; a freighter on the blocks | the shipyard |
| Fishing harbour | a mole round an inner basin, 3 floating pontoons with fingers, trawlers and launches, fish market, slipway | the small boats' own basin |
| Coast guard | 100 x 100 m quay, boathouse, gunboats, and a 60 m wide mech slipway at 1 in 8 from the deck to 6 m under | a mech walks in and skims out (survey M4) |
| Landside | port road, main line and sidings into the terminal, warehouses, customs, car park | the land the port serves |

The ships are the harbours change's own models (`assets/models/boats`) at true size: the freighter stands in for the
container ship. The tugs are block-out stand-ins until a model exists.

## 2. Every building breaks

The airport's rule (`airport-layout` section 6), through the same code: `tools/mockups/site_kit.js` holds the shapes and
the breakable buildings for both pages, with each building's own ground level (`building(name, fn, groundY)`), so a shed
on a 2.5 m quay stands on its quay. Every piece is held as built; none needs anchoring (the gantry's cab hangs on a hanger
held beside the boom, the goliath's crab rides on top of its girder). Destroying the legs on both rails of a gantry drops
its girder and boom (measured: 41 pieces fall); destroying the control tower's shaft drops its cab (28 pieces).

## 3. In the game (after approval)

`cb_core::ships::port::layout(harbour, cfg) -> Port` lays this out from a harbour's shore point and out direction, the
sizes in `data/world/ships.ron` under a new `port:` section, checked on load; `cb-app` draws it and gives each building
and crane a breakable kit set. The harbour traffic moors at its own berths (task 1.2). The ground and the seabed inside
the port's footprint are shaped to it (SP2): quays and breakwaters stand on the seabed, the dry dock is cut into the land.

## 4. Cost

Per full port, block-out geometry: about 4 900 breakable pieces (the container stacks 2 730 of them), drawn as one
instanced draw per material per building, about 140 draws; the breakwaters, quays and paint about 60 draws; the ships
their models' LODs. Support links are computed once at load, per building (the largest, a stack block, 182 pieces). Not
measured: the cloud session has no GPU.

## Open questions

In the survey, section SP (`docs/design/survey.html`). Built on the recommendations while open (provisional).

| Id | Question | Provisional |
|---|---|---|
| SP1 | Every harbour a full port? | The full port at the largest coastal city; the others a small port (one breakwater, the cargo pier, fishing harbour and coast guard slipway) |
| SP2 | May the port shape the ground and seabed (HB3 says nothing is dug or flattened)? | Yes, inside its own footprint |
| SP3 | Ships in port | The harbour traffic moors at these berths, and a few ships always lie alongside |
