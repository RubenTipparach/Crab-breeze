# Proposal: vegetation

## Why

Owner, 2026-10-24: "Trees system? Knocking down trees?" Open ground between cities is bare. Woods
give it scale (a 12 m mech beside a 20 m pine) and a mech wading through a forest, snapping trunks
and leaving a wake of fallen trees, is one of the most mech things there is.

## What changes

- Seeded scatter of trees from rules in data: forests by noise mask on open ground, thinner on
  slopes, none on roads, in cities (except park blocks), or in water; rows along rivers.
- Trees are instanced at three LODs plus an impostor card past 300 m, from tree models made by the
  art side (a Blender generator), with stable ids.
- Knocking down: a tree hit by a mech's body or leg, or by an explosion's impulse over its
  `break_impulse_n_s`, snaps at the base: it becomes an Avian rigid body pivoting on its stump,
  falls, rolls to rest, and stays down (its id remembered as fallen). The stump stays. Smaller
  trees snap with less.
- The mech feels it: a tree it walks into costs a little speed; a large trunk it cannot break stops
  it like a wall.

## Out of scope

Grass blades and undergrowth (a later change), fire, trees regrowing, chopping logs.
