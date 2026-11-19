#!/usr/bin/env bash
# Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018):
# the launcher compiles mod/ against the prebuilt SDK of the pinned Nightly. The Linux twin of
# tools/build-mod.ps1. The launcher builds only on the commit of the version it builds from, so
# this passes --update (the newest installed Nightly, which must be the one horizon-mod.toml
# pins); set NO_UPDATE=1 to keep the version the instance plays.
# The launcher is horizon-launcher on PATH, else the one the desktop entry starts; set
# HORIZON_LAUNCHER to pick another.
# It passes --trust-native-code: only the player runs this script, never an agent.
#     tools/build-mod.sh [INSTANCE]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
instance="${1:-horizon}"
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

"$launcher" native-build "$root/mod" --instance "$instance" "${update[@]}" --trust-native-code
echo "Built. Start the instance '$instance' in the launcher."
