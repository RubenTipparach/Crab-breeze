# Proposal: pilot-skill

## Why

Owner, 2026-10-24: "Mech should've been moving around jumping avoiding shots. It shot at buildings
and didn't use any piloting skill at all." The AI pilot (enemy mechs, and the scripted pilot of the
player's mech in a take) must fly like a good pilot.

## What changes

- **Threat awareness**: every shot in flight toward the mech is predicted (closest approach and
  time); one that will pass within 6 m inside 1.6 s is dodged: a sidestep across its path at full
  throttle, and a jet jump for a rocket or a shell that would land within 0.9 s, when the jets are
  cool (survey M3: heat-limited).
- **Never wasted fire**: the guns fire only with a clear line to a target (no building, no ally in
  the way), so buildings take fire only when they are the target.
- **Target priority**: the most dangerous visible target first: rocket teams and helicopters (they
  hurt), then tanks, then mechs, then technicals and infantry; nearer and already-damaged first.
- **Always moving**: engaging is an orbit at stand-off with the throttle never below 60 %, the torso
  on the target; a blocked line for 2 s brings a jump to a new spot.
- Mining Mike's autopilot (godot-sandbox `bot_brain.gd`, `5f4feacf`) is the model: tasks re-decided
  a few times a second, steering every step, danger stepped out of before it lands.

## Out of scope

Missile lock-on and countermeasures (G4), cover-to-cover planning.
