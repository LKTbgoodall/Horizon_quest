"""Draws the quest book's node frames: one sprite sheet of 4 shapes x 4 states.

Original art for Horizon Quests (MIT). Each cell is 64 x 64 pixels,
anti-aliased (drawn in 32-pixel units, sampled 4 x 4 times a pixel): a
dark outline, a bevelled ring in the colour of the quest's state (lighter on
the top-left half, darker on the bottom-right), a thin inner shadow and the
node's dark face, where the book draws the quest's icon. Rows are the shapes
(square, circle, diamond, hexagon), columns the states (locked, available,
in progress, done; rewards waiting use the done cell, the book adds a "!"
badge), as `hq_book::node_cell`
picks them; the colours are
`hq_book::state_look`'s. The corners follow `hq_book::shape_points`, so a
frame lines up with the polygon the book draws when the texture is missing.

    python tools/make-node-textures.py

writes mod/content/assets/horizon_quests/textures/gui/nodes.png (the sprite
`horizon_quests:gui/nodes`) and build/node_textures_preview.png (twice as
larger). No third-party library: the PNG is written with zlib.
"""
import math
import os
import struct
import zlib

CELL = 32  # the geometry is drawn in these units
OUT = 64  # pixels a cell in the texture
SAMPLES = 4
SHAPES = ["square", "circle", "diamond", "hexagon"]
# hq_book::state_look: locked, available, in progress, done.
STATES = [0x50505A, 0xD8D8E0, 0xE8C040, 0x40B050]
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
    sides, start = {"diamond": (4, 0.0), "hexagon": (6, 0.0)}[shape]
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


# Ring depths (from the edge, in 32-unit units): the outline, the coloured
# ring (its inner band in the plain colour), the inner shadow; the face
# starts after. Shapes other than the square have a thicker ring, so they
# read as boldly as the square's straight sides; the book draws the
# hexagon and the diamond bigger (`hq_book::shape_look`), so their ring is
# thinner in units, to look as thick on screen as the circle's. The third
# value is `hq_book::face_share`'s.
RINGS = {
    "square": (1.0, 2.5, 3.5, 4.5),
    "circle": (1.3, 3.4, 4.6, 5.6),
    "hexagon": (1.15, 3.05, 4.15, 5.05),
    "diamond": (0.95, 2.45, 3.3, 4.0),
}


def colour_at(shape, state, px, py):
    """The colour at a point of a 32-unit cell, or None outside."""
    colour = STATES[state]
    light, dark = shade(colour, 1.25), shade(colour, 0.7)
    outline, inner, ring, shadow = RINGS[shape]
    d = depth(shape, px, py)
    if d < 0:
        return None
    if d < outline:
        return OUTLINE
    if d < ring:
        if d >= inner:
            return colour
        top_left = (px - CELL / 2) + (py - CELL / 2) < 0
        return light if top_left else dark
    if d < shadow:
        return SHADOW
    return FACE


def cell(shape, state):
    """The cell's RGBA pixels, OUT pixels square, each sampled 4 x 4 times
    (smooth slanted edges; the square's straight ones stay sharp)."""
    scale = CELL / OUT
    pixels = [[None] * OUT for _ in range(OUT)]
    for y in range(OUT):
        for x in range(OUT):
            r = g = b = n = 0
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    c = colour_at(shape, state, (x + (sx + 0.5) / SAMPLES) * scale, (y + (sy + 0.5) / SAMPLES) * scale)
                    if c is not None:
                        r, g, b, n = r + (c >> 16), g + ((c >> 8) & 0xFF), b + (c & 0xFF), n + 1
            if n:
                pixels[y][x] = (round(r / n), round(g / n), round(b / n), round(n * 255 / SAMPLES**2))
    return pixels


def sheet():
    size = OUT * len(SHAPES)  # square: as many states as shapes
    img = [[None] * size for _ in range(size)]
    for row, shape in enumerate(SHAPES):
        for col in range(len(STATES)):
            for y, line in enumerate(cell(shape, col)):
                for x, colour in enumerate(line):
                    img[row * OUT + y][col * OUT + x] = colour
    return img


def rgba_rows(img, scale=1):
    rows = []
    for line in img:
        row = bytearray()
        for colour in line:
            pixel = b"\x00\x00\x00\x00" if colour is None else bytes(colour)
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
        f.write(png(rgba_rows(img, 2)))
    print(out)
    print(preview)


if __name__ == "__main__":
    main()
