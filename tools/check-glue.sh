#!/usr/bin/env bash
# Type-checks the bundle against the pinned Horizon source, without building or running the game.
# The Linux twin of tools/check-glue.ps1: horizon-mixins stages the launcher's copy of the pinned
# source with the mod applied, then `cargo check` compiles the glue against the real hooks.
# Nothing is built to run and no --trust-native-code is passed.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
compat="$root/mod/compatibility.json"
revision="$(sed -n 's/.*"horizon_revision": *"\([^"]*\)".*/\1/p' "$compat")"
mixins_revision="$(sed -n 's/.*"horizon_mixins_revision": *"\([^"]*\)".*/\1/p' "$compat")"
native="${XDG_DATA_HOME:-$HOME/.local/share}/horizon-launcher/native"
# HORIZON_SOURCE: a checkout of the pinned revision, for a Nightly the launcher has not built with yet.
source="${HORIZON_SOURCE:-$native/source/$revision}"
[ -d "$source" ] || { echo "The launcher has no copy of Horizon $revision in $native/source: build once with the launcher first, or set HORIZON_SOURCE to a checkout of it." >&2; exit 1; }
tool=""
for dir in "$native"/tools/*/; do
    name="$(basename "$dir")"
    if [[ "$mixins_revision" == "$name"* ]] && [ -x "$dir/bin/horizon-mixins" ]; then
        tool="$dir/bin/horizon-mixins"
        break
    fi
done
[ -n "$tool" ] || { echo "The launcher has no horizon-mixins $mixins_revision in $native/tools." >&2; exit 1; }

out="$root/build/check"
stage="$out/stage"
mkdir -p "$out"
# horizon-mixins only stages into a folder that does not exist yet.
rm -rf "$stage"

echo "Staging Horizon ${revision:0:7} with the mod applied..."
"$tool" apply --workspace "$source" --mods "$root/mod/mixins" --out "$stage"

echo "Type-checking the staged game (cargo check, nothing is run)..."
if (cd "$stage" && CARGO_TARGET_DIR="$out/target" cargo check -p horizon-client --bin horizon-client --locked --message-format short); then
    echo "The glue type-checks against Horizon ${revision:0:7}. Warnings above that name game.rs are the mod's."
else
    echo "The glue does NOT compile against Horizon ${revision:0:7}: see the errors above (game.rs holds the mod's modules)."
    exit 1
fi
