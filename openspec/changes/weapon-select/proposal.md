# Proposal: weapon-select

## Why

The owner (2026-10-25, while playing): "can scroll wheel change weaopon selection?", "I have all these
guns it should highlight my selected primary weapon, also, one of these mfds should tell me what I'm
targeting". Today the left mouse fires every primary-group gun at once and nothing shows which guns
are which; the lower dash screens are pictures, not readouts.

## What changes

- **A selected station:** each of the mech's weapons is a station (STA 1, 2, ...). One is selected.
  The left mouse fires the selected station; the right mouse fires the secondary group and F the
  missiles at the lock, as before.
- **The mouse wheel** (and `]`) steps the selection, skipping a station whose arm is lost.
- **The lower dash screens come alive** (cockpit-mfds): the left shows MECH, the centre a new TARGET
  page, the right a new WEAPONS page. Like the MFDs above, any of them steps through every page when
  pressed (free look and click).
- **WEAPONS:** every station with its name and state (READY, its reload in seconds, LOST), the selected
  one boxed in amber.
- **TARGET:** what is targeted (its name or kind), its range and bearing, the lock, and how hurt it is.
- **Enemy AI** keeps firing by group, as it does now.
