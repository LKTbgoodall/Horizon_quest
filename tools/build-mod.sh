#!/usr/bin/env bash
# Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018).
# Packs mod/ into build/horizon_quests.hmod (tools/pack-mod.py), installs it in the instance as
# the mod browser would (horizon-native-mods/ and mod.horizon_quests:true in horizon-mods.txt,
# so the launcher's Mods page lists it and its own rebuilds keep it), then has the launcher
# compile the instance's mods against the prebuilt SDK of the pinned Nightly. Our files of the
# old format (horizon_quests.hnmod, the content-only horizon-mods/horizon_quests.hmod) are moved
# to build/old-format/. The Linux twin of tools/build-mod.ps1. The launcher builds only on the
# commit of the version it builds from, so this passes --update (the newest installed Nightly, which must be the one horizon-mod.toml
# pins); set NO_UPDATE=1 to keep the version the instance plays.
# The launcher is horizon-launcher on PATH, else the one the desktop entry starts; set
# HORIZON_LAUNCHER to pick another.
# It passes --trust-native-code: only the player runs this script, never an agent.
#     tools/build-mod.sh [INSTANCE]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
instance="${1:-my-horizon}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"

launcher="${HORIZON_LAUNCHER:-}"
if [ -z "$launcher" ]; then
    if command -v horizon-launcher >/dev/null; then
        launcher="$(command -v horizon-launcher)"
    elif [ -f "$data/applications/horizon-launcher.desktop" ]; then
        launcher="$(sed -n 's/^Exec="\{0,1\}\([^"]*\)"\{0,1\}.*/\1/p' "$data/applications/horizon-launcher.desktop" | head -n 1)"
    fi
fi
[ -n "$launcher" ] && [ -x "$launcher" ] || { echo "horizon-launcher not found: put it on PATH or set HORIZON_LAUNCHER." >&2; exit 1; }
echo "Launcher: $launcher ($("$launcher" --version 2>/dev/null | head -n 1))"

# The game folder: the first instance, "horizon", plays in the game home itself; the others
# have their own folder under instances/.
game="$data/horizon"
[ "$instance" = "horizon" ] || game="$data/horizon/instances/$instance"
[ -d "$game" ] || { echo "No game folder for the instance '$instance': $game" >&2; exit 1; }

python3 "$root/tools/pack-mod.py" "$root/build/horizon_quests.hmod"
mkdir -p "$game/horizon-native-mods" "$game/horizon-mods"
for old in "$game/horizon-native-mods/horizon_quests.hnmod" "$game/horizon-mods/horizon_quests.hmod"; do
    if [ -f "$old" ]; then
        mkdir -p "$root/build/old-format"
        mv "$old" "$root/build/old-format/$instance-$(basename "$(dirname "$old")")-$(basename "$old")"
        echo "Moved the old-format $old to build/old-format/"
    fi
done
cp "$root/build/horizon_quests.hmod" "$game/horizon-native-mods/horizon_quests.hmod"
touch "$game/horizon-mods.txt"
grep -q '^mod\.horizon_quests:' "$game/horizon-mods.txt" \
    && sed -i 's/^mod\.horizon_quests:.*/mod.horizon_quests:true/' "$game/horizon-mods.txt" \
    || echo "mod.horizon_quests:true" >> "$game/horizon-mods.txt"
echo "Installed in $game"

update=(--update)
[ "${NO_UPDATE:-}" = "1" ] && update=()

# A newly installed Nightly must be prepared (its game resources made from the player's own
# Minecraft) before the launcher can build on it. Unprepared, native-build says so once, then
# later runs hang without a word. Preparing runs no mod code.
if [ "${#update[@]}" -gt 0 ]; then
    newest="$(ls -1d "$data/horizon-launcher/versions"/nightly-* 2>/dev/null | sort | tail -n 1 || true)"
    if [ -n "$newest" ] && [ ! -e "$newest/.local/horizon-client" ]; then
        echo "Preparing $(basename "$newest") from your Minecraft first..."
        "$launcher" prepare --version "$(basename "$newest")"
    fi
fi

"$launcher" native-build --instance "$instance" "${update[@]}" --trust-native-code
echo "Built. Close and reopen the launcher window, then play the instance '$instance'."
