# Design: signature-weapons

## 1. The loadouts

Groups as `weapon-kinds` section 4: right arm on Mouse Left (the selected station, `weapon-select`), left arm on
Mouse Right, missiles on F, back units on G. Each mech keeps a plain gun beside its signature, so it is never
helpless at a range its signature is bad at.

| Mech | Right arm (Mouse Left) | Left arm (Mouse Right) | F | G |
| --- | --- | --- | --- | --- |
| Foreman | 35 mm autocannon | **Boom cannon** | shoulder missile | |
| Driller | **Cutting lance** | 35 mm autocannon (its heavy autocannon) | shoulder missile | |
| Harrow | 35 mm autocannon | 120 mm cannon | **Twin racks** | |
| Ironhorse | **Steam cannon** | 35 mm autocannon | shoulder missile | |
| Mule | 35 mm autocannon | 35 mm autocannon | **Rocket ripple** | |
| Picket | **Rail rifle** | 35 mm autocannon (its machine gun) | shoulder missile | |
| Roustabout | **Breaching gun** | 35 mm autocannon | shoulder missile | |
| Sapper | 35 mm autocannon (its rifle) | rocket pod (its forearm pod) | shoulder missile | **Grenade launcher** (was the siege mortar; G21) |

Where a model's arm holds a melee tool (the Foreman's breaker, the Driller's auger, the Ironhorse's buffer ram, the
Mule's and Roustabout's piston fists), that arm also strikes in close combat (`close-combat`), as now.

## 2. The eight signatures

Starting numbers, in each mech's `weapons.ron`; playtests tune them.

| | Kind | Speed | Interval | Penetration | Damage | Blast | Heat (share of the bank) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Boom cannon | `Shell`, `burst: (3, 0.2)` | 850 m/s | 4.0 s a burst | 260 mm | 70 a shell | 5 MJ | 0 |
| Cutting lance | `Beam { range_m: 600 }` | instant | held | 160 mm | 90 a second | 0 | 0.30 a second |
| Twin racks | `Rocket`, `Seeker { max_locks: 12, lock_s: 0.25, range_m: 1500, cone_deg: 30, turn_deg_s: 60 }` | 240 m/s | 10 s after a salvo | 100 mm | 30 each | 1.5 MJ | 0 |
| Steam cannon | `Plasma { burst_m: 500 }` | 220 m/s, no drop | 1.8 s | 90 mm | 50 | 6 MJ | 0.20 a shot |
| Rocket ripple | `Rocket`, `burst: (16, 0.125)` | 240 m/s | 12 s a ripple | 120 mm | 30 each | 2 MJ | 0 |
| Rail rifle | `Slug { charge_s: 1.0 }`, scope 10x | 2 400 m/s | 4.0 s | 400 mm | 140 | 0 | 0.35 a shot |
| Breaching gun | `Pellets { count: 16, spread_deg: 2.0 }` | 550 m/s | 1.2 s | 45 mm a pellet | 8 a pellet (128 all in) | 0 | 0 |
| Grenade launcher (G21, replaces the siege mortar) | `Shell`, `lob: (elevation_deg: 30, min_m: 60, max_m: 600)` | 30 degrees, charge set by range | 1.5 s | 90 mm | 60 | 6 MJ | 0 |

What they mean in a fight (mechs about 24 m tall, a torso about 6 m across, parts of 100 to 160 structure):

- **Boom cannon**: 210 damage and 15 MJ in 0.4 s, every 4 s: 52 a second against the 120 mm cannon's 36, and three
  blasts walking across a target. The belt is the burst; no reload beyond the interval.
- **Cutting lance**: inside 600 m it holds one part for 1.3 s to take 120 structure. The bank fills in 4 s of
  firing (0.30 in, 0.05 out a second), so it cuts one part, then cools. Shorter and hotter than `weapon-kinds`' laser
  (1 500 m, 60 a second): a mining tool, not a sniper's.
