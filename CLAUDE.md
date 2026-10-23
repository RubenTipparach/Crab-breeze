# Crab Breeze: rules for Claude

Crab Breeze is a mech simulation, played entirely in first person. The pilot walks the world on
foot, climbs into their mech, and fights it from the cockpit. Buildings, cities, people and the
mech are modelled in high detail. The stack is Rust and Bevy.

These rules are binding. When a rule here conflicts with a general habit, a design doc or a tool's
default, this file wins. They were adapted from the owner's other projects (Star Crew Bridge
Simulator, Pale Blue Dot, Mining Mike in `godot-sandbox`, Mining the Sky, Falchion, Tom Lander,
the Undercity `fps-game-demo` and Ruben's Solitaire Collection), and every rule taken from there
says so. `docs/analysis/source-repos.md` records what was taken from each, at which revision.

> **THE LAW: first person, always, and the cockpit is the mech** (owner, 2026-10-23: "the game
> will be played entirely from fps view. So when the pilot is on the mech, they are in the
> cockpit"). The camera is the pilot's eyes. On foot it is their head; in the mech it is their
> head in the cockpit seat, with the canopy, frame, instruments and their own hands in view. There
> is no third-person camera, no chase camera, no external "mech view" and no cut to a menu or a
> black screen between walking and piloting: boarding is walked and climbed. Every playtest shows
> the game this way (section 9). It outranks every other rule, design and plan here.

**This file is the only copy of these rules.** `AGENTS.md` is a pointer to it and must stay one.
Write a rule here, never there. Two files that have to agree are two files that won't (Star Crew,
Pale Blue Dot). `openspec/config.yaml` points here for the same reason.

Contents:

1. Owner direction
2. The stack: Rust and Bevy
3. Spec-driven work: OpenSpec
4. Write it up before touching code
5. Writing style
6. Architecture and code quality
7. Performance
8. Working with the owner
9. Playtesting, recording and Google Drive
10. Art: modelling and texturing in high detail
11. Verification
12. Skills in this repository
13. Documented exceptions

---

## 1. Owner direction

From the owner's brief, 2026-10-23. Quotes are the owner's words.

- **The game.** "We're going to make the most detailed mech simulation ever made. We're going to
  model buildings, cities, people, and the mech in high detail."
- **Cockpits and first person.** "We need mech cockpits, the game will be played entirely from fps
  view. So when the pilot is on the mech, they are in the cockpit." (The law, above.)
- **Simulation, not arcade.** "Most detailed mech simulation" means the mech's systems are
  simulated, not faked: what the cockpit shows is the number the simulation uses (section 6.1).
  The systems simulated, and how deeply, are designed in OpenSpec changes before they are built.
- **Playtesting comes first.** "The most important thing is playtesting, obs recording and
  uploading to Google drive." Section 9.
