# Tasks: free-roam-missions

- [x] 1.1 `cb-core::missions`: `FreeRoam`, `start`, `stop`, the mail from `data/missions/mail.ron` (tests in design section 2)
- [x] 1.2 `Board::accept` abandons the job under way (`JobState::Abandoned`, `Event::Abandoned`; test `accepting_a_job_abandons_the_one_under_way`)
- [x] 2.1 `cb-app::missions`: `MissionSwitch`; the defence begins and ends on it; `--mission fortress`
- [x] 2.2 M opens the mail in free roam; Start and Stop on the mail page; the readout when none runs
- [x] 2.3 A career takes an abandoned job's enemies away
- [x] 2.4 A stopped mission's enemies leave at once, what they broke stays broken (owner, 2026-10-26, survey FR1): the raiders, the fortress and its deck guard and vehicles were already taken on the switch; their rounds in the air now go with them (test: `a_stopped_missions_vehicles_and_their_rounds_leave_at_once_and_only_theirs`)
- [ ] 3.1 Playtest on the owner's machine: free roam, M, start the Stratostacean, start the defence (the Stratostacean goes), stop
