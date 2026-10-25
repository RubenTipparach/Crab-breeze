# Design: mech-roster

The eight detailed mechs in the game, each flown from its own cockpit, at its own size. Built on `mech-rig` (the
contract and the binder), `mech-boarding` (the timeline) and `cockpit-overlay` (the sealed cabin drawn on its own
layer). Nothing here forks those: the new mechs go through the same binder, the same gait, the same boarding clock.

## 1. The glb meets the contract

`cb_core::mech::rig` lists 23 bones and 10 sockets. The binder (`cb-app/src/mech.rs` `bind_rig`) refuses a model that
lacks any of them and never checks parents; the pose maths does use the contract parent's pose, so a wrong parent
drives wrongly without a word. The eight glbs today:

| Gap | Mechs | Fix in `tools/blender/build_mechs.py` |
| --- | --- | --- |
| no `LADDER_base` | all eight | a socket under `root` on the ground, 3 m in front of `HATCH_entry` (the owner dropped the ladder; the socket stays as the spot the pilot walks to and climbs out onto) |
| `JET_L`, `JET_R` under `cockpit` | all eight | placed in the torso's frame at the same point (`back_jets`) |
| no `MUZZLE_L` | Picket, Sapper | at the shield's short gun (Picket) and the forearm missile pod's mouth (Sapper) |

**The check:** `cb-tools validate` opens every glb a shipped mech file names, reads its node tree from the glb's JSON
chunk, and fails on a missing contract name or a contract node whose parent is not the contract's, naming the file and
the node. A test runs the same check, so `cargo test --workspace` refuses a broken glb.

## 2. The cockpit interior

The interiors are shared: four glTFs (`assets/models/cockpit_<variant>/`) for eight mechs, 12 to 15 MB each with their
4096 px PBR atlases. Copying one into every mech glb would add about 100 MB to the repository, so the game loads the
interior beside the mech, as the page does.

**Data.** `MechConfig.interior: Option<InteriorConfig>`:

```ron
interior: Some((
    model: "models/cockpit_panorama/cockpit_panorama.gltf",
    rides: [(part: "INNER_lower", joint: "CHEST_lower_1"), (part: "INNER_upper", joint: "CHEST_upper_1")],
)),
```

`rides` is the glb's own `cockpit.rides` (written by `mech_boarding.pod`), copied by the generator (section 4).

**Spawning.** With the mech's scene the game spawns the interior's scene. Once the mech is bound and the interior has
spawned (`bind_interior`, after `bind_rig`): the interior's root goes under the `cockpit` bone at identity (the page:
`nodes.cockpit.add(pod)`); each riding part is reparented onto its chest joint keeping its model-space transform (the
page: `attach`), so it opens on the chest's tracks; every interior node joins the rig's names so a boarding motion can
name it. The interior's own top node is called `cockpit`; it is bound after the contract, so it can never be taken for
the bone.

**Lights.** The interiors carry six punctual lights each for the page. The game lights the cabin itself
(`cockpit-lighting`: cabin ambient and the screens' glow on the cockpit layer), so the interior's lights are despawned
when it is bound.

**The sealed view.** Today, seated and sealed, every mesh of the mech moves to the cockpit layer and is drawn over the
world. A mech with an interior instead:

| Mesh | Seated and sealed |
| --- | --- |
| the interior (seat, dash, frame, MFDs, sticks) | cockpit layer |
| the screen wall: interior nodes ending `_allview` (the `SCREEN_*_screen` nodes are console monitors and stay) | not drawn: the world shows through, as the page's screens show it |
| the rest of the mech (hull, chest armour, arms, legs) | not drawn (a layer no camera renders): it would stand between the screens and the world |

**The openings cut into the cabin** (`cb-app::screen_cut`). An interior is a closed box: each window quad has a back
plate 2 cm behind it, the pod's inner wall 4.2 cm behind, and a backing face in its own plane, so hiding the quads alone
left the screens black. When the interior binds, every cabin mesh loses the geometry inside the pyramid the eye sees
through each window triangle, from 2 mm in front of the window's plane back (triangles crossing an edge are split, their
UVs and normals interpolated); the window MFDs, hung 16 cm out, are untouched. Measured on the Foreman's Panorama: three
meshes cut (`pod_cabin` 14,392 to 15,708 triangles, `INNER_upper_1_cabin` 1,046 to 1,269, the console 8,292 to 8,296),
once per mesh asset, at load. The same cabin seen from outside with the hatch open still shows its window quads over the
openings.

Out of the seat, or with the hatch open, everything is drawn with the world as now. The stand-in and the Breeze, with
no interior, keep today's behaviour.

## 3. Sizes from data

A new `body` section in `MechConfig`, defaults equal to today's numbers so the stand-in and the Breeze are unchanged:

| Field | Default | Read by (today hard-coded) |
| --- | --- | --- |
| `eye_height_m` | 9.6 | the AI's eye (`mech.rs`), targeting (`targeting.rs` `EYE_M`), the showcases |
| `torso_hit_m` | (0.5, 3.9) | the torso capsule, from and to over the `torso` bone (`combat.rs`) |
| `torso_hit_radius_m` | 2.4 | the torso capsule |
| `arm_hit_radius_m`, `leg_hit_radius_m`, `head_hit_radius_m` | 1.0, 1.1, 1.1 | the limb and head capsules |
| `wreck_centre_m` | 10.0 | the wreck's topple (`WRECK_CENTRE_M`) |
| `blast_height_m` | 9.0 | the torso's death blast |
| `smash_height_m` | 5.0 | the point a mech striding into a wall breaks it |
| `cut_box_m` | (1.6, 3.0, 1.6) | the shot-off part's physics box |

