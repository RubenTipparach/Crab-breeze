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

## 3. The title menu (`cb-app::menu`, `Menu::Title`)

The owner (2026-10-24): "Build a menu for the game."

- **When it shows:** a plain run, with no `--mission`, `--showcase`, `--bench` or `--stills`, opens on
  the title menu. Runs started from the command line skip it, so playtests, the perf suite and the
  stills are unchanged.
- **What is behind it:** it opens the moment the game prints `GAME_READY` (the loading screen waits on
  the simulation running, so the title cannot pause it earlier). It sits over the live world, with
  the pilot standing by the mech, dimmed less than the pause menu so the world shows through.
- **Pausing:** opening it pauses virtual time and frees the mouse, as the pause menu does.
- **Pages:**
  - **Free roam:** today's plain run. The encounter mechs near the start are spawned when it is
    chosen, not at startup.
  - **City defence:** the defence mission. Its plugin is always present and runs only once the
    mission begins (`Defending` inserted), so the title can start it with no restart.
  - **Options**, **Controls** and **Quit**.
  - **Career entries:** New career and Continue join this list when the career change lands.
- **Escape** does nothing on the title page: there is nothing to go back to.

The command line's `--mission defence` inserts `Defending` before startup, exactly as the title's
choice does later: one way a mission begins.

## 4. Options (`Menu::Options`, `cb-core::settings`)

- **One settings record:** `Settings { mouse_deg_per_px, invert_pitch, fov_deg, shadows, vsync,
  fullscreen }`.
  - Its defaults are the data's own values: mouse and invert from the bindings, field of view from
    the pilot data. Shadows, vsync and fullscreen default to shadows on, vsync on and windowed.
  - It is validated like data: mouse 0.01-1 degree per pixel, field of view 60-110 degrees vertical.
- **Saving:** settings are saved to `settings.ron` at the repository root (gitignored: a player's own
  choice, not tuning) when the Options page is left, and loaded at startup over the defaults. A file
  that fails to load is reported and the defaults are used; it is not overwritten until Options is
  saved.
- **The page:** one row per setting, with - and + buttons for the numbers (mouse in steps of 0.01,
  field of view in steps of 5 degrees) and a button that flips each switch.
- **Live effect:** every change applies at once, from the one record:
  - mouse sensitivity and invert are read by the input sampler;
  - field of view goes to both the eye camera and the cockpit pass, which must match;
  - shadows go to the sun's shadow maps;
  - vsync goes to the window's present mode;
  - fullscreen goes to the window mode.
- **The pause menu** gains Options. Back from Options returns to the page it came from (the title or
  the pause menu).
- **Switching menus** (owner, 2026-10-25, survey J1: "ESC inside the mech will bring up the menu with
  buttons caan switch to other menus"): the pause menu has a row of page buttons along its top
  (Resume, Options, Controls, and Mail once mission-jobs is built, Quit at the end), so any page is
  one click from any other, not only through Back.
- **Volume** (owner, 2026-10-25: "add in controls for sfx/music, make sure it saves when I change
  it"): `sfx_volume` and `music_volume`, 0-1 (shown as 0-100 %, steps of 10 %), default 1. Sound
  effects scale every placed voice's level (`audio::level_voices`, so a playing loop follows at
  once); music scales the music player's level. A settings file without them loads with them at 1.
- **Saving on change:** every Options button saves `settings.ron` at once, as well as leaving the
  page, so a change survives a quit or a crash.
- **Precedence:** the environment switches for measurement (`CB_NO_SHADOWS`, `CB_UNCAPPED`) still win,
  so a perf run is never changed by a player's settings file.

## 5. Out of scope for now

Graphics presets beyond shadows, and
rebinding (task 2.3).
