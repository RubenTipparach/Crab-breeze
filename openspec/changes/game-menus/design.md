# Design: game-menus

## 1. The controls table (`cb-core::input::Bindings::controls`, pure)

One function turns the bindings into rows: a group, the control's name in the pilot's words, and its
keys in readable form (`KeyW` reads W, `ControlLeft` Left Ctrl, `MouseLeft` Left mouse). A test
checks every binding the game reads appears in the table, so a new binding cannot go undocumented.

## 2. The pause menu (`cb-app::menu`)

- Escape opens it. Opening it:
  - pauses virtual time, so the fixed step, and with it the simulation and the take, stop;
  - releases the mouse.
- Escape inside it goes back a page, and from the first page resumes.
- Resume re-grabs the mouse. A click with the menu open does not.
- Pages are UI on the cockpit pass, which draws last and is always on top. Buttons are Bevy UI
  buttons clicked with the mouse.
- Quit closes the game.
- The scripted takes never press Escape, so a take never opens the menu.

## 3. Out of scope for the first step

The title menu, the graphics and audio options and rebinding: tasks 2.x.