- **Twin racks**: twelve locks take 3.0 s of sweeping at 0.25 s each; twelve missiles leave 0.08 s apart (one second
  for the salvo). The Harrow is the Ace Combat mech: a sky full of trails on every raider in the cone.
- **Steam cannon**: 2.3 s to 500 m, where it bursts in the air if it has met nothing, so it is dodged at range and
  deadly in a street. Its burst draws the steam recipe big (the same `vent_steam` particles as the hatch, scaled 12x)
  and scalds: the blast path does the damage, nothing new.
- **Rocket ripple**: sixteen 2 MJ rockets in 1.9 s, landing across 20 m at 1 km (the pods alternate and the aim
  jitters 0.6 degrees by the seeded RNG, forked per mech and shot): an area weapon against a group or a building.
- **Rail rifle**: `weapon-kinds`' railgun with `railgun-scope`'s 10x scope; 400 mm beats any part. The Picket stands
  behind its shield and picks parts off at 2 km.
- **Breaching gun**: the cone is 3.5 m in radius at 100 m, so about 12 of 16 pellets hit a torso (96 damage), about 3
  at 200 m. A street fighter's gun beside a piston fist.
- **Grenade launcher** (survey G21, below; it replaces the siege mortar first written here): the shoulder barrel lays
  itself at 30 degrees and sets its charge for the range to the aim point (60 to 600 m), so the grenade lands where
  the crosshair is. Flight is `sqrt(2 d tan(30) / g)`: 3.4 s to 100 m, 5.9 s to 300 m, 8.4 s to 600 m, against the
  mortar's 7.8 s to 300 m and 14.3 s to 1 km. The apex is a quarter of the range times tan(30): 43 m at 300 m. It
  comes down at 30 degrees, so it clears a 24 m mech or a three-storey wall standing 42 m in front of the target:
  it still reaches behind cover, just not behind a tower. One every 1.5 s against the mortar's 6 s, 36 a sortie.

### The Sapper's grenade launcher (survey G21)

The owner, 2026-10-26, on "Is the mortar's 7 to 18 s flight too slow to be fun": **"maybe a grenade launcher?"**,
then "Approve all" and "Go and implement". Read as: keep the one weapon in the game that reaches behind cover, but
make it a quick, short-range one rather than a siege gun. So:

- **Lobbed and arcing**, the same `lob` rule the mortar was to use, at a lower 30 degrees instead of 45: lower is
  faster (the flight time grows with the square root of `tan(elevation)`), and 30 degrees still drops the grenade
  steeply enough to clear a mech-height wall in front of the target.
- **Shorter range**: 60 to 600 m instead of 100 to 1 500 m. Past 600 m it lays for 600 m and falls short.
- **Faster to use**: one every 1.5 s (the mortar's 6 s), 3.4 to 8.4 s of flight (the mortar's 7.8 to 18 s), a
  smaller blast (6 MJ against 20 MJ) and 60 damage against 120, so it is a suppressing gun, not a building-killer.
- **It fires from the Sapper's existing shoulder barrel** (`CANNON_R`, already in its glb), on G (fire group 3), so no
  new socket (`BACK_R`) and no model rebuild was needed. The long barrel reads more as a cannon than a launcher; a
  launcher model is for the Sapper's detailed pass.
- The TARGET page's time-of-flight countdown is not built yet.

## 3. Two new fields, in one place

Both on `WeaponSpec`, resolved in `weapons::launch` beside `weapon-kinds`' kinds (CLAUDE.md 6.2: one `match`, in
`weapons.rs`):

- `burst: Option<(u8, f32)>`: shots a pull and the gap between them, seconds. The pull starts the burst; the shots
  leave on the fixed steps the gaps fall on, each along the aim at its own moment; the interval runs from the last.
  Losing the arm mid-burst ends it.
