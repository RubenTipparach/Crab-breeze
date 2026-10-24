"""Crab Breeze: the cockpit's panels, one builder each, modelled in cockpit_kit's relief kit.

Each entry of PANELS is a face of the cockpit set (build_cockpit.py places it): its size in metres,
the texel density it is baked at, and the builder that models it. Panel space: x across, y up, the
face at z = 0, centred on the panel. Labels are what a mech pilot reads: the three ways to move
(walk, skate, jet), the reactor, coolant, the weapons, and the warnings round what can hurt them.
Every function and placard here is a placeholder for the simulation's own systems (CLAUDE.md 6.1):
when a system exists, its control is wired to the number it shows.
"""

import math

# MFD bezels are 3D in the set; these are where they sit on the dash face (centre x, y, size), so
# the dash is plain where a bezel covers it. build_cockpit.py reads the same table.
MFDS = (("SCREEN_left", -0.40, 0.035, 0.26), ("SCREEN_center", 0.0, 0.045, 0.30), ("SCREEN_right", 0.40, 0.035, 0.26))


def dash_main(P):
    w, h = P.w, P.h
    P.plates([(-w / 2, -0.03, -0.55, h / 2), (-w / 2, -h / 2, -0.55, -0.03), (-0.55, -0.135, 0.55, h / 2),
              (-0.55, -h / 2, 0.55, -0.135), (0.55, -h / 2, w / 2, h / 2)])
    for (x0, y0, x1, y1) in ((-w / 2, -0.03, -0.55, h / 2), (-w / 2, -h / 2, -0.55, -0.03), (0.55, -h / 2, w / 2, h / 2)):
        P.screws(x0, y0, x1, y1)
    P.screw_row(-0.53, -0.152, 0.53, -0.152, 14)
    P.screw_row(-0.53, h / 2 - 0.01, 0.53, h / 2 - 0.01, 14)
    P.code("PNL D-01", -0.765, -0.02)
    P.code("PNL D-02", -0.535, -0.125)
    P.code("PNL D-03", 0.565, 0.205)
    P.code("PNL D-04", -0.535, -0.21)
    # where the bezels sit: a darker mounting frame with its own fixings
    for _, x, y, s in MFDS:
        P.rect(x, y, s + 0.024, s + 0.024, 0.0, 0.002, "paint_dk", bevel=0.001)
        for dx in (-1, 1):
            for dy in (-1, 1):
                P.screw(x + dx * (s / 2 + 0.006), y + dy * (s / 2 + 0.006), z=0.002, r=0.003)
    # left wing: the caution and warning panel and the master lights
    cells = {(0, 0): ("REACTOR\nHOT", "amber"), (1, 0): ("COOLANT\nLOW", None), (2, 0): ("JET\nFUEL", None),
             (0, 1): ("GYRO", None), (1, 1): ("HYD\nPRESS", "amber"), (2, 1): ("SKATE\nWHL", None),
             (0, 2): ("ARMOR\nL ARM", "red_lit"), (1, 2): ("ARMOR\nR LEG", None), (2, 2): ("SENSOR", None),
             (0, 3): ("COMMS", None), (1, 3): ("LOCK\nWARN", None), (2, 3): ("FIRE", None)}
    P.annunciator(-0.765, 0.185, 3, 4, 0.068, 0.034, cells)
    P.text("CAUTION / WARNING", -0.663, 0.198, 0.0075)
    P.button(-0.72, -0.08, 0.06, 0.04, "amber", legend="MASTER\nCAUT", label="")
    P.button(-0.62, -0.08, 0.06, 0.04, "red_lit", legend="MASTER\nWARN")
    P.text("PUSH TO RESET", -0.67, -0.115, 0.0062)
    P.hazard(-0.77, -0.205, -0.565, -0.188)
    P.placard(-0.665, -0.15, 0.17, 0.025, ["EJECT HANDLE BETWEEN KNEES"], role="cautplate", size=0.0075)
    # right wing: two standby gauges and the jet fuel and reactor temperature numbers
    P.dial(0.665, 0.11, 0.055, label="REACTOR", lo=0, hi=1200, needle=0.58, red_from=0.85, units="DEG C")
    P.dial(0.665, -0.065, 0.055, label="JET FUEL", lo=0, hi=100, needle=0.72, red_from=None, units="PCT")
    P.text("STBY", 0.61, 0.195, 0.0065)
    P.screw(0.59, -0.2)
    # between the screens: brightness and mode knobs, small toggles
    for sx in (-1, 1):
        x = sx * 0.21
        P.knob(x, 0.12, 0.011, label="BRT", ticks=["", "", "", "", ""], pointer=0.4 * sx)
        P.knob(x, 0.03, 0.011, label="CONT", ticks=["", "", "", ""], pointer=-0.3)
        P.toggle(x, -0.07, up=sx > 0, top="DAY", bottom="NVG")
    # under the screens: a row of mode buttons
    labels = ("WALK", "SKATE", "JET", "HOVER", "NAV", "TGT", "MAP", "SYS", "WPN", "DMG", "COM", "IFF")
    for i, lb in enumerate(labels):
        x = -0.495 + i * 0.09
        cap = "green" if lb == "SKATE" else ("cyan" if lb == "TGT" else "lens_off")
        P.button(x, -0.175, 0.05, 0.022, cap, legend=lb)
    P.text("MODE", -0.555, -0.175, 0.0065, align="RIGHT")
    P.text("MFD 1", -0.40, 0.19, 0.0072)
    P.text("MFD 2", 0.0, 0.207, 0.0072)
    P.text("MFD 3", 0.40, 0.19, 0.0072)


