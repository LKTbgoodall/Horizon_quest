---
tags: [adr]
statut: proposé
date: 2026-11-10
---
# ADR-006 : Format des quêtes en JSON, un fichier par chapitre

## Contexte
FTB stocke un fichier par chapitre (SNBT, puis JSON5). Cela donne des diffs git lisibles et permet de travailler à plusieurs. `serde_json` est dans le lock Horizon.

## Décision proposée
Les définitions vont dans `config/horizon_quest/quests/` : `quests.json` plus `chapters/*.json`, et plus tard `reward_tables/` et `lang/`. On utilise du JSON strict. Le chargement par datapack (`NativeReloadListener::json`) viendra en V1.

## Conséquences
- + Éditable à la main dès le MVP, avant d'avoir un éditeur in-game.
- + Un import FTB reste possible (conversion JSON5/SNBT → JSON).
- − Pas de commentaires en JSON strict.

[[Index des décisions]] · [[Format des fichiers de quêtes]]
