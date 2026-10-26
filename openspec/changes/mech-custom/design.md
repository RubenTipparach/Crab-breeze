# Design: mech-custom

## 1. Data

- `data/career/parts.ron`: every buyable weapon (the specs `weapons` already uses) and armour grade, with prices.
- Each mech's data: which stations take which kinds, and the armour grades it can carry.

## 2. Rules (`cb-core::career`, pure)

- A fit is legal when the station takes the part's kind and the career owns it.
- An armour grade adds structure to a part and mass to the mech; locomotion reads the mass.
- The career's owned mech records its fit and livery; the game builds the mech's spec from data plus the fit.

## 3. In the game

The hangar bay's terminal is walked up to and used (first person); the refit page is an MFD page (`mfd-menus`). The
mech's weapons change on the model at once.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| MC1 | Can parts be sold back | yes, at half price | provisional |
| MC2 | Liveries free or paid | free | provisional |
