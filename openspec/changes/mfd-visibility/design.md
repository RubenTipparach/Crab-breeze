# Design: mfd-visibility

## 1. Where a screen goes

From `EYE_pilot`, a screen at elevation e and azimuth a, distance d: its centre at the eye plus d (sin a cos e, sin e,
cos a cos e) (glTF frame, +Z ahead), its face turned to the eye, its up the world's up projected onto its face.

| Row | Elevation | Azimuths | Distance (at most) | Screen sides | Spans |
| --- | --- | --- | --- | --- | --- |
| Hung | +30 degrees | -23, 0, +23 | 0.82 m | 0.28 m | about +20 to +40 degrees |
| Dash | -25 degrees | -26, 0, +26 | 0.80 m | 0.26, 0.30, 0.26 m | about -36 to -14 degrees |

A row comes nearer in a shallow cabin: the distance is cut until every screen of the row, and a point 10 cm beyond it
along the line from the eye, is inside the cabin's plan, and the screens, housings and gaps shrink by the same share, so
the row covers the same part of the view and no screen stands in a wall or behind a window frame. The row takes one
distance, the nearest any of its screens needs, so its gaps stay open. The hung row's 23 degree azimuths keep its outer
corners inside the 16:9 picture at 95 degrees (a flat picture's edge is where tan(elevation) / cos(azimuth) reaches
tan(47.5 degrees)).

The 95 degree view reaches 47.5 degrees above and below the eye line and 62.6 degrees to each side (16:9), so every
screen lies inside it; the fighting band (-10 to +20 degrees) is clear of both rows.

## 2. The dash and its pod

The dash body drops so its top sits under the dash screens' lower edges; each screen stands on a neck from it in a
housing, with nothing over its top (a hood read as a bar across the screen from the seat). The glare shield goes: nothing is over the screens to hide them. The dash keeps
its node (`CONSOLE_front`), riding the part the pilot leaves through.

## 3. Check

`tools/blender/measure_mfds.py` (a measurement instrument): for each of the six screens, the share of a 9 x 9 grid of
points on it inside the picture and not hidden, and its height over its width as drawn. Every cockpit: 100 % and
0.9-1.2. `tools/blender/measure_view.py`: the fighting band 25 % hidden at most.

Measured on the 512 px builds (geometry is the same at 4096 px), at 95 degrees:

| Cockpit | Screens whole | Aspects | Fighting band hidden |
| --- | --- | --- | --- |
| Canopy | 6 of 6 | 0.97-1.16 | 21.7 % |
| Panorama | 6 of 6 | 0.97-1.16 | 20.0 % |
| Cab | 6 of 6 | 0.97-1.16 | 21.3 % |
| Octagon | 6 of 6 | 0.97-1.16 | 22.5 % |
| Cinema | 6 of 6 | 0.97-1.16 | 17.3 % |
| Titan | 6 of 6 | 0.97-1.16 | 24.4 % |
| Tile wall | 6 of 6 | 0.97-1.16 | 17.3 % |
| Twin tier | 6 of 6 | 0.97-1.16 | 18.3 % |

## 4. Cost

A few hundred triangles a cockpit (the pods and stalks); eight rebakes of about 45 minutes each.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| MV1 | The rows' angles | Dash -25, hung +30 degrees | provisional |
