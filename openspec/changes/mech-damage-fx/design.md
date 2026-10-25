# Design: mech-damage-fx

## 1. Hit marks

- **The sheet**: `hit_marks`, a Material Maker graph in the `blast-scorch` pipeline, 4 x 4 cells at 1024 px, with
  albedo and alpha, normal and ORM (bare metal shows metallic 1 and roughness 0.35 at a puncture's rim).
  - Row 0: 35 mm punctures. A dark hole with a torn, bright rim, and a soot ring.
  - Row 1: 120 mm dents. A dished crater with a burnt halo and paint flaked off.
  - Row 2: rocket scorch splats. A sooty burst, blistered paint.
  - Row 3: gouges. Long scratches to bare metal, for glancing hits.
- **The decal**: a forward decal (`ForwardDecalMaterial<StandardMaterial>`, one material per row so marks batch)
  spawned on each hit that lands on a mech part (`combat.rs` `land()`, which knows the point, the shot's direction and
  the part). It is a child of the entity drawn there:
  - for a skin, the `ARM_*` or `LEG_*` node, or the skin's root for the torso and head;
  - for the stand-in, the part's bone.

  So it moves with the part, and comes off with it when the part is thrown.
  - Facing: along the surface normal of the hit (the hit capsule's normal at the point).
  - Turn: random from the shot's id.
  - Size: 35 mm 0.7 m, 120 mm 1.8 m, rocket 3.5 m, times the skin's scale over the stand-in's.
  - Glancing hits (under 25 degrees to the surface) take a gouge.
- **Caps**: 48 marks a mech (the oldest go first), 400 in the world. A mech's marks go when it is despawned.
- Numbers in `data/fx/hit_marks.ron`, validated.

## 2. Burning and smoking

- `Burning` gains a `scale`, and the stream follows a node:
  - for a skin, the stump is the skin's limb pivot (the `LEG_*` or `ARM_*` node's place before it was thrown);
  - the thrown limb carries its own `Burning` for 30 s.
- New recipes in `fx.ron`, drawn with the `fx-sprites` sheets:
  - `burning`: a stump. Fire 3 m and black smoke 6 m, rising 8 m/s, 8 s.
  - `smoking`: a critical part. Black smoke only, 4 m, 6 s.
  - `burning_hull`: a mech under 25 % in all. Fire at the torso and a thick column, 10 m puffs, 20 s.
- Stream rate 10 bursts a second, density 1, so a burning 24 m mech stays well under the 5 000 particle cap
  (about 400 live particles a fire).
- **Critical**: a part's health under `critical_frac` (0.25) smokes; the whole mech under it burns. From the sim's
  damage (`MechSim.damage`), read each frame. Presentation only: the sim decides nothing here.

## 3. Dust when things fall

- `fall_dust` (new recipe): a flat ring of `dust_wisps` thrown out at 10 m/s, and `dirt_clods` up and out. On paving,
  `fall_dust_concrete` (pale grit).
- Played when:
  - a thrown limb or a wreck first touches the ground (an avian3d `CollisionStart` against the terrain, once each);
  - a mech dies and its body comes down (`fall_wrecks`, when it reaches the ground);
  - a knocked-down mech's torso hits the ground (`close-combat` 2b).
- Scale: the cube root of the mass over 10 t. A 3 t arm is 0.67; a 60 t mech is 1.8.

## 4. Costs

| What | Cost |
| --- | --- |
| Decals | Up to 400 forward decals in 4 batches. Each covers a few hundred to a few thousand pixels at combat range |
| Texture memory | `hit_marks` 1024 px, 3 maps with mips: about 16 MiB |
| Particles | About 400 live a fire; capped by the 5 000 total |
| CPU | One spawn a hit; one health check a mech a frame |

Estimates. A cloud session measures nothing.

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| X6 | Do hit marks stay on the player's own mech (seen from the cockpit when looking at the arms)? | Yes |
| X7 | Does a burning mech go out? | A stump after 30 s; a critical mech burns until it dies |
