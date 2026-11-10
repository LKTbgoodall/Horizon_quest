---
tags: [reference, ftb-quests, storage]
source: ServerQuestFile.java, BaseQuestFile.java (main)
verifie_le: 2026-11-10
---
# Format de fichiers (FTB Quests)

Retour : [[FTB Quests - Vue d'ensemble]]

> [!note] SNBT → JSON5
> Les anciennes versions (1.16 à 1.20) utilisaient du **SNBT**. La branche actuelle utilise du **JSON5**, version de fichier 13. L'extension exacte vient de `Json5Util.FILE_EXT` (probablement `.json5`, non confirmé).

## Définition des quêtes (partagée, versionnable)
```
config/ftbquests/quests/
├── data.json5               ← réglages globaux (voir Modèle de données FTB)
├── chapter_groups.json5     ← liste des groupes
├── chapters/
│   └── <filename>.json5     ← un fichier par chapitre (quêtes, liens, images)
├── reward_tables/
│   └── <nom>.json5          ← id, order_index, données
└── lang/
    └── en_us.json5 …        ← titres, sous-titres, descriptions
```
Un fichier par chapitre permet des **diffs git lisibles** et le travail à plusieurs. C'est un bon choix à reprendre.

## Progression (propre au monde)
```
<monde>/ftbquests/<uuid-equipe>.json5   ← TeamData : tâches, quêtes complétées, récompenses réclamées
```

## Synchronisation (connaissance générale, non vérifiée dans le code)
Le serveur est **autoritaire** : il envoie le fichier de quêtes complet au client à la connexion, puis des deltas de progression. En mode édition, le client renvoie ses modifications au serveur, qui sauvegarde et rediffuse.

Notre format : [[Format des fichiers de quêtes]]
