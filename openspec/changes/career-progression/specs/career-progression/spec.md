## ADDED Requirements

### Requirement: Riskier jobs open with stronger mechs
Every job SHALL have a risk tier, and the board SHALL offer a tier only when the career owns a mech rated for it.

#### Scenario: Only the Foreman
- **WHEN** the career owns only the Foreman (tier 2)
- **THEN** no tier 3 or 4 job is offered

#### Scenario: A heavier mech bought
- **WHEN** the career buys a tier 3 mech
- **THEN** tier 3 jobs are offered from then on and the mail says so
