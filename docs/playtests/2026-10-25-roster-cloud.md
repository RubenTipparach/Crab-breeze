# The roster at the Foreman's standard: cloud stills, 2026-10-25

**This is not a playtest.** These are headless stills from a cloud session: Mesa's software Vulkan (lavapipe) under
Xvfb, 4 CPU cores and no GPU, with a 4096 px cockpit bake running beside the game for most of it. Frames took
2 to 7 s each. **No frame times are claimed here** (CLAUDE.md sections 7 and 9). No OBS, no Drive upload. The
playtest belongs on the owner's machine (`./run.sh --mech mechs/ironhorse.ron`, `mule`, `harrow`, and the rest).

- Branch `claude/great-keller-ng77hm`, with PR #27 (`game-loop`), PR #26 and the music branch merged in.
- Command: `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json xvfb-run -a -s "-screen 0 1920x1080x24"
  ./target/release/crab-breeze --mech mechs/<id>.ron --stills output/stills-<id>`, seed 42.
- Stills (1280 px): `docs/screenshots/roster/`.

## What the stills show

| Still | Shows |
| --- | --- |
| `ironhorse-seat.jpg` | The Excavator cab from the seat after `cockpit-parity`: three MFDs hung over an unbroken front screen, the dash with its three live screens under the glare shield, no rail across the view. Compare `docs/screenshots/cockpit-smear-fix/cab-ironhorse.jpg` (before: no dash, a beam through the middle of the view) |
| `ironhorse-outline.jpg` | Free look held, looking down at the dash's centre screen: the outline round it (`mfd-menus`). Shot at 4 times white, which washed it to cream; it is now 1.4 times, to stay amber |
| `ironhorse-hatch-open.jpg` | The cab open from below: the ramp down, the dash riding it as the step |
| `ironhorse-street.jpg` | The Ironhorse standing in the street before boarding |
| `foreman-seat.jpg`, `foreman-stride.jpg` | The Foreman, the standard: all six screens each showing one whole page (the five-page fix, below) |
| `harrow-seat.jpg` | The Canopy from the seat after `cockpit-parity`: the three new MFDs hung above the front screen and clear of it, the dash's three live screens below. Compare `docs/screenshots/cockpit-smear-fix/canopy-harrow.jpg` (before: no cabin in view, the pages in a strip over the bottom of the view) |
| `harrow-outline.jpg` | The outline on the Canopy's dash centre screen at the new glow (1.4 times white) |
| `harrow-up.jpg` | A Blender render of the baked Canopy from the eye, looking up at its new MFDs (their baked pictures; in the game they show the live pages) |

## What was found and fixed on the way

- **The MFD pages were laid out three wide on a five-page texture** (since `weapon-select`): Bevy's flex layout
  shrank five 512 px pages into 1 536 px, so each screen's slice straddled two pages. The page count is now one
  constant (`hud::MFD_PAGES`) both the layout and the screens read; the Foreman's stride still shows one whole page
  per screen.
- **Particle streams had no `EffectProperties`**, which `bevy_hanabi` documents as required when the effect declares
  a property (ours declares `size_scale`). Bursts carried them; streams (jet flames and trails, burning wrecks,
  rocket trails, now the boarding vents) did not. They now do.
- **Two stream parents had no `Visibility`** (Bevy warning B0004): the jet trail's and, at first, the vents'.
- **`cockpit_layer` hid any mesh under the mech it did not know**, which would have hidden the MFD outline as hull
  while sealed; a mesh that sets its own layers is marked `OwnLayers` and left alone.
- **The stills instrument shot the cockpit on the wall clock**: at 5 s a frame (0.25 s of game) it caught the chest
  still closing. It now waits on the boarding clock (the hatch sealed) and a frame count.

## What these stills cannot show

- **The boarding steam and mist.** The vents' particles did not show in any of these stills, even with every vent
  forced on for a test, while burst particles do draw on lavapipe (the dirt, fire and dust of earlier cloud stills).
  Why the continuous streams do not show here is not established. The vents are checked another way: the log shows them switching on the
  boarding clock exactly as designed (on the Ironhorse: 4 streams during the kneel, 7 as the seals break, then the
  sill's and the cabin's mist held while open), the data test checks every roster mech vents from its seals and
  sill, and `cb-tools validate` checks every vent's node exists in the mech or its cabin. Seeing them is for the
  owner's machine, as the rocket trails (the same stream path) were seen in the 2026-10-24 defence take.
- **Frame cost** of anything (cloud sessions measure nothing).

## Checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | passes |
| `cargo clippy --all-targets -- -D warnings` | passes |
| `cargo test --workspace` | passes (see the commit) |
| `cargo run -p cb-tools -- validate data/` | passes: every mech, every cockpit's six screens, eye, seat, lamps and ridden parts, every generated boarding's nodes |
| `openspec validate --all` | passes |
| Dash check | passes |
