---
name: city-roads
description: Build and change Crab Breeze's cities and their roads the repository's way - the Fargo-like city (block uses, house and strip blocks filled by the lot placer), highways through cities on viaducts with exits (trunks, ramps, decks), roads over roads with every height query choosing its level by the body's feet, the lane graph along a highway, the drive check that proves a car can drive every road, levels of detail and blended joins where surfaces meet, the three.js city mockup and its own road check, and headless stills of a city. Use whenever making or changing a city's layout, a block's use, streets, highways, exits, interchanges, ramps, bridges, viaducts, embankments, overpasses, car parks, lanes or traffic routes, road materials, or asking whether cars can drive it ("city layout", "highway through the city", "exit", "interchange", "ramp", "bridge", "viaduct", "overpass", "road levels", "drive check", "lane graph", "Fargo", "city mockup", "car park material").
metadata:
  author: Crab Breeze (Claude Code), from openspec change city-districts (2026-11-04 to 2026-11-13)
  version: "1.0"
---

# Cities and roads

The design is `openspec/changes/city-districts` (design sections 4 and 5, tasks); this skill is how the work is done.
The owner's rules behind it, in their words: "how do cars drive if the roads dont connect?", "seems like a waste of
land", "buildings in the middle of the highway", "highways can cut through cities, can go over cities or under cities,
because of one simple thing: EXITS!" (under means a bypass, bridges and levees, never a tunnel), "make sure cars can
actually drive on these!", "you need a driving bot or some sort of fill system", "use different shading/materials for
parking lot vs roads vs highways", "with LODs ... and make sure the texturing is blended correctly", "the mockup doesnt
even try to merge textures and geometry together well".

## The pieces and where they live

| Piece | Code |
| --- | --- |
| Cities, streets, blocks, block uses (`BlockUse`), zoning, `build_city` | `crates/cb-core/src/world/layout.rs` (`zones:` in `world.ron`) |
| Houses and businesses in house and strip blocks (the suburbs' own placer: `Zone::City`) | `world/small_buildings/lots.rs` (`city_blocks`) |
| Highways: routing, grading, ends pinned to a trunk (`lay_highway_to`, `TrunkEnd`) | `world/roads.rs` |
| The trunk through a city, its row (`BlockUse::Under`), its exits' ramps | `world/trunk.rs` (`trunk:` in `world.ron`), `lay_trunks` in `world/mod.rs` |
| Roads over the land or over a road: viaducts, ramps, piers (`RoadDeck`, `Decks`) | `world/decks.rs` |
| Height queries: `road_top_at`, `walk_top_at`, `Scene::support_at`, `OnRoads` | `world/mod.rs` |
| Lanes, junctions, ramps' one-way lanes, lane heights | `world/lanes.rs`; traffic drives them in `cb-core/src/traffic.rs` |
| Drawing streets, blocks, highways, decks; road materials | `crates/cb-app/src/world.rs`; drives and car parks per ground node in `cb-app/src/paving.rs` |
| The city mockup and its road and drive check | `docs/artifacts/city-layouts/` (`city_layouts.html`, `check_roads.js`) |

## Rules

- **Mockup first, then the write-up, then code** (CLAUDE.md section 4). A new road shape is tried in
  `city_layouts.html` and passes its own drive check before it goes near the generator.
- **One implementation.** Lots in a city use the suburbs' placer and its drive rule; a ramp's deck uses the same
  `RoadDeck` as a viaduct; a crater cuts drives with `world::crater_cuts` like it cuts streets. Never a second copy.
- **Roads over roads are decks.** Wherever any of a road's width stands 0.3 m or more over the land (`VIADUCT_OVER_M`:
  on fill while low, on piers once there is headroom) or over another road, it is a `RoadDeck`; a point on the outside
  of a deck's bend is on it (only its two ends stop it). `road_top_at` leaves a highway out where it is a deck; `Decks::top_under` gives the surface
  within a step (`STEP_M`) of a body's feet; `Decks::push_out` keeps a body off a slab with less than `HEADROOM_M` over its
  feet. Cars, the pilot, the people and the mech all choose their level by their feet: never by "the highest road here".
- **Every lane carries its own heights** where it rides its road's surface (a highway's grade, a deck's top), not the
  ground under it, or traffic drops through a viaduct onto the street below.
