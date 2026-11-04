# Proposal: city-districts

## Why

The owner, 2026-11-04: "let's upgrade those city layouts. Make a three js mockups with lots poly buildings of how cities
are built today vs how we want them to look future".

Every city today is the same machine (`cb_core::world::layout`):
- one flat square 560-960 m across;
- a straight grid of 70-110 m blocks, with a 36 m arterial every fourth street;
- each block cut into equal 18-36 m lots, one box on each, taller toward the centre (up to 60 storeys);
- 6 % of blocks parks and 8 % of lots plazas;
- a ring of house rows round the edge, and straight highways out.

From the air every city is a grey square; at mech height every street looks alike.

The first proposal (a city of themed districts round a river) missed. The owner:
- "the roads are barely visible";
- "it appears like youre assuming rivers in every city? that cant be right" (rivers were taken out of the game);
- "it also looks like a giant circular slab of concrete with a highway around it".

They pointed instead at Fargo: "a smaller city with some tall buildings in the middle, and lots of housing around. It is
close to a grid but not quite, it has a highway cutting through the city ... where almost all the commerce is centered
around. houses are arranged in many blocks". Then: "have a couple of variations of larger downtowns", "another kind of
highway that doesnt quite cut through the downtown", "different shapes of downtowns", "I also like the bigger downtown
we had, but we would need to scale up housing to match that larger downtown".

## What changes

A city is a Fargo-like town on open ground:
- **The highway** carries the commerce. Big-box stores, car lots and gas stations line it, each with its car park. In
  a bypass city the commerce lines the spur avenue into town instead.
- **The downtown** sits beside the highway, small or large:
  - a small one has a few towers of 40-90 m among mid-rise;
  - a large one has the towers of the first mockup (setback tiers, round towers, twin slabs, crowned slabs, 60-250 m).
- **The downtown's shape** is compact, along one bending main street, or two cores (a low brick old core and a new
  tower core).
- **Block after block of houses** with yards, an alley, garages and trees, on grids that drift a few degrees from one
  neighbourhood to the next. Parks, schools and corner shops sit among them.
- **A large downtown grows its housing**: the city is 1.45 times wider, and apartment blocks stand near the centre.
- **An industrial park** stands at the highway's end.
- **A ragged edge** fades into farmland. No slab, no ring.
- **The highway** either runs through the middle (beside the downtown, never through it) or bypasses the city round
  its edge, with a spur avenue into town.
- **Water only where the island has it**: inland, or on a coast with a small harbour. No rivers.

**The mockup** is `docs/artifacts/city-layouts/city_layouts.html` (published, link in its README). It puts both cities
on one ground at true scale and one camera, from the air, at mech height and in plan, with every variation a button
away.

## Not in this change yet

Many systems read the square grid today:
- the city record and collision;
- roads and traffic lanes, and the lamps;
- the suburbs and small buildings;
- the building types and their LODs;
- destruction and the far facades;
- the city defence mission.

How the generator replaces it is the design's open work, written once the owner has picked the look.

## Capabilities

- New: `city-districts`.
