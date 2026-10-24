# Design: pilot-skill

Incoming: for each shot not fired by the mech, relative position `p` and velocity `v` to the mech's
torso centre; time of closest approach `t = clamp(-p.v / v.v, 0, 3)`, miss distance `|p + v t|`. A
threat is `t < 1.6 s` and miss `< 6 m`. Dodge direction: perpendicular to `v` in the ground plane,
on the side the mech is already moving (or the open side); jump when `t < 0.9 s` and the shot is a
rocket or a 120 mm shell and `jet_heat < 0.5`. Dodging outranks engaging but not retreating.

Target choice (in the app, from what the pilot sees): score = threat weight (rocket team 5,
helicopter 4, tank 3, mech 3, technical 2, infantry 1) x (1 + 1 - health) / max(range, 50 m), only
targets with a clear line or within 2 s of one.

Cost: a few hundred shots x one dot product each per step.
