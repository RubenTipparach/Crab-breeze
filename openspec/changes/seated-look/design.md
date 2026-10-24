# Design: seated-look

## 1. The rule (`cb-core::pilot::coupled_look`, pure)

Seated, each fixed step, with `rel` the view's yaw relative to the mech's heading (wrapped to
-180..180 degrees), `limit` from `data/pilot/pilot.ron` (`seated_look_limit_deg: 90`) and `bound`
the larger of `limit` and the magnitude of last step's `rel`:

1. The mech turned this step (its legs, on the keys or by the rule below). If that left `|rel|` past
   `bound`, the cockpit carries the view: `rel` is held at `bound`. The view rides the seat.
2. The mouse moves `rel`. A move that would take `|rel|` past `bound` stops at `bound`; a move back
   toward the nose always applies. Past 90 degrees, `bound` can only shrink, so the view never ends
   up further round than it was.
3. Free look skips both: `rel` moves with the mouse, unlimited.

The view's world yaw is the heading plus the new `rel`.

## 2. The mech turns toward the view (`cb-core::pilot::look_turn`, pure)

Not in free look, while `|rel|` is within 2 degrees of the limit or past it, the mech's turn is full
toward the view (`turn_axis` +1 or -1). Keys still turn it when the view is inside the limit. When the
view has come back inside (by 2 degrees, so it does not chatter at the edge), the rule lets go.

## 3. Where it runs

- `PilotState::step` (Seated) applies section 1, using the mech's heading from `MechAccess` (already
  there) and the last step's `rel` kept on the pilot (`seated_rel_rad`).
- `cb-app::mech` adds section 2's turn to the seated controls (the torso still follows the view
  within its twist limit, as now).
- The scripted pilot writes look actions as a player does, so it is held by the same rule.

## 4. Tests

`the_view_stops_at_ninety_degrees_from_the_nose`, `pushing_past_the_edge_turns_the_mech_at_its_turn_rate`,
`free_look_can_look_behind`, `leaving_free_look_never_moves_the_view`, `the_cockpit_carries_the_view_when_the_mech_turns_away`.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C11 | The coupled limit | 90 degrees each side (the owner's number) | decided |
