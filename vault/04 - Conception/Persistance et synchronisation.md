---
tags: [conception, storage, network]
statut: brouillon
date: 2026-11-10
---
# Persistance et synchronisation

API : [[Hooks et API Horizon]] §1 et §4 · Décision : [[ADR-005 Progression en JSON dans le dossier du monde]]

## Où vivent les données
| Données | Emplacement | Pourquoi |
|---|---|---|
| **Définitions des quêtes** (modpack) | `config/horizon_quest/quests/` (via `native_config_dir()`) | partagées entre mondes, versionnables, éditables à la main |
| Définitions livrées en datapack (V1) | `data/<ns>/horizon_quest/*.json` via `native_add_server_reload_listeners` + `NativeReloadListener::json` | rechargées par `/reload`, distribuables |
| Quêtes d'exemple | embarquées dans `data/` du bundle (`native_data!`) | démo sans configuration |
| **Progression et équipes** | `<monde>/horizon-quest/` (via `native_world_dir()`) | suit les sauvegardes du monde (pattern smp) |
| Config du mod | `config/horizon_quest-server.toml` / `-client.toml` via `native_register_configs` | standard |
| Préférences UI client | `native_setting` / `native_set_setting` | par joueur local |

## Fichiers de progression
```
<monde>/horizon-quest/
├── teams.json                 ← équipes, membres, invitations
├── progress/<team-id>.json    ← TeamProgress (un fichier par équipe)
├── audit.log                  ← JSON lines : complétions, claims (debug, anti-dupe)
└── state.json                 ← version du schéma, horodatage
```
- Écriture **atomique** (temp + `sync_all` + rename), sur un thread d'écriture en arrière-plan. On n'écrit que ce qui a changé (`dirty`).
- Lecture : fichier absent → défaut. Fichier corrompu → **erreur, la partie se désactive, et on n'écrase jamais** (règle smp).
- Migrations : un champ `version` dans chaque fichier, et `hq_store::migrate()`.

## Synchronisation client Horizon (V2)
- Canal `horizon_quest:sync` (mod messages, API 40). Contraintes : ≤ 30 000 octets par message et ≤ 32 messages par tick.
- Protocole (versionné) :
  1. client → `Hello { proto_version, mod_version }` à la connexion. Le serveur répond `Welcome` ou `Incompatible` (il n'y a **pas** de handshake natif chez Horizon) ;
  2. serveur → `Definitions { chunk i/n, payload }`, compressé si besoin (`flate2` est dans le lock) ;
  3. serveur → `Progress { full | delta }` ;
  4. client → `Claim { reward }`, `Submit { task }`, `Check { task }`, `Track { quest }`, puis en V3 les `Edit…`.
- Un client sans le mod ou un vanilla ne reçoit rien et utilise le livre coffre.
- Alternative : `AttachmentType { sync: true }` sur `Holder::Player` pour la progression (plus simple, 1 MiB max, clients Horizon uniquement). **À évaluer.**

## Questions
- Les mod messages fonctionnent-ils en solo, entre le jeu fenêtre et le serveur intégré ? → [[Questions ouvertes]]