- **Highways meet the city at exits.** A ramp is one-way, at most 8 % (`trunk.ramp_grade`), level for its last
  `ramp_landing_m`, and lands at a street junction; nothing passes under a ramp lower than 5.9 m (4.5 m of headroom under
  a 1.2 m slab), so its low part stays inside its block. No connector turns more than 90 degrees onto or off a ramp.
  A ramp joins its deck the way a real one does: it swings out (eased, no corner) until its edge meets the deck's edge
  at the deck's height, runs level beside it with no barrier between (`ramp_gore_m`), and only then climbs or descends,
  its height set by distance along the deck, never by the rounding of its curves. A highway joining a trunk runs onto
  it in line (`lead_in_m`), never at a corner.
- **Lanes are the traffic's.** A road's lanes, tyre tracks and patch repairs are laid on the traffic's own lane offsets
  (`LaneLayout`, `RoadSurfaceConfig::lane_bounds`): never a lane every 3.5 m across whatever width the road has. A city
  street is 14 m, one lane each way and a parking strip; an arterial 22 m, two lanes each way.
- **A downtown block is lots, not a wall.** Two or four big lots a block; a tower (a slim shaft with room round it) on
  some, more near the heart and in a large city; mid-rise on the rest; the heart's lot always the tallest.
- **Grids stay connected.** Every street joins the network; a street cut off by a structure is not built (in the mockup
  the drive check's own fill prunes it). No building on a street, a highway or inside an exit's footprint.
- **Car parks, streets and highways look apart.** Streets on Asphalt027C, car parks on the worn asphalt set
  (`Surface::Lot`, their own mesh), highways and decks on Concrete047A, section roads gravel.
- **Joins are welded and blended, not butted.** Textures are mapped in world space, so the same material on two roads
  meets with no seam: a ramp in the streets' asphalt runs into its street invisibly. Where two materials meet (a concrete
  deck and an asphalt ramp, an embankment and the grass), fade one over the other with a vertex-alpha strip on the same
  world-space UVs. Geometry meets exactly: a landing is level with its street, its edges on the street's edges, no step,
  no gap, no two surfaces in one plane (z-fighting).
- **Levels of detail for every structure.** Decks, piers, barriers and ramps come in three levels (detail near; boxes;
  the slab only far), cut into 512 m tiles, and say what they cost (triangles, draw calls). A tile's level is chosen
  by the eye's distance to its square (`deck_draw::DeckLod`): Bevy's `VisibilityRange` measures from the entity's
  origin, and meshes kept in world space (for the crater cut and the seamless mapping) sit at the origin, so with it they
  were never drawn.

## The drive check

A road network is done when a car can drive it. In the mockup, `driveCheck` (in `city_layouts.html`): every road
reachable from the downtown (a fill over 6 m samples; two roads join where one lies on the other within 0.5 m),
4.5 m under every road over another, no grade over 8 %, turns of at least 400 m on a highway and 25 m on a ramp, every
ramp joined at both ends, no pier in a lane, and a bot that drives 12 trips. `node check_roads.js city_layouts.html
<dir with three/package>` runs it in every variation and fails on any problem. Prove a new check catches the bug it is
for: break the page on purpose (a ramp stopped short) and see it fail. In the game, the same rules are `cb-core` tests
over the generated plan and lane graph.

## Seeing it

Headless stills of a city in a cloud session (lavapipe, about 1 frame a second):

```sh
cargo build --release -p cb-app
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_WEATHER=1 CB_HOUR=11 CB_STILLS_SETTLE_S=50 \
  CB_STILLS_ONLY=fargo-air,fargo-houses,fargo-strip,fargo-downtown,fargo-highway,fargo-street \
  xvfb-run -a -s "-screen 0 1920x1080x24" ./target/release/crab-breeze --stills output/stills-fargo
```

Settle 50 s or the houses show as their far shells. Check the run every five minutes (CLAUDE.md section 9). The views
are `fargo_view` in `crates/cb-app/src/capture.rs`; add a view there for a new structure. `cb-tools cities` prints each
city's centre and size. A nuclear crash on a city: `CB_STILLS_NUKE=1 CB_NO_TREES=1 CB_NUKE_AT=x,z --stills <dir>`.
A still is not a playtest: the owner's machine signs it off.

## Before a push

`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --release` (the world
tests take about 20 minutes), `openspec validate --all`, the dash check, and `check_roads.js` if the mockup changed.
Re-pin the golden world only for a change meant to move it, and say why beside the new numbers.
