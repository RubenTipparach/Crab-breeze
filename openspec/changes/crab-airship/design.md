# Design: crab-airship

The fortress as the pilot meets it: a shape in the cloud base over the city, flak bursting on the way up,
a deck to land on, a fight on the deck, a walk out along a leg pylon to a thruster, and the whole thing
rolling over and going down. Mockup: `docs/mockups/crab_fortress.html` (block-out, at true scale with a
24 m Foreman and the crab mechs it carries). All numbers are first estimates, tuned from a run's log.

Frame: metres, +Y up, the fortress faces +Z, origin at the middle of the hull. Weight is in newtons.

## 1. The body

| Piece | Size | Notes |
| --- | --- | --- |
| Carapace hull | 420 x 280 x 70 m | A flattened oval shell, domed on top; the crab's back |
| Flight deck | 360 x 80 m, on the spine at y = +49 m | Two bow catapults, two hangar lifts (24 x 30 m), runway markings |
| Shoulders | two 40 m terraces either side of the deck, 6.4 m lower | Where flak turrets and deck guns sit; landable |
| Bridge | the bow tower and two eye stalks, 34 m above the deck | The crab's eyes: green lenses, fire control |
| Leg pylons | eight, four a side, 14 m wide walkways from the hull edge out and down to each thruster | Landable; a mech walks out along one to the thruster's intake |
| Lift thrusters | eight nacelles, 30 m across, 40 m tall, under the pylons' knees, nozzles 70 m below the rim; 400 m across the outermost | Exhaust straight down |
| Drive thrusters | two at the stern, 20 m across | Push it along at 15 m/s |
| Belly | the folded abdomen plate, two drop bays (30 x 40 m doors) | Crab mechs fall from here on chutes |

Mass 60 000 t, so its weight `W` is 588.6 MN.

Altitude: it cruises at 900 m between cities and comes down at 3 m/s to 600 m over the city it attacks,
then flies a 2.4 km racetrack over it at 15 m/s while it launches and drops.

## 2. Lift: how it stays up and how it falls

One rule in `cb-core::fortress`, pure and tested, and the cockpit's lift readout calls it (CLAUDE.md 6.1).

- A thruster's nominal thrust is `W / 8` (73.6 MN); its most is `T_max = W / 4` (147.2 MN).
- Each side must carry `W / 2`. A side's lift is `L_s = min(alive_s * T_max, W / 2)`.
- So a side holds with two or more thrusters left. Its live thrusters' throttle is
  `(W / 2) / (alive_s * T_max)`: 50 % with four, 67 % with three, 100 % with two (they glow white hot).
- **It falls when either side is down to one thruster or none.** Three kills on one side do it; spread
  evenly it holds through four kills (two a side, every thruster at 100 %) and falls on the fifth.
- Vertical: `a = g * (L_L + L_R) / W - g - k * v * |v|`, `k = 0.008 /m`. Holding, `a = 0`. One side at
  one thruster: `L = 0.75 W`, it sinks at up to 17.5 m/s (about 37 s from 600 m). One side at none:
  `L = 0.5 W`, up to 24.8 m/s.
- Roll: it lists toward the weaker side, toward `theta = 100 deg * |L_L - L_R| / W` at 4 deg/s: 25 deg
  with one thruster left on a side, 50 deg with none.

### 2b. The hull thrusters (owner, 2026-10-25)

"The crab mech once thrusters are destroyed. Have it open up more thrusters that are built into the hull. It is
only vulnerable from directly above or below the mech where the port holes are exposed."

- Six more thrusters are ducts through the carapace, three a side (x = +-104 m, z = -80, 0, +80 m), each a
  port hole 23 m across top and bottom under sliding 300 mm doors. Each carries at most `W / 6` (98.1 MN).
- They are sealed while the outer thrusters hold. When a side's outer thrusters can no longer carry `W / 2`,
  the doors slide open over 3 s with steam out of every port, and the hull thrusters light at 4 s. Meanwhile
  it sinks (about 30 m). Then a side's lift is `L_s = min(n_s * T_max + h_s * W / 6, W / 2)` and it holds
  again: any sink dies away (time constant 1.2 s) and the list comes back level at 4 deg/s.
- It falls when a side is short of `W / 2` even with its hull thrusters. With one outer thruster left on a side
  that takes two of that side's three hull thrusters (`W / 4 + W / 6 < W / 2`); with none left, one.
- A hull thruster is hit only through a port hole, and only by a round travelling within 30 deg of the
  fortress's vertical: directly above or below it. Anything else meets 300 mm of hull or door.

