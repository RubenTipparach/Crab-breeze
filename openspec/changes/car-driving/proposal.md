# Proposal: car-driving

## Why

The owner (2026-10-24): "the ability for the player to control cars and drive around." On foot the
pilot walks at a few metres a second on a 41 km island. A car gets them across a town, out to their
mech and around a base, and it is fun on its own.

## What changes

- **Getting in:**
  - Walk up to a parked car's driver door and press Use (E).
  - The door opens and you are carried into the driver's seat; the view stays yours.
- **Driving:**
  - W accelerates, S brakes and reverses, A and D steer, Space is the handbrake, the mouse looks
    round.
  - It drives arcade style (survey J8): grippy and simple, quick to pick up, sliding only on the
    handbrake, bouncing over kerbs and craters. The dashboard's speedometer shows the simulation's own
    speed.
- **First person only** (the law; superseded 2026-10-26: driving is third person, `car-chase-camera`): you see the modelled dashboard, the wheel turning with your
  steering, your hands on it, and the road through the windscreen. There is no outside camera.
- **Getting out:** stop and press E; you step out beside the door.
- **Which cars:** any car (survey J9), parked or in traffic. Walk up to a traffic car's driver door
  while it is stopped (at a light, in a queue) and press E: the driver gets out and runs, and you get in.
- **Damage:** a car you drive takes hits and can be crushed like any car (car-traffic); if it is
  crushed with you in it, you are thrown clear, hurt.
