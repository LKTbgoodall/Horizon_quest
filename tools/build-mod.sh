#!/usr/bin/env bash
# Packages Horizon Quests as a .hnmod, installs it in a Horizon instance and
# builds the game: the Linux (and macOS) twin of tools/build-mod.ps1.
#
#   1. Packs build/horizon_quests.hnmod from mod/mixins/horizon_quests (.rs,
#      .toml and the data/ files that native_data! compiles in).
#   2. Unless --no-install: copies it into the instance's horizon-native-mods
#      and turns it on, so the launcher lists it and keeps it in every build.
#   3. Unless --no-build: runs `horizon-launcher native-build` on the instance.
#
# Step 3 passes --trust-native-code: only the player runs this script, never an agent.
#
# Usage:
#   tools/build-mod.sh [--instance NAME] [--instance-dir DIR] [--no-install] [--no-build] [--uninstall]
#
# Without --instance: the default instance (on Linux, the game home
# ~/.local/share/horizon, or $HORIZON_HOME) and the launcher's selected one.
# With --instance NAME: ~/.config/horizon/instances/NAME or
# ~/.local/share/horizon/instances/NAME. --instance-dir DIR overrides the
# folder (the one that holds horizon-native-mods). The launcher is
# $HORIZON_LAUNCHER, else `horizon-launcher` on the PATH, else the binary the
# applications-menu entry (horizon-launcher.desktop) points at.
set -euo pipefail

mod_id="horizon_quests"
instance=""
instance_dir=""
no_install=0
no_build=0
uninstall=0
while [ $# -gt 0 ]; do
    case "$1" in
        --instance) instance="$2"; shift 2 ;;
        --instance-dir) instance_dir="$2"; shift 2 ;;
        --no-install) no_install=1; shift ;;
        --no-build) no_build=1; shift ;;
        --uninstall) uninstall=1; shift ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) echo "Unknown option: $1 (try --help)" >&2; exit 2 ;;
    esac
done

root="$(cd "$(dirname "$0")/.." && pwd)"
bundle="$root/mod/mixins/$mod_id"
out="$root/build"
hnmod="$out/$mod_id.hnmod"

# The commits the mod was built and tested on (the launcher's native/built.json).
horizon_rev="fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"
mixins_rev="5a64af938ec86b00d53d16a408aabc0038fb3e57"

# The version is the bundle manifest's and the hook API compatibility.json's,
# so they never disagree.
version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$bundle/horizon-mixin.toml" | head -n 1)"
hooks="$(tr -d '\r\n ' < "$root/mod/compatibility.json" | sed -n 's/.*"hooks":\([0-9]*\).*/\1/p')"
[ -n "$version" ] && [ -n "$hooks" ] || { echo "Could not read the version or the hooks." >&2; exit 1; }

find_instance() {
    if [ -n "$instance_dir" ]; then
        echo "$instance_dir"
        return
    fi
    local data="${XDG_DATA_HOME:-$HOME/.local/share}"
    if [ -n "$instance" ]; then
        for base in "${XDG_CONFIG_HOME:-$HOME/.config}" "$data" "$HOME/Library/Application Support"; do
            if [ -d "$base/horizon/instances/$instance" ]; then
                echo "$base/horizon/instances/$instance"
                return
            fi
        done
    else
        # No name: the default instance, which on Linux is the game home itself.
        for home in "${HORIZON_HOME:-}" "$data/horizon" "$HOME/Library/Application Support/horizon"; do
            if [ -n "$home" ] && [ -d "$home/horizon-native-mods" ]; then
                echo "$home"
                return
            fi
        done
    fi
    echo "Instance not found. Pass --instance-dir DIR: the folder that holds horizon-native-mods" >&2
    echo "(find ~ -maxdepth 7 -type d -name horizon-native-mods lists them)." >&2
    exit 1
}