Tests: `a_side_with_two_thrusters_holds_its_half`, `three_kills_on_one_side_open_the_hull_thrusters`,
`the_hull_thrusters_hold_a_side_with_one_outer_thruster`, `two_hull_thrusters_lost_on_that_side_bring_it_down`,
`a_hull_port_takes_damage_only_from_within_30_deg_of_vertical`, `two_kills_a_side_hold_at_full_throttle`, `a_falling_fortress_lists_toward_its_weak_side`,
`the_sink_speed_never_passes_its_terminal_speed`, `the_lift_readout_is_the_rule`.

## 3. Its parts and how each is hurt

Every part is its own target (boxed, lockable, `targeting`) and takes hits through the one
`weapons::fly` and `Damage::resolve`. The pilot's weapons today: 35 mm autocannon (90 mm, 12 damage),
120 mm cannon (260 mm, 90), shoulder missile (150 mm, 45).

| Part | Count | Hit points | Armour | Weak point | Lost, it |
| --- | --- | --- | --- | --- | --- |
| Lift thruster | 8 | 800 | a 300 mm plate over the outboard half and a skirt below the nozzle (owner, 2026-10-25: "armor on the thrusters facing outward. So the player would have to shoot at the thrusters from under"); the inboard side is exposed, 60 mm (owner, 2026-10-25: "should be damageable from this side") | the nozzle below and the grille on top: 40 mm, damage x2 | stops lifting (section 2); the leg breaks off at the knee and falls, exploding where it lands and taking the buildings there (owner, 2026-10-25: "when a pylon is destroyed it should fall off and fall onto the ground and explode") |
| Hull thruster | 6 | 900 | 300 mm hull and doors; sealed until it opens them (section 2b) | the port holes, 40 mm, damage x2, from within 30 deg of vertical only | stops lifting (section 2b) |
| Drive thruster | 2 | 600 | 140 mm | | slows the drift; both lost, it hangs where it is |
| Flak turret | 10 | 150 | 60 mm | | stops firing |
| Beam turret | 4 | 250 | 120 mm | the lens while charging, 40 mm | stops firing |
| Deck gun | 8 | 100 | 30 mm | | stops firing |
| Missile launcher | 4 | 300 | 60 mm | the tube faces while a salvo leaves, 40 mm | stops firing |
| Missile battery | 2 | 200 | 60 mm | | stops firing |
| Catapult | 2 | 400 | 100 mm | | launches stop from it |
| Drop bay | 2 | 600 | 140 mm | open doors, 40 mm | drops stop from it |
| Hangar lift | 2 | 400 | 100 mm | | no more deck guards from it |
| Bridge | 1 | 1 200 | 140 mm | the two eye lenses, 40 mm | every turret's aim error doubles |

So a thruster takes 9 cannon shells or 67 autocannon rounds on its inboard side, 5 shells or 34 rounds into
its nozzle or grille, or 18 missiles; nothing gets through the outboard plate. The way in is from under it (or off its pylon, down into the grille), inside the beam turrets' reach. A
hull thruster takes 5 cannon shells or 38 autocannon rounds through a port, from straight under or over it.

## 4. What defends it

| Defence | Where | Weapon | Range | Rate |
| --- | --- | --- | --- | --- |
| 10 flak turrets | the rim and shoulders | twin 105 mm, shells at 900 m/s bursting within 10 m (1 MJ) | 1 800 m | 2 rounds/s each, aim error 1.5 deg |
| 4 beam turrets | under the belly, between the thruster pairs | a beam: 1.5 s charge (the lens brightens, a tone in the cockpit), 3 s on, 10 damage/s to the part it touches | 900 m | every 6 s |
| 8 deck guns | the deck edges and pylon roots | 30 mm gatling at 1 000 m/s (40 mm, 1.5 damage) | 600 m | bursts of 20 at 20 rounds/s |
| 4 missile launchers | the shell's bow and stern corners (owner, 2026-10-25: "crab needs missile launchers") | a turning box of twelve tubes: ripple salvos of six lock-on missiles at 220 m/s (150 mm, 35 damage) | 2 500 m | a salvo every 9 s, a missile every 0.25 s |
| 2 missile batteries | aft deck | lock-on missiles at 220 m/s (150 mm, 35 damage) | 2 500 m | one every 5 s, 12 cells each |
| Fighters | two bow catapults, 24 in the hangar | the `fighter` unit (`invasion-forces`) | | one launch every 20 s, alternating catapults, up to 8 airborne |
| Dropped crabs | two belly bays, 12 aboard (Mites and Scuttlers) | the crab mechs (`crab-enemies`), on the one parachute rule | | a pair every 45 s, alternating bays |
| Deck guard | on the deck from the start: 2 Shrikes, 4 Mites | the crab mechs' own weapons and brain | | each lift brings a Mite every 60 s, up to 6 on deck, 8 in the hangar |

