---
tags: [reference, ftb-quests, teams]
source: ServerQuestFile.java, docs Navigating, docs FTB Teams
verifie_le: 2026-11-10
---
# Équipes et progression (FTB Quests + FTB Teams)

Retour : [[FTB Quests - Vue d'ensemble]]

## Principe
- La progression est stockée **par équipe** (`TeamData`, indexé par l'UUID de l'équipe). Un joueur seul a une *équipe personnelle* implicite.
- Une tâche complétée par **n'importe quel membre** est complétée pour toute l'équipe.

## Événements d'équipe
| Situation | Effet sur la progression |
|---|---|
| Création d'une party | la progression de l'équipe perso du créateur est **copiée** dans la party (`copyData`) |
| Rejoindre une party existante | la progression du joueur est **fusionnée** dans la party (`mergeData`), puis les auto-claims sont vérifiés |
| Quitter la party | le joueur **retrouve** son ancienne progression perso ; les récompenses déjà réclamées restent réclamées (`mergeClaimedRewards`) |

## FTB Teams (pour mémoire)
- Types d'équipe : personnelle, party, serveur (non joignable).
- Commandes : `/ftbteams list`, `/ftbteams party …` (create, join, invite, leave…), `/ftbteams msg`, `/ftbteams info`, `/ftbteams server <create|delete|settings>` (OP), `/ftbteams force-disband` (OP).
- Party : invitations, libre ou sur invitation, chat d'équipe, alliés.

## Permissions et états
- Permission d'éditeur : `ftbquests.editor`. Le mode édition est propre à chaque joueur et se coupe si la permission est perdue.
- `locked` : le joueur ne peut plus ouvrir le livre ni compléter de quêtes.

> [!question] Pour nous
> Horizon n'a pas d'équivalent à FTB Teams. Il faudra soit un **système d'équipe minimal intégré** au mod, soit une dépendance vers un autre mod. Voir [[Questions ouvertes]] et [[Persistance et synchronisation]].
