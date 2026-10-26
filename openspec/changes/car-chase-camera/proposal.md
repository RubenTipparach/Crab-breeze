# Proposal: car-chase-camera

## Why

The owner, 2026-10-26: "also driving should be in 3rd person", and "car driving should remain at 75". Until now a
car was driven from the driver's seat, in first person, under THE LAW (CLAUDE.md). Driving is now its one
exception: the LAW is amended to say so, and everything else (on foot, the mech, boarding, getting in and out of a
car) stays first person.

## What changes

- **Driving a car is seen from behind and above it**: a chase camera that frames a sedan from about 8 m and the
  bus from about 17 m, looking a little down at the road ahead.
- **The mouse is still the player's**: it orbits the camera all the way round the car, and up and down within a
  range. The camera turns with the car's heading (that is what a chase camera is), and with the mouse still for
  2 s while the car is moving it drifts back behind the car (provisional, `## Open questions` in the design).
  Parked, it stays where the player left it.
- **It never clips into a building or the ground**: it pulls in short of whatever stands between the car and it.
- **Getting in and out stay first person and never cut**: getting in, the view is carried from the pilot's eyes to
  the driver's seat as before, then eases out to the chase camera over 0.6 s; getting out, it eases back into the
  eyes while they are carried to the door. No cut, no black screen.
- **The field of view stays the on-foot one** (75 degrees with the Options offset, `seated-fov`); it never widens
  to the seated 95 in a car.
- **Every number is data**: `car_camera` in `data/pilot/pilot.ron`.
