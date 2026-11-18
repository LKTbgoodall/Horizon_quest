#!/usr/bin/env bash
# Points mod/.horizon/sdk/api at a Horizon checkout, so cargo check and cargo test build the mod
# against Horizon's mod API without the published SDK. The Linux twin of tools/link-sdk.ps1:
# ext/horizon by default, cloned (main, shallow) when missing, or the folder given.
#     tools/link-sdk.sh [HORIZON_CHECKOUT]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
horizon="${1:-$root/ext/horizon}"
if [ -z "${1:-}" ] && [ ! -f "$horizon/Cargo.toml" ]; then
    echo "Cloning Horizon main into $horizon (shallow)..."
    git clone --depth 1 --filter=blob:none https://github.com/horizon-rust/horizon.git "$horizon"
fi
[ -f "$horizon/crates/horizon-mod-api/Cargo.toml" ] || { echo "$horizon has no crates/horizon-mod-api: not a Horizon checkout with the mod API." >&2; exit 1; }
mkdir -p "$root/mod/.horizon/sdk"
ln -sfn "$(cd "$horizon" && pwd)" "$root/mod/.horizon/sdk/api"
echo "mod/.horizon/sdk/api -> $horizon ($(git -C "$horizon" rev-parse HEAD))"
