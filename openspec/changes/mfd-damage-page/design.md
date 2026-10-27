# Design: mfd-damage-page

## 1. The page

512 x 512 px like every page. Under the title, a body 300 px tall in the middle of the page, the parts as boxes laid out
like the design picture (a front view of a mech: head on the torso, arms either side, legs below), then the worst part's
line (`ALL PARTS OK`, or `LEG L 42%`).

| Part | Box (px, from the body's top left, 300 x 300) |
| --- | --- |
| Head | 120, 0, 60, 44 |
| Torso | 80, 52, 140, 112 |
| Arm L (pilot's left, drawn on the left) | 20, 60, 52, 120 |
| Arm R | 228, 60, 52, 120 |
| Leg L | 88, 172, 56, 128 |
| Leg R | 156, 172, 56, 128 |

A part's box: a 4 px border and a fill at 25 % alpha in its colour; its per cent inside it. A lost part: border dim
red at 50 %, no fill, `X`. A mech whose data names a part twice (two of a kind) shows the worse of the two.

## 2. One measure of hurt

`cb_core::mech::damage::Damage::health(parts, i)`: structure left over full structure, 0 when lost, clamped to 0-1.
`health_colour` (green, amber under 0.7, red under 0.3) lives beside the HUD's colours and is used by both the MECH
page's text and the DAMAGE page. Pure and tested in the core.

## 3. Pages

`hud::MFD_READOUTS` 5 to 6, `hud::MFD_PAGES` 7 to 8; `mfd_apps::READOUTS` gains `DAMAGE`; the MENU's readout grid
gains a third row (it fits above MAIL). The dash's left screen starts on page 5 (`mfd::DASH_SCREENS`).

## 4. Cost

One more 512 px page on the MFD texture (8 x 512 x 512 RGBA: 8 MB, up from 7 MB); about 20 UI nodes. No triangles.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| MD1 | Which screen starts on DAMAGE | The dash's left, under the hung MECH page | provisional |
