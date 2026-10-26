# Tasks: railgun-scope

Built 2026-10-26 (owner, "Approve all", then "Go and implement"; survey G24 decided). The rules are tested in
`cb-core`; the window itself is built, compiled and its systems initialise without overlapping queries (`the_scope_systems_queries_do_not_overlap`), but it has not been seen running.

- [x] 1.1 `scope_zoom` on `WeaponSpec`; `scope` and `scope_zoom` bindings (test: `a_scope_zoom_of_one_or_less_is_refused`)
- [x] 1.2 `ScopeCamera` on `CAM_main` along the aim ray, alive only while Scope is held; the window quad on the front screen (`cb-app::scope`)
- [x] 1.3 Mouse look scaled by `1 / zoom` while scoped (tests: `scoped_look_moves_a_quarter_as_far_at_4x`, `four_times_narrows_an_85_degree_view_to_25_point_8`)
- [ ] 1.4 The sight: range from the beam trace, drop mark from `lead`, heat and the charge as numbers, `NO CAM` with the head lost (built; test: `the_drop_mark_at_2_km_is_3_point_4_m_up_and_the_slug_lands_on_it`); the charge as a ring (not yet)
- [ ] 2.1 Measure on the owner's machine: the bench's `fight` scene scoped against unscoped, windowed, p50/p95/p99
- [ ] 2.2 Playtest: a railgun Brigand hitting a target at 1.5 km through the scope, recorded and uploaded
