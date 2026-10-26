# Design: mech-collision

## 1. The shape: boxes per bone, measured off the glb

Each contract bone's meshes (`pelvis_*`, `thigh_L_*`, ..., `docs/design/mech-rig.md`) are taken into the bone's own
frame at rest and filled into solid boxes:

1. Voxelise the bone's triangles on a 0.5 m grid in the bone's frame (a cell is solid where a triangle crosses it).
2. Flood the grid from outside; every cell the flood does not reach is inside, so hollow shells come out solid.
3. Merge solid cells greedily into boxes (longest run in x, then y, then z), at most 24 boxes a bone; past that the
   smallest neighbours merge (a box may grow by at most 0.5 m on a side).
4. Drop boxes under 0.25 m on every side (pipes, hoses, decals): the pilot is 0.70 m wide.

The result goes into the mech file, generated, never edited:

```ron
body: [
    (bone: "foot_L", min_m: (-1.25, 0.00, -2.00), max_m: (1.25, 1.50, 3.00)),
    ...
],
```

A mech with no `body` (the stand-in) is not solid, as now. `cb-tools validate` rebuilds the boxes from the glb and
refuses a file whose boxes differ by more than 1 cm.

## 2. Posing: the bones the game already draws

Each fixed step the app reads the world transform of every contract bone of each mech within 40 m of the pilot (its
posed skeleton, the same transforms that draw it; one implementation of the pose) and hands the core a list of posed
boxes: `MechBody { boxes: Vec<(Affine3A, Vec3, Vec3)> }` (bone to world, min, max). A wreck keeps its last pose.

## 3. The push-out

`cb-core` (`world::body`), no engine in it. The pilot is a circle of `body_radius_m` (0.35 m) on the ground plane,
standing from `feet_y + 0.4 m` (it steps over a lip that low, as with roofs) to `feet_y + eye_height_m + 0.2 m`
(1.9 m). For each posed box near the pilot:

1. Clip the box (8 corners, 12 edges in world space) to the horizontal slab between those heights.
2. Project what is left onto the ground plane and take its convex hull (a box's slice is convex).
3. Push the circle out of the polygon by the shortest way (`push_out_of_convex`, the same rule as
   `collision::push_out_of_rect`, which becomes its axis-aligned case).

Two passes settle a pilot wedged between two boxes, as for buildings. Boxes are visited in mech id, bone and box order,
so the result never depends on query order (CLAUDE.md section 6.3). `world::Scene` gains `bodies: &[MechBody]`, and its
`push_out_at` runs the boxes after buildings, rocks and trees: the walk itself does not change.

## 4. The rope's foot

`gen_mech_data.py` already poses the kneel (`kneel`). It also poses the boxes there and checks the pilot's circle at
`LADDER_base` against them; a mech that fails is reported and its data not written. A Rust test does the same on the
written data with the core's own push-out (the pilot at the rope's foot is not moved).

## 5. Cost

About 8 to 24 boxes a bone, 23 bones: at most 550 boxes a mech, usually far fewer, in the mech file (about 40 kB of RON).
Each step, mechs beyond 40 m are skipped; a box whose bounding circle misses the pilot's is skipped before clipping.
Expected well under 0.05 ms a fixed step with a mech beside the pilot; not measured (cloud sessions measure nothing,
CLAUDE.md section 7). No triangles, draw calls or texture memory.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| MC1 | Grid size of the fill | 0.5 m: tight enough to walk under a thigh, small enough to keep the file short | provisional |
| MC2 | Can the pilot stand on a mech's toe or a kneeling thigh? | No, walls only, for now | provisional |
