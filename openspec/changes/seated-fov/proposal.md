# Proposal: seated-fov

## Why

Owner, 2026-10-26: "normal fps fov is 75, it grows to 95 when inside the mech", "please implement the fov
transition". Today the view has one field of view, 95 degrees, on foot and seated alike (`view_fov_deg`, and the
Options page's setting).

## What changes

- **On foot the view is 75 degrees, seated in the cockpit 95** (vertical), both in `data/pilot/pilot.ron`
  (`on_foot_fov_deg`, `view_fov_deg`).
- **The view widens as the pilot sits and narrows as they stand**, eased over `fov_change_s` (0.8 s), never a jump.
  Only the field of view changes: the view's direction stays the player's (CLAUDE.md section 8).
- **The Options page's field of view stays the seated one** (a saved 95 means the same as before); on foot is that
  less the data's difference (95 - 75 = 20 degrees), so the slider moves both.
- **One system owns the lens's field of view** (`pilot::ease_fov`); the settings no longer set it themselves.
