## ADDED Requirements

### Requirement: Particles are drawn from Material Maker sheets
Smoke, steam, trails, fire, flame, dust, dirt, debris and rubble particles SHALL each be drawn from a cell of their kind's Material Maker sprite sheet, and fire and flame SHALL step through their sheet over their life.

#### Scenario: A blast's smoke
- **WHEN** an `explosion_ground` plays
- **THEN** its smoke particles each show a cell of `smoke_puffs`, not the disc

#### Scenario: Fire burns through its frames
- **WHEN** a fire particle is half way through its life
- **THEN** it shows frame 8 of `fire_flipbook`
