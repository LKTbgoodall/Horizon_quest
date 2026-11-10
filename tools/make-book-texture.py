"""Draws the Quest Book's item texture (16 x 16) in Horizon's colours.

Original pixel art for Horizon Quests (MIT), in the "Blocks" look of Horizon's
design system: a violet face with a light bevel and a dark lip, a dark outline,
light pages, and a sun in slices over a horizon line on the cover.

    python tools/make-book-texture.py

writes mod/content/assets/horizon_quests/textures/item/quest_book.png and, for
a look at it, build/quest_book_preview.png (16 times larger). No third-party
library: the PNG is written with zlib.
"""
import os
import struct
import zlib

# Horizon's Blocks palette (crates/horizon-theme, dark), plus transparency.
PALETTE = {
    ".": None,          # transparent
    "O": 0x05040A,      # outline
    "F": 0x5B3FD6,      # face (the main action's violet)
    "B": 0x8A74FF,      # face bevel
    "L": 0x3A2694,      # face lip
    "S": 0x2B2641,      # spine (button)
    "s": 0x3D3560,      # spine band (button bevel)
    "P": 0xECEDF3,      # page (ink)
    "p": 0xC5C7D1,      # page shade (ink soft)
    "W": 0xECEDF3,      # emblem
    "A": 0x9A86FF,      # accent
}

GRID = [
    "................",
    "..OOOOOOOOOOO...",
    "..OSsBBBBBBBOO..",
    "..OSSBFFFFFFPO..",
    "..OsSBFFWWFFpO..",
    "..OSSBFWWWWFPO..",
    "..OSSBFFFFFFpO..",
    "..OSSBWWWWWWPO..",
    "..OSSBAAAAAAPO..",
    "..OSSBFFFFFFpO..",
    "..OSSBFBBBBFPO..",
    "..OSSBFFFFFFpO..",
    "..OsSBFFBBFFPO..",
    "..OSSLLLLLLLPO..",
    "..OOOOOOOOOOOO..",
    "................",
]


def rgba_rows(grid, scale=1):
    rows = []
    for line in grid:
        assert len(line) == 16, line
        row = bytearray()
        for ch in line:
            colour = PALETTE[ch]
            px = b"\x00\x00\x00\x00" if colour is None else struct.pack(">I", (colour << 8) | 0xFF)
            row += px * scale
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
    out = os.path.join(root, "mod", "content", "assets", "horizon_quests", "textures", "item", "quest_book.png")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "wb") as f:
        f.write(png(rgba_rows(GRID)))
    preview = os.path.join(root, "build", "quest_book_preview.png")
    os.makedirs(os.path.dirname(preview), exist_ok=True)
    with open(preview, "wb") as f:
        f.write(png(rgba_rows(GRID, 16)))
    print(out)
    print(preview)


if __name__ == "__main__":
    main()
