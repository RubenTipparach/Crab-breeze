# Window panes from inside: before and after

Owner, 2026-10-29: "interior windows i think shouldnt have been shaded with reflection probe". Each image is a pair:
`main` on the left, branch `inner-glass` (PR #104) on the right.

| File | View |
|---|---|
| `tower-inside-out.jpg` | `tower-inside-out` (new): inside the 28-storey tower nearest the largest city's middle, third floor, 3 m in from its facade, looking out at the street |
| `airport-inside-out.jpg` | `airport-inside-out`: inside the airport terminal, looking out at the apron |
| `airport-inside-out-loaded.png` | the same view with the fix, after 240 s instead of 90 s, its furniture all in |

What they show:
- **Tower.** Before, the window from inside was the mirror glass: the sky and its clouds lay over a darkened street.
  After, the street, the tower across it and a pedestrian are seen plainly through the pane. The tower across the
  street still mirrors the sky on its outside, as it should.
- **Airport.** The terminal's panes already let most of the view through (`faeefe5e`). After, they no longer carry
  the faint sky reflection. Neither still caught the furniture whole: it is meshed on a background thread as you
  come near (a terminal's is 350 000 triangles), and on lavapipe the same CPU also draws every frame, so after 90 s
  it was half in on the left (the flat blue seat rows) and not yet in on the right. `airport-inside-out-loaded.png`
  is the fixed build again after 240 s: the seats all there, the windows clear behind them.

How they were made: headless stills on lavapipe (Mesa's software Vulkan, no GPU), release builds of each branch,
`CB_HOUR=12 CB_STILLS_SETTLE_S=90 CB_STILLS_WEATHER=1 CB_STILLS_ONLY=tower-inside-out,airport-inside-out
crab-breeze --stills <dir>`. No frame-time reading of any kind.
