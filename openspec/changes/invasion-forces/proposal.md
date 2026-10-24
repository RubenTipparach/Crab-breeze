# Proposal: invasion-forces

## Why

Owner, 2026-10-24: "We need some placeholder enemies. Tanks, helicopters. Fighter jets, crab airships,
and crab mechs." "Crab air ship is like a floating air fortress/carrier with turrets and it's where
it launches fighters and mechs. Mechs are parachuted down to the ground." "Actually tanks and mechs
should be air dropped via cargo plane." "Crab airships launch fighters and crab mechs parachute off
it." The defence waves are ten copies of one raider; Ace Combat (G4) is a mixed force arriving from
the sky, with big set-piece targets.

## What changes

Placeholder models (blocked-out boxes and cylinders in the units' sizes and colours) until each gets
its Blender model; every unit flies or drives through the one projectile, damage and AI code.

- **Tanks**: tracked, turret gun (`combined-arms` numbers: 12 m/s, 120 mm at 900 m/s, 200 mm front
  armour). They arrive by **cargo plane**: a plane crosses at 400 m, the tank drops on a parachute,
  lands, drives.
- **Helicopters**: hover and circle at 70-110 m, rocket pods (`combined-arms`).
- **Fighter jets**: fast (180-250 m/s), turn in wide circles, strafe with cannon and fire missiles
  from range; launched from airships.
- **Cargo planes**: big, slow (90 m/s), unarmed; fly a line over the attacked city at 400 m and drop
  tanks and enemy mechs on parachutes. Shooting one down before its drop kills its cargo.
- **Crab mechs**: the alien crabs (`crab-enemies`), dropped from airships on parachutes.
- **Crab airships**: a floating fortress and carrier, 300 m long, hovering at 600-900 m and drifting
  over the battle at 15 m/s; turrets round its hull (flak and beam guns), a launch deck that sends
  fighters off every 20 s, and bays that drop crab mechs on parachutes. Its turrets, engines and
  bays are separate lockable parts; destroying its engines brings it down. A set piece (Ace Combat's
  carrier fights).
- **Parachutes**: one rule for every drop: a body falls, a chute opens 2 s after release, it
  descends at 8 m/s, swings, lands, the chute folds; hit while descending, it can be killed before it
  lands.
- **The defence waves** become mixed forces from data (`data/missions/defence.ron`): which units
  arrive, how (driven in, cargo drop, airship), and when.

## Out of scope

Real models (each unit's Blender change), naval units, infantry, enemy AI tactics beyond each unit's
brain.
