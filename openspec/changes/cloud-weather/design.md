# Design: cloud-weather

Heights are above sea level. The island mockup (`docs/mockups/island_gen.js`, the `weather` block)
is the reference implementation of every rule below; the game ports it to `cb_core::world::weather`
with the tuning in `data/world/world.ron` under `weather:`.

## 1. Billboards, weighed

| Option | Looks | Cost | Taken |
| --- | --- | --- | --- |
| Lit blob meshes (`cloud-puffs`, built) | soft cumulus, faceted up close, all alike | one blended layer a blob, about 300,000 triangles in reach | replaced |
| The old sprite stacks | soft | seven large quads a puff: the overdraw the owner turned off | no |
| **One billboard a puff, eroded edge, sphere lighting, sorted** | soft, billowed edges, any shape a system needs (towers, anvils, decks, caps) | one quad a puff; blended layers only where puffs overlap | **yes** (owner, 2026-10-24) |
| Ray-marched storm towers at quarter resolution | best up close | 0.5-1 ms for the towers alone | fallback, survey W25 |

**A billboard.** A quad facing the eye, `rx` across and `ry` up. Seen from above, a flat puff
should look round, not squashed, so its height on screen is `mix(ry, rx, k^2)`, where `k` is the
vertical component of the direction to the eye. In the fragment:
- `d = |uv|`; discard past 1.
- The edge is eroded by noise in the sprite's plane: `dens = smoothstep(1, 0.45, d + (fbm(2.2 uv +
  seed) - 0.5) e + (noise(7 uv) - 0.5) 0.2)`, with `e` 0.62 (0.22 for the smooth lenticular caps).
- Normal: the sphere's, `right x + up y + back (sqrt(1 - d^2) + 0.15)`, nudged by the same noise.
- Lighting is `cloud-puffs` section 3 unchanged (wrap 0.6, silver 1.6), except:
  - `h` is the pixel's height through its system's layer, not the 600-1,100 m layer;
  - base shade goes from 0.4 to 0.85 and albedo from 0.92 to 0.55 with the puff's darkness (0
    white, 1 storm grey).
- Alpha is `dens` times the kind's opacity times the near fade (140 m).

**Soft edges against the ground.** A billboard that dips into a hillside ends in a hard straight
line where the ground cuts it; the mockup shows it with the orographic cloud. The game fades each
billboard by the depth behind it ("soft particles"): `alpha *= smoothstep(0, 60 m, scene_depth -
billboard_depth)`. That costs one read of the depth prepass, which Bevy already has. The mockup has no
depth prepass, so it keeps the orographic puffs clear of the ground beneath them instead.

**Sorting.** Puffs blend, so they are drawn back to front. The draw radius's puffs are re-sorted
when the eye has moved 30 m since the last sort.

## 2. Weather systems (seed 42 counts from the mockup)

The wind is from the west, 6 m/s (`clouds.wind_m_s` as today). Everything below drifts with it.

| System | Where | Heights | Puffs | Darkness |
| --- | --- | --- | --- | --- |
| Fair cumulus | clusters on a 2 km grid over land and sea; per tile `0.4 + 2.6 m` clusters over land (`m` the moisture of `island-biomes`), 1.1 at sea, none under a storm; a quarter of wet clusters grow to towering cumulus with tops of 2.2-3.6 km | base 600 m | 4,224 | 0.1, towering 0.3 |
| Storm cells | 3 cells: over the western jungle, at sea off the archipelago, and a smaller one at sea to the north-west; radius 1.9-3.2 km | 700 m to 6.5-10.5 km | 643 tower, 210 anvil, 72 scud | 0.75 at the base to 0.15 at the top |
| Anvil | 70 wide flat puffs (1.3-2.2 km across, 0.26-0.42 km thick) at 86-94 % of the tower's top, 2.6 km downwind | 5.6-9.9 km | in the above | 0.15 |
| Stratocumulus deck | over the sea in the 275-345 degree sector, 14.5-21.5 km out, with breaks where a 4 km noise is low | 700-1,000 m | about 840 | 0.25-0.45 |
| Orographic | against the range's windward (west) flank, 600-3,200 m from the spine, on ground above 500 m | ground + 120-380 m, up to 1.8 km | 455 | 0.2 |
| Lenticular caps | over up to 5 peaks above 1,900 m, 2.5 km apart, three stacked lenses | peak + 280-500 m | 6 at seed 42 | 0.05 |
| Cirrus | 90 streaks, 5-14 km long, 0.5-1.4 km wide | 8.2-9.4 km | flat quads, not puffs | |

**Rain shafts.** Under each storm, three billboards turned about the vertical, `0.6-0.9 x` the
cell's radius, from the sea to the base (720 m): streaks scrolling down, thick in the middle and
thin at the edges. Inside one, below its top, the glass takes rain at 0.7 density (`cloud-rain`).

**Storm darkening.** Ground, water and buildings under a cell take `1 - 0.6` of the sun at the
centre, back to full sun at the cell's radius (1.6 times the tower's), its edge wandering by a
1.5 km noise. One loop of three cells in the ground shader: about 30 instructions a pixel.

**The weather map** (owner, again, 2026-10-24, after the first version: the systems were there
but from above they drowned in evenly spread fair cumulus):
- **A cold front** off the west coast, lying south-west to north-east, 52 km long and 8.4 km
  wide: layered grey cloud from 900 m to 2.8 km, thickest and darkest along its middle, towers to
  6.2 km every 6 km, a rain shaft every 5 km (2.2 km across), and the ground and sea darkened along
  it. No fair cumulus within 1.5 km of it.
- **A high** over the sea to the south-east: fair cumulus thins to nothing within 6-17 km of its
  centre, so that sky is clear.
- **Cloud streets** over the southern sea: clusters snapped to rows 2.2 km apart, stretched along
  the wind.
- **Fair cumulus** over the land by moisture as before, now in three sizes: small flat humilis
  (60-170 m, 45 %), medium, and towering congestus (28 % of wet clusters).
- **Altocumulus:** a field of small flat puffs (150-260 m across, 45-70 m thick) at 3.3-3.7 km over
  the south-west, with gaps, at 0.6 opacity.
- The stratocumulus deck moves to the north, behind the front.
- **No square** (owner, 2026-10-25: "don't generate them in like a square type thing... we can have
  some clouds scatter outside the map"): cumulus covers a disc out to 34 km from the centre,
  thinning from 24 km with a 2.5 km noise on the edge, so it runs past the map over the sea; the
  front and the deck are not clipped to the map; cirrus lies on a 32 km disc. Seed 42: 7,397
  puffs.

Seed 42: 5,960 puffs (2,550 cumulus, 711 tower, 210 anvil, 1,169 deck, front and scud, 461
orographic, 6 caps, 853 altocumulus), 11 rain shafts, 7 darkened cells.

## 2a. In the game (built 2026-10-25)

The weather was first built on the crater island the game had then, and moved back to the
mockup's island the same day when the game took the mockup's land (`island-biomes`; owner:
"I wanted this island"). Everything below is in `data/world/world.ron` under `weather:`.

| System | Mockup | Game |
| --- | --- | --- |
| Storm cells | over the jungle 6.5 km west of the spine, off the archipelago, north-west at sea | the same three: (-5.9, -4.2), (-15.5, 9.5) and (-11, -14) km (x east, z south) |
| Orographic | the spine's west flank, 600-3,200 m out, ground above 500 m | any ground above 500 m that rises 60 m within 900 m downwind (the windward flank, wherever the range is), up to 520 puffs |
| Caps | peaks above 1,900 m | the same, found by trying places over the island |
| Fair cumulus | moisture of the mockup | the game's biome moisture (the mockup's rule, `Biomes::moisture`) |
| The deck's breaks | the mockup's fbm below -0.25 | the game's value noise (-1..1) below -0.45 |
| Wind | from the west, 6 m/s | the same (`clouds.wind_m_s`, was 3, 1) |

Seed 42 (`world::tests::seed_42_census_for_the_design`), on the mockup's island: 7,609 puffs
(3,784 cumulus, 812 tower, 210 anvil, 1,481 deck, front and scud, 520 orographic, 9 in three caps,
793 altocumulus), 14 rain shafts, 8 darkened cells, 90 cirrus.

**Drawn.** One mesh of every item within 20 km of the eye (not 9 km: storm towers and the front
should stand on the horizon, and a far billboard is a few pixels), sorted on the CPU, re-gathered
when the eye has moved 30 m through the drifting sky; rain shafts and cirrus are items of the same
mesh, so it is one draw, not three. One material, `assets/shaders/cloud.wesl`.

**The storm darkening is a texture on the sun**, not a loop in each shader: Bevy 0.20's
`DirectionalLightTexture` (a light cookie, feature `pbr_light_textures`) projects a 1024 px R8 map
along the sun's rays. Each texel holds `Clouds::sun_at` where its ray crosses the storms' base
(700 m), so the shade falls where a real storm's shadow would; the wind carries it by moving the
sun's translation, which nothing but the texture reads (the shadow cascades take only its
rotation). The ground, buildings, trees, grass and mechs all darken through Bevy's own lighting.
The map covers 80 km across the light; one bake, about 1 M lookups at start (logged), again only
if the sun turns. The water's light is its own (`water_light`) and does not read it yet.

Checked in the lavapipe stills: a debug checkerboard in place of the map darkened the ground in
4 km squares, and seed 42's map holds 102/255 (0.4) at the western cell, as `sun_at` gives. At the
mockup's 60 % the shade read weakly, because the sky's ambient light is unchanged under the storm,
so the cut under a cell is 85 % (under the front 60 %): a cumulonimbus blocks almost all direct sun.
The ambient under a storm staying bright is the next thing to darken.

## 3. Cost (expected; measured with `CB_NO_CLOUDS` on the owner's hardware)

| Item | Fair weather | Facing a storm |
| --- | --- | --- |
| Puffs in the draw radius (20 km as built, section 2a) | about 3,000 | about 4,000 |
| Draws | 1 (puffs, shafts and cirrus in one sorted mesh) | 1 |
| Blended layers a sky pixel | about 0.6 | up to 6 across 40 % of the screen |
| Fragment work at 1920 x 1080 | about 0.6 million shades, under 0.1 ms on an RTX 3070 | about 5 million, about 0.5 ms |
| CPU | a gather and sort of 3,000-4,000 items and a mesh rebuilt (about 1 MB) when the eye moves 30 m: about 1 ms, so 10 times a second at the jets' 300 m/s, to be measured | the same |
| Triangles | 2 a puff: about 8,000, against about 100,000 in view for the blobs | |
| Storm shade | one 1 MB R8 texture on the sun, sampled once a lit pixel by Bevy's light texture path | |

The fragment cost is the one to watch: a storm seen from inside its anvil is the worst case. The
sort can drop to per-tile order (tiles far to near, puffs sorted only in the three nearest tiles)
if it shows on the CPU.

## 4. What the game needs from it

- `Clouds::density_at` reads every kind of puff, so flying into the deck or a tower fogs the
  view; under a shaft it reports rain.
- Every kind is data: counts, radii, heights, darkness and opacity by kind in `weather:`.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W17 | How clouds are made | one billboard a puff, sorted (owner: "billboards or a mix of some cheap raymarch") | decided (owner, 2026-10-24) |
| W25 | Storm towers up close | billboards; a quarter-resolution raymarch of the tower only if they look flat in a playtest | proposed (survey W25) |
| W26 | Which weather the island has | the six systems above, from the west | proposed (survey W26) |
| W27 | Weather over a mission | fixed per mission and drifting with the wind; growing and dying later | proposed (survey W27) |
| W28 | Weather in the fight | storms hide targets from the eye only; radar sees through them (Ace Combat) | proposed (survey W28) |
