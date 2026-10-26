# Design: rain

## 1. How much rain, where (`cb-core::weather`, pure)

`rain_at(clouds, eye, setting) -> f32` in 0..1: inside a rain shaft of `Clouds` (`cloud-weather`), 1 at the shaft's
axis falling to 0 at its rim (smoothstep over the outer 30 %), 0 above the storm's base. With Rain everywhere, 1
everywhere below the cloud base. The value eases toward the target over `fade_s` (3 s). One function feeds the
streaks, the splashes, the lens and the sound.

## 2. The streaks (`cb-app::rain`)

- **One instanced draw.** `count` (20,000 at full rain) streaks in a cylinder round the eye, radius `radius_m`
  (60 m), height `height_m` (40 m), wrapped around the eye in the vertex shader. Each streak is a camera-facing
  quad of `length_m` (0.9 m) by `width_m` (0.012 m), stretched along its fall velocity (`fall_m_s` 9) plus the wind,
  and for a mech the eye's own motion, so rain streaks past at speed.
- **No CPU work per drop.** Each instance has a seed. Its position is `seed + velocity * time`, wrapped in the
  cylinder. The CPU writes one uniform a frame (eye, time, wind, amount); instances beyond `amount * count` are
  discarded in the vertex shader.
- Pale, blended, lit by the sky's ambient and the sun at a low weight; depth-tested against the scene, so buildings
  and the cockpit hide it.

## 3. Splashes

A ring of `splash_count` (600) small quads on the ground within 25 m: each reappears at a fresh random point when
its 0.3 s splash ends. Its height is the terrain's, or the water's surface (`Waters::surface_at`). Same shader
family, same single draw.

## 4. Data

`data/world/rain.ron`: `count`, `radius_m`, `height_m`, `length_m`, `width_m`, `fall_m_s`, `wind_m_s` (x, z),
`splash_count`, `fade_s`, `lens_max` (0.8), `sound_db`, validated on load.

## 5. Cost

One instanced draw of up to 20,000 quads (40,000 triangles) and one of 600: under 0.3 ms GPU on the RTX 3070 at
1920 x 1080 (blended streaks that are 1-3 px wide cover little of the screen). CPU: one uniform a frame. Measured
in a storm with `perf_suite.py` before it is called done.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| RN1 | Where it rains by default | under the storm cells only, with Rain everywhere as an option | provisional |
| RN2 | Do the screens show rain drops on their lenses when seated | yes, as the cockpit-camera glass already does for cloud | provisional |
