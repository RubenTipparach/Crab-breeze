## ADDED Requirements

### Requirement: The music follows the game's state
The game SHALL play the cue that `data/audio/music.ron` gives the current music state (title, boarding, combat, set piece, won), derived from the world by one rule in the simulation core.

#### Scenario: A fight starts in the cockpit
- **WHEN** the pilot is seated and an enemy within 2 km targets the mech
- **THEN** the music state becomes combat and the combat cue starts

### Requirement: Calm is silence
When the pilot is seated with no threat, no set piece and no stinger playing, the music SHALL fade out and stay silent, leaving the ambience; it SHALL fade only after 30 s without a threat or a shot.

#### Scenario: A wave cleared
- **WHEN** the last raider of a wave is destroyed and nothing else targets the mech
- **THEN** the combat cue keeps playing for 30 s, then fades out over 2 bars, and only the ambience is heard until the next threat

### Requirement: Cues change on the bar
A change of cue SHALL start on the next bar line of the playing cue and crossfade over the configured number of bars, so no cue is cut mid-bar.

#### Scenario: Combat begins mid-bar
- **WHEN** the state changes to combat 0.3 s into a bar of the 72 BPM boarding cue (a bar of 3.33 s)
- **THEN** the combat cue starts 3.03 s later, on the bar line, and the two cues crossfade over one bar

### Requirement: The score and the data agree on tempo
Validation SHALL refuse a music cue whose `bpm` differs from the tempo set in its Strudel score.

#### Scenario: A score is re-tempoed
- **WHEN** `music/sortie.strudel` changes to `setcps(156/60/4)` and `music.ron` still says 150
- **THEN** `cb-tools validate` fails naming the cue, both tempi and both files

### Requirement: One cue per state, the title on the menu
Each music state SHALL have its own cue: the title on the main menu and the loading screen, boarding, combat, the set piece while the air fortress is aloft and fighting, and won; on foot SHALL be silent.

#### Scenario: From the menu into the game
- **WHEN** the player picks a run on the main menu and the game becomes ready with the pilot on foot
- **THEN** the title cue plays through the loading screen and fades out over 2 of its bars once the game is ready

#### Scenario: The fortress comes over the city
- **WHEN** the pilot is seated and the air fortress mission is running with the fortress aloft
- **THEN** the set piece cue plays, over combat, until the fortress is brought down, when the win cue plays

