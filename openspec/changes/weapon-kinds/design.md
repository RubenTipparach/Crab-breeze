# Design: weapon-kinds

## 1. The kind is data, resolved in one place

`WeaponSpec` (`cb-core::weapons`) gains `kind: ShotKind`, default `Shell`, replacing `rocket: bool`:

```rust
pub enum ShotKind {
    Shell,                                       // today's guns: gravity, no drag
    Rocket,                                      // today's rockets and missiles: straight, no drop
    Pellets { count: u8, spread_deg: f32 },      // shotgun
    Slug { charge_s: f32 },                      // railgun: gravity, fires after a charge
    Beam { range_m: f32 },                       // laser: instant, damage per second while held
    Plasma { burst_m: f32 },                     // straight, bursts at burst_m if it meets nothing
}
```

One function turns a pull into shots, `weapons::launch(spec, muzzle, aim, rng) -> Launch`, and one
into flight, `fly`, each with a single `match` on the kind inside `weapons.rs` (CLAUDE.md 6.2:
no kind matches spread across files). Everything after the hit (part armour and damage through
`mech-damage`, buildings, trees, blasts through `terrain-deformation`) is the existing path,
unchanged. Data files with `rocket: true` are migrated to `kind: Rocket` in the same commit; an
unknown key stays an error.

Pellet spread uses the seeded simulation RNG forked per mech and shot (CLAUDE.md 6.3), so a golden
seed gives the same pattern every run.

## 2. The four weapons

| | Shotgun | Railgun | Laser | Plasma cannon |
| --- | --- | --- | --- | --- |
| `kind` | `Pellets { count: 12, spread_deg: 1.5 }` | `Slug { charge_s: 1.0 }` | `Beam { range_m: 1500 }` | `Plasma { burst_m: 650 }` |
| Speed | 600 m/s | 2 400 m/s | instant | 180 m/s, no drop |
| Interval | 1.0 s | 4.0 s | continuous | 1.6 s |
| Penetration | 40 mm a pellet | 400 mm | 120 mm | 80 mm |
| Damage | 9 a pellet (108 all in) | 140 | 60 a second | 55 |
| Blast | 0 | 0 | 0 | 4.0 MJ |
| Heat (share of the bank) | 0 | 0.35 a shot | 0.25 a second | 0.20 a shot |

What the numbers mean in a fight (mechs are about 24 m tall, a torso about 6 m across):

- **Shotgun**: the cone's half-angle is 1.5 degrees and pellets spread evenly over its disc, so
  against a torso of 3 m radius: the cone is 2.6 m in radius at 100 m (all 12 pellets hit), 5.2 m at
  200 m (about 4), 7.9 m at 300 m (about 2) and 10.5 m at 400 m (about 1). Pellets drop like shells.
- **Railgun**: flight to 2 km is 0.83 s with 3.4 m of drop, so the lead marker and the scope still
  matter at range. The charge starts on the pull and the slug leaves 1.0 s later along the aim at that
  moment; releasing early cancels it with no heat. 400 mm beats every part in the current roster.
- **Laser**: each fixed step (1/60 s) it traces one ray from the muzzle along the aim up to 1 500 m,
  and the first thing it meets takes `60 / 60 = 1` damage that step if 120 mm beats its armour. A mech
  holding the beam on one part for 2 s does 120. Four seconds of firing empties the heat bank.
  `lead` for a beam is the target itself (light has no flight time), so the lead marker sits on the
  target.
- **Plasma**: 180 m/s is slow on purpose: 3.6 s to 650 m, so a skating mech sidesteps it at range.
  It bursts on contact or at 650 m (an airburst), and the 4 MJ blast does the damage around it through
  the existing blast path.

These are starting numbers in `data/`, to be tuned by playtest.

## 3. Weapon heat

