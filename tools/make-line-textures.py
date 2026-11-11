"""Draws the chevrons and beads of the quest book's dependency lines.

Original art for Horizon Quests (MIT). A line is a row of thin chevrons
with a bead between each two, pointing to the quest the line opens. The game
draws a sprite upright, with no rotation and no tint, so the sheet holds
each chevron already turned: 32 directions (every 11.25 degrees, y down, 0
pointing right, turning clockwise on screen) in each of the four line
colours of `hq_book` (`LINE`, `LINE_DONE`, `LINE_NEEDS`, `LINE_OPENS`),
and one bead per colour. A chevron is a thin stroke in the line's colour,
lighter at its tip, with a dark edge all round; a bead is a round dot,
lighter at its top left, with a dark edge. Sampled 4 x 4 times a pixel.

Layout (512 x 288: the game leaves out GUI textures over 1024 x 512):
cells of 32 x 32 pixels, 16 a row; each colour's chevrons take two rows
(direction k at column k % 16, row 2 c + k / 16), the colours stacked from
top to bottom; the beads share the last row, colour c at column c.
`hq_book::chevron_cell` and `hq_book::bead_cell` pick a cell.

    python tools/make-line-textures.py

writes mod/content/assets/horizon_quests/textures/gui/chevrons.png (the
sprite `horizon_quests:gui/chevrons`) and build/line_textures_preview.png.
No third-party library: the PNG is written with zlib.
"""
import math
import os
import struct
import zlib

CELL = 32
SAMPLES = 4
DIRECTIONS = 32
COLUMNS = 16
ROWS = DIRECTIONS // COLUMNS
# hq_book: LINE, LINE_DONE, LINE_NEEDS, LINE_OPENS.
COLOURS = [0x8C7C88, 0x40A050, 0x38C8E8, 0xE8C838]
EDGE = (0x10, 0x10, 0x16)
# The chevron in the cell's own axes (u along its direction, v across), in
# pixels from the cell's centre: arms from (-4, +-7) to the tip (4, 0).
TIP = 4.0
SPREAD = 7.0
STROKE = 1.5
EDGE_WIDTH = 1.3
# The bead: its colour inside BEAD, its edge out to BEAD_EDGE.
BEAD = 7.0
BEAD_EDGE = 9.0


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


def chevron_at(k, colour, x, y):
    """The colour at a point of the chevron of direction k, or None."""
    angle = k * math.tau / DIRECTIONS
    ux, uy = math.cos(angle), math.sin(angle)
    px, py = x - CELL / 2, y - CELL / 2
    u, v = px * ux + py * uy, -px * uy + py * ux
    d = min(segment(u, v, -TIP, SPREAD, TIP, 0.0), segment(u, v, -TIP, -SPREAD, TIP, 0.0))
    if d <= STROKE:
        base = rgb(colour)
        # Lighter towards the tip.
        return mix(base, mix(base, (255, 255, 255), 0.4), (u + TIP) / (2 * TIP))
    if d <= STROKE + EDGE_WIDTH:
        return EDGE
    return None


def bead_at(colour, x, y):
    """The colour at a point of the bead, or None."""
    px, py = x - CELL / 2, y - CELL / 2
    d = math.hypot(px, py)
    if d <= BEAD:
        base = rgb(colour)
        # Lighter towards its top left.
        return mix(mix(base, (255, 255, 255), 0.55), base, (math.hypot(px + 2.5, py + 2.5) - 1.0) / 6.5)
    if d <= BEAD_EDGE:
        return EDGE
    return None


def cell(colour_at):
    rows = []
    for y in range(CELL):
        row = []
        for x in range(CELL):
            r = g = b = a = 0.0
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    c = colour_at(x + (sx + 0.5) / SAMPLES, y + (sy + 0.5) / SAMPLES)
                    if c is not None:
                        r, g, b, a = r + c[0], g + c[1], b + c[2], a + 1
            if a == 0:
                row.append((0, 0, 0, 0))
            else:
                row.append((round(r / a), round(g / a), round(b / a), round(a / SAMPLES**2 * 255)))
        rows.append(row)
    return rows


def sheet():
    width, height = CELL * COLUMNS, CELL * (ROWS * len(COLOURS) + 1)
    img = [[(0, 0, 0, 0)] * width for _ in range(height)]

    def put(x0, y0, rows):
        for y, line in enumerate(rows):
            img[y0 + y][x0:x0 + CELL] = line

    for c, colour in enumerate(COLOURS):
        for k in range(DIRECTIONS):
            put((k % COLUMNS) * CELL, (c * ROWS + k // COLUMNS) * CELL, cell(lambda x, y: chevron_at(k, colour, x, y)))
        put(c * CELL, ROWS * len(COLOURS) * CELL, cell(lambda x, y: bead_at(colour, x, y)))
    return img


def png(rows):
    width, height = len(rows[0]), len(rows)
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def preview(rows, back=(0x2A, 0x2E, 0x36)):
    out = []
    for row in rows:
        line = []
        for r, g, b, a in row:
            k = a / 255
            line.append(tuple(round(c * k + bc * (1 - k)) for c, bc in zip((r, g, b), back)) + (255,))
        out.append(line)
    return out


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rows = sheet()
    path = os.path.join(root, "mod", "content", "assets", "horizon_quests", "textures", "gui", "chevrons.png")
    with open(path, "wb") as f:
        f.write(png(rows))
    print(path)
    big = os.path.join(root, "build", "line_textures_preview.png")
    os.makedirs(os.path.dirname(big), exist_ok=True)
    with open(big, "wb") as f:
        f.write(png(preview(rows)))
    print(big)


if __name__ == "__main__":
    main()
