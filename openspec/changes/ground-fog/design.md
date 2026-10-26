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

## 2a. Aerial haze

Owner, 2026-10-26, with Ace Combat 8 shots of a coast and a fleet from altitude: "we also have more
distance fog affecting at higher altitudes", "hieght fog". The layer above is 120 m deep: from a
few hundred metres up the eye looks down through almost none of it, and far land and sea stay as
sharp and saturated as near ones. Real air has a second, far thinner and deeper layer of haze
(aerosol, a scale height of about 1-2 km) that greys and blues everything with distance at every
height. It is added to the same closed form, its own `rho1` and `H1`:

`tau_haze = rho1 * H1 * exp(-(e.y - h0) / H1) * (1 - exp(-L * d.y / H1)) / d.y`

- `haze_density_per_m` 1/45,000 m at sea level (a level ray loses about 60 % by 40 km), and
  `haze_scale_height_m` 1,500 m: from 1.5 km up, the coast 10 km away is a little blued, the far
  island and the horizon soft, as in the references.
- The same colour as the fog (the sky's light scattered back, the sun's forward), not tied to the
  mist map: haze does not pool in valleys.
- Cost: one more exponential a pixel in the fog pass.

## 3. The mist map

A 256 x 256 R8 texture over the 41 km world (160 m a texel), baked with the world plan from the
core's rules (`cb_core::world::mist`, pure and tested):

- **Hollows**: ground below the average of a ring of eight points 1.5 km round it, by the depth
  (a full hollow is 100 m deep).
- **Water**: lakes, rivers and wetland add up to 2 x within 300 m; the coast 1 x within 600 m.
- **Height**: nothing above 600 m (mist is low).
- Then a **preset** scales it (survey F1, built 2026-10-26): `morning` (rho0 x 2, mist x 1), `day`
  (rho0 x 1, mist x 0.5), `clear` (rho0 x 0.5, no mist). As built the day's half applies to all
  the mist, not to the hollows alone: the map holds one number a cell, so the hollows' share of it
  is not kept apart. Section 3a says which preset is in force.

