# Proposal: crab-airship

## Why

Owner, 2026-10-25: "need to design a mockup for the crab air fortress/carrier. It launches fighters and
crab mechs, and must be taken down. the mech must fly up and destroy the thrusters that keep it afloat,
but the platform will have lots of defensive turrets and fighters guarding it and a number of mechs that
will fight you." Earlier (2026-10-24, in `invasion-forces`): "Crab air ship is like a floating air
fortress/carrier with turrets and it's where it launches fighters and mechs. Mechs are parachuted down to
the ground."

`invasion-forces` sketched the airship in one table row (300 m, four engines) when mechs were 12 m. Mechs
are now 24 m and the crab mechs 11 to 18 m tall and up to 29 m wide, so a 300 m ship has no room on its
deck for a fight, and "four engines" does not say what the pilot does up there. This change is the
fortress on its own: what it is, how it stays up, what defends it, and how the pilot brings it down. Its
mockup is `docs/mockups/crab_fortress.html`.

## What changes

- **The Hermit** (working name, survey G20): a 420 m air fortress shaped like a crab's carapace, hanging
  on eight lift thrusters at the ends of eight leg pylons, four a side. It cruises at 900 m and comes down
  to 600 m over the city it attacks.
- **Plated thrusters and hull thrusters** (owner, 2026-10-25): each lift thruster's outboard side is a 300 mm
  plate, so the pilot gets under it to shoot; when a side fails it opens six thrusters built into the hull,
  whose port holes take hits only from directly above or below.
- **The thrusters are how it dies.** Each side has to carry half its weight, and one thruster can carry a
  quarter. A side down to one thruster cannot: the fortress rolls toward it and sinks. So three kills on
  one side bring it down; spread out, it takes five. The cockpit shows each side's lift from the same rule.
- **Getting there.** The jets burn for 20 s and climb at 60 m/s, so the pilot reaches 600 m with about 9 s
  of burn left. To recharge they land on it: the flight deck, the carapace shoulders, or the 14 m walkways
  along the leg pylons that lead out to each thruster's intake. Under each thruster a downwash column
  pushes a mech down and burns it, so a thruster is attacked from its pylon or from the side, not from
  below.
- **What defends it**: 10 flak turrets on the rim, 4 beam turrets under the belly guarding the thrusters,
  8 deck guns, 2 missile batteries, fighters off two bow catapults every 20 s, crab mechs dropped from two
  belly bays every 45 s, and a deck guard of crab mechs (2 Shrikes and 4 Mites) that fights the pilot when
  they land, with more brought up by the hangar lifts.
- **Every part is a lockable target** (41 of them): thrusters, turrets, catapults, bays, lifts, the drive
  thrusters (stopping its drift, so the pilot picks where it falls) and the bridge (its eyes: losing it
  halves the turrets' aim).
- **The fall** is a set piece: it lists, sinks at 17-25 m/s, sheds its guard and wrecks what it lands on.
- **Standing on it**: the deck carries whatever stands on it, at its drift and its list.
- Supersedes the airship row and requirement in `invasion-forces` (its task 2.2 now points here).

## Out of scope

The detailed Blender model (a block-out is chosen first, CLAUDE.md section 1), boarding or walking inside
it on foot, a fortress mission other than defence wave 3, more than one fortress at once, naval units.
