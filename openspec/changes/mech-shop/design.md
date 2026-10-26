# Design: mech-shop

## 1. The page (an MFD page, 512 x 512 px; `mfd-menus` draws it on any screen it is picked on)

**The list**

```text
+--------------------------------+
| SHOP        CR 1 234 000  BACK |
| ------------------------------ |
| FOREMAN          850 000 OWNED |
| IRONHORSE        700 000       |
| HARROW         1 200 000       |   <- the row under the crosshair, lit amber
| ROUSTABOUT       450 000       |
| DRILLER          520 000       |
| MULE             360 000       |
| PICKET           300 000       |
| SAPPER           600 000       |
| ------------------------------ |
| DELIVERY: AIR DROP NEAR YOU    |
+--------------------------------+
```

**The sheet** (click a row)

```text
+--------------------------------+
| HARROW             LIST  BACK |
| +-----------+  23.8 M  390 T   |
| | turning   |  WALK 43 KM/H    |
| | wireframe |  ARMOUR 160 MM   |
| | at scale  |  COCKPIT CANOPY  |
| +-----------+                  |
| SIGNATURE: TWIN RACKS          |
|   12 MISSILES, 12 LOCKS        |
| GUNS: 35 MM AC, 120 MM CANNON  |
| ------------------------------ |
| [ BUY 1 200 000 ]  AFTER 34 000|
+--------------------------------+
```

LIST returns to the list; BACK, on the list or the sheet, closes the shop and gives the screen back the page it
showed before its menu was opened (`mfd-menus`).

Every number on the sheet is read from the data the game flies the mech by (`data/mechs/<id>.ron`: height, mass,
speed, the parts' armour; its loadout from `signature-weapons`), never typed into the shop (CLAUDE.md 6.1: what the
cockpit shows is the number the simulation uses). Where today's roster shares the stand-in's tuning (the same walk
speed for all), the sheet shows that honestly; tuning the mechs apart is their own work.

**The wireframe.** While a sheet is on a screen, a small camera draws that mech's glb on a render layer of its own
into a 192 x 192 px image on the sheet: unlit phosphor green, edges only, turning at 20 degrees a second, with a 1.8 m
pilot beside it for scale. Off while no sheet is shown.

## 2. The rules (`cb-core::shop`, pure, on `career` and `air-bases`)

```rust
pub struct Listing { pub design: String /* mechs/<id>.ron */, pub price: i64 }
pub enum Refusal { Short(i64), NoFreeBay, NotListed }
pub enum To { Drop(Drop), Bay { base: u32, bay: u32, away: bool } }
pub fn buy(career: &mut Career, catalogue: &Catalogue, bays: &Bays, order: &Order, structure: &[f32], kept: usize)
    -> Result<Delivery, Refusal>
```

1. Where it goes: the air drop's site (section 6, `air_drop::drop_site`, found by the caller and passed in the
   order) when there is one. Only without a site, the bay: the current base's lowest free bay if the pilot is at a
   base, else the lowest free bay of the nearest base (by straight-line distance from the pilot, ties by base id)
   that has one. Neither: `NoFreeBay`, nothing charged.
2. Charge the price through `Career::charge(price, "bought <name>")`: short, `Short(missing)`, nothing changed.
3. Add the `OwnedMech` at full structure: in the field at the site with its drop on the career's clock, or in the
   bay; return the delivery for the page and the mail.
4. A drop sends a mail (`mission-jobs`): "Your Sapper is on its way by air: it lands 90 m from where you ordered
   it." A bay delivery to a base the pilot is not at says where it waits.

Every design is listed from the start (J14). Selling is not offered.

Tests (sentences): a purchase short of money changes nothing; a purchase with no site and no free bay anywhere changes
nothing and is refused before the charge; with a site, it is dropped there even when every bay is full; with no
site at a base, the mech goes to that base's lowest free bay; with no site away from the bases, it goes to the
nearest base with a free bay; the ledger records the price.

## 3. Prices (`data/career/catalogue.ron`, survey J4's range)

| Mech | Price (credits) | Why there |
| --- | --- | --- |
| Picket | 300 000 | a sentry: shield and rail rifle |
| Mule | 360 000 | a hauler with rocket pods |
| Roustabout | 450 000 | a brawler |
| Driller | 520 000 | a mining machine with a cutting lance |
| Sapper | 600 000 | a combat engineer with a siege mortar |
| Ironhorse | 700 000 | heavy, a steam cannon |
| Foreman | 850 000 | the all-rounder you start in |
| Harrow | 1 200 000 | the missile carrier, the Ace Combat mech |

With jobs paying 40 000 to 150 000 (`mission-jobs`), the cheapest takes three or four good jobs, the Harrow about
ten. Validated on load: every listed design exists in `data/mechs/`, every price within J4's range.

## 4. Where it shows

