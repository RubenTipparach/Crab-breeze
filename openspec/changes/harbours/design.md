# Design: harbours

## 1. World plan

The generator places a harbour where a coastal city meets deep water: a flattened quay strip along the shore, a
breakwater, and sea lanes (polylines) between harbours and out past the horizon. Placed after the cities, kept
clear of the air bases.

## 2. Ships

`cb-core::ships`: each ship follows its lane at its speed, turning gently; a hull floats at sea level; a destroyed
ship lists and sinks over 30 s. Raiders' gunboats are vehicles with guns (`units`).

## 3. Models

Freighter, tanker, fishing boat and gunboat built by Blender generators (`blender-hard-surface`), block-outs first.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| HB1 | Can the pilot board a ship on foot | yes, ships at the quay; later at sea | provisional |
| HB2 | How many harbours | one per coastal city, at most four | provisional |
