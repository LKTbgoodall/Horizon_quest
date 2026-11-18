---
tags: [horizon, reference, manifest]
source: playhorizon.dev/docs/make-a-mod, /docs/mod-manifest, horizon/docs/native-mods.md, api/mod-files.md, horizon-mods/native/smp
verified_on: 2026-11-18
---
# Native mods: structure and manifest

Back: [[Horizon - Overview]]

> [!warning] Horizon `main` changed the native mod format on 2026-11-16
> PR horizon-rust/horizon#17145 ("The mod API: #16941 merged with main", merged 2026-11-16 01:32 UTC, merge `85b5bcca7c`) brought the fork's mod API upstream ([[Fork - arismoko mod API]]). Everything below this box describes the **old** format, which our mod still uses and which the released launcher (0.2.69, built before the merge) and the Nightly `9f42064` still take. Once a launcher and a Nightly built from the new `main` ship, the old layout is **refused**. See § "The new format" at the end.
> **2026-11-17**: launcher **0.2.70** (and 0.2.71) is built after the merge: it refuses the old layout (`bundle_in`), and builds only on the installed version's revision, refusing mods that pin another one (`MISMATCH`). The newest Nightly is still `9f42064` (no mod API), so for now neither format builds with an up-to-date launcher ([[2026-11-17]]).
> **2026-11-18**: **Nightly 2026-11-18 (`0b14f8a`)** is the first with the mod API. Since horizon-rust/horizon#18051 every Nightly is published with the signed SDK of its exact commit: a prerelease `sdk-<commit>` (assets `api-<target>.tar.xz`, `api-source.tar.xz`, `engine-*`, `runtime-*`, `build-data.tar.xz`, `surface.json`) and `sdk.json` on the Nightly itself (build commit, toolchain `1.99.1`, `content_api` 0.2, **`mixins.rev`** to pin in `[native] mixins`). A Nightly without its `sdk-<commit>` cannot build native mods. Nightlies are started by hand (no schedule since 2026-11-15). Our mod is on it since v0.22.0 ([[2026-11-18]]).

## Typical layout (model from the `horizon-mods` registry)
```
horizon_quests/
├── Cargo.toml              # for `cargo test` ; [lib] path → src/lib.rs (pure rules)
├── src/lib.rs              # #[path = "../mixins/horizon_quests/hq_X.rs"] pub mod hq_X;
├── compatibility.json      # used by native.py (registry)
├── LICENSE
└── mixins/horizon_quests/
    ├── horizon-mixin.toml  # MANIFEST
    ├── <helper>.rs         # 1 file per injection/hook
    ├── *_glue.rs           # code that touches Game
    ├── hq_*.rs             # testable pure rules
    └── data/               # embedded files (native_data!)
```

## Three ways to write code
1. **Trait mod (typed API)**: `[api] path = "engine"`. The package exports `pub fn hooks() -> Box<dyn horizon_client::mod_api::Hooks>`. Its methods `(&self, host, &mut XxxEvent) -> Flow` return `Flow::Continue` or `Flow::Cancel`. Scaffold: `cargo horizon-mod new <path> --template native --horizon <commit>`.
   - ⚠️ Some hooks are **not yet in the trait**: `player_command`, `screen_key_pressed_pre`, `screen_char_typed_pre`, `screen_mouse_scrolled_pre`, `mouse_button_pre/post`, `player_respawned`, `server_data_loaded`… Some helpers are also missing on `Host`: `player`, `sidebar`…
2. **Mixin bundle + helpers** (the `smp` style): `[[injection]]` with `hook` + `hook_version` (catalog name) or a raw `target`, a `kind`, a `helper` file and `bindings`. A helper can convert `&mut Game` into `&mut dyn Host` to use the typed API.
3. **Trait injections**: `trait_method = "x"` generates `inject_<mod_id>_<x>`.

→ Choice: [[ADR-004 Mixin bundle style like smp]]

## `horizon-mixin.toml`: known keys
```toml
[mod]
id = "horizon_quests"      # a-z, 0-9, _
version = "0.1.0"
priority = 100

[compatibility]
horizon_revision = "<commit 40 hex>"   # MUST equal the one in compatibility.json

[api]                     # only for a trait mod
path = "engine"

[dependencies]            # other typed mods (optional)
# base_mod = "^1"
# extra = { version = "^2", optional = true }   → cargo feature #[cfg(feature="extra")]

[[module]]                # the file becomes crate::game::<name>
id = "hq_core"
file = "crates/horizon-client/src/game.rs"
name = "hq_core"
source = "hq_core.rs"
# visibility = "pub"      # if needed (e.g. module in client_lib.rs)

[[injection]]
id = "register"
kind = "HEAD"             # HEAD | RETURN | BEFORE_CALL | MODIFY_ARG | OVERWRITE
target = "horizon_client::game::native_hooks::Game<'_>::native_register_commands"
helper = "register.rs"
bindings = [ { name = "commands", ty = "&mut Vec<crate::game::native_server::NativeCommand>", value = "commands" } ]
# or : hook = "<catalog name>", hook_version = N   (horizon-mixins ≥ 0.6.0)

[[crate]]                 # internal crate of the bundle (e.g. voicechat-server)
# id = "relay"  path = "relay"

[[post_build]]
```
- A `RETURN` helper binds the result via `{ name="_result", ty="()", value="$value" }`.
- A helper is **a free function** whose parameters match the bindings by name and type.

