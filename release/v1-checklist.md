# Checklist to release 1.0.0

Tick in order. Claude prepares and checks; Bastien tests in game, decides and sends.

## 1. Everything works in game (Bastien)
- [x] `git pull`, `tools\build-mod.ps1`: the build succeeds (2026-11-11, v0.10.x).
- [x] (2026-11-11, seen through v0.10.7) The look of 0.6.0 to 0.7.3 in game: node frames, chevron lines and their colours, selection corners, the quest window (icons, checks, "!", counts, tooltips, pin), the choice window. Screenshots of anything odd.
- [ ] The server settings never tried: `completion_messages = false`, `parties = false`, `party_size = 2`, `detection_delay = 200` (the README checklist says how).
- [x] A full playthrough of both example chapters in a new world, solo (Bastien, 2026-11-11: "everything works").
- [ ] A short LAN session: a party, a shared task, a team reward if a pack has one.
- [ ] Leave and rejoin worlds a few times: progress, tracked quest and party come back.

## 2. Code ready for review (Claude)
- [x] No dead-code allowances; the game builds the mod with no warning (`tools/check-glue.ps1`).
- [x] Unit and end-to-end tests pass with `-D warnings`.
- [x] A last read of the glue for clarity: the old side panel's layout removed from `hq_book`, a client-state reset bug fixed (0.7.3).
- [x] `mod/README.md` trimmed: its version history is `CHANGELOG.md`, its checklist kept as the in-game test.

## 3. Documentation (Claude, Bastien reads)
- [x] Root `README.md`: what the mod does, commands, building.
- [x] `docs/making-quests.md`: the pack format for pack makers.
- [x] `CHANGELOG.md`.
- [x] Screenshots in the README: six taken by Bastien 2026-11-11, in `docs/images/` (see `registry-entry.md`).

## 4. Re-pin on a recent Horizon (Bastien, then Claude)
- [x] How the content half is published: one `.hmod` with content and code, which Horizon `main` supports ([ADR-014](../vault/05%20-%20Decisions/ADR-014%20Publish%20one%20hmod%20with%20content%20and%20code.md)).
- [x] Bastien updated the launcher (2026-11-11). `horizon-launcher versions`: the newest Nightly is `nightly-2026-11-08-fdc87ae`, **our pin**: nothing to re-pin, the registry's compile check runs on the same Horizon.
- [ ] **Try it now anyway**: `toolsuild-mod.ps1 -Combined` installs the one `.hmod` in `horizon-native-mods/` as the registry would. Three outcomes: the build refuses the file (the launcher is too old: wait); it builds but the Quest Book and the nodes are plain (the launcher took it, the game of our Nightly does not read `native-content/` yet: wait); it builds and the textures show (it works already: go on). Then `toolsuild-mod.ps1` without `-Combined` puts the two halves back.
  - **Tried 2026-11-11 on the dev PC (launcher 0.2.58, Nightly `fdc87ae`)**: the build **accepted** the combined file and built the code from it, but copied **no content** (no `native-content/` in the build, nothing in `horizon-mods/`). So an older launcher already installs our `.hmod` as code only: publishing the combined file loses nothing compared with an `.hnmod`, and players get the textures once their launcher and game read native content.
- [x] **Waiting** (Bastien's choice, 2026-11-11): native `.hmod` support (content shipped with the code) is on Horizon `main` but in no Nightly yet; 1.0.0 waits for it. Check with `horizon-launcher versions`. When a newer Nightly appears: Claude checks the glue against it, updates the pins, and Bastien checks in game that the launcher installs the one `.hmod` with its textures.
  - **2026-11-13**: `nightly-2026-11-13-9f42064` is out (launcher 0.2.63) and reads `native-content/` (absent at `fdc87ae`). Claude: the glue type-checks against `9f42064` with no warning, the hook catalogue only grew (434 hooks, 177 helpers; nothing removed or changed), horizon-mixins stays `5a64af9`, the pins are updated.
- [x] **Tried 2026-11-13** (Linux, launcher 0.2.63, `9f42064`): `tools/build-mod.sh --combined` builds, but the version has **no `native-content/`**: a launcher bug, `pipeline` copies only `mixins/<id>` of each bundle before `install_content` looks for the content root (reported as https://github.com/horizon-rust/horizon/issues/15933). Not fixed on `main` (`441f49d`). Until it is, players installing the combined file get the code without textures.
- [ ] **Bastien**: `git pull`, `tools\build-mod.ps1 -Combined` (Linux: `tools/build-mod.sh --combined`) on the new Nightly: the Quest Book item and the node frames must show their textures. Then the 0.11 to 0.15 editor tests on it.
- [ ] Run the registry's check: `cargo horizon-mod check build/horizon_quests.hmod`.

## 5. Release
- [ ] Version 1.0.0 in `Cargo.toml`, `horizon-mixin.toml`, `content/horizon-mod.toml`, `hq_meta::VERSION`; the changelog's "Unreleased" becomes "1.0.0".
- [ ] Build, test once more, git tag `v1.0.0`, GitHub release with the `.hmod`.
- [ ] Bastien sends the form (`registry-entry.md`), then answers the review.
