---
tags: [horizon, reference, manifeste]
source: playhorizon.dev/docs/make-a-mod, /docs/mod-manifest, horizon/docs/native-mods.md, api/mod-files.md, horizon-mods/native/smp
verifie_le: 2026-11-10
---
# Mods natifs : structure et manifeste

Retour : [[Horizon - Vue d'ensemble]]

## Arborescence type (modèle du registre `horizon-mods`)
```
horizon_quest/
├── Cargo.toml              # pour `cargo test` ; [lib] path → src/lib.rs (règles pures)
├── src/lib.rs              # #[path = "../mixins/horizon_quest/hq_X.rs"] pub mod hq_X;
├── compatibility.json      # utilisé par native.py (registre)
├── LICENSE
└── mixins/horizon_quest/
    ├── horizon-mixin.toml  # MANIFESTE
    ├── <helper>.rs         # 1 fichier par injection/hook
    ├── *_glue.rs           # code qui touche Game
    ├── hq_*.rs             # règles pures testables
    └── data/               # fichiers embarqués (native_data!)
```

## Trois façons d'écrire du code
1. **Trait mod (typed API)** : `[api] path = "engine"`. Le package exporte `pub fn hooks() -> Box<dyn horizon_client::mod_api::Hooks>`. Ses méthodes `(&self, host, &mut XxxEvent) -> Flow` renvoient `Flow::Continue` ou `Flow::Cancel`. Scaffold : `cargo horizon-mod new <path> --template native --horizon <commit>`.
   - ⚠️ Certains hooks **ne sont pas encore dans le trait** : `player_command`, `screen_key_pressed_pre`, `screen_char_typed_pre`, `screen_mouse_scrolled_pre`, `mouse_button_pre/post`, `player_respawned`, `server_data_loaded`… Certains helpers manquent aussi sur `Host` : `player`, `sidebar`…
2. **Bundle mixin + helpers** (le style de `smp`) : `[[injection]]` avec `hook` + `hook_version` (nom du catalogue) ou une `target` brute, un `kind`, un fichier `helper` et des `bindings`. Un helper peut convertir `&mut Game` en `&mut dyn Host` pour utiliser la typed API.
3. **Trait injections** : `trait_method = "x"` génère `inject_<mod_id>_<x>`.

→ Choix : [[ADR-004 Style bundle mixin comme smp]]

## `horizon-mixin.toml` : clés connues
```toml
[mod]
id = "horizon_quest"      # a-z, 0-9, _
version = "0.1.0"
priority = 100

[compatibility]
horizon_revision = "<commit 40 hex>"   # DOIT égaler celui de compatibility.json

[api]                     # seulement pour un trait mod
path = "engine"

[dependencies]            # autres mods typés (optionnel)
# base_mod = "^1"
# extra = { version = "^2", optional = true }   → feature cargo #[cfg(feature="extra")]

[[module]]                # le fichier devient crate::game::<name>
id = "hq_core"
file = "crates/horizon-client/src/game.rs"
name = "hq_core"
source = "hq_core.rs"
# visibility = "pub"      # si besoin (ex. module dans client_lib.rs)

[[injection]]
id = "register"
kind = "HEAD"             # HEAD | RETURN | BEFORE_CALL | MODIFY_ARG | OVERWRITE
target = "horizon_client::game::native_hooks::Game<'_>::native_register_commands"
helper = "register.rs"
bindings = [ { name = "commands", ty = "&mut Vec<crate::game::native_server::NativeCommand>", value = "commands" } ]
# ou : hook = "<nom catalogue>", hook_version = N   (horizon-mixins ≥ 0.6.0)

[[crate]]                 # crate interne au bundle (ex. voicechat-server)
# id = "relay"  path = "relay"

[[post_build]]
```
- Un helper `RETURN` lie le résultat via `{ name="_result", ty="()", value="$value" }`.
- Un helper est **une fonction libre** dont les paramètres correspondent aux bindings par nom et par type.

## `compatibility.json` (registre `horizon-mods`)
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
- `hooks` est le `NATIVE_HOOKS_API` **minimum**. Le mod compile sur toute version d'Horizon ≥ celle-là.
- Valeurs de `smp` au 2026-11-10 : hooks 413, horizon `5de5b6fd91b70be34562f56b747c9e06db692fa3`, mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

## `horizon-native.json` (dans le `.hnmod`, généré par `package`)
`schema: 1`, `id`, `version`, `game: "26.3"`, `horizon`, `mixins` (40 hex), `hooks` (optionnel). **Un champ inconnu fait refuser le fichier.**

## Contraintes
- Les crates autorisées sont **seulement celles du `Cargo.lock` d'Horizon** : pas de crate path ou git, pas de dépendance `horizon-client` (`E_CRATE_DEPENDENCY`). Disponibles : `serde` 1.0.229, `serde_json` 1.0.151, `toml` 1.1.6, `uuid` 1.28, `parking_lot`, `once_cell`, `rand` 0.10.3, `yaml-rust2`, `rusqlite`, et aussi `chrono`, `log`, `flate2`, `ureq` (vus dans smp).
- Un `.hnmod` ne contient que des `.rs`, des `.toml`, `data/`, README et LICENSE. **Les PNG et les fichiers lang** doivent aller dans la moitié contenu d'un `.hmod` combiné (`assets/`).
- `RUSTFLAGS=-D warnings` : **un warning fait échouer le build**.
- Codes d'erreur de build : `E_TARGET_NOT_FOUND`, `E_MODULE`, `E_HOOK_CATALOG`, `E_MODULE_CONFLICT`, `E_HOOK_UNKNOWN`, `E_HOOK_VERSION`. Un mod refusé est **ignoré** sur un build client, mais **fait échouer** un export serveur.
- Réglages client stockés dans `horizon-mods/.horizon-mod-data/<id>/<key>`.
- Installation : `<instance>/horizon-native-mods/` ; état on/off dans `horizon-mods.txt` (`mod.<id>:true`).
