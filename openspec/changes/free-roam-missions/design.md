# Design: free-roam-missions

## 1. What the player does

1. In free roam, press M (bindable, `mail`). The mail opens, as in a career: the game pauses and the mouse is free.
2. The inbox lists one message per mission, newest first: who sent it, the subject, a line on what it is, and a
   button: **Start**, or for the mission running now **Stop**.
3. Start: the mail closes and the mission begins where its message says (the defence on the start city, the Stratostacean
   over the city nearest the pilot). A message line says what was turned off, if anything.
4. Starting another mission turns the running one off first. Stop turns it off and starts nothing.
5. A won or lost mission stays on its readout (WON, LOST) until another starts or it is stopped.

## 2. The rule (`cb-core::missions`, pure)

```text
MissionKind: Defence | Fortress                    // one per mission plugin
FreeRoam { active: Option<MissionKind>, inbox: Vec<Message> }   // Message is mission-jobs' own
start(kind) -> Switch { stopped: Option<MissionKind>, started: MissionKind }
stop()      -> Option<MissionKind>
```

- `start` always stops the running mission first, the same mission included (a restart).
- The inbox is written once from `data/missions/mail.ron` (sender, subject, body, mission), validated: every mission
  has exactly one message.
- A career's `Board::accept` follows the same rule: the job under way becomes `Abandoned` (a new state) and
  `Event::Abandoned(id)` tells the game to take its enemies away; the ledger records it as not won, paid 0.

Tests: `starting_a_mission_turns_off_the_one_running`, `starting_the_running_mission_restarts_it`,
`stop_leaves_no_mission_running`, `every_mission_has_one_message`, `accepting_a_job_abandons_the_one_under_way`.

## 3. In the game (`cb-app::missions`)

- **The switch** is one message, `MissionSwitch { stop: Option<MissionKind>, start: Option<MissionKind> }`, written by
  the mail page (and by `--mission` at startup). Each mission plugin reads it and acts for its own kind only: the
  defence plugin and the fortress plugin each know how to begin and end themselves, so a new mission is one plugin
  and one line of data, never a `match` spread across files (CLAUDE.md 6.2). Stops run before starts, in the same
  frame.
- **Ending the defence:** `Defending` and `DefenceRun` removed (its systems stop, they run only while they exist),
  its raiders despawned, its waves' vehicles taken out of `Units`.
- **Ending the fortress:** the fortress, its falling legs and its fighters despawned; its shots in flight fly on.
- **The mail page** is the menu's MAIL page (`game-menus`, `mission-jobs`): a career shows its jobs, free roam its
  missions. M opens it in both.
- **The readout** (the MISSION MFD page and the HUD's mission line) shows the running mission's own readout, or
  `FREE ROAM: press M for mail`.

## 4. Cost

A few hundred bytes of mail; one message a switch. Starting a mission costs what that mission costs.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| FR1 | A stopped mission's enemies | They leave the world at once; what they destroyed stays | provisional (survey FR1) |
| FR2 | When free roam's mission mail arrives | All of it in the inbox from the start | provisional (survey FR2) |
