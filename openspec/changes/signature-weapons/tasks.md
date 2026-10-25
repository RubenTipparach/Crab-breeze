# Tasks: signature-weapons

Built after `weapon-kinds`, `multi-lock` and `railgun-scope`.

## 1. Core
- [ ] 1.1 `WeaponSpec.burst` and `lob`, in `weapons::launch`; tested (a burst's timing, a lob landing within 1 m at 300, 700 and 1 500 m)
- [ ] 1.2 `MechConfig.weapons` from `data/mechs/<id>/weapons.ron`; the generator writes the reference; validation refuses a socket the glb lacks

## 2. Data and models
- [ ] 2.1 The eight `weapons.ron` loadouts (design section 1 and 2)
- [ ] 2.2 `BACK_R` on the Sapper's glb (`mech_kit.py`), rebuilt; contract check

## 3. Cockpit and effects
- [ ] 3.1 WEAPONS page: the starred signature station and its state line
- [ ] 3.2 Recipes `muzzle_boom`, `beam_lance`, `muzzle_steam`, `burst_steam`, `muzzle_mortar`; the mortar's time of flight on TARGET

## 4. Checks
- [ ] 4.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 4.2 Playtest on the owner's machine: each mech's signature in the city defence, one take each
