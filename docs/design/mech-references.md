# Mech references: what to take from them

The owner, 2026-10-24: "we might need to study pictures of mechs", then a run of pictures with a word on each.
This page records what they showed, what each one teaches, and the rules that follow for Crab Breeze. CLAUDE.md
section 10 still holds: these are studied for principles (how a walking machine reads as heavy, practical and
believable), never copied for a silhouette, a limb or a weapon. Crab Breeze's mechs are its own 1980s industrial
machines (survey G1) on digitigrade legs (survey M2), about 24 m tall.

No pictures are kept in the repository: the fiction references are other studios' art. Where a picture is free to
reuse, it is linked.

## What the owner showed

| Reference | The owner's word | What it shows |
| --- | --- | --- |
| MechCommander 2 box art (BattleTech, 2001) | "some cool mechs here" | A lance of BattleMechs advancing through fire: big torsos carried high on long bird legs, guns on both arms, missiles on the shoulders |
| The Mad Cat, or Timber Wolf, from the MechCommander cinematics and its icon | "the cat", "very iconic" | The body is one rounded hull slung between the hips with the cockpit canopy on its sloping nose. Reverse-jointed legs carry it, box missile racks sit on top of the shoulders, and the arms hang low as gun pods |
| MechCommander (1998) box art | "chicken legs" | A heavy walker on reverse-jointed legs, its weight carried well forward of the hips |
| Scopedog (Armored Trooper VOTOMS) | "was very practical" | A small, boxy, olive-drab machine with a camera-turret head and simple slab armour, made to be produced and repaired in numbers. In its show it skates on rollers in its feet. The pilot climbs in when its chest and head hinge open |
| RX-79[G] Gundam Ground Type (08th MS Team) | "if we do gundams, I'd want something practical looking like this" | A field-worn army machine with chipped, faded paint and blocky armour. It carries a box-magazine gun, has a backpack and vents, and walks through jungle mud beside a tank |
| An olive field mech with its chest open (a stock 3D render), the Scopedog's open chest and head (a 1/12 figure), and an 08th MS Team Gundam's open hatch | "cockpit integration" | Where the cockpit sits and how it opens. All three put an upright seat in the middle of the chest, between the shoulders, and open it from the front in two parts. On the olive mech, the top half and the head hinge up and forward while the seat sits in a narrow tub running down to the pelvis. On the Scopedog, the head and upper chest lift away and the front drops. On the Gundam, the upper hatch flips up with monitors on its inner face and the lower hatch folds down |
| Ironhorse (our own detailed 12 m mech) | "this new iron horse is great" | Dark plating, a banded torso, drive-wheel knees, practical hardware everywhere |

Named earlier as references (CLAUDE.md, section 1): Armored Core (the core and its screen-walled cockpit),
Titanfall (kneeling to board, the hatch opening in stages) and the Gundam all-view pod.

