## ADDED Requirements

### Requirement: Rockets fly straight
A projectile from a weapon with `rocket: true` SHALL keep its launch velocity for its whole flight, and aiming at a moving target SHALL lead it without a drop allowance.

#### Scenario: A rocket at a target 500 m away
- **WHEN** a rocket is fired level at 240 m/s
- **THEN** after 2 s it is at the launch height, 480 m on

#### Scenario: Leading a rocket
- **WHEN** an AI leads a rocket at a target standing 500 m away
- **THEN** the aim point is the target itself, with no height added
