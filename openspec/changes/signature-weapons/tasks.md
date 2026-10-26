# Tasks: signature-weapons

Built 2026-10-26 after `weapon-kinds` and `multi-lock` (owner, "Approve all", then "Go and implement"; survey G20 and
G21 decided). Tested in `cb-core`; not yet playtested.

## 1. Core
- [x] 1.1 `WeaponSpec.burst` and `lob`, in `weapons`; tested (`a_burst_fires_three_rounds_and_reloads_from_the_last`, `losing_the_arm_mid_burst_ends_it`, `a_lobbed_grenade_lands_on_the_aim_point` at 100, 300 and 600 m, `a_lob_beyond_its_range_falls_short_at_its_longest`)
- [x] 1.2 Loadouts from `data/mechs/<id>/weapons.ron`, read beside the mech file by `MechConfig::load` (not a reference written by the generator, see design "Built"); validation refuses a socket the glb lacks

## 2. Data and models
- [x] 2.1 The eight `weapons.ron` loadouts (design section 1 and 2; the Sapper's grenade launcher per G21) (test: `every_roster_mech_carries_its_own_signature_weapon`)
- [x] 2.2 The Sapper's back unit: fires from the `CANNON_R` its glb already has, so no `BACK_R` and no rebuild

## 3. Cockpit and effects
- [ ] 3.1 WEAPONS page: the starred signature station and its state line (built); the TARGET page's time of flight (not yet)
- [ ] 3.2 Recipes `muzzle_boom`, `beam_lance`, `muzzle_steam`, `burst_steam`, `muzzle_mortar` (not yet: the existing muzzle recipes play)

## 4. Checks
- [x] 4.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 4.2 Playtest on the owner's machine: each mech's signature in the city defence, one take each
