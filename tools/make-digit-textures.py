"""Draws the small digits of the counts on the quest book's icons.

Original art for Horizon Quests (MIT). One sprite sheet of the digits 0 to
9, each in a cell of 24 x 32 pixels: a smooth white stroke with a dark edge
all round, anti-aliased (each pixel is sampled 5 x 5 times). The book shows a
cell 3 x 4 GUI pixels large (`hq_book::count_draws`): 8 texture pixels a GUI
pixel, enough for GUI scales up to 8. Each digit is built from strokes (lines
and elliptic arcs), no font is used.

    python tools/make-digit-textures.py

writes mod/content/assets/horizon_quests/textures/gui/digits.png (the sprite
`horizon_quests:gui/digits`) and build/digit_textures_preview.png (4 times
larger, on grey). No third-party library: the PNG is written with zlib.
"""
import math
import os
import struct
import zlib

W, H = 12, 16  # the cell in design units (strokes are drawn in these)
SCALE = 2  # texture pixels a design unit: each cell is 24 x 32 pixels
STROKE = 0.95  # half the white stroke's width, in design units
EDGE = 0.8  # the dark edge's width round the stroke
SAMPLES = 5
INK = (0xF6, 0xF6, 0xF6)
EDGE_INK = (0x1C, 0x1C, 0x1C)


def arc(cx, cy, rx, ry, start, end, steps=40):
    """Points of an elliptic arc, angles in degrees, y down (90 is down)."""
    return [
        (cx + rx * math.cos(math.radians(start + (end - start) * k / steps)),
         cy + ry * math.sin(math.radians(start + (end - start) * k / steps)))
        for k in range(steps + 1)
    ]


def flip(paths):
    """The same strokes turned half a turn round the cell's centre."""
    return [[(W - x, H - y) for x, y in path] for path in paths]


SIX = [arc(6.0, 10.0, 3.1, 3.1, 0, 360), arc(6.9, 10.0, 3.9, 7.0, 180, 288)]
DIGITS = [
    [arc(6.0, 8.0, 3.1, 5.1, 0, 360)],
    [[(3.9, 4.6), (6.4, 2.9), (6.4, 13.1)], [(3.8, 13.1), (8.8, 13.1)]],
    [arc(6.0, 5.8, 3.1, 2.9, 190, 375) + [(2.9, 13.1), (9.2, 13.1)]],
    [arc(6.0, 5.4, 2.9, 2.5, 200, 450), arc(6.0, 10.5, 3.2, 2.6, 270, 515)],
    [[(7.8, 13.1), (7.8, 2.9), (2.7, 10.1), (9.5, 10.1)]],
    [[(9.0, 2.9), (3.5, 2.9), (3.1, 7.5)] + arc(6.0, 9.9, 3.2, 3.2, 228, 495)],
    SIX,
    [[(2.8, 2.9), (9.2, 2.9), (4.8, 13.1)]],
    [arc(6.0, 5.3, 2.6, 2.4, 0, 360), arc(6.0, 10.5, 3.2, 2.6, 0, 360)],
    flip(SIX),
]


def distance(px, py, paths):
    best = math.inf
    for path in paths:
        for (ax, ay), (bx, by) in zip(path, path[1:]):
            dx, dy = bx - ax, by - ay
            length = dx * dx + dy * dy
            t = 0.0 if length == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length))
            best = min(best, math.hypot(ax + t * dx - px, ay + t * dy - py))
    return best


def glyph(paths):
    """The digit's RGBA pixels, row by row."""
    rows = []
    for y in range(H * SCALE):
        row = []
        for x in range(W * SCALE):
            ink = edge = 0
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    d = distance((x + (sx + 0.5) / SAMPLES) / SCALE, (y + (sy + 0.5) / SAMPLES) / SCALE, paths)
                    if d <= STROKE:
                        ink += 1
                    elif d <= STROKE + EDGE:
                        edge += 1
            n = SAMPLES * SAMPLES
            alpha = (ink + edge) / n
            if alpha == 0:
                row.append((0, 0, 0, 0))
                continue
            mix = ink / (ink + edge)
            colour = tuple(round(e + (i - e) * mix) for i, e in zip(INK, EDGE_INK))
            row.append(colour + (round(alpha * 255),))
        rows.append(row)
    return rows


def sheet():
    glyphs = [glyph(paths) for paths in DIGITS]
    return [sum((g[y] for g in glyphs), []) for y in range(H * SCALE)]


def png(rows):
    width, height = len(rows[0]), len(rows)
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def preview(rows, scale=4, back=(0x6A, 0x6A, 0x72)):
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
    path = os.path.join(root, "mod", "content", "assets", "horizon_quests", "textures", "gui", "digits.png")
    with open(path, "wb") as f:
        f.write(png(rows))
    print(path)
    big = os.path.join(root, "build", "digit_textures_preview.png")
    os.makedirs(os.path.dirname(big), exist_ok=True)
    with open(big, "wb") as f:
        f.write(png(preview(rows)))
    print(big)


if __name__ == "__main__":
    main()
