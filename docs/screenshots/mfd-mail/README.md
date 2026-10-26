# Taking a job on the cockpit's MAIL page in free roam, 2026-10-25

Headless stills from a cloud session (lavapipe, no GPU): **not a playtest, no frame times**. The stills script
(`CB_STILLS_MAIL=1 crab-breeze --mech mechs/foreman.ron --stills <dir>`) starts as free roam does from the title
menu (no enemy at the start, a board of jobs by mail that is never saved), boards the Foreman and then plays the MFD
by input as a player does: free look held, the view steered until the row lights, a click.

| Still | Shows |
| --- | --- |
| `mfd-menu.jpg` | The dash's centre screen clicked: its MENU (the five readouts and MAIL), outlined amber |
| `mfd-mail.jpg` | MAIL: credits, "no job under way", the first offer (city 13, 4 mechs, 18.9 km, CR 94 000) and ACCEPT |
| `mfd-accepted.jpg` | ACCEPT pressed: "under way", 18.9 km to the waypoint (`MFD: job 0 accepted on the MAIL page`) |
| `mfd-back.jpg` | BACK pressed: the screen shows TARGET again, the page it had before its menu was opened |
| `sheet.jpg` | The four, cropped to the dash |

Not shown here: flying to the waypoint and the fight (the job was 18.9 km away); the job's enemies coming in
150-350 m round the site and never within 1 km of the pilot is pinned by `jobs::enemy_spot`'s tests.