- `lob: Option<Lob>` (`elevation_deg`, `min_m`, `max_m`): before launch, the elevation is fixed and the muzzle speed
  solved for the horizontal range `d` to the aim point (the ground, building or mech the crosshair is on) and its
  height `dy` above the muzzle, `v^2 = g d^2 / (2 cos^2(e) (d tan(e) - dy))`, the range clamped to the gun's. Pure:
  `weapons::lob_velocity`. It also takes off the half step of gravity the fixed-step flight adds, so the stepped shell
  lands where the smooth arc would. Tested: a grenade lands within 1 m of the aim point at 100, 300 and 600 m.

**Loadouts as data.** `MechConfig.weapons` reads `data/mechs/<id>/weapons.ron` when the mech file names it
(`weapons: Some("mechs/<id>/weapons.ron")`, written by the generator as a reference, never the contents). The
validation refuses a loadout whose sockets the glb lacks (`MUZZLE_L/R`, `MISSILE_POD`, a new `BACK_R` for the
Sapper's mortar, added to its glb by `mech_kit.py`).

## 4. Cockpit, effects, sound

- **WEAPONS page** (`weapon-select`): the signature station first, marked with a star, and its own state line read
  from the gun state the shots fire from: `BURST 2/3`, `LOCKS 7/12`, `CHARGE 60 %`, `HEAT 45 %`, `TOF 11 s`.
- **Effects** (`combat-fx` recipes): `muzzle_boom` (three flashes), `beam_lance` (`weapon-kinds`' beam, orange-white),
  `muzzle_steam` and `burst_steam` (the vent recipe, large), `muzzle_mortar` (a tall smoke ring), and the existing
  rocket and missile trails.
- **Sound cues for `audio`**: `boom_cannon`, `lance_loop`, `rack_salvo`, `steam_cannon`, `rocket_ripple`,
  `rail_fire` (from `weapon-kinds`), `breaching_gun`, `mortar_thump`, `mortar_whistle`.

## 5. Cost

Per fixed step, the most a single mech adds: sixteen rockets in flight (the ripple), each the existing rocket path;
twelve missiles homing (the racks), the existing missile path; one 600 m beam (half the cost of `weapon-kinds`'
1 500 m beam: about 200 point tests). The mortar's solve is a square root a shot. Nothing new on the GPU beyond
recipes. Cloud sessions cannot measure; the owner's machine measures a ripple and a salvo in the city.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G20 | Each mech's signature weapon, as in section 1 | As listed: each one is the weapon its model already carries | **Decided**, owner 2026-10-26 ("Approve all"): as listed, the Sapper's changed by G21 |
| G21 | Is the mortar's 7 to 18 s flight too slow to be fun | Keep it: it is the siege weapon; the TARGET page counts it down | **Decided**, owner 2026-10-26: "maybe a grenade launcher?": a 30 degree, 60 to 600 m grenade launcher (section 2) |

## Built, and the calls made on the way

Built 2026-10-26 on the owner's "Approve all" and "Go and implement"; "Use recommended for survey" for the choices on
the way:

- **Loadouts**: `data/mechs/<id>/weapons.ron`, read beside the mech file by `MechConfig::load` (the player's mech,
  the career's designs, enemies), not named from the generated mech file, so the generator and the eight generated
  files are untouched. `cb-tools validate` refuses a gun whose socket the glb lacks.
- **Sockets the models already carry**: the Harrow's racks fire from `MISSILE_R`, the Mule's ripple from `ROCKET_R`
  (the pods do not alternate yet), the Sapper's launcher from `CANNON_R`. No model was rebuilt.
- **Bursts and salvos share one queue** (`Guns::queue`, `take_queued`): the boom cannon's three, the ripple's
  sixteen and the racks' twelve leave the same way, each along the aim at its own moment.
- **The WEAPONS page** lists the signature first and stars it (`STA 1*`), with its state from `Guns::state` and the
  pod's `LOCKS n/12`. The TARGET page's time of flight, and the effects and sounds of section 4, are not built.
- **The Picket's scope** is `railgun-scope`'s, added there.
