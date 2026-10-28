## ADDED Requirements

### Requirement: A perf run fails on any frame over 16.7 ms after GAME_READY
A run SHALL pass the frame budget only when it saw at least one frame after `GAME_READY` and none of them took longer
than 1000/60 ms; with `--perf-gate` the game SHALL exit non-zero when it did not pass.

#### Scenario: One slow frame
- **WHEN** one frame of 16.8 ms comes among 500 of 7 ms
- **THEN** the run fails and names that frame (test `one_frame_over_16_7_ms_fails_the_run_and_is_named`)

#### Scenario: No frames
- **WHEN** a run ends before any frame after `GAME_READY`
- **THEN** it does not pass (test `a_run_with_no_frames_does_not_pass`)
