# Proposal: cockpit-alarms

## Why

Owner, 2026-10-24, from the seat of the Foreman's cockpit: "we can add some dynamic lights too, and sparks and stuff
when the mech gets damaged", "like warning lights when limbs get shot off, or heat build up from rockets or skates",
"incoming missile lights would help too". The cabin is sealed (CLAUDE.md section 1) and lit by its own lamps and the
screens' glow (`cockpit-lighting`), so it is calm whatever happens outside. The pilot learns what is wrong with the
mech today only by reading the MFDs and the HUD strip. A real cockpit tells you with light: a red strobe when a limb
goes, the cabin warming orange as the boosters or skates overheat, an amber beacon when a missile locks on, sparks
off the panels when a round hits the chest.

## What changes

- **Alarm lamps in the cabin.** Each interior already marks its lamps (`LIGHT_overhead`, `LIGHT_rear`,
  `LIGHT_footwell`, `LIGHT_sill_L`, `LIGHT_sill_R`, `LIGHT_screen`); the game removes the glTF lights there today.
  Those points become the alarm lamps: dynamic point lights on the cockpit layer, dark until an alarm drives them.
- **Four alarms, each read off the simulation** (CLAUDE.md section 6.1, the light shows the number the sim uses):
  - **Part lost**: a part's armour reaching zero (`mech::damage`) strobes red for 3 s, then the lamp on that side
    holds dim red while the part is gone.
  - **Heat**: the boosters' and the skates' `heat` (`mech::locomotion`, 0 cold to 1 locked) warms the sill lamps
    orange from 0.6 on, pulsing at 1.0 when the feature locks out.
  - **Missile lock**: a missile in flight whose seeker is locked on the pilot's mech turns the overhead lamp into an
    amber beacon, flashing faster as the missile closes (time to impact from its range and closing speed).
  - **Hit**: a hit on the torso or the head throws a spark burst from the panel nearest the hit's side and flickers
    the cabin lamps for 0.2 s, scaled by the damage.
- **Priority when several fire**: missile lock, then part lost, then heat; the hit flicker lays over any of them.
- **Data**: colours, intensities, rates and thresholds in `data/cockpit_alarms.ron`, units in the keys.

## Out of scope

Sounds (a later `cockpit-audio` change), the alarm pages on the MFDs (`cockpit-hud`), the alarm-mute control (survey
A3 below), lights outside the cockpit (`combat-fx`).
