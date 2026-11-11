# Checklist to release 1.0.0

Tick in order. Claude prepares and checks; Bastien tests in game, decides and sends.

## 1. Everything works in game (Bastien)
- [ ] `git pull`, `tools\build-mod.ps1`: the build succeeds.
- [ ] The look of 0.6.0 to 0.7.3 in game: node frames, chevron lines and their colours, selection corners, the quest window (icons, checks, "!", counts, tooltips, pin), the choice window. Screenshots of anything odd.
- [ ] The server settings never tried: `completion_messages = false`, `parties = false`, `party_size = 2`, `detection_delay = 200` (the README checklist says how).
- [ ] A full playthrough of both example chapters in a new world, solo.
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
- [ ] Screenshots in the README (Bastien takes them, see `registry-entry.md`).

## 4. Re-pin on a recent Horizon (Bastien, then Claude)
- [x] How the content half is published: one `.hmod` with content and code, which Horizon `main` supports ([ADR-014](../vault/05%20-%20Decisions/ADR-014%20Publish%20one%20hmod%20with%20content%20and%20code.md)).
- [x] Bastien updated the launcher (2026-11-11). `horizon-launcher versions`: the newest Nightly is `nightly-2026-11-08-fdc87ae`, **our pin**: nothing to re-pin, the registry's compile check runs on the same Horizon.
- [ ] Native `.hmod` support (content shipped with the code) is on Horizon `main` but in no Nightly yet. When a newer Nightly appears: Claude checks the glue against it, updates the pins, and Bastien checks in game that the launcher installs the one `.hmod` with its textures.
- [ ] Run the registry's check: `cargo horizon-mod check build/horizon_quests.hmod`.

## 5. Release
- [ ] Version 1.0.0 in `Cargo.toml`, `horizon-mixin.toml`, `content/horizon-mod.toml`, `hq_meta::VERSION`; the changelog's "Unreleased" becomes "1.0.0".
- [ ] Build, test once more, git tag `v1.0.0`, GitHub release with the `.hmod`.
- [ ] Bastien sends the form (`registry-entry.md`), then answers the review.
