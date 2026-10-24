# Tasks: combat-fx

- [x] 1.1 `data/fx/fx.ron` recipes, loaded and validated (tests: `every_recipe_the_game_asks_for_is_in_the_file`, `a_missing_recipe_stops_startup_naming_it`, `an_unknown_layer_key_is_an_error`)
- [x] 1.2 `Fx` message and the layer spawners (Burst, Light, Fireball, Shock, Star); callers in combat, blast, mech and world ask by recipe name
- [x] 1.3 One tracer per shot, stretched along its velocity, gone the frame the shot stops
- [x] 1.4 Muzzle flashes by weapon, impacts by surface
- [x] 1.5 Layered explosions: fireball, shock, burning debris with trails, smoke column (test: `a_fireball_cools_from_white_to_soot`)
- [x] 1.6 Rocket motor flame, light and smoke trail
- [ ] 2.1 Light and particle caps; measure `--bench defence` against `CB_NO_PARTICLES=1` on the RTX 3070 machine
- [x] 2.2 Playtest: tracers, flashes, impacts, explosions and rocket trails seen from the cockpit in the defence take
- [ ] 3.1 Particle layers on `bevy_hanabi` (pinned to its Bevy 0.20 commit): gradients, velocity-aligned sparks, ribbon trails (T5)
- [ ] 3.2 Flipbook smoke, fire and dust from Blender simulations (G7)
- [ ] 3.3 Measure against the entity particles; playtest
