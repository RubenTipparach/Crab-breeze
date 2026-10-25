# Proposal: signature-weapons

## Why

The owner, 2026-10-25, on the advanced weapons (`weapon-kinds`, `railgun-scope`, `multi-lock`): "other mechs will
have more advanced weapons like shotguns, rail guns (with a scope), multi lock on missiles, lasers, plasma cannon",
and on the roster: "each mech comes with an epic unique weapon", as part of bringing every mech "up to the gold
standard that is the Foreman".

Today all eight roster mechs fly the same three guns (`default_weapons`: a 35 mm autocannon, a 120 mm cannon, a
shoulder missile), whatever their models carry: the Foreman's belt-fed boom cannon, the Driller's auger, the Harrow's
twelve missile cells, the Ironhorse's steam cannon, the Mule's rocket pods, the Picket's long gun, the Roustabout's
heavy rifle, the Sapper's shoulder cannon are all decoration.

## What changes

- **Each roster mech gets one signature weapon**, the one its model already carries, built on the kinds
  `weapon-kinds` adds and the rules `multi-lock` and `railgun-scope` add, so each plays differently:

  | Mech | Signature | How it plays |
  | --- | --- | --- |
  | Foreman | Boom cannon | Three 120 mm shells in 0.4 s, belt-fed; a wall of blasts |
  | Driller | Cutting lance | A short, hot beam (600 m): cuts a part off in under 2 s up close |
  | Harrow | Twin racks | Twelve missiles on twelve locks, released as one salvo |
  | Ironhorse | Steam cannon | A slow ball of superheated steam and slag that bursts in a scalding cloud |
  | Mule | Rocket ripple | Sixteen dumbfire rockets in 2 s from both shoulder pods |
  | Picket | Rail rifle | The railgun with the 10x scope, from behind its shield |
  | Roustabout | Breaching gun | A sixteen-pellet shotgun beside its piston fist |
  | Sapper | Siege mortar | A back-mounted mortar that lobs over buildings onto the aim point |

- **Two new weapon fields** where the kinds cannot say it: `burst` (shots a pull, and the gap between them: the
  Foreman's three, the Mule's sixteen) and `lob` (the gun elevates itself to land a shell on the aimed point by the
  high arc: the Sapper). Everything else is `weapon-kinds`' `ShotKind`, `multi-lock`'s `Seeker` and `railgun-scope`'s
  scope, unchanged.
- **Loadouts live in data**: each mech's weapons in `data/mechs/<id>/weapons.ron`, hand-tuned and left alone by the
  generator that writes the rest of the mech's data from its glb.
- **The cockpit shows it**: the WEAPONS page lists the signature station first, with its own state (the burst's
  rounds, the locks held, the rail's charge, the heat bank), from the state the shots fire from.

## Depends on

`weapon-kinds` (the kinds and the heat bank), `multi-lock` (the Harrow), `railgun-scope` (the Picket). Built after
them; nothing here duplicates them.

## Out of scope

The weapons' detailed models (the mechs already carry them), their sounds (cue names listed for `audio`), the enemy
combat mechs' loadouts (`weapon-kinds` section 5), and balance beyond starting numbers: playtests tune them.
