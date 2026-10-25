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
   ACCEPT). BACK on any app page, or Escape, returns that screen to its menu; Escape on the menu returns the screen
   to the page it showed before.
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

- `mfd::point_on(ray, screen) -> Option<Vec2>`: the ray against the screen mesh's two triangles in the screen's
  frame, the hit's barycentric weights applied to the mesh's own UVs, then the screen's page slice: a pixel on the
  page, 0 to 512 each way. Pure, tested on a quad: the ray through the quad's centre lands on (256, 256); through a
  corner, on that corner's pixel.
- Each page's UI rows are `Interaction` targets in page pixels (Bevy UI nodes on the MFD camera). The pointed pixel
  is turned into a hover and a press on the node under it through Bevy's UI picking with a custom pointer for the
  MFD camera, so buttons on a page work like any UI button: one pointing path for every page.

## 4. The pages on the texture

The MFD texture holds every page side by side (`hud::MFD_PAGES`): the five readouts, then MENU, MAP, MAIL, SHOP:
nine pages, 4 608 x 512 px. A screen shows one slice (its `page`). MENU is one page whose lit row follows the
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
