#!/usr/bin/env bash
# Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018):
# the launcher compiles mod/ against the prebuilt SDK of the pinned Nightly. The Linux twin of
# tools/build-mod.ps1. The launcher builds only on the commit of the version it builds from, so
# this passes --update (the newest installed Nightly, which must be the one horizon-mod.toml
# pins); set NO_UPDATE=1 to keep the version the instance plays.
# It passes --trust-native-code: only the player runs this script, never an agent.
#     tools/build-mod.sh [INSTANCE]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
instance="${1:-mon-horizon}"
update=(--update)
[ "${NO_UPDATE:-}" = "1" ] && update=()
horizon-launcher native-build "$root/mod" --instance "$instance" "${update[@]}" --trust-native-code
echo "Built. Start the instance '$instance' in the launcher."