def glare(P):
    w, h = P.w, P.h
    P.plates([(-w / 2, -h / 2, -0.25, h / 2), (-0.25, -h / 2, 0.25, h / 2), (0.25, -h / 2, w / 2, h / 2)], role="paint_dk")
    for x in (-0.7, -0.45, 0.45, 0.7):
        P.screw(x, 0.0)
    P.rect(0.0, 0.0, 0.42, 0.11, 0.0, 0.004, "fabric", bevel=0.002)
    for i in range(25):
        P.rect(-0.2 + i * 0.0167, 0.054, 0.006, 0.0012, 0.004, 0.0048, "steel")
        P.rect(-0.2 + i * 0.0167, -0.054, 0.006, 0.0012, 0.004, 0.0048, "steel")
    P.placard(-0.55, -0.03, 0.15, 0.03, ["HUD PROJECTOR", "DO NOT STEP"], size=0.0072)
    P.placard(0.55, -0.03, 0.15, 0.03, ["LASER CLASS 3R", "DO NOT LOOK INTO LENS"], role="cautplate", size=0.0068)


def knee(P):
    w, h = P.w, P.h
    P.plates([(-w / 2, -h / 2, -0.2, h / 2), (-0.2, -h / 2, 0.2, h / 2), (0.2, -h / 2, w / 2, h / 2)])
    P.screws(-w / 2, -h / 2, -0.2, h / 2)
    P.screws(0.2, -h / 2, w / 2, h / 2)
    # standby gauges
    P.dial(-0.4, 0.03, 0.06, label="GYRO", lo=-90, hi=90, needle=0.5, units="DEG")
    P.dial(-0.25, 0.03, 0.05, label="HYD PSI", lo=0, hi=50, needle=0.66, red_from=0.9, units="x100")
    P.dial(0.25, 0.03, 0.05, label="COOLANT", lo=0, hi=10, needle=0.42, red_from=0.15, units="BAR")
    P.dial(0.4, 0.03, 0.06, label="JET THRUST", lo=0, hi=120, needle=0.1, red_from=0.92, units="PCT")
    # the emergency panel: framed in hazard stripes
    P.hazard(-0.17, -0.13, 0.17, 0.13)
    P.rect(0.0, 0.0, 0.31, 0.23, 0.0015, 0.004, "paint_dk", bevel=0.001)
    P.placard(0.0, 0.095, 0.27, 0.028, ["EMERGENCY"], role="warnplate", ink="stencil", size=0.012)
    P.toggle(-0.09, 0.015, up=False, guard="red", label="REACTOR SCRAM", top="SCRAM", bottom="NORM")
    P.toggle(0.0, 0.015, up=False, guard="red", label="JET CUTOFF", top="CUT", bottom="RUN")
    P.toggle(0.09, 0.015, up=False, guard="yellow", label="EXT ARMS", top="JETT", bottom="NORM")
    P.placard(0.0, -0.085, 0.27, 0.03, ["GUARDED SWITCHES: LIFT COVER, THEN THROW", "SCRAM STOPS REACTOR IN 2 S"], size=0.0066)


