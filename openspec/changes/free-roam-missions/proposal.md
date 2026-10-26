# Proposal: free-roam-missions

## Why

Owner, 2026-10-25, on the Hermit Air Fortress mockup: "looks good, build it in game, add an email in free roam to
fight the crab fortress. Free roam can activate missions, if activating a new mission it should turn off existing
mission."

Today free roam has no mail: mail and jobs exist only in a career (`mission-jobs`), and the one mission, the city
defence, starts only from the title menu or `--mission defence`. A career refuses a second job while one is under way.

## What changes

- **Mail in free roam.** M opens the mail in free roam as it does in a career (survey J1). Its inbox holds one
  standing message for each mission: the city defence (`city-defence`) and the Hermit, the crab air fortress
  (`crab-airship`). Each message has a Start button.
- **One mission at a time.** Starting a mission turns off the one running: its enemies leave the world, its clock
  and its readout stop; what it destroyed stays destroyed. Starting the running mission again restarts it. The
  running mission's message has a Stop button that turns it off and leaves free roam quiet.
- **The same rule in a career.** Accepting a job while another is under way abandons that one: its enemies leave,
  it pays nothing, and the ledger records it as abandoned (it was refused before).
- **`--mission fortress`** starts free roam with the Hermit running, as `--mission defence` does the defence.
- The fortress mission itself (the Hermit in the game) is `crab-airship`, sections 7 and 9.

## Out of scope

The MFD MAIL page (`mission-jobs` task 2.1; free roam's mail uses the page a career's mail uses today), pay or
progress for free-roam missions, more missions than the two.
