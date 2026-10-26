# Tasks: mfd-menus

## 1. The outline
- [x] 1.1 One function for the pointed screen (outline and press); an amber frame per screen, built from its bounds, shown only while pointed at in free look

## 2. Menus
- [x] 2.1 `mfd::ray_uv`: the page point under the ray; tested on a quad
- [x] 2.2 MENU page; a screen's page chosen from its menu; each screen's home page; BACK restores it (one table of rows, `mfd_apps::items`, lays out and is pressed; the stills script takes a job by it)
- [x] 2.2b Escape while pointing at a MENU or MAIL does what BACK does, and the pause menu stays shut; on a readout or with nothing pointed at, Escape pauses as before (`mfd::turn_page`, tested)
- [ ] 2.3 MAP page: the plan drawn once; mech, job and enemy markers; three zooms
- [x] 2.4a MAIL (`mission-jobs`) as a page: credits, the job under way and its distance, three offers with ACCEPT
- [ ] 2.4b MENU row opens the pause menu (`game-menus`); SHOP (`mech-shop`) as a page
- [x] 2.4c With `free-roam-missions` on main (PR #33): free roam's MAIL page lists `missions::FreeRoamRun`'s missions with START/STOP (its `start`/`stop`, never the switch message by hand); a career's ACCEPT takes `Board::accept`'s events into `CareerRun.pending` as `menu::press` does (agreed with the fortress session, 2026-10-25)
- [x] 2.5 Survey C14 (owner, 2026-10-26: "mfds return to default as soon as you click back on them, leaving mfd
  focus"): leaving MFD focus puts every screen back on its default page and forgets its menu; nothing kept between
  sorties or saved (`cb_core::mfd_focus`, tested; `mfd::leave_focus`)

## 3. Checks
- [x] 3.1 `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 3.2 Playtest on the owner's machine: point at each screen in each cockpit, open each app, step back
