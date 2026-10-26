# Tasks: civilians

- [x] 1.1 People tiles round the pilot, routes on the pavements, the fixed step; tested
  (`cb-core::civilians`, homes peopled within 400 m and let go past 600 m, busier downtown; crossings, benches,
  standing about)
- [x] 1.2 Danger and fleeing; greeting and lines from data; tested (rounds, close-combat blows, blasts, mech feet
  and skate wheels; `data/people/lines.ron`; Use greets in the game and shows the answer)
- [x] 1.3 The toll: deaths from the pilot's own rounds, blasts and feet counted against the nearest base and sent as
  `CivilianLoss` for the career's standing (survey W34); `Explosion` carries whose blast it was; tested
- [x] 1.5 The toll charged to the career's standing by the one rule for every civilian, `cb_core::civil_standing`
  (people, cars with a driver in them and civil aircraft; each victim once; `standing_per_civilian` in
  `data/career/career.ron`); tested per source
- [x] 1.4 Running inside: `StreetDoors`, an implementation of `Doors` over every building's real street door
  (`world::interior::shell`, the rule the leaves hang by), in place of `NoDoors`; car drivers who give up their
  car run for the same doors; tested (`people_run_in_through_the_real_street_doors_of_the_island`)
- [ ] 2.1 The humanoid table and rig built (`blender-humanoid-characters`), walk and idle clips. Open: the pipeline's
  packs are not on the development machine, so the game draws labelled stand-in box figures at true size
  (`docs/mockups/civilians.html`, awaiting the owner's approval, survey W43)
- [ ] 2.2 Near, mid and far drawing; measured on the 3070. Built with the stand-ins (a pooled body near, one merged
  mesh past 60 m, a card past 250 m); CPU costs measured on the development Mac only, not on the 3070
- [ ] 3.1 Playtest: walk a street, greet three people, fire a gun and watch them run
