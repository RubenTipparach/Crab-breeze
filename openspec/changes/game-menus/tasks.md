# Tasks: game-menus

- [x] 1.1 `Bindings::controls`: the table of every control, with the test that none is missing
- [x] 1.2 Pause menu on Escape (Resume, Controls, Quit); the game pauses while it is open
- [x] 1.3 Controls page from the table
- [x] 2.1 Title menu after GAME_READY: Free roam, City defence, Options, Controls, Quit; command-line runs skip it
- [x] 2.2 Options: mouse sensitivity and invert, field of view, shadows, vsync, fullscreen; saved to settings.ron; live
- [x] 2.5 Page buttons along the top of the pause pages (survey J1)
- [x] 2.6 Sound effects and music volume on Options; every change saved at once
- [x] 2.7 Options on the main menu, the pause menu's own rows (one implementation, `menu::options_rows`); settings take
  effect at once, a fading music cue included (owner, 2026-10-28: "game needs options in main menu")
- [ ] 2.4 New career and Continue on the title menu (career)
- [ ] 2.3 Rebinding from the Controls page, written to the bindings data
- [x] 2.8 Menu buttons answer the pointer (owner, 2026-10-29: "also need hover/click feedback from ui buttons, please implement sfx for those too"): the border lights under the pointer with a quiet tick (`ui_hover`), a press clicks (`ui_click`); `cb-app::ui_feedback`, cues `Space::Ui` in `data/audio/cues.ron`
