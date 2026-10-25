# Proposal: blockout-enemies

## Why

Owner, 2026-10-25: "These crabs are awesome. Yes let's not add too much details for enemies. I think the combat mechs
and crabs make great enemies. Implement them." The raiders are tinted copies of the stand-in's boxes. The ten combat
mechs (13 to 22) and the six crab mechs (23 to 28) are built as low-poly block-outs, and the owner wants them as the
enemies, at that level of detail.

## What changes

- **Skins**: an enemy keeps the stand-in's skeleton, sim, sockets, hit capsules and damage parts (everything that already
  fights), hides the stand-in's boxes, and wears a block-out model on its root instead. `data/enemies/skins.ron` lists
  the sixteen skins (model, scale); an encounter or a defence raider names one (`skin: Some("tyrant")`).
- **Limbs**: every block-out carries one node per leg and per arm at its root joint (`LEG_<pair><L|R>`, `ARM_L`,
  `ARM_R`; tracks stay fixed). The game swings them from the sim's own gait phase, so a skin walks in step with the
  mech it rides: legs in alternating groups about their hips, claws and arms as the front pair.
- **Losing a part**: when the sim loses an arm or a leg, the matching limb of the skin comes off whole under physics (a
  leg part takes the front-most leg left on its side), and the stump burns, as the stand-in's cut bones do now.
- **The fights use them**: the first encounter and the defence raiders wear combat mechs and crabs.

## Out of scope

Hit capsules shaped to each skin (they stay the stand-in's), per-leg damage parts for the crabs (`crab-enemies`), the
crab gait proper (`crab-enemies`), LODs (`lods`), the crab fortress (`crab-fortress`).
