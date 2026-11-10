---
tags: [projet, questions]
mise_a_jour: 2026-11-10
---
# Questions ouvertes

> Format : `- [ ] **Question** : contexte → piste`. Une fois résolue, cocher et noter la réponse avec la date. Si elle mène à une décision, créer une ADR.

## Décisions à prendre (Bastien)
- [ ] **Licence** : MIT (comme Horizon et smp) ? GPL ? → il faut une licence libre pour publier
- [ ] **Id et nom du mod** : `horizon_quest` / « Horizon Quest » ? (id : `a-z0-9_`)
- [ ] **Langue des textes en jeu** : FR, EN, ou les deux via `lang/` dès le MVP ?
- [ ] **Textes des quêtes** : texte direct ou clés de traduction (comme FTB) ? → [[Modèle de données Horizon Quest]]
- [ ] **Équipes** : système intégré minimal (proposé) ou dépendance vers un autre mod ?
- [ ] **Item non donné** (inventaire plein) : le jeter au sol, ou une « boîte aux lettres » de récompenses ? → [[Récompenses - implémentation]]
- [ ] **Cible prioritaire** : solo/LAN ou serveur dédié ?

## Inconnues techniques (à vérifier dans le code ou auprès d'Horizon)
- [ ] **Serveur dédié** : la doc publique dit « Not there yet », mais `native-mods.md` documente `--server-output`. Est-ce utilisable aujourd'hui ? → tester
- [ ] **Signatures exactes** de `native_hud`, `native_fill`, `native_text`, `native_sprite`, `native_key`, `native_mod_key`… → lire `crates/horizon-client/src/game/native_hooks.rs`
- [ ] **`native_destroy_block`** : comment récupérer l'id du bloc cassé ?
- [ ] **`native_player_changed_dimension`** : la doc se contredit sur le solo (NM:2278 vs NM:1291) → tester
- [ ] **`native_mob_killed`** couvre-t-il les kills à distance (arc, trident) ?
- [ ] **Hooks ⏳ pending** : quand `player_inventory_changed`, `native_add_toast`, `native_title`, `native_boss_bar`, `native_has_permission` auront-ils un numéro ? → surveiller les Nightlies, demander sur Discord
- [ ] **Mod messages en solo** : fonctionnent-ils entre le jeu fenêtre et le serveur intégré ?
- [ ] **`/loot give`** est-il implémenté par Horizon ?
- [ ] **Un seul bundle client + serveur** : comment séparer proprement les deux états dans la même instance de mod ?
- [ ] **Clip/scissor 2D** pour le canvas : existe-t-il un helper non documenté (`gui_paint` ?)
- [ ] **Alias de commande** (`/q`) : possible ?
- [ ] **JSON5** : un parseur est-il disponible dans le `Cargo.lock` d'Horizon ? (pour l'import FTB)

## Résolues
- [x] *Horizon peut-il charger FTB Quests ?* → **Non**, pas de JVM ni de Mixins Java. Il faut réécrire (2026-11-10, [[ADR-001 Mod natif Rust pour Horizon]])
- [x] *Existe-t-il déjà un mod de quêtes sur Horizon ?* → **Non** (158 projets examinés le 2026-11-10)
- [x] *Un mod serveur peut-il afficher une GUI aux clients vanilla ?* → **Oui**, via les menus coffre (`native_open_menu`), comme smp (2026-11-10)