## `compatibility.json` (`horizon-mods` registry)
```json
{
  "game": "26.3",
  "hooks": 413,
  "horizon_repository": "https://github.com/horizon-rust/horizon",
  "horizon_revision": "<commit>",
  "horizon_mixins_repository": "https://github.com/horizon-rust/horizon-mixins",
  "horizon_mixins_revision": "<commit>"
}
```
- `hooks` is the **minimum** `NATIVE_HOOKS_API`. The mod compiles on any Horizon version ≥ that one.
- `smp` values as of 2026-11-10: hooks 413, horizon `5de5b6fd91b70be34562f56b747c9e06db692fa3`, mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

## `horizon-native.json` (inside the `.hnmod`, generated by `package`)
`schema: 1`, `id`, `version`, `game: "26.3"`, `horizon`, `mixins` (40 hex), `hooks` (optional). **An unknown field makes the file get rejected.**

## Constraints
- Allowed crates are **only those in Horizon's `Cargo.lock`**: no path or git crates, no `horizon-client` dependency (`E_CRATE_DEPENDENCY`). Available: `serde` 1.0.229, `serde_json` 1.0.151, `toml` 1.1.6, `uuid` 1.28, `parking_lot`, `once_cell`, `rand` 0.10.3, `yaml-rust2`, `rusqlite`, and also `chrono`, `log`, `flate2`, `ureq` (seen in smp).
- A `.hnmod` only contains `.rs` files, `.toml` files, `data/`, README and LICENSE. **PNGs and lang files** must go in the content half of a combined `.hmod` (`assets/`).
- `RUSTFLAGS=-D warnings`: **a warning fails the build**.
- Build error codes: `E_TARGET_NOT_FOUND`, `E_MODULE`, `E_HOOK_CATALOG`, `E_MODULE_CONFLICT`, `E_HOOK_UNKNOWN`, `E_HOOK_VERSION`. A rejected mod is **skipped** on a client build, but **fails** a server export.
- Client settings are stored in `horizon-mods/.horizon-mod-data/<id>/<key>`.
- Installation: `<instance>/horizon-native-mods/`; on/off state in `horizon-mods.txt` (`mod.<id>:true`).

## The new format (Horizon `main` after PR #17145, read 2026-11-16 at `90e97a1`)
Source: the modding book in Horizon `docs/modding/src/` (`how-to/native-mods/01` to `04`, `reference/manifest.md`, `start/first-mod.md`, `start/writing-hooks.md`, `concepts/native-route.md`). `docs/native-mods.md` is now only a pointer to the book.

