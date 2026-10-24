## ADDED Requirements

### Requirement: Simulation core without an engine
The workspace SHALL keep the simulation in `cb-core`, which SHALL NOT depend on any Bevy crate, rendering, windowing, audio or asset loading.

#### Scenario: The core builds and tests headless
- **WHEN** `cargo test -p cb-core` runs on a machine with no GPU
- **THEN** every core test runs and no Bevy crate is compiled for `cb-core`

### Requirement: Seeded, forkable randomness
`cb-core` SHALL provide one seeded random stream whose output depends only on its seed, and whose forks depend only on the parent's seed and the fork label.

#### Scenario: Forking leaves the parent unchanged
- **WHEN** a stream forks a child labelled `cities` and then draws a number
- **THEN** the number equals the one an unforked stream with the same seed draws first

### Requirement: Data files are validated on load
Every data file SHALL load through one loader that rejects unknown keys and non-finite numbers and names the file and the field in its error.

#### Scenario: A misspelled key stops loading
- **WHEN** a world file contains the key `city_cuont`
- **THEN** loading fails with an error naming the file and `city_cuont`
