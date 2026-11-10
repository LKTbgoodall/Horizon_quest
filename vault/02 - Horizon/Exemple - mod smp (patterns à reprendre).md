---
tags: [horizon, reference, exemple, patterns]
source: horizon-mods/native/smp (v0.36.0), recipes, coords-hud, fps-overlay
verifie_le: 2026-11-10
---
# Exemple de référence : le mod `smp`

> [!important] Pourquoi cette note
> `smp` (économie, claims, marché, enchères, casino…) est le **plus gros mod natif côté serveur** du registre, et il est fait par l'équipe Horizon. C'est **le modèle à copier** pour Horizon Quest : commandes serveur, menus coffre, sauvegarde JSON, état global et tests.
> Code : `horizon-mods/native/smp/` (https://github.com/horizon-rust/horizon-mods/tree/main/native/smp)

Retour : [[Hooks et API Horizon]] · [[Mods natifs - structure et manifeste]]

## 1. Organisation du projet
```
native/smp/
├── Cargo.toml            # package horizon-mod-smp, edition 2024, MIT ; deps : serde, serde_json
├── src/lib.rs            # #[path = "../mixins/smp/smp_X.rs"] pub mod smp_X;  → cargo test hors jeu
├── compatibility.json    # game 26.3, hooks 413, horizon_revision, horizon_mixins_revision
└── mixins/smp/
    ├── horizon-mixin.toml
    ├── register.rs, command.rs, tick.rs…   # 1 helper = 1 injection (paramètres = bindings)
    ├── glue.rs, *_glue.rs                  # code qui touche le jeu (pas dans lib.rs)
    └── smp_*.rs                            # RÈGLES PURES, testables (≈190 #[test])
```
- **Séparation règles / glue** : les modules `smp_*.rs` ne dépendent pas des types du jeu, ce qui les rend testables avec `cargo test`. Les `*_glue.rs` font le pont avec `Game`. **On reprend ce découpage** : voir [[Architecture]].
- Les modules deviennent `crate::game::<name>` (`[[module]] file="crates/horizon-client/src/game.rs"`). Ils se référencent via `super::smp_store`.
- `chrono`, `log`, `ureq` et `flate2` viennent du workspace Horizon, sans être déclarés dans le Cargo.toml. Seules les crates du `Cargo.lock` d'Horizon sont autorisées.

## 2. Injections utilisées (toutes `kind="HEAD"`, cible `Game<'_>::…` dans `horizon_client::game::native_hooks`)
| Hook | Bindings | Usage pour nous |
|---|---|---|
| `native_register_commands` | `commands: &mut Vec<NativeCommand>` | enregistrer `/quests …` |
| `native_perform_command` | `game, player: &NativeServerPlayer, command: &str, ran: &mut bool` | exécuter nos commandes |
| `native_server_tick` | `game, phase: ServerTickPhase` | boucle de détection des tâches (1×/s), sauvegarde |
| `native_level_tick_post` | `game, dimension: &str` | — |
| `native_world_lifecycle` | `phase: WorldPhase` | flush à `WorldCloseFinal \| WorldCloseFailed` |
| `native_menu_clicked` | `game, uuid: u128, window: i32, slot: i32, mode: i32` | **clics dans le livre (menu coffre)** |
| `native_menu_closed` | `game, uuid, window` | fermer la vue |
| `native_destroy_block` | `game, pos: [i32;3], canceled: &mut bool` | tâche « casser un bloc » (⚠ l'id du bloc n'est pas fourni) |
| `native_right_click_block`, `native_use_item_on`, `native_interact_entity` | … | tâches d'interaction |
| `native_attack_entity` | `game, kind: &str, pos, canceled` | ⚠ ne donne pas l'issue (le kill) |
| `native_living_incoming_damage` | `entity: i32, damage_type: &str, canceled` | ⚠ ni attaquant ni montant |
| `native_entity_join_level` | `entity: u32, kind, pos, loaded: bool` | — |
| `native_player_command` | `player: u128, command: &str` | — |
| `native_player_logged_out` | `game, player: u128, name: &str` | sauvegarde à la déconnexion |
| chat : `horizon_client_lib::lan::hub::chat::native_player_chat` | `player: u128, message: &str` | — |

Ce qu'il **n'y a pas** dans smp : pas de hook de kill, de mort ou de login (la connexion est détectée en diffant `native_server_players()` chaque seconde, la mort en sondant `health <= 0`), pas de pickup, de craft ni d'advancement, pas de toast. → À vérifier dans le catalogue : [[Hooks et API Horizon]].

## 3. Commandes
```rust
commands.push(NativeCommand::new("pay <player:word> <amount:integer>"));   // tout le monde
commands.push(NativeCommand::op("setwarp <name:word>", 3));                 // niveau op 3
```
- Types d'arguments : `word`, `integer`, `item`, `text` (gourmand). `[x]` = optionnel. **Une usage-string par forme de sous-commande.** Le client reçoit l'autocomplétion.
- Dispatch : la ligne arrive sans le `/`. On vérifie `game.native_on(ID)` et le premier mot, puis `*ran = true`.
- `NativeServerPlayer` : `uuid` (u128), `name`, `permission` (u8, 0-4), `game_mode`, `position`, `dimension`, `health`, `yaw`, `pitch`, `entity_id`.
- Parsing : `split_whitespace()` puis motifs de slices (`match args { [verb @ ("give"|"take"), who, amount, ..] => … }`).
- Permission revérifiée dans les règles : `if me.level < 3 { return vec![Out::Error(..)] }`.
- **Type de sortie pur** : `enum Out { Reply(String), Error(String), Tell(u128,String), All(String) }`, traduit par la glue en `native_feedback` / `native_tell` / `native_broadcast`. `native_feedback` ne marche **que dans une commande**.

## 4. Interface sans mod client : menus coffre
```rust
let window: Option<i32> = game.native_open_menu(uuid, 6 /*rows*/, Text::literal("Quêtes"), items /*Vec<Option<ItemStack>>*/);
game.native_set_menu_items(uuid, items);   // redessiner (page suivante…)
game.native_close_menu(uuid);
```
- Vues : `HashMap<u128, (Option<i32> /*window*/, Page)>`. On insère `(None, page)` avant d'ouvrir, puis on stocke `Some(w)`.
- Clics : `mode` 0 = clic, 1 = shift-clic. `slot` compte depuis 0. `native_menu_inventory_slot(SIZE, slot)` convertit un clic dans l'inventaire du joueur.
- **Layout pur** (`smp_market_menu.rs`) : `layout() -> Vec<Cell>` et `click(slot) -> Click`, testables. La dernière ligne (45..54) sert de barre de nav : `BACK=45`, `PREV=47`, `NEXT=51`, `CLOSE=53`. `PER_PAGE=45`.
- Item avec nom et lore :
```rust
let mut stack = ItemStack { item: item.into(), count: 1, damage: 0, extra: None };
stack.data_mut().custom_name = Some(component_nbt(&json!({"text": name, "italic": false, "color": "gold"})));
stack.data_mut().set_other("minecraft:lore", Some(Tag::List(lines)));
```
- Icônes usuelles : `gray_stained_glass_pane` (fond), `arrow` (nav), `barrier` (fermer), `book` (aide), `lime_concrete`/`red_concrete` (boutons).
- Bonus : une image 128×128 sur une carte (`map_id` + `native_send_map`).

## 5. Autres sorties visuelles
- Chat : `Text::literal(s).with_color(text::GOLD).bold()`, texte cliquable `ClickEvent::RunCommand` / `SuggestCommand` + `hover`.
- Barre d'action : `game.native_action_bar(uuid, Text)`.
- **Sidebar** : `game.native_sidebar(uuid, Some(&NativeSidebar { title, lines }))`. C'est idéal pour la **quête suivie / épinglée**.
- Tab list : `game.native_tab_list(uuid, &header, &footer)`.
- Titres, boss bars et sons : via **commandes console** `game.native_run_command("title … / bossbar … / playsound …")`, exécutées **après avoir relâché le lock**.
- Toast : ❌ pas d'API (seulement le son `minecraft:ui.toast.challenge_complete`).

## 6. Items et inventaire
- Dans une commande : `native_count_items(id)`, `native_take_items(id,n)`, `native_give_items(id,n)`, `native_room_for(id)`, `native_carried_items()`, `native_held()`.
- Hors commande (tick, clic) : `native_player_stacks(uuid)`, `native_give_stack(uuid, stack) -> reste`, ou `native_run_command("give …" / "clear …")`.
- Inventaire même hors ligne : `native_inventory(uuid, NativeInventoryKind::Player|Ender)`.
- Sérialisation : `item_to_nbt` / `item_from_nbt` + `write_network`, en hex.

## 7. Persistance
- Du **JSON via serde** dans `game.native_world_dir()?.join("horizon-smp")`, avec un fichier par partie. Joueurs indexés par UUID string (`uuid_string(u128)` / `parse_uuid`).
- `smp_store` :
  - `read_json` : fichier absent → `Default` ; fichier corrompu → `Err`, et la partie se désactive (**on n'écrase jamais**).
  - `save_json` : asynchrone, via un thread `smp-writer`.
  - `write_atomic` : temp + `sync_all` + rename.
  - `flush()`.
- Sauvegarde : en fin de tick, **seulement les parties modifiées** (`changed`), plus un flush à la fermeture du monde.
- `config.json` : sections `#[serde(default)]`, réécrit quand une nouvelle version ajoute des clés.
- Pas d'`AttachmentType` utilisé.

## 8. État et threads
- `static STATE: Mutex<Option<Smp>>`, ouvert paresseusement et rouvert si le dossier du monde change. Locks tolérants au poison : `.lock().unwrap_or_else(|e| e.into_inner())`.
- Tick économe : `TICKS: AtomicU64`, le travail ne se fait que tous les 20 ticks ou si `BUSY`.
- Ré-entrance : `try_lock()` dans les hooks *fire-and-forget*. Les effets qui redéclenchent des hooks (commandes) passent par une file `Vec<String>` exécutée après `drop(guard)`.
- Événements reçus pendant un lock (logout) : file statique traitée au tick suivant.
- Côté client : `thread_local!` + `RefCell`.

## 9. Tests
- `#[cfg(test)] mod tests` dans chaque fichier de règles, avec un dossier temp `std::env::temp_dir().join(format!("…-{}", process::id()))`.
- **Proofs E2E** : `proof.py` lance `horizon-client --dedicated --nogui` et deux clients `horizon-ctl --offscreen` qui lisent le chat, les menus (slots numérotés depuis 300) et l'inventaire. Commande : `native.py proof`.

## 10. Côté client (mod `recipes`, `coords-hud`)
- Panneau à côté des écrans de conteneur : `native_panel_items`, `native_panel_recipes`, `native_panel_event` (`Search`, `Clicked`, `Tab`, `Action`, `Back`…). Le jeu rend lui-même les icônes d'items.
- HUD : `native_hud(game, batch, corners)`, `native_fill`, `native_text`, `native_text_width`.
- Ligne Options > Mods : `NativeMod { id, title, description, keys }`. Touches via `native_mod_key`. Réglages via `native_setting` / `native_set_setting` (`horizon-mods/.horizon-mod-data/<id>/<key>`).
- Commande client : `native_command(game, command, ran)`, à distinguer de `native_perform_command` côté serveur.