As built (2026-10-25): `mist_at = low x (0.6 hollow + 0.5 near water + 0.3 near coast)`, `low`
falling to 0 between 360 m and 600 m; the density is `1 + 5 x` the mist averaged over the taps.
The rain haze is uniform while the eye is under a shaft (`rain_per_m` x the shaft's rain), not the
ray's length inside each shaft's cylinder: the shafts themselves are drawn as billboards.

The pass reads it with four taps along the ray (at 10, 30, 60 and 100 % of the way, capped at 6
km), so near mist and far valleys both show.

## 3a. Morning mist, thinning by day (survey F1)

Owner, 2026-10-26 ("Approve all", then "Go and implement"): F1, morning mist in the valleys and
the crater, thinning by day, and clear for combat-heavy missions.

- **The game has no time of day**: the sun stands still. So the fog keeps its own clock, data in
  `fog.clock`: the world begins at 06:00 (`start_h`), an hour passes every 300 s (`hours_per_s`
  1 / 300), and between 07:00 and 11:00 (`burn_off_h`), 5 to 25 minutes in, the `morning`
  strength eases (smoothstep) to the `day` one, where it stays. The clock is not wrapped: the
  mist does not come back at night, because there is no night to see it in. When a sky clock is
  built, `fog.clock` should read it instead of keeping its own (one clock, section 6.1).
- **A mission's own preset**: `fog.missions` maps a mission to a preset that holds while it runs,
  whatever the clock says. The defence and the air fortress, the two combat-heavy missions, fly in
  `clear` air. A career job and free roam with no mission running keep the clock's.
- **The crater**: the crater island is gone (`docs/design/world-look-plan.md`), so there is no
  crater floor to fill today. A crater, if one comes back, is a hollow, and the hollow rule fills
  it with no new code.
- `cb_core::world::mist::FogConfig::strength(mission, hour)` is the rule (tests
  `the_morning_mist_thins_by_day_on_the_clock`, `combat_heavy_missions_fly_in_clear_air`).
  `cb-app`'s `ground_fog.rs` runs the clock in the fixed step and keeps the strength in `FogNow`;
  the compose pass scales its density and mist strength by it, and so does the targeting
  (section 7), so what the pilot sees and what the sensors see are one fog.

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

## 6a. No ray march over the island (survey F3)

Owner, 2026-10-26 ("Approve all", then "Go and implement"): F3, no ray-marched volumetric fog on
the island; it is allowed later only for small local volumes.

- The island's fog is the analytic layer of section 2 and nothing else. Bevy's `VolumetricFog`
  (a ray march a pixel, 1-3 ms by the estimate in section 1, worse in the cockpit's wide view) is
  not put on any camera, and no `FogVolume` covers the island.
- Later, a small local volume may use it: steam when the cockpit opens, spray at a waterfall, each
  its own effect and its own change, measured (CLAUDE.md section 7) before it ships, bounded to a
  few tens of metres.
- A test keeps it so: `ground_fog::tests::no_ray_marched_fog_lies_over_the_island` fails on any
  use of the two in `cb-app`'s source, unless the line says `local fog volume (F3)`.

## 7. Fog hides targets (survey F2)

Owner, 2026-10-26 ("Approve all", then "Go and implement"): F2, ground fog also shortens radar
and lock-on range. This was not the recommendation (which was: fog hides from the eye only, as
cloud does, and the sensors see through it), so it changes the targeting.

- **The fog along the line**: `cb_core::world::mist::line_depth(cfg, strength, map, sea, eye,
  target, rain)` is the compose pass's own per-pixel fog (`fog_depth` in `water_post.wesl`: four
  mist taps at 10, 30, 60 and 100 % of the way out to `mist_taps_m`, averaged, thickening the
  layer's exact integral, the rain's haze at the eye) for the straight line from the pilot's eye to
  the target, at the strength F1 sets. So the fog the sensors see through is the fog drawn, at the
  same preset and on the same clock.
- **The threshold and the scale** (`data/hud/targeting.ron`): with `T = exp(-depth)` the share of
  the target's light that reaches the eye, the fog does nothing while `T >= fog_clear_share` (0.6).
  Below it the scale is `fog_min_scale + (1 - fog_min_scale) * T / fog_clear_share`, down to
  `fog_min_scale` (0.25) when nothing gets through. `cb_core::targeting::fog_scale` is the rule.
- **What it shortens**: the boxing range (`max_range_m`, which is the game's radar until a radar
  page is built: everything boxed is what the sensors hold), the lock range (`lock_range_m`), and
  the lock's build rate (a lock builds `scale` times as fast). `targeting::seen` and
  `targeting::in_lock_range` are the only range checks; the cockpit's boxes, the civil CIV boxes,
  the `IN RANGE` label and the lock all go through them, so they agree. Multi-lock (merged after
  this was written) takes the same rule: `in_lock_range` is the shoulder missile's
  `Seeker::in_range`, `Seeker::reaches` and `Seeker::keeps` take the fog toward the target, each
  `Target` carries its `fog`, and a pod's `Locks` build `fog` times as fast, for the pilot's pod and
  an enemy's pod locking on the pilot alike (`a_pod_takes_the_fog_rule_the_shoulder_missile_does`).
- **What it means in play**: in plain air the fog never reaches the threshold inside the 2 km
  boxing range (morning air at sea level lets 72 % through at 2 km). In full valley mist at dawn,
  about 37 % gets through 1 km, so a crab in the mist locks from about 900 m, not 1,200 m, and
  takes a quarter longer to lock. Under `clear` (the defence and the fortress) nothing changes.
- **Cost**: four bilinear map reads and two exponentials a target a fixed step, for at most the
  targets within 2 km: negligible next to the step (not measured; a cloud of 100 targets is 400
  reads).
- `CB_NO_GROUND_FOG=1` turns the drawing off but not the rule: it is a benchmark switch, and the
  simulation does not change with what is drawn.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| F1 | How much fog by default | morning mist in the valleys and the crater, thinning by day; clear for combat-heavy missions | decided, built (owner, 2026-10-26: "Approve all"; section 3a) |
| F2 | Does fog hide targets | from the eye only, as cloud does (W28); radar sees through it | decided otherwise, built: fog shortens the boxing and lock ranges and slows the lock (owner, 2026-10-26: "Approve all"; section 7) |
| F3 | Ray-marched volumetric fog | not now; Bevy's `VolumetricFog` later only for local volumes (steam at boarding, waterfall spray) | decided, recorded and guarded by a test (owner, 2026-10-26: "Approve all"; section 6a) |
