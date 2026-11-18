#!/usr/bin/env bash
# Checks the mod as the launcher will compile it: cargo test and cargo check with every warning an
# error, against Horizon's mod API. The Linux twin of tools/check-mod.ps1; runs tools/link-sdk.sh
# when mod/.horizon/sdk is missing. Nothing is built to run and no --trust-native-code is passed.
# mod/Cargo.lock is the registry-only lock the launcher reads; Cargo adds the linked API's crates
# to it while checking, so it is put back as it was afterwards.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
[ -f "$root/mod/.horizon/sdk/api/crates/horizon-mod-api/Cargo.toml" ] || "$root/tools/link-sdk.sh"
kept="$(mktemp)"
cp "$root/mod/Cargo.lock" "$kept"
trap 'cp "$kept" "$root/mod/Cargo.lock"; rm -f "$kept"' EXIT
export RUSTFLAGS="-D warnings"
cargo test --manifest-path "$root/mod/Cargo.toml"
cargo check --manifest-path "$root/mod/Cargo.toml" --lib
echo "The mod builds against the mod API with no warning, and its tests pass."
