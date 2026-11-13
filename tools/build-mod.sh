#!/usr/bin/env bash
# Packages Horizon Quests as one .hmod (code and content), installs it in a
# Horizon instance and builds the game: the Linux (and macOS) twin of
# tools/build-mod.ps1.
#
#   1. Packs build/horizon_quests.hmod, the one file of the mod: its native
#      code (horizon-native.json, mixins/horizon_quests/) and its content
#      (horizon-mod.toml and assets/ from mod/content: the Quest Book's look).
#   2. Unless --no-install: the pinned launcher still builds native code only
#      from horizon-native-mods/<id>.hnmod, so the code half goes there and the
#      content half to horizon-mods/<id>.hmod, both cut from the same sources;
#      the mod is turned on.
#      With --combined, the one .hmod goes to horizon-native-mods/<id>.hmod
#      instead (and the two halves are taken out): what a launcher with native
#      .hmod support installs from the registry (ADR-014).
#   3. Unless --no-build: runs `horizon-launcher native-build` on the instance.
#
# Step 3 passes --trust-native-code: only the player runs this script, never an agent.
#
# Usage:
#   tools/build-mod.sh [--instance NAME] [--instance-dir DIR] [--combined] [--no-install] [--no-build] [--uninstall]
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
combined=0
uninstall=0
while [ $# -gt 0 ]; do
    case "$1" in
        --instance) instance="$2"; shift 2 ;;
        --instance-dir) instance_dir="$2"; shift 2 ;;
        --no-install) no_install=1; shift ;;
        --no-build) no_build=1; shift ;;
        --combined) combined=1; shift ;;
        --uninstall) uninstall=1; shift ;;
        -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
        *) echo "Unknown option: $1 (try --help)" >&2; exit 2 ;;
    esac
done

root="$(cd "$(dirname "$0")/.." && pwd)"
bundle="$root/mod/mixins/$mod_id"
content="$root/mod/content"
out="$root/build"
hmod="$out/$mod_id.hmod"
native_part="$out/install/$mod_id.hnmod"
content_part="$out/install/$mod_id.hmod"

# The commits the mod was built and tested on (the launcher's native/built.json).
horizon_rev="9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"
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
        if [ "${exec#\"}" != "$exec" ]; then
            # Quoted: the path may hold spaces, it ends at the closing quote.
            exec="${exec#\"}"
            exec="${exec%%\"*}"
        else
            exec="${exec%% *}"
        fi
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
    rm -f "$dir/horizon-native-mods/$mod_id.hnmod" "$dir/horizon-native-mods/$mod_id.hmod" "$dir/horizon-mods/$mod_id.hmod"
    set_mod_line "$dir" false
    echo "Horizon Quests removed from $dir. Run native-build again to rebuild the game without it."
    exit 0
fi

content_version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$content/horizon-mod.toml" | head -n 1)"
if [ "$content_version" != "$version" ]; then
    echo "mod/content/horizon-mod.toml says $content_version, the bundle $version." >&2
    exit 1
fi

# 1. Two staged halves: native (horizon-native.json, mixins/<id>/...) and
# content (horizon-mod.toml, assets/, data/).
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
native="$stage/native"
cont="$stage/content"
mkdir -p "$native/mixins/$mod_id" "$cont" "$out/install"
printf '{"game":"26.3","hooks":%s,"horizon":"%s","id":"%s","mixins":"%s","schema":1,"version":"%s"}'     "$hooks" "$horizon_rev" "$mod_id" "$mixins_rev" "$version" > "$native/horizon-native.json"
(
    cd "$bundle"
    find . -type f \( -name '*.rs' -o -name '*.toml' -o -path './data/*' \) -not -path '*/target/*' | while read -r f; do
        mkdir -p "$native/mixins/$mod_id/$(dirname "$f")"
        cp "$f" "$native/mixins/$mod_id/$f"
    done
)
[ -f "$root/mod/LICENSE" ] && cp "$root/mod/LICENSE" "$native/mixins/$mod_id/LICENSE"
cp "$content/horizon-mod.toml" "$cont/"
for top in assets data; do
    [ -d "$content/$top" ] && cp -R "$content/$top" "$cont/"
done
# Every entry gets the same date, so the same sources make the same file.
find "$stage" -type f -exec touch -t 202601010000 {} +

# A zip of the halves given (folders of $stage) merged, files only, no
# folder entries, in a stable order.
make_zip() { # $1: the zip, then the halves
    local zip="$1" flat="$stage/flat" list="$stage/list" d
    shift
    rm -rf "$zip" "$flat" "$list"
    mkdir -p "$flat"
    for d in "$@"; do
        cp -R "$stage/$d/." "$flat/"
    done
    find "$flat" -type f -exec touch -t 202601010000 {} +
    (cd "$flat" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) > "$list"
    if command -v bsdtar > /dev/null; then
        (cd "$flat" && bsdtar --format zip -n -cf "$zip" -T "$list")
    elif command -v zip > /dev/null; then
        (cd "$flat" && zip -q -X -D "$zip" -@ < "$list")
    elif command -v python3 > /dev/null; then
        (cd "$flat" && python3 -c 'import sys, zipfile
with zipfile.ZipFile(sys.argv[1], "w", zipfile.ZIP_DEFLATED) as z:
    for name in open(sys.argv[2]).read().splitlines():
        z.write(name, name)' "$zip" "$list")
    else
        echo "Need bsdtar, zip or python3 to make the package." >&2
        exit 1
    fi
}
make_zip "$hmod" content native
make_zip "$native_part" native
make_zip "$content_part" content
echo "The mod: $hmod (version $version)"
[ "$no_install" = 1 ] && exit 0

# 2. Install it: the code where the launcher builds it, the content where the game reads it.
dir="$(find_instance)"
mkdir -p "$dir/horizon-native-mods" "$dir/horizon-mods"
if [ "$combined" = 1 ]; then
    # The one file, as the registry would install it; no halves beside it.
    rm -f "$dir/horizon-native-mods/$mod_id.hnmod" "$dir/horizon-mods/$mod_id.hmod"
    cp "$hmod" "$dir/horizon-native-mods/$mod_id.hmod"
else
    rm -f "$dir/horizon-native-mods/$mod_id.hmod"
    cp "$native_part" "$dir/horizon-native-mods/$mod_id.hnmod"
    cp "$content_part" "$dir/horizon-mods/$mod_id.hmod"
fi
set_mod_line "$dir" true
echo "Installed in $dir$([ "$combined" = 1 ] && echo ' (one combined .hmod)')."
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