- Any MFD, from its menu (`mfd-menus`).
- The ready room's catalogue terminal at a base (`air-bases`): the same page on the terminal's screen, pressed with
  the same pointing. `air-bases`' catalogue becomes this page; its `Catalogue::buy` is `shop::buy`.

## 5. Cost

One more MFD page (in `mfd-menus`' nine); the wireframe camera, only while a sheet is shown: one mech of about
100 000 triangles into 192 x 192 px, under 0.2 ms on the 3070 by estimate, measured on the owner's machine. The rules
run once a click.

## 6. The air drop (owner, 2026-10-26)

> "when a player orders a mech it should be air dropped close to wherever the player is on a good are on the ground
> (not stuck on a building)"

This replaces survey J16's bay delivery (accepted on its recommendation, 2026-10-25) and `airfields` section 3's
cargo landing. A bay is now only the fallback.

### 6.1 Where it lands (`cb_core::air_drop::drop_site`, pure)

The site is fixed the moment BUY is pressed, from where the player is: the pilot's feet on foot, the mech's feet in
the seat. Every number is in `data/career/air_drop.ron`, with its unit in its name.

| Rule | Default | Why |
| --- | --- | --- |
| The ring round the pilot | 40 to 150 m (`ring_min_m`, `ring_max_m`) | 40 m keeps a 24 m mech's fall and its dust off the pilot; 150 m is under half a minute's walk |
| Searched in rings | every 10 m, spots about 15 m apart on each ring (`ring_step_m`, `spot_spacing_m`) | fine enough to find a plaza between towers |
| Searched out to | 600 m (`search_max_m`); past it, the bay | the farthest that still reads as "near you" |
| Clear of buildings | the mech's own footprint radius (`body_radius_m` in its data) plus 6 m (`building_clear_m`), against every building's footprint, every hangar bay and its walls, every control tower and every rock | never on, in, or wedged against a building |
| Not in deep water | no deeper than the mech's skim depth (the lesser of `max_wade_depth_m` and `skim_depth_share` x `body_height_m`), and any water at all counts 300 m farther (`wet_penalty_m`) | the brief's wade limit, but dry ground always wins when there is any |
| Not on a slope | no steeper than 12 degrees across the footprint (`max_slope_deg`), measured as the rise between opposite points on its rim in four directions | a 24 m machine landing on a hillside tips; 12 degrees is a steep street |
| Clear of the pilot and the mechs | outside the ring's inner edge from the pilot; 10 m beyond the footprints of every other mech (`mech_clear_m`) | lands beside, never on |
| Preferred ground | paving (a road, a city street or plaza, a base's apron: the world's one cover rule, `WorldPlan::surface_at`) counts 30 m nearer (`paved_bonus_m`) | the owner's "a road or a plaza"; open soil is next |

The search: rings outward from `ring_min_m`. Every good spot inside the ring (up to `ring_max_m`) is scored by its
distance from the pilot, less the paving bonus, plus the water penalty, and the lowest score wins (ties by the order
the spots were tried). Only if the ring holds no good spot does the search go on past it, one ring at a time, and the
first ring with a good spot gives the site. Deterministic and seeded: each ring's first spot is at an angle drawn from
the career's seed forked by the mech's key, so two orders in the same place land in different places and the same
order always lands in the same place.

### 6.2 The drop (`cb_core::air_drop::Drop`, on the career's clock)

| Stage | Default | What the player sees |
| --- | --- | --- |
| Ordered | the site is fixed; `AIR DROP INBOUND` and a mail | a marker over the site with its distance |
| Inbound | release 25 s after the order (`delay_s`) | the aircraft appears 20 s before the release (`aircraft_shown_s`) and flies over the site at 320 m (`release_height_m`), 95 m/s (`aircraft_speed_m_s`), on a seeded track |
| Free fall | 2 s (`free_fall_s`), about 20 m | the mech drops out under the aircraft |
| Canopies | 14 m/s (`canopy_m_s`) | three canopies open over it |
| Retros | the last 25 m (`retro_height_m`), slowing to 3 m/s (`touchdown_m_s`) | dust blown out in a ring under it (`jet_wash`) |
| Landed | about 50 s after the order (25 s falling) | it lands on its feet through the mech's own landing: footprints and the landing dust (`ground-marks`), a fall of dust (`fall_dust`); the canopies drop away |

The career keeps the drop (`Career::drops`: the mech's key, the site, the order's time on the career's clock, the
track) and the mech's place in the field (`OwnedMech::field`), so a save mid-drop resumes it and a save after keeps
the mech where it landed.

**The pilot in the site.** While it falls, if the pilot on foot (or the mech they sit in) comes within the mech's
footprint plus 15 m (`pilot_clear_m`) of the site, the marker reads `DROP ZONE: MOVE CLEAR` and the site moves to the
nearest good spot on that circle round the pilot, closest to where it was (`air_drop::clear_of`); the canopies steer
it there at up to 6 m/s (`steer_m_s`). If the pilot is still inside the circle at touchdown, it lands on that spot,
just beside them. It never lands on the pilot.

### 6.3 Boarding it

A landed mech stands, stopped and sealed, as an owned mech in the field. The pilot boards it the way they board any
mech: walking up to it, it turns to them, kneels and lowers its rope (`mech-boarding`, `rope`). The one new rule is
which owned mech that is: the nearest owned mech standing within 40 m of the pilot's feet (`walk_up_m`), ties by key
(`air_drop::walking_up_to`). The game hands the boarding to it only while the pilot is on foot, not waiting on a
hatch, and the mech they were last in has closed its hatch. The career's `piloting` key follows, and the refit
rebuilds the flown mech from that design and its fit.

### 6.4 Drawing it (`cb-app` `air_drop`)

- **The aircraft** is a block-out stand-in, named so in its scene node: a 64 m fuselage, 72 m wing, a T tail and
  four engine pods in plain grey boxes. A modelled aircraft is `airfields`' work.
- **The canopies** are three flattened domes of 14 m radius on rigging lines, block-outs too.
- **The marker** is a diamond over the site with `AIR DROP` and the distance, on screen on foot and in the seat
  (there is no map or minimap yet; when one exists, the site goes on it).
- **The landing** reuses the mech's own landing (the sim lands it; `ground-marks` prints the feet and throws the
  dust), with `fall_dust` and `jet_wash` from `data/fx/fx.ron`.

