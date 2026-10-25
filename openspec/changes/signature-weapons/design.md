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
| Sapper | 35 mm autocannon (its rifle) | rocket pod (its forearm pod) | shoulder missile | **Siege mortar** |

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
| Siege mortar | `Shell`, `lob: true` | 45 degrees, charge set by range | 6.0 s | 120 mm (falls on the roof) | 120 | 20 MJ | 0 |

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
- **Siege mortar**: the gun lays itself at 45 degrees and sets its charge for the range to the aim point (100 to
  1 500 m), so the shell lands where the crosshair is, over whatever is between. Flight is `sqrt(2 d / g)`: 7.8 s to
  300 m, 14.3 s to 1 km, with an apex at a quarter of the range (250 m at 1 km): over any tower. The slowest weapon
  in the game and the only one that lands behind a building. The TARGET page shows the time of flight and a
  countdown to the splash.

## 3. Two new fields, in one place

Both on `WeaponSpec`, resolved in `weapons::launch` beside `weapon-kinds`' kinds (CLAUDE.md 6.2: one `match`, in
`weapons.rs`):

- `burst: Option<(u8, f32)>`: shots a pull and the gap between them, seconds. The pull starts the burst; the shots
  leave on the fixed steps the gaps fall on, each along the aim at its own moment; the interval runs from the last.
  Losing the arm mid-burst ends it.
- `lob: bool`: before launch, the elevation is fixed at 45 degrees and the muzzle speed set to
  `sqrt(g d)` for the horizontal range `d` to the aim point (the ground or building the crosshair is on), clamped to
  the gun's `range_m` (min 100, max 1 500). Pure: aim point, muzzle, g in; velocity out. Tested: a shell from the
  solved velocity lands within 1 m of the aim point on flat ground at 300, 700 and 1 500 m.

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
| G20 | Each mech's signature weapon, as in section 1 | As listed: each one is the weapon its model already carries | provisional (survey G20) |
| G21 | Is the mortar's 7 to 18 s flight too slow to be fun | Keep it: it is the siege weapon; the TARGET page counts it down | provisional (survey G21) |
