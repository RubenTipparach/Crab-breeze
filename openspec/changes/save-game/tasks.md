# Tasks: save-game

- [x] 1.1 `cb-core::save`: `Place`, `FreeRoamSave`, RON round trip, finite checks, tested
- [x] 1.2 `Career.place`, older saves load with none, tested
- [x] 2.1 Saving every 30 s, on the pause menu and on quit, players' runs only, never mid-boarding
- [x] 2.2 Restoring on Continue: mech position, heading, damage and cuts, missiles; pilot seated or on foot
- [ ] 3.1 Playtest: play free roam, walk off, quit, Continue, find the pilot where they were. 2026-10-25 desktop: the
  30 s autosave wrote `saves/free_roam.ron`; with the save's pilot moved 50 m past the mech and turned round,
  Continue put the pilot there facing the mech (screenshot). Still to see: a walk by hand, the seated restore, a
  career's Continue.
