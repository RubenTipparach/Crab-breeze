# Tasks: smoke-puffs

**Backed out, 2026-10-25** (owner: "can you back out of these puffs, they look bad"). The puff renderer,
the `Puffs` layer and the trail laying were removed from the game; the jets and missiles draw the sprite
trails again (`jet_trail`, `rocket_trail`). Nothing below is in the game. The design and the mockup stay
as a record of what was tried.

- [ ] 1.1 Shared cloud lighting: one shader, `assets/shaders/cloud.wesl`, draws clouds and, with `SMOKE`
  defined, puffs (`clouds_render::scene_light` is the one light for both)
- [ ] 1.2 `Layer::Puffs` in `cb-core::fx`, validated (test: `a_puffs_layer_is_validated`)
- [ ] 1.3 Smoke material, puff pool, life (tests: `a_puff_grows_rises_and_fades_over_its_life`,
  `a_puff_takes_a_lower_step_as_it_fades_and_none_when_gone`)
- [ ] 2.1 Puffs in place of sprite smoke
  - [ ] Missile trails and the jets' trail, laid by distance (`fx.ron` `puffs`; tests:
    `a_trail_lays_a_puff_every_spacing_across_steps`, `a_fast_step_still_lays_every_puff_and_a_jump_starts_afresh`,
    `a_trail_whose_puffs_are_narrower_than_their_spacing_is_refused`)
  - [ ] Explosion smoke as a `Puffs` layer in the explosion recipes
  - [ ] Skate thrust in the air
- [ ] 3.1 Stills and a recorded battle; city combat benchmark against the build before