def console(side):
    """The top of a side console: left carries the throttle and the reactor and comms; right
    carries the twist grip and the weapons."""
    def build(P):
        w, h = P.w, P.h
        P.plates([(-w / 2, h / 2 - 0.27, w / 2, h / 2), (-w / 2, -0.12, w / 2, h / 2 - 0.27), (-w / 2, -h / 2, w / 2, -0.12)])
        for (y0, y1) in ((h / 2 - 0.27, h / 2), (-0.12, h / 2 - 0.27), (-h / 2, -0.12)):
            P.screws(-w / 2, y0, w / 2, y1)
        if side == "L":
            # throttle quadrant: a slot along the console with its gates marked
            P.box(0.03, -0.11, 0.085, 0.36, -0.012, 0.0015, "rubber")
            for i in range(16):
                P.rect(0.0575, -0.1 + i * 0.03, 0.05, 0.0035, 0.0012, 0.0024, "paint_dk")
            gates = (("IDLE", -0.1), ("WALK", 0.0), ("RUN", 0.09), ("SKATE", 0.19), ("JET", 0.31))
            for name, y in gates:
                P.rect(0.0, y, 0.02, 0.0016, 0.0, 0.0006, "yellow" if name == "JET" else "stencil")
                P.text(name, -0.018, y, 0.0075, "yellow" if name == "JET" else "stencil", align="RIGHT")
            P.hazard(0.03, 0.3, 0.115, 0.32)
            P.text("THROTTLE", 0.0575, 0.375, 0.0078)
            P.placard(-0.085, 0.33, 0.11, 0.036, ["JET: PAST GATE", "LIFT TO ENGAGE"], role="cautplate", size=0.0068)
            # reactor and coolant
            P.text("REACTOR", -0.06, -0.06 + 0.04, 0.0078)
            P.toggle(-0.11, -0.02, up=True, top="ON", bottom="OFF", label="PUMP A")
            P.toggle(-0.06, -0.02, up=True, top="ON", bottom="OFF", label="PUMP B")
            P.toggle(-0.01, -0.02, up=False, top="AUTO", bottom="MAN", label="VENT")
            P.knob(-0.11, -0.09, 0.012, label="LOAD", ticks=["25", "50", "75", "100"], pointer=0.5)
            P.button(-0.03, -0.095, 0.03, 0.02, "amber", legend="BOOST")
            # comms
            P.text("COMMS", 0.0, -0.135, 0.0078)
            P.keypad(-0.1, -0.165, 4, 3, 0.03, ["1", "2", "3", "CLR", "4", "5", "6", "ENT", "7", "8", "9", "0"], lit={(3, 1)})
            P.knob(0.1, -0.2, 0.012, label="VOL", ticks=["", "", "", "", ""], pointer=-0.2)
            P.knob(0.1, -0.27, 0.012, label="CHAN", ticks=["1", "2", "3", "4", "5"], pointer=0.7)
            P.button(0.045, -0.33, 0.05, 0.022, "lens_off", legend="SQUAD")
            P.button(0.105, -0.33, 0.05, 0.022, "green", legend="CMD")
        else:
            # twist grip deck: plain plate with the grip's rotation scale
            P.ring(-0.04, 0.24, 0.06, 0.052, 0.0, 0.0012, "stencil")
            for i in range(13):
                a = -math.pi * 0.6 + math.pi * 1.2 * i / 12
                P.rect(-0.04 + math.sin(a) * 0.068, 0.24 + math.cos(a) * 0.068, 0.0012, 0.006, 0.0, 0.0008, "stencil", rot=-a)
            P.text("TORSO L", -0.11, 0.315, 0.0065)
            P.text("TORSO R", 0.03, 0.315, 0.0065)
            P.text("TWIST", -0.04, 0.155, 0.0078)
            P.placard(0.095, 0.24, 0.07, 0.07, ["TORSO", "LIMIT", "120 DEG"], role="cautplate", size=0.0072)
            # weapons
            P.placard(0.0, 0.06, 0.24, 0.026, ["WEAPONS"], role="warnplate", ink="stencil", size=0.011)
            P.toggle(-0.09, -0.0, up=False, guard="red", top="ARM", bottom="SAFE", label="MASTER ARM")
            P.knob(0.0, -0.005, 0.014, ticks=["GUN", "RAIL", "MSL", "ALL"], pointer=-0.6, label="SELECT")
            for i, (lb, cap) in enumerate((("STA 1", "green"), ("STA 2", "green"), ("STA 3", "amber"), ("STA 4", "lens_off"))):
                P.button(0.085, 0.02 - i * 0.028, 0.045, 0.02, cap, legend=lb)
            P.text("COUNTERMEASURES", 0.0, -0.135, 0.0078)
            P.button(-0.07, -0.17, 0.06, 0.03, "lens_off", legend="FLARE")
            P.button(0.0, -0.17, 0.06, 0.03, "lens_off", legend="CHAFF")
            P.button(0.07, -0.17, 0.06, 0.03, "red_lit", legend="ECM")
            P.text("FLR 24   CHF 18", 0.0, -0.2, 0.0068, mono=True)
            P.keypad(-0.09, -0.25, 4, 3, 0.03, ["A", "B", "C", "<", "D", "E", "F", ">", "1", "2", "3", "OK"], lit={(3, 2)})
            P.knob(0.1, -0.29, 0.012, label="LOCK", ticks=["", "", ""], pointer=0.0)
    return build


