# Airliner cabin textures

`fabric`, `trim`, `rubber` and `paint`, each with its normal (`_n`) and ORM (`_orm`) map, are cut unchanged out of the
furniture atlas's swatch strip (`assets/textures/furniture/`, written by `tools/blender/build_furniture.py`; its
`SOURCE.md` says how they were made): each a tileable 1 m patch baked light and neutral, tinted by the model. Cut by
`python3 tools/textures/crop_furniture_swatches.py` at the rectangles `assets/models/furniture.json` lists under
`swatches`. Used by `tools/blender/build_airliners.py` for the seats, carpet, cabin walls, panels and tyres.
