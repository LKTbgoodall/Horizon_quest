---
tags: [reference, ftb-quests, tasks]
source: docs FTB « Quest Types »
verifie_le: 2026-11-10
---
# Types de tâches FTB Quests

Retour : [[FTB Quests - Vue d'ensemble]]

| Type | Rôle | Options clés (connues) | Priorité chez nous |
|---|---|---|---|
| **Item** | Obtenir un item (n exemplaires) | item, count, consume (prendre les items ou juste vérifier), matching tag/composants (via FTB Filter System) | 🔴 MVP |
| **Checkmark** | Le joueur clique pour valider | (aucune) | 🔴 MVP |
| **Kill Entity** | Tuer n entités d'un type | entity, count | 🔴 MVP |
| **XP Levels** | Avoir / dépenser n niveaux | value, points/niveaux, consume | 🟠 V1 |
| **Visit Dimension** | Aller dans une dimension | dimension | 🟠 V1 |
| **Location** | Entrer dans une zone (cuboïde) | dim, x/y/z, w/h/d | 🟠 V1 |
| **Advancement** | Obtenir un progrès vanilla | advancement, critère | 🟠 V1 (si hook) |
| **Stat** | Une stat vanilla ≥ valeur | stat, value | 🟡 V2 |
| **Observation** | Regarder un bloc ou une entité pendant n ticks | cible, durée | 🟡 V2 |
| **Visit Biome** | Être dans un biome | biome | 🟡 V2 |
| **Find Structure** | Être dans une structure | structure / tag | 🟡 V2 |
| **Fluid** | Fournir un fluide (via un bloc tâche) | fluid, amount | ⚪ plus tard |
| **Forge Energy** | Fournir de l'énergie | amount | ⚪ non pertinent sur Horizon pour l'instant |
| **Stage** | Avoir un « game stage » | stage | ⚪ (dépend de GameStages) |
| **Custom** | Piloté par script (KubeJS) | — | 🟡 via API publique de notre mod |
| **Image** | (pseudo-type) image décorative | — | 🟡 |

> [!warning] Infos non vérifiées
> Les options précises des tâches (consume, NBT, count) ne sont **pas documentées** dans les pages lues. Le dossier `task/` du code source n'a pas été lu. Les options ci-dessus viennent de la connaissance générale du mod.

Correspondance avec les hooks Horizon : [[Tâches - correspondance hooks]]
