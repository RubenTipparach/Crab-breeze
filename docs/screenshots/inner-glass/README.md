# Window panes from inside: before and after

Owner, 2026-10-29: "interior windows i think shouldnt have been shaded with reflection probe". Each image is a pair:
`main` on the left, branch `inner-glass` (PR #104) on the right.

| File | View |
|---|---|
| `tower-inside-out.jpg` | `tower-inside-out` (new): inside the 28-storey tower nearest the largest city's middle, third floor, 3 m in from its facade, looking out at the street |
| `airport-inside-out.jpg` | `airport-inside-out`: inside the airport terminal, looking out at the apron |

What they show:
- **Tower.** Before, the window from inside was the mirror glass: the sky and its clouds lay over a darkened street.
  After, the street, the tower across it and a pedestrian are seen plainly through the pane. The tower across the
  street still mirrors the sky on its outside, as it should.
- **Airport.** The terminal's panes already let most of the view through (`faeefe5e`). After, they no longer carry
  the faint sky reflection. The flat blue patches on the floor in the left image are tiles not yet streamed in when
  that still was shot (lavapipe draws about one frame a second); they are not part of the change.

How they were made: headless stills on lavapipe (Mesa's software Vulkan, no GPU), release builds of each branch,
`CB_HOUR=12 CB_STILLS_SETTLE_S=90 CB_STILLS_WEATHER=1 CB_STILLS_ONLY=tower-inside-out,airport-inside-out
crab-breeze --stills <dir>`. No frame-time reading of any kind.
