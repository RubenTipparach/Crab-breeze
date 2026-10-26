# Proposal: cockpit-identity

## Why

The owner, 2026-10-25, after seeing every roster mech from the seat (`docs/screenshots/interiors/`): "spend some time
customizing cockpits so they all are unique and different", then "we have the little cockpit mockups, they were all
really good" and "dont change foreman".

Eight mechs sit in four interiors today: the Panorama (Foreman, Driller, Picket), the Excavator cab (Ironhorse, Mule),
the Canopy (Harrow) and the Titan (Roustabout, Sapper). From the seat the Foreman, Driller and Picket are the same
picture, and so are the Roustabout and Sapper. The block-out page (`tools/blender/build_cockpit_blockouts.py`, the Mech
Cockpits page) has six more layouts the owner liked that no mech carries.

## What changes

- **Every roster mech gets an interior of its own**, built in detail from a block-out the owner already saw:

  | Mech | Interior | Why it suits the mech |
  | --- | --- | --- |
  | Foreman | Panorama | unchanged (owner: "dont change foreman") |
  | Ironhorse | Excavator cab | unchanged |
  | Harrow | Canopy | unchanged |
  | Roustabout | Titan | unchanged |
  | Driller | **Octagon** | the Stevedore's industrial octagon, two rows of glass, gull-wing doors |
  | Picket | **Cinema** | the sniper's one huge seamless front screen, a single heavy door |
  | Mule | **Tile wall** | the hauler's 210 degrees of square tiles with no pillar ahead, three petal doors |
  | Sapper | **Twin tier** | the siege mech's two rings of screens under a bubble, the top rising on four rams |

- **The four layouts are tuned to the cockpit rules** before they are built: every seam between screen rows at least 15
  degrees above the eye line (`cockpit-parity`), and the three hung MFDs added where a layout had only the dash (Cinema, Tile wall, Twin tier), so
  every cockpit has the Foreman's six live screens. The block-out page is rebuilt from the same `LAYOUTS`, the one place
  each is defined.
- **The four mechs' chests are refitted** round their new interiors (`build_mechs.py`'s `chest_bay`, which already fits
  the pod to whatever the variant builds), and their boarding data regenerated (`gen_mech_data.py`).
- **Built and baked at the Foreman's settings**: 4096 px atlases, 16 contact occlusion samples, denoised, the same lamps.

Not changed: the Foreman's, Ironhorse's, Harrow's and Roustabout's interiors and chests, the game code.

## Impact

- `tools/blender/build_cockpit_blockouts.py` (four layouts tuned), `tools/blender/build_cockpit.py` (four variants
  registered), `tools/blender/build_mechs.py` (four chests), `assets/models/cockpit_{octagon,cinema,tilewall,twintier}/`,
  four mech glbs, `data/mechs/{driller,picket,mule,sapper}.ron` and their `boarding.ron`.
- Cost: four more interiors on disk (about 15 MB each, as the Canopy); only one is loaded at a time, as now.
