# Skill provenance

Where each skill in this directory came from, so a later copy from the source can be compared
(CLAUDE.md sections 6.5 and 13). All sources are the owner's repositories under
`RubenTipparach/`, read on 2026-10-23.

| Skill | Source | Revision | Copied | Changes here |
| --- | --- | --- | --- | --- |
| `openspec-*`, `../commands/opsx/*` | OpenSpec CLI 1.14.0 (`openspec init --tools claude`); `openspec-verify-change` and `opsx/verify.md` from Star-Crew-Bridge-Simulator, the same CLI version | Star Crew `8b0bbf1` | 2026-10-23 | None. Tool-owned: `openspec update` rewrites them. |
| `html-survey` (with `template.html`) | rubens-solitare-collection `.claude/skills/html-survey`, the owner's question system | `0ef4ade` | 2026-10-23 | An "In Crab Breeze" note after the front matter; the rest verbatim. |
| `obs-record` (with `scripts/obs_record.py`) | godot-sandbox (Mining Mike) `.claude/skills/obs-record`, itself from Pale-Blue-Dot | `3336b88` | 2026-10-23 | An "In Crab Breeze" note with this game's recipes and the Drive folder; the rest verbatim. Tom Lander's copy (`4a3bfe1`) has a retrying restore after a long take that Mining Mike's lacks; port it if a long take fails to restore OBS. |
| `blender-hard-surface` | Star-Crew-Bridge-Simulator `.claude/skills/blender-hard-surface` | `8b0bbf1` | 2026-10-23 | Description retargeted; an "In Crab Breeze" note (high detail, LODs, PBR, the cockpit set); the rest verbatim. |
| `blender-humanoid-characters` | Star-Crew-Bridge-Simulator, itself from fps-game-demo (Undercity) `f6cd25c` | `8b0bbf1` | 2026-10-23 | An "In Crab Breeze" note; the rest verbatim, Star Crew's note included. |
| `blender-csg-levels` | Star-Crew-Bridge-Simulator, itself from fps-game-demo (Undercity) `f6cd25c` | `8b0bbf1` | 2026-10-23 | Description widened to buildings and city blocks; an "In Crab Breeze" note; the rest verbatim. |
| `material-maker` | Star-Crew-Bridge-Simulator `.claude/skills/material-maker` | `8b0bbf1` | 2026-10-23 | Description retargeted; an "In Crab Breeze" note (full PBR maps); the rest verbatim. |
| `material-maker-headless` | Star-Crew-Bridge-Simulator (shared by the owner's repositories) | `8b0bbf1` | 2026-10-23 | "In this repository" rewritten for Crab Breeze; the shared part verbatim. |
| `threejs-mockups` | Star-Crew-Bridge-Simulator `.claude/skills/threejs-mockups` | `8b0bbf1` | 2026-10-23 | Description retargeted; an "In Crab Breeze" note (Mining Mike's orbit-at-true-scale rule); the rest verbatim. |
| `panel-textures` | Star-Crew-Bridge-Simulator `.claude/skills/panel-textures`, the owner's "textures skill" | `8b0bbf1` | 2026-10-24 | Description retargeted to the cockpit; an "In Crab Breeze" note (full PBR maps, the cockpit kit and panels); the rest verbatim. |
| `light-baking` | Star-Crew-Bridge-Simulator `.claude/skills/light-baking` | `8b0bbf1` | 2026-10-24 | Description retargeted; an "In Crab Breeze" note (texture atlases, Cycles bake, unlit export); the rest verbatim. |
| `playtest` | New in Crab Breeze, after Star Crew's `sc-client --playtest` (`crates/sc-client/src/playtest.rs`), Mining Mike's CLAUDE.md ("Playtest Runs Report Every Five Minutes", "One game at a time per device") and Tom Lander's `bot-run` (Drive upload of a share copy) | Star Crew `8b0bbf1`, godot-sandbox `3336b88`, tom-lander `4a3bfe1` | 2026-10-23 | No code copied. |
| `perf-measure` | New in Crab Breeze, after Pale-Blue-Dot's and Mining Mike's `perf-measure` (their rules; their tools drive their own games) | Pale-Blue-Dot `db25fe2`, godot-sandbox `3336b88` | 2026-10-23 | Rewritten for Bevy (diagnostics plugins, Tracy). |
| `clean-code` | New in Crab Breeze, after Falchion's `solid-principles`, Star Crew's CLAUDE.md section 6, Pale-Blue-Dot's "Implementation and verification" and Mining the Sky's "Don'ts" | Falchion `5d128af`, Star Crew `8b0bbf1`, Pale-Blue-Dot `db25fe2`, mining-the-sky `51464c0` | 2026-10-23 | SOLID rewritten from TypeScript to Rust and Bevy. |
| `bevy-architecture` | New in Crab Breeze, after Falchion's `modular-game-systems` and Pale-Blue-Dot's core and app crates | Falchion `5d128af`, Pale-Blue-Dot `db25fe2` | 2026-10-23 | |
| `deterministic-simulation` | Falchion `.claude/skills/deterministic-simulation`, rewritten for Rust and Bevy | `5d128af` | 2026-10-23 | |
| `testing-practices` | Falchion `.claude/skills/testing-practices`, rewritten for Rust and Bevy | `5d128af` | 2026-10-23 | |

Not copied, and why:
- (`light-baking` and `panel-textures` were left out on 2026-10-23 as serving Star Crew's low-poly
  look; the owner asked for them on 2026-10-24 for the cockpit, so they are copied above with notes.)
- Star Crew's and Pale-Blue-Dot's `owner-survey`: `html-survey` is the portable version of the same
  workflow, and the owner's latest word is HTML surveys.
- Tactical Airsuite's `balatro-pixel-art`, `blender-card-art`, `hand-drawn-illustration` and
  Falchion's `hand-drawn-art`, `ship-materials`, `thumbnails`, `procedural-geometry`: 2D or
  three.js art for other styles. Falchion's winding and normals rule is in `blender-hard-surface`'s
  checks already.
- Mining Mike's `campaign-saves`, Tom Lander's `bot-run`: tied to those games' tools; their lessons
  are in `playtest`.
