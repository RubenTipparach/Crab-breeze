## ADDED Requirements

### Requirement: A seeker gathers locks while its trigger is held
A missile weapon with a seeker SHALL add one lock every `lock_s` while its group is held, up to `max_locks`, on the target inside `range_m` and `cone_deg` with the fewest locks, nearest the aim on a tie.

#### Scenario: Sweeping three crabs with a six-cell pod
- **WHEN** three crabs stand inside the pod's 1 500 m and 25 degrees and the pilot holds F for 2.1 s
- **THEN** the pod holds six locks, two on each crab

### Requirement: Release fires a missile per lock
On release of the group, the weapon SHALL fire one missile per held lock, 0.08 s apart in lock order, each homing on its locked target.

#### Scenario: A salvo
- **WHEN** the pilot releases F holding locks on crabs A, B, A
- **THEN** three missiles leave within 0.16 s, two homing on A and one on B, and the pod reloads for 8 s

### Requirement: Locks drop when the target leaves
A lock SHALL be dropped when its target leaves `range_m` or moves beyond 1.5 times `cone_deg` from the aim before release.

#### Scenario: A crab ducks out of the cone
- **WHEN** a crab holding two locks moves 40 degrees off the aim of a 25 degree pod
- **THEN** both its locks are dropped and the pod builds new ones on the remaining targets

### Requirement: The single-lock missile is the same rule
The shoulder missile SHALL be a seeker with one lock, 1 200 m range, 12 degree cone and 1.0 s lock, and SHALL fire on press when its lock is already held.

#### Scenario: Today's missile
- **WHEN** a target has been inside 1 200 m and 12 degrees for 1.0 s and the pilot presses F
- **THEN** one missile fires at once and homes on it