### 6.5 Survey J16, answered: the marker and swapping at a base

The owner's answer to J16: "a marker appears on your hud to show mech delivery. you can swap mechs out at base, with foreman always availible".

**The marker** (`cb-app` `air_drop::draw_markers`): one per drop in `Career::drops`, from the order until the pilot
sits in that mech (`Career::boarded` ends the drop), drawn over the site 30 m up on the HUD layer, shown on foot and
in the seat: `<> AIR DROP SAPPER 85 M` and `INBOUND 12 S`, `FALLING 140 M`, or `WALK UP TO BOARD`; red with
`DROP ZONE: MOVE CLEAR` while the pilot stands in the zone.

**Swapping at a base** (`Career::call_out`, pure): the terminal's HANGAR tab lists every owned mech, where it is
(HERE, IN THE FIELD 1.2 KM, BASE 2 BAY 1, AIR DROP INBOUND) and CALL OUT for each that may come.

| Rule | Why |
| --- | --- |
| A mech standing on this base's site, or waiting unseen in one of its bays, can be called | "swap mechs out at base" |
| The career's starting mech (`start_mech`, the Foreman) can be called from anywhere, and comes back whole when it was wrecked | "with foreman always availible" |
| The mech in use cannot be called (it is already here) | nothing to swap |
| It comes out standing in the terminal's own bay, or the base's first bay no mech stands in, facing the apron; with none clear, `NO CLEAR BAY` | it stands where the pilot can walk up to it |

The swap itself is the walk up (section 6.3): the pilot walks to the called mech and boards it, and the mech they
leave stays parked where they left it, kept by the career. Tests: a mech parked here comes and one parked elsewhere
does not, the mech in use is refused, the Foreman comes from far away and whole after a wreck.

### 6.6 Cost

The search runs once a purchase: at most 60 rings of at most 250 spots, each a building grid lookup, a water query,
five height samples and the rocks; well under 50 ms by estimate, once. The drop draws one block-out aircraft (about
100 triangles) and three canopies while it flies: nothing measurable. One more mech stands in the world afterwards,
as any parked mech does (`mech-roster`'s per-mech cost).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J15 | Prices, as in section 3 | As listed: cheaper for the plain machines, the Harrow dearest | provisional (survey J15) |
| J16 | Buying away from a base | Superseded by the owner, 2026-10-26: air dropped near the pilot anywhere; a bay only as the fallback. The owner's J16 answer: "a marker appears on your hud to show mech delivery. you can swap mechs out at base, with foreman always availible" (section 6.5) | answered |
| D1 | How far from the pilot it lands | 40 to 150 m | provisional (survey D1) |
| D2 | Canopies, retro thrusters, or both | Canopies, with a retro burst in the last 25 m | provisional (survey D2) |
| D3 | How long from BUY to touchdown | About 50 s (25 s to the release, 25 s falling) | provisional (survey D3) |
| D4 | Water at the site | Dry ground always wins; water up to the mech's skim depth only when the ring has nothing dry | provisional (survey D4) |
| D5 | No good ground within 600 m | Fall back to a hangar bay, as before | provisional (survey D5) |
| D6 | The pilot walks into the site | The canopies steer it aside and the marker warns; it lands beside them if they stay | provisional (survey D6) |
| D7 | Which owned mech the pilot boards | Whichever owned mech they walk up to, within 40 m | provisional (survey D7) |
| D8 | The aircraft | A block-out stand-in until `airfields` models one | provisional (survey D8) |
