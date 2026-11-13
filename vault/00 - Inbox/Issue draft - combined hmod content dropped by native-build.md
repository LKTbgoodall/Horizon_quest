---
tags: [inbox, horizon, issue-draft]
date: 2026-11-13
status: sent 2026-11-13 as horizon-rust/horizon#15933
---
# Issue draft for horizon-rust/horizon: a combined `.hmod`'s content is dropped by `native-build`

**Title**: Launcher: content of a combined native `.hmod` never reaches `native-content/` (pipeline copies only `mixins/<id>`)

**Seen with**: launcher 0.2.63, Nightly `nightly-2026-11-13-9f42064`, Linux x86_64. Still the same code on `main` at `441f49d`.

**Steps**
1. Put a combined mod in `horizon-native-mods/<id>.hmod`: `horizon-native.json`, `horizon-mod.toml`, `assets/<id>/...`, `mixins/<id>/horizon-mixin.toml` (ours: `horizon_quests`, built by `tools/build-mod.sh --combined`).
2. `horizon-launcher native-build --trust-native-code`.

**Expected**: the built version holds `native-content/<id>/` (horizon-mod.toml and assets), as `install_content` intends.
**Got**: the build succeeds and the code works, but the version folder has no `native-content/` at all.

**Why (reading `launcher/src/native_mods.rs` at 9f42064)**
- `installed_mods_into` unpacks the whole `.hmod` into `native/installed.incoming/<id>/` (manifest and assets are there).
- `bundle_in` returns a `Bundle` whose `dir` is `<root>/mixins/<id>`; `source::bundle_root` relies on that to find the content root two levels up.
- `pipeline` then copies each bundle with `copy_bundle(b, &incoming.join(&b.id))`, which copies only `b.dir` (the `mixins/<id>` folder), and rewrites `dir: incoming.join(&b.id)` (line ~2145).
- `install_content(&mods, dir)` (line ~2480) gets those rewritten bundles: `bundle_root` no longer finds `horizon-native.json` two levels up, falls back to `bundle.dir`, finds no `horizon-mod.toml`, and copies nothing.
- `combined_content_is_snapshotted_with_its_exact_native_build` calls `install_content` on `bundles(&staged)` directly, so it does not go through `pipeline`'s copy and misses this.

**Possible fix**: copy the content root too in `pipeline` (keep the `<root>/mixins/<id>` shape in `mods.incoming`), or snapshot the content from the found bundles before `dir` is rewritten.