Turrets lead their target with the same lead the cockpit's lead marker uses (`targeting`). A turret
fires only where it has a clear line (the hull hides the pilot from the far side's guns), so the shoulders
and the pylons have blind spots a pilot can use.

## 5. Getting there and staying on

- **The climb.** Jets (`boost-and-skate`): 56 m/s2 of thrust, climb to 60 m/s, 20 s of burn, 8 s to
  recharge. 600 m from the ground takes about 10.7 s (9.3 s of burn left); 900 m about 15.7 s (4.3 s
  left). A tower roof cuts the climb. Flak and missiles reach out to the whole climb.
- **Landing on it** (owner, 2026-10-26, survey H4: "Approve all", the recommendation: reach it on the jets as they
  are and land on the deck, a shoulder or a pylon walkway to recharge and fight there). Measured on the shipped
  mechs: straight up from the city's ground at full lift every roster mech passes the deck's 649 m in 11.5 s with
  8.5 s of burn left. A mech that comes down on a platform keeps only its speed relative to it, and one that lifts
  off takes the platform's velocity with it (`Platform::ride`), so a pilot who matches the deck's 15 m/s and lands
  stands where they landed instead of skidding 11 m along it. The jets recharge as they do on the ground: 1.5 s,
  then empty to full in 8 s. The deck, the shoulders and the pylon walkways are supports. The fortress is a
  moving support: `support_at` returns the surface's velocity at the contact point (drift plus roll), and
  whatever stands there is carried with it. The crabs' feet hold to 35 deg of list; the pilot's mech
  slides above 20 deg (friction 0.36). Falling off is a fall: the jets brake it once recharged.
- **Downwash.** Under each live thruster a column 30 m across, widening to 60 m 150 m below: a mech in it
  is pushed down at 25 m/s2 and takes 15 damage/s on every part inside. It is drawn (shimmer, a dust ring
  where it meets cloud), so the pilot sees it. A thruster is attacked from its pylon or the side.

## 6. The fall

When the lift rule says it falls: launches and drops stop, every live thruster flares at 100 %, the
fortress lists and sinks (section 2), and anything not holding on slides off the deck (crabs past 35 deg
too). Burning breaks and smoke columns follow its damage (`mech-damage-fx` recipes). On the ground it
destroys every building cell under its footprint and within 60 m of it, throws debris (`debris-physics`)
and stays as a burning wreck. While both drive thrusters live it drifts along its racetrack at 15 m/s, so
where it falls is the pilot's choice: stop the drive first to drop it where it is, or wait until it is
over open ground.

**Its crash is city damage** (owner, 2026-10-26, survey H5: "Approve all", the recommendation). The cells its wreck
breaks (`Fortress::wreck_breaks`: every building whose middle is under its 280 x 420 m footprint or within 60 m of
it, `crash.half_size_m` and `crash.break_margin_m`) are the attacked city's own, so the one measure of a city's
damage, `mission::city_damage` (the share of its building cells destroyed), counts them with everything else that
broke: the defence loses on it, and when the fortress joins wave 3 (task 2.5) its wreck counts toward that wave's
50 % with nothing more to build. In its own mission the HUD line and the log carry the city's damage
(`CITY 2 DAMAGE 4 %`), and the win line says what the wreck cost the city. It drifts at 15 m/s on its circle; with
both drive thrusters lost its drift dies away over 20 s and it hangs, so a fall from there lands straight below,
while a fall with the drives running carries it on along the circle (392 m in the 26 s fall from 600 m with one side gone). Measured on seed
42: brought down with its drives stopped over the largest city, its wreck broke 75 buildings, 23.2 % of that city's
cells.

### Its look (owner, 2026-10-25)

"make the crab more ornate with spikes and more armor like geometry", "get rid of the square plates", "I prefer
something blue and orange for the crab armor, use same armor techniques as the mech", "paneling must look organized
and intentional", "each element has paneling that compliments its shape", "NEVER HAVE A CROSS in the seams", "you want
t junctions, and about 20-30% diagonal lines", "we want normal maps for large and small details".

- Spikes all round the rim, across the shell, two great horns and a row of teeth at the bow, back-swept spines at
  the stern, fins along the shoulders, spines on every leg's knee, spikes on the thruster plates and a keel of spikes
  down the belly; instanced, one draw for the shell's and one per thruster.
- Painted the mechs' way, with the plating kit (`tools/textures/gen_mech_plating.mjs`): neutral paint tinted by the
  `stratostacean` palette in `tools/blender/mech_palettes.py` (owner: "main color orange, secondary a dark orange", "more
  grey on this platform? red turrets?": an orange carapace, dark orange underside, belly and legs, a grey platform on
  top, red turrets and launchers; the colours picked from the owner's 42-colour palette: orange `#e98537`, dark
  orange `#94493a`, grey `#b0a7b8`, crimson `#ac2847` turrets, red `#ec273f` livery bands, `#2c1e31` structure),
  full PBR (colour, normal, ORM, emission, a livery mask).
- Its armour is designed for this hull, not tiled (owner: "paneling should look like pieces of custom made armor cut
  and fitted with precision", "you just generate them and tiled them ... that is not the point of panel design"). The
  features are written once, in `tools/mockups/fortress_layout.json`: the mockup builds the geometry from it and the
  kit lays the panelling out against it. Five sheets:
  - **fortress_shell** (the shell, top and underside): an armoured belt round the rim, broken by a root plate at
    each leg pylon and a plate round each drive; a round doubler framing each hull thruster port; big side plates
    between seams run out from the doublers, divided by deliberate diagonals; a bow plate on the centreline.
  - **fortress_belly**: bands between the abdomen's ribs, split on the keel and at either side by turns, a dark keel
    stripe down alternate bands.
  - **fortress_side** (every long wall: spine sides, shoulder walls and tops): bays between the hull's structural
    stations (bow, bridge, each pylon, each port, stern), so seams line up from part to part; a rail along the top;
    the pylon bays split by a diagonal.
  - **nacelle** (every cylinder, once round): a collar band, the body in four (two with a diagonal), a nozzle band.
  - **panel** (every other face): one fitted panel with a border, a split and chamfered top corners.
  - Precision: plates flush with a few millimetres' step between neighbours, seams as thin engraved grooves with a
    small chamfer, fasteners in even rows, hatches and grilles placed on purpose; every seam ends on another in a T
    (never a cross), about a quarter of them diagonal.
- Its guard crabs walk on two-bone IK: each leg's coxa yaws its plane onto the foot, femur and lower leg bend in it
  with the knee up; a foot stays planted until the body has carried it a quarter of the leg's reach from where it
  wants to be, then swings there in 0.3 s, one group of legs at a time (tripods for six legs, diagonal pairs for
  four). Under their chutes the legs hang a little limp and sway (owner: "use IK solver to actually move them
  correctly when they are grounded, if they are flying have them go limp slightly"). The game's version is
  `crab-enemies`' gait; this is the mockup's.
- Its guards turn on the nearest mech within 700 m they can see, and face out over the side with nobody near;
  its turrets, fighters and missiles take the nearest mech too, so friendly mechs flying round it draw its fire.

## 7. Data and code

- `data/missions/fortress.ron`: everything in sections 1 to 5 and 9 with the unit in the key (`mass_t`,
  `thruster_max_share`, `sink_drag_per_m`, `list_deg_per_weight`, `launch_interval_s`, each part kind's `hp`,
  `armour_mm`, weak point and hit capsule, each defence's gun), validated on load.
- `cb-core::fortress`: the lift rule with the hull thrusters, the pose step (height, sink, list, drift on its
  circle), part states, how a hit is taken (section 3, by the round's direction in the part's frame), the turrets'
  aim and fire clocks, the launch clock, the downwash test. Pure and deterministic (fixed step, seeded).
- `cb-core::world`: the `Scene` gains platforms, moving surfaces a body stands on (`support_at`), with the motion
  they carry a body by.
- `cb-app::fortress`: the mission's begin and end (`free-roam-missions`), the model, its parts' hit capsules, the
  turrets' fire through the one `Shots`, the fighters through `units`, the platforms, the fall, the legs breaking off,
  the crash, the readout.
- The defence's wave 3 bringing the fortress (G16) waits until the fortress has been played on its own.

## 8. Cost

- Triangles: block-out 30 000 (the plates and the six hull ducts add about 4 000). The detailed model: LOD0 220 000 (hull and pylons 120 000, parts
  100 000), LOD1 60 000 beyond 1.5 km, LOD2 12 000 beyond 5 km. The deck guard at its own LODs.
- Draw calls: the hull in 6 material groups, 51 parts each one mesh, barrels with their turret: about 90
  at LOD0, doubled by the shadow pass.
- Textures: the mech plating set (CLAUDE.md section 13) tinted to the invaders' palette, no new set: 4
  maps at 4 096 px, about 85 MB as BC7 with mips (360 MB if left RGBA8).
- CPU per fixed step: the pose under 5 us; 51 parts aiming and firing about 0.1 ms; up to 8 fighters and
  18 crabs on their existing brains.
- Effects: 8 downwash columns and 10 exhaust plumes (`rocket-plumes`), flak bursts as `combat-fx` recipes.
- All of it measured with `--bench defence` on the owner's machine; a cloud session measures nothing.

## 9. In the game: the first build (owner, 2026-10-25)

"looks good, build it in game, add an email in free roam to fight the crab fortress." The Stratostacean is a mission
of its own, started from free roam's mail (`free-roam-missions`).

**The model.** The block-out the owner approved is the mockup, so the game's model is exported from it:
`tools/mockups/export_fortress_glb.mjs` loads `docs/mockups/crab_fortress.html` headless and writes
`assets/models/fortress_stratostacean.glb` with three.js's glTF exporter, and `fortress_stratostacean.json`, its manifest
(triangles, materials, budget, node names). One implementation of its shape (CLAUDE.md 6.1): the page and the game
draw the same geometry from `tools/mockups/fortress_layout.json`. The livery mask is baked into each material's
colour map (the page mixes it in a shader), the normal and ORM maps go in as they are, the spikes' instances become
plain meshes and the effects (plumes, downwash, range shells) stay out. Nodes the game drives are named:
`PART_<id>` for each of the 51 parts, `YAW_<id>` and `PITCH_<id>` for each turret, `LEG_<id>` for each leg below its
knee, `DOOR_<id>` for the hull ports' doors. Budget 60 000 triangles. The Blender model replaces it later (task 3.1;
CLAUDE.md section 13 lists the exception).

**Its levels of detail** (owner, 2026-10-25: "crab fortress needs lod probably, like spikes should not render until we
are up close"). The spikes are detail: the exporter names their merged meshes `DETAIL_*` and the manifest counts the
triangles with them (LOD0) and without them (LOD1). The game draws a `DETAIL_` mesh only within `detail_range_m`
(700 m, fading out over the last 100 m) of its origin, the hull's middle for the shell's spikes and each nacelle
for its studs, with Bevy's own `VisibilityRange` (the trees' mechanism). From the ground 600 m below or a street
away, the hull, thrusters and turrets draw; on the climb, on the deck or under a thruster, the spikes are there. The
Blender model's LODs (task 3.1) replace this split.

**Where.** It comes over the city nearest the pilot when the mission starts, its hull's middle 600 m above that
city's ground, and flies a circle 380 m round the city's centre at 15 m/s (about 2.4 km a lap), nose along it.
One drive thruster lost halves the speed; both lost, it stops over 20 s and hangs.

**Hits.** Each part is a capsule from its node: thrusters along their axis, turrets round their mounts, the
catapults, bays and lifts as flat capsules, the sizes in data. The hull itself is seven capsules along its length
(the shell and the belly), with no hit points: a round meeting them stops on 300 mm. The hull thrusters are listed
before the hull, so a round down a port reaches the thruster. What a hit does is section 3, from the round's
direction and the point it struck in the part's own frame: a lift thruster's outboard half and skirt are 300 mm,
its inboard side 60 mm, its nozzle (below `-18 m`) and grille (above `+18 m`) 40 mm at double damage; a hull port
counts only within 30 deg of the fortress's vertical and only once the doors are open.

**Defences in this build.** The 10 flak turrets (shells that burst within 10 m of the pilot's mech as a 1 MJ
explosion), the 8 deck guns, the 4 missile launchers and 2 missile batteries (lock-on missiles on the existing
seeker), the 4 beam turrets (a charge the pilot sees, then 10 damage/s to the mech's torso while the beam is on and
the line is clear), and fighters: one off a live catapult every 20 s, up to 8 in the air, flown by the existing
`fighter` unit. Every turret turns at 60 deg/s, fires only within 5 deg of its aim and only at the pilot's mech while the pilot sits in it (an empty mech is no
threat; a cloud run found the parked mech destroyed before the pilot reached it), and
leads it with `weapons::lead`. Losing the bridge doubles every aim error. Not yet: the crabs dropped from the bays.

**The deck guard** (owner, 2026-10-26, survey H6: "Approve all", the recommendation: 2 Shrikes and 4 Mites on deck,
a Mite up each hangar lift every 60 s). `guard` in `fortress.ron`: the six start at the approved mockup's posts, the
Shrikes on the flight deck at z = +40 and -105 m, the Mites on the shoulders, each an enemy mech wearing its crab's
block-out on the defence raiders' brain and guns (a 25 mm autocannon, 700 m), spawned standing on the top under its
post (`Fortress::stand_at`). `Fortress::guard_lift` runs one 60 s clock: each time it comes round, each working lift
brings a Mite up onto the deck over it while fewer than 6 stand and the 8 below decks last; a lost lift brings no
more, and none come up while it falls. A guard keeps to the tops it stands on (`platform::keep_on`: walking out over
an edge stops it there, as a wall does) and does not hop on its jets; like every enemy it waits until the pilot is
in the seat. Their ids sit below the vehicles' so a hit on one is a mech's hit. They leave the world with the
mission. Not yet: crabs sliding off a deck listing past 35 deg (they keep to it).

**Standing on it.** The deck, the two shoulders and the eight pylon walkways are platforms: boxes in the fortress's
frame whose tops a mech or the pilot stands on (`support_at`), carried with the fortress as it flies, sinks and
lists. The walls of the spine and the bridge do not push yet (a block-out).

**Downwash.** A live lift thruster pushes a mech in the column under it (30 m across, widening to 60 m 150 m down)
down at 25 m/s2 and burns its torso at 15 damage/s.

**The legs.** A destroyed lift thruster's leg breaks off at the knee and falls under gravity, tumbling; where it
lands it explodes (30 MJ) and breaks every building cell within 25 m.

**The fall and the crash.** As section 6; on the ground it breaks every building cell within its footprint and 60 m
of it, throws a 200 MJ explosion at its middle and stays as a wreck. Won when it has crashed; lost when the pilot's
mech is destroyed.

**The readout.** The MISSION page and the HUD's mission line: `STRATOSTACEAN  LIFT PORT 100 %  STBD 67 %  ALT 600 M`, the
hull thrusters when open, `FALLING`, `DOWN: WON`. Every number is the rule's own (section 2).

Tests (cb-core): `the_lift_readout_is_the_rule` and the section 2 tests, `a_round_on_the_outboard_plate_does_nothing`,
`the_autocannon_cannot_hurt_a_cowl_but_doubles_on_a_grille`, `a_turret_does_not_fire_until_it_is_on_its_aim`,
`a_lost_catapult_halves_the_launches`, `nothing_launches_while_falling`,
`a_mech_standing_on_the_drifting_deck_stays_where_it_stood`, `a_mech_under_a_live_thruster_is_pushed_down_and_burned`,
`the_same_seed_flies_the_same_fortress`. (cb-tools) `cb-tools validate` checks every part in `fortress.ron` against
the glb's nodes.

Cost of this build: the model 60 000 triangles at most, about 40 draws (one per material and sheet; parts that move
are their own nodes); the turrets' and lift's step under 0.1 ms; the hit capsules 51 parts plus 7 hull, tested by
each shot's metre steps as a mech's parts are. Measured on the owner's machine with the mission running.

## 10. Where it appears, and the scripted pilot that brings it down (owner, 2026-10-30)

The owner: "build a playtest bot to defeat the crab fortress. requirement: crab fortress always spawns above nearest
city center".

- **Over the city's centre.** When the mission starts it appears directly over the centre of the city nearest the
  pilot (or the mech they sit in), `altitude_m` over that city's ground, then flies straight out at its speed onto
  its circle (`circle_radius_m`, 380 m: about 25 s) and circles from there. Before, it appeared on the circle,
  380 m off the centre. `Fortress::radius_m` is how far out it is; nothing else about its flight changes.
- **What the Foreman can reach.** Its torso aims from -20 to +25 deg (`pitch_limits_deg`). A lift thruster's nozzle
  hangs about 530 m over the city's ground, so a shot at it from the ground needs about 1.2 km of standoff
  (530 / tan 23 deg); the autocannon's rounds fly 6.6 km (1 100 m/s for 6 s), so that is in reach, and the nozzle
  is weak from any side (40 mm, double damage: 34 rounds each). A hull thruster is hurt only from within 30 deg of
  vertical (section 2b); no gun of a mech standing or flying can point that steeply. Missiles can: a locked
  missile turns 70 deg/s onto its target and climbs into the port from below. So the falling side's hull
  thrusters are **boxed and lockable while they are lit** (as the lift thrusters always are), and nowhere else
  (provisional, H7).
- **Two of three hull thrusters** (owner, 2026-10-30: "in order for the crab to fall, 2 out of three internal
  thrusters must be taken out"): lit, a side's hull thrusters hold its half, running past their rating, until it has
  lost `hull_kills_to_fall` (2) of them, whatever is left of its lift thrusters. The lift readout is the same rule.
- **It weaves** (owner, 2026-10-30: "have crab carrier move around a bit too so it's not so easy to shoot down"): on
  its circle its radius swings `weave_m` (120 m) either side, `weave_per_turn` (3) times a turn: 260-500 m out, its
  nose along the path.
- **The scripted pilot's order** (`Fortress::next_to_bring_down`, pure, in `cb-core`): the side with fewer live
  lift thrusters (port on a tie), its lift thrusters in the shape's order until the hull thrusters light (three
  down), then that side's hull thrusters. Three lift kills and two hull kills bring it down. As it falls the pilot
  gets 700 m clear and watches it crash and explode (8 s on the wreck); the script ends with a minute more.
- **Up close, on tapped jets** (owner, 2026-10-30, choice B: "you can tap jets so it recharges as you fly up"): for
  a lift thruster the pilot skates to within 600 m, then hovers 150 m off it, outboard (so the line clears the hull),
  the nozzle 12 deg up: the jets burn until it rises and coast at least 2 s so they cool (`playtest::tap_jets`;
  a 56 m/s2 jet holds a height indefinitely at about 30 % duty). It fires only when the first thing on the line from
  the selected gun's own barrel (`MechRig::muzzle`, the guns' rule) to the nozzle is the thruster (owner: "the bot
  isnt raycasting from gun barrel to target"). A single missile fires only at a full lock; a multi-lock pod (the
  Harrow's racks) holds the key 3.5 s, then lets the salvo go. The take is flown in the Harrow (owner: "the harrow
  you can see through the top of the mech, might be more fun").
- **The beat** `BringDownFortress(cap_s)` (playtest-pipeline): stand off the target thruster so its nozzle is at
  most 22 deg up, skating there when out of band and stopping in it; aim the view at the nozzle (the capsule's
  lower end, 13 m further along its axis, so the round enters the weak cap) led by `weapons::lead`; hold the
  autocannon; at a hull thruster, close to 120 m off it along the ground (nearly under it, so the missile climbs in
  within 30 deg of vertical; from 500 m out they came in about 45 deg off and did nothing), cycle targets until that
  thruster is selected and fire missiles at the lock. Done when the fortress crashes; failed if the pilot's mech goes down or `cap_s` passes. The script is
  `data/playtests/fortress-down.ron`. After the crash, `ClearHostiles(cap_s)` fights every hostile left (its
  fighters, its deck guard, any vehicle) nearest first, as `FightJob` does, until none is left (owner, 2026-10-30:
  "take out all hostiles too").

## 11. It goes nuclear (owner, 2026-10-30)

"Crab carrier when it impacts the ground goes nuclear in a big mushroom cloud and leave a giant Crater".
`crash.nuke` in `fortress.ron`:

- **The crater**: 520 m across (`crater_radius_m` 260), 60 m deep at its middle, a 15 m rim, scorched out to 470 m.
  It is dug through the city and its roads (`Deformation::dig_crater`; a shell's crater still leaves the city's
  ground alone), the same bowl-and-rim shape every crater has (`Deformation::carve`, one implementation).
- **Flattened**: every building and house within 450 m of the impact comes down, on top of the hull's own footprint.
- **The flash**: a light of about 1.4 x 10^10 lm out to 3 km for 4 s, a fireball 90 m across, a dust ring thrown
  along the ground (`nuke_flash`). No drawn shock shell: the first take showed it as a flat pale 450 m bubble
  (owner, 2026-10-30: "That violated our realism style").
- **The mushroom cloud** (`fortress::mushroom_cloud`): for 70 s, five times a second, a puff of smoke and fire up the
  stem (`nuke_stem`) and two rolling out of the cap (`nuke_cap`); the cap climbs to 900 m over 25 s (smoothstep) and
  spreads to about 290 m across. Placed by a fixed sequence, so the same crash builds the same cloud.
- **Cost**: about 350 bursts over 70 s, each 4-10 particles at most (sprites 35-50 m across), within the particle cap;
  one crater remesh of about 64 ground tiles at once (a hitch, measured on the next run, not guessed).

## 12. Every leg, then half the hull; all or nothing (owner, 2026-10-31)

The owner, after the recorded take: "all crab legs have to be destroyed, then half the internal thrusters would have
to be destroyed", "the crab carrier either looses thrust completely or it doesnt, so that might make the fall more fun
to watch. Even with one leg, it'll stay afloat", "the mech isnt flying often enough, it flew once, then skated around
and wasted time", "lets have it shoot down the stragglers after downing the crab carrier".

This replaces the fall rule of sections 2 and 2b and the "two of three hull thrusters" of section 10.

- **Its legs hold it.** While any of its eight lift thrusters is working, it holds its height. There are no sides, no
  sinking and no list. The live ones share the weight: each runs at `W / (n * T_max)`, so 25 % with eight, 100 % with
  two and, past its rating, with one. The readout's port and starboard lift both read `W / 2` while it holds.
- **The hull opens when the last leg goes.** With every lift thruster down, the doors open and the hull thrusters
  light (`doors_open_s`, `hull_light_s`). It holds through the opening: the ducts take the weight as they open.
- **It falls when half its hull thrusters are gone.** It falls once `hull_kills_to_fall` (3) of its six hull
  thrusters are destroyed, on either side. Then its thrust is gone at once, and it falls under gravity and drag
  alone (`a = -g - k v |v|`, terminal speed `sqrt(g / k)`, 35 m/s), level, until it hits. There is no partial
  lift and no list.
- **The scripted pilot's order** (`Fortress::next_to_bring_down`): every lift thruster, port side first, then
  three hull thrusters in the shape's order. Eight lift kills and three hull kills bring it down.
- **It flies.** The jets fire the mech up into an arc, and they recharge faster than it falls, so tapping them keeps
  it in the air indefinitely (owner, 2026-10-31: "occasionaly tapping space would allow the mech to stay in flight
  indefinitely"). The pilot never skates in the fight. It flies to every target on tapped jets (`playtest::hover`, up
  to 25 m/s, faster than the fortress's 15 m/s) up near the fortress ("its jet altitude shoould be near the crab").
  At a lift thruster it hovers 150 m off the nozzle, outboard. At a hull port it hovers 180 m under it and 90 m out
  (`BRING_DOWN_PORT_BELOW_M`, `BRING_DOWN_MISSILE_STANDOFF_M`), so its missiles climb in about 27 deg off vertical.
- **It clears the stragglers.** After the crash, `fortress-nuke` watches the cloud for 25 s and then runs
  `ClearHostiles`. That beat now aims at aircraft as well as mechs and vehicles: it leads a flying target with
  `weapons::lead`, lifts the view to it, and hovers on tapped jets when the target is more than 15 deg above the
  torso's aim limit.

Tests: `while_any_leg_works_it_holds_its_height`, `the_hull_lights_only_when_every_leg_is_gone`,
`it_falls_only_when_half_its_hull_thrusters_are_gone`, `with_no_thrust_it_falls_level_at_its_terminal_speed`,
`shooting_what_next_to_bring_down_names_brings_it_down_in_eleven_kills`, `the_lift_readout_is_the_rule`.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| H1 | The fortress's size and how it dies | 420 m (mechs are 24 m now), 600-900 m up, eight lift thrusters; it falls when a side is down to one | provisional (survey H1; G15 accepted the first 300 m sketch) |
| H2 | Its name | the Hermit (a crab that carries its own shell) | decided: **the Stratostacean**, the owner's own name (2026-10-26, survey H2); `name` in `data/missions/fortress.ron` is what the mail, the HUD, the TARGET page and the log say |
| H3 | How it falls | the side rule, then the hull thrusters: three outer on one side open the hull, two hull thrusters on that side bring it down | provisional (survey H3; the hull thrusters and the plates are the owner's, 2026-10-25) |
| H4 | How the pilot gets up and stays up | the jets as they are; land on the deck, shoulders or a pylon to recharge | decided (owner, 2026-10-26, survey H4: recommendation accepted, "Approve all"): every roster mech reaches the deck on its jets with 8.5 s of burn left; landing hands the mech's velocity over to the deck (section 5) |
| H5 | Does its crash count as city damage | yes: the pilot chooses where it falls (stop its drive first, or wait) | decided (owner, 2026-10-26, survey H5: recommendation accepted, "Approve all"); section 6 |
| H7 | How a lit hull thruster is reached | box and lock it while lit, so missiles climb into its port (no mech gun points within 30 deg of vertical) | provisional (2026-10-30, section 10) |
| H6 | Who guards the deck | 2 Shrikes and 4 Mites aboard, a Mite up each lift every 60 s | decided (owner, 2026-10-26, survey H6: recommendation accepted, "Approve all"); section 9, the deck guard. The cap of 6 on deck and 8 below are section 4's, kept |
