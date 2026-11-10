"""Draws the Quest Book's item texture (16 x 16) in Horizon's colours.

Original pixel art for Horizon Quests (MIT): a closed book lying tilted, seen
from above, at vanilla item size and hand-placed pixel by pixel. Its outline
and layout follow the lying-book silhouette Bastien gave as a reference (a wide
cover tilted up to the right, the spine curling down the left, the pages along
the bottom edge over a dark back cover), but every colour and detail is ours:
Horizon's violets, a light bevel on the top edges and a small round gold sun
on the cover (lit top left, shaded bottom right) instead of the reference's
spots.

    python tools/make-book-texture.py [gold|violet]

writes mod/content/assets/horizon_quests/textures/item/quest_book.png (the
"gold" accents by default), the mod's icon for the registry, mod/icon.png (the
same book 16 times larger: 256 x 256), and, for a look at both,
build/quest_book_preview.png
(each variant 24 times larger, side by side). No third-party library: the PNG
is written with zlib.
"""
import os
import struct
import sys
import zlib

SIZE = 16
PREVIEW_SCALE = 24

# Horizon's violets (crates/horizon-theme), from darkest to lightest.
VIOLETS = {
    "O": 0x1E1358,  # outline
    "B": 0x2E1F7E,  # back cover
    "d": 0x3A2694,  # lip, spine's underside
    "s": 0x4A31B8,  # hinge shadow, spine's side
    "c": 0x5B3FD6,  # face
    "l": 0x6C52E6,  # light on the face, spine's top
    "h": 0x8A74FF,  # bevel
    # Pages, light to dark.
    "p": 0xF4F2EA,
    "q": 0xDCD9CE,
    "r": 0xB9B5A8,
}

ACCENTS = {
    # sun (U lit, W shaded); G, g, b, N unused for now
    "gold": dict(G=0xFFE08A, g=0xE8B54A, b=0xA8741F, U=0xFFD36B, W=0xF2A93B, N=0xFFE9B0),
    "violet": dict(G=0xC9BEFF, g=0x9A86FF, b=0x6A55D9, U=0xF4F2EA, W=0xC9BEFF, N=0x9A86FF),
}

# One letter per pixel ("." is transparent), see VIOLETS and ACCENTS.
PIXELS = (
    "................",
    "........OOO.....",
    "......OOshhO....",
    "....OOshccchO...",
    "..OOshccUUcchO..",
    "OOshlccUUWWcchO.",
    "OslcccccWWccccdO",
    "OOscccccccccddr.",
    "OOqsccccccddrqr.",
    "OspqslccddrqqrOB",
    ".OspqsddrqqrOBOO",
    "..OspqrqqrOBOO..",
    "...OspqrBBOO....",
    "....OsBBOO......",
    ".....OOO........",
    "................",
)


def draw(accent="gold"):
    palette = {**VIOLETS, **ACCENTS[accent]}
    return [[None if ch == "." else palette[ch] for ch in row] for row in PIXELS]


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
    accent = sys.argv[1] if len(sys.argv) > 1 else "gold"
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = os.path.join(root, "mod", "content", "assets", "horizon_quests", "textures", "item", "quest_book.png")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "wb") as f:
        f.write(png(rgba_rows(draw(accent))))
    # Preview: every variant side by side, PREVIEW_SCALE times larger, 4 px apart.
    gap = [None] * 4
    variants = [draw(name) for name in ACCENTS]
    joined = [sum((v[y] + gap for v in variants), [])[:-4] for y in range(SIZE)]
    preview = os.path.join(root, "build", "quest_book_preview.png")
    os.makedirs(os.path.dirname(preview), exist_ok=True)
    with open(preview, "wb") as f:
        f.write(png(rgba_rows(joined, PREVIEW_SCALE)))
    # The registry's icon (playhorizon.dev): the book, pixel for pixel, 16 times larger.
    icon = os.path.join(root, "mod", "icon.png")
    with open(icon, "wb") as f:
        f.write(png(rgba_rows(draw(accent), 16)))
    print(out)
    print(icon)
    print(preview)


if __name__ == "__main__":
    main()
