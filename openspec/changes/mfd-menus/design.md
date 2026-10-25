# Design: mfd-menus

## 1. What the player does

1. Hold free look (Left Ctrl, bindable). The guns hold (`cockpit-mfds`); the mouse turns the head as far as the
   seat allows (`seated-look`).
2. Look at a screen. It gets an **amber outline** round its bezel: this one will take the click.
3. Click (any fire button). The screen shows its **menu**:

   ```text
   +--------------------------+
   |  MFD 2          [BACK]   |
   |  ----------------------  |
   |  MECH      FLIGHT        |
   |  MISSION   WEAPONS       |
   |  TARGET                  |
   |  ----------------------  |
   |  > MAP                   |   <- the row under the crosshair, lit amber
   |    MAIL  (2 new)         |
   |    SHOP                  |
   |    MENU                  |
   +--------------------------+
   ```

4. Look at a row (it lights) and click: the screen shows that page from now on. Each screen keeps its own page, so
   the pilot can keep the MAP on one, MAIL on another and the readouts on the rest.
5. On an app page, the same pointing works on its own rows and buttons (the shop's BUY, the map's zoom, a mail's
   ACCEPT). **BACK** (top right of the menu and of every app page), or Escape while pointing at the screen, closes
   the menu or app and gives the screen back the readout it showed when its menu was opened: one press, back to the
   mech's own view (owner, 2026-10-25: "clicking back should return to mech view and restore the mfd to its original
   screen"). Each screen remembers that readout, its **home**, from the click that opened its menu until BACK; going
   from the menu to an app and on to another keeps it. Picking a readout from the menu makes that the screen's page,
   and so its home the next time. The one step inside an app is the shop's sheet, whose **LIST** button returns to the
   shop's list (`mech-shop`).
6. Let go of free look: the outline goes, the screens keep their pages, the guns are live again.

## 2. The outline

- **Which screen:** the nearest screen the view's centre ray enters within 3 m, the same test the press uses now
  (`mfd::press`, `ray_hits_box`). One function finds it for both the outline and the click, so the outlined screen
  is always the one that takes the click (CLAUDE.md 6.1).
- **What it is:** a frame of four bars, 12 mm wide, lying 8 mm outside the screen's own rectangle and 3 cm in front
  of it toward the eye: 1 cm in front of the dash's bezels, which stand about 2 cm proud of their glass, so it never sits inside a bezel or shares a plane with one (no z-fighting). Unlit
  amber `(1.0, 0.62, 0.15)` at 1.4 times white, so the bloom gives it a soft glow (at 4 times the tonemapper washed it to cream). Built once per screen from the
  screen mesh's own bounds (the two long axes are the rectangle, the thin one its facing; the eye's side picks the
  front), a child of the screen, so it rides an opening part with its screen. On the cockpit render layer, like the
  cabin. Hidden unless pointed at in free look.
- **Seated only, screens only:** the outline and the menus work while seated with the cabin sealed; standing in the
  open hatch nothing lights.

## 3. Pointing on a page

The click and the row lighting need the spot on the page, not only the screen:

- `mfd::ray_uv`: the ray, in the screen's own frame, against the screen mesh's triangles; the hit's barycentric
  weights applied to the mesh's own UVs give the point on the page, 0 to 1 from its top left (times 512: page
  pixels). Pure, tested on a quad: through the middle it lands mid-page, near a corner near that corner, off the
  quad or looking away nothing.
- **One table of rows** (`mfd_apps::items`): every row and button of MENU and MAIL as a rectangle in page pixels. The
  page's nodes are laid out from it and the pointed pixel is tested against it (`item_at`), so the row that lights
  amber is the row a click presses (CLAUDE.md 6.1). This replaces the Bevy UI picking first planned here: a custom
  pointer on a render-to-texture camera was a second path to keep in step, the table is one.
- **Nothing to read or press above 80 px:** from the seat, the dash's glare shield hides the top 15 % of the dash's
  screens (seen in the first stills, 2026-10-25), so every row starts below it; tested.

## 4. The pages on the texture

The MFD texture holds every page side by side (`hud::MFD_PAGES`): the five readouts, then MENU, MAP, MAIL, SHOP:
nine pages, 4 608 x 512 px when MAP and SHOP are built; seven (the readouts, MENU, MAIL) today, 3 584 x 512 px. A screen shows one slice (its `page`). MENU is one page whose lit row follows the
pointer, so two screens showing their menus at once show the same rows; only the pointed one is pressed.

| Page | Built in | Notes |
| --- | --- | --- |
| MECH, FLIGHT, MISSION, WEAPONS, TARGET | `hud` (built) | unchanged |
| MENU | `mfd` | the rows above, MAIL's count of unread from `mission-jobs` |
| MAP | `mfd` | the world plan drawn once into a 512 px image (cities, roads, water, bases), the mech's arrow, the job site and enemies over it; three zooms (the whole island, 8 km, 2 km), centred on the mech |
| MAIL | `mission-jobs` | its page, unchanged |
| SHOP | `mech-shop` | its page |
| (MENU row) | `game-menus` | not a page: picking it opens the pause menu, as Escape does |

## 5. Cost

- **Texture:** 4 608 x 512 RGBA, 9.4 MB, against 5.2 MB now.
- **UI:** the MFD camera draws only the pages some screen shows (the others' root nodes hidden), so the cost is at
  most six pages, as now.
- **Outline:** 6 frames of 8 triangles, one material; one is visible at a time.
- **Pointing:** one ray against six boxes and one quad a frame while free look is held.
- **MAP:** the base map is drawn once when the world is planned (512 x 512, a few milliseconds); markers are UI nodes.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C13 | The outline's colour | Amber, the selected-station colour on the WEAPONS page | provisional (survey C13) |
| C14 | Should the screens remember their pages between sorties | Yes, saved with the career | provisional (survey C14) |
