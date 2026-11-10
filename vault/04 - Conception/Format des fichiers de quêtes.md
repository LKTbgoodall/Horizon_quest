---
tags: [conception, format]
statut: brouillon
date: 2026-11-10
---
# Format des fichiers de quêtes

Inspiration : [[Format de fichiers FTB]] · Décision : [[ADR-006 Format des quêtes en JSON par chapitre]]

## Arborescence
```
config/horizon_quest/quests/
├── quests.json            ← version + réglages globaux + groupes de chapitres
├── chapters/
│   ├── 01_getting_started.json
│   └── 02_nether.json
├── reward_tables/         ← V1
│   └── common_loot.json
└── lang/                  ← V1 (si textes traduisibles)
    ├── fr_fr.json
    └── en_us.json
```

## Exemple de chapitre (v0)
```json
{
  "id": "getting_started",
  "title": "Premiers pas",
  "icon": "minecraft:oak_sapling",
  "progression": "linear",
  "quests": [
    {
      "id": "first_wood",
      "title": "Du bois !",
      "description": ["Frappe un arbre. Oui, à mains nues."],
      "x": 0, "y": 0,
      "tasks": [ { "id": "t1", "type": "item", "item": "#minecraft:logs", "count": 8 } ],
      "rewards": [ { "id": "r1", "type": "item", "item": "minecraft:apple", "count": 2 },
                   { "id": "r2", "type": "xp", "points": 20 } ]
    },
    {
      "id": "crafting_table",
      "title": "Établi",
      "x": 2, "y": 0,
      "dependencies": ["first_wood"],
      "tasks": [ { "id": "t1", "type": "item", "item": "minecraft:crafting_table", "count": 1 } ],
      "rewards": [ { "id": "r1", "type": "xp_levels", "levels": 1 } ]
    }
  ]
}
```
- `item` : `"minecraft:x"` pour un id, `"#ns:tag"` pour un tag.
- Les ids de tâches et récompenses sont uniques **dans la quête**. L'id complet est `chapitre/quête/tâche`.
- Pour tout champ absent, on prend le défaut du chapitre, puis celui du fichier.
- JSON strict, pour l'instant (`serde_json` est dans le lock Horizon ; ce n'est pas le cas d'un parseur JSON5 → vérifier si on veut des commentaires).

## Validation au chargement
- Ids dupliqués, dépendances inconnues, **cycles**, items inconnus (warning) → rapport clair dans la console et dans `/quests validate`.
- Un chapitre invalide est **ignoré**, pas tout le fichier.
