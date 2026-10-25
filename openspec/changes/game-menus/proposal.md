# Proposal: game-menus

## Why

The owner (2026-10-24): "Document these controls in options menu please. We'll need proper game menus
soon". The game has no menus: Escape only releases the mouse, and the controls are known only from
the source and `data/input/bindings.ron`.

## What changes

- **Pause menu (first):** Escape pauses the game and opens a pause menu with Resume, Controls and
  Quit. Escape again goes back a page, or resumes.
- **Controls page (first):** every control, in groups (on foot, in the mech, weapons and targeting,
  cockpit, game), read from the bindings data when the page opens, so it is never out of date.
- **Later, in this change:**
  - A title menu: new game, mission select, options.
  - Options pages: graphics, audio, mouse sensitivity and pitch invert.
  - Rebinding a control from the Controls page, written back to the bindings data.
