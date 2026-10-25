# Tasks: cockpit-parity

## 1. Layouts
- [x] 1.1 Builder keys `dash_y_m` and `mfd_top_m` (defaults today's); the Cab `mfd=("top", "dash")`, rows (0.30, 0.94), (0.97, 1.93), `dash_y_m` 0.79, the ramp carries the dash; the Canopy `mfd=("top", "dash")`, `mfd_top_m` (0.15, 0.22)
- [x] 1.2 The block-out page's Cab and Canopy rebuilt from the same layouts

## 2. Interiors
- [x] 2.1 Rebuild and bake the Cab and the Canopy at 4096 px, 16 occlusion samples; manifests updated; triangle budget held
- [x] 2.2 Look at each from the eye (headless still) before it is done: no rail across the front screen, the MFDs clear of the screen and the bubble

## 3. Checks
- [x] 3.1 `cb-tools validate` opens every named interior and refuses a missing screen, socket, lamp or ridden part
- [x] 3.2 The MFD fallback strip only for a mech with no interior; its comment in `hud.rs` corrected
- [x] 3.3 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 3.4 Playtest on the owner's machine: sit in the Ironhorse, the Mule and the Harrow; read all six screens; press each
