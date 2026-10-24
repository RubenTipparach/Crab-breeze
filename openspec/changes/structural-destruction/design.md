# Design: structural-destruction

## 1. The graph

```text
Element { kind, storey, mass_kg, hp_mj, rests_on: [Ground | ElementId], lateral: [ElementId] (floor bays only), modules }
```

- **Columns**: storey 0 rests on the ground. Above that, a column rests on the column below
  (listed first, the load line) and on the up to four floor bays below it, which hold it up if its
  column goes. The mockup showed the floor-bay fallback is needed. Without it, losing one column drops
  the whole column line out of an intact building.
- **Floor bays** rest on their four corner columns. They hang laterally from their four neighbours.
- **Wall panels** rest on the floor bay inside them (or the ground) and the columns at their ends.
  **Glass, fire escapes and awnings** rest on their wall panel.
- **Partitions, cores, furniture** rest on the floor bay under their centre. **Parapets and roof
  props** rest on the roof bay under them.

Ids are generation order: building, then element. Solve order is (storey down, kind rank, id), never
hash or ECS order (section 6.3).

## 2. A solve (one building, on change)

1. **Connectivity**: breadth-first from elements resting on the ground, over standing elements.
   Then floor bays with no support take a hang from a neighbour held directly (`hang_hops = 1`), and
   what rests on them is supported too.
2. **Islands**: unsupported standing elements, joined by union-find over their bonds. Each island
   leaves the graph and falls as one cluster. It tips away from the centroid of what it last stood on,
   at 0.35 rad/s.
3. **Load line**: a column carries load only if every column below it to the ground stands.
4. **Load pass**, top storey down. Each element adds its weight to its load and splits the load
   evenly over its targets:
   - floor bay: its standing load-line corner columns, else the neighbours it hangs from, else it
     fails now;
   - load-line column: the column below;
   - other column: the floor bays it stands on;
   - anything else: what it rests on.
5. **Check**: `stress = load / (capacity x support_factor)`.
   - Capacity is `safety_factor x` the intact load, from a design solve at generation.
   - `support_factor` for floor bays: four corners 1.0, three 0.85, two across 0.7, two on one edge
     0.45 (a cantilever), one 0.25, hung 0.4.
   - An element over 1.0 is scheduled to fail after a creak drawn from `creak_s` (0.5-1.6 s) on the
     simulation's forked stream.

## 3. Falling islands and impact

- Clusters fall under 9.81 m/s^2 as rigid bodies in the core: position, velocity, orientation and spin,
  in fixed steps.
- Contact is tested at up to 48 sample pieces against the highest standing floor or rubble under
  them.
- When a cluster lands on standing floor bays after falling more than 0.5 m, each bay under it takes
  `weight x impact_factor x min(2, 1 + fall / 3.5 m) / bays`. A bay pushed over its capacity fails at
  once, and the cluster keeps falling at 55 % of its speed. Otherwise the cluster stops and shatters.
- Shattering hands its pieces to the debris pool (Avian bodies up to the `debris-physics` budget) and
  its fines to `rubble`.

## 4. Drawing breakage

- Every building vertex carries a `u32` element index (local to the building). Each building has a
  storage buffer: per element, standing (1 bit), stress (8 bits, for the stress view and the cockpit
  target readout) and storey (for cutaways in tools).
- `ExtendedMaterial<StandardMaterial, BuildingMask>`: the vertex stage moves a broken element's
  vertices to one point, so its triangles cover no pixels. The same test runs in the prepass and
  shadow vertex stages.
- A break writes a few bytes to the buffer. No mesh is rebuilt, and draws stay at one per material per
  building.
- **Pieces**: each module's `FRAC_` cells (cut in Blender at build time, 4-12 per module) spawn from a
  pool of 9 000 instanced pieces when the element breaks. Pieces fly from the blast at
  `6 m/s x sqrt(E) x (1 - d / 1.5R) + 1 m/s`. When the pool is full, the oldest resting piece becomes
  rubble (`rubble`).
- **Damage stages before a break** (from Star Crew's `hull-repair`): a wall panel under 75 % hp shows
  crack and scorch decals. Under 40 % it swaps to its holed variant (two pre-broken meshes per module,
  same element). At 0 it breaks.

## 5. Cost

From the mockup (JavaScript, a browser; not a game measurement):

| Item | Value |
| --- | --- |
| Elements on the block | 10 454 (tower 3 216) |
| Bonds | 21 929 |
| Worst building's solve | 0.7-2.1 ms |
| Collapse of the warehouse | 775 elements broken, about 5 000 pieces resting, 645-719 m3 of rubble |

Expected in Rust, measured in `perf-bench` with the `side` demolition in view:

- A solve is about 0.1-0.2 ms for a 3 000-element tower on one core.
- At most 4 solves per fixed step, so the worst step stays under 1 ms.
- Memory: about 48 bytes an element plus the mask (4 bytes). A tower is about 170 kB.
- City-wide, graphs exist only for buildings within the interior radius or damaged. The rest are
  regenerated from the seed when needed.

## 6. Tuning (data, `data/world/buildings.ron`)

| Key | Default | Unit |
| --- | --- | --- |
| `safety_factor` | 2.0 | ratio |
| `impact_factor` | 2.0 | ratio |
| `creak_s` | (0.5, 1.6) | s |
| `hang_hops` | 1 | bays |
| `slab_support_factor` | four 1.0, three 0.85, diagonal 0.7, one_edge 0.45, one 0.25, hung 0.4 | ratio |
| `hp_mj` | column 2.5, slab 3.0, wall 0.7, glass 0.02, partition 0.15, core 1.2, furniture 0.05, prop 0.35, parapet 0.5, escape 0.25 | MJ |
| `debris_pool` | 9 000 | pieces |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W12 | How deep does destruction go? | structural: columns and floors carry load, overloaded members creak then fail, storeys pancake | provisional (survey W12) |
| W15 | Creak before a collapse | 0.5-1.6 s of dust and groaning, shown on the target readout | provisional (survey W15) |
