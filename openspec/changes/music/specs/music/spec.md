## ADDED Requirements

### Requirement: The music follows the game's state
The game SHALL play the cue that `data/audio/music.ron` gives the current music state (title, on foot, boarding, combat, set piece, won), derived from the world by one rule in the simulation core.

#### Scenario: A fight starts on foot
- **WHEN** the pilot is seated and an enemy within 2 km targets the mech
- **THEN** the music state becomes combat and the combat cue starts

### Requirement: Cues change on the bar
A change of cue SHALL start on the next bar line of the playing cue and crossfade over the configured number of bars, so no cue is cut mid-bar.

#### Scenario: Combat begins mid-bar
- **WHEN** the state changes to combat 0.3 s into a bar of the 92 BPM on-foot cue (a bar of 2.61 s)
- **THEN** the combat cue starts 2.31 s later, on the bar line, and the two cues crossfade over one bar

### Requirement: The score and the data agree on tempo
Validation SHALL refuse a music cue whose `bpm` differs from the tempo set in its Strudel score.

#### Scenario: A score is re-tempoed
- **WHEN** `music/sortie.strudel` changes to `setcps(156/60/4)` and `music.ron` still says 150
- **THEN** `cb-tools validate` fails naming the cue, both tempi and both files
