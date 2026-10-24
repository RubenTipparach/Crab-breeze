# Tasks: mech-roster

## 1. Assets
- [ ] 1.1 Retire the Stevedore from `build_mechs.py` and the Mechs page
- [ ] 1.2 `JET_L`/`JET_R` on the torso, `LADDER_base` in front of the hatch, `MUZZLE_L` on the Picket and the Sapper
- [ ] 1.3 Rebuild the eight mechs; z-fight check reads 0

## 2. Data
- [ ] 2.1 `tools/mechs/gen_mech_data.py`: measure each glb, write `data/mechs/<id>.ron` and its `boarding.ron`
- [ ] 2.2 `cb-tools validate` checks every mech glb against the contract and its file's eye, hip and height

## 3. Game
- [ ] 3.1 `MechConfig.body` (eye, hit capsules, wreck, blast, smash, cut box), defaults as today; every 12 m constant reads it
- [ ] 3.2 `MechConfig.interior`: spawn the interior, hang it on `cockpit`, move riding parts to their chest joints, drop its lights
- [ ] 3.3 The sealed view: the interior on the cockpit layer, the screen wall and the hull not drawn
- [ ] 3.4 The shipped boarding timelines test reads every mech in `data/mechs/`

## 4. Checks
- [ ] 4.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`
- [ ] 4.2 Playtest on the owner's machine (`./run.sh --mech mechs/foreman.ron`): walk, board, fight, from the seat
