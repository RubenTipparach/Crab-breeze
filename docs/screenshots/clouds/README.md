# Clouds in the game: lavapipe stills

Stills of the `cloud-puffs` clouds (lumpy lit blobs at 600-1100 m) and the attempt at the rain on the
glass (`cloud-rain`), taken 2026-10-24 in a cloud session: the release build, Mesa's software Vulkan
(lavapipe) under Xvfb, 1920 x 1080. **These are lavapipe images with no frame time:** the software
renderer draws about one frame a second, which says nothing about the game on the owner's RTX 3070.
The PNGs are reduced to 256 colours to keep them small.

How they were made: `target/release/crab-breeze --stills output/stills`, whose capture instrument
(`crates/cb-app/src/capture.rs`) gained three beats for this: after the stride it stops and looks up
through the front screens (`sky.png`), then rockets up and skates toward the nearest cloud, the way
the world showcase does (it now shares `nearest_cloud` and `fly_to_cloud` with
`crates/cb-app/src/showcase.rs`), shooting once 400 m up (`climb.png`), once within 700 m of the
cloud (`cloud-near.png`), and once after 2.5 s of game time inside the cloud or when the beat gives up
(`cloud-rain.png`). The beats write input actions only. The instrument also had a bug: it shot the
street during the loading screen, before `GAME_READY`. It now waits for it.
`sun-take.png` is a screen grab from `--showcase sun`.

| Still | What it shows | What is wrong |
|---|---|---|
| `sky.png` | Seated, stopped in the street, looking up through the front screens | Only towers and clear sky: the city hides the clouds from the street. |
| `climb.png` | 412 m up on the rockets, heading west | The clouds show only at the right edge, as dim blue-grey lumps over the sea. **Most of the view is dark slabs**: in flight the screen wall shows dark geometry, not the outside. It needs a look on the owner's machine, whether it is cockpit or hull geometry in front of the eye or the screens' feed stalling under a software renderer. |
| `cloud-near.png` | 686 m up, skating and rocketing toward the cloud | The same dark slabs over most of the view. A band of clouds shows at the left over the coast, flat blue-grey at this distance, with no lit tops visible. |
| `cloud-rain.png` | The end of the flight beat | **No rain on the glass:** the mech never got into a cloud. The rockets ran empty at 686 m ("EMPTY, recharging"), short of the cloud. The beat gave up after 0.0 s inside. With the 20 s rocket limit, the take needs a cloud that is lower or closer, or a climb that recharges on the way. |
| `sun-take.png` | `--showcase sun`: on foot in the street, the view turning up toward the sun | Under lavapipe the take's beats, timed in real seconds, end before the turn finishes: towers fill the frame, with one small distant cloud at the bottom right. |

`--showcase world` was run too, with a screen grab every 20 s. It completed (`PLAYTEST_RESULT ok`),
with 11 warnings about despawning entities that were already gone. Its beats are timed in real seconds
and the simulation runs at about a fifth of real time here, so it never reached its cloud (at 647 m).
Two grabs are kept:

| Still | What it shows | What is wrong |
|---|---|---|
| `world-rockets.png` | Climbing out of the city on the rockets: clouds over the forest to the left | The dark slabs again cover the right half. The clouds are flat blue-grey shapes with no lit tops. |
| `world-over-the-wood.png` | Gliding down toward the river landing: the coast, the wood, a road, a bank of cloud above | The clearest view of the clouds: lumpy, grey and blue, darker than the sky. The dark slabs cover the left half. |

**The dark slabs are in every airborne frame of both runs**, at different headings, and in none on the
ground. They look like geometry between the eye and the screens when the mech flies, not a one-off
frame.

After the clouds were built the owner said (2026-10-24): "I prefer clouds to be billboards or a mix of
some cheap raymarch". That is recorded against W17 in `openspec/changes/cloud-puffs/design.md`. These
stills are the blobs that design will replace.