The look ray's mech column (3 m round, 12.5 m tall) reads each mech's `body_radius_m` and `body_height_m`.

## 4. Each mech's data, generated

`tools/mechs/gen_mech_data.py` reads each glb and writes `data/mechs/<id>.ron` and `data/mechs/<id>/boarding.ron`. It
measures, in model space at rest: the pelvis height, `EYE_pilot`, `HATCH_entry`, the top of the model, and the chest's
half width. Scale `k = body height / 12 m`, against the tuned stand-in (`placeholder.ron`):

| Value | Rule | Foreman |
| --- | --- | --- |
| `hip_height_m` | measured pelvis height | 13.10 |
| `body_height_m` | measured top | 24.01 |
| `eye_height_m` | measured `EYE_pilot` | 18.36 |
| `body_radius_m` | measured half width of everything but the arms and legs | 7.56 |
| `max_wade_depth_m` | eye less 1.2 m (the cockpit floor) | 17.16 |
| `stride_max_m` | 2.2 x hip height (the stand-in's 13 m on 5.9 m) | 28.86 |
| `step_height_m`, `bob_m`, `sway_m`, `turn_step_radius_m`, `brush_radius_m` | stand-in x k | 1.8, 0.5, 0.3, 5.0, 11.0 |
| `mass_kg` | 50 t x k^3 | 400 t |
| hit capsules, wreck, blast, smash, cut box | defaults x k | |
| speeds, rates, jets, skates, weapons | the stand-in's | |

The Foreman column is what the generator measured and wrote. All eight, measured:

| Mech | Height | Hips | Eyes | Half width | Chest tracks | Interior |
| --- | --- | --- | --- | --- | --- | --- |
| Foreman | 24.01 | 13.10 | 18.36 | 7.56 | 4 | Panorama |
| Ironhorse | 22.67 | 12.14 | 16.13 | 5.75 | 9 | Excavator cab |
| Harrow | 23.79 | 14.83 | 19.03 | 4.34 | 2 | Canopy |
| Roustabout | 23.69 | 14.33 | 18.83 | 4.40 | 5 | Titan |
| Driller | 23.85 | 10.63 | 14.18 | 4.20 | 4 | Panorama |
| Mule | 23.68 | 13.30 | 17.56 | 4.50 | 2 | Excavator cab |
| Picket | 23.95 | 11.40 | 15.48 | 4.54 | 4 | Panorama |
| Sapper | 23.85 | 12.89 | 17.34 | 4.78 | 5 | Titan |

The sit lowers the pelvis to 0.29 of its height (the stand-in's ratio), which leaves the Foreman's eye about 9 m up
when it kneels: the pilot's hop to the hatch covers it, as it does the Breeze's. Whether the sill should come lower is
for the playtest (open question R4).

**Boarding.** The glb's root extras carry the chest's tracks (`node`, `t0`, `d`, an `axis` and an angle in radians, or
`pos` and a slide in metres). Each becomes one stage: `start_s = sit + 0.2 s + t0`, with the angle in degrees about the
node's own axis, or the slide turned into the node's frame (the page slides in the parent's). The sit:
`pelvis_height_m` = 0.3 x hip height (the stand-in's 1.8 m on 6.2 m), hocks at 90 degrees, torso bow 10 degrees, over
3 s, as the page sits. `ready_s` is the last stage's end plus 0.3 s. Vents and rams are not read by the game yet.

The generator is run after `build_mechs.py`; a test loads every shipped mech file, and `cb-tools validate` checks the
eye, hip and height in each generated file against the glb it names within 0.05 m, so a rebuilt mech with stale data
fails. (The Breeze's file predates the generator and inherits the stand-in's sizes; only the contract is checked for it.)

## 5. Cost

| Mech | Triangles | Draws | glb |
| --- | --- | --- | --- |
| Foreman | 23,714 | 114 | 2.1 MB |
| Ironhorse | 21,126 | 100 | 2.0 MB |
| Harrow | 15,768 | 111 | 1.8 MB |
| Roustabout | 16,215 | 105 | 1.8 MB |
| Driller | 17,280 | 106 | 1.9 MB |
| Mule | 13,700 | 107 | 1.7 MB |
| Picket | 12,842 | 93 | 1.7 MB |
| Sapper | 13,149 | 93 | 1.7 MB |

The interior adds 19,506 to 25,056 triangles and 10 to 15 draws (its manifest), and four 4096 px atlases, front and
cabin (albedo, emission, normal, ORM): 2 x 4 x 64 MB = 512 MB of GPU memory uncompressed with mips, against the 3070's
8 GB. Seated and sealed, the hull is not drawn, so the cabin view costs the interior alone. Nothing here is measured: a
cloud session has no GPU (CLAUDE.md section 7); a session on the owner's machine measures the cockpit view.

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| R1 | Which mech should the game start in? | The Foreman, once a playtest shows it walking, boarding and fighting; the stand-in until then |
| R2 | Should enemy mechs use the new roster? | Yes, a later change: each enemy needs its own boarding timeline, as the player's is shared today |
| R3 | 400 t for a 24 m mech knocks buildings flat: keep the cube law? | Keep it; tune after a playtest |
| R4 | Kneeling leaves the hatch about 9 m up on most of the eight: fold the legs further? | Keep the stand-in's ratio until the playtest shows the climb |
