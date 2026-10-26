# Tasks: mech-custom

- [x] 1.1 `data/career/parts.ron`, station kinds per mech, validated (2026-10-25: a station's kind is read from the
  design's own stock weapon, the missile group the pod and any other an arm, so the generated mech files need no new
  field; `armour_max` per mech is in `catalogue.ron`; `CareerData::load` checks every grade it allows is priced)
- [x] 1.2 Fit rules and armour grades in the core, the fit saved with the owned mech; tested
- [ ] 2.1 The refit page in a hangar bay; buying parts; the model's weapons change (2026-10-25: the bay terminal's
  REFIT tab, only for the mech stopped in that bay: stations with what each fires, FIT a spare, STOCK, BUY a weapon,
  RAISE the armour; the flown mech is rebuilt at once (`refit::fitted`: guns, structure, mass and walk speed). The
  weapon models on the mech do not change yet)
- [x] 2.2 Liveries from the mech palettes (2026-10-25: six tints from `mech_palettes.py`, multiplied over the model's
  own materials, free)
- [ ] 3.1 Playtest: buy a gun, fit it in a bay, fire it
