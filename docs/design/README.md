# Crab Breeze design hub

## Survey

Open questions for the owner live in the survey page, never in chat (CLAUDE.md section 8):
https://claude.ai/artifact/EbeWFXQeCbcM7ok3tsp64h (source: `docs/design/survey.html`; answers in
its `answers` collection, read with `ArtifactData`). Keep editing this one rather than starting
another. It replaced https://claude.ai/artifact/KndrEC5xE7wBmAd8WM4aTP on 2026-10-25 (owner: "make a new one"),
which was published from an account the cloud sessions cannot open; every answer given there is
written into the page's Answered section, so nothing was lost. Its new database started empty.

Question id prefixes: `G` game and feel, `M` the mech, `C` the cockpit, `W` world (cities,
buildings, people), `T` tech and tools, `P` playtesting, `A` audio and music.

## Changes

| Change | State | Presents |
| --- | --- | --- |
| `playtest-pipeline` | proposed | The scripted pilot, output lines, OBS recording, Drive upload, reports |
| `cargo-workspace` | built | Rust workspace on Bevy 0.20.0: `cb-core`, `cb-app`, `cb-tools`; seeded RNG; data loader |
| `procgen-world` | built, playtest pending | Seeded world: hills, cities with street grids, box buildings 2-60 storeys, highways |
| `mech-rig` | built, playtest pending | Stand-in mech on the rig contract, digitigrade walk, torso twist, glb drop-in |
| `mech-boarding` | proposed | The sealed cockpit and its all-view screen, the mech sitting down, the staged hatch with steam and mist |
| `pilot-boarding` | built, playtest pending | Pilot on foot in first person, boarding up the lift into `EYE_pilot`, climbing out |
| `cockpit-parity` | built, playtest pending | Every roster cockpit up to the Foreman's: the Cab gets the dash and its row seam out of the view, the Canopy its hanging MFDs, both re-baked; six live MFD screens in every cockpit, checked by `cb-tools validate` |
| `boarding-vents` | built, playtest pending | Every mech steams and mists as it kneels and opens: the seals, hinges, sill, cabin and leg rams, from its own models' `VENT_*` empties |
| `mfd-menus` | outline built; menus proposed | In free look the MFD under the crosshair is outlined; clicking any MFD opens its menu: readouts, MAP, MAIL, SHOP and the game menu |
| `mech-shop` | proposed | The SHOP page on any MFD (and the base terminal): every roster mech for sale, its sheet from its own data, bought through the career's ledger, delivered to a free bay |
| `signature-weapons` | proposed | One unique weapon per roster mech on `weapon-kinds`: boom cannon, cutting lance, twin racks, steam cannon, rocket ripple, rail rifle, breaching gun, siege mortar |
| `terrain-water` | built, in the 2026-10-24 world take | Rivers and lakes from the seed, wading drag, ripples, spray and steam |
| `terrain-materials` | proposed | PBR ground layers splatted by slope, water, city and scorch (textures from the art side) |
| `vegetation` | built (procedural trees until models arrive) | Seeded woods, instanced LODs, mechs snapping trees that fall under Avian physics |
| `terrain-deformation` | built | Craters from explosions, dirt bursts, scorch; one ground query |
| `mech-damage` | built | Parts shot off one by one: arms, head (backup cameras), legs (limp) |
| `audio` | proposed | Sound effects: cues in data, rocket rumble, sound delayed by distance, the muffled cockpit |
| `weapon-kinds` | proposed | Shotguns, railguns, lasers and plasma cannons in the one projectile path; a weapon heat bank; fire group 4 for back units |
| `railgun-scope` | proposed | The scope as a 4x/10x window on the front screen; the pilot's view never zooms |
| `multi-lock` | proposed | Hold F to sweep up to six locks, release for a salvo; today's missile is the one-lock case |
| `music` | proposed (motifs, six cues and the pipeline made) | The score played by game state, switching on the bar; motifs in `music.md`, scores in `music/` |
| `clouds` | built (no shadows or raindrops yet) | Cheap fly-through cumulus, in-cloud fog, cloud shadows; camera raindrops later |

