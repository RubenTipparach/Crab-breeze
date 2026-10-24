"""Crab Breeze: placeholder art for the cockpit's displays (three MFDs and the HUD combiner).

  python3 tools/textures/gen_cockpit_displays.py

These are mockup pictures of what the screens will show, drawn with PIL: a tactical radar in the
middle, the mech's damage and heat on the left, weapons and jet fuel on the right, and an Ace
Combat style HUD (pitch ladder, heading tape, speed and altitude boxes, the lock reticle). In the
game every number here comes from the simulation (CLAUDE.md 6.1) and the screens are drawn live;
the screen nodes in the cockpit glb (SCREEN_*, HUD_combiner) are where that picture goes.
Writes assets/textures/cockpit/displays/{mfd_left,mfd_center,mfd_right,hud}.png.
"""
import math
import os

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
OUT = os.path.join(ROOT, "assets", "textures", "cockpit", "displays")
MONO = os.path.join(ROOT, "tools", "fonts", "DejaVuSansMono-Bold.ttf")
S = 768
GREEN, CYAN, AMBER, RED, WHITE, DIM = (90, 255, 140), (80, 230, 255), (255, 178, 60), (255, 70, 60), (230, 245, 235), (40, 120, 70)


def font(px):
    return ImageFont.truetype(MONO, px)


def screen():
    im = Image.new("RGB", (S, S), (6, 14, 10))
    d = ImageDraw.Draw(im)
    for y in range(0, S, 3):                       # faint scanlines
        d.line([(0, y), (S, y)], fill=(8, 18, 13))
    return im, d


def osb_labels(d, top, bottom, left=(), right=()):
    f = font(22)
    for i, t in enumerate(top):
        d.text((S * (i + 0.5) / 5, 26), t, fill=GREEN, font=f, anchor="mm")
    for i, t in enumerate(bottom):
        d.text((S * (i + 0.5) / 5, S - 26), t, fill=GREEN, font=f, anchor="mm")
    for i, t in enumerate(left):
        d.text((14, S * (i + 1) / 5), t, fill=GREEN, font=f, anchor="lm")
    for i, t in enumerate(right):
        d.text((S - 14, S * (i + 1) / 5), t, fill=GREEN, font=f, anchor="rm")


def glow(im):
    """A phosphor bloom: the picture over a blurred copy of itself."""
    b = im.filter(ImageFilter.GaussianBlur(6))
    return Image.blend(b, im, 0.75).point(lambda v: min(255, int(v * 1.15)))


