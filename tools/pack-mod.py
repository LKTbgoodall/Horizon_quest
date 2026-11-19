#!/usr/bin/env python3
"""Packs mod/ into build/horizon_quests.hmod, the one file of the mod on Horizon's mod API.

Takes only what the launcher's package reader accepts (horizon-mod-package, manifest.rs):
horizon-mod.toml, Cargo.toml, Cargo.lock, icon.png, README* / LICENSE* at the top, .rs and .toml
files under src/, and everything under assets/ and data/. No mod-id folder level, '/' separators,
sorted entries and fixed dates, so the same sources give the same file.
    python3 tools/pack-mod.py [OUTPUT]
"""
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MOD = ROOT / "mod"
TOP = ("horizon-mod.toml", "Cargo.toml", "Cargo.lock", "icon.png")


def files():
    for path in sorted(MOD.iterdir()):
        if path.is_file() and (path.name in TOP or path.name.startswith(("README", "LICENSE"))):
            yield path
    for tree in ("src", "assets", "data"):
        for path in sorted((MOD / tree).rglob("*")):
            if not path.is_file():
                continue
            if tree == "src" and path.suffix not in (".rs", ".toml"):
                continue
            yield path


def main():
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "build" / "horizon_quests.hmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in files():
            info = zipfile.ZipInfo(path.relative_to(MOD).as_posix(), (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, path.read_bytes())
            count += 1
    print(f"Packed {count} files into {out}")


if __name__ == "__main__":
    main()
