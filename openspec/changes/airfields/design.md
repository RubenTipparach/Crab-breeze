# Airfields design (proposal)

## 1. Placement

- Count: 3 on seed 42 (one near the capital's pass, two on the outer ring), from data.
- A runway is 1.8 km x 60 m with a 200 m apron and hangars, flattened like a city square
  (`Flats`), oriented along the flattest direction, within 2 km of a city, joined to it by a routed
  highway (`curved-roads`).
- Clear of the crater's rim, lakes, rivers (no bridges on a runway) and other cities' bands.

## 2. The roster

- `Roster { owned: Vec<MechId>, lost: Vec<MechId>, credits }` in `cb-core`, saved with the game.
- The Foreman is permanent: when destroyed it is recovered to the nearest airfield after a delay.
- Any other mech, destroyed, moves to `lost` and is gone.
- Credits come from missions (amounts are a survey question, A2).

## 3. Delivery

- An order at an airfield terminal starts a delivery: a cargo aircraft lands, taxis to the apron
  and the mech walks down its ramp to a boarding spot. Seen in first person from the apron.
- Delivery time and price per mech live in `data/mechs/<id>.ron` (`price`, `delivery_s`).

## Costs

- Generation: placement search over the drainage grid, under 10 ms.
- Draw: a runway mesh, a few hangar meshes per airfield; one aircraft model when delivering.

## Open questions

| Id | Question | Provisional |
|---|---|---|
| A1 | How many airfields, and can the player use any of them? | Three, all usable |
| A2 | What does a mech cost, and how are credits earned? | Mission pay; prices by class |
| A3 | Can a destroyed Foreman be recovered at once, or after a wait? | After 120 s at the nearest airfield |
| A4 | Can the player fly the cargo aircraft, or only watch it land? | Only watch |