def mfd_center():
    im, d = screen()
    c, R = (S // 2, S // 2 + 30), 300
    for k in range(1, 5):
        d.ellipse([c[0] - R * k / 4, c[1] - R * k / 4, c[0] + R * k / 4, c[1] + R * k / 4], outline=DIM, width=2)
    for a in range(0, 360, 30):
        r = math.radians(a)
        d.line([c, (c[0] + R * math.sin(r), c[1] - R * math.cos(r))], fill=(20, 70, 40), width=1)
    # sweep wedge
    for i in range(40):
        a = math.radians(40 - i)
        col = (int(30 + i * 1.5), int(90 + i * 4), int(50 + i * 2))
        d.line([c, (c[0] + R * math.sin(a), c[1] - R * math.cos(a))], fill=col, width=3)
    # terrain blocks (the city) and contacts
    for (x, y, w, h) in ((-120, -60, 50, 30), (-40, -150, 70, 40), (60, -90, 40, 60), (100, 40, 60, 30), (-150, 80, 40, 40)):
        d.rectangle([c[0] + x, c[1] + y, c[0] + x + w, c[1] + y + h], outline=(30, 110, 60), width=2)
    for (x, y, kind) in ((-80, -210, "hostile"), (130, -170, "hostile"), (40, -260, "lock"), (-190, -40, "friend"), (170, 120, "hostile")):
        p = (c[0] + x, c[1] + y)
        if kind == "friend":
            d.ellipse([p[0] - 10, p[1] - 10, p[0] + 10, p[1] + 10], outline=CYAN, width=3)
        else:
            col = RED if kind == "hostile" else AMBER
            d.polygon([(p[0], p[1] - 13), (p[0] + 12, p[1] + 9), (p[0] - 12, p[1] + 9)], outline=col, width=3)
            if kind == "lock":
                d.rectangle([p[0] - 22, p[1] - 22, p[0] + 22, p[1] + 22], outline=AMBER, width=3)
                d.text((p[0] + 28, p[1] - 10), "TGT 02  2.4 KM", fill=AMBER, font=font(20))
    # own mech
    d.polygon([(c[0], c[1] - 18), (c[0] + 13, c[1] + 14), (c[0], c[1] + 6), (c[0] - 13, c[1] + 14)], fill=WHITE)
    d.text((c[0], c[1] - R - 22), "N", fill=GREEN, font=font(26), anchor="mm")
    d.text((70, 90), "RNG 4 KM", fill=GREEN, font=font(22))
    d.text((S - 70, 90), "HDG 012", fill=GREEN, font=font(22), anchor="ra")
    osb_labels(d, ("TAC", "MAP", "SCL+", "SCL-", "IFF"), ("NAV", "SYS", "WPN", "DMG", "COM"))
    return glow(im)


def mfd_left():
    im, d = screen()
    d.text((S // 2, 70), "CHASSIS  BREEZE", fill=GREEN, font=font(26), anchor="mm")
    cx = S // 2
    parts = {  # name: (polygon points around cx, status colour)
        "torso": ([(cx - 90, 170), (cx + 90, 170), (cx + 110, 330), (cx - 110, 330)], GREEN),
        "cockpit": ([(cx - 50, 140), (cx + 50, 140), (cx + 40, 185), (cx - 40, 185)], GREEN),
        "arm_l": ([(cx + 120, 180), (cx + 190, 200), (cx + 175, 400), (cx + 120, 380)], RED),
        "arm_r": ([(cx - 120, 180), (cx - 190, 200), (cx - 175, 400), (cx - 120, 380)], GREEN),
        "leg_l": ([(cx + 30, 340), (cx + 110, 340), (cx + 130, 600), (cx + 60, 610)], AMBER),
        "leg_r": ([(cx - 30, 340), (cx - 110, 340), (cx - 130, 600), (cx - 60, 610)], GREEN),
    }
    for name, (pts, col) in parts.items():
        d.polygon(pts, outline=col, width=4)
        if col != GREEN:
            for i in range(0, 300, 14):
                d.line([(pts[0][0] - 200 + i, pts[0][1]), (pts[0][0] - 100 + i, pts[0][1] + 100)], fill=tuple(v // 3 for v in col))
            d.polygon(pts, outline=col, width=4)
    d.text((cx + 200, 300), "L ARM\n38 %", fill=RED, font=font(22))
    d.text((cx + 150, 560), "R LEG\n71 %", fill=AMBER, font=font(22))
    # heat bar
    d.text((60, 650), "HEAT", fill=GREEN, font=font(22))
    d.rectangle([150, 650, S - 60, 676], outline=GREEN, width=2)
    d.rectangle([152, 652, 152 + int((S - 214) * 0.62), 674], fill=AMBER)
    d.text((60, 690), "REACTOR 696 C   COOLANT 4.2 BAR", fill=GREEN, font=font(20))
    osb_labels(d, ("DMG", "HEAT", "PWR", "", "LOG"), ("NAV", "SYS", "WPN", "DMG", "COM"))
    return glow(im)


def mfd_right():
    im, d = screen()
    d.text((S // 2, 70), "WEAPONS", fill=GREEN, font=font(26), anchor="mm")
    rows = (("STA 1", "RAIL LANCE", "RDY", GREEN), ("STA 2", "GATLING 6B", "1840", GREEN),
            ("STA 3", "MSL x6", "LOCK", AMBER), ("STA 4", "FLARE x24", "AUTO", DIM))
    for i, (sta, name, st, col) in enumerate(rows):
        y = 140 + i * 70
        d.rectangle([60, y - 26, S - 60, y + 26], outline=col, width=2)
        d.text((80, y), sta, fill=col, font=font(22), anchor="lm")
        d.text((210, y), name, fill=col, font=font(24), anchor="lm")
        d.text((S - 80, y), st, fill=col, font=font(22), anchor="rm")
    # jet fuel and skate
    d.text((60, 470), "JET FUEL", fill=GREEN, font=font(22))
    for i in range(10):
        col = GREEN if i < 7 else (20, 50, 30)
        d.rectangle([60 + i * 64, 500, 60 + i * 64 + 52, 560], fill=col)
    d.text((60, 590), "BURN 14 S   SKATE WHL 92 %", fill=GREEN, font=font(22))
    d.text((60, 630), "MASTER ARM  SAFE", fill=AMBER, font=font(24))
    osb_labels(d, ("WPN", "STA", "MODE", "", "JETT"), ("NAV", "SYS", "WPN", "DMG", "COM"))
    return glow(im)


def hud():
    """The combiner's symbology on a transparent ground (alpha is the symbol)."""
    W = 768
    im = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    g = (110, 255, 150, 255)
    c = W // 2
    f = font(24)
    # pitch ladder
    for k in (-2, -1, 1, 2):
        y = c - k * 105
        dash = k < 0
        for s in (-1, 1):
            x0, x1 = c + s * 70, c + s * 210
            if dash:
                for t in range(0, 140, 28):
                    d.line([(min(x0, x1) + t, y), (min(x0, x1) + t + 16, y)], fill=g, width=4)
            else:
                d.line([(x0, y), (x1, y)], fill=g, width=4)
            d.line([(x1, y), (x1, y + (12 if k > 0 else -12))], fill=g, width=4)
            d.text((x1 + s * 34, y), f"{abs(k) * 10}", fill=g, font=f, anchor="mm")
    d.line([(c - 330, c), (c - 120, c)], fill=g, width=4)
    d.line([(c + 120, c), (c + 330, c)], fill=g, width=4)
    # boresight and flight path marker
    d.line([(c - 26, c), (c - 8, c)], fill=g, width=4)
    d.line([(c + 8, c), (c + 26, c)], fill=g, width=4)
    d.ellipse([c - 14, c + 26, c + 14, c + 54], outline=g, width=4)
    # heading tape
    for i, h in enumerate(range(350, 380, 5)):
        x = 150 + i * 94
        d.line([(x, 60), (x, 80 if h % 10 else 92)], fill=g, width=3)
        if h % 10 == 0:
            d.text((x, 112), f"{h % 360:03d}", fill=g, font=f, anchor="mm")
    d.polygon([(c, 132), (c - 10, 150), (c + 10, 150)], fill=g)
    # speed and altitude boxes
    d.rectangle([40, c - 26, 150, c + 26], outline=g, width=4)
    d.text((95, c), "86", fill=g, font=font(30), anchor="mm")
    d.text((95, c + 50), "KM/H", fill=g, font=f, anchor="mm")
    d.rectangle([W - 160, c - 26, W - 40, c + 26], outline=g, width=4)
    d.text((W - 100, c), "11.6", fill=g, font=font(30), anchor="mm")
    d.text((W - 100, c + 50), "M AGL", fill=g, font=f, anchor="mm")
    # mode and lock
    d.text((60, W - 120), "SKATE", fill=g, font=font(30))
    d.text((60, W - 80), "JET 70%", fill=g, font=f)
    d.text((W - 60, W - 120), "MSL 6", fill=g, font=font(30), anchor="ra")
    d.text((W - 60, W - 80), "TGT 2.4 KM", fill=g, font=f, anchor="ra")
    lx, ly = c + 150, c - 90
    d.rectangle([lx - 36, ly - 36, lx + 36, ly + 36], outline=(255, 190, 80, 255), width=4)
    d.ellipse([lx - 60, ly - 60, lx + 60, ly + 60], outline=(255, 190, 80, 255), width=3)
    b = im.filter(ImageFilter.GaussianBlur(4))
    return Image.alpha_composite(b, im)


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, fn in (("mfd_left", mfd_left), ("mfd_center", mfd_center), ("mfd_right", mfd_right), ("hud", hud)):
        fn().save(os.path.join(OUT, f"{name}.png"), optimize=True)
        print(name)


main()
