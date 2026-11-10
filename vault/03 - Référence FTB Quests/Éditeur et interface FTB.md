---
tags: [reference, ftb-quests, ui]
source: docs FTB Navigating, Tips & Tricks, Styling, Opening
verifie_le: 2026-11-10
---
# Éditeur et interface (FTB Quests)

Retour : [[FTB Quests - Vue d'ensemble]]

## Ouvrir le livre
- Icône de livre en haut à gauche de l'inventaire.
- Item **livre de quêtes** (craftable), en clic droit.
- Touche dédiée (**non assignée** par défaut).
- `/ftbquests open_book [questId]`.

## Disposition de l'écran
```
┌──────────┬────────────────────────────────────────┬─────┐
│ Chapitres│        CANVAS du chapitre              │ i ⚙ │
│ (sidebar │   (o)──►(o)──►(◇)                      │Claim│
│ au survol│          └──►(o)                       │ all │
│ + épingle│   drag = pan, molette = zoom           │     │
└──────────┴────────────────────────────────────────┴─────┘
Clic sur une quête → fenêtre : titre, sous-titre, description,
tâches (clic = recette/infos), récompenses (clic = réclamer),
flèches ◄ ► = dépendances / dépendants, icône lien, épingle = suivre la quête
```
- La sidebar des chapitres s'ouvre au survol du bord gauche, avec une épingle pour la garder ouverte.
- On fait glisser une zone vide pour **panner**. L'icône « i » liste les raccourcis et `Ctrl+F` cherche une quête.
- **Quête épinglée** : elle est suivie à l'écran (HUD).

## Mode édition
- `/ftbquests editing_mode [true|false] [player]`, avec la permission `ftbquests.editor`.
- Clic droit sur le vide → créer une quête en choisissant un type de tâche.
- Un « + » dans les sections tâches et récompenses pour en ajouter.
- Clic droit sur une quête ou un chapitre → éditer, déplacer, supprimer.
- Le « + » vert de la sidebar crée un chapitre ou un groupe.
- Le rouage (en bas à droite) → éditer les tables de récompenses.

## Thème
- `ftbquests/ftb_quests_theme.txt` dans un resource pack, avec des sélecteurs `[*]`, `[#tag]`, `[quest-id]`.
- Formes personnalisées : `textures/shapes/<nom>/{background,outline,shape}.png`.

Notre version : [[Interface - Livre de quêtes]]
