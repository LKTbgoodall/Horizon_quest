---
tags: [conception, commands]
statut: brouillon
date: 2026-11-10
---
# Commandes et permissions

Référence : [[Commandes FTB]] · API : [[Hooks et API Horizon]] §6 · Pattern : [[Exemple - mod smp (patterns à reprendre)]] §3

> Rappel : on enregistre une usage-string **par forme**. Le parsing se fait à la main dans `hq_commands` (pur), qui renvoie `Vec<Out>`.

## Joueurs (niveau 0)
| Usage-string | Effet | Phase |
|---|---|---|
| `quests` | ouvre le livre | MVP |
| `quests open <quest:word>` | ouvre une quête | MVP |
| `quests track [quest:word]` | suit une quête (sidebar), ou arrête si l'argument est vide | MVP |
| `quests claim [quest:word]` | réclame (tout, ou une seule quête) | MVP |
| `quests check <task:word>` | valide une tâche checkmark | MVP |
| `quests submit <quest:word>` | soumet les items (tâches consume) | MVP |
| `quests team create <name:word>` | crée une party | V1 |
| `quests team invite <player:player>` / `accept` / `leave` / `info` | gestion d'équipe | V1 |

## Admins (niveau 2+)
| Usage-string | Niveau | Effet | Phase |
|---|---|---|---|
| `quests reload` | 2 | recharge les définitions | MVP |
| `quests validate` | 2 | rapport de validation | MVP |
| `quests complete <player:player> <quest:word>` | 2 | force la complétion | MVP |
| `quests reset <player:player> <quest:word>` | 2 | réinitialise une quête | MVP |
| `quests reset_all <player:player>` | 3 | réinitialise tout | MVP |
| `quests progress <player:player> <task:word> <amount:integer>` | 2 | progression custom | V2 |
| `quests lock <player:player> <state:boolean>` | 3 | verrouille le livre | V1 |
| `quests block_rewards <player:player> <state:boolean>` | 3 | bloque les claims | V1 |
| `quests edit [state:boolean]` | 2 | mode édition | V3 |
| `quests import_ftb <path:text>` | 4 | import FTB Quests | V3 |

## Alias
`q` → `quests` (à vérifier : une usage-string séparée est-elle nécessaire ?).

## Permissions
- MVP : niveaux op (`NativeCommand::op(usage, level)`), plus une re-vérification dans les règles.
- Plus tard : nœuds `horizon_quest.admin`, `horizon_quest.editor` quand `native_has_permission` sortira (⏳).
