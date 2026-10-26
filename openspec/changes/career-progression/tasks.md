# Tasks: career-progression

- [x] 1.1 Job tiers and pay in data; the generator offers only tiers the owned mechs open; tested
- [x] 1.2 Mech `tier` in each mech's data, validated
- [x] 1.3 Standing per base in the career, gained and lost by job outcome, saved; tested
- [x] 2.1 The mail shows each offer's tier and the standing; a message when a tier opens
- [x] 3.1 Playtest: earn with the Foreman, buy a heavier mech, see tier 3 offered (2026-10-25,
  `docs/playtests/2026-10-25-career-loop.md`: one job's pay plus a disclosed grant bought the Sapper; tier 3 offered)

Built 2026-10-25 on the recommendations (CP1 four tiers, CP2 standing only changes pay): tiers live in
`data/jobs/jobs.ron` per kind; the board picks a kind among those with a tier open, then a tier among the open
ones at even odds; a job is offered by the base nearest its site; buying a mech that opens a tier mails "Tier n
contracts open" and offers one of that tier at once, beyond the open limit (`Board::open_tier`).
