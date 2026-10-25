# Proposal: railgun-scope

## Why

Owner, 2026-10-25: "rail guns (with a scope)". A railgun hits at 2 km (`weapon-kinds`), where a
mech is a few pixels on the screen wall. The pilot needs to see that far to use it. The law
(CLAUDE.md) says the camera is the pilot's eyes and never zooms out of the cockpit, so the scope is
an instrument in the cockpit: a magnified picture on the front screen, the way a gunner's sight is a
screen in a real armoured vehicle.

## What changes

- **Hold Scope** (`scope`, Mouse Middle and Z by default, bindable) with a scoped weapon on board: a
  window opens in the middle of the front screen showing the view down the aim at 4 times
  magnification. Press `scope_zoom` (mouse wheel) for 10 times. Release Scope and the window closes.
- **The pilot's own view does not change.** The cabin, the screen wall and the rest of the world stay
  where they were; only the window is magnified. The view stays the player's: while scoped, mouse
  look is slowed by the magnification so the picture does not swing wildly, and nothing turns it.
- **The sight is the simulation's numbers**: a reticle at the aim, the range from a rangefinder (the
  laser's ray, `weapon-kinds`), the railgun's drop mark from `weapons::lead` with drop (the function
  the slug flies by), the charge ring while it charges and the weapon heat bank.
- **The scope rides the sensor head.** Its camera sits on the head (`CAM_main`); lose the head
  (`mech-damage`) and the scope is gone with the main cameras. The backup cameras have no zoom.
- **Any weapon can be scoped in data**: `scope_zoom: [4.0, 10.0]` on its `WeaponSpec`. The railgun
  has it; nothing else does until the owner asks.

## Out of scope

Thermal or night modes, a scope for the enemies (the AI aims by numbers already), stabilisation
modelling beyond the mech's own sway.