def breakers(P):
    """The inside face of a side console: rows of pop-out circuit breakers with their collars."""
    w, h = P.w, P.h
    P.plates([(-w / 2, -h / 2, 0.0, h / 2), (0.0, -h / 2, w / 2, h / 2)])
    P.screws(-w / 2, -h / 2, 0.0, h / 2)
    P.screws(0.0, -h / 2, w / 2, h / 2)
    names = ("MFD1", "MFD2", "MFD3", "HUD", "IFF", "RADAR", "LIDAR", "COMMS", "FAN A", "FAN B", "PUMP A", "PUMP B",
             "GYRO", "SEAT", "CANOPY", "LAMPS", "JET L", "JET R", "WHL L", "WHL R", "GUN", "RAIL", "MSL", "ECM")
    k = 0
    for row in range(3):
        y = 0.13 - row * 0.105
        P.placard(-0.36, y + 0.05, 0.08, 0.018, [("POWER", "DRIVE", "WEAPON")[row]], size=0.0075)
        for col in range(8):
            x = -0.33 + col * 0.09 + (0.04 if col >= 4 else 0)
            popped = (row, col) in ((1, 2), (2, 5))
            P.cyl(x, y, 0.011, 0.0, 0.003, "steel", seg=6)
            P.cyl(x, y, 0.0075, 0.003, 0.009 if not popped else 0.018, "bezel", seg=24)
            if popped:
                P.ring(x, y, 0.0076, 0.0055, 0.0085, 0.0105, "stencil", seg=24)
            P.text(names[k % len(names)], x, y - 0.024, 0.0065)
            P.text(("5", "7.5", "10", "15")[k % 4], x, y + 0.019, 0.0058)
            k += 1
    P.hazard(-w / 2 + 0.01, -h / 2 + 0.01, w / 2 - 0.01, -h / 2 + 0.03)
    P.text("PULL TO ISOLATE  PUSH TO RESET", 0.0, -h / 2 + 0.042, 0.0072)


