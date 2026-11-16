#!/usr/bin/env bash
# Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018):
# the launcher compiles mod/ against the prebuilt SDK of the pinned build. The Linux twin of
# tools/build-mod.ps1. Needs a Nightly built with the mod API (newer than nightly-2026-11-13-9f42064).
# It passes --trust-native-code: only the player runs this script, never an agent.
#     tools/build-mod.sh [INSTANCE]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
instance="${1:-mon-horizon}"
horizon-launcher native-build "$root/mod" --instance "$instance" --trust-native-code
echo "Built. Start the instance '$instance' in the launcher."
