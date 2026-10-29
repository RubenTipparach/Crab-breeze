# The Sapper from its seat: mockup against game

Asked for by the owner through the desktop session (branch `perf-probe`, 2026-10-29): which parts of the mech
should be visible from the cockpit, and whether the game shows them. Each image is a pair, **the three.js mockup
on the left, the game on the right**, from the Sapper's `EYE_pilot`, seated and sealed, at the seated field of view
(`view_fov_deg: 95.0` vertical, `data/pilot/pilot.ron`).

| File | View |
|---|---|
| `seat-ahead.jpg` | straight ahead |
| `seat-left45.jpg` | 45 degrees left |
| `seat-right45.jpg` | 45 degrees right |
| `seat-down-left.jpg` | 45 degrees left, 30 degrees down |
| `seat-down-right.jpg` | 45 degrees right, 30 degrees down |

## How they were made

- **Mockup** (left): `tools/mockups/shoot_seat_view.mjs` on the mech design page
  (`python3 tools/mockups/gen_mech_designs.py`, served from `output/mech_site`). The camera sits at `EYE_pilot`
  with the mech standing idle. The hull is hidden by the game's own rule (`is_hull` in `crates/cb-app/src/mech.rs`:
  a mesh whose nearest contract bone is `torso_twist`, `torso` or `cockpit`, with no `GUN_` joint between; 40 meshes
  on the Sapper). The cockpit interior is hidden too, so the picture is what the screen wall should show of the
  mech. Everything else is drawn. The page's ground is a plain plane under a flat sky, so there is no city.
- **Game** (right): headless stills on lavapipe (Mesa's software Vulkan, no GPU), release build of `perf-probe`
  with `CB_MECH=mechs/sapper.ron CB_STILLS_SEAT=1 --stills <dir>`. The scripted pilot walks to the Sapper, boards it
  and waits for the hatch to seal, as a player does. Then, holding free look, it turns the view by look input to
  each direction (each was reached within 0.5 degrees, logged as `STILLS_SEAT`) and shoots it. The cabin interior,
  its screens and the HUD are drawn as in play.
- **The Sapper's glb is not rebuilt** with commit `10c89b5` (moving the shoulder cannon onto its own `GUN_cannon_R`
  joint): no Blender was run, as the owner asked. In both renders the cannon still hangs off the cockpit bone, so
  the hull rule hides it in both.

## What each view shows

| View | Mockup (what should be seen) | Game | Missing in the game |
|---|---|---|---|
| Ahead | the right arm's gun at the lower right; the tip of the left arm's gun at the lower left | the right gun's barrel and housing through the lower front-right screen; the left gun's yellow tip by the dash on the left | nothing; both guns are cut by the screen frames and the dash |
| 45 degrees left | the left shoulder pad filling the upper left; the left arm, elbow and gun below it | the shoulder pad through the upper left screens; part of the arm and gun through the lower left screens, much of it behind the frame and the dash | most of the left arm and the elbow (behind the dash and frames) |
| 45 degrees right | the right shoulder pad, upper right; the right arm and its gun below | the shoulder pad through the upper right screens; parts of the arm through the lower right screens | most of the right arm and its gun (behind the dash and frames) |
| Down left | the left shoulder and arm, the elbow, and below them the top of the left leg (thigh and knee) | the cabin floor and the left console fill the view; the shoulder pad shows in the upper left screen | **the leg**: there is no screen below the dash to see it through; the arm |
| Down right | the right shoulder above; the right arm, its elbow and gun; the top of the right leg below | the right console and the dash fill the lower half; the shoulder pad and the top of the gun show through the right screens | **the leg**; most of the arm and its elbow |

**Shoulder cannon:** in none of the five, in either render. Its glb is not rebuilt, so it is still hull.
**Arms:** the guns show ahead, and the arms show in part at 45 degrees left and right (shoulder pads plainly, the
forearms and guns partly), in the mockup and the game alike. **Legs:** only in the mockup's downward views. In the game the
cabin floor and dash cover the bottom of the view, so no leg is seen from the seat in any of the five directions.

## Not done here

- Rebuilding the Sapper (`tools/blender/build_mechs.py -- sapper`) and its data, which would bring the shoulder
  cannon into view: the owner said no Blender for this comparison.
- A frame-time reading of any kind: lavapipe renders in software.
