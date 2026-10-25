# Design: mission-jobs

## 1. Mail (`cb-core::mail`, pure)

```text
Message { id: u32, from: String, subject: String, body: String, day_s: f64, read: bool,
          offer: Option<Offer> }
Offer { job: JobSpec, expires_s: f64, state: Open | Accepted | Declined | Expired | Done | Failed }
```

Messages are written from templates in `data/jobs/mail.ron` (senders, subject and body lines with
`{city}`, `{pay}`, `{enemy}` slots) by the job generator. The generator draws from a `SimRng` forked
from the career's seed by message count, so the same career gets the same mail (deterministic).

The inbox keeps the newest 40 messages. Three offers are open at a time; when one closes, the next
arrives after 30-90 s of play (data).

## 2. Jobs (`cb-core::jobs`, pure)

```text
JobSpec { kind: JobKind, site: Vec2, radius_m: f32, pay: i64, time_limit_s: f32,
          forces: ForcesKey /* invasion-forces */, protect: Option<Protect> }
JobKind: Defend { city: u32, waves: u8 } | Intercept | Destroy { target: TargetKind }
```

- **Generation:** a kind is chosen by weight from `data/jobs/jobs.ron`; its site comes from the world
  plan, always in or within 3 km of a city (survey J2): the city itself for Defend, open ground on its
  edge for Intercept, a valley or shore near it for Destroy, its forces from the kind's force table, and its pay from the forces' strength times a
  per-kind rate (40 000-150 000 credits, provisional on J4).
- **States:** Offered, Accepted (travelling), Active (the fight: forces arrive when the pilot is within
  `radius_m`, default 1 500 m), Won, Failed.
- **Win and loss** come from the kind's rule. Defend reuses `cb-core::mission` (city-defence) as it is:
  its waves, its city damage and its loss at half damage. Intercept is won when every raider is
  destroyed. Destroy is won when the target is. Every kind fails on its clock and on the pilot's mech
  being destroyed.
- **The registry:** `JobKind` maps to one rule object implementing a small `JobRule` trait
  (`start`, `step`, `outcome`), so a new kind is one implementation and data, not `match` arms
  spread across files (CLAUDE.md 6.2).

## 3. In the game (`cb-app::jobs`, `cb-app::mail`)

- **MAIL page:** a fourth MFD page (the MFD texture grows from three pages to four, 2048 x 512). The
  list shows sender and subject; pressing a line opens it; an offer shows ACCEPT and DECLINE as
  pressable rows. Same press as the other pages (free look, look, click).
- **M focuses the mail** (survey J1): pressing M puts the MAIL page on the screen nearest the
  centre of the view and points the mouse on it, as a base terminal does; M or Esc steps back. Look
  input pauses only while the mouse points on the screen; the view itself never moves.
- **The menu's mail button** (survey J1): Esc in the mech opens the pause menu, whose buttons switch
  between its pages (game-menus); MAIL is one of them, drawing the same page.
- **The base terminal:** a desk in the ready room (air-bases) with a screen showing the same page,
  pressed with Use when the pilot stands at it. One page, two screens, one implementation.
- **Nav:** the accepted site shows on the nav page as a bearing and distance, and as a marker on the
  screens over the site, drawn by the code that draws target boxes.
- **Arrival:** within `radius_m` the job turns Active and its forces are released through
  invasion-forces' spawner (drops and arrivals, no pop-in in view).
- **Outcome:** pay through `Career::pay`, a result message, the save written.
- **The defence today:** the existing `--mission defence` start stays for playtests; on a career it is
  one Defend job among others.

## 4. Travel, and losing the mech

- **Travel:** jobs are in or near cities, and the pilot gets there on the jets and skates as they
  are; both recharge (survey J2). There is no cruise mode (jet-cruise is withdrawn). A test key starts
  the accepted job at its site, so its fight can be tested without the trip.
- **Ejecting** (survey J3): when the piloted mech is destroyed, the pilot is thrown clear: the view
  rides an `EyePath` out of the hatch and down to the ground beside the wreck, with look staying the
  player's, and the pilot is on foot (`PilotMode::OnFoot`). The job fails. The wreck is recovered to
  the nearest base, where it stands in a bay waiting for a full repair, charged when repaired. The
  pilot gets there on foot, or in a car (car-driving).

## 5. Cost

- Mail and jobs: a few hundred bytes each; the generator runs once per new offer (microseconds).
- The MAIL page: 512 x 512 more UI drawn into the MFD texture only while seated with screens
  (about 0.05 ms of UI layout, measured when built).
- The fight itself costs what invasion-forces and city-defence already cost.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J1 | Where you read mail and accept jobs | MFD MAIL page, M to focus it, the menu's mail button, and the base terminal | decided (survey J1) |
| J2 | How you get to a job | No cruise: jets and skates as they are; jobs in and near cities | decided (survey J2) |
| J3 | Losing your mech | You eject and get to the nearest base on foot; the mech is recovered there with a full repair bill; no pay | decided (survey J3) |
| J12 | Which kinds of job first | Defend a city, intercept a raid, destroy a crab nest | decided (survey J12) |
