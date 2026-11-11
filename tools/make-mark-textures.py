"""Draws the small signs of the quest book: checks, "?", "!", padlock, buttons.

Original art for Horizon Quests (MIT). One sprite sheet of 64 x 64 pixel
cells, anti-aliased (each pixel is sampled 4 x 4 times), in the order of
`hq_book::MarkCell`:

0. a green check with a dark edge (a task done, a reward claimed);
1. a gold "?" with a dark edge (a random reward);
2. a red disc with a white "!" (a reward to claim);
3. a padlock (a locked quest);
4. a round grey button with a white check (a checkmark task to tick);
5. the same button in green (the checkmark task done).

The book shows a cell 8 to 16 GUI pixels large, so the art stays smooth at
any GUI scale. Shapes are built from distances to lines, arcs and discs.

    python tools/make-mark-textures.py

writes mod/content/assets/horizon_quests/textures/gui/marks.png (the sprite
`horizon_quests:gui/marks`) and build/mark_textures_preview.png (twice as
large, on grey). No third-party library: the PNG is written with zlib.
"""
import math
import os
import struct
import zlib

CELL = 64
SAMPLES = 4


def rgb(colour):
    return ((colour >> 16) & 0xFF, (colour >> 8) & 0xFF, colour & 0xFF)


def mix(a, b, t):
    t = max(0.0, min(1.0, t))
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def segment(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    length = dx * dx + dy * dy
    t = 0.0 if length == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length))
    return math.hypot(ax + t * dx - px, ay + t * dy - py)


def polyline(px, py, points):
    return min(segment(px, py, *a, *b) for a, b in zip(points, points[1:]))


def arc(cx, cy, r, start, end, steps=48):
    """Points of a circular arc, degrees, y down (90 is down)."""
    return [(cx + r * math.cos(math.radians(start + (end - start) * k / steps)),
             cy + r * math.sin(math.radians(start + (end - start) * k / steps))) for k in range(steps + 1)]


def rounded_box(px, py, x0, y0, x1, y1, r):
    """Signed distance to a rounded rectangle (negative inside)."""
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    hx, hy = (x1 - x0) / 2 - r, (y1 - y0) / 2 - r
    qx, qy = abs(px - cx) - hx, abs(py - cy) - hy
    outside = math.hypot(max(qx, 0), max(qy, 0))
    return outside + min(max(qx, qy), 0) - r


def stroked(d, half, edge, ink, edge_ink, y=None, light=None):
    """A stroke `half` wide each side with a dark edge `edge` wide round it;
    `light` brightens its top (y small) a little."""
    if d <= half:
        colour = ink
        if light is not None and y is not None:
            colour = mix(light, ink, y / CELL * 1.6)
        return colour
    if d <= half + edge:
        return edge_ink
    return None


CHECK = [(13, 34), (26, 47), (51, 17)]
GREEN, GREEN_LIGHT, GREEN_EDGE = rgb(0x5CE83C), rgb(0xA8FF8C), rgb(0x0C2208)


def check(x, y):
    return stroked(polyline(x, y, CHECK), 6.0, 3.5, GREEN, GREEN_EDGE, y, GREEN_LIGHT)


QUESTION = arc(32, 21, 11, 195, 405) + [(32, 34), (32, 39)]
GOLD, GOLD_LIGHT, GOLD_EDGE = rgb(0xFFD040), rgb(0xFFF0A0), rgb(0x2A1C04)


def question(x, y):
    d = min(polyline(x, y, QUESTION), max(0.0, math.hypot(x - 32, y - 51) - 1.0))
    return stroked(d, 5.5, 3.5, GOLD, GOLD_EDGE, y, GOLD_LIGHT)


RED, RED_LIGHT, RED_EDGE, WHITE = rgb(0xE03434), rgb(0xFF7A6A), rgb(0x4A0A0A), rgb(0xFFFFFF)


