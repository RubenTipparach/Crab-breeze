# Tasks: mission-jobs

- [x] 1.1 `cb-core::mail`: messages, offers, the inbox limit, the template filler, deterministic from the seed
- [x] 1.2 `cb-core::jobs`: JobSpec, the generator from the world plan, the JobRule registry (Defend over city-defence, Intercept, Destroy), tested
- [x] 1.3 `data/jobs/jobs.ron` and `data/jobs/mail.ron`, validated
- [x] 2.1 MAIL MFD page (`mfd_apps`, after the five readouts): offers, the job under way, ACCEPT by press (decline and reading a body by press still to come)
- [x] 2.2 Nav bearing and the site marker on the screens (`hud::draw_job`)
- [x] 2.3 Arrival releases the forces; the outcome pays through the career and writes the save
- [ ] 2.4 Test key: start the accepted job at its site
- [x] 2.5 M focuses the MAIL page; the pause menu's MAIL button
- [x] 2.6 Ejecting when the mech is destroyed; the wreck recovered to the nearest base
- [ ] 3.1 Playtest: read mail, accept, travel, fight, get paid

## Free roam (owner, 2026-10-25)

- [x] 4.1 Free roam spawns no encounter enemies at the start; the email MFD offers missions there as in a career (a board never saved: `CareerStart::FreeRoam`)
- [x] 4.2 An accepted mission's enemies spawn only on reaching its waypoint (1.5 km), 150-350 m round it and never within 1 km of the pilot (`jobs::enemy_spot`, `spawn_ring_m`, `clear_of_pilot_m`; tested)
- [x] 4.3 Scripted takes (`--showcase`, `--bench`, `--stills`) start their fights directly, without the email MFD (`Play::Encounters`)
- [ ] 4.4 Playtest in free roam: board with no one firing, accept on the email MFD, fly to the waypoint, fight
