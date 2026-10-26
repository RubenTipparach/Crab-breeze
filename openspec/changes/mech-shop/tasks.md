# Tasks: mech-shop

Built with `career` and `air-bases`, after `mfd-menus`.

## 1. Core
- [x] 1.1 `cb-core::shop::buy` on the career's ledger and the bases' bays; tested (short, no bay, at a base, in the field, the ledger)
- [x] 1.2 `data/career/catalogue.ron` with validation (designs exist, prices within J4) (2026-10-25: `CareerData::load`,
  run by `cb-tools validate`, also loads every listed design's mech data)

## 2. Page
- [ ] 2.1 SHOP list and sheet on the MFD texture; numbers from the mech data; BUY and its refusals (2026-10-25: the
  list is built, on the terminal's screen: name, mass, walk speed, torso armour and guns from each mech's data, OWNED,
  BUY with the price, and BOUGHT or SHORT n / NO FREE BAY after; no sheet, and not yet on the cockpit MFDs)
- [ ] 2.2 The turning wireframe at scale, only while a sheet is shown
- [x] 2.3 The base's catalogue terminal shows the same page (`air-bases`) (2026-10-25: the bay terminal's SHOP tab)

## 4. The air drop (owner, 2026-10-26: "when a player orders a mech it should be air dropped close to wherever the player is on a good are on the ground (not stuck on a building)"; supersedes J16 and `airfields` section 3)
- [ ] 4.1 `cb_core::air_drop::drop_site` with `data/career/air_drop.ron`; tested (never in a building in a dense
  block, never in deep water, never on a steep slope, inside the ring when it can be, the fallback, the same seed)
- [ ] 4.2 The drop on the career's clock (`Career::drops`, `OwnedMech::field`), its fall and the pilot kept clear;
  `shop::buy` drops, the bay only as the fallback; tested
- [ ] 4.3 The game: the aircraft and canopies (block-out stand-ins), the fall and the landing through the mech's own
  landing, the marker and the warning, the dropped mech boardable by walking up to it, Continue keeping it
- [ ] 4.4 The shop's text: AIR DROP NEAR YOU; the bay only as the fallback
- [ ] 4.5 `career-loop` playtest beat 5 waits for the drop, walks up to the mech and boards it
- [ ] 4.6 Survey J16, answered ("a marker appears on your hud to show mech delivery. you can swap mechs out at base, with foreman always availible"): the HUD marker until the mech is boarded; the terminal's HANGAR tab and
  `Career::call_out`, the Foreman always available; tested in cb-core
- [ ] 4.7 Playtest on the owner's machine (only when the owner says so)

## 3. Checks
- [ ] 3.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 3.2 Playtest on the owner's machine: earn job pay, open SHOP from an MFD, buy a mech in the field, watch it dropped near you, board it
