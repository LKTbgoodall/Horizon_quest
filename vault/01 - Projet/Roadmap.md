---
tags: [projet, roadmap]
mise_a_jour: 2026-11-10
---
# Roadmap

> Cocher au fur et à mesure. La **phase en cours** est indiquée dans [[00 - Accueil]].
> Priorités des fonctionnalités : [[Types de tâches FTB]], [[Types de récompenses FTB]].

## Phase 0 : mise en place ⬅️ EN COURS
- [x] Recherche Horizon et FTB Quests (2026-11-10)
- [x] Vault de projet (2026-11-10)
- [ ] Valider les ADR 002 à 006 → [[Index des décisions]]
- [ ] Choisir la licence et l'id du mod (`horizon_quest` ?) → [[Questions ouvertes]]
- [ ] Installer les outils → [[Build, test et outils]] § Prérequis
- [ ] `git init`, `.gitignore`, premier commit et push vers https://github.com/LKTbgoodall/Horizon_quest
- [ ] Recloner les références dans `ext/` (horizon : docs + `native-hooks.json` + `crates/horizon-client/src/game/native_hooks.rs` ; horizon-mods)
- [ ] **Hello world** : copier `native/fps-overlay` ou `coords-hud`, le renommer, le builder avec le launcher et le voir en jeu
- [ ] **Hello world serveur** : une commande `/quests` qui répond dans le chat (pattern `register.rs` + `command.rs` de smp)
- [ ] Vérifier les signatures réelles des hooks dans `native_hooks.rs` → mettre à jour [[Hooks et API Horizon]]

## Phase 1 : cœur logique pur (sans le jeu, `cargo test`)
- [ ] `hq_model` : types + serde + chargement de `quests.json` et `chapters/*.json`
- [ ] Validation : ids dupliqués, dépendances inconnues, cycles
- [ ] `hq_logic` : dépendances (4 modes + `min_required`), visibilité, startable, linear/flexible
- [ ] `hq_progress` : `QuestEvent` → progression → `Vec<Effect>`
- [ ] `hq_rewards` : item, xp, xp_levels, claim team/joueur, auto-claim
- [ ] `hq_store` : JSON atomique, migrations
- [ ] `hq_teams` minimal : équipe perso implicite
- [ ] Chapitre d'exemple « Premiers pas » dans `data/`

## Phase 2 : MVP serveur jouable 🎯
- [ ] Glue : état global, `server_tick`, `world_lifecycle`, `player_logged_out`
- [ ] Commandes MVP → [[Commandes et permissions]]
- [ ] Tâches : item (sondage), checkmark, kill (`mob_killed`)
- [ ] Récompenses : item (`give_stack`), xp, niveaux
- [ ] Livre coffre : chapitres → quêtes → détail, réclamer, soumettre
- [ ] Quête suivie dans la sidebar et notifications dans le chat
- [ ] Sauvegarde dans `<monde>/horizon-quest/`
- [ ] Test en solo, puis sur un serveur dédié (engine export)
- [ ] **Release 0.1.0** (privée, entre amis)

## Phase 3 : V1
- [ ] Tâches : craft, xp, dimension, location, advancement
- [ ] Récompenses : commande, choice, random, toast, tables de récompenses
- [ ] Parties (équipes) : create, invite, join, leave, avec les règles de fusion de FTB
- [ ] Quêtes répétables, optionnelles, tâches séquentielles
- [ ] Config TOML (`native_register_configs`)
- [ ] Quêtes en datapack (`NativeReloadListener`)
- [ ] Textes traduisibles (lang)
- [ ] **Publication 0.x sur playhorizon.dev** → [[Publication et politique des mods]]

## Phase 4 : V2, livre graphique (client Horizon)
- [ ] `hq_net` : handshake, envoi découpé des définitions, deltas de progression
- [ ] Écran de mod : canvas pan/zoom, formes, lignes, icônes
- [ ] Sidebar des chapitres, panneau de quête, boutons, `show_recipes`
- [ ] Toasts client, ouverture par touche ou par item
- [ ] Assets (`.hmod` combiné) : textures des formes, icône

## Phase 5 : V3, éditeur et import
- [ ] Éditeur in-game (mode édition)
- [ ] Import FTB Quests (JSON5/SNBT)
- [ ] API pour les autres mods (tâches et récompenses custom)
- [ ] Tâches stat, biome, structure (selon les hooks disponibles)
