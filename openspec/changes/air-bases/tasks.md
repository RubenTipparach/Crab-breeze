# Tasks: air-bases



- [x] 1.1 Base placement in the world plan, flattened, joined to a highway; tested for the slope and distance rules
  (2026-10-25: five bases on seed 42, clear of cities by 600 m, of every highway's grading and of the drainage
  courses; trees and grass kept off them by the shared cover rule)

- [ ] 1.2 `cb-core::bases`: arrival, instant repair (order, per-part charges, stop when short), catalogue purchase, tested
  (2026-10-25: the repair order and rearm in `cb_core::bases::service`, free for now so no charges; arrival is
  stopping in a bay; the purchase is `cb_core::shop::buy`)

- [ ] 1.3 `data/bases/bases.ron` (layout sizes, repair share), `price` in each mech's data, validated

- [ ] 2.1 `tools/blender/build_airbase.py`: block-out first (owner rule), then detail, PBR, LODs and manifest

- [ ] 2.2 Spawn bases and parked mechs by distance; doors (2026-10-25: a block-out is drawn in the game: the
  apron, three open hangar bays, landing pads, a control tower; no parked mechs or doors yet; `--start base`)

- [ ] 2.3 Repair bay: gantries, sparks and steam, the MECH MFD showing the repair and its cost (2026-10-25: a mech
  that stops in any hangar bay is repaired and rearmed at once, free, owner: "just need to enter airbase hangar,
  and I would get repairs and free ammo (free repairs fornow)"; no gantries, effects or MFD readout yet)

- [ ] 2.4 Terminals: Use to point at the screen; MAIL page and the catalogue

- [ ] 2.5 Switching mechs by boarding a parked one

- [ ] 3.1 Measure the hangar full of mechs on the 3070

- [ ] 3.2 Playtest: land, repair, buy a mech with job pay, switch into it, fly out

