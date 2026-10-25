## ADDED Requirements

### Requirement: Building detail is built off the main thread
A nearby building's dressing (window frames, ledges, parapets, roof props, glass) SHALL be meshed on worker threads, so the frame that asks for it does not wait for it, and the plain box SHALL show until the dressing is ready.

#### Scenario: Walking into a city
- **WHEN** the pilot walks toward buildings that come inside the dressing radius
- **THEN** no frame spends more than 4 ms in `dress_near_player`, and each building's dressing replaces its plain box once it is built
