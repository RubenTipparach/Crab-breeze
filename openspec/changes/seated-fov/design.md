# Design: seated-fov

## 1. The two views

| | Field of view (vertical) | Across 16:9 |
| --- | --- | --- |
| On foot | `on_foot_fov_deg`, 75 degrees | about 106 degrees |
| Seated | `view_fov_deg`, 95 degrees (the Options setting) | about 125 degrees |

With the Options setting at S degrees: seated S, on foot S - (95 - 75), never under 30.

## 2. The change

`cb_core::pilot::fov_blend(blend, seated, dt_s, change_s)` steps a 0-1 blend toward 1 while seated, 0 otherwise, at
1 / `change_s` a second; `fov_deg(on_foot, seated, blend)` mixes the two with a smoothstep of the blend, so the view
eases in and out. Pure, tested. `cb-app::pilot::ease_fov` runs every frame, seated meaning `PilotMode::Seated`, and
sets the pilot camera's and the cockpit pass's lens.

## 3. Cost

One smoothstep a frame.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| FV1 | Widen during the climb into the seat, or once seated | Once seated (the carry into the seat stays at the on-foot view) | provisional |
