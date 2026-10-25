## ADDED Requirements

### Requirement: Jobs are offered by mail and accepted by the pilot

The game SHALL offer jobs as messages with a site, a pay and a time limit, and a job SHALL start only
after the pilot accepts it.

#### Scenario: Accepting an offer
- **WHEN** the pilot opens an offer on the MAIL page and presses ACCEPT
- **THEN** the job's site appears on the nav page and its clock starts

### Requirement: A job's fight starts at its site

An accepted job's forces SHALL arrive only when the pilot comes within the job's radius of its site.

#### Scenario: Arriving
- **WHEN** the pilot comes within 1 500 m of an accepted intercept job's site
- **THEN** the job is active and its raiders arrive

### Requirement: Outcomes pay through the career

A won job SHALL pay its pay through the career's ledger and a failed job SHALL pay nothing.

#### Scenario: Winning
- **WHEN** the last raider of an intercept job is destroyed
- **THEN** the pilot's money rises by the job's pay and a message reports the win

### Requirement: Free roam starts quiet and fights only accepted missions

In free roam, no enemy SHALL be in the world when the run begins. Enemies SHALL come only for a
mission the pilot has accepted on the email MFD, and only when the pilot reaches that mission's
waypoint, far enough out that they are not firing as the pilot climbs aboard (owner, 2026-10-25: "dont
start wave unless player goes to ther email mfd and accepts a mission in free roam", "remember enemies
dont spawn until you arrive at misison waypoint", "the enemies shuodl be farther away so they dont
start shooting as soon as I board in freeplay"). A scripted take SHALL start its fight itself without
the email MFD (owner: "the playtest harness doesnt need toactivate in mfd, only when platyer platys
the shipped game").

#### Scenario: Boarding in free roam
- **WHEN** a free-roam run begins and the pilot walks to the mech and boards it
- **THEN** no enemy mech or vehicle exists and nothing fires at the pilot

#### Scenario: Accepting a mission
- **WHEN** the pilot accepts a mission on the email MFD in free roam
- **THEN** its waypoint shows on the screens and still no enemy spawns

#### Scenario: Reaching the waypoint
- **WHEN** the pilot comes within the mission's radius of its waypoint
- **THEN** the mission's enemies spawn around the waypoint, none within the boarding stand-off of the pilot

#### Scenario: A scripted take
- **WHEN** the fight or defence take runs (`--showcase` or `--bench`)
- **THEN** its enemies are there as before, with no email MFD step
