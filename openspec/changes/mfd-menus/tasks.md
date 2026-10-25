# Tasks: mfd-menus

## 1. The outline
- [x] 1.1 One function for the pointed screen (outline and press); an amber frame per screen, built from its bounds, shown only while pointed at in free look

## 2. Menus
- [ ] 2.1 `mfd::point_on`: the page pixel under the ray; tested on a quad
- [ ] 2.2 The MFD camera as a UI picking pointer; MENU page; a screen's page chosen from its menu; each screen's home page; BACK and Escape restore it (tested: menu, app, sheet, BACK)
- [ ] 2.3 MAP page: the plan drawn once; mech, job and enemy markers; three zooms
- [ ] 2.4 MENU row opens the pause menu (`game-menus`); MAIL (`mission-jobs`) and SHOP (`mech-shop`) as pages

## 3. Checks
- [x] 3.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 3.2 Playtest on the owner's machine: point at each screen in each cockpit, open each app, step back
