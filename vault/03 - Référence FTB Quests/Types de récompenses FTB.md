---
tags: [reference, ftb-quests, rewards]
source: docs FTB « Quest Rewards » + reward/*.java
verifie_le: 2026-11-10
---
# Types de récompenses FTB Quests

Retour : [[FTB Quests - Vue d'ensemble]]

| Type | Rôle | Priorité chez nous |
|---|---|---|
| **Item** | Donne un item (et un nombre) | 🔴 MVP |
| **XP** | Donne des points d'XP | 🔴 MVP |
| **XP Levels** | Donne des niveaux | 🔴 MVP |
| **Command** | Exécute une commande à la complétion (il faut régler le niveau de permission 2 dans la récompense) | 🟠 V1 |
| **Choice** | Le joueur **choisit** un item dans une table | 🟠 V1 |
| **Random** | Un item aléatoire d'une table pondérée (toujours au moins 1) | 🟠 V1 |
| **Loot** | Tirage pondéré d'une table, peut ne rien donner (*empty weight*) | 🟡 V2 |
| **All Table** | Tous les items d'une table, sans pondération | 🟡 V2 |
| **Toast** | Affiche un toast avec un message | 🟠 V1 |
| **Advancement** | Accorde un progrès | 🟡 V2 |
| **Stage** | Ajoute ou retire un game stage | ⚪ |
| **Currency** | Monnaie (présent dans le code, pas dans la doc) | 🟡 (lien possible avec l'économie de `smp`) |
| **Custom** | Ne fait rien, sert aux scripts | 🟡 |

## Réclamation (claim)
- `RewardAutoClaim` : `default`, `disabled`, `enabled`, `no_toast`, `invisible`. Le défaut au niveau du fichier est `disabled`.
- `RewardClaimType` : `CAN_CLAIM`, `CANT_CLAIM`, `CLAIMED`.
- Une récompense peut être une **team reward** : réclamée une seule fois pour toute l'équipe, au lieu d'une fois par joueur.
- Un bouton « Claim all » réclame tout. Option « exclude from claim all » (clé exacte non vérifiée).
- `/ftbquests block_rewards` bloque la réclamation pour un joueur.

## Tables de récompenses / loot crates
- Une table est une liste pondérée d'items, dans `reward_tables/<nom>`.
- Elle peut être convertie en **loot crate** : un item `ftbquests:lootcrate` qu'on peut placer dans des loot tables vanilla, et les crates s'imbriquent.
- Commandes : `import_reward_table_from_chest`, `export_reward_table_to_chest`.

Implémentation chez nous : [[Récompenses - implémentation]]
