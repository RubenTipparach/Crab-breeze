"""The mechs' colour schemes, one per chassis, in linear RGB: the block-outs are painted in them flat
(build_mech_blockouts.py) and the detailed builds take them over the plating's normal and occlusion maps
(build_mechs.py), so a mech is the same colours low-poly and in detail (owner, 2026-10-24: "the coloring is bad, why
dont you use the cool color scheme from your mockup?").

BODY the main armour, BODY2 the second armour, FRAME the structure, ACCENT the livery stripe, METAL bare steel, GLOW the
lamps; ARMOR (bolt-on plates) and YELLOW where a chassis has them.
"""

PALETTES = {
    "foreman": dict(BODY=(0.82, 0.80, 0.74), BODY2=(0.62, 0.60, 0.56), FRAME=(0.10, 0.10, 0.12), ACCENT=(0.95, 0.62, 0.10),
                    RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.70, 0.2)),
    "stevedore": dict(BODY=(0.12, 0.32, 0.72), BODY2=(0.20, 0.22, 0.26), FRAME=(0.08, 0.08, 0.10), ACCENT=(0.85, 0.16, 0.12),
                      RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(0.55, 0.90, 1.0)),
    "ironhorse": dict(BODY=(0.20, 0.17, 0.15), BODY2=(0.32, 0.27, 0.22), FRAME=(0.07, 0.07, 0.08), ACCENT=(0.78, 0.58, 0.25),
                      RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.78, 0.38)),
    "harrow": dict(BODY=(0.30, 0.33, 0.20), BODY2=(0.24, 0.25, 0.26), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.88, 0.70, 0.12),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.52, 0.53, 0.55), GLOW=(0.55, 1.0, 0.6)),
    "roustabout": dict(BODY=(0.36, 0.38, 0.40), BODY2=(0.22, 0.23, 0.24), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.92, 0.42, 0.10),
                       RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.75, 0.3)),
    "grader": dict(BODY=(0.86, 0.64, 0.10), BODY2=(0.20, 0.20, 0.21), FRAME=(0.07, 0.07, 0.08), ACCENT=(0.08, 0.08, 0.08),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.65, 0.2)),
    "logger": dict(BODY=(0.17, 0.30, 0.20), BODY2=(0.26, 0.24, 0.20), FRAME=(0.07, 0.08, 0.07), ACCENT=(0.92, 0.48, 0.10),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.8, 0.4)),
    "lineman": dict(BODY=(0.30, 0.40, 0.52), BODY2=(0.22, 0.24, 0.27), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.95, 0.55, 0.10),
                    RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.85, 0.4)),
    "driller": dict(BODY=(0.48, 0.20, 0.12), BODY2=(0.30, 0.29, 0.28), FRAME=(0.08, 0.07, 0.07), ACCENT=(0.90, 0.78, 0.15),
                    RUBBER=(0.06, 0.06, 0.06), METAL=(0.58, 0.58, 0.60), GLOW=(0.5, 0.9, 1.0)),
    "bowser": dict(BODY=(0.62, 0.14, 0.12), BODY2=(0.58, 0.59, 0.60), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.95, 0.80, 0.15),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.62, 0.63, 0.66), GLOW=(1.0, 0.75, 0.3)),
    "mule": dict(BODY=(0.90, 0.72, 0.16), BODY2=(0.28, 0.27, 0.25), FRAME=(0.08, 0.08, 0.08), ACCENT=(0.80, 0.16, 0.10),
                 RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.7, 0.3)),
    "pumper": dict(BODY=(0.70, 0.10, 0.08), BODY2=(0.24, 0.24, 0.26), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.92, 0.88, 0.70),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.62, 0.63, 0.66), GLOW=(0.4, 0.6, 1.0)),
    "ploughman": dict(BODY=(0.16, 0.30, 0.55), BODY2=(0.24, 0.24, 0.25), FRAME=(0.07, 0.07, 0.08), ACCENT=(0.90, 0.75, 0.20),
                      RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.85, 0.5)),
    # The two humanoids take the Gundam's build, not its livery (owner, 2026-10-24: "no dont make it obvious"): field
    # colours of Crab Breeze's own, with an orange hazard stripe here and there.
    "picket": dict(BODY=(0.60, 0.56, 0.44), BODY2=(0.29, 0.32, 0.22), FRAME=(0.12, 0.12, 0.13), ACCENT=(0.80, 0.40, 0.12),
                   YELLOW=(0.24, 0.25, 0.26), RUBBER=(0.06, 0.06, 0.06), METAL=(0.45, 0.46, 0.48), GLOW=(1.0, 0.62, 0.25)),
    "sapper": dict(BODY=(0.42, 0.44, 0.45), BODY2=(0.20, 0.22, 0.24), FRAME=(0.11, 0.11, 0.12), ACCENT=(0.82, 0.42, 0.12),
                   YELLOW=(0.24, 0.25, 0.26), ARMOR=(0.31, 0.33, 0.24), RUBBER=(0.06, 0.06, 0.06), METAL=(0.45, 0.46, 0.48),
                   GLOW=(0.45, 0.9, 1.0)),
    # The crab air fortress (openspec/changes/crab-airship; owner, 2026-10-25: "use same armor techniques as the mech",
    # "more grey on this platform? red turrets?", "main color orange, secondary a dark orange", then a palette to pick
    # from). BODY #e98537 the carapace and the shoulder walls, BODY2 #94493a the second armour (the underside, the
    # belly, the legs), PLATFORM #b0a7b8 the platform on top (the spine, the bridge, the shoulder tops, the lifts),
    # TURRET #ac2847 the turrets and missile launchers, FRAME #2c1e31 the structure, ACCENT #ec273f the
    # livery bands, METAL #deceed, GLOW #9de64e, RUBBER #10121c: the owner's sRGB, kept here linear.
    # Read by gen_crab_fortress.py.
    "stratostacean": dict(BODY=(0.815, 0.235, 0.038), BODY2=(0.296, 0.067, 0.042), PLATFORM=(0.434, 0.386, 0.479),
                   TURRET=(0.413, 0.021, 0.063), FRAME=(0.025, 0.013, 0.031), ACCENT=(0.839, 0.020, 0.050),
                   RUBBER=(0.005, 0.006, 0.012), METAL=(0.730, 0.617, 0.847), GLOW=(0.337, 0.791, 0.076)),
}
