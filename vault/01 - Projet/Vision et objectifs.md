---
tags: [projet]
date: 2026-11-10
---
# Vision et objectifs

## Le projet
**Horizon Quest** : un mod de quêtes pour **[[Horizon - Vue d'ensemble|Horizon]]**, la réimplémentation en Rust de Minecraft Java 26.3. Il s'inspire fortement de **[[FTB Quests - Vue d'ensemble|FTB Quests]]** : un livre de quêtes avec des chapitres, des quêtes reliées par des dépendances, des tâches et des récompenses, et une progression par équipe.

- Dépôt : https://github.com/LKTbgoodall/Horizon_quest (vide au 2026-11-10)
- Dossier local : `C:\Users\bastieng.BE3\Desktop\Bastien\autre\Horizon_quest`
- Aucun mod de quêtes n'existe dans le catalogue Horizon (158 projets au 2026-11-10) : **la place est libre**.

## Pourquoi
- Les modpacks ont besoin d'un fil conducteur, et FTB Quests est le standard sur Forge et NeoForge.
- Les mods Java ne tournent pas sur Horizon : il faut une réimplémentation native.

## Objectifs
1. **Parité fonctionnelle** avec le cœur de FTB Quests (voir [[Roadmap]] pour le découpage MVP, V1 et V2).
2. **Format de quêtes ouvert et lisible** : un fichier par chapitre, des diffs git propres, des textes traduisibles.
3. **Serveur autoritaire** : la progression est stockée et validée côté serveur, et fonctionne en solo comme en multijoueur.
4. **Publication** sur le catalogue officiel (https://playhorizon.dev/mods) sous licence libre (voir [[Publication et politique des mods]]).

## Pistes pour faire mieux que FTB
- Un éditeur plus découvrable, avec moins de menus cachés au clic droit.
- **Import de quêtes FTB Quests** (JSON5/SNBT → notre format) pour porter des modpacks existants. Ce serait un gros argument d'adoption.
- Une API publique pour les autres mods Horizon (tâches et récompenses custom, comme KubeJS chez FTB).
- Pas de dépendance obligatoire à un mod d'équipes tiers.

## Non-objectifs (pour l'instant)
- Les tâches d'énergie ou de fluides (il n'y a pas d'écosystème tech équivalent).
- La compatibilité binaire avec FTB Quests : on vise l'**import** de données, pas le code.

## Contraintes
- Licence FTB Quests *All Rights Reserved* : on ne reprend **ni code ni assets**, seulement les concepts.
- Horizon est jeune et son API de mods natifs est marquée **experimental** (voir [[Hooks et API Horizon]]).
