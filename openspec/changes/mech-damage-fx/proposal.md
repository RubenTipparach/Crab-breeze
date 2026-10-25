# Proposal: mech-damage-fx

## Why

Owner, 2026-10-25:
- "yea we need decals on mechs too when they get hit!", "so we can see damage reflected on them";
- "when parts fly off or critical damage, the mech needs to be on fire and smoking";
- "when they fall they also should kick up dust".

A hit on a mech today plays sparks and a puff, then leaves nothing: a mech at 10 % looks like a mech at 100 %. A lost
part's stump burns with a thin stream (two 1.8 m smoke puffs and one fire puff every 0.1 s). On a skinned enemy that
stream sits at the stand-in's bone. That bone is scaled for the 5.9 m-hip stand-in, not the 24 m skin, so the stream
is inside the skin and cannot be seen. A thrown limb does not burn. Nothing kicks up dust when a limb or a wreck lands.

## What changes

- **Hit marks that stay**: every hit on a mech leaves a decal on the part it struck, from a Material Maker sheet:
  - bullet punctures with bright bare-metal rims;
  - cannon dents with a burnt ring;
  - rocket scorch splats;
  - gouged scratches.

  The decal rides the part (an arm's marks swing with the arm), is sized by the calibre, and stays for the mech's
  life. A badly hit part ends up covered.
- **Burning and smoking**:
  - A lost part's stump burns where the skin's limb was. The thrown limb burns on the ground for 30 s.
  - A part under 25 % health smokes black.
  - A mech under 25 % in all burns from the torso and trails a black column.
  - All of it is sized to the mech, drawn from the `fx-sprites` smoke and fire sheets.
- **Dust when things fall**:
  - A thrown limb, a falling wreck or a mech going down throws a ring of dust and clods where it lands, scaled by its
    mass.
  - A knocked-down mech (`close-combat` 2b) does too when it hits the ground.

## Out of scope

Deformed armour (meshes that bend), hit marks on buildings, sound.