**One flat package, the same for the project folder and the `.hmod`:**
```text
horizon_quests/
  horizon-mod.toml   # the ONLY manifest: id, name, version, api = "0.2", ... plus [native]
  icon.png           # optional, square PNG <= 512 x 512, <= 256 KiB
  assets/horizon_quests/
  data/horizon_quests/
  Cargo.toml         # one crate, depends on horizon-mod-api (lib `mod_api`), never on the game
  Cargo.lock         # registry dependency pins (horizon-launcher native-lock)
  src/lib.rs         # #[mod_api::horizon_mod] impl Hooks for Mod { ... }
```
```toml
[native]
game = "26.3"
horizon = "<40 hex Horizon commit>"
mixins = "<40 hex horizon-mixins commit>"
```
- **Refused**: `horizon-native.json`, a hand-written `horizon-mixin.toml`, a `mixins/<id>/` level, input `native/` or `rust/` folders. No mod-id folder level in the archive. Unknown manifest fields refuse it.
- **Hooks** are methods of `mod_api::Hooks` (declared in `crates/horizon-mod-vocab/src/hooks/`, 729 files), with a typed host: `&mut dyn ServerHost` / `ClientHost`. `#[mod_api::horizon_mod]` on the one `impl Hooks` at the crate root subscribes the mod to the methods it writes. Per-object behaviour goes in **Parts**.
- **No more `native_*` hook points or `native-hooks.json` catalogue** for mods; `NATIVE_HOOKS_API` and `compatibility.json` "hooks" no longer mean anything.
- **The game is never rebuilt.** The launcher compiles the mod's crate against a **prebuilt SDK** of the pinned build (downloaded, signed), then links. A mod that uses only the mod API can build on later builds when every API item it uses is unchanged (`uses.json`, `cargo horizon-mod fits`).
- **Dependencies**: any crates.io crate with an exact version and checksum in the mod's `Cargo.lock` (no more "only crates in Horizon's lock"); no git / path-outside / `[patch]`. A mod build script is refused (`E_NATIVE_BUILD_SCRIPT`), and so are Cargo features (`E_NATIVE_FEATURES`). Beware: a registry copy of `serde` is not the SDK's (E0277 across the boundary).
- **Data**: no `native_data!`. Files go in the normal `data/<id>/` and `assets/<id>/` trees, read with server reload listeners (`Hooks::add_server_reload_listeners`) or `host.resource(PackType::ServerData, &id)`. Code-only tables must be generated Rust source in `src/` (no build script).
- **Content and code in one `.hmod`, natively**: the launcher compiles `src/`, the game loads `assets/` and `data/`. This is what [[ADR-014 Publish one hmod with content and code]] wanted; horizon#15933 (content dropped) does not apply to this format.
- **The native route** still exists for game internals: `[[native.mixin]]`, `[[native.loop]]`, `[[native.bridge]]`, `[[native.module]]` under `[native]` (horizon-mixins rows, written by the launcher in its staging area). A mod using it is tied to one exact build, gets the game's real types (`Game`...) and is not sandboxed. `[[native.state]]` is refused (use attachments). Lighter: top-level `[[inject]]` listeners (`head` / `return` / `replace`) with `mod_api` values.
- **Tools**: `cargo horizon-mod new <path> --template native` (pins the current Nightly, fetches the SDK into `.horizon/sdk/`, never committed), `check`, `build` (writes `target/horizon-mod/<id>.hmod` with `uses.json`), `fits`, `update` (moves the pin, lists what breaks), `hooks <text>` (searches hooks, with their NeoForge / Fabric / Paper names). Build: `horizon-launcher native-build <path> --instance NAME --trust-native-code` (Bastien only).
- **Limits**: 511 source files / 64 MiB, content 65,536 files, 16 MiB each; a registry file 64 MiB.

### Learned while porting (2026-11-16, Horizon `main` `c07bfa7`)
- **What the package takes** (`horizon-mod-package` `manifest.rs`, `source_file`): `Cargo.toml`, `Cargo.lock`, files under `src/` ending in `.rs` or `.toml`, `README*` / `LICENSE*` at the top, `horizon-mod.toml`, `icon.png`, `assets/`, `data/`. Any other `.rs` or `.toml` refuses the package ("unexpected package path"): **no `tests/` or `examples/` folder with Rust in the mod's root**. Other files elsewhere are left out silently, so an `include_str!` of a file outside `src/` breaks the launcher's build. `.git`, `target`, `.horizon` are never packed.
- **Type-checking without the SDK**: the template's `horizon-mod-api = { path = ".horizon/sdk/api/crates/horizon-mod-api" }` works with `.horizon/sdk/api` a junction or symlink to a Horizon checkout (cargo finds the checkout's workspace). `cargo check` of the API crates takes about 1.5 min the first time; `cargo test` links (no unresolved host symbols). Do **not** put a `[workspace]` table in the mod's `Cargo.toml`: the SDK crates then inherit from it and fail.
- **Item file** `data/<id>/horizon/item/<path>.json`: `ItemFile` with `deny_unknown_fields`: `template`, `max_stack_size` (1 to 99), `rarity` (lowercase), `components` (a map, `!` removes), `crafting_remainder`, `block`, `arm_pose`.
- `side = "both"` is required for a mod with registry declarations (a `side = "server"` mod declaring an item is refused); the old content half's `side = "client"` no longer applies.
- **Building a commit without a Nightly** is possible (`horizon-launcher developer-install` of a built checkout, `python3 tools/prebuilt_game.py build SDK_FOLDER`, then `HORIZON_LAUNCHER_DEVELOPER_SDK=SDK_FOLDER horizon-launcher native-build MODS --version DEVELOPER_TAG --trust-native-code`), but it needs the whole game built and the exporters prepared: not worth it for us.
- `cargo horizon-mod check FILE` validates a packed `.hmod` without the SDK (our ported package passes, 2026-11-16). `cargo horizon-mod check` in a project fetches the pinned SDK first; with none published it empties `.horizon/sdk/` (a junction there is removed, its target untouched).
