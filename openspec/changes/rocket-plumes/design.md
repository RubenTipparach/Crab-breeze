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
- Each rocket's plume is 4.5 m long and 0.45 m across, its core at `plume_glow` (0.35) of the look's brightness,
  its motor light 50 000 lm and its launch light 2.5e6 lm out to 30 m (owner, 2026-10-29: "missile.jmpact was way
  too bright. The radius should be like 1/2 that and brightness toned way down..same with missile.plume": half the
  width, about a third of the glow, a quarter of the light). The blasts a missile makes (`explosion_*`) went the same
  way: fireballs and shock rings half the radius, their light 1e8 lm out to 150 m (it was 4e8 lm to 300 m).
- (Before 2026-10-29: 4 m long and 0.8 m across.) A jet's is 3 to 7 m, from its throttle, and 1.2 m across. Per-instance length and thrust come through the transform's scale and a per-instance colour, so all plumes share one material and batch.

## 3. The trail

- `trail_spacing_m` falls to 1 m, and the trail layer becomes:
  - smoke from the `fx-sprites` sheet, 1 m growing as it hangs, 6 s;
  - an ember Flame, 0.4 m, 0.15 s.
- Grey runs from warm (0.8, 0.75, 0.7) to cool (0.6, 0.62, 0.65).
- Puffs drift with the wind (`world.wind_m_s`, already used by the clouds).
- At 240 m/s a rocket lays 240 puffs a second, 1 440 alive over 6 s. The trail is a stream effect whose capacity (rate times life times 1.4, about 2 000) is the per-rocket cap: a full trail stops laying until its oldest puffs die.

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