def alert(x, y):
    d = math.hypot(x - 32, y - 32)
    if d > 29:
        return None
    if d > 26:
        return RED_EDGE
    bar = segment(x, y, 32, 15, 32, 36)
    dot = math.hypot(x - 32, y - 46.5)
    if bar <= 4.5 or dot <= 4.8:
        return WHITE
    return mix(RED_LIGHT, RED, (y - 6) / 34)


METAL, METAL_LIGHT, METAL_SHADE, LOCK_EDGE = rgb(0xD4D4DA), rgb(0xF2F2F6), rgb(0x9C9CA6), rgb(0x1C1C22)
SHACKLE = [(20, 32)] + arc(32, 22, 12, 180, 360) + [(44, 32)]


def lock(x, y):
    body = rounded_box(x, y, 10, 29, 54, 58, 6)
    if body <= -3:
        keyhole = math.hypot(x - 32, y - 40) <= 4.5 or (abs(x - 32) <= 2.2 and 40 <= y <= 50)
        if keyhole:
            return LOCK_EDGE
        return mix(METAL_LIGHT, METAL_SHADE, (y - 30) / 26)
    if body <= 0:
        return LOCK_EDGE
    d = polyline(x, y, SHACKLE)
    if d <= 3.6:
        return mix(METAL_LIGHT, METAL, (y - 8) / 20)
    if d <= 6.6:
        return LOCK_EDGE
    return None


BUTTON_CHECK = [(18, 33), (28, 43), (47, 22)]
GREYS = (rgb(0xC0C0C8), rgb(0x7A7A82), rgb(0x2A2A30), rgb(0x55555C))
GREENS = (rgb(0x86F070), rgb(0x2E9E2A), rgb(0x0E3A0C), rgb(0x1E6A1A))


def button(colours):
    top, bottom, rim, shadow = colours

    def draw(x, y):
        d = math.hypot(x - 32, y - 32)
        if d > 30:
            return None
        if d > 27:
            return rim
        if polyline(x, y, BUTTON_CHECK) <= 4.6:
            return WHITE
        if polyline(x - 1.5, y - 2.5, BUTTON_CHECK) <= 4.6:
            return mix(shadow, bottom, 0.3)
        body = mix(top, bottom, (y - 5) / 54)
        # A soft ring of light on the top edge.
        if d > 24 and y < 30:
            body = mix(body, top, 0.5)
        return body

    return draw


CELLS = [check, question, alert, lock, button(GREYS), button(GREENS)]


def cell(draw):
    rows = []
    for y in range(CELL):
        row = []
        for x in range(CELL):
            r = g = b = a = 0.0
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    c = draw(x + (sx + 0.5) / SAMPLES, y + (sy + 0.5) / SAMPLES)
                    if c is not None:
                        r, g, b, a = r + c[0], g + c[1], b + c[2], a + 1
            n = SAMPLES * SAMPLES
            if a == 0:
                row.append((0, 0, 0, 0))
            else:
                row.append((round(r / a), round(g / a), round(b / a), round(a / n * 255)))
        rows.append(row)
    return rows


def sheet():
    cells = [cell(draw) for draw in CELLS]
    return [sum((c[y] for c in cells), []) for y in range(CELL)]


def png(rows):
    width, height = len(rows[0]), len(rows)
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def preview(rows, scale=2, back=(0x3A, 0x3A, 0x42)):
    out = []
    for row in rows:
        line = []
        for r, g, b, a in row:
            k = a / 255
            line += [tuple(round(c * k + bc * (1 - k)) for c, bc in zip((r, g, b), back)) + (255,)] * scale
        out += [line] * scale
    return out


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rows = sheet()
    path = os.path.join(root, "mod", "content", "assets", "horizon_quests", "textures", "gui", "marks.png")
    with open(path, "wb") as f:
        f.write(png(rows))
    print(path)
    big = os.path.join(root, "build", "mark_textures_preview.png")
    os.makedirs(os.path.dirname(big), exist_ok=True)
    with open(big, "wb") as f:
        f.write(png(preview(rows)))
    print(big)


if __name__ == "__main__":
    main()
