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
