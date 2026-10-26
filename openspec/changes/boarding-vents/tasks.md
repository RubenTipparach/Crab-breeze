# Tasks: boarding-vents

## 1. Core
- [x] 1.1 `Vent.dir`, `Vent.hold`, `vent_emits`; `sealed_sitting_s` from the first stage that moves; tested
- [x] 1.2 `ParticleKind::Mist`; `vent_steam` and `vent_mist` in `RECIPES` and `data/fx/fx.ron`
- [x] 1.3 The Breeze's two mist vents hold

## 2. Data
- [x] 2.1 `gen_mech_data.py` writes the five vent stages from the glb's and the interior's empties; every roster `boarding.ron` regenerated
- [x] 2.2 The shipped timelines test checks every roster mech vents and that every vent node exists in its models

## 3. Game
- [x] 3.1 `cb-app::vents`: bind each vent node under the piloted mech once rig and interior are bound; switch its `FxStream` from `vent_emits`
- [x] 3.2 `FxStream.density`
- [ ] 3.3 Headless stills of a boarding: the seals' steam, the mist over the sill (taken; the vents switch on the boarding clock as designed, by the log, but their particles did not show on lavapipe: `docs/playtests/2026-10-25-roster-cloud.md`; seen on the owner's machine in 4.2)

## 3b. Survey M12
- [x] 3b.1 Only the mech you board vents (owner, 2026-10-26, "Approve all"): checked already true on living-island:
  `vents::bind` binds under the `Piloted` mech only and `vents::switch` turns a stream on only under it (`mine`)

## 4. Checks
- [x] 4.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 4.2 Playtest on the owner's machine: board and leave each mech, watch the steam and mist from the ground and the seat