def wall(side):
    """The cockpit wall above a side console: an inspection window onto a live circuit board,
    the high voltage warning, and the cable trough to the canopy sill."""
    def build(P):
        w, h = P.w, P.h
        P.plates([(-w / 2, -h / 2, w / 2, h / 2)], role="paint_lt")
        P.screws(-w / 2, -h / 2, w / 2, h / 2)
        x0, x1 = -0.3, 0.12
        P.window(x0, -0.074, x1, 0.074, seed=11 if side == "L" else 23)
        for i in range(6):
            P.screw(x0 - 0.006 + i * (x1 - x0 + 0.012) / 5, 0.0855, z=0.009, r=0.0028)
            P.screw(x0 - 0.006 + i * (x1 - x0 + 0.012) / 5, -0.0855, z=0.009, r=0.0028)
        P.placard(0.24, 0.05, 0.19, 0.06, ["DANGER", "HIGH VOLTAGE 880 V", "ISOLATE BEFORE OPENING"], role="warnplate",
                  ink="stencil", size=0.0085)
        P.hazard(0.15, -0.025, 0.335, -0.008)
        P.placard(0.24, -0.06, 0.19, 0.04, ["ACCESS PANEL W-" + ("14" if side == "L" else "15"), "8 SCREWS  TORQUE 4 NM"],
                  size=0.0072)
        # cable trough along the top
        P.box(-w / 2 + 0.01, 0.095, w / 2 - 0.01, 0.12, 0.0, 0.012, "paint_dk", bevel=0.002)
        for i, r in enumerate((0.0045, 0.0035, 0.004)):
            P.tube([(-w / 2, 0.101 + i * 0.008, 0.016), (w / 2, 0.101 + i * 0.008, 0.016)], r,
                   ("red", "rubber", "yellow")[i])
        for x in (-0.32, -0.08, 0.16, 0.34):
            P.box(x - 0.006, 0.093, x + 0.006, 0.122, 0.0, 0.022, "steel")
    return build


def rear(P):
    """The bulkhead behind the seat: the main breaker board, a reactor interface board under a
    window, the coolant warning, the canopy jettison handle."""
    w, h = P.w, P.h
    P.plates([(-w / 2, -h / 2, -0.2, h / 2), (-0.2, 0.05, w / 2, h / 2), (-0.2, -h / 2, w / 2, 0.05)])
    P.screws(-w / 2, -h / 2, -0.2, h / 2)
    P.screws(-0.2, 0.05, w / 2, h / 2)
    P.screws(-0.2, -h / 2, w / 2, 0.05)
    # breaker board on the left
    P.placard(-0.42, 0.4, 0.3, 0.03, ["MAIN BUS"], size=0.012)
    for row in range(6):
        for col in range(5):
            x, y = -0.54 + col * 0.06, 0.33 - row * 0.075
            P.cyl(x, y, 0.012, 0.0, 0.003, "steel", seg=6)
            P.cyl(x, y, 0.008, 0.003, 0.012, "bezel", seg=24)
            P.text(f"B{row * 5 + col + 1:02d}", x, y - 0.024, 0.0068)
    P.hazard(-0.6, -0.15, -0.24, -0.13)
    P.placard(-0.42, -0.25, 0.3, 0.12, ["CAUTION", "BREAKERS ARE LIVE", "WITH REACTOR SCRAMMED", "ISOLATE AT GROUND POWER"],
              role="cautplate", size=0.012)
    # the reactor interface board under its window
    x0, x1, y0, y1 = -0.13, 0.48, 0.12, 0.4
    P.window(x0, y0, x1, y1, seed=42, density=1.1)
    for i in range(8):
        P.screw(x0 + i * (x1 - x0) / 7, y1 + 0.011, z=0.009, r=0.003)
        P.screw(x0 + i * (x1 - x0) / 7, y0 - 0.011, z=0.009, r=0.003)
    P.text("REACTOR INTERFACE  RIU-2", (x0 + x1) / 2, y1 + 0.03, 0.011)
    # coolant lines crossing the lower right, with their warning
    for i, yy in enumerate((-0.05, -0.12)):
        P.tube([(-0.2, yy, 0.02), (0.2, yy, 0.02), (0.35, yy - 0.08, 0.02), (0.65, yy - 0.08, 0.02)], 0.014,
               ("steel", "steel")[i])
        for x in (-0.1, 0.1, 0.5):
            P.box(x - 0.012, yy - (0.08 if x > 0.3 else 0) - 0.022, x + 0.012, yy - (0.08 if x > 0.3 else 0) + 0.022, 0.0,
                  0.04, "paint_lt", bevel=0.003)
    P.placard(0.12, -0.3, 0.36, 0.07, ["WARNING: REACTOR COOLANT", "300 BAR  280 DEG C", "DO NOT LOOSEN FITTINGS"],
              role="warnplate", ink="stencil", size=0.012)
    # canopy jettison handle
    P.hazard(0.42, -0.42, 0.62, -0.2)
    P.rect(0.52, -0.31, 0.16, 0.18, 0.0015, 0.004, "paint_dk")
    P.box(0.47, -0.33, 0.57, -0.29, 0.004, 0.03, "yellow", bevel=0.006)
    P.text("CANOPY", 0.52, -0.25, 0.012)
    P.text("JETTISON", 0.52, -0.37, 0.011)


