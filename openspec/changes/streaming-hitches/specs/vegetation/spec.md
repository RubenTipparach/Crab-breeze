## ADDED Requirements

### Requirement: Trees near a body are grown in bounded steps
In one fixed step the forest SHALL grow every tile within `must_radius_m` of a body, at most `urgent_tiles_per_step`
more within `urgent_radius_m`, and at most `tiles_per_step` more within the keep radius, nearest first.

#### Scenario: An enemy brought in somewhere new
- **WHEN** a body appears 6 km from every grown tile
- **THEN** the tiles within 60 m of it are grown that step, the rest of its 400 m ring waits, and the ring is complete
  within a few steps (test `a_body_new_to_a_place_grows_its_near_trees_at_once_and_the_rest_a_few_a_step`)
