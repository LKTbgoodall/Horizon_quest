---
tags: [reference, ftb-quests]
source: docs.feed-the-beast.com, github.com/FTBTeam/FTB-Quests (branche main), CurseForge
verifie_le: 2026-11-10
---
# FTB Quests : vue d'ensemble

> [!info] Rôle de cette note
> C'est le **modèle** qu'on veut reproduire sur Horizon. Cette section sert d'aide-mémoire pour ne pas oublier une fonctionnalité. On ne copie **ni le code ni les assets** : FTB Quests est sous licence *All Rights Reserved*, on réimplémente uniquement les concepts.

## En une phrase
FTB Quests est un mod de quêtes **orienté équipes**, avec un **livre de quêtes** (GUI à canvas pannable) organisé en **chapitres**. Chaque quête contient des **tâches** à accomplir et des **récompenses** à réclamer, et les quêtes sont reliées par des **dépendances**.

## Hiérarchie
```
Fichier de quêtes (data)
└── Groupe de chapitres
    └── Chapitre
        ├── Quêtes ──┬── Tâches (1..n)
        │            └── Récompenses (0..n)
        ├── Liens de quête (quest_links : une quête affichée dans un autre chapitre)
        └── Images (décoratives, sur le canvas)
Tables de récompenses (reward tables / loot crates)
```

## Détails
- [[Modèle de données FTB]] : tous les champs des quêtes et des chapitres, les dépendances et la visibilité
- [[Types de tâches FTB]]
- [[Types de récompenses FTB]]
- [[Équipes et progression FTB]]
- [[Éditeur et interface FTB]]
- [[Format de fichiers FTB]]
- [[Commandes FTB]]

## Versions, plateformes et licence
- Loaders : Forge et NeoForge (et Fabric dans le dépôt : `common/`, `fabric/`, `neoforge/`). Versions de Minecraft jusqu'à 26.1.2 (fichier 26.1.2.10 du 5 nov. 2026).
- Licence : **All Rights Reserved**.
- Intégrations via *FTB XMod Compat* : JEI/REI, KubeJS, GameStages, FTB Filter System.

## Plaintes et limites connues (à faire mieux)
- La configuration des tâches (consommer les items, matching NBT/components) est peu documentée.
- L'éditeur in-game est puissant mais peu découvrable : beaucoup de clics droits et de menus cachés.
- Le système est dépendant de FTB Teams et FTB Library.
- Voir [[Vision et objectifs]] pour nos axes d'amélioration.
