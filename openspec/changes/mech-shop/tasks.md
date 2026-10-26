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

## 3. Checks
- [ ] 3.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 3.2 Playtest on the owner's machine: earn job pay, open SHOP from an MFD, buy a mech in the field, find it in its bay, board it
