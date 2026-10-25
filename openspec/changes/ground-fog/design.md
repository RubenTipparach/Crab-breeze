# Design: ground-fog

Heights are above sea level. Densities are per metre (extinction, 1/m).

## 1. Options weighed

| Option | Looks | Cost at 1920 x 1080 on an RTX 3070 (estimated, to be measured) | Taken |
| --- | --- | --- | --- |
| Bevy's `DistanceFog` (flat, by distance) | one colour by distance; covers the sky; no height | none extra | kept only for inside cloud and under water, as now |
| Bevy's `VolumetricFog` with a `FogVolume` over the island | true volumetric: sun shafts, shadows in the fog | a ray march a pixel at reduced resolution, 1-3 ms, worse in the cockpit's wide view | no (perhaps later for local volumes) |
| **Analytic height fog in the compose pass, scaled by a mist map, plus rain haze** | thick low, thin high, filling valleys; lit by sky and sun; no shafts | about 0.1-0.2 ms: one depth read, an exponential integral and 4 mist taps a pixel | **yes** |

## 2. The fog

Density at a point `p`: `rho(p) = m(p.xz) * rho0 * exp(-(p.y - h0) / H)`, with `h0` sea level,
`H` the scale height (default 120 m), `rho0` the density at sea level (default 1/12,000 m, so
level ground fades to half at about 8 km) and `m` the mist map (1 in plain air, up to 6 in a valley
at dawn).

Along a ray from the eye `e` to a pixel at `x` (length `L`, direction `d`), with `m` taken as its
average `m_bar` over 4 taps along the ray, the optical depth has the closed form

`tau = m_bar * rho0 * H * exp(-(e.y - h0) / H) * (1 - exp(-L * d.y / H)) / d.y`

(and `m_bar * rho0 * exp(-(e.y - h0)/H) * L` when `d.y` is near zero). Transmittance is
`exp(-tau)`; the pixel becomes `colour * T + fog_colour * (1 - T)`. Sky pixels (depth 0) take
`L` = 40 km, so the horizon gets the low layer and the sky overhead stays clear.

**The fog's colour**: `ambient * a + sun * shade * HG(dot(d, s), g = 0.6) * b`, where `HG` is the
Henyey-Greenstein phase, `shade` the storm shade the sun carries (`Clouds::sun_at`, the same map),
`a` and `b` data. Toward the sun it glows; away from it it is the sky's blue-grey.

**Rain haze**: for the (at most 16) rain shafts within 20 km, the ray's length inside each
shaft's cylinder (below its top) adds `rho_rain` (default 1/1,500 m) of density, coloured grey.
Standing in a shaft the world fades out by about 3 km.

## 3. The mist map

A 512 x 512 R8 texture over the 41 km world (80 m a texel), baked once at start on a worker thread
from the core's rules (`cb_core::world::mist`, pure and tested):

- **Hollows**: ground below the average of its surroundings within 1.5 km, by the depth (a
  valley 100 m deep gets about 3 x).
- **Water**: lakes, rivers and wetland add up to 2 x within 300 m; the coast 1 x within 600 m.
- **The crater floor**: inside the rim's cliff band, 2 x.
- **Height**: nothing above 600 m (mist is low).
- Then a **preset** scales it: `morning` (all of the above, rho0 x 2), `day` (rho0 x 1, hollows x
  0.5), `clear` (rho0 x 0.5, no mist).

The pass reads it with four taps along the ray (at 10, 30, 60 and 100 % of the way, capped at 6
km), so near mist and far valleys both show.

## 4. In the cockpit

The compose pass already skips the sealed cabin (`cabin_dry_radius_m`); the fog does the same, so
the screens show the world fogged and the cockpit interior stays clear. The fog is in the pilot's
view and in the screen cameras alike, since the screens show the outside.

## 5. Cost

| Item | Cost |
| --- | --- |
| The compose pass | today it runs only while the eye or the glass is wet; with fog on it runs every frame: one full-screen pass, about 0.1 ms at 1080p on a 3070 for the fog's math (estimate) |
| Mist map | 256 KB, baked once (about 50 ms on a worker thread) |
| CPU | the uniform each frame; the shafts within 20 km (at most 16) |

Measured with `CB_NO_GROUND_FOG` against the same take on the owner's hardware, in the cockpit, in
the densest scene (CLAUDE.md section 7).

## 6. What the game needs from it

- `cb_core::world::mist::mist_at(p)` is the one mist rule: the map is baked from it, and anything
  that needs to know how foggy it is at a place (a sensor, an AI's sight) asks it, not the texture.
- The fog's numbers (`rho0`, `H`, the presets, the rain haze) are data in `data/world/world.ron`
  under `fog:`.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| F1 | How much fog by default | morning mist in the valleys and the crater, thinning by day; clear for combat-heavy missions | proposed (survey F1) |
| F2 | Does fog hide targets | from the eye only, as cloud does (W28); radar sees through it | proposed (survey F2) |
| F3 | Ray-marched volumetric fog | not now; Bevy's `VolumetricFog` later only for local volumes (steam at boarding, waterfall spray) | proposed (survey F3) |
