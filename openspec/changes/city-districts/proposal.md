# Proposal: city-districts

## Why

The owner, 2026-11-04: "let's upgrade those city layouts. Make a three js mockups with lots poly buildings of how cities
are built today vs how we want them to look future".

Every city today is the same machine (`cb_core::world::layout`): one flat square 560-960 m across, a straight grid of
70-110 m blocks with a 36 m arterial every fourth street, each block cut into equal 18-36 m lots with one box on each,
taller toward the centre (up to 60 storeys), 6 % of blocks parks and 8 % of lots plazas, a ring of house rows round the
edge and straight highways out. From the air every city is a grey square; at mech height every street looks alike.

## What changes

A city grows in **districts**, the way real cities do, in the 1980s industrial setting (survey G1). Each district has
its own grid angle, block size, street width and its own way of filling a block:

- **Old town** on a river bend: radial lanes and rings round a cathedral square, attached three-to-six-storey houses
  with pitched roofs along every lane.
- **Downtown**: its own rotated grid of 84 m blocks, podiums with towers on them (setback tiers, round towers, twin
  slabs, crowned slabs), tallest at the heart.
- **Courtyard blocks**: mid-rise perimeter blocks (four to eight storeys) round green courtyards, the city's bulk.
- **Rowhouses**: back-to-back terraces with gardens between, on narrow blocks.
- **Apartment estate**: slabs and point towers set in green superblocks.
- **Port and works** along the river downstream: sheds with sawtooth roofs, tank farms, smokestacks, container yards,
  quay cranes.
- **A river** through the city with embankments and bridges; **a ring road**; **avenues** radiating from the old town;
  **a rail line and a station**; **a stadium**; **a central park**.
- **Suburbs** beyond the ring: curving streets with detached houses and gardens, not straight rows.

A block an avenue or the river crosses is cut into smaller ones instead of being dropped.

The mockup is `docs/mockups/city_layouts.html` (published): both cities on one ground at true scale, on one camera,
from the air, at mech height and in plan.

## Not in this change yet

How this replaces the generator (the city record, collision, roads, traffic lanes, lamps, the suburbs and small
buildings, the building types and their LODs, destruction, the far facades and the city defence mission all read the
square grid today) is the design's open work, written once the owner has picked the look.

## Capabilities

- New: `city-districts`.
