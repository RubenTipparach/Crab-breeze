# Design: rocket-plumes

## 1. Today

- `fx.rs` streams `rocket_trail` from each rocket, `trail_spacing_m` (3 m) apart: one Trail particle (1.1 m disc, 12 s) and one Flame dot (0.8 m, 0.06 s).
- `mech.rs` `jet_flames` streams `jet_flame` at 60 bursts a second from each nozzle while the jets fire.

## 2. The plume

- One mesh (`plume`, a 16-sided cone 1 m long and 1 m across at its open end, its tip at the nozzle, built once in code from numbers) and one material (`PlumeMaterial`, additive, unlit, no shadows, `assets/shaders/plume.wesl`).
- Shader:
  - core brightness `exp(-r^2 / 0.04)` along the axis;
  - an fbm noise scrolling down the cone at `40 m/s` times the thrust;
  - colour from white (1.0, 0.95, 0.85) at the nozzle through yellow (1.0, 0.75, 0.3) to orange (1.0, 0.35, 0.08), emissive 12 to 2 times;
  - a jet's plume adds shock diamonds, `0.5 + 0.5 cos(2 pi x / 0.18)` along its first 40 %;
  - the far end fades out.
- Each rocket's plume is 4 m long and 0.8 m across. A jet's is 3 to 7 m, from its throttle, and 1.2 m across. Per-instance length and thrust come through the transform's scale and a per-instance colour, so all plumes share one material and batch.

## 3. The trail

- `trail_spacing_m` falls to 0.6 m, and the trail layer becomes:
  - smoke from the `fx-sprites` sheet, 0.6 m growing at 0.5 m/s, 8 s;
  - an ember Flame, 0.4 m, 0.15 s.
- Grey runs from warm (0.8, 0.75, 0.7) to cool (0.6, 0.62, 0.65).
- Puffs drift with the wind (`world.wind_m_s`, already used by the clouds).
- At 300 m/s a rocket lays 500 puffs a second. Ten rockets in flight at once is 5 000 a second at 8 s each: over the 5 000 cap. So the trail is held to `trail_max_per_rocket` (400) live puffs: the oldest go first, and the spacing widens past 200 m from the eye.

## 4. Light and dust

- A point light rides each rocket: 2e6 lumens, 40 m range, warm. It shares the 8-light cap, so the nearest 4 rockets get one.
- Jets under 15 m above the ground play `jet_wash_dirt` (or `jet_wash_concrete`) every 0.1 s at the point under the mech: a flat ring of dust thrown out at 12 m/s.

## 5. Costs

| What | Cost |
| --- | --- |
| Draws | 1 for every plume (batched) |
| Fragments | Additive cones, each a few thousand pixels at range. A jet plume in the cockpit's lower view is about 60 000 px |
| Particles | Up to 400 a rocket, under the 5 000 cap |
| Lights | Within the existing 8 |
| CPU | One transform a plume a frame |

Estimates. A cloud session measures nothing.

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| X4 | Rocket smoke: thick white (solid motor) or thin grey? | Thick white, as Ace Combat's missiles |
| X5 | Do the mech's jets leave a smoke trail too? | No, a clear hot exhaust only |
