# Proposal: fx-sprites

## Why

Owner, 2026-10-25: "I think a lot of the fx need work". Every particle in the game (smoke, fire, dust, dirt, debris,
spray) is drawn with one sprite: a soft round disc, tinted by a colour over its life. A blast's smoke column is a stack
of grey balls, its fire a cluster of orange balls, its dirt round dots. Nothing has the edges, folds, lit side and
shadowed side real smoke and fire have.

## What changes

- **Sprite sheets from Material Maker** (the `blast-scorch` pipeline), sheets of 4 by 4 cells, 1024 px:
  - `smoke_puffs`: 16 billowing puffs, lit from above (the sun is high in every scene), with soft ragged edges;
  - `fire_flipbook`: 16 frames of a flame tongue licking up and breaking apart, played over each particle's life;
  - `dust_wisps`: 16 thin, torn dust clouds;
  - `dirt_clods`: 16 chunks of soil and stone, hard edged.
- **Each particle kind gets its sheet**: smoke, steam and trail take `smoke_puffs` (a random cell each); fire and
  flame play `fire_flipbook` over their life; dust takes `dust_wisps`; dirt, debris and rubble take `dirt_clods`;
  sparks keep their streak. Batching holds: one effect asset and one texture per kind.
- **Smoke reads as a volume**: each puff starts dark and dense near the fire and fades lighter and thinner, and turns
  slowly (the sheet's lit side stays up, so the turn is under 20 degrees).

## Out of scope

Lit particles (normal-mapped lighting per particle), soft particles against the depth buffer (`combat-fx` follow-up),
plumes (`rocket-plumes`), scorch marks (`blast-scorch`).
