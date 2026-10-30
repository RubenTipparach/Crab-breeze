## ADDED Requirements

### Requirement: Ships sail in and out of every seaport
Each seaport's ships SHALL follow a timetable from the seed and the clock: anchor, sail in through the mouth, berth,
stay `dwell_s`, sail out; no two ships SHALL hold one berth at once.

#### Scenario: A ship berths and leaves
- **WHEN** the clock runs through one ship's schedule
- **THEN** it passes the mouth inbound, lies at its berth for `dwell_s`, and passes the mouth outbound

#### Scenario: One ship a berth
- **WHEN** every slot's schedule over a game day is laid out
- **THEN** no two ships are at one berth at the same time

### Requirement: Lane ships moor at the ports
A lane ship SHALL stop at each end of its lane for `dwell_s`.

#### Scenario: Dwell honoured
- **WHEN** a lane ship reaches a port
- **THEN** it stays there `dwell_s` before sailing back