- **Foundations before the game.** "I would like you to establish all these things before we
  start our game": clean code, SOLID principles, an optimisation framework, modelling and
  texturing skills, playtesting, OBS recording and Drive uploads, OpenSpec
  (https://openspec.dev) and the owner's survey system for answering questions.
- **Bevy** (owner, 2026-10-23: "our tech stack should be bevy", "because bevy is awesome").
  Section 2.
- **Ace Combat is the inspiration** (owner, 2026-10-23: "Game will take inspiration from ace
  combat"). Fast, readable air and ground combat, lock-on missiles, set-piece missions against big
  targets, all seen from the cockpit (the law still holds: no chase camera). What exactly is taken
  from Ace Combat is a survey question (`G4`) and then an OpenSpec change.
- **Three ways to move** (owner, 2026-10-23: "Mechs will be able to take limited flight with
  powered jets or skate around on the ground"): walking, skating on the ground, and limited
  powered flight on jets. Flight is limited by a resource the simulation tracks (fuel, heat or
  both: survey `M3`), so the mech is a ground machine that can take to the air, not an aircraft.
  Each mode is simulated, and the cockpit shows the numbers the simulation uses (section 6.1).

## 2. The stack: Rust and Bevy

- **Rust, stable toolchain, and Bevy**, pinned to an exact version in the workspace `Cargo.toml`
  (`bevy = "=0.20.0"`, released 2026-10-08, when the workspace is created). A Bevy upgrade is its
  own change, with the migration guide read and a playtest after.
- **Read the docs for the pinned version.** Bevy's API moves every release. Check the release
  notes, the migration guides and docs.rs for the pinned version; never write Bevy code from memory
  of another version.
- **Pale Blue Dot is the reference Bevy workspace** (`pbd-core` for engine-independent rules,
  `pbd-app` for the Bevy adapter). Crab Breeze follows the same split (section 6.2).
- **Assets are glTF (`.glb`)** built by committed Blender generators (section 10) and loaded by
  Bevy's `AssetServer`. Materials are PBR (`StandardMaterial`) fed by Material Maker maps.
- **Desktop first.** Windows is the owner's playtest machine; Linux builds and runs in cloud
  sessions for tests and headless captures. A web build is not a target unless the owner adds it.

## 3. Spec-driven work: OpenSpec

Requirements and in-flight design live in `openspec/`, driven by the OpenSpec CLI
(`npm install -g @fission-ai/openspec@1.14.0`, Node 20.19+; https://openspec.dev). The
SessionStart hook installs it in cloud sessions. The split is the point (Pale Blue Dot, Star Crew):

- **`openspec/specs/<capability>/spec.md` is what the game DOES.** Every requirement in it is true
  today and pinned by a passing check: a unit test, a data validation, a measurement or a
  playtest. A requirement for behaviour the game doesn't have yet does not belong there.
- **`openspec/changes/<name>/` is what is designed but not built**: `proposal.md` (why, what
  changes), `design.md` (how, with numbers, formulas, costs and an `## Open questions` table),
  `tasks.md`, and spec deltas under `specs/<capability>/spec.md` with `## ADDED / MODIFIED /
  REMOVED Requirements`, `### Requirement:` headings, SHALL statements and `#### Scenario:`
  WHEN/THEN blocks.
- **`openspec/changes/archive/`** takes a change once its deltas are merged into the specs and its
  work is real.

| Command | Does |
|---|---|
| `/opsx:explore` | Think the problem through |
| `/opsx:propose` | Write the planning artifacts, then stop |
| `/opsx:apply` | Implement (a separate request) |
| `/opsx:verify` | Check the implementation against the change |
| `/opsx:archive` | Close the change when it lands |

- **Small, single-purpose changes.** One per capability, never an umbrella.
- **`openspec validate --all` passes before every push.**
- **Specs move with the code**: a requirement moves into `openspec/specs/` in the commit that makes
  it true and adds the check that proves it.

## 4. Write it up before touching code

Standing instruction in Pale Blue Dot and Star Crew, adopted here:
1. Investigate and measure.
2. Put the finding and the plan in `openspec/`: a proposal, a design and the spec deltas.
3. Stop there.

Editing source is a separate step, taken after the write-up exists and on a request to take it. A
change argued in a document can be redirected for the cost of reading it; the same change argued
in a diff has already been made.

- **One exception: a measurement instrument**, code whose only job is to produce a number the
  write-up needs. Say in the write-up what it measures first, and keep it to the instrument.
- **Mockups and documentation tooling are not game code**, and are built with the write-up.
- Do not rename or rescale a tuning value, alter a default, or refactor toward a plan before the
  plan is written down.

## 5. Writing style

**Never use em dashes (U+2014) or en dashes (U+2013), anywhere** (Undercity, Star Crew, Mining the
Sky): docs, comments, commit messages, PR bodies, in-game text, UI strings, logs and replies. Use a
colon, commas, parentheses, a period, or a plain hyphen for a range (`10-25 m`).

```sh
LC_ALL=C.UTF-8 grep -rnIP '\x{2014}|\x{2013}' --exclude-dir=.git --exclude-dir=.claude \
  --exclude-dir=target . && echo FAIL
```

- **Numbers carry units**, SI unless a table says otherwise: metres, seconds, kilograms, newtons,
  kilowatts, kelvin or degrees Celsius (say which).
- **Plain words, from the player's side** in anything the owner reads: what they will see or do,
  then how it works.

## 6. Architecture and code quality

The skills `clean-code`, `bevy-architecture`, `deterministic-simulation` and `testing-practices`
are the working detail of this section.

### 6.1 One implementation of every rule

**Avoid divergent code paths that share functionality at all costs** (Undercity's highest-priority
rule; Mining the Sky: "never rebuild an existing feature from scratch").
- Same behaviour, same code. Parameterise, don't clone. Search before writing anything new.
- **What the cockpit shows is computed by the code that resolves it.** A gauge, a lead indicator
  or a heat warning that disagrees with the outcome is the bug this rule prevents.
- Report duplication when you find it. Never add a third copy.

### 6.2 SOLID, and a simulation core with no engine in it

- **`cb-core` holds the simulation** (locomotion, power, heat, weapons, ballistics, damage, stable
  ids) with no rendering, windowing, audio or asset dependency. It runs headless and is tested
  without a GPU. **`cb-app` adapts it to Bevy**; the core never calls the renderer.
- **SOLID** (Falchion's skill, in Rust): one reason to change per module and per plugin; extension
  by registries, traits and data, never `match` arms spread across files; every `impl` honours its
  whole trait; small traits and narrow queries; dependencies (RNG, time step, data) passed in and
  wired in the composition root.
- **One plugin per capability**; simulation in `FixedUpdate`, presentation in `Update`; ordering by
  named system sets; cross-capability messages by events and observers.
- **Input is actions**, mapped from devices in data; the scripted pilot writes the same actions.

### 6.3 Determinism

Fixed steps in seconds, seeded and forked randomness, stable order (never hash or query order
deciding an outcome), stable ids, finite-value guards, golden-seed tests (Falchion, Star Crew).

### 6.4 Tuning lives in data

No inline tuning. Tuning and content live in `data/` (RON) with the unit in the key (`mass_kg`,
`stride_m`), validated on load. A missing field inherits the code default; a present zero is zero;
an unknown key is an error; a file that fails to load stops startup with its path and field.

### 6.5 Conventions

- Every file opens with a `//!` comment saying what it owns and why it lives there. Every public
  item is documented. Units in names or docs.
- Tests sit with the rule and read as sentences; a bug fix brings its regression test.
- Warnings are errors: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`.
- Guard the edges: reject non-finite numbers where they enter; check indices; an operation that
  does not fit a capacity changes nothing.
- Validate the real artifact: when two things must agree (Rust and WGSL layouts, a glb and its
  manifest), a check loads both, never a hand-written copy of the expected value.
- Borrowed code keeps its provenance (source repository and revision).

## 7. Performance

The `perf-measure` skill is the method. High detail is the point of the game, so cost is measured,
said out loud and kept in check, never guessed.
- **Targets** until the owner sets others: on the owner's most powerful machine at 1920 x 1080,
  p95 within 8.3 ms and p99 within 16.7 ms, in the cockpit, in the densest scene.
- **A claim is a measurement**: release build, real time, windowed, nothing else running, a scripted
  scene, old against new interleaved in one sitting, p50/p95/p99 and the spread reported. A
  difference smaller than the spread is not a result. Never call an unmeasured design a speedup
  (Pale Blue Dot, Mining Mike).
- **Every design says what it costs**: triangles, draw calls, texture memory, CPU time per fixed
  step, in plain numbers.
- **Every asset has a budget** per LOD in its manifest, and the build refuses one over budget.
- **Cloud sessions measure nothing**: no GPU. They say so in the PR and ask a session on the
  owner's hardware to measure.

## 8. Working with the owner

- **Ask with the survey, never in chat** (Star Crew, Pale Blue Dot, Tactical Airsuite). Open
  questions go into the survey page `docs/design/survey.html`, published as an HTML artifact whose
  database holds the answers, with options, a recommendation and a place to answer. The
  `html-survey` skill says how; the link is in `docs/design/README.md`. The reply gives the survey
  link and reminds the owner to fill it in.
- **Surveys are always HTML artifacts, never Claude Docs** (owner, in Tactical Airsuite,
  2026-10-21: "format surveys as html artifacts please, I actually really like those").
- **A blank answer is the recommendation accepted.** Build on recommendations while a question is
  open, and mark them provisional in the change's `## Open questions`.
- **End every reply with the artifact links** (owner rule in every repository): each artifact made
  or changed since the owner's last message, with one line on what is new, or one line saying
  there were none.
- **No scheduled PR check-ins** (owner rule in Pale Blue Dot, Star Crew and Mining Mike: "you
  should never rearm pr checkins"). No `send_later`, routine, `/loop` or timer that wakes the
  session to re-read a PR. A PR is looked at when the owner asks, when its events arrive, or as
  part of work in hand. The one exception is the five-minute check on a running desktop playtest
  (section 9).
- **Test on the owner's hardware when it is connected** (Star Crew, 2026-10-10). Before running
  the game in a cloud container, look for a connected session on the owner's machines
  (`list_sessions`). The most powerful device does the building, playing, recording and
  measuring; messages carry the ask, git carries the results.

## 9. Playtesting, recording and Google Drive

The owner's first priority. The `playtest` and `obs-record` skills are the method.
- **A feature is done when a playtest shows it in the running game**, not when a test passes or a
  mockup looks right.
- **A playtest is the game as it is played**: first person throughout, the walk to the mech and the
  climb into the cockpit included, nothing cut together (the law; Star Crew 2026-10-12: a take
  that cuts between views "DOES NOT MEET MY REQUIREMENTS").
- **The scripted pilot plays fair**: one scripted pilot sends the same input actions a player does,
  following beats in `data/playtests/<name>.ron`, and prints `PLAYTEST_START`, `PLAYTEST_RESULT`
  and `PLAYTEST_END` for the recorder (`openspec/changes/playtest-pipeline`).
- **Record with OBS** between `PLAYTEST_START` and `PLAYTEST_END`, in real time, with Game Capture,
  and **look at the contact sheet** before calling a take done.
- **Upload every take to Google Drive**: a small H.264 share copy (`-crf 26 -preset slow`) into the
  Drive for Desktop folder `G:/My Drive/dungeon-crawler-2026/` (the folder Tom Lander's and Mining
  Mike's videos use), named `crab-breeze-<YYYY-MM-DD>-<what>.mp4`. Confirm it with the Google Drive
  connector and give the owner the link. The full OBS take stays local.
- **One game per device** (Mining Mike): no two games, and no build or benchmark beside a take.
- **A desktop run is checked every five minutes** while it plays (Mining Mike: "this should be hard
  req for playtest runs on desktop"): screenshot, look, report in two or three lines, stop a run
  that is stuck.
- **Report each playtest** in `docs/playtests/<date>-<name>.md` with the commit, the machine, each
  beat's outcome, the Drive link and the contact sheet.
- **Cloud sessions cannot record.** They render headless stills through lavapipe and say so.

## 10. Art: modelling and texturing in high detail

The skills `blender-hard-surface`, `blender-humanoid-characters`, `blender-csg-levels`,
`material-maker`, `material-maker-headless` and `threejs-mockups` are the method. They come from
Star Crew and Undercity, each with an "In Crab Breeze" note on what changes.
- **High detail, not low poly.** This is the opposite of Star Crew's style: PBR materials (base
  colour, normal, occlusion-roughness-metallic, emission), normal maps baked from the full-detail
  model, real-time lighting and shadows.
- **Meshes are files built by committed generators** (star-crew-64, Undercity, Star Crew): a Blender
  script writes the source deterministically and exports a byte-reproducible glb with a manifest.
  Hand-sculpted sources, if the owner adds them, are committed with their export script.
- **Hard surface the CSG way** (Star Crew, owner 2026-10-05: "look at how to do hard surface
  modeling in blender with booleans and CSG"): primitives, named cutters through Exact booleans,
  bevels where light catches, cleaned, checked, exported.
- **Every hero asset ships LODs** with a triangle budget each, and the build refuses one over budget.
- **The cockpit is a set built to the pilot's eye.** It is real geometry on the mech's torso; every
  control the pilot touches is a named node the game can animate and hit-test; the camera sits at
  the seat's `EYE_pilot` empty.
- **Textures are Material Maker graphs** (Star Crew, owner 2026-10-05: "apply textures from our
  material maker skills"). A graph is the one implementation of how a material looks; one data file
  lists every material; a contact sheet is looked at before it is done. Never write a procedural
  image generator that imitates a graph.
- **Texture lines start on the 3D model** (Star Crew, 2026-10-10): panel seams, armour boundaries and
  markings are placed and checked on the model first, then baked into the UV atlas.
- **True relative scale** (Mining Mike: "orbit it, at true relative scale"). A model is reviewed in
  an orbitable viewer with the pilot (1.8 m) and the mech beside it, sizes measured, never estimated.
- **No z-fighting, ever** (Undercity). Two surfaces never share a plane while overlapping.
- **People** are built from a table with one rig and one shared animation library (Undercity).

## 11. Verification

Before claiming anything is done, run what applies:

| Check | Command |
|---|---|
| Specs | `openspec validate --all` |
| Dash check | Section 5 |
| Format and lint | `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` |
| Tests | `cargo test --workspace` |
| Data | `cargo run -p cb-tools -- validate data/` (when it exists) |
| In the game | a playtest (section 9), or headless stills in a cloud session |

- **Know what is proven.** Distinguish implemented, validated and proposed work in docs, PRs and
  replies. A design is not a feature, and a green test is not a visual sign-off.
- **Every major step ends in something the owner can check**: a playtest video on Drive, or
  captures in `docs/screenshots/` when the game cannot be played yet.

## 12. Skills in this repository

| Skill | Use |
|---|---|
| `openspec-*` and `/opsx:*` | The OpenSpec workflow (section 3). Tool-owned: `openspec update` rewrites them. |
| `html-survey` | The owner's question system: questions in the survey page, answers read back (section 8). |
| `playtest` | Running, watching, recording, uploading and reporting a playtest (section 9). |
| `obs-record` | Recording a window with OBS and copying it to Google Drive (section 9). |
| `perf-measure` | Measuring frame time and backing a performance claim (section 7). |
| `clean-code` | One implementation, SOLID in Rust and Bevy, conventions, review checklist (section 6). |
| `bevy-architecture` | Crates, plugins, schedules, the first-person camera, extension points (sections 2, 6). |
| `deterministic-simulation` | Fixed steps, seeded randomness, stable order, golden seeds (section 6.3). |
| `testing-practices` | What to test, how, and the checks before a push (section 11). |
| `blender-hard-surface` | The mech, the cockpit, vehicles and props in Blender, the CSG way (section 10). |
| `blender-humanoid-characters` | Pilots, crews and civilians from a table, rigged (section 10). |
| `blender-csg-levels` | Buildings and city blocks with brushes and cutters (section 10). |
| `material-maker` | Making and judging materials from Material Maker graphs (section 10). |
| `material-maker-headless` | Running Material Maker with no GPU in a cloud session (section 10). |
| `threejs-mockups` | Orbitable model and cockpit mockups as artifact pages (section 10). |

Provenance for every skill is in `.claude/skills/PROVENANCE.md`.

## 13. Documented exceptions

Nothing is an exception until it is listed here with its reason.
- **Tool-owned files keep their tool's text.** The OpenSpec CLI writes `.claude/skills/openspec-*`
  and `.claude/commands/opsx/*`. Don't hand-edit them. The dash check excludes `.claude/`.
- **Copied skills keep their source's text** below their "In Crab Breeze" note (and its front
  matter description, retargeted), so a later copy from the source can be diffed. Where the source
  and the note disagree, the note and this file win.
