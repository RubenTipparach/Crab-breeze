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
| DELIVERY: HOME BASE, BAY 3     |
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
pub enum Refusal { Short(i64), NoFreeBay }
pub fn buy(career: &mut Career, catalogue: &Catalogue, bases: &Bases, design: &str, at: Option<BaseId>)
    -> Result<Delivery, Refusal>
```

1. Find the bay: the current base's lowest free bay if the pilot is at a base (`at`), else the lowest free bay of
   the nearest base (by straight-line distance from the mech, ties by base id) that has one.
   No bay anywhere: `NoFreeBay`, nothing charged.
2. Charge the price through `Career::charge(price, "bought <name>")`: short, `Short(missing)`, nothing changed.
3. Add the `OwnedMech` at full structure in that bay; return the delivery (base, bay) for the page and the mail.
4. A delivery to a base the pilot is not at sends a mail (`mission-jobs`): "Your Harrow is in bay 2 at North Field."

Every design is listed from the start (J14). Selling is not offered.

Tests (sentences): a purchase short of money changes nothing; a purchase with no free bay anywhere changes nothing
and is refused before the charge; at a base, the mech goes to that base's lowest free bay; away from the bases, it
goes to the nearest base with a free bay; the ledger records the price.

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

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J15 | Prices, as in section 3 | As listed: cheaper for the plain machines, the Harrow dearest | provisional (survey J15) |
| J16 | Buying away from a base | Allowed: delivered to the nearest base with a free bay, waiting on arrival | provisional (survey J16) |
