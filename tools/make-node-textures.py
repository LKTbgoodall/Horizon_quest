"""Draws the quest book's node frames: one sprite sheet of 5 shapes x 5 states.

Original pixel art for Horizon Quests (MIT). Each cell is 32 x 32 pixels: a
dark outline, a bevelled ring in the colour of the quest's state (lighter on
the top-left half, darker on the bottom-right), a thin inner shadow and the
node's dark face, where the book draws the quest's icon. Rows are the shapes
(square, circle, diamond, hexagon, octagon), columns the states (locked,
available, in progress, rewards waiting, done), as `hq_book::node_cell`
picks them; the colours are
`hq_book::state_look`'s. The corners follow `hq_book::shape_points`, so a
frame lines up with the polygon the book draws when the texture is missing.

    python tools/make-node-textures.py

writes mod/content/assets/horizon_quests/textures/gui/nodes.png (the sprite
`horizon_quests:gui/nodes`) and build/node_textures_preview.png (8 times
larger). No third-party library: the PNG is written with zlib.
"""
import math
import os
import struct
import zlib

CELL = 32
SHAPES = ["square", "circle", "diamond", "hexagon", "octagon"]
# hq_book::state_look, in NODE_STATES order.
STATES = [0x50505A, 0xD8D8E0, 0xE8C040, 0x50B8FF, 0x40B050]
OUTLINE = 0x0E0E16
SHADOW = 0x1A1A24
FACE = 0x262632


def shade(colour, factor):
    channels = [(colour >> s) & 0xFF for s in (16, 8, 0)]
    out = [max(0, min(255, round(c * factor))) for c in channels]
    return (out[0] << 16) | (out[1] << 8) | out[2]


def corners(shape, centre, radius):
    """hq_book::shape_points, in pixel coordinates (y down)."""
    cx, cy = centre
    if shape == "square":
        r = radius
        return [(cx - r, cy + r), (cx + r, cy + r), (cx + r, cy - r), (cx - r, cy - r)]
    sides, start = {"diamond": (4, 0.0), "hexagon": (6, 0.0), "octagon": (8, math.pi / 8)}[shape]
    return [
        (cx + radius * math.cos(start + k * math.tau / sides), cy - radius * math.sin(start + k * math.tau / sides))
        for k in range(sides)
    ]


def depth(shape, x, y):
    """How far the point is inside the shape's edge, in pixels (negative: outside)."""
    centre = (CELL / 2, CELL / 2)
    radius = CELL / 2
    if shape == "circle":
        return radius - math.hypot(x - centre[0], y - centre[1])
    points = corners(shape, centre, radius)
    inside = math.inf
    for i, (ax, ay) in enumerate(points):
        bx, by = points[(i + 1) % len(points)]
        ex, ey = bx - ax, by - ay
        length = math.hypot(ex, ey)
        # Distance to the edge's line, positive towards the centre.
        d = ((x - ax) * ey - (y - ay) * ex) / length
        cd = ((centre[0] - ax) * ey - (centre[1] - ay) * ex) / length
        inside = min(inside, d if cd > 0 else -d)
    return inside


def cell(shape, state):
    colour = STATES[state]
    light, dark = shade(colour, 1.25), shade(colour, 0.7)
    pixels = [[None] * CELL for _ in range(CELL)]
    for y in range(CELL):
        for x in range(CELL):
            px, py = x + 0.5, y + 0.5
            d = depth(shape, px, py)
            if d < 0:
                continue
            if d < 1:
                pixels[y][x] = OUTLINE
            elif d < 3.5:
                top_left = (px - CELL / 2) + (py - CELL / 2) < 0
                pixels[y][x] = light if top_left else dark
                if d >= 2.5:
                    pixels[y][x] = colour
            elif d < 4.5:
                pixels[y][x] = SHADOW
            else:
                pixels[y][x] = FACE
    return pixels


def sheet():
    size = CELL * len(SHAPES)
    img = [[None] * size for _ in range(size)]
    for row, shape in enumerate(SHAPES):
        for col in range(len(STATES)):
            for y, line in enumerate(cell(shape, col)):
                for x, colour in enumerate(line):
                    img[row * CELL + y][col * CELL + x] = colour
    return img


def rgba_rows(img, scale=1):
    rows = []
    for line in img:
        row = bytearray()
        for colour in line:
            pixel = b"\x00\x00\x00\x00" if colour is None else struct.pack(">I", (colour << 8) | 0xFF)
            row += pixel * scale
        rows.extend([bytes(row)] * scale)
    return rows


def png(rows):
    width, height = len(rows[0]) // 4, len(rows)
    raw = b"".join(b"\x00" + r for r in rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = os.path.join(root, "mod", "content", "assets", "horizon_quests", "textures", "gui", "nodes.png")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    img = sheet()
    with open(out, "wb") as f:
        f.write(png(rgba_rows(img)))
    preview = os.path.join(root, "build", "node_textures_preview.png")
    os.makedirs(os.path.dirname(preview), exist_ok=True)
    with open(preview, "wb") as f:
        f.write(png(rgba_rows(img, 8)))
    print(out)
    print(preview)


if __name__ == "__main__":
    main()
