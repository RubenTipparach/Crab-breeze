# Design: combat-fx

Presentation only (`cb-app::fx`): nothing here decides an outcome, so it keeps its own cheap
random numbers and never touches the simulation's streams (CLAUDE.md section 6.3). The simulation
says what happened (a shot flew, hit a surface, a blast went off); the effect is chosen from that.

## 1. Recipes in data

`data/fx/fx.ron` holds every effect as a recipe: a list of layers, each one of

| Layer | Fields (units in the key) | Draws |
| --- | --- | --- |
| `Burst` | `kind`, `count`, `speed_m_s`, `spread`, `size_m`, `life_s`, `offset_m`, `along_dir` | billboard particles of one kind (the existing kinds plus `Debris` and `Trail`) |
| `Light` | `lumens`, `range_m`, `life_s`, `colour`, `lift_m` | a point light fading as (1 - t)^2 |
| `Fireball` | `radius_m`, `grow_s`, `life_s` | an emissive sphere that grows, then cools white, orange, soot, and fades |
| `Shock` | `radius_m`, `life_s` | a thin translucent shell expanding fast, and a ground ring |
| `Star` | `size_m`, `life_s`, `offset_m` | three glowing blades round the barrel's axis and a glow across it, turned randomly: the muzzle flash shape |
| `Ripple` | `radius_m` | a ring spreading on water |

A recipe's numbers scale with the message's `scale` (a 6 MJ blast is `scale` 1.82, the cube root
of energy over 1 MJ, since a fireball's radius grows with the cube root of yield). Loaded and
validated like all data: an unknown key, a non-finite number or a recipe the code asks for that
the file lacks stops startup with its path.

Recipes the game asks for: `muzzle_autocannon`, `muzzle_cannon`, `muzzle_rocket`,
`impact_mech`, `impact_building`, `impact_ground`, `impact_water`, `explosion_ground`,
`explosion_water`, `explosion_air`, `rocket_trail`, `foot_dust`, `foot_spray`, `jet_flame`,
`skate_grit`, `skate_spray`, `wall_smash`, `tree_fall`, `part_lost`, `burning`. Counts scale
with a second number, `density` (a heavy landing throws three times the dust).

## 2. Tracers and trails

A shot in flight gets one tracer entity, keyed by the projectile's index in `Shots` order each
step (stable order, never a hash). Its transform: centred half a streak behind the shot, length
`speed x tracer_s` (`tracer_s` 0.008 s: 9 m at 1100 m/s), width `tracer_width_m` (0.25 m light,
0.5 m heavy), rotated so its long axis lies along the velocity and its face turns to the eye.
Material: emissive amber, additive. When the shot stops, the tracer goes in the same frame.

A rocket gets the motor flame (a `Flame` burst at the tail every frame), a point light of 2 x 10^5
lm riding with it, and a `Trail` puff dropped every `trail_spacing_m` (3 m), each lasting
`trail_life_s` (12 s), growing 0.3 of its size per second, drifting up at 0.4 m/s. A 2 km rocket
leaves about 670 puffs.

## 3. Explosions

Layers, at `scale` 1 (1 MJ), seconds and metres:

| Layer | Numbers |
| --- | --- |
| Light | 4 x 10^8 lm per MJ, 300 m, 0.6 s (as now) |
| Fireball | 4.5 m, grows over 0.15 s, lives 0.9 s; colour white (0 s), yellow-orange (0.2 s), deep orange (0.5 s), soot (0.9 s) |
| Shock | 22 m in 0.35 s; ground ring 30 m over 1.5 s |
| Debris | 24 burning chunks, 25 m/s, cone 0.9, each trailing smoke for its 2.5 s |
| Dirt, dust | as today's blast (160 clods, 60 dust) |
| Smoke column | 30 puffs rising 3 m/s, growing 0.25 per s, 20 s |

## 4. Budget and cost

- Particles stay under `MAX_PARTICLES` (5000, as now); a burst that does not fit is cut, never
  queued. Trails count against it.
- Meshes: one quad, one star, one sphere (32 x 16, 1 k triangles), one shell, one ring, shared.
  Materials: one per particle kind plus one per live fireball (its colour animates); at most 64
  fireballs alive.
- Draws: billboards batch per material (about 12 draws); each fireball is one draw.
- Lights: at most 24 effect point lights alive (the oldest goes first); none cast shadows.
- CPU: moving 5000 billboards is about 0.3 ms a frame on the owner's Mac (the city fight's
  measured particle switch, `CB_NO_PARTICLES`); this change adds the tracer and fireball updates,
  a few microseconds each. Measured with `--bench defence` against `CB_NO_PARTICLES=1` on the RTX
  3070 machine.
- Texture memory: none new (the soft disc, 16 KB).

## 5. Sprites

Particles keep the soft-disc sprite until the owner picks where flipbooks come from (survey `G7`):
a Blender smoke and fire simulation rendered to a flipbook atlas, or Material Maker noise graphs.
A flipbook will arrive as a new field on `Burst` layers, so the recipes keep their names.

## 6. Next: GPU particles (owner, 2026-10-24: "the fx look way too rough", "is there not a package for good looking particles in bevy?")

The first pass draws every particle as one entity with the same flat, unlit soft disc, so smoke
reads as dark blobs and burning debris as strings of dots. The next pass moves the particle layers
onto [`bevy_hanabi`](https://github.com/djeedai/bevy_hanabi) (MIT or Apache-2.0), Bevy's GPU
particle system: colour and size gradients over a particle's life, flipbook sprites, velocity
aligned sparks, ribbons for trails, and hundreds of thousands of particles for the cost of a few
thousand entities today. Its 0.19 release targets Bevy 0.19; its main branch moved to Bevy 0.20 on
2026-10-15 (commit `77630ff1eed8b104f34e91d23e9440428dc5ab01`), not yet released. The plan
(survey T5): pin that commit as a git dependency, move to the crates.io release when 0.20 ships.
The recipes stay the one description of every effect; each `Burst` layer becomes a hanabi effect
asset built from its numbers, and the flipbooks (G7) feed its textures. Lights, fireballs, shells
and stars stay Bevy entities. Measured against today's particles on the RTX 3070.

As built (2026-10-24): `cb-app::particles` builds one effect asset per recipe layer and particle
count (cached), spawned as a one-shot effect entity at the burst; colour over life per particle
kind (fire white-hot to orange to soot, burning debris cooling to a smoking cinder, smoke fading in
and out), size growing with age, sparks oriented along their velocity, the soft disc as the sprite.
A rocket's trail is a continuous effect on its own entity beside the motor, switched off when the
rocket stops so its smoke hangs on. Order-independent transparency is gone with it: hanabi draws
outside it, and the OIT resolve would have composited water and clouds over the particles; the
cameras are back on MSAA. Not yet seen in a run.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G7 | Where smoke, fire and explosion sprites come from | Blender Mantaflow smoke and fire baked and rendered to 8 x 8 flipbook atlases, with PBR-lit normal maps | provisional (survey G7); soft discs until then |
| T5 | Particles on `bevy_hanabi` | yes, pinned to its Bevy 0.20 commit until the release | provisional (survey T5) |
| G8 | Near blasts jolt the cockpit | A short position-only jolt of the cockpit; the view's direction stays the player's | provisional (survey G8); not built |
