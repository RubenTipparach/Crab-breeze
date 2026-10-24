# Requests between sessions

The Mac engine session cannot message the cloud art session (the server refuses: untrusted
device), so requests go here, newest first. Answer by editing the line or by a commit.

## 2026-10-24, Mac engine session to cloud art session

1. **PBR terrain textures** (owner: "Someone needs to make pbr terrain textures"). Change
   `terrain-materials`, tasks 1.1-1.2: Material Maker graphs for grass, dry grass, dirt, rock,
   mud, gravel, asphalt, concrete and scorch; tiling, about 2 m a tile, 2048 px, albedo, normal,
   ORM and height, in `assets/textures/terrain/<name>_{albedo,normal,orm,height}.png`, listed in
   `data/materials/terrain.ron`.
2. **Tree models** for `vegetation`: pine, oak and birch, three LODs and an impostor card each,
   trunk as its own node so the engine can snap it at the base.
3. **Breeze's hatch**: the game loads only `mech_breeze.glb`; the hatch nodes are in
   `cockpit_breeze.glb`. Please merge the pod into the mech glb. Give hinge nodes names that do not
   prefix their meshes (`LATCH_*` also catches `LATCH_1_ext`), or tell me the exact names and I
   will list them. Origin on the hinge.
4. **Breeze's stair foot**: `LADDER_base` is under its belly, so the pilot walks between its legs
   (showcase take, `docs/playtests/2026-10-24-showcase.md`). Put it at the foot of the stair, in
   front, with the mech sat.
5. **Breeze does not fold when it sits**: what are its pelvis height at rest and its leg segment
   lengths? `data/mechs/breeze/boarding.ron` asks for `pelvis_height_m: 1.4`.
6. **A `head` bone** for `mech-damage`: the sensor head that carries the all-view cameras (parent
   `torso`, a `CAM_main` empty under it), plus hidden `STUMP_*` caps at each arm, leg and the head.
   A contract change for both sides in one commit; tell me when and I will move the engine with it.
7. **Survey**: I added W1-W8, M7, M8 and T4 to `docs/design/survey.html`; please republish if my
   republish of the artifact did not land.
