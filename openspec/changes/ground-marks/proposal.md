# Proposal: ground-marks

## Why

Owner, 2026-10-24: "Next steps is: making mech steps really impact. Stamp mech footprint decals like
we did with mining mike. Kick up dirt, skates should leave dirt trails. Concrete would get concrete
footprints".

Today a 12 m mech's foot comes down and nothing is left behind: a puff of tan dust the same on soil and
on a city street, no print, no trail behind the skates, and nothing felt in the cockpit. Mining Mike
(`godot-sandbox`, branch `origin/claude/long-haul-playtest`, commit 74e1412c) already solved the print:
GroundDents, one batch of quads drawn as a multiply over the ground with parallax and lit relief, so a
print looks pressed in while the ground's own texture and light show through.

## What changes

- **Footprints, skate trails and crater scorches**, a port of Mining Mike's GroundDents: one ring of
  marks (1 024, the oldest replaced first), each mark a FOOT, TRAIL or CRATER (the GROOVE shape comes
  along), fading over the last third of its life, drawn by one entity with one mesh and one material
  from one storage buffer: **one draw call** for every mark on the island. Each mark lies on the
  ground's real height (the one height query, sampled on a 5 by 5 grid) and bends over its slope.
- **The ground decides the mark**: soil (open country and parks) takes a dark, pressed print with a
  lip of soil; paving (highways, bridges, city streets and plazas, building lots) takes a shallow scuff
  with radial cracks, a crushed rim and pale dust, no lip; water takes none (ripples and spray already
  show it). One pure rule, `WorldPlan::surface_at`, is the shared cover rule with no clearance.
- **Impact on every foot strike**: clods and dust from the foot on soil; grit, concrete chips and a
  few sparks on paving; a landing from a boost presses both feet deeper and throws a ring of dust.
  Skates throw a spray of dirt on soil and sparks and grit on paving, and lay grooves or rubber
  streaks end to end.
- **The pilot feels it**: a short drop and recovery of the pilot's eye on the piloted mech's foot
  strikes, a centimetre standing to 4 cm at speed, 7 cm on a landing, over 0.15 s. Position only,
  never the view's direction (CLAUDE.md section 8).
- **Grass under a print is crushed**: no blade stands on a soil print or skate groove for its life, so the dirt shows (design section 10).
- **One foot-strike rule**: the gait's stance start (`cb_core::mech::foot_strikes`) is the one
  place a strike is decided; the footfall spray, the print, the effect and the jolt all ask it.
- **Measurable**: `CB_NO_GROUND_MARKS=1` lays and draws no mark (the effects and the jolt stay), so
  the benchmark suite can compare.

## Not in this change

- Prints from the pilot on foot, from enemy crabs, or tank treads.
- A mark that stays for good (a persistent per-tile texture of trampled ground): survey `M10`.
- Snow, sand or mud as surfaces: the island has none yet.
