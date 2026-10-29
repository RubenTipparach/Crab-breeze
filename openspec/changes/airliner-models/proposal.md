# Proposal: airliner-models

## Why

The owner, 2026-10-29, after looking at the blueprint sheet (`docs/reference/airliner-blueprints/`): "approve all,
send it, make me models, and setup rig and materials so I can see planes with their gear down, up, props rotating,
and navigation lights. planes dont need interiors, but shoudl be able to be shot down or shot at by mech, so adhere
to best practices when desigining destrctible planes, would be cool to do interiors with passengser seats to. ah just
ddo it, make interiors work! no need for light baking lol, windows are too small anyways".

The airliner flies today as an approved block-out stand-in (`civil-air-traffic` task 3.8 is open), there is no
propliner or wide-body at all, and a civil aircraft takes hits as two capsules and one health bar: it is whole until
it falls, and its wreck is six boxes.

## What changes

- **Three airliners modelled in Blender**, one generator, `tools/blender/build_airliners.py`, on the freighter's
  method (`blender-hard-surface`, `combined-lowpoly-vehicle`), with shapes taken from the NASA public-domain
  three-views the owner approved and sizes from the real aircraft:
  - `propliner`: a 1930s-50s twin radial propliner after the DC-3 (19.7 m long, 29.0 m span), 21 seats;
  - `airliner`: an early four-jet narrowbody after the Boeing 720 (41.3 m, 39.9 m), 138 seats;
  - `widebody`: a three-engine wide-body after the L-1011 TriStar (54.2 m, 47.3 m), 306 seats.
  None wears a real airline's or maker's name or livery.
- **Rigged by data.** Every moving part is a named pivot node, and the model's manifest says how it moves (axis,
  angle, time): landing gear legs and their doors fold into their wells, propellers turn, ailerons, elevators,
  rudder and flaps hinge. The page and the game read the same manifest.
- **Navigation lights** as on a real airliner: red on the left wing tip, green on the right, white on the tail, a red
  beacon flashing on top and under the belly, white strobes double-flashing on the tips, and landing lights in the
  wing roots; each an emissive part with a named light empty.
- **Interiors.** The fuselage is a hollow shell with real window holes. Inside: the cabin floor, every passenger seat
  row, overhead bins, bulkheads, galleys and the flight deck with its seats and panel. Lit by the game's lights, not
  baked (owner: "no need for light baking").
- **Destructible the way games do it well**: each aircraft is split at its structural joints into damage zones, each
  its own mesh with its own hit capsules, health share and break caps, so a shot takes off a wing tip, an engine, the
  tail or breaks the fuselage in two, showing the cabin.
- **A mockup page**, `docs/mockups/airliners.html`, shows the three at true scale beside a pilot and the Foreman, with
  gear up and down, propellers turning, lights flashing day and night, the interior cut away and the damage zones
  pulled apart.

The game code (drawing these models in place of the stand-in, the per-zone damage, the detached parts as debris) is
in this change's tasks and waits until the owner has looked at the page (CLAUDE.md section 4).

## Impact

- New: `tools/blender/aircraft_kit.py` (the freighter's primitives, moved, so both builds share one implementation),
  `tools/blender/build_airliners.py`, `assets/models/aircraft/{propliner,airliner,widebody}.{glb,json}`,
  `tools/mockups/gen_airliners.py`, `tools/mockups/airliners_src.html`, `docs/mockups/airliners.html`.
- Later, at apply: `data/world/air_traffic.ron` (two new kinds, the airliner resized), `cb_core::air_traffic`
  (damage zones), `cb-app::air_traffic` (models, rig, lights, zones, debris).
