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

The Foreman and the Ironhorse stay (owner: "I think iron and forman are solid designs"); their detailed builds now
carry the Panorama and the Excavator cab, and their chests open as one body with them (`docs/design/cockpit.md`, "The
four in detail"). The test for each block-out is still whether its silhouette reads as a practical machine on
chicken legs with its cockpit in the body, from the block-out alone; the owner picks before any is detailed.
