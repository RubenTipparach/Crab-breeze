# Design: civilians

## 1. Simulation (`cb-core::civilians`, pure)

- People live in tiles round the pilot (as trees do, `Forest::keep_around`): spawned from the street pieces of the
  city blocks within 400 m, kept while within 600 m, gone beyond.
- Each walks a route of pavement points; a fixed step moves them; a danger event (a blast, a shot, a mech's footfall
  within a radius) turns them to flee for a time.
- Greeting: a person within 2.5 m in front of the pilot, Use pressed: the person stops and faces the pilot; a line is
  picked from `data/people/lines.ron` by place and seed.

## 2. Drawing

Near (under 60 m): the rigged humanoid, walking or idle. Mid (60-250 m): a low-poly LOD. Far: instanced impostor
cards. Budget: 200 near, 2,000 in all, measured on the 3070.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CV1 | Can civilians be hurt | they flee; none are harmed on screen | provisional |
| CV2 | Voiced lines or text | text first, voice later | provisional |