def floor(P):
    """The cockpit floor: diamond tread plate in four panels, a hazard strip at the front edge."""
    w, h = P.w, P.h
    P.plates([(-w / 2, -h / 2, 0.0, 0.0), (0.0, -h / 2, w / 2, 0.0), (-w / 2, 0.0, 0.0, h / 2), (0.0, 0.0, w / 2, h / 2)],
             role="steel")
    pitch = 0.05
    i = 0
    y = -h / 2 + 0.03
    while y < h / 2 - 0.02:
        x = -w / 2 + 0.03 + (pitch / 2 if i % 2 else 0)
        while x < w / 2 - 0.02:
            a = 0.6 if (int((x + 5) / pitch) + i) % 2 else -0.6
            P.rect(x, y, 0.028, 0.0075, 0.0, 0.0018, "steel", rot=a)
            x += pitch
        y += pitch * 0.5
        i += 1
    for (x0, y0, x1, y1) in ((-w / 2, -h / 2, 0.0, 0.0), (0.0, -h / 2, w / 2, 0.0), (-w / 2, 0.0, 0.0, h / 2), (0.0, 0.0, w / 2, h / 2)):
        P.screws(x0, y0, x1, y1, inset=0.018)
    P.hazard(-w / 2 + 0.01, h / 2 - 0.06, w / 2 - 0.01, h / 2 - 0.02, z=0.0)


def footwell(P):
    """The wall at the front of the footwell, behind the pedals: an access door and the cable
    runs down to the leg actuators."""
    w, h = P.w, P.h
    P.plates([(-w / 2, -h / 2, w / 2, h / 2)], role="paint_dk")
    P.box(-0.18, -0.15, 0.18, 0.15, 0.0, 0.005, "paint", bevel=0.002)
    P.screws(-0.18, -0.15, 0.18, 0.15)
    P.placard(0.0, 0.1, 0.24, 0.03, ["ACCESS F-2  LEG ACTUATOR BUS"], size=0.0085)
    P.vent(-0.12, -0.11, 0.12, 0.04, 8, z=0.005)
    for i, x in enumerate((-0.42, -0.36, 0.36, 0.42)):
        P.tube([(x, h / 2, 0.02), (x, 0.0, 0.03), (x * 1.05, -h / 2, 0.02)], 0.012, ("rubber", "red", "red", "rubber")[i])
    P.hazard(-w / 2 + 0.02, -h / 2 + 0.02, w / 2 - 0.02, -h / 2 + 0.05)


# name: (width m, height m, px per m, builder)
PANELS = {
    "dash_main":   (1.56, 0.44, 2000, dash_main),
    "glare":       (1.56, 0.16, 1400, glare),
    "knee":        (1.10, 0.30, 1800, knee),
    "console_L":   (0.32, 0.80, 2000, console("L")),
    "console_R":   (0.32, 0.80, 2000, console("R")),
    "breakers_L":  (0.84, 0.40, 1400, breakers),
    "breakers_R":  (0.84, 0.40, 1400, breakers),
    "wall_L":      (0.78, 0.26, 1500, wall("L")),
    "wall_R":      (0.78, 0.26, 1500, wall("R")),
    "rear":        (1.30, 0.90, 1300, rear),
    "floor":       (1.30, 1.30, 900, floor),
    "footwell":    (1.10, 0.50, 1000, footwell),
}
