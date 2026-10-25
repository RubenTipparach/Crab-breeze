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

Seed 42: 5,960 puffs (2,550 cumulus, 711 tower, 210 anvil, 1,169 deck, front and scud, 461
orographic, 6 caps, 853 altocumulus), 11 rain shafts, 7 darkened cells.

## 3. Cost (expected; measured with `CB_NO_CLOUDS` on the owner's hardware)

| Item | Fair weather | Facing a storm |
| --- | --- | --- |
| Puffs in the 9 km draw radius | about 1,200 | about 2,000 |
| Draws | 3 (puffs, shafts, cirrus) | 3 |
| Blended layers a sky pixel | about 0.6 | up to 6 across 40 % of the screen |
| Fragment work at 1920 x 1080 | about 0.6 million shades, under 0.1 ms on an RTX 3070 | about 5 million, about 0.5 ms |
| CPU | a sort of 1,200-2,000 keys when the eye moves 30 m: about 0.2 ms | the same |
| Triangles | 2 a puff: about 4,000, against about 100,000 in view for the blobs | |

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