| `jet-flight` | built | Jets limited by heat, landing on roofs, flame; for flying into clouds |
| `combat-fx` | built, in the 2026-10-24 defence take | Effects as recipes in `data/fx/fx.ron`: one streak per shot, muzzle stars and flashes, impacts by surface, layered explosions (fireball, shock, burning debris, smoke column), rocket trails |
| `rockets` | built, in the 2026-10-24 defence take | Dumbfire rockets: straight flight, no drop, led without drop; 3 in 10 raiders carry a pod |
| `targeting` | built, in the 2026-10-24 defence take | Ace Combat targeting: boxes with range, the selected target, lead marker, edge arrow; missile lock in range (1 200 m) and lock-on missiles on F |
| `crab-airship` | mockup; first build in the game (section 9) | The Stratostacean air fortress: 420 m, eight lift thrusters on leg pylons (it falls when a side is down to one), flak, beams, deck guns, missiles, fighters off two catapults, crabs dropped on chutes, a crab deck guard; land on it to recharge and walk a pylon to a thruster's intake |
| `free-roam-missions` | proposed | Free roam's mail (M) starts the missions, the city defence and the Stratostacean; starting one turns off the one running; a career's second job abandons the first |
| `city-defence` | built, won on take 2026-10-24 (too easy: retune next) | Three waves of 10 raiders on three cities; lost past 50 % city damage, 10 min a wave, or the mech; `--mission`, `--showcase`, `--bench defence` |

| `perf-bench` | built | `--bench world|fight`: offscreen 1080p frame time by beat; baseline in docs/benchmarks |
| `weapons` | built | Autocannon and cannon, ballistic shots, hits by part |
| `enemy-mechs` | built | AI pilots through the same controls; the city fight take |
| `building-destruction` | built | Buildings of cells that break and collapse |
| `debris-physics` | built | Avian 0.8 for chunks and wrecks on the drawn ground |
| `building-generation` | proposed, mockup | Facades, storeys, setbacks and roof clutter from a split grammar on the 6 m grid; four 1980s archetypes |
| `building-interiors` | proposed, mockup | Floorplans round a core, flats, offices, shops; furniture placed by interior rules; walkable on foot |
| `structural-destruction` | proposed, mockup | Replaces the cells: a structural graph where load decides what gives, creaks, pancakes; breaks cost no remesh |
| `rubble` | proposed, mockup | Rubble as ground (the crater layer), angle of repose, pieces settling on top, dust |

Requests between the Mac and cloud sessions: `docs/design/requests.md`.

Design notes: `music.md` (the score: key, the spine, the stride, every motif in notes, the cues and the rules for new music), `cockpit.md` (the cockpit, the owner's reviews, the four detailed from block-outs and how the chest
opens as one body with them, open questions), `cockpit-blockouts.md` (the ten cockpit block-outs with screenshots of
each opening), `mech-references.md` (the mech pictures the owner showed, what to take from them, and the two new mechs
built on it), `mech-rig.md`, `alien-rig.md`.

## Mockups

