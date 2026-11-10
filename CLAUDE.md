# Horizon Quests: instructions for Claude

An FTB Quests-style quest mod for **Horizon** (Minecraft Java 26.3 rewritten in Rust), written as a **native Rust mod**.
**Talk to the user in French.** **Write every project file in English**: vault notes, code, comments, commit messages.

## Project memory = `vault/` (Obsidian)
**At the start of every session, BEFORE anything else:**
1. Read `vault/00 - Home.md` (status, next step, vault map).
2. Read the most recent note in `vault/06 - Journal/`.
3. Read the notes the task needs. The most important ones:
   - `vault/02 - Horizon/Horizon hooks and API.md` (the API reference)
   - `vault/02 - Horizon/Example - smp mod (patterns to reuse).md` (the code model)
   - `vault/04 - Design/Architecture.md`

**At the end of every session (or after a significant step):**
1. Create or update `vault/06 - Journal/YYYY-MM-DD.md` (template: `vault/99 - Templates/Template - Journal.md`).
2. Tick off `vault/01 - Project/Roadmap.md`.
3. Update "Where we are", "Next step" and `updated` in `vault/00 - Home.md`.
4. Questions → `vault/01 - Project/Open questions.md`. Significant decision → a new ADR in `vault/05 - Decisions/` plus `Decision index.md`.
5. Something learned or corrected about Horizon → update the reference note and its `verified_on`.

Vault conventions: `[[Note title]]` wiki links, YAML frontmatter, API status ✅ / ⏳ pending / ❌ / ❓.

## Project rules
- **Never copy FTB Quests code, assets or text** (All Rights Reserved). We reimplement the concepts.
- No Mojang code or assets in the repo: game data is read at runtime.
- Use only **hook points** from the catalog, with no raw function injections (ADR-003) and no unnumbered ⏳ pending hooks.
- Only crates already in Horizon's `Cargo.lock` are allowed.
- `RUSTFLAGS=-D warnings`: any warning breaks the build.
- Logic goes in **pure, testable modules** (`hq_*.rs`, `cargo test`). Code that touches `Game` goes in `*_glue.rs` (the smp pattern).
- **Never run a command with `--trust-native-code` yourself** (Horizon rule: it is the player's decision). Prepare the command; the user runs it.

## Git and GitHub
- The only author is **LKTbgoodall**: `user.name = LKTbgoodall`, `user.email = bgoodall79@gmail.com` (repo-local config).
- **No `Co-Authored-By`**, and no Claude or "Generated with" attribution line, in commits or PRs.
- Remote: https://github.com/LKTbgoodall/Horizon_quest (branch `main`).
- **Commit and push after each step** without asking: Bastien tests by pulling on another PC. Commit messages in English.

## Machines
- **Dev PC (this one)**: no game, so no in-game test. Rust stable 1.99.1 `x86_64-pc-windows-gnu` is installed via rustup (`%USERPROFILE%\.cargo\bin`) for `cargo test`.
- **Home PC (Windows)**: Horizon launcher + Minecraft. Bastien runs `git pull` there, builds with `horizon-launcher native-build … --trust-native-code`, tests in game and reports back.
- So: prepare ready-to-run commands for the home PC, and ask for screenshots or `native/build.log` when something needs checking in game.

## Mod identity
- Mod id `horizon_quests`, display name "Horizon Quests", license MIT (ADR-009). In-game text is English-only for now.

## External references
- Horizon docs: https://playhorizon.dev/docs
- Engine: https://github.com/horizon-rust/horizon (`docs/native-mods.md`, `native-hooks.json`)
- Registry and examples: https://github.com/horizon-rust/horizon-mods (`native/smp`)
- Local reference clones planned in `ext/` (gitignored).