Real walking machines, free to look at:
- [Menzi Muck M545 walking excavator](https://commons.wikimedia.org/wiki/File:2018-10-30_(850)_Walking_excavator_Menzi_Muck_M545_at_Mariazellerbahn_in_Rabenstein_an_der_Pielach,_Austria.jpg)
  (CC BY-SA 4.0). A cab on a slewing body, four legs, and wheels on the legs: it walks over rough ground and
  rolls on flat ground. That is Crab Breeze's walk-and-skate (survey M4) in a real machine.
- [Kuratas, from behind](https://commons.wikimedia.org/wiki/File:KURATAS_-_his_back.jpg) (CC BY 2.0). A real
  4 m piloted robot. Its legs are hydraulic cylinders, hoses, coil springs, bolted brackets and rust: the joints
  look the way a working joint has to look.

## What to take

1. **Chicken legs are the silhouette.** Reverse-jointed, digitigrade legs (M2), heavier than the body (the owner
   earlier: "legs on these mechs to be bigger than the torso, just for balance"). The weight sits forward of the
   hips, so the machine always looks like it is leaning into a step.
2. **The body can be one hull between the hips, with the cockpit in its nose.** The Mad Cat pattern: a sloped nose
   with a canopy. It fits Crab Breeze's sealed screen cockpits, since the nose is where the screens look out, and
   it suits the owner's top two block-outs. Low bank's canopy swings forward over the nose, and Panorama's
   clamshell splits the nose.
3. **Practical over pretty.**
   - Olive drab, gunmetal, faded and chipped paint, stencils and hazard markings. No white show paint (the owner
     on Foreman: "I dont like the grills or the white paint").
   - Boxy slab armour that could be cut from plate, with the joints left visible.
   - Hydraulics, hoses and springs where a joint needs them, as on Kuratas and our Ironhorse and Foreman.
4. **Weapons look like equipment**: box missile racks on top of the shoulders, gun pods for forearms or
   box-magazine rifles carried in a hand. Each one shows how it is fed and reloaded.
5. **Wheels in the feet for skating**, as on the Scopedog and the Menzi Muck. Survey M4 already chose powered
   wheels the pilot toggles on. The feet should show where the wheels are when they are folded away.
6. **The head is a sensor, not a face.** The Scopedog's camera turret and the Ground Type's visor and mount are
   what a practical machine carries. The pilot is in the body and sees through the screens.
7. **The cockpit lives in the middle of the chest and opens from the front in two parts** ("cockpit integration"):
   the top half up, the bottom half down, sometimes with the head going with the top. That is exactly the Panorama
   block-out's clamshell, one of the owner's top two, carried by the chest.
8. **Boarding is mechanical and visible.** The Scopedog's chest hinges open and Titanfall's titans kneel: the
   staged openings on the cockpit block-outs (clamps, rams, steam) are the same idea.

## The first two built on these rules

The owner, 2026-10-24: "Where's the new mechs?", "We still need those new mechs". Two new chassis, as 24 m low-poly
block-outs first (the owner's rule: "I prefer you make low poly low textured verions before going full detail"), in
`tools/blender/build_mech_blockouts.py`, each carrying its cockpit block-out at true size where the pilot sits:

| Mech | After | Cockpit | What it takes from the rules |
| --- | --- | --- | --- |
| Harrow | The Mad Cat | Canopy, in the nose | 2: one hull slung between the hips, its nose cut to a slope, the canopy rising out of it. 1: heavy chicken legs, 2.4 times the hull's volume. 4: box missile racks on the shoulders (tube grids cut by booleans), gun-pod arms with magazines. 5: a skate wheel in a housing at each heel. 6: a camera mast, not a face. Olive drab. |
| Roustabout | The Scopedog | Titan, in the middle of the chest | 7: the chest splits at the cockpit's seam; the upper chest, the camera turret with it, is the Titan's top half that rises and tilts back, the lower front its step. 4: a box-magazine rifle in one hand, a pipe grab in the other. 3: a backpack with vents, slab armour, gunmetal with orange hazard. 5: wheel feet. |

Then eight more (owner, 2026-10-24: "I need like 10 mech mock ups", "it's not like something we need highres
either"), each after a 1980s working machine on chicken legs, each carrying a different one of the ten cockpit
block-outs, so every cockpit has a chassis to be judged in. Contact sheets: `docs/screenshots/mechs/blockouts_new.png` and `blockouts_new_side.png`.

| Mech | After | Cockpit | Its tell |
| --- | --- | --- | --- |
| Grader | A road dozer | Panorama | A dozer blade on push arms from the hips, a ripper behind, a beacon bar |
| Logger | A forestry harvester | Low bank | A low wide hull, a knuckle crane with a log grapple on its back, a felling head with a saw |
| Lineman | A utility bucket truck | Cinema | A telescoping boom with a work bucket off the shoulder, a cable drum, insulator posts |
| Driller | A mine drill rig | All-view sphere | The ball held in a ring frame, an auger arm, a lattice derrick on its back |
| Bowser | A fuel tanker | Octagon | A tank across its back with hazard diamonds, a hose reel arm, a filler boom |
| Mule | A mine haul truck | Twin tier | The heaviest legs, a canopy roof over the cab, a dump bed on its back |
| Pumper | A fire engine | Tile wall | Water monitors on both shoulders, hose reels at the hips, a ladder down its back, a light bar |
| Ploughman | A farm tractor | Excavator cab | A bonnet with a grille and stack, tractor wheels for hips, a plough share, a three-point hitch |

Each of the ten has its own proportions (owner, 2026-10-24: "All 10 mechs have roughly the same ratios. Id like more
variation in arm size torso size and leg shape"; `VARIETY` in the generator):

| Mech | Torso | Arms | Legs |
| --- | --- | --- | --- |
| Harrow | 1.1 | 0.8 | Knee far forward, hock high and far back, heavy slabs |
| Roustabout | 1.08 | 1.4 | Short and stocky, piston shins |
| Grader | 0.85 | 1.0 | The thickest slabs, a low hock; its blade frame hangs off the hips |
| Logger | 0.9 | 1.1 | Stilts: a near-flat thigh, a long thin piston shin, the hock high |
| Lineman | 0.85 | 0.7 | Thin pistons, thigh and shin |
| Driller | 1.3 | 1.5 | Lattice shins under a big chest and big arms |
| Bowser | 1.1 | 0.75 | Upright and stocky, the knee high, the hock low |
| Mule | 1.0 | 1.3 | The heaviest slabs, thighs nearly meeting |
| Pumper | 0.9 | 1.3 | Thin piston shins, the knee forward |
| Ploughman | 1.05 | 0.65 | Round piston thighs, the hock almost on the ground |

Torso is the upper body's scale about the top of the waist (the cockpit moves with it and keeps its true size); arms
scale about the shoulder. Every torso stands on a 2 m waist and every leg is slimmer than the first pass (owner,
2026-10-24: "arms clip through the leg. Perhaps torso a little higher, legs a little less thick"). The generator checks
every arm and tool frame against every leg piece with a mesh intersection test and writes what it finds to the
manifest (`arm_leg_clips`): none on any of the thirteen. Contact sheets, front and side:
`docs/screenshots/mechs/blockouts_new.png`, `blockouts_new_side.png`.

Bigger arms where the job needs them (owner, 2026-10-24: "larger arms on some for punching, larger arms for bigger
guns"): the Mule's fists and the Roustabout's left hand are punching arms (`fist()`: a thick forearm with a piston,
a fist block with a knuckle plate); the Harrow's gun pods are longer and heavier, fed from the outside; the Bowser's
cannon is a twin-barrel autocannon with a drum magazine on a thick forearm; the Roustabout's rifle is heavier.

And two humanoids after the Gundam (owner, 2026-10-24: "Make a couple Gundam variant", after the RX-79[G]: "if we do
gundams, I'd want something practical looking like this"; of a first pass, "None of those look like gundams"; of a
second with the V fin and the white, blue and red, "no dont make it obvious"). `gundam()` takes the build and leaves
the badges: a broad chest with slatted intakes over a narrow waist, front and side skirts, big rounded shoulder pads,
long legs with knee pads and shins flaring at the ankle, a backpack with thrusters and exhaust stacks; but a helmet
with one visor band, a sensor brow and a single swept antenna, no V fin, no tricolour, no buckle, in Crab Breeze's own
field colours with orange hazard stripes. Close-up: `docs/screenshots/mechs/gundams.png`.

| No. | Mech | After | Cockpit | Its tell |
| --- | --- | --- | --- | --- |
| 11 | Picket | The RX-79[G]'s build, in sand and olive | Panorama (the upper hatch flips up, the lower folds down) | A 100 mm machine gun with a box magazine, the tall shield, a radio whip on the backpack |
| 12 | Sapper | The Full Armor idea, in gunmetal and olive | Titan | Bolt-on armour plates, a long cannon over the right shoulder, a missile pod on the left forearm, a rifle |

The contact sheets number the twelve new mechs 1 to 12 (owner, 2026-10-24: "can you add numbers to those mechs"),
in the order Harrow, Roustabout, Grader, Logger, Lineman, Driller, Bowser, Mule, Pumper, Ploughman, Picket, Sapper;
the page's buttons carry the same numbers.

The Foreman and the Ironhorse stay (owner: "I think iron and forman are solid designs"); their detailed builds now
carry the Panorama and the Excavator cab, and their chests open as one body with them (`docs/design/cockpit.md`, "The
four in detail"). The test for each block-out is still whether its silhouette reads as a practical machine on
chicken legs with its cockpit in the body, from the block-out alone; the owner picks before any is detailed.

## Combat mechs, built from parts (13 to 22)

The owner, 2026-10-24, picking 1, 2, 6, 8, 11 and 12 to build in detail ("Any cockpit style should do. They all are
pretty standard"), then: "I need more combat mech mockups. Your 12 mechs are mostly trash because they are random farm
trucks doing stuff..I want combat mechs with guns like in armored core". So the next ten are built the way Armored Core
builds its machines, from interchangeable parts, each part one function in `tools/blender/build_mech_blockouts.py` and
each mech one line of `COMBAT` (`--combat` writes `assets/models/mech_combat.json`):

- **Legs**: reverse joint (a high forward knee, a long shin back to the hock, three claws, a calf booster, a heel skate
  wheel), biped (a big knee guard, a shin flaring over the ankle with a booster in its calf, a long foot), tracks (two
  track units, road wheels and sprockets, skirts, a turret ring) and four legs (high knees, far-out pads, a belly jet).
- **Core**: a side profile extruded across with its front raked back, the cockpit block-out in a well cut in the front
  at true size, armour layered either side, a collar.
- **Heads**: mono (one lens in a raked helmet), visor, crest (swept antennas), sensor (three lenses, rangefinder ears),
  dome (a lens ring).
- **Hand weapons**: rifle, gatling (six barrels and a drum), bazooka, sniper rifle, shotgun, pile driver, shield (with a
  short gun in the hand), grenade launcher.
- **Back units**: missile box, vertical missile cells, shoulder cannon, railgun, radar dish, rocket pod.
- **Boosters**: a back pack with two main nozzles, swept wing vanes for the flyers, side jets.

The silhouettes and the parts are Crab Breeze's own: angular slab armour in field colours with a hazard accent, 1980s
hardware (box magazines, belt drums, missile boxes); nothing is copied from a game. Contact sheets, three-quarter and
side: `docs/screenshots/mechs/combat.png`, `combat_side.png`. No arm touches a leg on any of the ten (the same mesh
intersection check).

| No. | Mech | Role | Legs | Right hand | Left hand | Back | Cockpit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 13 | Sabre | The standard | Biped | Rifle | Shield | Missile box, radar | Panorama |
| 14 | Swift | Light, fast | Reverse joint | Rifle | Gatling | Two rocket pods, wing vanes | Canopy |
| 15 | Bulldog | Heavy | Tracks | Gatling | Gatling | Two shoulder cannons | Excavator cab |
| 16 | Tarantula | Artillery | Four legs | Rifle | Gatling | Cannon, vertical missiles | Low bank |
| 17 | Hatchet | Brawler | Reverse joint | Shotgun | Pile driver | Rocket pod, missile box, side jets | Titan |
| 18 | Longbow | Sniper | Light biped | Sniper rifle | Rifle | Radar, missile box | Cinema |
| 19 | Anvil | Heavy assault | Heavy biped | Gatling | Grenade launcher | Two missile boxes | Tile wall |
| 20 | Skylark | Flyer | Light biped | Bazooka | Rifle | Two vertical launchers, wing vanes | Twin tier |
| 21 | Brigand | Raider | Reverse joint | Bazooka | Gatling | Railgun, missile box, side jets | Octagon |
| 22 | Rampart | Fortress | Tracks | Gatling | Shield | Railgun, vertical missiles | All-view sphere |

Tracks and four legs stand outside survey M2's digitigrade rule and the owner's "legs bigger than the torso": they are
here because the owner asked for Armored Core's range, and the survey decides whether they stay.