find_launcher() {
    if [ -n "${HORIZON_LAUNCHER:-}" ]; then
        echo "$HORIZON_LAUNCHER"
        return
    fi
    if command -v horizon-launcher > /dev/null; then
        command -v horizon-launcher
        return
    fi
    # The applications-menu entry the launcher writes points at its binary.
    local entry="${XDG_DATA_HOME:-$HOME/.local/share}/applications/horizon-launcher.desktop" exec
    if [ -f "$entry" ]; then
        exec="$(sed -n 's/^Exec=//p' "$entry" | head -n 1)"
        exec="${exec%% %*}"
        exec="${exec#\"}"
        exec="${exec%\"}"
        if [ -x "$exec" ]; then
            echo "$exec"
            return
        fi
    fi
    echo "horizon-launcher not found: set HORIZON_LAUNCHER=/path/to/horizon-launcher." >&2
    exit 1
}

set_mod_line() { # $1: true to turn the mod on, false to take it out
    local list="$1/horizon-mods.txt" tmp
    tmp="$(mktemp)"
    if [ -f "$list" ]; then
        grep -v "^mod\.$mod_id:" "$list" > "$tmp" || true
    fi
    if [ "$2" = true ]; then
        echo "mod.$mod_id:true" >> "$tmp"
    fi
    mv "$tmp" "$list"
}

if [ "$uninstall" = 1 ]; then
    dir="$(find_instance)"
    rm -f "$dir/horizon-native-mods/$mod_id.hnmod"
    set_mod_line "$dir" false
    echo "Horizon Quests removed from $dir. Run native-build again to rebuild the game without it."
    exit 0
fi

# 1. The .hnmod package: a zip of horizon-native.json and mixins/<id>/...
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/mixins/$mod_id" "$out"
printf '{"game":"26.3","hooks":%s,"horizon":"%s","id":"%s","mixins":"%s","schema":1,"version":"%s"}' \
    "$hooks" "$horizon_rev" "$mod_id" "$mixins_rev" "$version" > "$stage/horizon-native.json"
(
    cd "$bundle"
    find . -type f \( -name '*.rs' -o -name '*.toml' -o -path './data/*' \) -not -path '*/target/*' | while read -r f; do
        mkdir -p "$stage/mixins/$mod_id/$(dirname "$f")"
        cp "$f" "$stage/mixins/$mod_id/$f"
    done
)
[ -f "$root/mod/LICENSE" ] && cp "$root/mod/LICENSE" "$stage/mixins/$mod_id/LICENSE"
rm -f "$hnmod"
# Files only, no folder entries, as the Windows script packs them.
(cd "$stage" && { echo horizon-native.json; find mixins -type f | sort; }) > "$stage.list"
trap 'rm -rf "$stage" "$stage.list"' EXIT
if command -v bsdtar > /dev/null; then
    (cd "$stage" && bsdtar --format zip -n -cf "$hnmod" -T "$stage.list")
elif command -v zip > /dev/null; then
    (cd "$stage" && zip -q -D "$hnmod" -@ < "$stage.list")
elif command -v python3 > /dev/null; then
    (cd "$stage" && xargs -d '\n' python3 -m zipfile -c "$hnmod" < "$stage.list")
else
    echo "Need bsdtar, zip or python3 to make the package." >&2
    exit 1
fi
echo "Package: $hnmod (version $version)"
[ "$no_install" = 1 ] && exit 0

# 2. Install it in the instance.
dir="$(find_instance)"
[ -d "$dir/horizon-native-mods" ] || mkdir -p "$dir/horizon-native-mods"
cp "$hnmod" "$dir/horizon-native-mods/$mod_id.hnmod"
set_mod_line "$dir" true
echo "Installed in $dir."
[ "$no_build" = 1 ] && exit 0

# 3. Build the game with every native mod turned on in the instance.
launcher="$(find_launcher)"
echo "Launcher: $launcher"
if [ -n "$instance" ]; then
    "$launcher" native-build --instance "$instance" --trust-native-code
else
    # No name: the launcher's selected instance.
    "$launcher" native-build --trust-native-code
fi
echo "Ready: start Horizon from the launcher."
