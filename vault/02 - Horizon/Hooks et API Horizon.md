---
tags: [horizon, reference, api, hooks]
source: horizon/docs/native-mods.md (NM, 5523 lignes), native-hooks.json (434 hooks, 177 helpers), mods.md, native-hooks-neoforge.md, api/stability.md, api/server.md
verifie_le: 2026-11-10
---
# Hooks et API Horizon (vue « mod de quêtes »)

Retour : [[Horizon - Vue d'ensemble]] · Exemples concrets : [[Exemple - mod smp (patterns à reprendre)]]

> [!warning] Légende
> - **API N** : il faut `"hooks": N` minimum. Le niveau courant de main est **≥ 415** (NF indique 409 le 5 nov., NM documente jusqu'à 415). `smp` cible 413.
> - **⏳ pending** : documenté sous la forme `@API:name@`, sans numéro pour l'instant, et **absent du snapshot `native-hooks.json`**. On **ne peut pas** encore le cibler. À revérifier à chaque mise à jour d'Horizon : [[Questions ouvertes]].
> - Beaucoup de signatures (`native_hud`, `native_fill`, `native_text`, `native_sprite`, `native_key`…) ne sont **pas documentées** : il faut lire `crates/horizon-client/src/game/native_hooks.rs` dans le code d'Horizon.
> - Les références `NM:1234` renvoient aux lignes de `docs/native-mods.md` du dépôt horizon.

## 1. Réseau et synchronisation client ↔ serveur
| Besoin | API | Statut |
|---|---|---|
| Messages de mod | `native_net_send_to_player(player, channel, data)` / `native_net_send_to_server(channel, data)` ; réception `native_net_from_client(player, channel, data)` / `native_net_from_server(channel, data)` | ✅ API 40 |
| Canal | `horizon:mod_net`, sous-canal `mod_id:name` (`[a-z0-9_]` : `[a-z0-9_./-]`, 1-64 caractères chacun) | |
| Limites | **30 000 octets par message, 32 messages par tick** dans chaque sens et par joueur. Tous les mods entendent tout, il faut filtrer par canal. Un client vanilla ne reçoit **jamais** ces messages. | |
| Attachments synchronisés | `AttachmentType { sync: true }` : envoyé aux clients Horizon (le joueur et ceux qui le suivent, ou tout le niveau), 1 MiB max | ✅ |
| Payload brut | `native_send_custom_payload_to_player` (1 MiB, atteint aussi les clients vanilla), `native_custom_payload_from_client` | ⏳ |
| Tracking | `native_player_start_tracking` / `stop_tracking` | ✅ API 141 |
| Connexion | `native_player_logged_in` / `native_player_logged_out` (appairés, y compris à l'arrêt du serveur) | ✅ API 57 |

⚠️ Les données venant du client sont **non fiables** : il faut les traiter comme une saisie du joueur et tout revalider côté serveur.

## 2. Déclencheurs de tâches (côté serveur)
| Tâche | Hook | Statut |
|---|---|---|
| **Obtenir un item** | `native_player_inventory_changed(player, stacks)` : 41 slots, déclenché par le check vanilla `InventoryChangeTrigger`. « *what quest … mods need instead of polling* » | ⏳ (idéal) |
| | `native_item_picked_up` (UUID, `canceled`) | ✅ API 40 |
| | `native_item_picked_up_v2(player, stack, thrower, position, canceled)` | ⏳ |
| | **Repli** : sondage `native_inventory(uuid, kind)` (41 slots) toutes les N ticks | ✅ API 132 |
| | `native_count_items(item)` (joueur actif, stacks simples) | ✅ API 9 |
| **Crafter** | `native_item_crafted` (uuid, recipe id, résultat avec composants). Grille et table seulement | ✅ API 161 |
| **Fondre** | `native_furnace_extract(player, name, pos, item, count)` | ✅ API 228 |
| Autres | `native_anvil_craft_post` (327), `native_enchant_item` (209), `native_trade_with_villager` (310), `native_brewing_complete` (198) | ✅ |
| **Tuer** | `native_mob_killed` (mêlée, sweep, lance ; nomme le joueur) | ✅ API 40 |
| | `native_living_death(entity, kind, damage_type, attacker, cancel)` (mobs et joueurs) | ✅ API 392 |
| | `native_loot_drops(context, drops)` : le contexte inclut le joueur tueur ou casseur | ✅ API 75 |
| Mort du joueur | `native_player_died` | ✅ API 40 |
| **Casser un bloc** | `native_destroy_block` (état du joueur injecté ; ⚠ smp ne lit pas l'id du bloc) | ✅ API 14 |
| Poser un bloc | `native_place_block` | ✅ API 14 |
| **Advancement** | `native_player_advancement_done(player, name, advancement)` | ✅ API 249 |
| | `native_player_advancement_progress` (320), `native_player_advancement_criterion_grant` (339) | ✅ |
| **Dimension** | `native_player_changed_dimension(player, name, from, to)`. ⚠ La doc se contredit sur le solo (NM:2278 vs NM:1291) : **à tester** | ✅ API 154 |
| | `native_player_portal` (271), `native_player_teleported` (170) | ✅ |
| **Position / zone** | `native_player_tick` (chaque tick pour chaque joueur) ou sondage de `native_server_players()` | ✅ API 31 |
| | `native_player_move(...)` | ⏳ |
| **Biome** | serveur : `native_chunk_snapshot(...).noise_biome` + `native_biome_name` | ⏳ |
| **Structure** | ❌ **pas documenté** | ❌ |
| **Stat** | `native_player_stat_award(player, name, stat_type, stat, value, cancel)` | ✅ API 321 |
| | lecture `native_player_stats(uuid)`, `native_player_stat(...)` | ⏳ |
| **XP** | `native_player_xp_change` (394), `native_player_level_change(player, name, from, to)` (347) | ✅ |
| Utiliser un item | `native_use_item` (13), `native_use_item_on` (13), `native_living_entity_use_item_finish` (90, manger/boire) | ✅ |
| Interagir avec une entité | `native_interact_entity` (29) | ✅ |
| Pêcher, apprivoiser, élever | `native_player_fish` (160), `native_entity_tame` (220), `native_entity_breed` (213) | ✅ |
| Chat | `native_player_chat(player, name, message, replies, cancel)` | ✅ API 399 |
| **Trigger custom** | `native_trigger_criterion(uuid, trigger, test)` : déclenche un advancement de datapack | ✅ API 69 |
| Tick | `native_server_tick` (4), `native_level_tick_pre/post` (23) | ✅ |

## 3. Donner des récompenses
| Récompense | API | Statut |
|---|---|---|
| **Item avec composants** | `native_give_stack(uuid, stack)` depuis n'importe quel hook serveur. **Renvoie ce qui n'a pas tenu, rien n'est jeté** : c'est à nous de le jeter ou de le stocker | ✅ API 98 |
| Item simple | `native_give_items(item, count)` (joueur actif uniquement) | ✅ API 9 |
| Construire un stack | `ItemStack::from_nbt`, `mod_api::parse_snbt`, `native_item_from_snbt` | partiel ⏳ |
| **XP** | `native_give_experience(uuid, points)`, `native_give_levels(uuid, levels)`. Renvoie `false` si la liste des joueurs est occupée | ✅ API 101 |
| **Commande** | `native_run_command(line)` : console, niveau 4, sans sortie, **seulement là où aucun joueur n'est injecté** (tick, menu). Les récompenses sont donc **différées au tick** | ✅ API 21 |
| | `native_run_command_captured`, `native_run_command_as(player, line)` | ⏳ |
| Message | `native_tell(uuid, text)`, `native_broadcast`, `native_feedback` (dans une commande), `native_action_bar(uuid, text)` | ✅ |
| Titre / boss bar | `native_title(...)`, `native_boss_bar(...)`. En attendant : commandes `title` / `bossbar` via `native_run_command` (comme smp) | ⏳ |
| **Toast** | client : `native_add_toast(id, icon, title, description, challenge)`. « *A quest mod's server half tells the player's client half (mod messages) and the client half calls it* ». Côté vanilla : passer par un advancement + `native_trigger_criterion` | ⏳ / ✅ |
| Son | serveur : `native_play_sound_to` (⏳), commande `playsound` ; client : `native_play_local_sound` (83) | |
| Effets, téléport | `native_add_effect`, `native_teleport_player` | ⏳ |
| Loot table | pas de helper « tirer et donner » : on passe par `native_run_command("loot give …")` (non vérifié) ou par **nos propres tables** | ❓ |

## 4. Persistance
| Moyen | Détails | Pour nous |
|---|---|---|
| **Fichiers dans le monde** | `native_world_dir()` : « *a server mod keeps its data in a folder of its own so the world's backups carry it* » (le pattern de smp : `world/horizon-smp/*.json`) | ✅ **choix principal** : [[ADR-005 Progression en JSON dans le dossier du monde]] |
| AttachmentType | `Holder::Level/Chunk/BlockEntity/Entity/Player(uuid)`, codec en NBT `Compound`. ⚠ Uniquement les joueurs **en ligne**. Au respawn, seulement `copy_on_death: true`. Sans codec, la valeur est perdue au restart | utile pour **sync** des données par joueur vers le client |
| Fichiers joueur | `native_player_load_from_file` / `save_to_file` (323/324) : `players/data/<uuid>.<suffix>` | option |
| Config | `native_config_dir()` (26) ; `native_register_configs` + `NativeConfig::new(mod_id, kind, default)` → `config/<id>-<kind>.toml` (397), hooks `native_config_loading/reloading/unloading` | ✅ config du mod |
| Datapacks | `native_add_server_reload_listeners` + `NativeReloadListener::json(id, folder)` lit `data/<ns>/<folder>/*.json` de tous les packs, appliqué à chaque `/reload` (400) | 💡 **quêtes livrables en datapack !** |
| Données embarquées | `native_data!("<mod id>", "path")` (41) | quêtes par défaut |
| Réglages client | `native_setting` / `native_set_setting`, `native_data_dir(mod_id)` (⏳) | préférences UI |

## 5. Interface
### Côté serveur, compatible clients vanilla
- **Menus coffre** (API 10) : `native_open_menu(uuid, rows, title, items)`, `native_set_menu_items`, `native_close_menu`. Clics dans `native_menu_clicked(uuid, window, slot, button, mode)` : « *No click moves anything* ». `native_menu_closed`. Affichés en `generic_9x1..9x6`.
- **Sidebar** : `native_sidebar(uuid, sidebar)`, 15 lignes, pour un seul joueur (⚠ pas sur `Host`, à appeler depuis un helper mixin). `native_tab_list` (49).
- Chat et action bar (voir plus haut). Cartes 128×128 (images de chapitre ?). Hologrammes client (⏳).

### Côté client (Horizon uniquement) : un vrai écran custom
- **Écran de mod** (API 111) : `native_mod_screen_open(mod_id)`, `native_mod_screen()`, `native_mod_screen_close(mod_id)`. Échap le ferme. Il ne met pas le solo en pause et s'ouvre seulement « par-dessus le monde ».
- Rendu dans `native_screen_render_post` (78) : id d'écran, souris en pixels GUI, partial tick.
- **Entrées** (toutes cancellables) :
  - `native_screen_mouse_button_pressed_pre` / `_released_pre` (80) ;
  - `native_screen_mouse_dragged_pre(screen, button, delta, modifiers, canceled)` (123) → **pan du canvas** ;
  - `native_screen_mouse_scrolled_pre` (93) → **zoom** ;
  - `native_screen_key_pressed_pre`, `native_screen_char_typed_pre` (104) → **saisie de texte (éditeur)** ;
  - `native_screen_key_released_pre` (118) ;
  - `native_screen_opening/init_post/closing` (365-367).
- **Dessin** :
  - `native_fill`, `native_text`, `native_text_width`, `native_sprite` (textures GUI `assets/<ns>/textures/gui`) ;
  - `native_quad(batch, points, argb)` → **lignes de dépendance** ;
  - `native_item_stack(batch, stack, at, size)` (313) → **icônes de quêtes** ;
  - `native_item_model` (111) ;
  - `native_mod_image(w, h, rgba)` (355) ;
  - nine-slice, tooltips et `gui_paint` (411, détails non documentés) ;
  - `native_draw_scene` (3D).
- ❌ Ce qui **n'existe pas** : un clip/scissor 2D général, une bibliothèque de widgets, une zone de texte, une liste scrollable. **Tout est à construire nous-mêmes** (voir [[Interface - Livre de quêtes]]).
- **Visualiseur de recettes** : `native_show_recipes(stack, lookup)`. « *A mod that draws items on screens of its own (a quest book, a guide) can open whatever recipe viewer is installed* ».
- HUD : `native_hud`, `native_gui_layer_pre/post` (139), `native_gui_render_post` (410), `native_world_primitives` (lignes, boîtes et texte dans le monde, `.through_walls()`) → **waypoint de quête**.
- Ouvrir le livre :
  - touche : `native_key`, `native_input_key` (121), `native_mod_key` / `native_set_mod_key` (411) ;
  - commande client : `native_commands` / `native_command` (35) ;
  - item réclamé : `native_claim_items(mod_id, ids)` + `native_poll` (116), un clic droit avec cet item n'atteint jamais le serveur ;
  - bouton dessiné sur l'inventaire.
- Lecture de l'inventaire du client : `host.own_inventory()` (⏳), `host.menu_stacks()`, `native_player()`.

## 6. Commandes
- `native_register_commands(commands)` (API 8), appelé une fois à l'ouverture du monde.
- Création : `NativeCommand::new("quests open [quest:word]")` ou `NativeCommand::op("quests reload", 2)`.
- Types d'arguments : `word`, `text` (reste de la ligne, en dernier), `integer`, `number`, `boolean`, `player`, `item`, `block_pos`. Les arguments optionnels vont à la fin entre `[]`. ⏳ `word?`, `text?`, `player_name`.
- Exécution : `native_perform_command(player, command, ran)`. **Ce n'est pas un exécuteur Brigadier** : on parse la ligne soi-même, l'arbre ne sert qu'à l'autocomplétion côté client.
- Console : `native_perform_console_command` (99).
- Permissions : niveaux op 0-4 via `NativeCommand::op`. Les nœuds de permission (`native_has_permission(player, node, default_level)`, `NativeCommand::permission(node)`) sont ⏳.

## 7. Typed API : règles de dispatch
- « *Mixin injections run first … The first `Cancel` stops dispatch. For replacement hooks, the first `Some` wins. Collection hooks call every mod.* »
- Les hooks prennent `&self` et doivent être `Send + Sync` → état dans `Mutex` ou atomiques. Une instance sert **à la fois** le jeu fenêtre et le jeu serveur.
- Tests : `horizon_client::mod_api::dispatch_<hook>(&mut host, &mods, args…)` avec un double de `Host`.