The boosters' heat budget (`cb-core::mech::locomotion`, share used 0 to 1, locks when full, relights
below a mark) is generalised to a `HeatBank { burn, recharge_s, relight }` and used twice: the
boosters' bank as now, and a weapon bank per mech (`weapon_heat: (recharge_s: 20.0, relight: 0.4)` in
the mech's data: an empty bank refills in 20 s, 0.05 a second). A shot adds its share; a beam adds
`share_per_s * dt`. At 1.0 the bank locks and every heat weapon stops (the laser cuts out mid-beam)
until it is back under 0.4. Sustained fire: the laser fills the bank in 5 s (0.25 in, 0.05 out a
second), the plasma cannon in about 13 s, the railgun on its 4 s interval on its sixth shot, and
mixing them fills it faster: two rail shots (0.5 after the cooling between them) and three seconds of laser lock it.

**Decided (survey G23, owner, 2026-10-26: "Approve all", then "Go and implement"): the weapons have a bank of
their own, separate from the jets', refilling in 20 s.** A jet hop never blocks the guns and a hot laser never
grounds the mech. As built: the bank is `Guns::heat`, the same `Budget` type the boosters and skates use
(`cb-core::mech::locomotion`), cooled by the same `Budget::tick` at rest, so there is one implementation of a
budget that fills, locks and relights. Its numbers are `MechConfig.weapon_heat` (`recharge_s: 20.0`,
`relight_heat: 0.4`; the code default, which a mech's data may override). Each weapon's share is `heat` on its
`WeaponSpec`.

## 4. Fire groups and actions

Groups 0 to 3: right arm (`fire_primary`, Mouse Left), left arm (`fire_secondary`, Mouse Right),
missiles (`fire_missile`, F), back units (`fire_back`, G and Mouse4). `data/input/bindings.ron` gains
`fire_back`; validation allows groups 0 to 3. A beam fires while its group is held; a slug starts its
charge on the press.

## 5. Enemies and loadouts

Loadouts live in each mech's `weapons` list, so the combat mechs carry what their block-outs hold
(Hatchet a shotgun, Brigand and Rampart a railgun on the back).

**Decided (survey G22, owner, 2026-10-26: "Approve all", then "Go and implement"): anyone can carry every
weapon, on both sides.** This was not the recommendation (which split them: crabs the energy weapons, humans the
shotguns and railguns). Shotguns, railguns, lasers, plasma and multi-lock pods are open to human mechs and alien
crabs alike, and to the player through the hangar's parts list. As built:

- **The raiders** (`data/missions/defence.ron`, also every job's enemies) cycle seven loadouts: two autocannons, a
  rocket pod, a shotgun (a Hatchet), a laser (a Shrike), a plasma cannon (a Bruiser) and a railgun (a Brigand).
- **The first fight** (`data/world/encounters.ron`): the Pincher keeps its autocannon, the Bruiser carries a
  shotgun and the Tyrant a plasma cannon.
- **The hangar** (`data/career/parts.ron`): the rail rifle is now a charging slug with heat, and a shotgun, a laser
  and a plasma cannon are for sale for any arm station.
- Each AI's orbit range is set for its gun (a shotgun's 120 to 150 m, a railgun's 700 m). The AI pilot (`cb-core::ai`) fires
each kind through the same actions, with one rule per kind in its fire decision: shotgun inside
200 m; railgun when the lead error is under 0.3 degrees and the target is beyond 300 m; laser held
while the aim is on the target and the bank is under 0.8; plasma inside 450 m, led with `lead`.

## 6. Cockpit, effects and sound

- **Weapons MFD page** (`cockpit-mfds`): each weapon, its group key, READY / CHARGING (with the
  railgun's charge) / HOT, and the weapon heat bank, from the state above.
- **Effects** (`combat-fx` recipes in `data/fx/fx.ron`): a pellet cone muzzle blast and 12 tracers; a
  railgun streak that hangs straight in the air for 0.6 s with a blue ionised glow; a beam drawn as a
  glowing line from the muzzle to the hit with a burn mark where it touches; a plasma ball as an
  emissive sphere with a point light, a bright burst on impact.
- **Sound cues for `audio`**: `shotgun`, `rail_charge`, `rail_fire`, `laser_loop`, `laser_stop`,
  `plasma_fire`, `plasma_burst`, `heat_lock` (the cockpit's overheat alarm).

## 7. Cost

- **Pellets**: 12 projectiles a shot, each tested every metre along its step (10 m a step at
  600 m/s): 120 point tests a step per shot in flight, for under 0.7 s. Small next to the autocannon.
- **Railgun**: 40 point tests a step for under 1 s.
- **Beam**: a 1 500 m ray every step is the expensive one. Tested every metre it would be 1 500 point
  tests a step per beam. Instead: mech parts by an exact ray-capsule test (a few dozen capsules), then
  the ground, water, trees and buildings marched at 4 m with a 1 m refine around the first hit. About
  400 point tests a step. Budget: 0.1 ms a fixed step per beam, checked by a `cb-core` timing test
  (a measurement instrument, CLAUDE.md section 4). Cloud sessions cannot measure frame time; the
  owner's machine measures the effects' GPU cost.
- **Plasma ball light**: one point light per ball in flight, at most 8 lit (the oldest goes dark).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G22 | Who carries the new weapons | Human combat mechs: shotguns and railguns. Alien crabs: lasers and plasma | **Decided**, owner 2026-10-26: anyone can carry every weapon, on both sides (section 5) |
| G23 | Do weapon heat and jet heat share one bank | Separate banks | **Decided**, owner 2026-10-26: separate, refilling in 20 s (section 3) |

## Built, and the calls made on the way

Built 2026-10-26 on the owner's "Approve all" and "Go and implement"; where a choice came up that would have been a
survey question, the owner said "Use recommended for survey", and these were taken:

- **The old `rocket: true` still loads** as `kind: Rocket` (`WeaponSpec::shot`), so data on other branches keeps
  working; the shipped data is migrated, and a file giving both is refused.
- **The beam is resolved through `cb_core::weapons::trace`**: an exact ray-capsule test for parts, buildings and
  trees marched every 4 m and refined at 0.25 m. The same trace is the scope's rangefinder (`railgun-scope`).
- **Measured, and the march changed for it**: marched every 4 m the way section 7 first said, a 1 500 m beam cost
  2.8 ms a step (an Apple M4 Max, release, CPU only), because the terrain's height is a 3.3 us lookup and the hit
  test asked it twice a sample. Now the ground is sampled at a stride that grows with the beam's clearance (no slope
  steeper than 1.5 can reach it sooner) and bisected to 0.1 m: 0.113 ms 30 m over a city, still 13 % over the
  0.1 ms budget. The height cache in PR #75 (`perf-free-roam`) is the next cut. Every shell's hit test now asks the
  height once a sample instead of twice (`terrain_at`), the same rule, half the lookups.
- **A laser's `speed_m_s` and `interval_s` are unused** (it has no flight and fires every held step) but are kept
  in the data so every weapon has one shape; its lead is the target itself (`ShotKind::lead_speed_m_s`).
- **A plasma ball's burst distance** is its speed times its time aloft; it bursts through the existing blast path.
- **Muzzles**: a weapon fires from its own socket when the model has a node of that name (`CANNON_R`,
  `MISSILE_L`), else its arm's contract muzzle, so a back unit needs no new contract socket.
- **Mouse 4** reads `MouseBack` in the bindings (`MouseForward` is Mouse 5).
- **The beam is drawn as a stand-in line** (`combat::BeamsDrawn`) until `combat-fx` has its recipe; the hit
  already shows the existing impact effect.
- Not built yet: the effects recipes and sound cues of section 6, and the playtest (task 5.1). The beam's 0.1 ms
  budget is a measurement instrument (`a_beam_trace_costs_about_a_tenth_of_a_millisecond`, run in release).
