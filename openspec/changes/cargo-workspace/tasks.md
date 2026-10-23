# Tasks: cargo-workspace

- [ ] 1.1 Workspace `Cargo.toml` with `bevy = "=0.20.0"`, crates `cb-core`, `cb-app`, `cb-tools`
- [ ] 1.2 `SimRng` with `fork` (tests: `the_same_seed_gives_the_same_stream`, `forking_does_not_advance_the_parent`, `different_labels_give_different_streams`)
- [ ] 1.3 `load_ron` with unknown-key and non-finite rejection (tests: `an_unknown_key_is_an_error_naming_the_file`, `a_missing_field_takes_the_default`, `a_non_finite_number_is_rejected`)
- [ ] 1.4 `cb-tools validate data/` loads every data file through `load_ron`
