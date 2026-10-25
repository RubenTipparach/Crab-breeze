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
- **Landing on it.** The deck, the shoulders and the pylon walkways are supports. The fortress is a
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
over open ground. Whether a fall on the city counts as city damage is G23.

### Its look (owner, 2026-10-25)

"make the crab more ornate with spikes and more armor like geometry", "get rid of the square plates", "I prefer
something blue and orange for the crab armor, use same armor techniques as the mech", "paneling must look organized
and intentional", "each element has paneling that compliments its shape", "NEVER HAVE A CROSS in the seams", "you want
t junctions, and about 20-30% diagonal lines", "we want normal maps for large and small details".

- Spikes all round the rim, across the shell, two great horns and a row of teeth at the bow, back-swept spines at
  the stern, fins along the shoulders, spines on every leg's knee, spikes on the thruster plates and a keel of spikes
  down the belly; instanced, one draw for the shell's and one per thruster.
- Painted the mechs' way, with the plating kit (`tools/textures/gen_mech_plating.mjs`): neutral paint tinted by the
  `hermit` palette in `tools/blender/mech_palettes.py` (owner: "main color orange, secondary a dark orange", "more
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

- `data/world/fortress.ron`: everything in sections 1 to 5 with the unit in the key (`mass_t`,
  `thruster_max_share`, `sink_drag_per_m`, `list_deg_per_weight`, `launch_interval_s`, parts with
  `hp`, `armour_mm`, `weak_point`), validated on load.
- `cb-core::fortress`: the lift rule, the pose step (altitude, list, drift on the racetrack), part
  states, launch and drop timers, the downwash test. Pure and deterministic (fixed step, seeded).
- `cb-core::support`: `support_at` gains the moving-surface velocity.
- `cb-app::fortress`: spawns the block-out (then the Blender model), its parts as entities with hit
  shapes, the targeting boxes, the downwash and plume effects, the cockpit's lift readout.
- `data/missions/defence.ron`: wave 3 brings the fortress (G16).

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

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G15 | The fortress's size and how it dies | 420 m (mechs are 24 m now), 600-900 m up, eight lift thrusters; it falls when a side is down to one | provisional (survey G15, reworded) |
| G20 | Its name | the Hermit (a crab that carries its own shell) | provisional (survey G20) |
| G21 | How it falls | the side rule, then the hull thrusters: three outer on one side open the hull, two hull thrusters on that side bring it down | provisional (survey G21; the hull thrusters and the plates are the owner's, 2026-10-25) |
| G22 | How the pilot gets up and stays up | the jets as they are; land on the deck, shoulders or a pylon to recharge | provisional (survey G22) |
| G23 | Does its crash count as city damage | yes: the pilot chooses where it falls (stop its drive first, or wait) | provisional (survey G23) |
| G24 | Who guards the deck | 2 Shrikes and 4 Mites aboard, a Mite up each lift every 60 s | provisional (survey G24) |