| Page | Published | Built by |
| --- | --- | --- |
| Crab Breeze Sounds (`docs/mockups/sounds.html`): the first 36 sound effects (mech, weapons, impacts, cockpit), synthesised placeholders, each with a waveform strip and a play button | https://claude.ai/artifact/6XyiVnCGRqbLXq6qPjSXhg | `tools/audio/render_sfx.sh` (generator `tools/audio/gen_sfx.py`), then `python3 tools/audio/gen_sfx_page.py` (the sounds are embedded in the page) |
| Crab Breeze Music (`docs/mockups/music.html`): the five cues (moody rebel cyberpunk) to play, every motif as a piano roll drawn from the score with its notes and meaning, the musical world | https://claude.ai/artifact/17Lj37SLPpsARPqE9ZfV9C | `tools/music/render.sh` for each `music/<cue>.strudel`, then published with `output/music/*.mp3` as files (skill `music-authoring`) |
| Mech Cockpits (`docs/mockups/cockpits.html`): the detailed Panorama, Titan, Excavator cab and Canopy, the first three built interiors (wall, harness, jockey) and the ten seated block-outs, each opening by the front or the back in stages (scrub with the slider): sit at the pilot's eye with the world on the screens, or orbit the pod beside a pilot. Was the Breeze Cockpit page | https://claude.ai/artifact/AqMpKdiBjn9e1uQPT1c1aj | `tools/mockups/gen_cockpit_page.py` from `assets/models/cockpit_<variant>/`, published with them as files |
| Crab Breeze Mechs (`docs/mockups/mech_designs.html`): on top, the 24 m low-poly redesign (`tools/blender/build_mech_blockouts.py`, legs against torso measured), the working mechs numbered 1 to 12 and, in their own viewer, ten combat mechs built from Armored Core style parts (13 to 22: legs, core, head, hand weapons, back units, boosters); below, the detailed 12 m round: Foreman (Panorama), Stevedore (harness) and Ironhorse (Excavator cab) at true scale; each kneels and opens its chest with steam (the Stevedore drops its ladder; the Foreman and Ironhorse, opening as one body with their cockpits, fold their front flaps down flush with the floor and leave the doorway clear); board it in first person or sit in each cockpit | https://claude.ai/artifact/B5tX8ycUR2tLBknKyMb8bc | `tools/mockups/gen_mech_designs.py` from `assets/models/mech_<id>.glb` and `assets/models/cockpit_<variant>/`, published with them as files |
| Crab Breeze Buildings (`docs/mockups/buildings.html`): a seeded city block with facades, roofs, floorplans and furniture; click to fire, demolition scripts, stress view, falling storeys and rubble piles; the pilot and a 12 m mech for scale. Research in `docs/analysis/buildings-research.md` | https://claude.ai/artifact/FhoXXPFzGMvrmqea9SYaRS | hand-written page; shots by `tools/mockups/shoot_buildings.mjs` into `docs/screenshots/buildings/` |
| Cockpit MFD Shop (`docs/mockups/mfd_shop.html`): three dash MFDs in free look: point at one for the amber outline, click for its menu (readouts, MAP, MAIL, SHOP, the game menu), open a mech's sheet and buy it with an example career (`mfd-menus`, `mech-shop`) | https://claude.ai/artifact/2A6KyHEvVbSD1MBVK3Ys3P | hand-written page |
| Alien Crab Mechs (`docs/mockups/alien_mechs.html`): the three enemy crab machines (Skitter 6 m, Snapper 12 m, Big Boy 30 m) at true scale beside the Breeze | https://claude.ai/artifact/DAdKyPWjNyjur74vQaZTA8 | `tools/mockups/gen_alien_mechs.py` from `assets/models/alien_*.glb` (`tools/blender/build_alien_mechs.py`, rig in `docs/design/alien-rig.md`) |
| Stratostacean Air Fortress (`docs/mockups/crab_fortress.html`, source `tools/mockups/crab_fortress_src.html`): shoot the crab air fortress down. Click to fire your mech's cannon, autocannon or missiles at any part, from your view or from your cockpit (armour, weak points and hit points per the design); the lift thrusters are plated outboard, so you get under it, and when a side fails it opens six hull thrusters whose ports only take hits from straight above or below; painted orange, dark orange and grey with red turrets, the mechs' way, its armour designed for this hull (a belt with pylon root plates, port doublers, side plates with deliberate diagonals, walls on the hull's stations; flush plates, fine seams, T-junctions only); guard crabs walking on IK, limp under their chutes; two allied mechs flying round it under fire; fighters launch off the catapults and make runs on you, the turrets track and fire (flak, beams, deck guns, missiles), crab mechs drop on chutes and guard the deck, replaced up the hangar lifts; the lift board runs the design's lift rule until it falls and crashes on the city. Block-out at true scale with the Foreman, a pilot and the crab block-outs | https://claude.ai/artifact/GkXpHR9BGs8qJs8v6zF1tt | `tools/mockups/gen_crab_fortress.py`; shots by `tools/mockups/shoot_fortress.mjs` into `docs/screenshots/fortress/` |
| Crab Breeze Island (`docs/mockups/island.html`, rules in `island_gen.js`): the 41 km island from seed 42 at true scale, with the range, the desert and canyons, jungle, swamps, rivers from mountain tarns through valley lakes, beaches, cliffs, archipelago, rock formations, trees, cities and highways, and billboard clouds in weather systems; orbit or fly, with rain on the glass inside a cloud, a biome map and a 24 m mech for scale | https://claude.ai/artifact/NyHSzekHMnyspZyUrP5rTx | `docs/mockups/island.html` with `island_gen.js` published beside it |
| Crab Breeze Main Street (`docs/mockups/small_buildings.html`, parts in `small_buildings_kit.js`): sixteen 1980s houses (the suburban four in three variants each, in the row behind) and small businesses on one street with real windows, interiors, signs and cars; walk at the pilot's or mech's eye, day, dusk or night, cutaway | https://claude.ai/artifact/NsAB4HQMjoPd8Nazq9vTcW | `docs/mockups/small_buildings.html` with `small_buildings_kit.js` published beside it |
